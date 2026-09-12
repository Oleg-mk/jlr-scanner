//! BMW DS2 over K-line, as far as this product speaks it (ADR-0029).
//!
//! DS2 is the diagnostic protocol of the BMW-era body electronics of the
//! Range Rover L322 of 2006–2007: a byte protocol on a single K-line at
//! 9600 baud, 8 data bits, even parity. A frame is the module's node
//! address, the length of the whole frame, the data, and an XOR checksum
//! over everything before it. The module answers in the same shape, its
//! first data byte saying whether the request was accepted.
//!
//! Two requests exist here: the identification (`0x00`) and the fault
//! memory (`0x04`). The clear-fault request (`0x05`) and every other
//! command are deliberately absent, the frame encoder is private so that
//! no caller can build a command this crate does not name, and the
//! architecture check keeps both so. Nothing here interprets a module's
//! data beyond the frame: the bytes, their printable text, and the
//! module's own acceptance byte. Which words of a fault memory are codes
//! is the module's own layout, and ADR-0029 leaves it to the day a module
//! answers.

use std::fmt;

/// The protocol family name the platform documents use for these buses.
pub const PROTOCOL_FAMILY: &str = "DS2";
/// The read-only capability the knowledge base records for a module that
/// answers the identification request.
pub const ECU_IDENTIFICATION_CAPABILITY: &str = "ds2.ecu_identification.read_only";
/// The read-only capability for the fault-memory request.
pub const FAULT_MEMORY_CAPABILITY: &str = "ds2.fault_memory.read_only";

/// The identification request: what the module says it is.
pub const COMMAND_IDENTIFICATION: u8 = 0x00;
/// The fault-memory request: the codes the module holds.
pub const COMMAND_FAULT_MEMORY: u8 = 0x04;
/// The first data byte of a reply to a request the module accepted.
pub const REPLY_ACCEPTED: u8 = 0xA0;
/// The physical layer DS2 is specified on. The bus record from the platform
/// document is what a line is opened with; these are what the document is
/// checked against, and a difference is said rather than resolved.
pub const NOMINAL_BAUD: u32 = 9600;
/// The byte framing of the same specification, in the words the platform
/// ingest records under `sdd_iso_settings`.
pub const NOMINAL_FRAMING: &str = "data_bits=8;parity=even;stop_bits=1";
/// The shortest frame: address, length and checksum.
pub const MIN_FRAME_LEN: usize = 3;

/// A request this product makes: one of the two named commands, framed for
/// one node address. There is no constructor for any other command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ds2Request {
    bytes: Vec<u8>,
}

impl Ds2Request {
    /// The identification request to one module.
    pub fn ecu_identification(node_address: u8) -> Self {
        Self {
            bytes: encode(node_address, &[COMMAND_IDENTIFICATION]),
        }
    }

    /// The fault-memory request to one module.
    pub fn fault_memory(node_address: u8) -> Self {
        Self {
            bytes: encode(node_address, &[COMMAND_FAULT_MEMORY]),
        }
    }

    pub fn node_address(&self) -> u8 {
        self.bytes[0]
    }

    pub fn command(&self) -> u8 {
        self.bytes[2]
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// The frame for one command with its data: address, length, data, XOR.
/// Private on purpose — see the crate documentation.
fn encode(node_address: u8, data: &[u8]) -> Vec<u8> {
    let length = MIN_FRAME_LEN + data.len();
    debug_assert!(
        length <= u8::MAX as usize,
        "a DS2 frame is at most 255 bytes"
    );
    let mut bytes = Vec::with_capacity(length);
    bytes.push(node_address);
    bytes.push(length as u8);
    bytes.extend_from_slice(data);
    bytes.push(xor_checksum(&bytes));
    bytes
}

/// The XOR of every byte.
pub fn xor_checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0, |checksum, byte| checksum ^ byte)
}

/// One frame as a module sent it, checked: the node address and the data
/// between the length and the checksum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ds2Frame {
    pub node_address: u8,
    pub data: Vec<u8>,
}

impl Ds2Frame {
    /// Whether the module accepted the request: its first data byte is
    /// `0xA0`.
    pub fn accepted(&self) -> bool {
        self.data.first() == Some(&REPLY_ACCEPTED)
    }

    /// The module's first data byte, as its own word on the request.
    pub fn status(&self) -> Option<u8> {
        self.data.first().copied()
    }

    /// The data after the acceptance byte when the request was accepted;
    /// the whole data otherwise, so that nothing the module said is lost.
    pub fn payload(&self) -> &[u8] {
        if self.accepted() {
            &self.data[1..]
        } else {
            &self.data
        }
    }
}

/// The frame at the head of a byte stream, and how many bytes it took.
pub fn parse_leading_frame(bytes: &[u8]) -> Result<(Ds2Frame, usize), Ds2Error> {
    if bytes.len() < MIN_FRAME_LEN {
        return Err(Ds2Error::TooShort(bytes.len()));
    }
    let declared = bytes[1];
    let length = declared as usize;
    if length < MIN_FRAME_LEN {
        return Err(Ds2Error::LengthTooSmall(declared));
    }
    if bytes.len() < length {
        return Err(Ds2Error::Incomplete {
            declared,
            available: bytes.len(),
        });
    }
    let frame = &bytes[..length];
    let expected = xor_checksum(&frame[..length - 1]);
    let actual = frame[length - 1];
    if expected != actual {
        return Err(Ds2Error::Checksum { expected, actual });
    }
    Ok((
        Ds2Frame {
            node_address: frame[0],
            data: frame[2..length - 1].to_vec(),
        },
        length,
    ))
}

/// Exactly one frame, the whole of the bytes.
pub fn parse_frame(bytes: &[u8]) -> Result<Ds2Frame, Ds2Error> {
    let (frame, consumed) = parse_leading_frame(bytes)?;
    if consumed != bytes.len() {
        return Err(Ds2Error::TrailingBytes(bytes.len() - consumed));
    }
    Ok(frame)
}

/// Every frame in a stream, in order. On a single-wire K-line the tester's
/// own request comes back as an echo before the module's answer;
/// [`strip_echo`] removes it first.
pub fn split_frames(bytes: &[u8]) -> Result<Vec<Ds2Frame>, Ds2Error> {
    let mut frames = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let (frame, consumed) = parse_leading_frame(&bytes[offset..])?;
        frames.push(frame);
        offset += consumed;
    }
    Ok(frames)
}

/// The stream with the echo of `request` removed from its head when it is
/// there, and unchanged when it is not.
pub fn strip_echo<'a>(stream: &'a [u8], request: &Ds2Request) -> &'a [u8] {
    let echo = request.as_bytes();
    if stream.starts_with(echo) {
        &stream[echo.len()..]
    } else {
        stream
    }
}

/// The printable text of a payload: ASCII `0x20..=0x7E` as it is, every
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

/// An identification as this product shows it: the bytes and their text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ds2Identification {
    pub node_address: u8,
    pub bytes: Vec<u8>,
    pub text: String,
}

/// The identification in an accepted reply; a reply the module did not
/// accept is an error carrying the module's own status byte.
pub fn identification(frame: &Ds2Frame) -> Result<Ds2Identification, Ds2Error> {
    let payload = accepted_payload(frame)?;
    Ok(Ds2Identification {
        node_address: frame.node_address,
        bytes: payload.to_vec(),
        text: printable_text(payload),
    })
}

/// A fault memory as the module hands it over: the bytes. Their layout is
/// the module's own; the join with SDD's fault index happens where the
/// index is, one word at a time, and a word the index does not hold stays
/// raw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ds2FaultMemory {
    pub node_address: u8,
    pub bytes: Vec<u8>,
}

/// The fault memory in an accepted reply.
pub fn fault_memory(frame: &Ds2Frame) -> Result<Ds2FaultMemory, Ds2Error> {
    let payload = accepted_payload(frame)?;
    Ok(Ds2FaultMemory {
        node_address: frame.node_address,
        bytes: payload.to_vec(),
    })
}

/// The 16-bit words of a payload read big-endian from `offset`, as the
/// candidates a fault index is asked about. A trailing odd byte is not a
/// word and is left out.
pub fn words_from(bytes: &[u8], offset: usize) -> Vec<u16> {
    bytes
        .get(offset..)
        .unwrap_or(&[])
        .chunks_exact(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
        .collect()
}

fn accepted_payload(frame: &Ds2Frame) -> Result<&[u8], Ds2Error> {
    match frame.status() {
        Some(REPLY_ACCEPTED) => Ok(frame.payload()),
        Some(status) => Err(Ds2Error::NotAccepted { status }),
        None => Err(Ds2Error::EmptyReply),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ds2Error {
    /// Fewer bytes than the shortest frame.
    TooShort(usize),
    /// The length byte names fewer bytes than a frame has.
    LengthTooSmall(u8),
    /// The length byte names more bytes than arrived.
    Incomplete {
        declared: u8,
        available: usize,
    },
    Checksum {
        expected: u8,
        actual: u8,
    },
    /// Bytes after the one frame that was asked for.
    TrailingBytes(usize),
    /// The module answered without accepting; the byte is its own word.
    NotAccepted {
        status: u8,
    },
    /// A frame with no data at all, where an answer was expected.
    EmptyReply,
}

impl fmt::Display for Ds2Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort(len) => write!(f, "DS2 frame too short: {len} bytes"),
            Self::LengthTooSmall(declared) => {
                write!(f, "DS2 length byte {declared} is below the shortest frame")
            }
            Self::Incomplete {
                declared,
                available,
            } => write!(
                f,
                "DS2 frame incomplete: {declared} bytes declared, {available} available"
            ),
            Self::Checksum { expected, actual } => write!(
                f,
                "DS2 checksum mismatch: expected 0x{expected:02X}, got 0x{actual:02X}"
            ),
            Self::TrailingBytes(count) => write!(f, "{count} bytes after the DS2 frame"),
            Self::NotAccepted { status } => write!(
                f,
                "the module did not accept the request; its status byte is 0x{status:02X}"
            ),
            Self::EmptyReply => write!(f, "the module answered with an empty frame"),
        }
    }
}

impl std::error::Error for Ds2Error {}
