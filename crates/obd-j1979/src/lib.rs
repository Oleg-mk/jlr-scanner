//! Vehicle-independent SAE J1979 / ISO 15031-5 codec: the legislated OBD-II
//! services, read-only, exactly as the standard defines them.
//!
//! What is here is what every emissions-relevant module on every OBD-II car
//! answers the same way, JLR or not: current data (mode 01), freeze frame
//! (02), stored, pending and permanent fault codes (03, 07, 0A), on-board
//! monitoring results (06) and vehicle information (09). What is not here,
//! and never will be, is the two services of the standard that change the
//! vehicle: clearing codes (04) and controlling on-board systems (08). The
//! request type is a closed enum — no caller supplies a mode or a byte — and
//! every decoder refuses what it cannot read rather than guessing at it.
//!
//! This crate models no UDS, no transport, no vehicle knowledge and no
//! arbitrary payloads.

pub mod bench;
pub mod current_data;
pub mod dtc;
pub mod monitor;
pub mod support;
pub mod vehicle_info;

use std::fmt;

pub use current_data::{DecodedParameter, ParameterValue};
pub use dtc::DtcKind;
pub use monitor::MonitorResult;
pub use vehicle_info::VehicleInformation;

pub const CALIBRATION_FIELD_LEN: usize = 16;
/// ISO 15765-4 lets one mode 01 or 06 request carry up to six items.
pub const MAX_ITEMS_PER_REQUEST: usize = 6;

/// A read-only J1979 request. Constructed only through the functions below,
/// which check what the standard checks; the interface never sees a mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum J1979Request {
    /// Mode 09 InfoType 04 — kept as its own variant because F8's live path
    /// was built on it and validates it by name.
    CalibrationIdentification,
    /// Mode 01: current data for one to six PIDs.
    CurrentData {
        pids: [u8; MAX_ITEMS_PER_REQUEST],
        count: u8,
    },
    /// Mode 02: one PID of one freeze frame.
    FreezeFrame { pid: u8, frame: u8 },
    /// Mode 03: confirmed (stored) emission-related fault codes.
    StoredDtcs,
    /// Mode 07: pending fault codes, from the current or last driving cycle.
    PendingDtcs,
    /// Mode 0A: permanent fault codes, which only the vehicle itself clears.
    PermanentDtcs,
    /// Mode 06: on-board monitoring test results for one to six monitors.
    MonitorResults {
        mids: [u8; MAX_ITEMS_PER_REQUEST],
        count: u8,
    },
    /// Mode 09: one vehicle-information InfoType from the read-only set.
    VehicleInformation { info_type: u8 },
}

impl J1979Request {
    pub const fn calibration_identification() -> Self {
        Self::CalibrationIdentification
    }

    pub fn current_data(pids: &[u8]) -> Result<Self, J1979Error> {
        let (items, count) = pack_items(pids)?;
        Ok(Self::CurrentData { pids: items, count })
    }

    pub const fn freeze_frame(pid: u8, frame: u8) -> Self {
        Self::FreezeFrame { pid, frame }
    }

    pub const fn stored_dtcs() -> Self {
        Self::StoredDtcs
    }

    pub const fn pending_dtcs() -> Self {
        Self::PendingDtcs
    }

    pub const fn permanent_dtcs() -> Self {
        Self::PermanentDtcs
    }

    pub fn monitor_results(mids: &[u8]) -> Result<Self, J1979Error> {
        let (items, count) = pack_items(mids)?;
        Ok(Self::MonitorResults { mids: items, count })
    }

    /// The InfoTypes this crate reads: the support bitmaps, VIN, calibration
    /// identifications, calibration verification numbers, in-use performance
    /// tracking, ECU name and ECU serial number. Anything else is refused.
    pub fn vehicle_information(info_type: u8) -> Result<Self, J1979Error> {
        if vehicle_info::is_read_only_info_type(info_type) {
            Ok(Self::VehicleInformation { info_type })
        } else {
            Err(J1979Error::UnsupportedInfoType(info_type))
        }
    }

    /// The service (mode) byte the request carries.
    pub const fn mode(self) -> u8 {
        match self {
            Self::CalibrationIdentification | Self::VehicleInformation { .. } => 0x09,
            Self::CurrentData { .. } => 0x01,
            Self::FreezeFrame { .. } => 0x02,
            Self::StoredDtcs => 0x03,
            Self::PendingDtcs => 0x07,
            Self::PermanentDtcs => 0x0A,
            Self::MonitorResults { .. } => 0x06,
        }
    }

    /// The positive response begins with the mode plus 0x40.
    pub const fn positive_sid(self) -> u8 {
        self.mode() + 0x40
    }

    /// The bytes on the wire. Never more than seven, so always one ISO-TP
    /// single frame.
    pub fn encoded(self) -> Vec<u8> {
        match self {
            Self::CalibrationIdentification => vec![0x09, 0x04],
            Self::CurrentData { pids, count } => {
                let mut bytes = vec![0x01];
                bytes.extend_from_slice(&pids[..usize::from(count)]);
                bytes
            }
            Self::FreezeFrame { pid, frame } => vec![0x02, pid, frame],
            Self::StoredDtcs => vec![0x03],
            Self::PendingDtcs => vec![0x07],
            Self::PermanentDtcs => vec![0x0A],
            Self::MonitorResults { mids, count } => {
                let mut bytes = vec![0x06];
                bytes.extend_from_slice(&mids[..usize::from(count)]);
                bytes
            }
            Self::VehicleInformation { info_type } => vec![0x09, info_type],
        }
    }
}

fn pack_items(items: &[u8]) -> Result<([u8; MAX_ITEMS_PER_REQUEST], u8), J1979Error> {
    if items.is_empty() {
        return Err(J1979Error::NoItems);
    }
    if items.len() > MAX_ITEMS_PER_REQUEST {
        return Err(J1979Error::TooManyItems(items.len()));
    }
    let mut packed = [0u8; MAX_ITEMS_PER_REQUEST];
    packed[..items.len()].copy_from_slice(items);
    Ok((packed, items.len() as u8))
}

/// What a module answered, decoded as far as the standard lets it be.
#[derive(Clone, Debug, PartialEq)]
pub enum J1979Response {
    CalibrationIdentification(CalibrationIdentificationResult),
    CurrentData(Vec<DecodedParameter>),
    FreezeFrame {
        frame: u8,
        parameters: Vec<DecodedParameter>,
    },
    Dtcs {
        kind: DtcKind,
        codes: Vec<String>,
    },
    MonitorResults(Vec<MonitorResult>),
    VehicleInformation(VehicleInformation),
}

/// Decode a positive or negative response to `request`. A negative response
/// (`7F mode code`) is an answer and comes back as its own error variant, so
/// the caller can show the module's refusal rather than a failure of ours.
pub fn decode_response(request: J1979Request, payload: &[u8]) -> Result<J1979Response, J1979Error> {
    if payload.is_empty() {
        return Err(J1979Error::TruncatedResponse);
    }
    if payload[0] == 0x7F {
        return Err(J1979Error::NegativeResponse {
            service: payload.get(1).copied().unwrap_or(0),
            code: payload.get(2).copied().unwrap_or(0),
        });
    }
    let expected = request.positive_sid();
    if payload[0] != expected {
        return Err(J1979Error::UnexpectedPositiveSid {
            expected,
            actual: payload[0],
        });
    }
    match request {
        J1979Request::CalibrationIdentification => {
            decode_calibration_identification(request, payload)
                .map(J1979Response::CalibrationIdentification)
        }
        J1979Request::CurrentData { pids, count } => {
            current_data::decode_current_data(&pids[..usize::from(count)], &payload[1..])
                .map(J1979Response::CurrentData)
        }
        J1979Request::FreezeFrame { pid, frame } => {
            current_data::decode_freeze_frame(pid, frame, &payload[1..])
                .map(|parameters| J1979Response::FreezeFrame { frame, parameters })
        }
        J1979Request::StoredDtcs => {
            dtc::decode_dtc_list(&payload[1..]).map(|codes| J1979Response::Dtcs {
                kind: DtcKind::Stored,
                codes,
            })
        }
        J1979Request::PendingDtcs => {
            dtc::decode_dtc_list(&payload[1..]).map(|codes| J1979Response::Dtcs {
                kind: DtcKind::Pending,
                codes,
            })
        }
        J1979Request::PermanentDtcs => {
            dtc::decode_dtc_list(&payload[1..]).map(|codes| J1979Response::Dtcs {
                kind: DtcKind::Permanent,
                codes,
            })
        }
        J1979Request::MonitorResults { .. } => {
            monitor::decode_monitor_results(&payload[1..]).map(J1979Response::MonitorResults)
        }
        J1979Request::VehicleInformation { info_type } => {
            vehicle_info::decode_vehicle_information(info_type, &payload[1..])
                .map(J1979Response::VehicleInformation)
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
    UnexpectedPositiveSid {
        expected: u8,
        actual: u8,
    },
    UnexpectedInfoType {
        expected: u8,
        actual: u8,
    },
    InvalidItemCount(u8),
    InvalidResponseLength {
        expected: usize,
        actual: usize,
    },
    InvalidCalibrationLength(usize),
    InvalidCalibrationEncoding,
    InvalidCalibrationPadding,
    EmptyCalibration,
    /// A request needs at least one PID or MID.
    NoItems,
    /// More items than one ISO 15765-4 request carries.
    TooManyItems(usize),
    /// An InfoType outside the read-only set this crate serves.
    UnsupportedInfoType(u8),
    /// `7F mode code`: the module refused, and said why.
    NegativeResponse {
        service: u8,
        code: u8,
    },
    /// A PID whose length the standard does not fix appeared before the end
    /// of a multi-PID answer, so the bytes after it cannot be attributed.
    UnknownPidLength(u8),
    /// The bytes for a PID stopped short of what its definition needs.
    TruncatedParameter {
        pid: u8,
        expected: usize,
        actual: usize,
    },
    /// A module answered a PID that was not asked for.
    UnexpectedPid(u8),
    /// The answer holds a frame number other than the one asked for.
    UnexpectedFrame {
        expected: u8,
        actual: u8,
    },
    /// A monitoring result that ends before its nine bytes do.
    TruncatedMonitorResult,
    /// Vehicle information whose item count does not fit its bytes.
    InvalidInformationLength {
        info_type: u8,
        actual: usize,
    },
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
            Self::NoItems => formatter.write_str("a J1979 request needs at least one item"),
            Self::TooManyItems(count) => write!(
                formatter,
                "a J1979 request carries at most {MAX_ITEMS_PER_REQUEST} items, {count} given"
            ),
            Self::UnsupportedInfoType(info_type) => write!(
                formatter,
                "J1979 InfoType {info_type:#04x} is outside the read-only set"
            ),
            Self::NegativeResponse { service, code } => write!(
                formatter,
                "the module refused service {service:#04x}: negative response {code:#04x} ({})",
                negative_response_text(*code)
            ),
            Self::UnknownPidLength(pid) => write!(
                formatter,
                "PID {pid:#04x} has no fixed length and is not last in the answer"
            ),
            Self::TruncatedParameter {
                pid,
                expected,
                actual,
            } => write!(
                formatter,
                "PID {pid:#04x} needs {expected} byte(s), {actual} given"
            ),
            Self::UnexpectedPid(pid) => {
                write!(
                    formatter,
                    "the answer carries PID {pid:#04x}, which was not asked for"
                )
            }
            Self::UnexpectedFrame { expected, actual } => write!(
                formatter,
                "freeze frame {actual} answered where frame {expected} was asked for"
            ),
            Self::TruncatedMonitorResult => {
                formatter.write_str("a monitoring result ends before its nine bytes do")
            }
            Self::InvalidInformationLength { info_type, actual } => write!(
                formatter,
                "InfoType {info_type:#04x} answer of {actual} byte(s) does not fit its item count"
            ),
        }
    }
}

impl std::error::Error for J1979Error {}

/// ISO 14229 / ISO 15031-5 negative response codes a legislated module may
/// give a read-only request.
pub fn negative_response_text(code: u8) -> &'static str {
    match code {
        0x10 => "general reject",
        0x11 => "service not supported",
        0x12 => "sub-function not supported",
        0x13 => "incorrect message length or invalid format",
        0x21 => "busy, repeat request",
        0x22 => "conditions not correct",
        0x31 => "request out of range",
        0x78 => "response pending",
        _ => "unlisted code",
    }
}

pub fn decode_calibration_identification(
    request: J1979Request,
    payload: &[u8],
) -> Result<CalibrationIdentificationResult, J1979Error> {
    if payload.len() < 3 {
        return Err(J1979Error::TruncatedResponse);
    }
    let (expected_sid, expected_info_type) = match request {
        J1979Request::CalibrationIdentification => (0x49, 0x04),
        other => (other.positive_sid(), 0x04),
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
    fn every_request_is_one_single_frame_and_names_its_mode() {
        let requests = [
            J1979Request::current_data(&[0x0C, 0x0D, 0x05, 0x04, 0x11, 0x0B]).unwrap(),
            J1979Request::freeze_frame(0x0C, 0),
            J1979Request::stored_dtcs(),
            J1979Request::pending_dtcs(),
            J1979Request::permanent_dtcs(),
            J1979Request::monitor_results(&[0x01, 0x21]).unwrap(),
            J1979Request::vehicle_information(0x02).unwrap(),
        ];
        let modes = [0x01, 0x02, 0x03, 0x07, 0x0A, 0x06, 0x09];
        for (request, mode) in requests.iter().zip(modes) {
            let bytes = request.encoded();
            assert!(bytes.len() <= 7, "{request:?} does not fit one frame");
            assert_eq!(bytes[0], mode);
            assert_eq!(request.positive_sid(), mode + 0x40);
        }
        assert_eq!(
            J1979Request::current_data(&[0x0C, 0x0D]).unwrap().encoded(),
            [0x01, 0x0C, 0x0D]
        );
    }

    #[test]
    fn a_request_refuses_no_items_too_many_items_and_an_active_info_type() {
        assert_eq!(J1979Request::current_data(&[]), Err(J1979Error::NoItems));
        assert_eq!(
            J1979Request::monitor_results(&[1, 2, 3, 4, 5, 6, 7]),
            Err(J1979Error::TooManyItems(7))
        );
        // InfoType 0x01 is the message count of the K-line form, not data.
        assert_eq!(
            J1979Request::vehicle_information(0x01),
            Err(J1979Error::UnsupportedInfoType(0x01))
        );
    }

    #[test]
    fn a_negative_response_is_an_answer_with_its_reason() {
        let error = decode_response(J1979Request::stored_dtcs(), &[0x7F, 0x03, 0x12]).unwrap_err();
        assert_eq!(
            error,
            J1979Error::NegativeResponse {
                service: 0x03,
                code: 0x12
            }
        );
        assert!(error.to_string().contains("sub-function not supported"));
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
        assert_eq!(
            decode_response(J1979Request::calibration_identification(), &payload).unwrap(),
            J1979Response::CalibrationIdentification(CalibrationIdentificationResult {
                calibration_ids: vec!["CX23-14C204-ZAD".into()]
            })
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
