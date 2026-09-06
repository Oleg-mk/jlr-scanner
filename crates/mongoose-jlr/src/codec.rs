use crate::ProtocolError;

const FRAME_HEADER_LENGTH: usize = 4;
const COMMAND_HEADER_LENGTH: usize = 12;
const VERIFIER_XOR: u16 = 0x51E6;
pub const MAX_PAYLOAD_LENGTH: usize = 0x1800;
pub const GET_BOARD_INFO_COMMAND: u16 = 0x0109;
pub const BOARD_INFO_RESPONSE_COMMAND: u16 = 0x8109;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    payload: Vec<u8>,
}

impl Frame {
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    fn encode(&self) -> Result<Vec<u8>, ProtocolError> {
        encode_payload(&self.payload)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MongoosePacket {
    route_a: u16,
    route_b: u16,
    opcode: u8,
    flag: u8,
    sequence: u16,
    field_08: u16,
    field_0a: u16,
    body: Vec<u8>,
}

impl MongoosePacket {
    pub fn get_board_info(sequence: u16) -> Result<Self, ProtocolError> {
        validate_sequence(sequence)?;
        Ok(Self {
            route_a: 1,
            route_b: 0,
            opcode: 0x09,
            flag: 0x01,
            sequence,
            field_08: 0,
            field_0a: 0,
            body: Vec::new(),
        })
    }

    pub fn from_frame(frame: &Frame) -> Result<Self, ProtocolError> {
        let payload = frame.payload();
        if payload.len() < COMMAND_HEADER_LENGTH {
            return Err(ProtocolError::PayloadTooShort(payload.len()));
        }
        Ok(Self {
            route_a: u16::from_le_bytes([payload[0], payload[1]]),
            route_b: u16::from_le_bytes([payload[2], payload[3]]),
            opcode: payload[4],
            flag: payload[5],
            sequence: u16::from_le_bytes([payload[6], payload[7]]),
            field_08: u16::from_le_bytes([payload[8], payload[9]]),
            field_0a: u16::from_le_bytes([payload[10], payload[11]]),
            body: payload[COMMAND_HEADER_LENGTH..].to_vec(),
        })
    }

    pub fn command(&self) -> u16 {
        u16::from(self.opcode) | (u16::from(self.flag) << 8)
    }

    pub fn sequence(&self) -> u16 {
        self.sequence
    }

    pub fn route_a(&self) -> u16 {
        self.route_a
    }

    pub fn route_b(&self) -> u16 {
        self.route_b
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn encode(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut payload = Vec::with_capacity(COMMAND_HEADER_LENGTH + self.body.len());
        payload.extend_from_slice(&self.route_a.to_le_bytes());
        payload.extend_from_slice(&self.route_b.to_le_bytes());
        payload.push(self.opcode);
        payload.push(self.flag);
        payload.extend_from_slice(&self.sequence.to_le_bytes());
        payload.extend_from_slice(&self.field_08.to_le_bytes());
        payload.extend_from_slice(&self.field_0a.to_le_bytes());
        payload.extend_from_slice(&self.body);
        Frame { payload }.encode()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardInfoResponse {
    pub route_a: u16,
    pub route_b: u16,
    pub response_command: u16,
    pub sequence: u16,
    pub raw_board_info: Vec<u8>,
}

impl BoardInfoResponse {
    /// Whether the adapter answered from its bootloader: the firmware's
    /// board-info carries a running tick at body offset 4, the bootloader's
    /// has zeros there (observed 2026-09-06, ADR-0018). A body too short to
    /// tell is taken as firmware, so nothing is sent on a guess.
    pub fn in_bootloader(&self) -> bool {
        self.raw_board_info.len() >= 8 && self.raw_board_info[4..8] == [0, 0, 0, 0]
    }

    pub(crate) fn from_frame(frame: &Frame, expected_sequence: u16) -> Result<Self, ProtocolError> {
        let packet = MongoosePacket::from_frame(frame)?;
        if packet.opcode != 0x09 || packet.flag != 0x81 {
            return Err(ProtocolError::UnexpectedResponse {
                opcode: packet.opcode,
                flag: packet.flag,
            });
        }
        validate_sequence(packet.sequence)?;
        if packet.sequence != expected_sequence {
            return Err(ProtocolError::SequenceMismatch {
                expected: expected_sequence,
                actual: packet.sequence,
            });
        }
        Ok(Self {
            route_a: packet.route_a,
            route_b: packet.route_b,
            response_command: packet.command(),
            sequence: packet.sequence,
            raw_board_info: packet.body,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SequenceCounter {
    next: u8,
}

impl Default for SequenceCounter {
    fn default() -> Self {
        Self { next: 1 }
    }
}

impl SequenceCounter {
    pub fn new(next: u8) -> Self {
        Self {
            next: if next == 0 { 1 } else { next },
        }
    }

    pub fn allocate(&mut self) -> u16 {
        let allocated = self.next;
        self.next = if self.next == u8::MAX {
            1
        } else {
            self.next + 1
        };
        u16::from(allocated)
    }
}

#[derive(Clone, Debug, Default)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
}

impl FrameDecoder {
    pub fn push(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    pub fn next_frame(&mut self) -> Result<Option<Frame>, ProtocolError> {
        if self.buffer.len() < FRAME_HEADER_LENGTH {
            return Ok(None);
        }
        let length = u16::from_le_bytes([self.buffer[0], self.buffer[1]]);
        let verifier = u16::from_le_bytes([self.buffer[2], self.buffer[3]]);
        if length == 0 || usize::from(length) > MAX_PAYLOAD_LENGTH {
            self.buffer.remove(0);
            return Err(ProtocolError::InvalidLength(length));
        }
        let expected_verifier = length ^ VERIFIER_XOR;
        if verifier != expected_verifier {
            self.buffer.remove(0);
            return Err(ProtocolError::InvalidVerifier {
                length,
                expected: expected_verifier,
                actual: verifier,
            });
        }
        let frame_length = FRAME_HEADER_LENGTH + usize::from(length);
        if self.buffer.len() < frame_length {
            return Ok(None);
        }
        let payload = self.buffer[FRAME_HEADER_LENGTH..frame_length].to_vec();
        self.buffer.drain(..frame_length);
        Ok(Some(Frame { payload }))
    }

    pub fn finish(self) -> Result<(), ProtocolError> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let expected = if self.buffer.len() >= FRAME_HEADER_LENGTH {
            FRAME_HEADER_LENGTH + usize::from(u16::from_le_bytes([self.buffer[0], self.buffer[1]]))
        } else {
            FRAME_HEADER_LENGTH
        };
        Err(ProtocolError::TruncatedFrame {
            expected,
            actual: self.buffer.len(),
        })
    }
}

fn encode_payload(payload: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    if payload.is_empty() || payload.len() > MAX_PAYLOAD_LENGTH {
        return Err(ProtocolError::PayloadTooLarge(payload.len()));
    }
    let length =
        u16::try_from(payload.len()).map_err(|_| ProtocolError::PayloadTooLarge(payload.len()))?;
    let mut bytes = Vec::with_capacity(FRAME_HEADER_LENGTH + payload.len());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(&(length ^ VERIFIER_XOR).to_le_bytes());
    bytes.extend_from_slice(payload);
    Ok(bytes)
}

fn validate_sequence(sequence: u16) -> Result<(), ProtocolError> {
    if sequence == 0 || sequence > u16::from(u8::MAX) {
        Err(ProtocolError::ZeroSequence)
    } else {
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn encode_board_info_response(sequence: u16, body: &[u8]) -> Vec<u8> {
    let packet = MongoosePacket {
        route_a: 0,
        route_b: 1,
        opcode: 0x09,
        flag: 0x81,
        sequence,
        field_08: 0,
        field_0a: 0,
        body: body.to_vec(),
    };
    packet.encode().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOLDEN_REQUEST: [u8; 16] = [
        0x0C, 0x00, 0xEA, 0x51, 0x01, 0x00, 0x00, 0x00, 0x09, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];

    #[test]
    fn exact_get_board_info_serialization_matches_physical_request() {
        assert_eq!(
            MongoosePacket::get_board_info(1).unwrap().encode().unwrap(),
            GOLDEN_REQUEST
        );
    }

    #[test]
    fn verifier_is_length_xor_51e6() {
        let encoded = MongoosePacket::get_board_info(1).unwrap().encode().unwrap();
        assert_eq!(
            u16::from_le_bytes([encoded[2], encoded[3]]),
            12 ^ VERIFIER_XOR
        );
    }

    #[test]
    fn wrong_verifier_is_rejected() {
        let mut encoded = GOLDEN_REQUEST;
        encoded[2] ^= 1;
        let mut decoder = FrameDecoder::default();
        decoder.push(&encoded);
        assert!(matches!(
            decoder.next_frame(),
            Err(ProtocolError::InvalidVerifier { .. })
        ));
    }

    #[test]
    fn truncated_frame_is_reported_at_end_of_stream() {
        let mut decoder = FrameDecoder::default();
        decoder.push(&GOLDEN_REQUEST[..10]);
        assert!(decoder.next_frame().unwrap().is_none());
        assert!(matches!(
            decoder.finish(),
            Err(ProtocolError::TruncatedFrame { .. })
        ));
    }

    #[test]
    fn fragmented_reads_form_one_frame() {
        let mut decoder = FrameDecoder::default();
        for byte in GOLDEN_REQUEST {
            decoder.push(&[byte]);
        }
        let packet = MongoosePacket::from_frame(&decoder.next_frame().unwrap().unwrap()).unwrap();
        assert_eq!(packet.command(), GET_BOARD_INFO_COMMAND);
        assert_eq!(packet.sequence(), 1);
    }

    #[test]
    fn two_frames_are_parsed_from_one_stream() {
        let mut stream = GOLDEN_REQUEST.to_vec();
        stream.extend_from_slice(&GOLDEN_REQUEST);
        let mut decoder = FrameDecoder::default();
        decoder.push(&stream);
        assert!(decoder.next_frame().unwrap().is_some());
        assert!(decoder.next_frame().unwrap().is_some());
        assert!(decoder.next_frame().unwrap().is_none());
    }

    #[test]
    fn invalid_lengths_are_rejected() {
        let mut zero = FrameDecoder::default();
        zero.push(&[0, 0, 0xE6, 0x51]);
        assert_eq!(zero.next_frame(), Err(ProtocolError::InvalidLength(0)));

        let length = (MAX_PAYLOAD_LENGTH as u16) + 1;
        let mut oversized = FrameDecoder::default();
        oversized.push(&[
            length.to_le_bytes()[0],
            length.to_le_bytes()[1],
            (length ^ VERIFIER_XOR).to_le_bytes()[0],
            (length ^ VERIFIER_XOR).to_le_bytes()[1],
        ]);
        assert_eq!(
            oversized.next_frame(),
            Err(ProtocolError::InvalidLength(length))
        );
    }

    #[test]
    fn sequence_rolls_over_to_one_and_never_emits_zero() {
        let mut sequences = SequenceCounter::new(0xFF);
        assert_eq!(sequences.allocate(), 0xFF);
        assert_eq!(sequences.allocate(), 1);
        for _ in 0..1024 {
            assert_ne!(sequences.allocate(), 0);
        }
    }

    fn physical_board_info_fixture() -> Vec<u8> {
        include_str!("../tests/fixtures/board_info_response.hex")
            .split_whitespace()
            .map(|byte| u8::from_str_radix(byte, 16).unwrap())
            .collect()
    }

    #[test]
    fn parses_exact_physical_184_byte_board_info_response() {
        let bytes = physical_board_info_fixture();
        assert_eq!(bytes.len(), 184);
        assert_eq!(&bytes[..4], &[0xB4, 0x00, 0x52, 0x51]);

        let mut decoder = FrameDecoder::default();
        decoder.push(&bytes);
        let frame = decoder.next_frame().unwrap().unwrap();
        assert!(decoder.next_frame().unwrap().is_none());
        let info = BoardInfoResponse::from_frame(&frame, 1).unwrap();

        assert_eq!(info.response_command, BOARD_INFO_RESPONSE_COMMAND);
        assert_eq!(info.sequence, 1);
        assert_eq!(info.raw_board_info.len(), 168);
        assert_eq!(
            &info.raw_board_info[..28],
            &[
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x02, 0x01, 0x01, 0xB4, 0xB0,
                0x00, 0x00, 0xD8, 0x55, 0x00, 0x00, 0x00, 0x08, 0x01, 0x01, 0x00, 0x10, 0x01, 0x01,
            ]
        );
        assert!(info.raw_board_info[28..].iter().all(|byte| *byte == 0));
    }
}
