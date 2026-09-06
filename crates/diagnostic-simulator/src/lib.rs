//! Deterministic, scriptable ECU-response simulator for offline protocol tests.

use std::collections::VecDeque;
use transport_api::{CanFrame, CanFrameSource, CanId, CanSourceError, CanSourceKind};
use uds::{NegativeResponseCode, UdsRequest};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SimulatorBehavior {
    Positive {
        response_payload: Vec<u8>,
    },
    Negative {
        code: NegativeResponseCode,
    },
    Timeout,
    MalformedSequence {
        response_payload: Vec<u8>,
    },
    Delayed {
        delay_us: u64,
        response_payload: Vec<u8>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimulatorScenario {
    pub expected_request: UdsRequest,
    pub behavior: SimulatorBehavior,
}

/// Protocol-neutral application payload scenario for offline-only simulation.
///
/// Raw bytes are confined to this fixture layer. Application callers still use
/// typed diagnostic intent and cannot transmit these bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PayloadSimulatorScenario {
    pub expected_request_payload: Vec<u8>,
    pub behavior: PayloadSimulatorBehavior,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PayloadSimulatorBehavior {
    Response {
        response_payload: Vec<u8>,
    },
    Timeout,
    MalformedSequence {
        response_payload: Vec<u8>,
    },
    Delayed {
        delay_us: u64,
        response_payload: Vec<u8>,
    },
}

pub struct SimulatorSource {
    frames: VecDeque<CanFrame>,
    expected_request_payload: Vec<u8>,
}

impl SimulatorSource {
    pub fn script(
        scenario: SimulatorScenario,
        actual_request: &UdsRequest,
        route: impl Into<String>,
        response_id: CanId,
    ) -> Result<Self, CanSourceError> {
        if scenario.expected_request != *actual_request {
            return Err(CanSourceError::InvalidData(
                "request does not match scripted simulator scenario".into(),
            ));
        }
        let expected_request_payload = scenario.expected_request.as_bytes().to_vec();
        let behavior = match scenario.behavior {
            SimulatorBehavior::Timeout => PayloadSimulatorBehavior::Timeout,
            SimulatorBehavior::Negative { code } => PayloadSimulatorBehavior::Response {
                response_payload: vec![0x7f, actual_request.service_id(), nrc_byte(code)],
            },
            SimulatorBehavior::Positive { response_payload } => {
                PayloadSimulatorBehavior::Response {
                    response_payload: positive_response(actual_request, response_payload),
                }
            }
            SimulatorBehavior::MalformedSequence { response_payload } => {
                PayloadSimulatorBehavior::MalformedSequence {
                    response_payload: positive_response(actual_request, response_payload),
                }
            }
            SimulatorBehavior::Delayed {
                delay_us,
                response_payload,
            } => PayloadSimulatorBehavior::Delayed {
                delay_us,
                response_payload: positive_response(actual_request, response_payload),
            },
        };
        Self::script_payload(
            PayloadSimulatorScenario {
                expected_request_payload,
                behavior,
            },
            actual_request.as_bytes(),
            route,
            response_id,
        )
    }

    pub fn script_payload(
        scenario: PayloadSimulatorScenario,
        actual_request_payload: &[u8],
        route: impl Into<String>,
        response_id: CanId,
    ) -> Result<Self, CanSourceError> {
        let PayloadSimulatorScenario {
            expected_request_payload,
            behavior,
        } = scenario;
        if expected_request_payload != actual_request_payload {
            return Err(CanSourceError::InvalidData(
                "request payload does not match scripted simulator scenario".into(),
            ));
        }
        let route = route.into();
        let (delay_us, response_payload, malformed) = match behavior {
            PayloadSimulatorBehavior::Timeout => {
                return Ok(Self {
                    frames: VecDeque::new(),
                    expected_request_payload,
                })
            }
            PayloadSimulatorBehavior::Response { response_payload } => (0, response_payload, false),
            PayloadSimulatorBehavior::MalformedSequence { response_payload } => {
                (0, response_payload, true)
            }
            PayloadSimulatorBehavior::Delayed {
                delay_us,
                response_payload,
            } => (delay_us, response_payload, false),
        };
        if response_payload.is_empty() {
            return Err(CanSourceError::InvalidData(
                "empty simulator response".into(),
            ));
        }
        let mut segmented = isotp::segment(&response_payload, None)
            .map_err(|error| CanSourceError::InvalidData(error.to_string()))?;
        if malformed && segmented.len() > 1 {
            segmented[1][0] = 0x22;
        }
        let mut frames = VecDeque::new();
        for (index, data) in segmented.into_iter().enumerate() {
            frames.push_back(
                CanFrame::new(
                    delay_us + index as u64 * 1_000,
                    route.clone(),
                    response_id,
                    data,
                )
                .map_err(|error| CanSourceError::InvalidData(error.to_string()))?,
            );
        }
        Ok(Self {
            frames,
            expected_request_payload,
        })
    }

    pub fn validate_expected_request_payload(
        &self,
        actual_request_payload: &[u8],
    ) -> Result<(), CanSourceError> {
        if self.expected_request_payload == actual_request_payload {
            Ok(())
        } else {
            Err(CanSourceError::InvalidData(
                "prepared request does not match simulator expectation".into(),
            ))
        }
    }
}

fn positive_response(request: &UdsRequest, response_payload: Vec<u8>) -> Vec<u8> {
    let mut response = vec![request.service_id().wrapping_add(0x40)];
    response.extend(response_payload);
    response
}

fn nrc_byte(code: NegativeResponseCode) -> u8 {
    match code {
        NegativeResponseCode::GeneralReject => 0x10,
        NegativeResponseCode::ServiceNotSupported => 0x11,
        NegativeResponseCode::SubFunctionNotSupported => 0x12,
        NegativeResponseCode::IncorrectMessageLengthOrInvalidFormat => 0x13,
        NegativeResponseCode::ConditionsNotCorrect => 0x22,
        NegativeResponseCode::RequestOutOfRange => 0x31,
        NegativeResponseCode::SecurityAccessDenied => 0x33,
        NegativeResponseCode::InvalidKey => 0x35,
        NegativeResponseCode::ExceededNumberOfAttempts => 0x36,
        NegativeResponseCode::RequiredTimeDelayNotExpired => 0x37,
        NegativeResponseCode::ResponsePending => 0x78,
        NegativeResponseCode::Other(value) => value,
    }
}

impl CanFrameSource for SimulatorSource {
    fn source_kind(&self) -> CanSourceKind {
        CanSourceKind::Simulator
    }
    fn next_frame(&mut self) -> Result<Option<CanFrame>, CanSourceError> {
        Ok(self.frames.pop_front())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_neutral_payload_seam_checks_request_and_uses_existing_isotp() {
        let response = vec![0x49, 0x04, 0x01, 0x43, 0x58, 0x32, 0x33, 0x2d, 0x31];
        let scenario = PayloadSimulatorScenario {
            expected_request_payload: vec![0x09, 0x04],
            behavior: PayloadSimulatorBehavior::Response {
                response_payload: response,
            },
        };
        assert!(SimulatorSource::script_payload(
            scenario.clone(),
            &[0x09, 0x02],
            "synthetic-can",
            CanId::standard(0x7e8).unwrap(),
        )
        .is_err());
        let mut source = SimulatorSource::script_payload(
            scenario,
            &[0x09, 0x04],
            "synthetic-can",
            CanId::standard(0x7e8).unwrap(),
        )
        .unwrap();
        source
            .validate_expected_request_payload(&[0x09, 0x04])
            .unwrap();
        assert!(source
            .validate_expected_request_payload(&[0x09, 0x02])
            .is_err());
        assert_eq!(source.next_frame().unwrap().unwrap().data[0] >> 4, 1);
        assert_eq!(source.next_frame().unwrap().unwrap().data[0] >> 4, 2);
    }

    #[test]
    fn covers_positive_negative_timeout_malformed_and_delayed_behaviors() {
        let request = UdsRequest::read_data_by_identifier(0x1234);
        let id = CanId::standard(0x708).unwrap();
        let make = |behavior| {
            SimulatorSource::script(
                SimulatorScenario {
                    expected_request: request.clone(),
                    behavior,
                },
                &request,
                "synthetic-can",
                id,
            )
            .unwrap()
        };

        assert!(make(SimulatorBehavior::Positive {
            response_payload: vec![0x12, 0x34, 1],
        })
        .next_frame()
        .unwrap()
        .is_some());
        assert!(make(SimulatorBehavior::Negative {
            code: NegativeResponseCode::RequestOutOfRange,
        })
        .next_frame()
        .unwrap()
        .is_some());
        assert!(make(SimulatorBehavior::Timeout)
            .next_frame()
            .unwrap()
            .is_none());
        let mut malformed = make(SimulatorBehavior::MalformedSequence {
            response_payload: vec![0x12, 0x34, 1, 2, 3, 4, 5, 6, 7],
        });
        malformed.next_frame().unwrap();
        assert_eq!(malformed.next_frame().unwrap().unwrap().data[0] & 0x0f, 2);
        let mut delayed = make(SimulatorBehavior::Delayed {
            delay_us: 5_000,
            response_payload: vec![0x12, 0x34, 1],
        });
        assert_eq!(delayed.next_frame().unwrap().unwrap().timestamp_us, 5_000);
    }
}
