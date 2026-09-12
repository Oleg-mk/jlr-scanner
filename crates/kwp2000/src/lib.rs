//! KWP2000 — ISO 14230 over K-line — as far as this product speaks it
//! (ADR-0029).
//!
//! The ABS of the Defender L316 answers KWP2000 on J1962 pin 7 at
//! 10400 baud after a fast initialisation. A message is a format byte
//! carrying the addressing mode and the length, the target and source
//! addresses, the service bytes and a sum checksum; a length that does not
//! fit the format byte follows the addresses as a byte of its own.
//!
//! Three requests exist here: `StartCommunication 0x81`, the link handshake
//! ISO 14230-2 requires after the fast initialisation and before anything
//! else — a handshake, not a diagnostic session; `ReadEcuIdentification
//! 0x1A`; and `ReadDiagnosticTroubleCodesByStatus 0x18`. No session control,
//! no tester-present, no clear (`0x14`), no security access, no routine, no
//! write of any kind: the encoder is private so that no caller can frame a
//! service this crate does not name, and the architecture check lists the
//! forbidden constructors. Nothing here interprets what a module says
//! beyond the message: an identification is the record's bytes and their
//! printable text; a fault list is code and status byte, joined to SDD's
//! index where the index is.

use std::fmt;

/// The protocol family name the platform documents use for this bus.
pub const PROTOCOL_FAMILY: &str = "KW2000";
/// The read-only capability the knowledge base records for a module that
/// answers `ReadEcuIdentification`.
pub const READ_ECU_IDENTIFICATION_CAPABILITY: &str =
    "kwp2000.service1a.read_ecu_identification.read_only";
/// The read-only capability for `ReadDiagnosticTroubleCodesByStatus`.
pub const READ_DTC_BY_STATUS_CAPABILITY: &str = "kwp2000.service18.read_dtc_by_status.read_only";

/// The tester's own address on the link, as ISO 14230 gives it.
pub const TESTER_ADDRESS: u8 = 0xF1;
/// `StartCommunication`: the link handshake after the fast initialisation.
pub const SID_START_COMMUNICATION: u8 = 0x81;
/// `ReadEcuIdentification`, by identification option.
pub const SID_READ_ECU_IDENTIFICATION: u8 = 0x1A;
/// `ReadDiagnosticTroubleCodesByStatus`, by status mask and group.
pub const SID_READ_DTC_BY_STATUS: u8 = 0x18;
/// The first byte of a negative response.
pub const NEGATIVE_RESPONSE_SID: u8 = 0x7F;
/// A positive response carries the request's service id plus this.
pub const POSITIVE_RESPONSE_OFFSET: u8 = 0x40;
/// The response code that says the module has the request and answers later.
pub const RESPONSE_PENDING: u8 = 0x78;
/// The group of fault codes that means every group.
pub const ALL_DTC_GROUPS: u16 = 0xFF00;
/// Fast initialisation (ISO 14230-2): the line held low, then high, each
/// for 25 ms, and then `StartCommunication`.
pub const FAST_INIT_LOW_MS: u32 = 25;
pub const FAST_INIT_HIGH_MS: u32 = 25;
/// The physical layer the L316 document states for this bus; the bus record
/// is what a line is opened with, this is what it is checked against.
pub const NOMINAL_BAUD: u32 = 10_400;
/// Format byte with address information present and physical addressing.
const FORMAT_PHYSICAL: u8 = 0x80;
/// The bit that says the header carries target and source addresses.
const FORMAT_ADDRESS_INFORMATION: u8 = 0x80;
const FORMAT_LENGTH_MASK: u8 = 0x3F;
/// Format byte, target, source and checksum around an empty payload.
pub const MIN_MESSAGE_LEN: usize = 4;

/// A request this product makes: one of the three named services, framed
/// with physical addressing from the tester to one module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KwpRequest {
    bytes: Vec<u8>,
}

impl KwpRequest {
    /// The link handshake to one module, sent right after the fast
    /// initialisation. The module answers with its two key bytes.
    pub fn start_communication(target: u8) -> Self {
        Self {
            bytes: encode_physical(target, &[SID_START_COMMUNICATION]),
        }
    }

    /// One identification record, by the option the module documents.
    pub fn read_ecu_identification(target: u8, identification_option: u8) -> Self {
        Self {
            bytes: encode_physical(
                target,
                &[SID_READ_ECU_IDENTIFICATION, identification_option],
            ),
        }
    }

    /// The fault codes matching a status mask in a group of codes;
    /// [`ALL_DTC_GROUPS`] asks for every group.
    pub fn read_dtc_by_status(target: u8, status_mask: u8, group: u16) -> Self {
        let [group_high, group_low] = group.to_be_bytes();
        Self {
            bytes: encode_physical(
                target,
                &[SID_READ_DTC_BY_STATUS, status_mask, group_high, group_low],
            ),
        }
    }

    pub fn target(&self) -> u8 {
        self.bytes[1]
    }

    pub fn source(&self) -> u8 {
        self.bytes[2]
    }

    pub fn service_id(&self) -> u8 {
        self.bytes[3]
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// A physically addressed message from the tester with a short payload.
/// Private on purpose — see the crate documentation.
fn encode_physical(target: u8, payload: &[u8]) -> Vec<u8> {
    debug_assert!(
        payload.len() <= FORMAT_LENGTH_MASK as usize,
        "every request this crate makes fits the format byte"
    );
    let mut bytes = Vec::with_capacity(MIN_MESSAGE_LEN + payload.len());
    bytes.push(FORMAT_PHYSICAL | payload.len() as u8);
    bytes.push(target);
    bytes.push(TESTER_ADDRESS);
    bytes.extend_from_slice(payload);
    bytes.push(sum_checksum(&bytes));
    bytes
}

/// The sum of every byte, modulo 256.
pub fn sum_checksum(bytes: &[u8]) -> u8 {
    bytes
        .iter()
        .fold(0u8, |checksum, byte| checksum.wrapping_add(*byte))
}

/// One message as it arrived, checked: who it is to, who it is from, and
/// the service bytes between the header and the checksum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KwpMessage {
    pub target: u8,
    pub source: u8,
    pub payload: Vec<u8>,
}

/// The message at the head of a byte stream, and how many bytes it took.
/// Both length forms are read; a format byte without address information
/// is refused, because this product only ever speaks with addresses.
pub fn parse_leading_message(bytes: &[u8]) -> Result<(KwpMessage, usize), KwpError> {
    if bytes.len() < MIN_MESSAGE_LEN {
        return Err(KwpError::TooShort(bytes.len()));
    }
    let format = bytes[0];
    if format & FORMAT_ADDRESS_INFORMATION == 0 {
        return Err(KwpError::NoAddressInformation(format));
    }
    let (length, header_len) = match format & FORMAT_LENGTH_MASK {
        0 => (bytes[3] as usize, 4),
        short => (short as usize, 3),
    };
    let total = header_len + length + 1;
    if bytes.len() < total {
        return Err(KwpError::Incomplete {
            declared: total,
            available: bytes.len(),
        });
    }
    let message = &bytes[..total];
    let expected = sum_checksum(&message[..total - 1]);
    let actual = message[total - 1];
    if expected != actual {
        return Err(KwpError::Checksum { expected, actual });
    }
    Ok((
        KwpMessage {
            target: message[1],
            source: message[2],
            payload: message[header_len..total - 1].to_vec(),
        },
        total,
    ))
}

/// Exactly one message, the whole of the bytes.
pub fn parse_message(bytes: &[u8]) -> Result<KwpMessage, KwpError> {
    let (message, consumed) = parse_leading_message(bytes)?;
    if consumed != bytes.len() {
        return Err(KwpError::TrailingBytes(bytes.len() - consumed));
    }
    Ok(message)
}

/// Every message in a stream, in order. On a single-wire K-line the
/// tester's own request comes back as an echo before the module's answer;
/// [`strip_echo`] removes it first.
pub fn split_messages(bytes: &[u8]) -> Result<Vec<KwpMessage>, KwpError> {
    let mut messages = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let (message, consumed) = parse_leading_message(&bytes[offset..])?;
        messages.push(message);
        offset += consumed;
    }
    Ok(messages)
}

/// The stream with the echo of `request` removed from its head when it is
/// there, and unchanged when it is not.
pub fn strip_echo<'a>(stream: &'a [u8], request: &KwpRequest) -> &'a [u8] {
    let echo = request.as_bytes();
    if stream.starts_with(echo) {
        &stream[echo.len()..]
    } else {
        stream
    }
}

/// What a message says about the request it answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KwpReply {
    /// The service's own answer, after the response service id.
    Positive { service: u8, data: Vec<u8> },
    /// The module refused or deferred; the code is its own word.
    Negative { service: u8, code: u8 },
}

/// The reply in a message to a request for `service`. A positive response
/// to another service, or a negative one about another service, is not the
/// answer to this request and is said to be so.
pub fn interpret(message: &KwpMessage, service: u8) -> Result<KwpReply, KwpError> {
    let Some(first) = message.payload.first().copied() else {
        return Err(KwpError::EmptyPayload);
    };
    if first == NEGATIVE_RESPONSE_SID {
        let (Some(about), Some(code)) = (message.payload.get(1), message.payload.get(2)) else {
            return Err(KwpError::Malformed(
                "a negative response is service and code",
            ));
        };
        if *about != service {
            return Err(KwpError::UnexpectedService {
                expected: service,
                actual: *about,
            });
        }
        return Ok(KwpReply::Negative {
            service,
            code: *code,
        });
    }
    let expected = service.wrapping_add(POSITIVE_RESPONSE_OFFSET);
    if first != expected {
        return Err(KwpError::UnexpectedService {
            expected,
            actual: first,
        });
    }
    Ok(KwpReply::Positive {
        service,
        data: message.payload[1..].to_vec(),
    })
}

/// The module's side of the handshake: its two key bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartCommunicationReply {
    pub key_bytes: [u8; 2],
}

pub fn start_communication_reply(reply: &KwpReply) -> Result<StartCommunicationReply, KwpError> {
    let data = positive_data(reply)?;
    match data {
        [first, second, ..] => Ok(StartCommunicationReply {
            key_bytes: [*first, *second],
        }),
        _ => Err(KwpError::Malformed(
            "StartCommunication answers with two key bytes",
        )),
    }
}

/// An identification record as this product shows it: the option the
/// module echoes, the bytes, and their printable text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EcuIdentification {
    pub option: u8,
    pub bytes: Vec<u8>,
    pub text: String,
}

pub fn ecu_identification(reply: &KwpReply) -> Result<EcuIdentification, KwpError> {
    let data = positive_data(reply)?;
    let Some((option, record)) = data.split_first() else {
        return Err(KwpError::Malformed(
            "ReadEcuIdentification answers with the option and the record",
        ));
    };
    Ok(EcuIdentification {
        option: *option,
        bytes: record.to_vec(),
        text: printable_text(record),
    })
}

/// One fault code with the status byte the module gives beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DtcByStatus {
    pub code: u16,
    pub status: u8,
}

/// The fault list: the module's count, then code high, code low and status
/// for each. A list whose count does not match its bytes is malformed and
/// is not guessed at.
pub fn dtc_by_status(reply: &KwpReply) -> Result<Vec<DtcByStatus>, KwpError> {
    let data = positive_data(reply)?;
    let Some((count, records)) = data.split_first() else {
        return Err(KwpError::Malformed(
            "ReadDiagnosticTroubleCodesByStatus answers with a count",
        ));
    };
    if records.len() != *count as usize * 3 {
        return Err(KwpError::Malformed(
            "the fault list does not hold three bytes per counted code",
        ));
    }
    Ok(records
        .chunks_exact(3)
        .map(|record| DtcByStatus {
            code: u16::from_be_bytes([record[0], record[1]]),
            status: record[2],
        })
        .collect())
}

/// The printable text of a record: ASCII `0x20..=0x7E` as it is, every
/// other byte as `.`. Nothing is parsed out of it.
pub fn printable_text(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| match byte {
            0x20..=0x7E => *byte as char,
            _ => '.',
        })
        .collect()
}

/// ISO 14230-3's name for a response code, where this product has one.
pub fn response_code_text(code: u8) -> &'static str {
    match code {
        0x10 => "general reject",
        0x11 => "service not supported",
        0x12 => "sub-function not supported or invalid format",
        0x21 => "busy, repeat the request",
        0x22 => "conditions not correct or request sequence error",
        0x31 => "request out of range",
        0x33 => "security access denied",
        RESPONSE_PENDING => "request correctly received, response pending",
        _ => "a response code this product does not name",
    }
}

fn positive_data(reply: &KwpReply) -> Result<&[u8], KwpError> {
    match reply {
        KwpReply::Positive { data, .. } => Ok(data),
        KwpReply::Negative { service, code } => Err(KwpError::Refused {
            service: *service,
            code: *code,
        }),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KwpError {
    /// Fewer bytes than the shortest message.
    TooShort(usize),
    /// A format byte without target and source addresses.
    NoAddressInformation(u8),
    /// The length names more bytes than arrived.
    Incomplete {
        declared: usize,
        available: usize,
    },
    Checksum {
        expected: u8,
        actual: u8,
    },
    /// Bytes after the one message that was asked for.
    TrailingBytes(usize),
    /// A message with no service byte at all.
    EmptyPayload,
    /// An answer to some other service.
    UnexpectedService {
        expected: u8,
        actual: u8,
    },
    /// The module answered negatively; the code is its own word.
    Refused {
        service: u8,
        code: u8,
    },
    Malformed(&'static str),
}

impl fmt::Display for KwpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort(len) => write!(f, "KWP message too short: {len} bytes"),
            Self::NoAddressInformation(format) => write!(
                f,
                "KWP format byte 0x{format:02X} carries no target and source address"
            ),
            Self::Incomplete {
                declared,
                available,
            } => write!(
                f,
                "KWP message incomplete: {declared} bytes declared, {available} available"
            ),
            Self::Checksum { expected, actual } => write!(
                f,
                "KWP checksum mismatch: expected 0x{expected:02X}, got 0x{actual:02X}"
            ),
            Self::TrailingBytes(count) => write!(f, "{count} bytes after the KWP message"),
            Self::EmptyPayload => write!(f, "the KWP message carries no service byte"),
            Self::UnexpectedService { expected, actual } => write!(
                f,
                "expected a response to service 0x{expected:02X}, got 0x{actual:02X}"
            ),
            Self::Refused { service, code } => write!(
                f,
                "service 0x{service:02X} refused with code 0x{code:02X}: {}",
                response_code_text(*code)
            ),
            Self::Malformed(what) => write!(f, "malformed KWP response: {what}"),
        }
    }
}

impl std::error::Error for KwpError {}
