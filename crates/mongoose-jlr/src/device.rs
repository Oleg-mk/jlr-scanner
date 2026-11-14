use crate::status::status_words;
use crate::{
    list_vehicle_routes, passive, CanIdFormat, NetworkType, OpenReceiveRoute, PassiveCapability,
    RawCanFrame, VehicleRoute, VehicleRouteId,
};
use crate::{BoardInfoResponse, FrameDecoder, MongoosePacket, ProtocolError, SequenceCounter};
use diagnostic_execution::{PreparedDiagnosticTransaction, TransactionSafetyClass};
use obd_j1979::{decode_calibration_identification, J1979Error, J1979Request};
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::time::{Duration, Instant};
use transport_api::{ByteTransport, CanId, TransportError};

const BOARD_INFO_READ_TIMEOUT: Duration = Duration::from_secs(1);
const COMMAND_READ_TIMEOUT: Duration = Duration::from_secs(1);
/// How long a transmit's acknowledgement is waited for (ADR-0042, decision
/// 3). The firmware has acknowledged every transmit at once — in a car and
/// on a bare adapter alike (the gate of 2026-11-13: 0 ms); the "three
/// seconds" the bench of 2026-09-08 seemed to show were the probe's own
/// collection window, as the firmware's ticks in that transcript say. Five
/// seconds is an upper bound a late acknowledgement cannot miss, so that it
/// never meets the next command as a stray frame.
pub const TRANSMIT_ACK_TIMEOUT: Duration = Duration::from_secs(5);
/// After the jump the firmware is polled with board-info until it answers
/// with a running tick; how long it takes to start was not measurable on
/// the bench (the adapter stays in firmware until it is unplugged), so the
/// transport waits for the fact rather than for a guessed delay.
const FIRMWARE_START_TIMEOUT: Duration = Duration::from_secs(5);
const FIRMWARE_POLL_INTERVAL: Duration = Duration::from_millis(100);
/// A read is never asked to wait less than this: a zero timeout means "do
/// not wait" to a serial driver, and a deadline a few microseconds away is
/// still a deadline.
const MINIMUM_READ_TIMEOUT: Duration = Duration::from_millis(1);

/// The adapter's own answer to a transmit (ADR-0042, decision 1): recorded as
/// it came, with the firmware's words when there were any, and the time the
/// acknowledgement took — milliseconds on a live bus, seconds on a dead one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TransmitAcknowledgement {
    pub status: u32,
    pub message: String,
    pub waited_ms: u64,
}

/// A frame the adapter sent that no wait was for: a `cIndication`, or the
/// answer to an earlier command arriving late. Its identity is kept so the
/// next session file says what the first two could not (ADR-0042).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnsolicitedFrame {
    pub command: u16,
    pub sequence: u16,
    /// The word at the status position, when the record was long enough.
    pub status: Option<u32>,
}

impl fmt::Display for UnsolicitedFrame {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:#06X} sequence {}",
            self.command, self.sequence
        )?;
        match self.status {
            Some(status) => write!(formatter, " status {}", status_words(status)),
            None => formatter.write_str(" (no status word)"),
        }
    }
}

/// One pass entry put on a CAN channel after its pins (ADR-0043, decision 5):
/// the identifier format it passes, the firmware's status word and words, and
/// the four bytes an accepted entry's answer carries — their meaning unknown,
/// kept as they came.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiveFilterEntry {
    pub format: CanIdFormat,
    pub status: u32,
    pub message: String,
    pub word: Option<u32>,
}

impl ReceiveFilterEntry {
    /// Accepted: status zero and no words, as the bare adapter answered.
    pub fn accepted(&self) -> bool {
        self.status == 0 && self.message.is_empty()
    }
}

impl fmt::Display for ReceiveFilterEntry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} pass {}",
            format_words(self.format),
            status_words(self.status)
        )?;
        if let Some(word) = self.word {
            write!(formatter, " (word {word:#010X})")?;
        }
        if !self.message.is_empty() {
            write!(formatter, ": {}", self.message)?;
        }
        Ok(())
    }
}

/// An identifier format as the records say it.
pub(crate) fn format_words(format: CanIdFormat) -> &'static str {
    match format {
        CanIdFormat::Standard => "11-bit",
        CanIdFormat::Extended => "29-bit",
    }
}

/// What the adapter did around one operation, counted where the frames pass,
/// from the open to the close (ADR-0042, decision 4). Every result carries
/// it, and it can be read after a failure; the shell writes it into every
/// read's record.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExchangeTrace {
    /// The pass entries the channel got before it listened, in order
    /// (ADR-0043): accepted ones, and the refused one that ended the open.
    pub receive_filters: Vec<ReceiveFilterEntry>,
    /// Outbound records acknowledged.
    pub transmits: u32,
    pub last_acknowledgement: Option<TransmitAcknowledgement>,
    /// `cInboundData` records the adapter delivered, kept or not.
    pub inbound_frames: u64,
    /// `cIndication` records, and the last of them.
    pub indications: u32,
    pub last_indication: Option<UnsolicitedFrame>,
    /// The same indications counted by their status word (ADR-0043,
    /// decision 5), so a lost-frame word (`0x119` iMsgRxQueueOvflw) shows
    /// even when the last indication is a completed transmit.
    pub indications_by_status: BTreeMap<Option<u32>, u32>,
    /// Command-class frames that were not the answer awaited, and the last.
    pub stale_frames: u32,
    pub last_stale_frame: Option<UnsolicitedFrame>,
    /// Frames dropped: too short for a command header, an inbound record on
    /// another channel, one the route could not parse while no read wanted
    /// it.
    pub dropped_frames: u64,
    /// Bytes the codec could not frame, dropped one at a time.
    pub dropped_bytes: u64,
    /// The close's failure in words, when the route's close failed.
    pub close_error: Option<String>,
}

impl fmt::Display for ExchangeTrace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<String> = Vec::new();
        if !self.receive_filters.is_empty() {
            parts.push(format!(
                "receive filter {}",
                self.receive_filters
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        match &self.last_acknowledgement {
            Some(acknowledgement) => {
                let mut text = format!(
                    "transmit acknowledged with status {} after {} ms",
                    status_words(acknowledgement.status),
                    acknowledgement.waited_ms
                );
                if !acknowledgement.message.is_empty() {
                    text.push_str(": ");
                    text.push_str(&acknowledgement.message);
                }
                if self.transmits > 1 {
                    text.push_str(&format!(" ({} transmits)", self.transmits));
                }
                parts.push(text);
            }
            None => parts.push("no transmit acknowledged".into()),
        }
        parts.push(format!("{} inbound frames", self.inbound_frames));
        if let Some(indication) = &self.last_indication {
            let by_status = self
                .indications_by_status
                .iter()
                .map(|(status, count)| match status {
                    Some(status) => format!("{count} with {}", status_words(*status)),
                    None => format!("{count} with no status word"),
                })
                .collect::<Vec<_>>()
                .join(", ");
            if by_status.is_empty() {
                parts.push(format!(
                    "{} indication(s), last {indication}",
                    self.indications
                ));
            } else {
                parts.push(format!(
                    "{} indication(s), {by_status}, last {indication}",
                    self.indications
                ));
            }
        }
        if let Some(stale) = &self.last_stale_frame {
            parts.push(format!(
                "{} stale frame(s), last {stale}",
                self.stale_frames
            ));
        }
        if self.dropped_frames > 0 {
            parts.push(format!("{} frame(s) dropped", self.dropped_frames));
        }
        if self.dropped_bytes > 0 {
            parts.push(format!("{} byte(s) dropped", self.dropped_bytes));
        }
        if let Some(close) = &self.close_error {
            parts.push(format!("close failed: {close}"));
        }
        formatter.write_str(&parts.join("; "))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MongooseCalibrationIdentificationResult {
    pub responder: u32,
    pub request_payload: Vec<u8>,
    pub raw_diagnostic_response: Vec<u8>,
    pub calibration_ids: Vec<String>,
    pub trace: ExchangeTrace,
}

#[derive(Debug)]
pub enum MongooseDiagnosticError {
    UnsupportedTransaction(&'static str),
    CanConnection(ProtocolError),
    RequestTransmission(ProtocolError),
    ResponseReception(ProtocolError),
    /// Kept for the shell's mapping; since ADR-0042 an answer that was
    /// received stands when the close after it fails, and the close's error
    /// travels in the trace instead.
    ChannelClose(ProtocolError),
    IsoTp(isotp::IsoTpError),
    J1979(J1979Error),
    UnexpectedResponder {
        expected: u32,
        actual: u32,
    },
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

/// One frame from the adapter, by its command word.
enum Incoming {
    /// `cInboundData`: a frame from the bus, for the open route.
    Inbound(crate::Frame),
    /// `cIndication`: the adapter's own word, recorded and waited on.
    Indication(UnsolicitedFrame),
    /// The answer to some command: the awaited one, or a stale one.
    Command {
        command: u16,
        sequence: u16,
        status: Option<u32>,
        frame: crate::Frame,
    },
    /// Shorter than a command header: dropped and counted.
    Short,
}

fn classify(frame: crate::Frame) -> Incoming {
    let payload = frame.payload();
    if payload.len() < passive::COMMAND_HEADER_LENGTH {
        return Incoming::Short;
    }
    let command = passive::read_u16(payload, 4).expect("header length checked");
    let sequence = passive::read_u16(payload, 6).expect("header length checked");
    let status = passive::read_u32(payload, passive::COMMAND_HEADER_LENGTH);
    match command {
        passive::INBOUND_DATA => Incoming::Inbound(frame),
        passive::INDICATION => Incoming::Indication(UnsolicitedFrame {
            command,
            sequence,
            status,
        }),
        _ => Incoming::Command {
            command,
            sequence,
            status,
            frame,
        },
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
    /// A channel a failed close left on the adapter: closed again before
    /// the next channel is asked for, so that the adapter holds one at a
    /// time and a close that went unanswered does not block the session.
    stale_channel: Option<u16>,
    /// Frames of the open route that arrived while a command's answer was
    /// awaited; the next receive hands them out first. Cleared at every open.
    pending_frames: VecDeque<RawCanFrame>,
    trace: ExchangeTrace,
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
            stale_channel: None,
            pending_frames: VecDeque::new(),
            trace: ExchangeTrace::default(),
        }
    }

    /// What the adapter did around the last operation — or around this one,
    /// while it runs (ADR-0042, decision 4).
    pub fn exchange_trace(&self) -> &ExchangeTrace {
        &self.trace
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
            let request = passive::jump_to_firmware_request(sequence);
            self.command_exchange(
                &request,
                passive::JUMP_TO_FIRMWARE_RESPONSE,
                sequence,
                COMMAND_READ_TIMEOUT,
                |frame| passive::parse_jump_response(frame, sequence),
            )?;
            let deadline = Instant::now() + FIRMWARE_START_TIMEOUT;
            loop {
                std::thread::sleep(FIRMWARE_POLL_INTERVAL);
                match self.get_board_info() {
                    Ok(info) if !info.in_bootloader() => break,
                    // Still the bootloader, or no answer yet while the
                    // firmware starts: ask again until the deadline.
                    Ok(_) | Err(ProtocolError::Transport(TransportError::Timeout)) => {}
                    Err(error) => return Err(error),
                }
                if Instant::now() >= deadline {
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

    /// Sends exactly one allowlisted cGetBoardInfo request and waits for its
    /// answer — its own, by command word and sequence; a frame an earlier
    /// session left in the stream is skipped and counted.
    pub fn get_board_info(&mut self) -> Result<BoardInfoResponse, ProtocolError> {
        if self.closed {
            return Err(TransportError::Closed.into());
        }
        let sequence = self.sequences.allocate();
        let request = MongoosePacket::get_board_info(sequence)?.encode()?;
        self.command_exchange(
            &request,
            crate::BOARD_INFO_RESPONSE_COMMAND,
            sequence,
            BOARD_INFO_READ_TIMEOUT,
            |frame| BoardInfoResponse::from_frame(frame, sequence),
        )
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
        // An answer that was received stands; the close's failure is in the
        // trace and the channel is remembered (ADR-0042, decision 5).
        let _ = self.close_route();
        result.map(|mut result| {
            result.trace = self.trace.clone();
            result
        })
    }

    /// The next frame of the open route, within `timeout`: one kept earlier,
    /// or one read now. The adapter's own records on the way — indications,
    /// late answers, another channel's frames — are counted, not the read's
    /// failure.
    pub fn receive_frame(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<RawCanFrame>, ProtocolError> {
        self.require_open_transport()?;
        let open_route = self.open_route.ok_or(ProtocolError::NoOpenRoute)?;
        if let Some(frame) = self.pending_frames.pop_front() {
            return Ok(Some(frame));
        }
        let deadline = Instant::now() + timeout;
        loop {
            let frame = match self.read_frame(deadline) {
                Ok(frame) => frame,
                Err(ProtocolError::Transport(TransportError::Timeout)) => return Ok(None),
                Err(error) => return Err(error),
            };
            match classify(frame) {
                Incoming::Inbound(frame) => {
                    self.trace.inbound_frames += 1;
                    match passive::parse_inbound_can(&frame, open_route) {
                        Ok(frame) => return Ok(Some(frame)),
                        // Another channel's frame — one a failed close left
                        // open on the adapter, say — is not this read's.
                        Err(ProtocolError::RouteMismatch { .. }) => self.trace.dropped_frames += 1,
                        Err(error) => return Err(error),
                    }
                }
                Incoming::Indication(frame) => self.record_indication(frame),
                Incoming::Command {
                    command,
                    sequence,
                    status,
                    ..
                } => self.record_stale(UnsolicitedFrame {
                    command,
                    sequence,
                    status,
                }),
                Incoming::Short => self.trace.dropped_frames += 1,
            }
        }
    }

    /// Opens a K-line for one read (`ADR-0029`, slice B). Every word this
    /// sends is a hypothesis and lives in `crate::kline`; the adapter has
    /// never answered one. Only the probe path calls it.
    pub(crate) fn open_kline_line(
        &mut self,
        resource: u16,
        baud: u32,
        pin: u8,
        parity: crate::kline::LineParity,
        wakeup: &str,
        node_address: u8,
    ) -> Result<(), ProtocolError> {
        self.require_open_transport()?;
        if self.open_route.is_some() {
            return Err(ProtocolError::RouteAlreadyOpen);
        }
        self.begin_operation();
        let route = *passive::route_by_id(if pin == 8 {
            VehicleRouteId::KLine8
        } else {
            VehicleRouteId::KLine7
        });
        self.ensure_firmware()?;

        let open_sequence = self.sequences.allocate();
        let open_request = passive::diagnostic_open_channel_request(open_sequence, resource, baud);
        let open_response = self.command_exchange(
            &open_request,
            passive::OPEN_CHANNEL_RESPONSE,
            open_sequence,
            COMMAND_READ_TIMEOUT,
            |frame| passive::parse_open_response(frame, open_sequence, resource),
        )?;
        let opened = OpenReceiveRoute {
            route,
            device_channel_id: open_response.channel_id,
        };
        let channel = opened.device_channel_id;

        // The line, then how a byte on it is framed. A failure at either
        // step closes the channel rather than leaving it open on a guess.
        let pin_sequence = self.sequences.allocate();
        let pin_request = crate::kline::set_pin_request(pin_sequence, channel, pin);
        if let Err(error) = self.command_exchange(
            &pin_request,
            passive::SET_PIN_RESPONSE,
            pin_sequence,
            COMMAND_READ_TIMEOUT,
            |frame| passive::parse_set_pin_response(frame, pin_sequence, channel),
        ) {
            let _ = self.close_channel(channel);
            return Err(error);
        }

        let parity_sequence = self.sequences.allocate();
        let parity_request = crate::kline::set_parity_request(parity_sequence, channel, parity);
        if let Err(error) = self.command_exchange(
            &parity_request,
            crate::kline::SET_CONFIG_RESPONSE,
            parity_sequence,
            COMMAND_READ_TIMEOUT,
            |frame| crate::kline::parse_set_config_response(frame, parity_sequence, channel),
        ) {
            let _ = self.close_channel(channel);
            return Err(error);
        }

        // The bus's own wake-up, where it asks for one. ISO 14230's fast
        // init is a command this adapter performs itself; DS2 asks for none.
        if wakeup == crate::kline::WAKEUP_KW2000_FAST {
            let init_sequence = self.sequences.allocate();
            let init_request =
                crate::kline::fast_init_request(init_sequence, channel, node_address);
            if let Err(error) = self.command_exchange(
                &init_request,
                crate::kline::FAST_INIT_RESPONSE,
                init_sequence,
                COMMAND_READ_TIMEOUT,
                |frame| crate::kline::parse_fast_init_response(frame, init_sequence, channel),
            ) {
                let _ = self.close_channel(channel);
                return Err(error);
            }
        }

        self.open_route = Some(opened);
        Ok(())
    }

    /// Puts one prepared request's bytes on an open K-line. Not public: the
    /// only way in is `execute_prepared_kline_read_as_probe`, which takes a
    /// prepared transaction.
    pub(crate) fn transmit_line_bytes(&mut self, bytes: &[u8]) -> Result<(), ProtocolError> {
        let open_route = self.open_route.ok_or(ProtocolError::NoOpenRoute)?;
        let sequence = self.sequences.allocate();
        let request =
            crate::kline::outbound_line_request(sequence, open_route.device_channel_id, bytes);
        self.acknowledged_transmit(&request, sequence, open_route.device_channel_id)
    }

    /// Whatever the line carried back, until it goes quiet.
    pub(crate) fn receive_line_bytes(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<Vec<u8>>, ProtocolError> {
        self.require_open_transport()?;
        let open_route = self.open_route.ok_or(ProtocolError::NoOpenRoute)?;
        let deadline = Instant::now() + timeout;
        loop {
            let frame = match self.read_frame(deadline) {
                Ok(frame) => frame,
                Err(ProtocolError::Transport(TransportError::Timeout)) => return Ok(None),
                Err(error) => return Err(error),
            };
            match classify(frame) {
                Incoming::Inbound(frame) => {
                    self.trace.inbound_frames += 1;
                    match crate::kline::parse_inbound_line(&frame, open_route) {
                        Ok(bytes) => return Ok(Some(bytes)),
                        Err(ProtocolError::RouteMismatch { .. }) => self.trace.dropped_frames += 1,
                        Err(error) => return Err(error),
                    }
                }
                Incoming::Indication(frame) => self.record_indication(frame),
                Incoming::Command {
                    command,
                    sequence,
                    status,
                    ..
                } => self.record_stale(UnsolicitedFrame {
                    command,
                    sequence,
                    status,
                }),
                Incoming::Short => self.trace.dropped_frames += 1,
            }
        }
    }

    /// Closes the open route. Whatever the adapter answers, the route is
    /// released here: on the first real car (2026-11-12) a close the adapter
    /// did not answer left the route marked open, and every later read was
    /// refused by this code, not by the car. A close that fails is reported,
    /// written into the trace, its channel remembered and closed again before
    /// the next one is opened.
    pub fn close_route(&mut self) -> Result<(), ProtocolError> {
        self.require_open_transport()?;
        let open_route = self.open_route.take().ok_or(ProtocolError::NoOpenRoute)?;
        let closed = self.close_channel(open_route.device_channel_id);
        if let Err(error) = &closed {
            self.stale_channel = Some(open_route.device_channel_id);
            self.trace.close_error = Some(error.to_string());
        }
        closed
    }

    /// Closes the OS transport without sending a Mongoose cCloseDevice command.
    /// A route still open is closed first, as well as it can be: a disconnect
    /// is never refused for the adapter's sake.
    pub fn close(&mut self) -> Result<(), ProtocolError> {
        if self.open_route.is_some() {
            let _ = self.close_route();
        }
        if !self.closed {
            self.transport.close()?;
            self.closed = true;
        }
        Ok(())
    }

    /// The start of one operation: the trace of the last one goes, and so do
    /// frames kept for a route that is no longer open (ADR-0042, decision 6).
    fn begin_operation(&mut self) {
        self.trace = ExchangeTrace::default();
        self.pending_frames.clear();
    }

    /// One command and its one answer: the request written, the frame with
    /// the response word and the sequence waited for within `timeout`, then
    /// parsed by the command's own parser.
    fn command_exchange<R>(
        &mut self,
        request: &[u8],
        expected_command: u16,
        sequence: u16,
        timeout: Duration,
        parse: impl FnOnce(&crate::Frame) -> Result<R, ProtocolError>,
    ) -> Result<R, ProtocolError> {
        self.transport.write_all(request)?;
        let frame = self.wait_for_answer(expected_command, sequence, timeout)?;
        parse(&frame)
    }

    /// The answer to one command (ADR-0042, decision 2): the frame whose
    /// command word and sequence are the ones expected. On the way, inbound
    /// records are kept for the open route, indications are recorded, and a
    /// command-class frame with another sequence — an earlier command's
    /// answer arriving late — is skipped and recorded. A frame with the
    /// expected sequence and another command is an error, as is the
    /// deadline.
    fn wait_for_answer(
        &mut self,
        expected_command: u16,
        expected_sequence: u16,
        timeout: Duration,
    ) -> Result<crate::Frame, ProtocolError> {
        let deadline = Instant::now() + timeout;
        loop {
            let frame = self.read_frame(deadline)?;
            match classify(frame) {
                Incoming::Inbound(frame) => self.keep_inbound(frame),
                Incoming::Indication(frame) => self.record_indication(frame),
                Incoming::Short => self.trace.dropped_frames += 1,
                Incoming::Command {
                    command,
                    sequence,
                    status,
                    frame,
                } => {
                    if sequence != expected_sequence {
                        self.record_stale(UnsolicitedFrame {
                            command,
                            sequence,
                            status,
                        });
                        continue;
                    }
                    if command != expected_command {
                        return Err(ProtocolError::UnexpectedCommand {
                            expected: expected_command,
                            actual: command,
                        });
                    }
                    return Ok(frame);
                }
            }
        }
    }

    /// The next frame from the adapter before `deadline`: one already
    /// decoded, or one read with the time that is left. Bytes the codec
    /// cannot frame are dropped one at a time and counted, so a stream left
    /// half-read by an earlier session resynchronises on the next frame.
    fn read_frame(&mut self, deadline: Instant) -> Result<crate::Frame, ProtocolError> {
        let mut read_buffer = [0_u8; 512];
        loop {
            match self.decoder.next_frame() {
                Ok(Some(frame)) => return Ok(frame),
                Ok(None) => {}
                Err(_) => {
                    self.trace.dropped_bytes += 1;
                    continue;
                }
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(TransportError::Timeout.into());
            }
            self.transport
                .set_read_timeout(remaining.max(MINIMUM_READ_TIMEOUT))?;
            match self.transport.read(&mut read_buffer)? {
                0 => return Err(TransportError::Closed.into()),
                read => self.decoder.push(&read_buffer[..read]),
            }
        }
    }

    /// An inbound record that arrived while a command's answer was awaited:
    /// kept for the open CAN route, so a module's answer that races the
    /// transmit's acknowledgement is not lost; counted and dropped when no
    /// route is open or the route is a line, whose bytes the line reader
    /// gathers itself.
    fn keep_inbound(&mut self, frame: crate::Frame) {
        self.trace.inbound_frames += 1;
        let Some(route) = self.open_route else {
            return;
        };
        if route.route.network_type != NetworkType::Can {
            return;
        }
        match passive::parse_inbound_can(&frame, route) {
            Ok(frame) => self.pending_frames.push_back(frame),
            Err(_) => self.trace.dropped_frames += 1,
        }
    }

    fn record_indication(&mut self, frame: UnsolicitedFrame) {
        self.trace.indications += 1;
        *self
            .trace
            .indications_by_status
            .entry(frame.status)
            .or_default() += 1;
        self.trace.last_indication = Some(frame);
    }

    fn record_stale(&mut self, frame: UnsolicitedFrame) {
        self.trace.stale_frames += 1;
        self.trace.last_stale_frame = Some(frame);
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
        self.begin_operation();
        if let Some(stale) = self.stale_channel.take() {
            // The channel a failed close left behind: closed, as well as it
            // can be, before the adapter is asked for another.
            let _ = self.close_channel(stale);
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
        // A route without a known resource word cannot be opened; the K-line
        // routes are refused above already, and this keeps it so should a
        // route ever be marked ready before its word is known (ADR-0029).
        let route_word =
            passive::resource_route(route_id).ok_or(ProtocolError::PassiveRouteUnavailable {
                route: route.id.as_str(),
                reason: passive::K_LINE_NOT_OPENED,
            })?;

        let open_sequence = self.sequences.allocate();
        let open_request = if listen_only {
            passive::open_channel_request(open_sequence, route_word, bitrate)
        } else {
            passive::diagnostic_open_channel_request(open_sequence, route_word, bitrate)
        };
        let open_response = self.command_exchange(
            &open_request,
            passive::OPEN_CHANNEL_RESPONSE,
            open_sequence,
            COMMAND_READ_TIMEOUT,
            |frame| passive::parse_open_response(frame, open_sequence, route_word),
        )?;
        let opened = OpenReceiveRoute {
            route,
            device_channel_id: open_response.channel_id,
        };
        let channel = opened.device_channel_id;

        let pin_sequence = self.sequences.allocate();
        let set_pin_request = passive::set_pin_request(pin_sequence, channel, opened.route);
        if let Err(error) = self.command_exchange(
            &set_pin_request,
            passive::SET_PIN_RESPONSE,
            pin_sequence,
            COMMAND_READ_TIMEOUT,
            |frame| passive::parse_set_pin_response(frame, pin_sequence, channel),
        ) {
            let _ = self.close_channel(channel);
            return Err(error);
        }
        // Without a pass entry the firmware hands the host nothing from the
        // channel (ADR-0043); a refused entry closes it rather than leaving a
        // read to time out on a channel that cannot be heard.
        if let Err(error) = self.add_pass_entries(channel) {
            let _ = self.close_channel(channel);
            return Err(error);
        }
        self.open_route = Some(opened);
        Ok(opened)
    }

    /// The pass entries every CAN channel gets after its pins (ADR-0043,
    /// decisions 1, 4 and 5): one per identifier format, each recorded in the
    /// trace as the firmware answered it. Status zero with no words is
    /// accepted; anything else ends the open with the firmware's words.
    fn add_pass_entries(&mut self, channel: u16) -> Result<(), ProtocolError> {
        for &format in passive::PASS_ENTRY_FORMATS {
            let sequence = self.sequences.allocate();
            let request = passive::pass_everything_entry_request(sequence, channel, format);
            let answer = self.command_exchange(
                &request,
                passive::TABLE_ADD_ENTRY_RESPONSE,
                sequence,
                COMMAND_READ_TIMEOUT,
                |frame| passive::parse_table_entry_answer(frame, sequence, channel),
            )?;
            let entry = ReceiveFilterEntry {
                format,
                status: answer.status,
                message: answer.message,
                word: answer.word,
            };
            let accepted = entry.accepted();
            self.trace.receive_filters.push(entry.clone());
            if !accepted {
                return Err(ProtocolError::ReceiveFilterRefused {
                    format: format_words(format),
                    status: entry.status,
                    message: entry.message,
                });
            }
        }
        Ok(())
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
                    trace: self.trace.clone(),
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
        self.acknowledged_transmit(&request, sequence, open_route.device_channel_id)
    }

    /// One outbound record and its acknowledgement (ADR-0042, decision 1):
    /// the acknowledgement ends the wait whatever its status word, and the
    /// word, the firmware's words and the time it took go into the trace.
    /// Text after the status is a refusal — every refusal this firmware has
    /// given carried text, and no acknowledgement has.
    fn acknowledged_transmit(
        &mut self,
        request: &[u8],
        sequence: u16,
        channel_id: u16,
    ) -> Result<(), ProtocolError> {
        self.transport.write_all(request)?;
        let sent = Instant::now();
        let frame = self.wait_for_answer(
            passive::OUTBOUND_DATA_RESPONSE,
            sequence,
            TRANSMIT_ACK_TIMEOUT,
        )?;
        let acknowledgement =
            passive::parse_outbound_acknowledgement(&frame, sequence, channel_id)?;
        let waited_ms = u64::try_from(sent.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.trace.transmits += 1;
        self.trace.last_acknowledgement = Some(TransmitAcknowledgement {
            status: acknowledgement.status,
            message: acknowledgement.message.clone(),
            waited_ms,
        });
        if !acknowledgement.message.is_empty() {
            return Err(ProtocolError::DeviceStatus {
                operation: "cOutboundData",
                status: acknowledgement.status,
                message: acknowledgement.message,
            });
        }
        Ok(())
    }

    /// One close of one channel on the adapter: after a failed setup, after
    /// an operation, or for a channel a failed close left behind.
    fn close_channel(&mut self, channel_id: u16) -> Result<(), ProtocolError> {
        let sequence = self.sequences.allocate();
        let request = passive::close_channel_request(sequence, channel_id);
        self.command_exchange(
            &request,
            passive::CLOSE_CHANNEL_RESPONSE,
            sequence,
            COMMAND_READ_TIMEOUT,
            |frame| passive::parse_close_response(frame, sequence, channel_id),
        )
        .map(|_| ())
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
        command_response, command_response_with_text, inbound_can_frame, indication,
        outbound_acknowledgement, pass_everything_entry_request, table_entry_accepted,
        CAN1_RESOURCE_ROUTE, CAN2_RESOURCE_ROUTE, CLOSE_CHANNEL_RESPONSE, DEVICE_ROUTE,
        JUMP_TO_FIRMWARE_RESPONSE, OPEN_CHANNEL_RESPONSE, OUTBOUND_DATA_RESPONSE,
        PASS_ENTRY_FORMATS, SET_PIN_RESPONSE, TABLE_ADD_ENTRY_RESPONSE,
        TRANSMIT_ACKNOWLEDGED_STATUS,
    };

    /// The pass entries every CAN open adds after its pins (ADR-0043). The
    /// scripts count with it, so they hold whichever formats the gate keeps.
    const ENTRIES: u16 = PASS_ENTRY_FORMATS.len() as u16;
    const E: usize = PASS_ENTRY_FORMATS.len();
    /// The first sequence after the first open's board-info, open, pins and
    /// entries.
    const NEXT: u16 = 4 + ENTRIES;

    /// The firmware's answers to one open's pass entries, the first at
    /// sequence `first`, as the bare adapter gave them on 2026-11-14.
    fn entries_accepted(channel: u16, first: u16) -> Vec<Vec<u8>> {
        (0..ENTRIES)
            .map(|index| {
                table_entry_accepted(
                    channel,
                    first + index,
                    0x50 + u32::from(index),
                    0x04CC + u32::from(index),
                )
            })
            .collect()
    }

    fn entry_steps(channel: u16, first: u16) -> Vec<Step> {
        entries_accepted(channel, first)
            .into_iter()
            .map(Step::Bytes)
            .collect()
    }

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

    /// What the adapter does next when read: bytes, or silence. `Silent`
    /// stays at the front and answers every read with a timeout until the
    /// device writes something, as a quiet adapter does.
    enum Step {
        Bytes(Vec<u8>),
        Silent,
    }

    #[derive(Default)]
    struct MockState {
        writes: Vec<Vec<u8>>,
        /// Every read timeout the device set, in order.
        timeouts: Vec<Duration>,
        closes: usize,
    }

    struct MockTransport {
        state: Arc<Mutex<MockState>>,
        steps: VecDeque<Step>,
    }

    impl MockTransport {
        fn new(reads: Vec<Vec<u8>>) -> (Self, Arc<Mutex<MockState>>) {
            Self::scripted(reads.into_iter().map(Step::Bytes).collect())
        }

        fn scripted(steps: Vec<Step>) -> (Self, Arc<Mutex<MockState>>) {
            let state = Arc::new(Mutex::new(MockState::default()));
            (
                Self {
                    state: Arc::clone(&state),
                    steps: steps.into(),
                },
                state,
            )
        }
    }

    impl ByteTransport for MockTransport {
        fn write_all(&mut self, bytes: &[u8]) -> Result<(), TransportError> {
            self.state.lock().unwrap().writes.push(bytes.to_vec());
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

        fn set_read_timeout(&mut self, timeout: Duration) -> Result<(), TransportError> {
            self.state.lock().unwrap().timeouts.push(timeout);
            Ok(())
        }

        fn close(&mut self) -> Result<(), TransportError> {
            self.state.lock().unwrap().closes += 1;
            Ok(())
        }
    }

    /// Board-info, open, pins and the pass entries for one channel, as the
    /// firmware answers them: the start of every operation's script.
    fn opened(channel: u16) -> Vec<Step> {
        let mut steps = vec![
            Step::Bytes(firmware_board_info(1)),
            Step::Bytes(command_response(channel, OPEN_CHANNEL_RESPONSE, 2, 0)),
            Step::Bytes(command_response(channel, SET_PIN_RESPONSE, 3, 0)),
        ];
        steps.extend(entry_steps(channel, 4));
        steps
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

    /// ADR-0042, decision 2: an answer with another sequence is not this
    /// command's. It is skipped and recorded, and the wait goes on to its
    /// deadline — here, the adapter's silence.
    #[test]
    fn an_answer_with_another_sequence_is_skipped_and_recorded() {
        let response = encode_board_info_response(2, &[]);
        let (transport, _) = MockTransport::new(vec![response]);
        let mut device = MongooseJlrDevice::open(transport);
        assert_eq!(
            device.get_board_info(),
            Err(ProtocolError::Transport(TransportError::Timeout))
        );
        let trace = device.exchange_trace();
        assert_eq!(trace.stale_frames, 1);
        assert_eq!(
            trace.last_stale_frame,
            Some(UnsolicitedFrame {
                command: 0x8109,
                sequence: 2,
                status: None,
            })
        );
    }

    /// The expected sequence under another command word is the firmware
    /// answering this command with something else: an error, not a skip.
    #[test]
    fn the_expected_sequence_under_another_command_is_an_error() {
        let mut response = encode_board_info_response(1, &[]);
        response[8] = 0x08;
        let (transport, _) = MockTransport::new(vec![response]);
        let mut device = MongooseJlrDevice::open(transport);
        assert_eq!(
            device.get_board_info(),
            Err(ProtocolError::UnexpectedCommand {
                expected: 0x8109,
                actual: 0x8108,
            })
        );
    }

    /// A stream an earlier session left half-read: the answer to its last
    /// close, and bytes no frame begins with, ahead of the board-info this
    /// session asks for. Both are counted and the connection stands.
    #[test]
    fn frames_and_bytes_left_by_an_earlier_session_are_skipped_before_board_info() {
        let mut leftovers = command_response(CAN1_RESOURCE_ROUTE, CLOSE_CHANNEL_RESPONSE, 97, 0);
        leftovers.extend_from_slice(&[0xFF, 0xFF, 0x00]);
        leftovers.extend_from_slice(&firmware_board_info(1));
        let (transport, _) = MockTransport::new(vec![leftovers]);
        let mut device = MongooseJlrDevice::open(transport);

        let info = device.get_board_info().unwrap();
        assert!(!info.in_bootloader());
        let trace = device.exchange_trace();
        assert_eq!(trace.stale_frames, 1);
        assert_eq!(
            trace
                .last_stale_frame
                .map(|frame| (frame.command, frame.sequence)),
            Some((CLOSE_CHANNEL_RESPONSE, 97))
        );
        assert_eq!(trace.dropped_bytes, 3);
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

    /// The first real car (2026-11-12): a close the adapter did not answer
    /// left the route marked open, so every later read was refused by this
    /// code. A failed close releases the route and is written into the trace;
    /// the next open closes the stale channel first, then opens its own.
    #[test]
    fn a_close_the_adapter_does_not_answer_releases_the_route_and_is_retried_before_the_next_open()
    {
        let mut steps = opened(CAN1_RESOURCE_ROUTE);
        // The close (sequence NEXT) is not answered: a silence ends at the
        // close's own write, so a second one keeps the adapter quiet until
        // the next command is written.
        steps.push(Step::Silent);
        steps.push(Step::Silent);
        // The stale channel's close before the next open.
        steps.push(Step::Bytes(command_response(
            CAN1_RESOURCE_ROUTE,
            CLOSE_CHANNEL_RESPONSE,
            NEXT + 1,
            0,
        )));
        steps.push(Step::Bytes(command_response(
            CAN2_RESOURCE_ROUTE,
            OPEN_CHANNEL_RESPONSE,
            NEXT + 2,
            0,
        )));
        steps.push(Step::Bytes(command_response(
            CAN2_RESOURCE_ROUTE,
            SET_PIN_RESPONSE,
            NEXT + 3,
            0,
        )));
        steps.extend(entry_steps(CAN2_RESOURCE_ROUTE, NEXT + 4));
        let (transport, state) = MockTransport::scripted(steps);
        let mut device = MongooseJlrDevice::open(transport);

        device.open_receive_route(VehicleRouteId::HsCan).unwrap();
        assert!(device.close_route().is_err(), "the close was not answered");
        assert!(
            device.open_route.is_none(),
            "the route is released all the same"
        );
        assert_eq!(device.stale_channel, Some(CAN1_RESOURCE_ROUTE));
        assert_eq!(
            device.exchange_trace().close_error.as_deref(),
            Some("transport read timed out")
        );

        let reopened = device.open_receive_route(VehicleRouteId::MsCan).unwrap();
        assert_eq!(reopened.device_channel_id, CAN2_RESOURCE_ROUTE);
        assert!(device.stale_channel.is_none());
        assert!(
            device.exchange_trace().close_error.is_none(),
            "a new operation starts with a clean trace"
        );
        let writes = &state.lock().unwrap().writes;
        // Board-info, open, pins, entries, the close that failed, the stale
        // close, open, pins, entries.
        assert_eq!(writes.len(), 7 + 2 * E);
        assert_eq!(
            writes[4 + E],
            passive::close_channel_request(NEXT + 1, CAN1_RESOURCE_ROUTE),
            "the stale channel is closed before a new one is opened"
        );
    }

    /// A disconnect with a route still open closes the route first instead
    /// of refusing: the person at the screen must always be able to leave.
    #[test]
    fn close_with_a_route_open_closes_the_route_first() {
        let mut steps = opened(CAN1_RESOURCE_ROUTE);
        steps.push(Step::Bytes(command_response(
            CAN1_RESOURCE_ROUTE,
            CLOSE_CHANNEL_RESPONSE,
            NEXT,
            0,
        )));
        let (transport, state) = MockTransport::scripted(steps);
        let mut device = MongooseJlrDevice::open(transport);
        device.open_receive_route(VehicleRouteId::HsCan).unwrap();

        device.close().unwrap();

        assert!(device.open_route.is_none());
        let state = state.lock().unwrap();
        assert_eq!(
            state.writes.len(),
            4 + E,
            "board-info, open, pins, entries, close"
        );
        assert_eq!(state.closes, 1);
    }

    #[test]
    fn passive_open_receive_close_and_reopen_are_production_path() {
        let mut responses = vec![
            firmware_board_info(1),
            command_response(CAN1_RESOURCE_ROUTE, OPEN_CHANNEL_RESPONSE, 2, 0),
            command_response(CAN1_RESOURCE_ROUTE, SET_PIN_RESPONSE, 3, 0),
        ];
        responses.extend(entries_accepted(CAN1_RESOURCE_ROUTE, 4));
        responses.extend([
            inbound_can_frame(CAN1_RESOURCE_ROUTE, 0, 1234, 0x321, &[1, 2, 3]),
            command_response(CAN1_RESOURCE_ROUTE, CLOSE_CHANNEL_RESPONSE, NEXT, 0),
            command_response(CAN2_RESOURCE_ROUTE, OPEN_CHANNEL_RESPONSE, NEXT + 1, 0),
            command_response(CAN2_RESOURCE_ROUTE, SET_PIN_RESPONSE, NEXT + 2, 0),
        ]);
        responses.extend(entries_accepted(CAN2_RESOURCE_ROUTE, NEXT + 3));
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
        assert_eq!(device.exchange_trace().inbound_frames, 1);
        device.close_route().unwrap();

        let reopened = device.open_receive_route(VehicleRouteId::MsCan).unwrap();
        assert_eq!(reopened.device_channel_id, CAN2_RESOURCE_ROUTE);
        let writes = &state.lock().unwrap().writes;
        // Board-info once, then open, pins, entries, close, open, pins,
        // entries.
        assert_eq!(writes.len(), 6 + 2 * E);
        assert_eq!(&writes[0][8..10], &[0x09, 0x01]);
        assert_eq!(&writes[1][4..6], &CAN1_RESOURCE_ROUTE.to_le_bytes());
        assert_eq!(&writes[1][16..20], &0x1000_0000_u32.to_le_bytes());
        assert_eq!(&writes[1][20..24], &500_000_u32.to_le_bytes());
        assert_eq!(&writes[2][20..24], &6_u32.to_le_bytes());
        assert_eq!(&writes[2][24..28], &14_u32.to_le_bytes());
        assert_eq!(&writes[4 + E][4..6], &CAN2_RESOURCE_ROUTE.to_le_bytes());
        assert_eq!(&writes[4 + E][20..24], &125_000_u32.to_le_bytes());
        assert_eq!(&writes[5 + E][20..24], &3_u32.to_le_bytes());
        assert_eq!(&writes[5 + E][24..28], &11_u32.to_le_bytes());
    }

    #[test]
    fn a_bootloader_answer_is_followed_by_the_firmware_jump() {
        let responses: Vec<Vec<u8>> = vec![
            bootloader_board_info(1),
            command_response(DEVICE_ROUTE, JUMP_TO_FIRMWARE_RESPONSE, 2, 0),
            // The firmware is still starting: the first poll gets the
            // bootloader's answer again, the second the firmware's.
            bootloader_board_info(3),
            firmware_board_info(4),
            command_response(CAN1_RESOURCE_ROUTE, OPEN_CHANNEL_RESPONSE, 5, 0),
            command_response(CAN1_RESOURCE_ROUTE, SET_PIN_RESPONSE, 6, 0),
        ]
        .into_iter()
        .chain(entries_accepted(CAN1_RESOURCE_ROUTE, 7))
        .collect();
        let (transport, state) = MockTransport::new(responses);
        let mut device = MongooseJlrDevice::open(transport);
        device.open_receive_route(VehicleRouteId::HsCan).unwrap();
        let writes = &state.lock().unwrap().writes;
        assert_eq!(writes.len(), 6 + E);
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
        let refusal = command_response_with_text(
            CAN1_RESOURCE_ROUTE,
            SET_PIN_RESPONSE,
            3,
            0x206,
            0x4433_2211,
            "SetPins: MongoosePro JLR board only supports CAN on pins 6 and 14",
        );
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
            "cSetPin failed with device status 0x00000206 ePinNotSupported: SetPins: MongoosePro JLR board only supports CAN on pins 6 and 14"
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
        for entry in entries_accepted(CAN1_RESOURCE_ROUTE, 4) {
            first_read.extend_from_slice(&entry);
        }
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

    /// ADR-0042, decision 1: the word the firmware has given every transmit —
    /// `0x100`, no text — ends the wait and goes into the trace; it is not a
    /// refusal. The 1.2.5 and 1.2.6 builds abandoned every request on it.
    #[test]
    fn a_transmit_acknowledged_with_status_0x100_is_recorded_not_refused() {
        let mut steps = opened(CAN1_RESOURCE_ROUTE);
        steps.push(Step::Bytes(outbound_acknowledgement(
            CAN1_RESOURCE_ROUTE,
            NEXT,
            0x07F9_6350,
        )));
        let (transport, state) = MockTransport::scripted(steps);
        let mut device = MongooseJlrDevice::open(transport);
        device
            .open_route_internal(VehicleRouteId::HsCan, false)
            .unwrap();

        device
            .transmit_diagnostic_frame(CanId::Standard(0x7E0), &[0x02, 0x10, 0x01])
            .unwrap();

        let trace = device.exchange_trace();
        assert_eq!(trace.transmits, 1);
        let acknowledgement = trace.last_acknowledgement.as_ref().unwrap();
        assert_eq!(acknowledgement.status, TRANSMIT_ACKNOWLEDGED_STATUS);
        assert!(acknowledgement.message.is_empty());
        // The acknowledgement was waited for with the transmit's own timeout,
        // not the one-second command wait.
        let timeouts = &state.lock().unwrap().timeouts;
        let last = *timeouts.last().unwrap();
        assert!(
            last > Duration::from_millis(4_900) && last <= TRANSMIT_ACK_TIMEOUT,
            "{last:?}"
        );
    }

    /// The one judgment kept: text after the status is a refusal, as every
    /// refusal of this firmware has carried text — here the bench's words for
    /// a transmit on a channel that is not open.
    #[test]
    fn an_acknowledgement_with_the_firmware_words_is_a_refusal() {
        let mut steps = opened(CAN1_RESOURCE_ROUTE);
        steps.push(Step::Bytes(command_response_with_text(
            CAN1_RESOURCE_ROUTE,
            OUTBOUND_DATA_RESPONSE,
            NEXT,
            2,
            77,
            "cOutboundData: channel is not open",
        )));
        let (transport, _) = MockTransport::scripted(steps);
        let mut device = MongooseJlrDevice::open(transport);
        device
            .open_route_internal(VehicleRouteId::HsCan, false)
            .unwrap();

        let error = device
            .transmit_diagnostic_frame(CanId::Standard(0x7E0), &[0x02, 0x10, 0x01])
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "cOutboundData failed with device status 0x00000002 eInvalidChannelId: cOutboundData: channel is not open"
        );
        let acknowledgement = device
            .exchange_trace()
            .last_acknowledgement
            .as_ref()
            .unwrap();
        assert_eq!(acknowledgement.status, 2);
        assert_eq!(
            acknowledgement.message,
            "cOutboundData: channel is not open"
        );
    }

    /// ADR-0042, decision 2: a `cIndication` between a command and its answer
    /// is recorded and the wait goes on. In the second real-car session the
    /// close's wait took such a frame for its answer, and every later command
    /// read the one before it.
    #[test]
    fn an_indication_during_a_wait_is_recorded_and_the_wait_goes_on() {
        let mut steps = opened(CAN1_RESOURCE_ROUTE);
        steps.push(Step::Bytes(outbound_acknowledgement(
            CAN1_RESOURCE_ROUTE,
            NEXT,
            10,
        )));
        // The indication arrives while the close's answer is awaited.
        steps.push(Step::Bytes(indication(CAN1_RESOURCE_ROUTE, NEXT, 0, 11)));
        steps.push(Step::Bytes(command_response(
            CAN1_RESOURCE_ROUTE,
            CLOSE_CHANNEL_RESPONSE,
            NEXT + 1,
            0,
        )));
        let (transport, _) = MockTransport::scripted(steps);
        let mut device = MongooseJlrDevice::open(transport);
        device
            .open_route_internal(VehicleRouteId::HsCan, false)
            .unwrap();
        device
            .transmit_diagnostic_frame(CanId::Standard(0x7E0), &[0x02, 0x10, 0x01])
            .unwrap();

        device.close_route().unwrap();

        let trace = device.exchange_trace();
        assert_eq!(trace.indications, 1);
        assert_eq!(
            trace.last_indication,
            Some(UnsolicitedFrame {
                command: 0x000A,
                sequence: NEXT,
                status: Some(0),
            })
        );
        assert!(trace.close_error.is_none());
        assert!(device.stale_channel.is_none());
    }

    /// An indication that arrives while the module's answer is awaited is
    /// recorded the same way, and the answer behind it is still received.
    #[test]
    fn an_indication_during_the_receive_is_recorded_and_the_answer_behind_it_received() {
        let mut steps = opened(CAN1_RESOURCE_ROUTE);
        steps.push(Step::Bytes(outbound_acknowledgement(
            CAN1_RESOURCE_ROUTE,
            NEXT,
            10,
        )));
        steps.push(Step::Bytes(indication(CAN1_RESOURCE_ROUTE, NEXT, 0, 11)));
        steps.push(Step::Bytes(inbound_can_frame(
            CAN1_RESOURCE_ROUTE,
            0,
            12,
            0x7E8,
            &[0x06, 0x50, 0x01, 0x00, 0x32, 0x01, 0xF4],
        )));
        let (transport, _) = MockTransport::scripted(steps);
        let mut device = MongooseJlrDevice::open(transport);
        device
            .open_route_internal(VehicleRouteId::HsCan, false)
            .unwrap();
        device
            .transmit_diagnostic_frame(CanId::Standard(0x7E0), &[0x02, 0x10, 0x01])
            .unwrap();

        let frame = device
            .receive_frame(Duration::from_millis(10))
            .unwrap()
            .unwrap();
        assert_eq!(frame.arbitration_id, 0x7E8);
        let trace = device.exchange_trace();
        assert_eq!(trace.indications, 1);
        assert_eq!(trace.inbound_frames, 1);
    }

    /// The bench of 2026-09-06/08 by another door: the firmware acknowledged
    /// a transmit after three seconds, the earlier builds waited one. An
    /// acknowledgement that arrives only when the close is already awaited
    /// is a stale frame now — skipped, recorded, and the close's own answer
    /// still read.
    #[test]
    fn a_late_acknowledgement_is_a_stale_frame_that_does_not_shift_the_session() {
        let mut steps = opened(CAN1_RESOURCE_ROUTE);
        // Nothing within the acknowledgement wait: one silence ends at the
        // transmit's write, the second lasts until the close is written.
        steps.push(Step::Silent);
        steps.push(Step::Silent);
        // Then, while the close is awaited, the transmit's acknowledgement.
        steps.push(Step::Bytes(outbound_acknowledgement(
            CAN1_RESOURCE_ROUTE,
            NEXT,
            10,
        )));
        steps.push(Step::Bytes(command_response(
            CAN1_RESOURCE_ROUTE,
            CLOSE_CHANNEL_RESPONSE,
            NEXT + 1,
            0,
        )));
        let (transport, _) = MockTransport::scripted(steps);
        let mut device = MongooseJlrDevice::open(transport);
        device
            .open_route_internal(VehicleRouteId::HsCan, false)
            .unwrap();

        let error = device
            .transmit_diagnostic_frame(CanId::Standard(0x7E0), &[0x02, 0x10, 0x01])
            .unwrap_err();
        assert_eq!(error, ProtocolError::Transport(TransportError::Timeout));
        assert_eq!(device.exchange_trace().transmits, 0);

        device.close_route().unwrap();
        let trace = device.exchange_trace();
        assert_eq!(trace.stale_frames, 1);
        assert_eq!(
            trace.last_stale_frame,
            Some(UnsolicitedFrame {
                command: OUTBOUND_DATA_RESPONSE,
                sequence: NEXT,
                status: Some(TRANSMIT_ACKNOWLEDGED_STATUS),
            })
        );
        assert!(trace.close_error.is_none());
    }

    /// ADR-0042, decision 3: a close after a read that ran its budget down
    /// is waited for with its own second, not with the milliseconds the last
    /// receive had left.
    #[test]
    fn the_close_sets_its_own_timeout_whatever_the_last_receive_left() {
        let mut steps = opened(CAN1_RESOURCE_ROUTE);
        steps.push(Step::Silent);
        steps.push(Step::Bytes(command_response(
            CAN1_RESOURCE_ROUTE,
            CLOSE_CHANNEL_RESPONSE,
            NEXT,
            0,
        )));
        let (transport, state) = MockTransport::scripted(steps);
        let mut device = MongooseJlrDevice::open(transport);
        device.open_receive_route(VehicleRouteId::HsCan).unwrap();

        assert!(device
            .receive_frame(Duration::from_millis(3))
            .unwrap()
            .is_none());
        device.close_route().unwrap();

        let timeouts = &state.lock().unwrap().timeouts;
        let receive = timeouts[timeouts.len() - 2];
        let close = timeouts[timeouts.len() - 1];
        assert!(receive <= Duration::from_millis(3), "{receive:?}");
        assert!(
            close > Duration::from_millis(900) && close <= COMMAND_READ_TIMEOUT,
            "{close:?}"
        );
    }

    /// A module's answer that races the acknowledgement is kept for the read;
    /// one kept and never read does not survive the next open (ADR-0042,
    /// decision 6).
    #[test]
    fn a_frame_kept_during_the_acknowledgement_wait_is_the_next_received_and_does_not_survive_a_reopen(
    ) {
        let mut steps = opened(CAN1_RESOURCE_ROUTE);
        steps.push(Step::Bytes(inbound_can_frame(
            CAN1_RESOURCE_ROUTE,
            0,
            9,
            0x7E8,
            &[0x02, 0x50, 0x01],
        )));
        steps.push(Step::Bytes(outbound_acknowledgement(
            CAN1_RESOURCE_ROUTE,
            NEXT,
            10,
        )));
        steps.push(Step::Bytes(inbound_can_frame(
            CAN1_RESOURCE_ROUTE,
            0,
            11,
            0x7E8,
            &[0x02, 0x50, 0x02],
        )));
        steps.push(Step::Bytes(outbound_acknowledgement(
            CAN1_RESOURCE_ROUTE,
            NEXT + 1,
            12,
        )));
        steps.push(Step::Bytes(command_response(
            CAN1_RESOURCE_ROUTE,
            CLOSE_CHANNEL_RESPONSE,
            NEXT + 2,
            0,
        )));
        steps.push(Step::Bytes(command_response(
            CAN1_RESOURCE_ROUTE,
            OPEN_CHANNEL_RESPONSE,
            NEXT + 3,
            0,
        )));
        steps.push(Step::Bytes(command_response(
            CAN1_RESOURCE_ROUTE,
            SET_PIN_RESPONSE,
            NEXT + 4,
            0,
        )));
        steps.extend(entry_steps(CAN1_RESOURCE_ROUTE, NEXT + 5));
        steps.push(Step::Silent);
        let (transport, _) = MockTransport::scripted(steps);
        let mut device = MongooseJlrDevice::open(transport);
        device
            .open_route_internal(VehicleRouteId::HsCan, false)
            .unwrap();

        device
            .transmit_diagnostic_frame(CanId::Standard(0x7E0), &[0x02, 0x10, 0x01])
            .unwrap();
        let first = device
            .receive_frame(Duration::from_millis(10))
            .unwrap()
            .unwrap();
        assert_eq!(first.payload(), [0x02, 0x50, 0x01]);
        // A second transmit keeps a second frame, which nobody reads.
        device
            .transmit_diagnostic_frame(CanId::Standard(0x7E0), &[0x02, 0x10, 0x01])
            .unwrap();
        assert_eq!(device.pending_frames.len(), 1);
        device.close_route().unwrap();

        device
            .open_route_internal(VehicleRouteId::HsCan, false)
            .unwrap();
        assert!(device.pending_frames.is_empty());
        assert!(device
            .receive_frame(Duration::from_millis(5))
            .unwrap()
            .is_none());
    }

    /// An inbound record on another channel — one a failed close left open
    /// on the adapter, say — is counted, and the read goes on to its own.
    #[test]
    fn an_inbound_frame_on_another_channel_is_counted_not_fatal() {
        let mut steps = opened(CAN1_RESOURCE_ROUTE);
        steps.push(Step::Bytes(inbound_can_frame(
            CAN2_RESOURCE_ROUTE,
            0,
            5,
            0x123,
            &[1],
        )));
        steps.push(Step::Bytes(inbound_can_frame(
            CAN1_RESOURCE_ROUTE,
            0,
            6,
            0x321,
            &[2],
        )));
        let (transport, _) = MockTransport::scripted(steps);
        let mut device = MongooseJlrDevice::open(transport);
        device.open_receive_route(VehicleRouteId::HsCan).unwrap();

        let frame = device
            .receive_frame(Duration::from_millis(10))
            .unwrap()
            .unwrap();
        assert_eq!(frame.arbitration_id, 0x321);
        let trace = device.exchange_trace();
        assert_eq!(trace.inbound_frames, 2);
        assert_eq!(trace.dropped_frames, 1);
    }

    /// The trace in words, as the shell writes it into a read's record.
    #[test]
    fn the_trace_says_in_words_what_the_adapter_did() {
        let trace = ExchangeTrace {
            receive_filters: vec![
                ReceiveFilterEntry {
                    format: CanIdFormat::Standard,
                    status: 0,
                    message: String::new(),
                    word: Some(0x04CC),
                },
                ReceiveFilterEntry {
                    format: CanIdFormat::Extended,
                    status: 0,
                    message: String::new(),
                    word: Some(0x04CD),
                },
            ],
            transmits: 1,
            last_acknowledgement: Some(TransmitAcknowledgement {
                status: 0x100,
                message: String::new(),
                waited_ms: 3,
            }),
            inbound_frames: 0,
            indications: 2,
            last_indication: Some(UnsolicitedFrame {
                command: 0x000A,
                sequence: 5,
                status: Some(0x106),
            }),
            indications_by_status: BTreeMap::from([(Some(0x106), 1), (Some(0x119), 1)]),
            stale_frames: 0,
            last_stale_frame: None,
            dropped_frames: 0,
            dropped_bytes: 0,
            close_error: Some("transport read timed out".into()),
        };
        assert_eq!(
            trace.to_string(),
            "receive filter 11-bit pass 0x00000000 eSuccess (word 0x000004CC), 29-bit pass 0x00000000 eSuccess (word 0x000004CD); transmit acknowledged with status 0x00000100 eMsgQueued after 3 ms; 0 inbound frames; 2 indication(s), 1 with 0x00000106 iMsgTxDone, 1 with 0x00000119 iMsgRxQueueOvflw, last 0x000A sequence 5 status 0x00000106 iMsgTxDone; close failed: transport read timed out"
        );
        assert_eq!(
            ExchangeTrace::default().to_string(),
            "no transmit acknowledged; 0 inbound frames"
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

        let mut reads = vec![
            firmware_board_info(1),
            command_response(CAN1_RESOURCE_ROUTE, OPEN_CHANNEL_RESPONSE, 2, 0),
            command_response(CAN1_RESOURCE_ROUTE, SET_PIN_RESPONSE, 3, 0),
        ];
        reads.extend(entries_accepted(CAN1_RESOURCE_ROUTE, 4));
        reads.extend([
            outbound_acknowledgement(CAN1_RESOURCE_ROUTE, NEXT, 10),
            inbound_can_frame(CAN1_RESOURCE_ROUTE, 0, 10, 0x7e8, &frames[0]),
            outbound_acknowledgement(CAN1_RESOURCE_ROUTE, NEXT + 1, 20),
            inbound_can_frame(CAN1_RESOURCE_ROUTE, 0, 20, 0x7e8, &frames[1]),
            inbound_can_frame(CAN1_RESOURCE_ROUTE, 0, 30, 0x7e8, &frames[2]),
            command_response(CAN1_RESOURCE_ROUTE, CLOSE_CHANNEL_RESPONSE, NEXT + 2, 0),
        ]);
        let (transport, state) = MockTransport::new(reads);
        let mut device = MongooseJlrDevice::open(transport);
        let result = device
            .execute_prepared_calibration_identification(&transaction, Duration::from_millis(250))
            .unwrap();

        assert_eq!(result.request_payload, [0x09, 0x04]);
        assert_eq!(result.responder, 0x7e8);
        assert_eq!(result.raw_diagnostic_response, response);
        assert_eq!(result.calibration_ids, [OBSERVED_CALIBRATION_ID]);
        assert_eq!(result.trace.transmits, 2);
        assert_eq!(result.trace.inbound_frames, 3);
        assert!(result.trace.close_error.is_none());

        let writes = &state.lock().unwrap().writes;
        assert_eq!(writes.len(), 6 + E);
        assert_eq!(&writes[1][16..20], &0_u32.to_le_bytes());
        assert_eq!(&writes[2][20..24], &6_u32.to_le_bytes());
        assert_eq!(&writes[2][24..28], &14_u32.to_le_bytes());
        assert!(writes[3 + E].ends_with(&[0x00, 0x00, 0x07, 0xE0, 0x02, 0x09, 0x04]));
        assert!(writes[4 + E].ends_with(&[0x00, 0x00, 0x07, 0xE0, 0x30, 0x00, 0x00]));
    }

    /// ADR-0043, decisions 1 and 5: after the pins, one pass entry per
    /// identifier format, the record the bare adapter accepted; each answer
    /// is in the trace with its word.
    #[test]
    fn every_can_channel_gets_its_pass_entries_after_the_pins_and_they_are_in_the_trace() {
        let (transport, state) = MockTransport::scripted(opened(CAN2_RESOURCE_ROUTE));
        let mut device = MongooseJlrDevice::open(transport);
        device
            .open_route_internal(VehicleRouteId::MsCan, false)
            .unwrap();

        let writes = &state.lock().unwrap().writes;
        assert_eq!(writes.len(), 3 + E);
        for (index, format) in PASS_ENTRY_FORMATS.iter().enumerate() {
            assert_eq!(
                writes[3 + index],
                pass_everything_entry_request(4 + index as u16, CAN2_RESOURCE_ROUTE, *format)
            );
        }
        let filters = &device.exchange_trace().receive_filters;
        assert_eq!(filters.len(), E);
        assert!(filters.iter().all(ReceiveFilterEntry::accepted));
        assert_eq!(filters[0].format, CanIdFormat::Standard);
        assert_eq!(filters[0].word, Some(0x04CC));
    }

    /// ADR-0043, decision 4: a refused entry closes the channel and ends the
    /// open in the firmware's words — here the type sweep's own refusal.
    #[test]
    fn a_refused_pass_entry_closes_the_channel_and_says_so_in_the_firmware_words() {
        let steps = vec![
            Step::Bytes(firmware_board_info(1)),
            Step::Bytes(command_response(
                CAN1_RESOURCE_ROUTE,
                OPEN_CHANNEL_RESPONSE,
                2,
                0,
            )),
            Step::Bytes(command_response(
                CAN1_RESOURCE_ROUTE,
                SET_PIN_RESPONSE,
                3,
                0,
            )),
            Step::Bytes(command_response_with_text(
                CAN1_RESOURCE_ROUTE,
                TABLE_ADD_ENTRY_RESPONSE,
                4,
                0x202,
                0x03DA_1D50,
                "FilterAdd: Invalid Filter type",
            )),
            Step::Bytes(command_response(
                CAN1_RESOURCE_ROUTE,
                CLOSE_CHANNEL_RESPONSE,
                5,
                0,
            )),
        ];
        let (transport, state) = MockTransport::scripted(steps);
        let mut device = MongooseJlrDevice::open(transport);

        let error = device
            .open_receive_route(VehicleRouteId::HsCan)
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "the adapter refused the receive filter (11-bit pass entry, cTableAddEntry status 0x00000202 eInvalidValue): FilterAdd: Invalid Filter type"
        );
        assert!(device.open_route.is_none());
        let writes = &state.lock().unwrap().writes;
        assert_eq!(writes.len(), 5, "board-info, open, pins, one entry, close");
        assert_eq!(
            writes[4],
            passive::close_channel_request(5, CAN1_RESOURCE_ROUTE)
        );
        let filters = &device.exchange_trace().receive_filters;
        assert_eq!(filters.len(), 1);
        assert!(!filters[0].accepted());
        assert_eq!(filters[0].status, 0x202);
    }

    /// The third real-car session's shape (2026-11-13) with the entries in
    /// place: the transmit queued, completed, and nothing received. The
    /// trace says all of it, the indication counted by its word.
    #[test]
    fn the_third_sessions_shape_with_the_entries_says_queued_completed_and_nothing_received() {
        let mut steps = opened(CAN1_RESOURCE_ROUTE);
        steps.push(Step::Bytes(outbound_acknowledgement(
            CAN1_RESOURCE_ROUTE,
            NEXT,
            10,
        )));
        steps.push(Step::Bytes(indication(
            CAN1_RESOURCE_ROUTE,
            NEXT,
            0x106,
            11,
        )));
        steps.push(Step::Silent);
        steps.push(Step::Bytes(command_response(
            CAN1_RESOURCE_ROUTE,
            CLOSE_CHANNEL_RESPONSE,
            NEXT + 1,
            0,
        )));
        let (transport, _) = MockTransport::scripted(steps);
        let mut device = MongooseJlrDevice::open(transport);
        device
            .open_route_internal(VehicleRouteId::HsCan, false)
            .unwrap();
        device
            .transmit_diagnostic_frame(CanId::Standard(0x7E0), &[0x02, 0x10, 0x01])
            .unwrap();
        assert!(device
            .receive_frame(Duration::from_millis(5))
            .unwrap()
            .is_none());
        device.close_route().unwrap();

        let trace = device.exchange_trace();
        assert_eq!(trace.receive_filters.len(), E);
        assert_eq!(trace.inbound_frames, 0);
        assert_eq!(
            trace.indications_by_status,
            BTreeMap::from([(Some(0x106), 1)])
        );
        let words = trace.to_string();
        assert!(
            words.contains("transmit acknowledged with status 0x00000100 eMsgQueued"),
            "{words}"
        );
        assert!(words.contains("1 with 0x00000106 iMsgTxDone"), "{words}");
    }
}
