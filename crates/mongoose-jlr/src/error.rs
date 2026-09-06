use std::fmt;
use transport_api::TransportError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    ActiveRoute,
    NoOpenRoute,
    RouteAlreadyOpen,
    PassiveRouteUnavailable {
        route: &'static str,
        reason: &'static str,
    },
    UnexpectedCommand {
        expected: u16,
        actual: u16,
    },
    DeviceStatus {
        operation: &'static str,
        status: u32,
        /// The firmware's own words after the status, when it gave any.
        message: String,
    },
    InvalidChannel(u16),
    /// The adapter acknowledged the jump from its bootloader but its
    /// firmware never answered board-info within the start timeout.
    FirmwareNotStarted,
    RouteMismatch {
        expected: u16,
        actual: u16,
    },
    InvalidCanDataSize(u16),
    InvalidCanId {
        arbitration_id: u32,
        extended: bool,
    },
    InvalidLength(u16),
    PayloadTooLarge(usize),
    PayloadTooShort(usize),
    InvalidVerifier {
        length: u16,
        expected: u16,
        actual: u16,
    },
    TruncatedFrame {
        expected: usize,
        actual: usize,
    },
    UnexpectedResponse {
        opcode: u8,
        flag: u8,
    },
    SequenceMismatch {
        expected: u16,
        actual: u16,
    },
    ZeroSequence,
    Transport(TransportError),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ActiveRoute => {
                formatter.write_str("close the passive receive route before closing the transport")
            }
            Self::NoOpenRoute => formatter.write_str("no passive receive route is open"),
            Self::RouteAlreadyOpen => {
                formatter.write_str("a passive receive route is already open")
            }
            Self::PassiveRouteUnavailable { route, reason } => {
                write!(formatter, "route {route} is not passive-openable: {reason}")
            }
            Self::UnexpectedCommand { expected, actual } => write!(
                formatter,
                "unexpected Mongoose command {actual:#06X}; expected {expected:#06X}"
            ),
            Self::DeviceStatus {
                operation,
                status,
                message,
            } => {
                if message.is_empty() {
                    write!(
                        formatter,
                        "{operation} failed with device status {status:#010X}"
                    )
                } else {
                    write!(
                        formatter,
                        "{operation} failed with device status {status:#010X}: {message}"
                    )
                }
            }
            Self::InvalidChannel(channel) => {
                write!(
                    formatter,
                    "invalid Mongoose channel identifier {channel:#06X}"
                )
            }
            Self::FirmwareNotStarted => formatter.write_str(
                "the adapter accepted the jump to its firmware but the firmware did not answer; unplug and replug the adapter, then try again",
            ),
            Self::RouteMismatch { expected, actual } => write!(
                formatter,
                "inbound route mismatch: expected {expected:#06X}, got {actual:#06X}"
            ),
            Self::InvalidCanDataSize(size) => {
                write!(
                    formatter,
                    "invalid inbound CAN data size {size}; expected 4..=12"
                )
            }
            Self::InvalidCanId {
                arbitration_id,
                extended,
            } => write!(
                formatter,
                "invalid {} CAN identifier {arbitration_id:#010X}",
                if *extended { "extended" } else { "standard" }
            ),
            Self::InvalidLength(length) => {
                write!(formatter, "invalid frame payload length {length}")
            }
            Self::PayloadTooLarge(length) => write!(formatter, "payload is too large: {length}"),
            Self::PayloadTooShort(length) => {
                write!(formatter, "command payload is too short: {length}")
            }
            Self::InvalidVerifier {
                length,
                expected,
                actual,
            } => write!(
                formatter,
                "invalid verifier for length {length}: expected {expected:#06X}, got {actual:#06X}"
            ),
            Self::TruncatedFrame { expected, actual } => {
                write!(
                    formatter,
                    "truncated frame: expected {expected} bytes, got {actual}"
                )
            }
            Self::UnexpectedResponse { opcode, flag } => {
                write!(
                    formatter,
                    "unexpected response command {opcode:02X} {flag:02X}"
                )
            }
            Self::SequenceMismatch { expected, actual } => {
                write!(
                    formatter,
                    "response sequence mismatch: expected {expected}, got {actual}"
                )
            }
            Self::ZeroSequence => formatter.write_str("zero sequence is invalid"),
            Self::Transport(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ProtocolError {}

impl From<TransportError> for ProtocolError {
    fn from(error: TransportError) -> Self {
        Self::Transport(error)
    }
}
