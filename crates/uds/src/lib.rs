//! Generic UDS message correlation and a deliberately narrow safe request API.

use std::fmt;

mod dtc;
pub use dtc::{
    decode_dtc_by_status_mask, DtcRecord, DtcStatusReport, SUB_FUNCTION_REPORT_DTC_BY_STATUS_MASK,
};

pub const SID_DIAGNOSTIC_SESSION_CONTROL: u8 = 0x10;
pub const SID_READ_DTC_INFORMATION: u8 = 0x19;
pub const SID_READ_DATA_BY_IDENTIFIER: u8 = 0x22;
pub const SID_TESTER_PRESENT: u8 = 0x3e;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UdsRequest {
    bytes: Vec<u8>,
}

impl UdsRequest {
    pub fn diagnostic_session_control(session: u8) -> Self {
        Self {
            bytes: vec![SID_DIAGNOSTIC_SESSION_CONTROL, session],
        }
    }

    pub fn read_dtc_information(sub_function: u8, parameters: &[u8]) -> Self {
        let mut bytes = vec![SID_READ_DTC_INFORMATION, sub_function];
        bytes.extend_from_slice(parameters);
        Self { bytes }
    }

    pub fn read_data_by_identifier(identifier: u16) -> Self {
        Self {
            bytes: vec![
                SID_READ_DATA_BY_IDENTIFIER,
                (identifier >> 8) as u8,
                identifier as u8,
            ],
        }
    }

    pub fn tester_present(suppress_positive_response: bool) -> Self {
        Self {
            bytes: vec![
                SID_TESTER_PRESENT,
                if suppress_positive_response {
                    0x80
                } else {
                    0x00
                },
            ],
        }
    }

    pub fn service_id(&self) -> u8 {
        self.bytes[0]
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NegativeResponseCode {
    GeneralReject,
    ServiceNotSupported,
    SubFunctionNotSupported,
    IncorrectMessageLengthOrInvalidFormat,
    ConditionsNotCorrect,
    RequestOutOfRange,
    SecurityAccessDenied,
    InvalidKey,
    ExceededNumberOfAttempts,
    RequiredTimeDelayNotExpired,
    ResponsePending,
    Other(u8),
}

impl NegativeResponseCode {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0x10 => Self::GeneralReject,
            0x11 => Self::ServiceNotSupported,
            0x12 => Self::SubFunctionNotSupported,
            0x13 => Self::IncorrectMessageLengthOrInvalidFormat,
            0x22 => Self::ConditionsNotCorrect,
            0x31 => Self::RequestOutOfRange,
            0x33 => Self::SecurityAccessDenied,
            0x35 => Self::InvalidKey,
            0x36 => Self::ExceededNumberOfAttempts,
            0x37 => Self::RequiredTimeDelayNotExpired,
            0x78 => Self::ResponsePending,
            other => Self::Other(other),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PositiveResponse {
    pub service_id: u8,
    pub payload: Vec<u8>,
    pub raw: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NegativeResponse {
    pub request_service_id: u8,
    pub code: NegativeResponseCode,
    pub raw: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UdsResponse {
    Positive(PositiveResponse),
    Negative(NegativeResponse),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypedDiagnosticResult {
    DiagnosticSessionControl { session: u8, parameters: Vec<u8> },
    ReadDtcInformation { sub_function: u8, data: Vec<u8> },
    ReadDataByIdentifier { identifier: u16, data: Vec<u8> },
    TesterPresent { sub_function: u8, data: Vec<u8> },
    Negative(NegativeResponse),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UdsError {
    EmptyResponse,
    MalformedNegativeResponse,
    UnexpectedService { expected: u8, actual: u8 },
    MalformedPositiveResponse,
    CorrelationMismatch,
}

impl fmt::Display for UdsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyResponse => formatter.write_str("empty UDS response"),
            Self::MalformedNegativeResponse => {
                formatter.write_str("malformed UDS negative response")
            }
            Self::UnexpectedService { expected, actual } => write!(
                formatter,
                "unexpected UDS response service: expected {expected:#x}, got {actual:#x}"
            ),
            Self::MalformedPositiveResponse => {
                formatter.write_str("malformed UDS positive response")
            }
            Self::CorrelationMismatch => {
                formatter.write_str("UDS response does not correlate with request")
            }
        }
    }
}
impl std::error::Error for UdsError {}

pub fn parse_response(request: &UdsRequest, bytes: &[u8]) -> Result<UdsResponse, UdsError> {
    let response = parse_message(bytes)?;
    let expected = request.service_id().wrapping_add(0x40);
    match &response {
        UdsResponse::Negative(negative) if negative.request_service_id != request.service_id() => {
            Err(UdsError::CorrelationMismatch)
        }
        UdsResponse::Positive(positive) if positive.service_id != expected => {
            Err(UdsError::UnexpectedService {
                expected,
                actual: positive.service_id,
            })
        }
        _ => Ok(response),
    }
}

/// Parse any UDS response without creating an outbound request surface.
pub fn parse_message(bytes: &[u8]) -> Result<UdsResponse, UdsError> {
    let response_sid = *bytes.first().ok_or(UdsError::EmptyResponse)?;
    if response_sid == 0x7f {
        if bytes.len() < 3 {
            return Err(UdsError::MalformedNegativeResponse);
        }
        return Ok(UdsResponse::Negative(NegativeResponse {
            request_service_id: bytes[1],
            code: NegativeResponseCode::from_byte(bytes[2]),
            raw: bytes.to_vec(),
        }));
    }
    Ok(UdsResponse::Positive(PositiveResponse {
        service_id: response_sid,
        payload: bytes[1..].to_vec(),
        raw: bytes.to_vec(),
    }))
}

pub fn typed_result(
    request: &UdsRequest,
    response: UdsResponse,
) -> Result<TypedDiagnosticResult, UdsError> {
    let UdsResponse::Positive(positive) = response else {
        let UdsResponse::Negative(negative) = response else {
            unreachable!()
        };
        return Ok(TypedDiagnosticResult::Negative(negative));
    };
    match request.service_id() {
        SID_DIAGNOSTIC_SESSION_CONTROL => {
            let (&session, parameters) = positive
                .payload
                .split_first()
                .ok_or(UdsError::MalformedPositiveResponse)?;
            if session != request.as_bytes()[1] {
                return Err(UdsError::CorrelationMismatch);
            }
            Ok(TypedDiagnosticResult::DiagnosticSessionControl {
                session,
                parameters: parameters.to_vec(),
            })
        }
        SID_READ_DTC_INFORMATION => {
            let (&sub_function, data) = positive
                .payload
                .split_first()
                .ok_or(UdsError::MalformedPositiveResponse)?;
            if sub_function != request.as_bytes()[1] {
                return Err(UdsError::CorrelationMismatch);
            }
            Ok(TypedDiagnosticResult::ReadDtcInformation {
                sub_function,
                data: data.to_vec(),
            })
        }
        SID_READ_DATA_BY_IDENTIFIER => {
            if positive.payload.len() < 2 {
                return Err(UdsError::MalformedPositiveResponse);
            }
            let identifier = u16::from_be_bytes([positive.payload[0], positive.payload[1]]);
            let requested = u16::from_be_bytes([request.as_bytes()[1], request.as_bytes()[2]]);
            if identifier != requested {
                return Err(UdsError::CorrelationMismatch);
            }
            Ok(TypedDiagnosticResult::ReadDataByIdentifier {
                identifier,
                data: positive.payload[2..].to_vec(),
            })
        }
        SID_TESTER_PRESENT => {
            let (&sub_function, data) = positive
                .payload
                .split_first()
                .ok_or(UdsError::MalformedPositiveResponse)?;
            if sub_function & 0x7f != request.as_bytes()[1] & 0x7f {
                return Err(UdsError::CorrelationMismatch);
            }
            Ok(TypedDiagnosticResult::TesterPresent {
                sub_function,
                data: data.to_vec(),
            })
        }
        _ => Err(UdsError::CorrelationMismatch),
    }
}

pub fn decode(request: &UdsRequest, bytes: &[u8]) -> Result<TypedDiagnosticResult, UdsError> {
    typed_result(request, parse_response(request, bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_only_declared_safe_requests() {
        assert_eq!(
            UdsRequest::diagnostic_session_control(1).as_bytes(),
            &[0x10, 1]
        );
        assert_eq!(
            UdsRequest::read_dtc_information(2, &[0xff]).as_bytes(),
            &[0x19, 2, 0xff]
        );
        assert_eq!(
            UdsRequest::read_data_by_identifier(0x1234).as_bytes(),
            &[0x22, 0x12, 0x34]
        );
        assert_eq!(UdsRequest::tester_present(false).as_bytes(), &[0x3e, 0]);
    }

    #[test]
    fn decodes_positive_did_with_arbitrary_payload() {
        assert_eq!(
            decode(
                &UdsRequest::read_data_by_identifier(0x1234),
                &[0x62, 0x12, 0x34, 0xde, 0xad]
            )
            .unwrap(),
            TypedDiagnosticResult::ReadDataByIdentifier {
                identifier: 0x1234,
                data: vec![0xde, 0xad],
            }
        );
    }

    #[test]
    fn decodes_negative_response_and_nrc() {
        let result = decode(
            &UdsRequest::read_data_by_identifier(0x1234),
            &[0x7f, 0x22, 0x31],
        )
        .unwrap();
        assert!(matches!(
            result,
            TypedDiagnosticResult::Negative(NegativeResponse {
                code: NegativeResponseCode::RequestOutOfRange,
                ..
            })
        ));
    }

    #[test]
    fn rejects_uncorrelated_service_and_identifier() {
        let request = UdsRequest::read_data_by_identifier(0x1234);
        assert!(matches!(
            decode(&request, &[0x50, 1]),
            Err(UdsError::UnexpectedService { .. })
        ));
        assert_eq!(
            decode(&request, &[0x62, 0x99, 0x99]),
            Err(UdsError::CorrelationMismatch)
        );
        assert_eq!(
            decode(&request, &[0x7f, 0x19, 0x31]),
            Err(UdsError::CorrelationMismatch)
        );
    }

    #[test]
    fn generic_parser_preserves_unknown_positive_payload() {
        assert_eq!(
            parse_message(&[0x6a, 1, 2, 3]).unwrap(),
            UdsResponse::Positive(PositiveResponse {
                service_id: 0x6a,
                payload: vec![1, 2, 3],
                raw: vec![0x6a, 1, 2, 3],
            })
        );
    }
}
