use crate::{
    list_vehicle_routes, passive, OpenReceiveRoute, PassiveCapability, RawCanFrame, VehicleRoute,
    VehicleRouteId,
};
use crate::{BoardInfoResponse, FrameDecoder, MongoosePacket, ProtocolError, SequenceCounter};
use diagnostic_execution::{PreparedDiagnosticTransaction, TransactionSafetyClass};
use obd_j1979::{decode_calibration_identification, J1979Error, J1979Request};
use std::collections::VecDeque;
use std::fmt;
use std::time::{Duration, Instant};
use transport_api::{ByteTransport, CanId, TransportError};

const BOARD_INFO_READ_TIMEOUT: Duration = Duration::from_secs(1);
const COMMAND_READ_TIMEOUT: Duration = Duration::from_secs(1);
/// After the jump the firmware is polled with board-info until it answers
/// with a running tick; how long it takes to start was not measurable on
/// the bench (the adapter stays in firmware until it is unplugged), so the
/// transport waits for the fact rather than for a guessed delay.
const FIRMWARE_START_TIMEOUT: Duration = Duration::from_secs(5);
const FIRMWARE_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReceiveCounters {
    pub dropped_frames: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MongooseCalibrationIdentificationResult {
    pub responder: u32,
    pub request_payload: Vec<u8>,
    pub raw_diagnostic_response: Vec<u8>,
    pub calibration_ids: Vec<String>,
}

#[derive(Debug)]
pub enum MongooseDiagnosticError {
    UnsupportedTransaction(&'static str),
    CanConnection(ProtocolError),
    RequestTransmission(ProtocolError),
    ResponseReception(ProtocolError),
    ChannelClose(ProtocolError),
    IsoTp(isotp::IsoTpError),
    J1979(J1979Error),
    UnexpectedResponder { expected: u32, actual: u32 },
    Timeout,
}

impl fmt::Display for MongooseDiagnosticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTransaction(reason) => {
                write!(
                    formatter,
                    "unsupported prepared diagnostic transaction: {reason}"
                )
            }
            Self::CanConnection(error) => write!(formatter, "CAN connection failed: {error}"),
            Self::RequestTransmission(error) => {
                write!(formatter, "diagnostic request transmission failed: {error}")
            }
            Self::ResponseReception(error) => {
                write!(formatter, "diagnostic response reception failed: {error}")
            }
            Self::ChannelClose(error) => write!(formatter, "CAN channel close failed: {error}"),
            Self::IsoTp(error) => error.fmt(formatter),
            Self::J1979(error) => error.fmt(formatter),
            Self::UnexpectedResponder { expected, actual } => write!(
                formatter,
                "unexpected diagnostic responder: expected {expected:#05X}, got {actual:#05X}"
            ),
            Self::Timeout => formatter.write_str("no response from ECU before timeout"),
        }
    }
}

impl std::error::Error for MongooseDiagnosticError {}

impl From<isotp::IsoTpError> for MongooseDiagnosticError {
    fn from(value: isotp::IsoTpError) -> Self {
        Self::IsoTp(value)
    }
}

impl From<J1979Error> for MongooseDiagnosticError {
    fn from(value: J1979Error) -> Self {
        Self::J1979(value)
    }
}

pub struct MongooseJlrDevice<T: ByteTransport> {
    transport: T,
    decoder: FrameDecoder,
    sequences: SequenceCounter,
    closed: bool,
    /// Set once board-info has shown the firmware running (after a jump
    /// from the bootloader if that is where the adapter was).
    firmware_ready: bool,
    open_route: Option<OpenReceiveRoute>,
    pending_frames: VecDeque<RawCanFrame>,
    counters: ReceiveCounters,
}

impl<T: ByteTransport> MongooseJlrDevice<T> {
    /// Composes an already-open byte transport without sending any command.
    pub fn open(transport: T) -> Self {
        Self {
            transport,
            decoder: FrameDecoder::default(),
            sequences: SequenceCounter::default(),
            closed: false,
            firmware_ready: false,
            open_route: None,
            pending_frames: VecDeque::new(),
            counters: ReceiveCounters::default(),
        }
    }

    /// The adapter powers up in its bootloader, which refuses every channel
    /// command; board-info tells which state it is in, and `cJumpToFirmware`
    /// starts the stored firmware in place (ADR-0018). Done once per device
    /// session, before the first channel command.
    fn ensure_firmware(&mut self) -> Result<(), ProtocolError> {
        if self.firmware_ready {
            return Ok(());
        }
        let info = self.get_board_info()?;
        if info.in_bootloader() {
            let sequence = self.sequences.allocate();
            self.transport.set_read_timeout(COMMAND_READ_TIMEOUT)?;
            self.transport
                .write_all(&passive::jump_to_firmware_request(sequence))?;
            self.wait_for_response(|frame| passive::parse_jump_response(frame, sequence))?;
            let deadline = std::time::Instant::now() + FIRMWARE_START_TIMEOUT;
            loop {
                std::thread::sleep(FIRMWARE_POLL_INTERVAL);
                match self.get_board_info() {
                    Ok(info) if !info.in_bootloader() => break,
                    // Still the bootloader, or no answer yet while the
                    // firmware starts: ask again until the deadline.
                    Ok(_) | Err(ProtocolError::Transport(TransportError::Timeout)) => {}
                    Err(error) => return Err(error),
                }
                if std::time::Instant::now() >= deadline {
                    return Err(ProtocolError::FirmwareNotStarted);
                }
            }
        }
        self.firmware_ready = true;
        Ok(())
    }

    pub fn list_vehicle_routes(&self) -> &'static [VehicleRoute] {
        list_vehicle_routes()
    }

    /// Sends exactly one allowlisted cGetBoardInfo request and waits for one response.
    pub fn get_board_info(&mut self) -> Result<BoardInfoResponse, ProtocolError> {
        if self.closed {
            return Err(TransportError::Closed.into());
        }
        let sequence = self.sequences.allocate();
        let request = MongoosePacket::get_board_info(sequence)?.encode()?;
        self.transport.set_read_timeout(BOARD_INFO_READ_TIMEOUT)?;
        self.transport.write_all(&request)?;

        let mut read_buffer = [0_u8; 512];
        loop {
            if let Some(frame) = self.decoder.next_frame()? {
                return BoardInfoResponse::from_frame(&frame, sequence);
            }
            let read = self.transport.read(&mut read_buffer)?;
            if read == 0 {
                return Err(TransportError::Closed.into());
            }
            self.decoder.push(&read_buffer[..read]);
        }
    }

    pub fn open_receive_route(
        &mut self,
        route_id: VehicleRouteId,
    ) -> Result<OpenReceiveRoute, ProtocolError> {
        self.open_route_internal(route_id, true)
    }

    /// Executes the only F8 live-candidate operation. The wire-level CAN send
    /// primitives are private; callers must provide an F7-validated transaction.
    pub fn execute_prepared_calibration_identification(
        &mut self,
        transaction: &PreparedDiagnosticTransaction,
        timeout: Duration,
    ) -> Result<MongooseCalibrationIdentificationResult, MongooseDiagnosticError> {
        let route_id = validate_calibration_transaction(transaction)?;
        self.open_route_internal(route_id, false)
            .map_err(MongooseDiagnosticError::CanConnection)?;
        let result = self.execute_calibration_inner(transaction, timeout);
        let close = self.close_route();
        match (result, close) {
            (Ok(result), Ok(())) => Ok(result),
            (Ok(_), Err(error)) => Err(MongooseDiagnosticError::ChannelClose(error)),
            (Err(error), _) => Err(error),
        }
    }

    pub fn receive_frame(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<RawCanFrame>, ProtocolError> {
        self.require_open_transport()?;
        let open_route = self.open_route.ok_or(ProtocolError::NoOpenRoute)?;
        if let Some(frame) = self.pending_frames.pop_front() {
            return Ok(Some(frame));
        }
        self.transport.set_read_timeout(timeout)?;
        loop {
            if let Some(frame) = self.decoder.next_frame()? {
                if !passive::is_inbound_data(&frame) {
                    self.counters.dropped_frames += 1;
                    continue;
                }
                return passive::parse_inbound_can(&frame, open_route).map(Some);
            }
            let mut read_buffer = [0_u8; 512];
            match self.transport.read(&mut read_buffer) {
                Ok(0) => return Err(TransportError::Closed.into()),
                Ok(read) => self.decoder.push(&read_buffer[..read]),
                Err(TransportError::Timeout) => return Ok(None),
                Err(error) => return Err(error.into()),
            }
        }
    }

    pub fn close_route(&mut self) -> Result<(), ProtocolError> {
        self.require_open_transport()?;
        let open_route = self.open_route.ok_or(ProtocolError::NoOpenRoute)?;
        let sequence = self.sequences.allocate();
        self.transport.write_all(&passive::close_channel_request(
            sequence,
            open_route.device_channel_id,
        ))?;
        self.wait_for_response(|frame| {
            passive::parse_close_response(frame, sequence, open_route.device_channel_id)
        })?;
        self.open_route = None;
        Ok(())
    }

    pub fn receive_counters(&self) -> ReceiveCounters {
        self.counters
    }

    /// Closes the OS transport without sending a Mongoose cCloseDevice command.
    pub fn close(&mut self) -> Result<(), ProtocolError> {
        if self.open_route.is_some() {
            return Err(ProtocolError::ActiveRoute);
        }
        if !self.closed {
            self.transport.close()?;
            self.closed = true;
        }
        Ok(())
    }

    fn wait_for_response<R>(
        &mut self,
        parse: impl Fn(&crate::Frame) -> Result<R, ProtocolError>,
    ) -> Result<R, ProtocolError> {
        let mut read_buffer = [0_u8; 512];
        loop {
            if let Some(frame) = self.decoder.next_frame()? {
                if passive::is_inbound_data(&frame) {
                    self.counters.dropped_frames += 1;
                    continue;
                }
                return parse(&frame);
            }
            let read = self.transport.read(&mut read_buffer)?;
            if read == 0 {
                return Err(TransportError::Closed.into());
            }
            self.decoder.push(&read_buffer[..read]);
        }
    }

    pub(crate) fn open_route_internal(
        &mut self,
        route_id: VehicleRouteId,
        listen_only: bool,
    ) -> Result<OpenReceiveRoute, ProtocolError> {
        self.require_open_transport()?;
        if self.open_route.is_some() {
            return Err(ProtocolError::RouteAlreadyOpen);
        }
        let route = *passive::route_by_id(route_id);
        let PassiveCapability::Ready = route.passive_capability else {
            let PassiveCapability::Blocked(reason) = route.passive_capability else {
                unreachable!()
            };
            return Err(ProtocolError::PassiveRouteUnavailable {
                route: route.id.as_str(),
                reason,
            });
        };
        let bitrate = route
            .bitrate
            .expect("ready CAN route must have a confirmed bitrate");
        self.ensure_firmware()?;
        let route_word = passive::resource_route(route_id);

        let open_sequence = self.sequences.allocate();
        self.transport.set_read_timeout(COMMAND_READ_TIMEOUT)?;
        let open_request = if listen_only {
            passive::open_channel_request(open_sequence, route_word, bitrate)
        } else {
            passive::diagnostic_open_channel_request(open_sequence, route_word, bitrate)
        };
        self.transport.write_all(&open_request)?;
        let open_response = self.wait_for_response(|frame| {
            passive::parse_open_response(frame, open_sequence, route_word)
        })?;
        let opened = OpenReceiveRoute {
            route,
            device_channel_id: open_response.channel_id,
        };

        let pin_sequence = self.sequences.allocate();
        let set_pin_request =
            passive::set_pin_request(pin_sequence, opened.device_channel_id, opened.route);
        if let Err(error) = self.transport.write_all(&set_pin_request) {
            let _ = self.close_channel_after_failed_setup(opened.device_channel_id);
            return Err(error.into());
        }
        match self.wait_for_response(|frame| {
            passive::parse_set_pin_response(frame, pin_sequence, opened.device_channel_id)
        }) {
            Ok(_) => {
                self.open_route = Some(opened);
                Ok(opened)
            }
            Err(error) => {
                let _ = self.close_channel_after_failed_setup(opened.device_channel_id);
                Err(error)
            }
        }
    }

    fn execute_calibration_inner(
        &mut self,
        transaction: &PreparedDiagnosticTransaction,
        timeout: Duration,
    ) -> Result<MongooseCalibrationIdentificationResult, MongooseDiagnosticError> {
        let request_payload = transaction.protocol_request().encoded().to_vec();
        let request_frames = isotp::segment(&request_payload, None)?;
        let [request_frame] = request_frames.as_slice() else {
            return Err(MongooseDiagnosticError::UnsupportedTransaction(
                "Calibration Identification request must be one ISO-TP single frame",
            ));
        };
        let request_id = standard_id(transaction.physical_request_id())?;
        let expected_response = standard_id(transaction.expected_response_id())?;
        self.transmit_diagnostic_frame(CanId::Standard(request_id), request_frame)
            .map_err(MongooseDiagnosticError::RequestTransmission)?;

        let started = Instant::now();
        let mut reassembler = isotp::Reassembler::new();
        loop {
            let Some(remaining) = timeout.checked_sub(started.elapsed()) else {
                return Err(MongooseDiagnosticError::Timeout);
            };
            let Some(frame) = self
                .receive_frame(remaining)
                .map_err(MongooseDiagnosticError::ResponseReception)?
            else {
                return Err(MongooseDiagnosticError::Timeout);
            };
            if frame.arbitration_id != u32::from(expected_response) {
                if (0x7e8..=0x7ef).contains(&frame.arbitration_id) {
                    return Err(MongooseDiagnosticError::UnexpectedResponder {
                        expected: u32::from(expected_response),
                        actual: frame.arbitration_id,
                    });
                }
                continue;
            }

            let first_frame = matches!(
                isotp::decode_frame(frame.payload())?,
                isotp::IsoTpFrame::First { .. }
            );
            let completed =
                reassembler.accept(frame.payload(), started.elapsed().as_micros() as u64)?;
            if first_frame {
                self.transmit_diagnostic_frame(CanId::Standard(request_id), &[0x30, 0x00, 0x00])
                    .map_err(MongooseDiagnosticError::RequestTransmission)?;
            }
            if let Some(raw_response) = completed {
                let decoded = decode_calibration_identification(
                    transaction.protocol_request(),
                    &raw_response,
                )?;
                return Ok(MongooseCalibrationIdentificationResult {
                    responder: u32::from(expected_response),
                    request_payload,
                    raw_diagnostic_response: raw_response,
                    calibration_ids: decoded.calibration_ids,
                });
            }
        }
    }

    pub(crate) fn transmit_diagnostic_frame(
        &mut self,
        arbitration_id: CanId,
        payload: &[u8],
    ) -> Result<(), ProtocolError> {
        let open_route = self.open_route.ok_or(ProtocolError::NoOpenRoute)?;
        let sequence = self.sequences.allocate();
        let request = passive::outbound_data_request(
            sequence,
            open_route.device_channel_id,
            arbitration_id.value(),
            arbitration_id.is_extended(),
            payload,
        );
        self.transport.write_all(&request)?;
        self.wait_for_outbound_response(sequence, open_route)?;
        Ok(())
    }

    fn wait_for_outbound_response(
        &mut self,
        sequence: u16,
        open_route: OpenReceiveRoute,
    ) -> Result<(), ProtocolError> {
        let mut read_buffer = [0_u8; 512];
        loop {
            if let Some(frame) = self.decoder.next_frame()? {
                if passive::is_inbound_data(&frame) {
                    self.pending_frames
                        .push_back(passive::parse_inbound_can(&frame, open_route)?);
                    continue;
                }
                passive::parse_outbound_response(&frame, sequence, open_route.device_channel_id)?;
                return Ok(());
            }
            let read = self.transport.read(&mut read_buffer)?;
            if read == 0 {
                return Err(TransportError::Closed.into());
            }
            self.decoder.push(&read_buffer[..read]);
        }
    }

    fn close_channel_after_failed_setup(&mut self, channel_id: u16) -> Result<(), ProtocolError> {
        let sequence = self.sequences.allocate();
        self.transport
            .write_all(&passive::close_channel_request(sequence, channel_id))?;
        self.wait_for_response(|frame| passive::parse_close_response(frame, sequence, channel_id))?;
        Ok(())
    }

    fn require_open_transport(&self) -> Result<(), ProtocolError> {
        if self.closed {
            Err(TransportError::Closed.into())
        } else {
            Ok(())
        }
    }
}

pub(crate) fn standard_id(id: CanId) -> Result<u16, MongooseDiagnosticError> {
    match id {
        CanId::Standard(value) => Ok(value),
        CanId::Extended(_) => Err(MongooseDiagnosticError::UnsupportedTransaction(
            "F8 requires 11-bit CAN identifiers",
        )),
    }
}

fn validate_calibration_transaction(
    transaction: &PreparedDiagnosticTransaction,
) -> Result<VehicleRouteId, MongooseDiagnosticError> {
    if transaction.safety_class() != TransactionSafetyClass::ReadOnly {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "safety class is not READ_ONLY",
        ));
    }
    if transaction.protocol_request() != J1979Request::CalibrationIdentification {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "operation is not Calibration Identification",
        ));
    }
    if transaction.capability_id() != "obd.service09.infotype04.calibration_id.read_only" {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "capability is not the allowlisted Mode 09 InfoType 04 read",
        ));
    }
    if transaction.protocol_family() != "ISO15765-4 / SAE J1979"
        || transaction.addressing_mode() != "normal_physical"
    {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "protocol or addressing mode is outside F8",
        ));
    }
    if transaction.backend_route() != VehicleRouteId::HsCan.as_str() {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "F8 profile requires the confirmed hs-can route",
        ));
    }
    let route = passive::route_by_id(VehicleRouteId::HsCan);
    if transaction.physical_pins() != route.obd_pins
        || Some(transaction.bitrate_bps()) != route.bitrate
    {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "prepared route does not match the Mongoose HS-CAN descriptor",
        ));
    }
    standard_id(transaction.physical_request_id())?;
    standard_id(transaction.expected_response_id())?;
    Ok(VehicleRouteId::HsCan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::encode_board_info_response;
    use crate::passive::{
        command_response, inbound_can_frame, CAN1_RESOURCE_ROUTE, CAN2_RESOURCE_ROUTE,
        CLOSE_CHANNEL_RESPONSE, DEVICE_ROUTE, JUMP_TO_FIRMWARE_RESPONSE, OPEN_CHANNEL_RESPONSE,
        OUTBOUND_DATA_RESPONSE, SET_PIN_RESPONSE,
    };

    /// Board-info as the running firmware answers it: a non-zero tick at
    /// body offset 4. The first exchange of every route open.
    fn firmware_board_info(sequence: u16) -> Vec<u8> {
        encode_board_info_response(sequence, &[0, 0, 0, 0, 0x10, 0x20, 0x30, 0x40, 5, 1, 1, 1])
    }

    /// Board-info as the bootloader answers it: zeros where the tick goes.
    fn bootloader_board_info(sequence: u16) -> Vec<u8> {
        encode_board_info_response(sequence, &[0, 0, 0, 0, 0, 0, 0, 0, 5, 2, 1, 1])
    }
    use diagnostic_execution::{
        prepare_read_only_transaction, DiagnosticTargetIdentity, ReadOnlyDiagnosticIntent,
    };
    use jlr_profiles::{
        x250_2010_supercharged_ecm_environment, DIAGNOSTIC_IMPLEMENTATION, ECU_FAMILY,
        OBSERVED_CALIBRATION_ID,
    };
    use obd_j1979::{encode_calibration_identification_response, CalibrationId};
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct MockState {
        writes: Vec<Vec<u8>>,
        timeout: Option<Duration>,
        closes: usize,
    }

    struct MockTransport {
        state: Arc<Mutex<MockState>>,
        reads: VecDeque<Vec<u8>>,
    }

    impl MockTransport {
        fn new(reads: Vec<Vec<u8>>) -> (Self, Arc<Mutex<MockState>>) {
            let state = Arc::new(Mutex::new(MockState::default()));
            (
                Self {
                    state: Arc::clone(&state),
                    reads: reads.into(),
                },
                state,
            )
        }
    }

    impl ByteTransport for MockTransport {
        fn write_all(&mut self, bytes: &[u8]) -> Result<(), TransportError> {
            self.state.lock().unwrap().writes.push(bytes.to_vec());
            Ok(())
        }

        fn read(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
            let bytes = self.reads.pop_front().ok_or(TransportError::Timeout)?;
            let count = bytes.len().min(buffer.len());
            buffer[..count].copy_from_slice(&bytes[..count]);
            Ok(count)
        }

        fn set_read_timeout(&mut self, timeout: Duration) -> Result<(), TransportError> {
            self.state.lock().unwrap().timeout = Some(timeout);
            Ok(())
        }

        fn close(&mut self) -> Result<(), TransportError> {
            self.state.lock().unwrap().closes += 1;
            Ok(())
        }
    }

    #[test]
    fn open_performs_no_implicit_application_write() {
        let (transport, state) = MockTransport::new(vec![]);
        let _device = MongooseJlrDevice::open(transport);
        assert!(state.lock().unwrap().writes.is_empty());
    }

    #[test]
    fn board_info_uses_one_request_and_accepts_fragmented_response() {
        let response = encode_board_info_response(1, &[0xAA, 0xBB, 0xCC]);
        let fragments = response.chunks(3).map(<[u8]>::to_vec).collect();
        let (transport, state) = MockTransport::new(fragments);
        let mut device = MongooseJlrDevice::open(transport);
        let info = device.get_board_info().unwrap();

        assert_eq!(info.response_command, 0x8109);
        assert_eq!(info.raw_board_info, [0xAA, 0xBB, 0xCC]);
        let state = state.lock().unwrap();
        assert_eq!(state.writes.len(), 1);
        assert_eq!(
            state.writes[0],
            [
                0x0C, 0x00, 0xEA, 0x51, 0x01, 0x00, 0x00, 0x00, 0x09, 0x01, 0x01, 0x00, 0x00, 0x00,
                0x00, 0x00
            ]
        );
    }

    #[test]
    fn sequence_match_is_required() {
        let response = encode_board_info_response(2, &[]);
        let (transport, _) = MockTransport::new(vec![response]);
        let mut device = MongooseJlrDevice::open(transport);
        assert_eq!(
            device.get_board_info(),
            Err(ProtocolError::SequenceMismatch {
                expected: 1,
                actual: 2
            })
        );
    }

    #[test]
    fn response_command_must_match_board_info() {
        let mut response = encode_board_info_response(1, &[]);
        response[8] = 0x08;
        let (transport, _) = MockTransport::new(vec![response]);
        let mut device = MongooseJlrDevice::open(transport);
        assert_eq!(
            device.get_board_info(),
            Err(ProtocolError::UnexpectedResponse {
                opcode: 0x08,
                flag: 0x81
            })
        );
    }

    #[test]
    fn close_is_transport_only_and_idempotent() {
        let (transport, state) = MockTransport::new(vec![]);
        let mut device = MongooseJlrDevice::open(transport);
        device.close().unwrap();
        device.close().unwrap();
        let state = state.lock().unwrap();
        assert!(state.writes.is_empty());
        assert_eq!(state.closes, 1);
    }

    #[test]
    fn passive_open_receive_close_and_reopen_are_production_path() {
        let responses = vec![
            firmware_board_info(1),
            command_response(CAN1_RESOURCE_ROUTE, OPEN_CHANNEL_RESPONSE, 2, 0),
            command_response(CAN1_RESOURCE_ROUTE, SET_PIN_RESPONSE, 3, 0),
            inbound_can_frame(CAN1_RESOURCE_ROUTE, 0, 1234, 0x321, &[1, 2, 3]),
            command_response(CAN1_RESOURCE_ROUTE, CLOSE_CHANNEL_RESPONSE, 4, 0),
            command_response(CAN2_RESOURCE_ROUTE, OPEN_CHANNEL_RESPONSE, 5, 0),
            command_response(CAN2_RESOURCE_ROUTE, SET_PIN_RESPONSE, 6, 0),
        ];
        let (transport, state) = MockTransport::new(responses);
        let mut device = MongooseJlrDevice::open(transport);

        let opened = device.open_receive_route(VehicleRouteId::HsCan).unwrap();
        assert_eq!(opened.device_channel_id, CAN1_RESOURCE_ROUTE);
        let frame = device
            .receive_frame(Duration::from_millis(10))
            .unwrap()
            .unwrap();
        assert_eq!(frame.arbitration_id, 0x321);
        assert_eq!(frame.payload(), [1, 2, 3]);
        device.close_route().unwrap();

        let reopened = device.open_receive_route(VehicleRouteId::MsCan).unwrap();
        assert_eq!(reopened.device_channel_id, CAN2_RESOURCE_ROUTE);
        let writes = &state.lock().unwrap().writes;
        // Board-info once, then open, pins, close, open, pins.
        assert_eq!(writes.len(), 6);
        assert_eq!(&writes[0][8..10], &[0x09, 0x01]);
        assert_eq!(&writes[1][4..6], &CAN1_RESOURCE_ROUTE.to_le_bytes());
        assert_eq!(&writes[1][16..20], &0x1000_0000_u32.to_le_bytes());
        assert_eq!(&writes[1][20..24], &500_000_u32.to_le_bytes());
        assert_eq!(&writes[2][20..24], &6_u32.to_le_bytes());
        assert_eq!(&writes[2][24..28], &14_u32.to_le_bytes());
        assert_eq!(&writes[4][4..6], &CAN2_RESOURCE_ROUTE.to_le_bytes());
        assert_eq!(&writes[4][20..24], &125_000_u32.to_le_bytes());
        assert_eq!(&writes[5][20..24], &3_u32.to_le_bytes());
        assert_eq!(&writes[5][24..28], &11_u32.to_le_bytes());
    }

    #[test]
    fn a_bootloader_answer_is_followed_by_the_firmware_jump() {
        let responses = vec![
            bootloader_board_info(1),
            command_response(DEVICE_ROUTE, JUMP_TO_FIRMWARE_RESPONSE, 2, 0),
            // The firmware is still starting: the first poll gets the
            // bootloader's answer again, the second the firmware's.
            bootloader_board_info(3),
            firmware_board_info(4),
            command_response(CAN1_RESOURCE_ROUTE, OPEN_CHANNEL_RESPONSE, 5, 0),
            command_response(CAN1_RESOURCE_ROUTE, SET_PIN_RESPONSE, 6, 0),
        ];
        let (transport, state) = MockTransport::new(responses);
        let mut device = MongooseJlrDevice::open(transport);
        device.open_receive_route(VehicleRouteId::HsCan).unwrap();
        let writes = &state.lock().unwrap().writes;
        assert_eq!(writes.len(), 6);
        // The jump: device route, command word 0x0103 (wire 03 01), no body.
        assert_eq!(&writes[1][4..6], &DEVICE_ROUTE.to_le_bytes());
        assert_eq!(&writes[1][8..10], &[0x03, 0x01]);
        assert_eq!(writes[1].len(), 16);
        assert_eq!(&writes[2][8..10], &[0x09, 0x01]);
        assert_eq!(&writes[3][8..10], &[0x09, 0x01]);
        assert_eq!(&writes[4][8..10], &[0x06, 0x00]);
    }

    #[test]
    fn a_firmware_that_never_starts_is_an_error_not_a_channel_command() {
        let responses = vec![
            bootloader_board_info(1),
            command_response(DEVICE_ROUTE, JUMP_TO_FIRMWARE_RESPONSE, 2, 0),
        ];
        let (transport, state) = MockTransport::new(responses);
        let mut device = MongooseJlrDevice::open(transport);
        let error = device
            .open_receive_route(VehicleRouteId::HsCan)
            .unwrap_err();
        assert_eq!(error, ProtocolError::FirmwareNotStarted);
        // Only board-info requests after the jump; no channel command was sent.
        let writes = &state.lock().unwrap().writes;
        assert!(writes.len() > 2);
        assert!(writes[2..].iter().all(|write| write[8..10] == [0x09, 0x01]));
    }

    #[test]
    fn a_refusal_carries_the_firmware_words() {
        let mut refusal = command_response(CAN1_RESOURCE_ROUTE, SET_PIN_RESPONSE, 3, 0x206);
        // Rebuild with the firmware's text after status and tick.
        let mut payload = refusal.split_off(4);
        payload.truncate(16);
        payload.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
        payload.extend_from_slice(
            b"SetPins: MongoosePro JLR board only supports CAN on pins 6 and 14\0",
        );
        let length = payload.len() as u16;
        refusal = length.to_le_bytes().to_vec();
        refusal.extend_from_slice(&(length ^ 0x51E6).to_le_bytes());
        refusal.extend_from_slice(&payload);
        let responses = vec![
            firmware_board_info(1),
            command_response(CAN1_RESOURCE_ROUTE, OPEN_CHANNEL_RESPONSE, 2, 0),
            refusal,
            command_response(CAN1_RESOURCE_ROUTE, CLOSE_CHANNEL_RESPONSE, 4, 0),
        ];
        let (transport, _) = MockTransport::new(responses);
        let mut device = MongooseJlrDevice::open(transport);
        let error = device
            .open_receive_route(VehicleRouteId::HsCan)
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "cSetPin failed with device status 0x00000206: SetPins: MongoosePro JLR board only supports CAN on pins 6 and 14"
        );
    }

    #[test]
    fn fragmented_and_coalesced_serial_reads_are_supported() {
        let open = command_response(CAN1_RESOURCE_ROUTE, OPEN_CHANNEL_RESPONSE, 2, 0);
        let pin = command_response(CAN1_RESOURCE_ROUTE, SET_PIN_RESPONSE, 3, 0);
        let inbound_a = inbound_can_frame(CAN1_RESOURCE_ROUTE, 0, 1, 0x100, &[]);
        let inbound_b = inbound_can_frame(CAN1_RESOURCE_ROUTE, 0, 2, 0x101, &[0xAA]);
        let mut first_read = firmware_board_info(1);
        first_read.extend_from_slice(&open);
        first_read.extend_from_slice(&pin);
        first_read.extend_from_slice(&inbound_a[..7]);
        let mut second_read = inbound_a[7..].to_vec();
        second_read.extend_from_slice(&inbound_b);
        let (transport, _) = MockTransport::new(vec![first_read, second_read]);
        let mut device = MongooseJlrDevice::open(transport);
        device.open_receive_route(VehicleRouteId::HsCan).unwrap();
        assert_eq!(
            device
                .receive_frame(Duration::from_millis(10))
                .unwrap()
                .unwrap()
                .arbitration_id,
            0x100
        );
        assert_eq!(
            device
                .receive_frame(Duration::from_millis(10))
                .unwrap()
                .unwrap()
                .arbitration_id,
            0x101
        );
    }

    #[test]
    fn typed_prepared_transaction_drives_fixed_multiframe_calibration_exchange() {
        let target = DiagnosticTargetIdentity::new(ECU_FAMILY, DIAGNOSTIC_IMPLEMENTATION).unwrap();
        let transaction = prepare_read_only_transaction(
            &x250_2010_supercharged_ecm_environment(),
            ReadOnlyDiagnosticIntent::calibration_identification(target),
        )
        .unwrap();
        let response = encode_calibration_identification_response(&[CalibrationId::new(
            OBSERVED_CALIBRATION_ID,
        )
        .unwrap()])
        .unwrap();
        let frames = isotp::segment(&response, None).unwrap();
        assert_eq!(frames.len(), 3);

        let reads = vec![
            firmware_board_info(1),
            command_response(CAN1_RESOURCE_ROUTE, OPEN_CHANNEL_RESPONSE, 2, 0),
            command_response(CAN1_RESOURCE_ROUTE, SET_PIN_RESPONSE, 3, 0),
            command_response(CAN1_RESOURCE_ROUTE, OUTBOUND_DATA_RESPONSE, 4, 0),
            inbound_can_frame(CAN1_RESOURCE_ROUTE, 0, 10, 0x7e8, &frames[0]),
            command_response(CAN1_RESOURCE_ROUTE, OUTBOUND_DATA_RESPONSE, 5, 0),
            inbound_can_frame(CAN1_RESOURCE_ROUTE, 0, 20, 0x7e8, &frames[1]),
            inbound_can_frame(CAN1_RESOURCE_ROUTE, 0, 30, 0x7e8, &frames[2]),
            command_response(CAN1_RESOURCE_ROUTE, CLOSE_CHANNEL_RESPONSE, 6, 0),
        ];
        let (transport, state) = MockTransport::new(reads);
        let mut device = MongooseJlrDevice::open(transport);
        let result = device
            .execute_prepared_calibration_identification(&transaction, Duration::from_millis(250))
            .unwrap();

        assert_eq!(result.request_payload, [0x09, 0x04]);
        assert_eq!(result.responder, 0x7e8);
        assert_eq!(result.raw_diagnostic_response, response);
        assert_eq!(result.calibration_ids, [OBSERVED_CALIBRATION_ID]);

        let writes = &state.lock().unwrap().writes;
        assert_eq!(writes.len(), 6);
        assert_eq!(&writes[1][16..20], &0_u32.to_le_bytes());
        assert_eq!(&writes[2][20..24], &6_u32.to_le_bytes());
        assert_eq!(&writes[2][24..28], &14_u32.to_le_bytes());
        assert!(writes[3].ends_with(&[0x00, 0x00, 0x07, 0xE0, 0x02, 0x09, 0x04]));
        assert!(writes[4].ends_with(&[0x00, 0x00, 0x07, 0xE0, 0x30, 0x00, 0x00]));
    }
}
