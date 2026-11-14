//! Listen-only capture of a vehicle route.
//!
//! A capture is the tester's zero-risk first contact with a car and the
//! artifact the tester programme collects. Everything here is receive-only:
//! the route is opened with `DT_LISTEN_ONLY`, so the adapter neither transmits
//! nor acknowledges frames, and a capture cannot disturb the vehicle. The
//! channel gets its pass entries like every CAN channel (ADR-0043): they
//! choose what the adapter hands up, and transmit nothing.
//!
//! What a capture can establish is bounded, and the bound is stated where the
//! result is shown: traffic present or absent at the configured rate on the
//! configured pins. Listening cannot tell which of a vehicle's buses it is
//! hearing, and a silent pair cannot be told apart from a mismatched rate or
//! an unconnected one.

use crate::device::{MongooseJlrDevice, ReceiveFilterEntry};
use crate::error::ProtocolError;
use crate::passive::{RawCanFrame, VehicleRoute, VehicleRouteId};
use std::time::{Duration, Instant};
use transport_api::ByteTransport;

/// How long one receive call may wait before the loop re-checks the deadline.
pub const CAPTURE_IDLE_TIMEOUT: Duration = Duration::from_millis(200);
/// Frames kept per capture. A busy 500 kbit/s bus produces a few thousand a
/// second; the cap bounds memory and is reported as `truncated`.
pub const CAPTURE_MAX_FRAMES: usize = 50_000;

/// One received frame with the host-side offset from the start of the capture.
/// The adapter's own timestamp travels inside `frame`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapturedFrame {
    pub frame: RawCanFrame,
    pub host_offset_us: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteCapture {
    pub route: VehicleRoute,
    pub frames: Vec<CapturedFrame>,
    /// How long the route was actually open for listening.
    pub listened: Duration,
    /// Non-data packets the adapter delivered during the capture: its own
    /// indications, late answers, and frames too short or on another
    /// channel. Nothing was transmitted, so every one of them is the
    /// adapter's own word (ADR-0042).
    pub dropped_frames: u64,
    /// The frame cap was reached before the deadline; more traffic existed.
    pub truncated: bool,
    /// The pass entries the channel got before it listened (ADR-0043), as
    /// the firmware answered them.
    pub receive_filters: Vec<ReceiveFilterEntry>,
}

impl<T: ByteTransport> MongooseJlrDevice<T> {
    /// Listen on a route for `duration`, or until `max_frames` frames have been
    /// received, then close the route. Only the listen-only open, the pin
    /// selection, the pass entries and the close are written to the adapter.
    pub fn capture_route(
        &mut self,
        route_id: VehicleRouteId,
        duration: Duration,
        idle_timeout: Duration,
        max_frames: usize,
    ) -> Result<RouteCapture, ProtocolError> {
        let open = self.open_receive_route(route_id)?;
        let started = Instant::now();
        let deadline = started + duration;
        let mut frames = Vec::new();
        let mut truncated = false;

        let outcome = loop {
            if frames.len() >= max_frames {
                truncated = true;
                break Ok(());
            }
            if Instant::now() >= deadline {
                break Ok(());
            }
            match self.receive_frame(idle_timeout) {
                Ok(Some(frame)) => frames.push(CapturedFrame {
                    frame,
                    host_offset_us: u64::try_from(started.elapsed().as_micros())
                        .unwrap_or(u64::MAX),
                }),
                Ok(None) => continue,
                Err(error) => break Err(error),
            }
        };
        let listened = started.elapsed();
        // The trace began at this capture's open, so its counts are the
        // capture's own.
        let trace = self.exchange_trace();
        let dropped_frames =
            trace.dropped_frames + u64::from(trace.indications) + u64::from(trace.stale_frames);
        let receive_filters = trace.receive_filters.clone();
        let closed = self.close_route();

        match (outcome, closed) {
            (Ok(()), Ok(())) => Ok(RouteCapture {
                route: open.route,
                frames,
                listened,
                dropped_frames,
                truncated,
                receive_filters,
            }),
            (Ok(()), Err(error)) | (Err(error), _) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::encode_board_info_response;
    use crate::passive::{
        command_response, inbound_can_frame, indication, open_channel_request,
        pass_everything_entry_request, table_entry_accepted, CAN1_RESOURCE_ROUTE,
        CAN2_RESOURCE_ROUTE, CLOSE_CHANNEL_RESPONSE, OPEN_CHANNEL_RESPONSE, PASS_ENTRY_FORMATS,
        SET_PIN_RESPONSE,
    };
    use crate::CanIdFormat;

    /// The pass entries every CAN open adds after its pins (ADR-0043).
    const ENTRIES: u16 = PASS_ENTRY_FORMATS.len() as u16;
    const E: usize = PASS_ENTRY_FORMATS.len();

    fn entry_steps(channel: u16) -> Vec<Step> {
        (0..ENTRIES)
            .map(|index| {
                Step::Bytes(table_entry_accepted(
                    channel,
                    4 + index,
                    0x50 + u32::from(index),
                    0x04CC + u32::from(index),
                ))
            })
            .collect()
    }
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use transport_api::TransportError;

    /// What the adapter does next when read. `Silent` stays at the front of the
    /// script, answering every read with a timeout, until the device writes
    /// something — which is exactly how a quiet bus behaves until the tester
    /// closes the channel.
    enum Step {
        Bytes(Vec<u8>),
        Silent,
    }

    struct ScriptedTransport {
        steps: VecDeque<Step>,
        writes: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    impl ScriptedTransport {
        fn new(steps: Vec<Step>) -> (Self, Arc<Mutex<Vec<Vec<u8>>>>) {
            let writes = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    steps: steps.into(),
                    writes: Arc::clone(&writes),
                },
                writes,
            )
        }
    }

    impl ByteTransport for ScriptedTransport {
        fn write_all(&mut self, bytes: &[u8]) -> Result<(), TransportError> {
            self.writes.lock().unwrap().push(bytes.to_vec());
            if matches!(self.steps.front(), Some(Step::Silent)) {
                self.steps.pop_front();
            }
            Ok(())
        }

        fn read(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
            match self.steps.front() {
                Some(Step::Silent) | None => Err(TransportError::Timeout),
                Some(Step::Bytes(_)) => {
                    let Some(Step::Bytes(bytes)) = self.steps.pop_front() else {
                        unreachable!()
                    };
                    let count = bytes.len().min(buffer.len());
                    buffer[..count].copy_from_slice(&bytes[..count]);
                    Ok(count)
                }
            }
        }

        fn set_read_timeout(&mut self, _timeout: Duration) -> Result<(), TransportError> {
            Ok(())
        }

        fn close(&mut self) -> Result<(), TransportError> {
            Ok(())
        }
    }

    const CHANNEL: u16 = CAN1_RESOURCE_ROUTE;

    /// Board-info as the running firmware answers it: the first exchange
    /// of every route open (ADR-0018).
    fn firmware_board_info(sequence: u16) -> Vec<u8> {
        encode_board_info_response(sequence, &[0, 0, 0, 0, 0x10, 0x20, 0x30, 0x40, 5, 1, 1, 1])
    }

    fn session(channel: u16, frames: Vec<Vec<u8>>, silent_after_frames: bool) -> Vec<Step> {
        let mut steps = vec![
            Step::Bytes(firmware_board_info(1)),
            Step::Bytes(command_response(channel, OPEN_CHANNEL_RESPONSE, 2, 0)),
            Step::Bytes(command_response(channel, SET_PIN_RESPONSE, 3, 0)),
        ];
        steps.extend(entry_steps(channel));
        steps.extend(frames.into_iter().map(Step::Bytes));
        if silent_after_frames {
            steps.push(Step::Silent);
        }
        steps.push(Step::Bytes(command_response(
            channel,
            CLOSE_CHANNEL_RESPONSE,
            4 + ENTRIES,
            0,
        )));
        steps
    }

    #[test]
    fn capture_keeps_frames_in_order_and_writes_only_the_listen_only_commands() {
        let (transport, writes) = ScriptedTransport::new(session(
            CHANNEL,
            vec![
                inbound_can_frame(CHANNEL, 0, 10, 0x321, &[1, 2, 3]),
                inbound_can_frame(CHANNEL, 0, 20, 0x7E8, &[0x06, 0x49, 0x02]),
            ],
            false,
        ));
        let mut device = MongooseJlrDevice::open(transport);

        let capture = device
            .capture_route(
                VehicleRouteId::HsCan,
                Duration::from_secs(5),
                Duration::from_millis(10),
                2,
            )
            .unwrap();

        assert_eq!(capture.route.id, VehicleRouteId::HsCan);
        assert_eq!(capture.route.obd_pins, &[6, 14]);
        assert_eq!(capture.frames.len(), 2);
        assert_eq!(capture.frames[0].frame.arbitration_id, 0x321);
        assert_eq!(capture.frames[0].frame.payload(), &[1, 2, 3]);
        assert_eq!(capture.frames[1].frame.arbitration_id, 0x7E8);
        assert!(capture.frames[0].host_offset_us <= capture.frames[1].host_offset_us);
        // Stopped at the cap, so more may have followed.
        assert!(capture.truncated);
        assert_eq!(capture.dropped_frames, 0);

        // Board-info, listen-only open, pin selection, the pass entries,
        // close. The open carries the CAN1 resource, the listen-only flag and
        // the route's rate; nothing resembling a CAN transmission is ever
        // written.
        let writes = writes.lock().unwrap();
        assert_eq!(writes.len(), 4 + E);
        assert_eq!(
            writes[1],
            open_channel_request(2, CAN1_RESOURCE_ROUTE, 500_000)
        );
        assert_eq!(
            writes[3],
            pass_everything_entry_request(4, CAN1_RESOURCE_ROUTE, CanIdFormat::Standard)
        );
        assert!(writes.iter().all(|write| write[8..10] != [0x08, 0x00]));
        assert_eq!(capture.receive_filters.len(), E);
        assert!(capture.receive_filters.iter().all(|entry| entry.accepted()));
    }

    #[test]
    fn a_silent_pair_yields_no_frames_and_the_route_is_still_closed() {
        let (transport, writes) =
            ScriptedTransport::new(session(CAN2_RESOURCE_ROUTE, vec![], true));
        let mut device = MongooseJlrDevice::open(transport);

        let capture = device
            .capture_route(
                VehicleRouteId::MsCan,
                Duration::from_millis(30),
                Duration::from_millis(5),
                100,
            )
            .unwrap();

        assert!(capture.frames.is_empty());
        assert!(!capture.truncated);
        assert!(capture.listened >= Duration::from_millis(30));
        assert_eq!(capture.route.obd_pins, &[3, 11]);
        assert_eq!(writes.lock().unwrap().len(), 4 + E);
        assert_eq!(
            writes.lock().unwrap()[1],
            open_channel_request(2, CAN2_RESOURCE_ROUTE, 125_000)
        );
    }

    /// Nothing is transmitted during a capture, so an adapter record that is
    /// not a frame — an indication, a late answer — is counted as dropped
    /// and the frames around it are still captured (ADR-0042).
    #[test]
    fn the_adapters_own_records_during_a_capture_are_counted_as_dropped() {
        let (transport, _) = ScriptedTransport::new(session(
            CHANNEL,
            vec![
                inbound_can_frame(CHANNEL, 0, 10, 0x321, &[1]),
                indication(CHANNEL, 0, 0, 11),
                command_response(CHANNEL, CLOSE_CHANNEL_RESPONSE, 90, 0),
                inbound_can_frame(CHANNEL, 0, 12, 0x322, &[2]),
            ],
            false,
        ));
        let mut device = MongooseJlrDevice::open(transport);

        let capture = device
            .capture_route(
                VehicleRouteId::HsCan,
                Duration::from_secs(5),
                Duration::from_millis(10),
                2,
            )
            .unwrap();

        assert_eq!(capture.frames.len(), 2);
        assert_eq!(capture.frames[1].frame.arbitration_id, 0x322);
        assert_eq!(capture.dropped_frames, 2);
    }

    #[test]
    fn a_transport_that_closes_mid_capture_is_an_error_not_a_partial_result() {
        // The script ends without a close response: after the frame the
        // transport reports closed (a zero-length read), which the device
        // surfaces instead of returning half a capture as if it were whole.
        let mut steps = vec![
            Step::Bytes(firmware_board_info(1)),
            Step::Bytes(command_response(CHANNEL, OPEN_CHANNEL_RESPONSE, 2, 0)),
            Step::Bytes(command_response(CHANNEL, SET_PIN_RESPONSE, 3, 0)),
        ];
        steps.extend(entry_steps(CHANNEL));
        steps.push(Step::Bytes(inbound_can_frame(CHANNEL, 0, 10, 0x321, &[1])));
        steps.push(Step::Bytes(Vec::new()));
        let (transport, _writes) = ScriptedTransport::new(steps);
        let mut device = MongooseJlrDevice::open(transport);

        let result = device.capture_route(
            VehicleRouteId::HsCan,
            Duration::from_secs(5),
            Duration::from_millis(5),
            100,
        );
        assert!(result.is_err());
    }
}
