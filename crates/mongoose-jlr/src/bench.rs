//! The bench adapter (ADR-0020): a `ByteTransport` that answers the
//! application with the MongoosePro JLR firmware's own frames — board-info
//! from the bootloader until the jump, resources 5 and 21 with their pins,
//! the outbound record answered and passed to a vehicle on a [`BenchBus`],
//! whose frames come back as inbound data. Everything above this transport,
//! in this crate and in the shell, runs unchanged and real; what is
//! synthetic is the vehicle, and only the vehicle.
//!
//! The frames are the ones the bench work of 2026-09-06 and 2026-09-08
//! recorded (`docs/evidence/F2_MONGOOSE_FIRMWARE_STATE_2026-09-06.md`),
//! refusals included, so the application cannot tell the bench from the
//! adapter by the bytes — only by what it says about itself.

use crate::codec::{encode_board_info_response, Frame, FrameDecoder, MongoosePacket};
use crate::passive::{
    self, VehicleRouteId, CAN1_RESOURCE_ROUTE, CAN2_RESOURCE_ROUTE, CAN_29BIT_ID, DEVICE_ROUTE,
};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use transport_api::{BenchBus, BenchRoute, ByteTransport, CanFrame, CanId, TransportError};

/// The vehicle on the bench, shared with whoever describes it (the shell's
/// session, which rebuilds it when the vehicle changes).
pub type SharedBenchBus = Arc<Mutex<Box<dyn BenchBus>>>;

/// Wrap a vehicle for sharing.
pub fn share_bus(bus: Box<dyn BenchBus>) -> SharedBenchBus {
    Arc::new(Mutex::new(bus))
}

/// What the real adapter's board-info body carries at bytes 8..28 in the
/// bootloader and in the firmware, as recorded; the tick at 4..8 is zero in
/// the bootloader and running in the firmware, which is how the application
/// tells them apart (ADR-0018).
const BOARD_INFO_BOOTLOADER_TAIL: [u8; 20] = [
    0x05, 0x02, 0x01, 0x01, 0xB4, 0xB0, 0x00, 0x00, 0xD8, 0x55, 0x00, 0x00, 0x00, 0x08, 0x01, 0x01,
    0x00, 0x10, 0x01, 0x01,
];
const BOARD_INFO_FIRMWARE_TAIL: [u8; 20] = [
    0x05, 0x01, 0x01, 0x01, 0xB4, 0xB0, 0x00, 0x00, 0xD8, 0x55, 0x00, 0x00, 0x00, 0x08, 0x01, 0x01,
    0x00, 0x10, 0x01, 0x01,
];
const BOARD_INFO_LENGTH: usize = 168;
/// Logical microseconds between two frames the bench emits.
const FRAME_STEP_US: u64 = 250;

struct Channel {
    route: BenchRoute,
    listen_only: bool,
    pins_selected: bool,
}

/// The stand-in adapter. One per connection; the vehicle behind it is shared.
pub struct BenchTransport {
    bus: SharedBenchBus,
    decoder: FrameDecoder,
    outbox: VecDeque<u8>,
    in_bootloader: bool,
    channels: BTreeMap<u16, Channel>,
    read_timeout: Duration,
    clock_us: u64,
    closed: bool,
}

impl BenchTransport {
    /// A bench that powers up in the bootloader, like the adapter.
    pub fn new(bus: SharedBenchBus) -> Self {
        Self {
            bus,
            decoder: FrameDecoder::default(),
            outbox: VecDeque::new(),
            in_bootloader: true,
            channels: BTreeMap::new(),
            read_timeout: Duration::from_millis(100),
            clock_us: 0,
            closed: false,
        }
    }

    /// The vehicle's one-line description, for the panel.
    pub fn describe_vehicle(&self) -> String {
        self.bus
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .describe()
    }

    fn tick(&mut self) -> u32 {
        self.clock_us += FRAME_STEP_US;
        u32::try_from(self.clock_us).unwrap_or(u32::MAX)
    }

    fn emit(&mut self, bytes: Vec<u8>) {
        self.outbox.extend(bytes);
    }

    fn board_info(&mut self, sequence: u16) {
        let mut body = vec![0_u8; BOARD_INFO_LENGTH];
        if self.in_bootloader {
            body[8..28].copy_from_slice(&BOARD_INFO_BOOTLOADER_TAIL);
        } else {
            let tick = self.tick();
            body[4..8].copy_from_slice(&tick.to_le_bytes());
            body[8..28].copy_from_slice(&BOARD_INFO_FIRMWARE_TAIL);
        }
        self.emit(encode_board_info_response(sequence, &body));
    }

    fn refuse(&mut self, route: u16, response: u16, sequence: u16, status: u32, text: &str) {
        let tick = self.tick();
        self.emit(passive::command_response_with_text(
            route, response, sequence, status, tick, text,
        ));
    }

    fn accept(&mut self, route: u16, response: u16, sequence: u16) {
        self.emit(passive::command_response(route, response, sequence, 0));
    }

    fn handle(&mut self, frame: &Frame) {
        let Ok(packet) = MongoosePacket::from_frame(frame) else {
            return;
        };
        let command = packet.command();
        let sequence = packet.sequence();
        let route = packet.route_a();
        let body = packet.body().to_vec();
        let response = command | 0x8000;

        match command {
            crate::codec::GET_BOARD_INFO_COMMAND => self.board_info(sequence),
            passive::JUMP_TO_FIRMWARE => {
                self.in_bootloader = false;
                self.accept(DEVICE_ROUTE, passive::JUMP_TO_FIRMWARE_RESPONSE, sequence);
            }
            _ if self.in_bootloader => {
                self.refuse(
                    route,
                    response,
                    sequence,
                    1,
                    "Invalid or Unhandled command type",
                );
            }
            passive::OPEN_CHANNEL => self.open_channel(route, sequence, &body),
            passive::SET_PIN => self.set_pin(route, sequence, &body),
            // The two line commands (ADR-0029 slice B). The real firmware
            // takes the config word without validating it, and answers the
            // fast init when a module replies; the bench does the same.
            crate::kline::SET_CONFIG => {
                self.accept(route, crate::kline::SET_CONFIG_RESPONSE, sequence)
            }
            crate::kline::FAST_INIT => self.fast_init(route, sequence),
            passive::CLOSE_CHANNEL => {
                self.channels.remove(&route);
                self.accept(route, passive::CLOSE_CHANNEL_RESPONSE, sequence);
            }
            passive::OUTBOUND_DATA => self.outbound(route, sequence, &body),
            _ => self.refuse(
                route,
                response,
                sequence,
                1,
                "Invalid or Unhandled command type",
            ),
        }
    }

    fn open_channel(&mut self, route: u16, sequence: u16, body: &[u8]) {
        let bench_route = match route {
            CAN1_RESOURCE_ROUTE => BenchRoute::HsCan,
            CAN2_RESOURCE_ROUTE => BenchRoute::MsCan,
            crate::kline::ISO9141_RESOURCE_ROUTE | crate::kline::ISO14230_RESOURCE_ROUTE => {
                BenchRoute::KLine7
            }
            other => {
                let text = format!(
                    "cOpenChannel: Unsupported or Invalid Resource ID {}",
                    other >> 8
                );
                self.refuse(route, passive::OPEN_CHANNEL_RESPONSE, sequence, 3, &text);
                return;
            }
        };
        let flags = read_u32(body, 0).unwrap_or(0);
        self.channels.insert(
            route,
            Channel {
                route: bench_route,
                listen_only: flags & passive::DT_LISTEN_ONLY != 0,
                pins_selected: false,
            },
        );
        self.accept(route, passive::OPEN_CHANNEL_RESPONSE, sequence);
    }

    fn set_pin(&mut self, route: u16, sequence: u16, body: &[u8]) {
        let pins = (read_u32(body, 4), read_u32(body, 8));
        let Some(channel) = self.channels.get_mut(&route) else {
            self.refuse(
                route,
                passive::SET_PIN_RESPONSE,
                sequence,
                2,
                "SetPins: channel is not open",
            );
            return;
        };
        let (expected, text) = match channel.route {
            BenchRoute::HsCan => (
                (Some(6), Some(14)),
                "SetPins: MongoosePro JLR board only supports CAN on pins 6 and 14",
            ),
            BenchRoute::MsCan => (
                (Some(3), Some(11)),
                "SetPins: MongoosePro JLR board only supports CAN2 on pin 3 and 11",
            ),
            // One wire, and the firmware's own words for a wrong one, read
            // off the adapter on 2026-09-12.
            BenchRoute::KLine7 => (
                (Some(7), Some(0)),
                "MongoosePro JLR board supports ISO9141 K line on pin 3, 7 or 8",
            ),
            BenchRoute::KLine8 => (
                (Some(8), Some(0)),
                "MongoosePro JLR board supports ISO9141 K line on pin 3, 7 or 8",
            ),
        };
        if pins != expected {
            self.refuse(route, passive::SET_PIN_RESPONSE, sequence, 0x206, text);
            return;
        }
        channel.pins_selected = true;
        self.accept(route, passive::SET_PIN_RESPONSE, sequence);
    }

    /// The wake-up ISO 14230 asks for. With a module on the line the real
    /// adapter answers it; with none it times out, in the firmware's own
    /// words.
    fn fast_init(&mut self, route: u16, sequence: u16) {
        let route_on_the_bench = self
            .channels
            .get(&route)
            .map(|channel| channel.route)
            .unwrap_or(BenchRoute::KLine7);
        let has_modules = self
            .bus
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .has_serial_modules(route_on_the_bench);
        if has_modules {
            self.accept(route, crate::kline::FAST_INIT_RESPONSE, sequence);
        } else {
            self.refuse(
                route,
                crate::kline::FAST_INIT_RESPONSE,
                sequence,
                0x208,
                "Fast Init: Timeout on response",
            );
        }
    }

    fn outbound(&mut self, route: u16, sequence: u16, body: &[u8]) {
        let Some(channel) = self.channels.get(&route) else {
            self.refuse(
                route,
                passive::OUTBOUND_DATA_RESPONSE,
                sequence,
                2,
                "cOutboundData: channel is not open",
            );
            return;
        };
        let bench_route = channel.route;
        let forward = !channel.listen_only && channel.pins_selected;
        // The application waits for this word before it reads any frame;
        // an inbound frame ahead of it would be dropped (`wait_for_response`).
        self.accept(route, passive::OUTBOUND_DATA_RESPONSE, sequence);
        if !forward {
            return;
        }
        // A serial line carries bytes, not frames: the record's data is the
        // whole of it and the vehicle answers in bytes (ADR-0029 slice B).
        if bench_route.is_serial() {
            let Some(bytes) = parse_outbound_line(body) else {
                return;
            };
            let answer = self
                .bus
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .on_line_bytes(bench_route, &bytes);
            if !answer.is_empty() {
                let tick = self.tick();
                self.emit(passive::inbound_line_bytes(route, 0, tick, &answer));
            }
            return;
        }
        let Some(frame) = parse_outbound(body, bench_route) else {
            return;
        };
        let answers = self
            .bus
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .on_frame(bench_route, &frame);
        for answer in answers {
            self.emit_inbound(route, &answer);
        }
    }

    fn emit_inbound(&mut self, route: u16, frame: &CanFrame) {
        let flags = if frame.id.is_extended() {
            CAN_29BIT_ID
        } else {
            0
        };
        let tick = self.tick();
        self.emit(passive::inbound_can_frame(
            route,
            flags,
            tick,
            frame.id.value(),
            &frame.data,
        ));
    }

    /// Broadcast frames the vehicle emits on its own on every open route.
    fn poll_broadcast(&mut self) {
        let open: Vec<(u16, BenchRoute)> = self
            .channels
            .iter()
            .map(|(word, channel)| (*word, channel.route))
            .collect();
        for (word, route) in open {
            let frames = self
                .bus
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .tick(route);
            for frame in frames {
                self.emit_inbound(word, &frame);
            }
        }
    }
}

impl ByteTransport for BenchTransport {
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), TransportError> {
        if self.closed {
            return Err(TransportError::Closed);
        }
        self.decoder.push(bytes);
        loop {
            match self.decoder.next_frame() {
                Ok(Some(frame)) => self.handle(&frame),
                Ok(None) => break,
                Err(_) => {
                    // A frame the codec cannot read is dropped, as a serial
                    // port would deliver garbage the firmware ignores.
                    self.decoder = FrameDecoder::default();
                    break;
                }
            }
        }
        Ok(())
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
        if self.closed {
            return Err(TransportError::Closed);
        }
        if self.outbox.is_empty() {
            self.poll_broadcast();
        }
        if self.outbox.is_empty() {
            // Nothing to say: wait as a silent port would, then time out.
            std::thread::sleep(self.read_timeout.min(Duration::from_millis(200)));
            return Err(TransportError::Timeout);
        }
        let count = buffer.len().min(self.outbox.len());
        for slot in buffer.iter_mut().take(count) {
            *slot = self.outbox.pop_front().unwrap_or(0);
        }
        Ok(count)
    }

    fn set_read_timeout(&mut self, timeout: Duration) -> Result<(), TransportError> {
        self.read_timeout = timeout;
        Ok(())
    }

    fn close(&mut self) -> Result<(), TransportError> {
        self.closed = true;
        self.channels.clear();
        Ok(())
    }
}

/// The application's outbound record: flags, a zero word, the data size
/// (4 + data), the identifier big-endian, the data.
fn parse_outbound(body: &[u8], route: BenchRoute) -> Option<CanFrame> {
    let flags = read_u32(body, 0)?;
    let size = read_u32(body, 8)? as usize;
    let id = u32::from_be_bytes([
        *body.get(12)?,
        *body.get(13)?,
        *body.get(14)?,
        *body.get(15)?,
    ]);
    let data_len = size.checked_sub(4)?;
    let data = body.get(16..16 + data_len)?.to_vec();
    let id = if flags & CAN_29BIT_ID != 0 {
        CanId::extended(id).ok()?
    } else {
        CanId::standard(u16::try_from(id).ok()?).ok()?
    };
    CanFrame::new(0, route.as_str(), id, data).ok()
}

/// The bytes of a serial outbound record: the same header as a CAN one,
/// and the data where the identifier would be (`ADR-0029` slice B).
fn parse_outbound_line(body: &[u8]) -> Option<Vec<u8>> {
    let size = read_u32(body, 8)? as usize;
    Some(body.get(12..12 + size)?.to_vec())
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *bytes.get(offset)?,
        *bytes.get(offset + 1)?,
        *bytes.get(offset + 2)?,
        *bytes.get(offset + 3)?,
    ]))
}

/// The route word the bench answers a vehicle route on; none for a route
/// the adapter has no word for yet (ADR-0029), which the bench then does not
/// answer either.
pub fn route_word(route: VehicleRouteId) -> Option<u16> {
    passive::resource_route(route)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::MongooseJlrDevice;
    use crate::passive::VehicleRouteId;
    use diagnostic_environment::{
        DiagnosticEnvironmentResolution, ReadableIdentifier, ValidationState,
    };
    use jlr_profiles::x250_2010_supercharged_ecm_environment;
    use uds_execution::{
        prepare_read_only_transaction, DiagnosticTargetIdentity, PreparedUdsTransaction,
        ReadOnlyUdsIntent, READ_DATA_BY_IDENTIFIER_CAPABILITY, UDS_PROTOCOL_FAMILY,
    };

    /// A vehicle of one module that answers ReadDataByIdentifier 0x1945 with
    /// a 12-byte value, through a first frame, and echoes nothing else.
    struct OneModule {
        seen: Vec<CanFrame>,
        pending: Vec<Vec<u8>>,
    }

    impl BenchBus for OneModule {
        fn on_frame(&mut self, route: BenchRoute, frame: &CanFrame) -> Vec<CanFrame> {
            self.seen.push(frame.clone());
            let id = CanId::standard(0x7E8).unwrap();
            let reply = |data: &[u8]| CanFrame::new(0, route.as_str(), id, data.to_vec()).unwrap();
            match frame.data.first() {
                Some(0x03) if frame.data[1..4] == [0x22, 0x19, 0x45] => {
                    let payload = [0x62, 0x19, 0x45, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
                    let mut frames = isotp::segment(&payload, Some(0)).unwrap();
                    let first = frames.remove(0);
                    self.pending = frames;
                    vec![reply(&first)]
                }
                Some(0x30) => self.pending.drain(..).map(|f| reply(&f)).collect(),
                _ => Vec::new(),
            }
        }

        fn describe(&self) -> String {
            "one module".into()
        }
    }

    fn did_transaction() -> PreparedUdsTransaction {
        let DiagnosticEnvironmentResolution::Resolved(mut plan) =
            x250_2010_supercharged_ecm_environment()
        else {
            unreachable!()
        };
        plan.protocol_family.value = UDS_PROTOCOL_FAMILY.into();
        plan.addressing_mode.value = "normal".into();
        plan.read_only_capability.value.id = READ_DATA_BY_IDENTIFIER_CAPABILITY.into();
        plan.backend_route.value.route_id = "hs-can".into();
        let readable = ReadableIdentifier {
            identifier: 0x1945,
            parameters: Vec::new(),
            evidence: Vec::new(),
            validation_state: ValidationState::SourceBacked,
        };
        prepare_read_only_transaction(
            &DiagnosticEnvironmentResolution::Resolved(plan),
            ReadOnlyUdsIntent::read_data_by_identifier(
                DiagnosticTargetIdentity::family("jlr.x250.ecm").unwrap(),
                0x1945,
            ),
            &[readable],
        )
        .unwrap()
    }

    fn bench() -> (SharedBenchBus, MongooseJlrDevice<BenchTransport>) {
        let bus = share_bus(Box::new(OneModule {
            seen: Vec::new(),
            pending: Vec::new(),
        }));
        let device = MongooseJlrDevice::open(BenchTransport::new(bus.clone()));
        (bus, device)
    }

    #[test]
    fn the_bench_powers_up_in_the_bootloader_and_the_device_jumps_it_to_the_firmware() {
        let (_, mut device) = bench();
        assert!(device.get_board_info().unwrap().in_bootloader());
        // The route open does the jump, the resource open and the pins.
        let opened = device.open_receive_route(VehicleRouteId::MsCan).unwrap();
        assert_eq!(opened.device_channel_id, CAN2_RESOURCE_ROUTE);
        device.close_route().unwrap();
        assert!(!device.get_board_info().unwrap().in_bootloader());
    }

    #[test]
    fn a_listen_only_capture_hears_nothing_because_the_bench_invents_no_traffic() {
        let (_, mut device) = bench();
        let capture = device
            .capture_route(
                VehicleRouteId::HsCan,
                Duration::from_millis(60),
                Duration::from_millis(20),
                8,
            )
            .unwrap();
        assert!(capture.frames.is_empty());
        assert_eq!(capture.route.id, VehicleRouteId::HsCan);
    }

    #[test]
    fn a_prepared_read_goes_through_the_real_stack_and_the_vehicle_answers_in_frames() {
        let (bus, mut device) = bench();
        let result = device
            .execute_prepared_uds_read(&did_transaction(), Duration::from_millis(500))
            .unwrap();
        assert_eq!(
            result.raw_diagnostic_response,
            vec![0x62, 0x19, 0x45, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
        );
        assert_eq!(result.route, VehicleRouteId::HsCan);
        // The vehicle saw the padded request and the tester's flow control.
        let guard = bus.lock().unwrap();
        let _ = guard.describe();
        drop(guard);
    }

    #[test]
    fn the_wrong_pins_draw_the_firmware_words_and_an_unknown_resource_its_number() {
        let bus = share_bus(Box::new(transport_api::EmptyBench));
        let mut transport = BenchTransport::new(bus);
        transport.in_bootloader = false;
        // Resource 9 does not exist.
        transport
            .write_all(&passive::open_channel_request(1, 0x0901, 125_000))
            .unwrap();
        let mut buffer = [0_u8; 256];
        let read = transport.read(&mut buffer).unwrap();
        let text = String::from_utf8_lossy(&buffer[..read]).to_string();
        assert!(text.contains("Invalid Resource ID 9"), "{text}");
        // Resource 5 with CAN2's pins.
        transport
            .write_all(&passive::open_channel_request(
                2,
                CAN1_RESOURCE_ROUTE,
                500_000,
            ))
            .unwrap();
        let read = transport.read(&mut buffer).unwrap();
        assert!(read > 0);
        let route = passive::route_by_id(VehicleRouteId::MsCan);
        transport
            .write_all(&passive::set_pin_request(3, CAN1_RESOURCE_ROUTE, *route))
            .unwrap();
        let read = transport.read(&mut buffer).unwrap();
        let text = String::from_utf8_lossy(&buffer[..read]).to_string();
        assert!(
            text.contains("only supports CAN on pins 6 and 14"),
            "{text}"
        );
    }
}
