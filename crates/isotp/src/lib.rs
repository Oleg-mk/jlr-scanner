//! Vehicle-independent ISO 15765-2 transport core for classic CAN.
//!
//! F4 supports normal addressing. CAN identifier selection is deliberately
//! external to this crate.

use std::fmt;

const SINGLE_CAPACITY: usize = 7;
const FIRST_CAPACITY: usize = 6;
const CONSECUTIVE_CAPACITY: usize = 7;
const MAX_CLASSIC_PAYLOAD: usize = 0x0fff;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowStatus {
    ContinueToSend,
    Wait,
    Overflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlowControl {
    pub status: FlowStatus,
    pub block_size: u8,
    pub separation_time_us: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IsoTpFrame {
    Single(Vec<u8>),
    First { total_len: usize, initial: Vec<u8> },
    Consecutive { sequence: u8, payload: Vec<u8> },
    FlowControl(FlowControl),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IsoTpError {
    EmptyFrame,
    InvalidFrameType(u8),
    InvalidLength,
    PayloadTooLarge(usize),
    InvalidFlowStatus(u8),
    InvalidSeparationTime(u8),
    UnexpectedFrame,
    SequenceMismatch { expected: u8, actual: u8 },
    Timeout,
}

impl fmt::Display for IsoTpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFrame => formatter.write_str("empty ISO-TP frame"),
            Self::InvalidFrameType(kind) => {
                write!(formatter, "invalid ISO-TP frame type: {kind:#x}")
            }
            Self::InvalidLength => formatter.write_str("invalid ISO-TP length"),
            Self::PayloadTooLarge(length) => {
                write!(formatter, "ISO-TP payload exceeds classic limit: {length}")
            }
            Self::InvalidFlowStatus(status) => {
                write!(formatter, "invalid flow status: {status:#x}")
            }
            Self::InvalidSeparationTime(value) => {
                write!(formatter, "invalid STmin value: {value:#x}")
            }
            Self::UnexpectedFrame => formatter.write_str("unexpected ISO-TP frame"),
            Self::SequenceMismatch { expected, actual } => write!(
                formatter,
                "ISO-TP sequence mismatch: expected {expected:#x}, got {actual:#x}"
            ),
            Self::Timeout => formatter.write_str("ISO-TP session timed out"),
        }
    }
}
impl std::error::Error for IsoTpError {}

pub fn decode_frame(data: &[u8]) -> Result<IsoTpFrame, IsoTpError> {
    let pci = *data.first().ok_or(IsoTpError::EmptyFrame)?;
    match pci >> 4 {
        0x0 => {
            let length = (pci & 0x0f) as usize;
            if length == 0 || length > SINGLE_CAPACITY || data.len() < length + 1 {
                return Err(IsoTpError::InvalidLength);
            }
            Ok(IsoTpFrame::Single(data[1..=length].to_vec()))
        }
        0x1 => {
            if data.len() < 2 {
                return Err(IsoTpError::InvalidLength);
            }
            let total_len = (((pci & 0x0f) as usize) << 8) | data[1] as usize;
            if total_len <= SINGLE_CAPACITY || total_len > MAX_CLASSIC_PAYLOAD {
                return Err(IsoTpError::InvalidLength);
            }
            let available = (total_len.min(FIRST_CAPACITY)).min(data.len().saturating_sub(2));
            if available != total_len.min(FIRST_CAPACITY) {
                return Err(IsoTpError::InvalidLength);
            }
            Ok(IsoTpFrame::First {
                total_len,
                initial: data[2..2 + available].to_vec(),
            })
        }
        0x2 => {
            if data.len() < 2 {
                return Err(IsoTpError::InvalidLength);
            }
            Ok(IsoTpFrame::Consecutive {
                sequence: pci & 0x0f,
                payload: data[1..].to_vec(),
            })
        }
        0x3 => {
            if data.len() < 3 {
                return Err(IsoTpError::InvalidLength);
            }
            let status = match pci & 0x0f {
                0 => FlowStatus::ContinueToSend,
                1 => FlowStatus::Wait,
                2 => FlowStatus::Overflow,
                other => return Err(IsoTpError::InvalidFlowStatus(other)),
            };
            Ok(IsoTpFrame::FlowControl(FlowControl {
                status,
                block_size: data[1],
                separation_time_us: decode_st_min(data[2])?,
            }))
        }
        kind => Err(IsoTpError::InvalidFrameType(kind)),
    }
}

pub fn decode_st_min(value: u8) -> Result<u32, IsoTpError> {
    match value {
        0x00..=0x7f => Ok(value as u32 * 1_000),
        0xf1..=0xf9 => Ok((value as u32 - 0xf0) * 100),
        _ => Err(IsoTpError::InvalidSeparationTime(value)),
    }
}

fn padded(mut frame: Vec<u8>, padding: Option<u8>) -> Vec<u8> {
    if let Some(byte) = padding {
        frame.resize(8, byte);
    }
    frame
}

/// Deterministically segment a payload. Flow-control timing is enforced by
/// Sender when frames are emitted interactively.
pub fn segment(payload: &[u8], padding: Option<u8>) -> Result<Vec<Vec<u8>>, IsoTpError> {
    if payload.is_empty() {
        return Err(IsoTpError::InvalidLength);
    }
    if payload.len() > MAX_CLASSIC_PAYLOAD {
        return Err(IsoTpError::PayloadTooLarge(payload.len()));
    }
    if payload.len() <= SINGLE_CAPACITY {
        let mut frame = Vec::with_capacity(8);
        frame.push(payload.len() as u8);
        frame.extend_from_slice(payload);
        return Ok(vec![padded(frame, padding)]);
    }

    let mut frames = Vec::new();
    let mut first = vec![
        0x10 | ((payload.len() >> 8) as u8 & 0x0f),
        payload.len() as u8,
    ];
    first.extend_from_slice(&payload[..FIRST_CAPACITY]);
    frames.push(padded(first, padding));
    let mut sequence = 1u8;
    for chunk in payload[FIRST_CAPACITY..].chunks(CONSECUTIVE_CAPACITY) {
        let mut frame = vec![0x20 | sequence];
        frame.extend_from_slice(chunk);
        frames.push(padded(frame, padding));
        sequence = (sequence + 1) & 0x0f;
    }
    Ok(frames)
}

#[derive(Clone, Debug)]
struct PartialMessage {
    total_len: usize,
    payload: Vec<u8>,
    next_sequence: u8,
    last_timestamp_us: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Reassembler {
    partial: Option<PartialMessage>,
}

impl Reassembler {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn is_pending(&self) -> bool {
        self.partial.is_some()
    }

    pub fn accept(
        &mut self,
        data: &[u8],
        timestamp_us: u64,
    ) -> Result<Option<Vec<u8>>, IsoTpError> {
        match decode_frame(data)? {
            IsoTpFrame::Single(payload) => {
                if self.partial.is_some() {
                    return Err(IsoTpError::UnexpectedFrame);
                }
                Ok(Some(payload))
            }
            IsoTpFrame::First { total_len, initial } => {
                if self.partial.is_some() {
                    return Err(IsoTpError::UnexpectedFrame);
                }
                self.partial = Some(PartialMessage {
                    total_len,
                    payload: initial,
                    next_sequence: 1,
                    last_timestamp_us: timestamp_us,
                });
                Ok(None)
            }
            IsoTpFrame::Consecutive { sequence, payload } => {
                let partial = self.partial.as_mut().ok_or(IsoTpError::UnexpectedFrame)?;
                if sequence != partial.next_sequence {
                    return Err(IsoTpError::SequenceMismatch {
                        expected: partial.next_sequence,
                        actual: sequence,
                    });
                }
                let remaining = partial.total_len - partial.payload.len();
                partial
                    .payload
                    .extend_from_slice(&payload[..payload.len().min(remaining)]);
                partial.next_sequence = (partial.next_sequence + 1) & 0x0f;
                partial.last_timestamp_us = timestamp_us;
                if partial.payload.len() == partial.total_len {
                    return Ok(Some(self.partial.take().expect("partial exists").payload));
                }
                Ok(None)
            }
            IsoTpFrame::FlowControl(_) => Err(IsoTpError::UnexpectedFrame),
        }
    }

    pub fn expire(&mut self, now_us: u64, timeout_us: u64) -> Result<(), IsoTpError> {
        if self
            .partial
            .as_ref()
            .is_some_and(|partial| now_us.saturating_sub(partial.last_timestamp_us) > timeout_us)
        {
            self.partial = None;
            return Err(IsoTpError::Timeout);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SenderState {
    WaitingForFlowControl,
    Sending,
    Complete,
    Overflow,
}

#[derive(Clone, Debug)]
pub struct Sender {
    frames: Vec<Vec<u8>>,
    cursor: usize,
    state: SenderState,
    block_remaining: Option<u8>,
    separation_time_us: u32,
    next_due_us: u64,
    last_activity_us: u64,
}

impl Sender {
    pub fn start(
        payload: &[u8],
        padding: Option<u8>,
        now_us: u64,
    ) -> Result<(Self, Vec<u8>), IsoTpError> {
        let frames = segment(payload, padding)?;
        let state = if frames.len() == 1 {
            SenderState::Complete
        } else {
            SenderState::WaitingForFlowControl
        };
        let first = frames[0].clone();
        Ok((
            Self {
                frames,
                cursor: 1,
                state,
                block_remaining: None,
                separation_time_us: 0,
                next_due_us: now_us,
                last_activity_us: now_us,
            },
            first,
        ))
    }

    pub fn on_flow_control(&mut self, data: &[u8], now_us: u64) -> Result<(), IsoTpError> {
        let IsoTpFrame::FlowControl(flow) = decode_frame(data)? else {
            return Err(IsoTpError::UnexpectedFrame);
        };
        self.last_activity_us = now_us;
        match flow.status {
            FlowStatus::ContinueToSend => {
                self.state = SenderState::Sending;
                self.block_remaining = (flow.block_size != 0).then_some(flow.block_size);
                self.separation_time_us = flow.separation_time_us;
                self.next_due_us = now_us;
            }
            FlowStatus::Wait => self.state = SenderState::WaitingForFlowControl,
            FlowStatus::Overflow => self.state = SenderState::Overflow,
        }
        Ok(())
    }

    pub fn next_consecutive(&mut self, now_us: u64) -> Result<Option<Vec<u8>>, IsoTpError> {
        if self.state == SenderState::Overflow {
            return Err(IsoTpError::PayloadTooLarge(self.frames.len()));
        }
        if self.state != SenderState::Sending || now_us < self.next_due_us {
            return Ok(None);
        }
        let Some(frame) = self.frames.get(self.cursor).cloned() else {
            self.state = SenderState::Complete;
            return Ok(None);
        };
        self.cursor += 1;
        self.last_activity_us = now_us;
        self.next_due_us = now_us + self.separation_time_us as u64;
        if self.cursor == self.frames.len() {
            self.state = SenderState::Complete;
        } else if let Some(remaining) = self.block_remaining.as_mut() {
            *remaining -= 1;
            if *remaining == 0 {
                self.state = SenderState::WaitingForFlowControl;
            }
        }
        Ok(Some(frame))
    }

    pub fn expire(&mut self, now_us: u64, timeout_us: u64) -> Result<(), IsoTpError> {
        if self.state != SenderState::Complete
            && now_us.saturating_sub(self.last_activity_us) > timeout_us
        {
            return Err(IsoTpError::Timeout);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_frame_round_trip_and_padding() {
        let frames = segment(&[0x22, 0x12, 0x34], Some(0xaa)).unwrap();
        assert_eq!(frames[0].len(), 8);
        assert_eq!(
            Reassembler::new().accept(&frames[0], 0).unwrap(),
            Some(vec![0x22, 0x12, 0x34])
        );
    }

    #[test]
    fn multi_frame_round_trip_and_sequence_wrap() {
        let payload: Vec<u8> = (0..120).collect();
        let frames = segment(&payload, None).unwrap();
        let mut rx = Reassembler::new();
        let mut result = None;
        for (index, frame) in frames.iter().enumerate() {
            result = rx.accept(frame, index as u64).unwrap();
        }
        assert_eq!(result, Some(payload));
    }

    #[test]
    fn detects_wrong_sequence_and_missing_first_frame() {
        let mut rx = Reassembler::new();
        assert_eq!(rx.accept(&[0x21, 1], 0), Err(IsoTpError::UnexpectedFrame));
        rx.accept(&[0x10, 0x08, 1, 2, 3, 4, 5, 6], 0).unwrap();
        assert_eq!(
            rx.accept(&[0x22, 7, 8], 1),
            Err(IsoTpError::SequenceMismatch {
                expected: 1,
                actual: 2
            })
        );
    }

    #[test]
    fn detects_malformed_lengths_and_oversize() {
        assert_eq!(decode_frame(&[0x07, 1]), Err(IsoTpError::InvalidLength));
        assert_eq!(
            decode_frame(&[0x10, 0x07, 1, 2, 3, 4, 5, 6]),
            Err(IsoTpError::InvalidLength)
        );
        assert_eq!(
            segment(&vec![0; 4096], None),
            Err(IsoTpError::PayloadTooLarge(4096))
        );
    }

    #[test]
    fn timeout_clears_partial_message() {
        let mut rx = Reassembler::new();
        rx.accept(&[0x10, 0x08, 1, 2, 3, 4, 5, 6], 10).unwrap();
        assert_eq!(rx.expire(111, 100), Err(IsoTpError::Timeout));
        assert!(!rx.is_pending());
    }

    #[test]
    fn flow_control_decodes_block_size_and_stmin() {
        assert_eq!(
            decode_frame(&[0x30, 3, 0xf3]).unwrap(),
            IsoTpFrame::FlowControl(FlowControl {
                status: FlowStatus::ContinueToSend,
                block_size: 3,
                separation_time_us: 300,
            })
        );
        assert!(decode_st_min(0x80).is_err());
    }

    #[test]
    fn sender_honors_flow_control_block_and_separation() {
        let payload: Vec<u8> = (0..30).collect();
        let (mut tx, first) = Sender::start(&payload, None, 0).unwrap();
        assert_eq!(first[0] >> 4, 1);
        assert_eq!(tx.next_consecutive(0).unwrap(), None);
        tx.on_flow_control(&[0x30, 2, 1], 10).unwrap();
        assert!(tx.next_consecutive(10).unwrap().is_some());
        assert_eq!(tx.next_consecutive(500).unwrap(), None);
        assert!(tx.next_consecutive(1_010).unwrap().is_some());
        assert_eq!(tx.next_consecutive(2_010).unwrap(), None);
        tx.on_flow_control(&[0x30, 0, 0], 2_020).unwrap();
        assert!(tx.next_consecutive(2_020).unwrap().is_some());
    }
}
