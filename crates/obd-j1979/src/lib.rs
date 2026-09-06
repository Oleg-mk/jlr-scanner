//! Minimal, vehicle-independent SAE J1979 codec for the F7 read-only slice.
//!
//! This crate intentionally does not model UDS, transport, vehicle knowledge,
//! arbitrary modes, or caller-provided payloads.

use std::fmt;

pub const CALIBRATION_FIELD_LEN: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum J1979Request {
    CalibrationIdentification,
}

impl J1979Request {
    pub const fn calibration_identification() -> Self {
        Self::CalibrationIdentification
    }

    pub const fn encoded(self) -> [u8; 2] {
        match self {
            Self::CalibrationIdentification => [0x09, 0x04],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalibrationId(String);

impl CalibrationId {
    pub fn new(value: impl Into<String>) -> Result<Self, J1979Error> {
        let value = value.into();
        if value.is_empty() || value.len() > CALIBRATION_FIELD_LEN {
            return Err(J1979Error::InvalidCalibrationLength(value.len()));
        }
        if !value.bytes().all(|byte| (0x20..=0x7e).contains(&byte)) {
            return Err(J1979Error::InvalidCalibrationEncoding);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalibrationIdentificationResult {
    pub calibration_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum J1979Error {
    TruncatedResponse,
    UnexpectedPositiveSid { expected: u8, actual: u8 },
    UnexpectedInfoType { expected: u8, actual: u8 },
    InvalidItemCount(u8),
    InvalidResponseLength { expected: usize, actual: usize },
    InvalidCalibrationLength(usize),
    InvalidCalibrationEncoding,
    InvalidCalibrationPadding,
    EmptyCalibration,
}

impl fmt::Display for J1979Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TruncatedResponse => formatter.write_str("truncated J1979 response"),
            Self::UnexpectedPositiveSid { expected, actual } => write!(
                formatter,
                "unexpected J1979 positive SID: expected {expected:#04x}, got {actual:#04x}"
            ),
            Self::UnexpectedInfoType { expected, actual } => write!(
                formatter,
                "unexpected J1979 InfoType: expected {expected:#04x}, got {actual:#04x}"
            ),
            Self::InvalidItemCount(count) => {
                write!(formatter, "invalid J1979 calibration item count: {count}")
            }
            Self::InvalidResponseLength { expected, actual } => write!(
                formatter,
                "invalid J1979 calibration response length: expected {expected}, got {actual}"
            ),
            Self::InvalidCalibrationLength(length) => {
                write!(formatter, "invalid calibration ID length: {length}")
            }
            Self::InvalidCalibrationEncoding => {
                formatter.write_str("calibration ID is not printable ASCII")
            }
            Self::InvalidCalibrationPadding => {
                formatter.write_str("calibration ID contains non-trailing NUL padding")
            }
            Self::EmptyCalibration => formatter.write_str("calibration ID is empty"),
        }
    }
}

impl std::error::Error for J1979Error {}

pub fn decode_calibration_identification(
    request: J1979Request,
    payload: &[u8],
) -> Result<CalibrationIdentificationResult, J1979Error> {
    if payload.len() < 3 {
        return Err(J1979Error::TruncatedResponse);
    }
    let (expected_sid, expected_info_type) = match request {
        J1979Request::CalibrationIdentification => (0x49, 0x04),
    };
    if payload[0] != expected_sid {
        return Err(J1979Error::UnexpectedPositiveSid {
            expected: expected_sid,
            actual: payload[0],
        });
    }
    if payload[1] != expected_info_type {
        return Err(J1979Error::UnexpectedInfoType {
            expected: expected_info_type,
            actual: payload[1],
        });
    }
    let count = payload[2];
    if count == 0 {
        return Err(J1979Error::InvalidItemCount(count));
    }
    let expected_len = 3 + usize::from(count) * CALIBRATION_FIELD_LEN;
    if payload.len() != expected_len {
        return Err(J1979Error::InvalidResponseLength {
            expected: expected_len,
            actual: payload.len(),
        });
    }

    let mut calibration_ids = Vec::with_capacity(usize::from(count));
    for field in payload[3..].chunks_exact(CALIBRATION_FIELD_LEN) {
        let content_len = field
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(field.len());
        if field[content_len..].iter().any(|byte| *byte != 0) {
            return Err(J1979Error::InvalidCalibrationPadding);
        }
        if content_len == 0 {
            return Err(J1979Error::EmptyCalibration);
        }
        let content = &field[..content_len];
        if !content.iter().all(|byte| (0x20..=0x7e).contains(byte)) {
            return Err(J1979Error::InvalidCalibrationEncoding);
        }
        calibration_ids.push(
            std::str::from_utf8(content)
                .map_err(|_| J1979Error::InvalidCalibrationEncoding)?
                .to_owned(),
        );
    }

    Ok(CalibrationIdentificationResult { calibration_ids })
}

/// Encode a typed synthetic response fixture using the standard one-byte item
/// count and fixed 16-byte, trailing-NUL-padded calibration fields.
pub fn encode_calibration_identification_response(
    calibration_ids: &[CalibrationId],
) -> Result<Vec<u8>, J1979Error> {
    let count = u8::try_from(calibration_ids.len())
        .ok()
        .filter(|count| *count > 0)
        .ok_or(J1979Error::InvalidItemCount(0))?;
    let mut payload = Vec::with_capacity(3 + calibration_ids.len() * CALIBRATION_FIELD_LEN);
    payload.extend([0x49, 0x04, count]);
    for calibration_id in calibration_ids {
        payload.extend(calibration_id.as_str().bytes());
        payload.resize(
            payload.len() + CALIBRATION_FIELD_LEN - calibration_id.as_str().len(),
            0,
        );
    }
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn positive() -> Vec<u8> {
        encode_calibration_identification_response(
            &[CalibrationId::new("CX23-14C204-ZAD").unwrap()],
        )
        .unwrap()
    }

    #[test]
    fn encodes_only_mode09_infotype04() {
        assert_eq!(
            J1979Request::calibration_identification().encoded(),
            [0x09, 0x04]
        );
    }

    #[test]
    fn decodes_counted_fixed_width_calibration_and_removes_nul_padding() {
        let payload = positive();
        assert_eq!(payload.len(), 19);
        assert_eq!(payload[0..3], [0x49, 0x04, 0x01]);
        assert_eq!(
            decode_calibration_identification(J1979Request::calibration_identification(), &payload)
                .unwrap()
                .calibration_ids,
            ["CX23-14C204-ZAD"]
        );
    }

    #[test]
    fn rejects_wrong_sid_and_info_type() {
        let mut wrong_sid = positive();
        wrong_sid[0] = 0x48;
        assert!(matches!(
            decode_calibration_identification(
                J1979Request::calibration_identification(),
                &wrong_sid
            ),
            Err(J1979Error::UnexpectedPositiveSid { .. })
        ));
        let mut wrong_info = positive();
        wrong_info[1] = 0x02;
        assert!(matches!(
            decode_calibration_identification(
                J1979Request::calibration_identification(),
                &wrong_info
            ),
            Err(J1979Error::UnexpectedInfoType { .. })
        ));
    }

    #[test]
    fn rejects_truncation_count_mismatch_and_zero_items() {
        assert_eq!(
            decode_calibration_identification(
                J1979Request::calibration_identification(),
                &[0x49, 0x04]
            ),
            Err(J1979Error::TruncatedResponse)
        );
        let mut count_mismatch = positive();
        count_mismatch[2] = 2;
        assert!(matches!(
            decode_calibration_identification(
                J1979Request::calibration_identification(),
                &count_mismatch
            ),
            Err(J1979Error::InvalidResponseLength { .. })
        ));
        assert_eq!(
            decode_calibration_identification(
                J1979Request::calibration_identification(),
                &[0x49, 0x04, 0]
            ),
            Err(J1979Error::InvalidItemCount(0))
        );
    }

    #[test]
    fn rejects_non_ascii_interior_padding_and_empty_field() {
        let mut non_ascii = positive();
        non_ascii[3] = 0xff;
        assert_eq!(
            decode_calibration_identification(
                J1979Request::calibration_identification(),
                &non_ascii
            ),
            Err(J1979Error::InvalidCalibrationEncoding)
        );
        let mut interior_padding = positive();
        interior_padding[4] = 0;
        assert_eq!(
            decode_calibration_identification(
                J1979Request::calibration_identification(),
                &interior_padding
            ),
            Err(J1979Error::InvalidCalibrationPadding)
        );
        let mut empty = positive();
        empty[3..].fill(0);
        assert_eq!(
            decode_calibration_identification(J1979Request::calibration_identification(), &empty),
            Err(J1979Error::EmptyCalibration)
        );
    }
}
