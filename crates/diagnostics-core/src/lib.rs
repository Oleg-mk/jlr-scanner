//! Shared diagnostic orchestration for live, replay, and simulator CAN sources.
//! This crate consumes a read-only source and exposes no CAN transmit API.

use std::fmt;
use transport_api::{CanFrameSource, CanId, CanSourceError, CanSourceKind};
use uds::{TypedDiagnosticResult, UdsError, UdsRequest};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticResult {
    pub source_kind: CanSourceKind,
    pub route: String,
    pub response_id: CanId,
    pub completed_timestamp_us: u64,
    pub value: TypedDiagnosticResult,
}

#[derive(Debug)]
pub enum DiagnosticError {
    Source(CanSourceError),
    IsoTp(isotp::IsoTpError),
    Uds(UdsError),
    Timeout,
    TruncatedIsoTp,
}

impl fmt::Display for DiagnosticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => write!(formatter, "{error}"),
            Self::IsoTp(error) => write!(formatter, "{error}"),
            Self::Uds(error) => write!(formatter, "{error}"),
            Self::Timeout => formatter.write_str("diagnostic response timed out"),
            Self::TruncatedIsoTp => formatter.write_str("CAN source ended during ISO-TP message"),
        }
    }
}
impl std::error::Error for DiagnosticError {}

impl From<CanSourceError> for DiagnosticError {
    fn from(value: CanSourceError) -> Self {
        Self::Source(value)
    }
}
impl From<isotp::IsoTpError> for DiagnosticError {
    fn from(value: isotp::IsoTpError) -> Self {
        Self::IsoTp(value)
    }
}
impl From<UdsError> for DiagnosticError {
    fn from(value: UdsError) -> Self {
        Self::Uds(value)
    }
}

/// Consume response frames through the same protocol path regardless of source.
///
/// The request is correlation context only. F4 deliberately does not transmit it.
pub fn run_transaction(
    source: &mut impl CanFrameSource,
    request: &UdsRequest,
    route: &str,
    response_id: CanId,
    timeout_us: u64,
) -> Result<DiagnosticResult, DiagnosticError> {
    let source_kind = source.source_kind();
    let mut reassembler = isotp::Reassembler::new();
    let mut first_timestamp = None;
    let mut last_timestamp = None;

    while let Some(frame) = source.next_frame()? {
        if frame.route != route || frame.id != response_id {
            continue;
        }
        let start = *first_timestamp.get_or_insert(frame.timestamp_us);
        if frame.timestamp_us.saturating_sub(start) > timeout_us {
            return Err(DiagnosticError::Timeout);
        }
        if last_timestamp.is_some() {
            reassembler.expire(frame.timestamp_us, timeout_us)?;
        }
        last_timestamp = Some(frame.timestamp_us);
        if let Some(payload) = reassembler.accept(&frame.data, frame.timestamp_us)? {
            return Ok(DiagnosticResult {
                source_kind,
                route: route.to_owned(),
                response_id,
                completed_timestamp_us: frame.timestamp_us,
                value: uds::decode(request, &payload)?,
            });
        }
    }

    if reassembler.is_pending() {
        Err(DiagnosticError::TruncatedIsoTp)
    } else {
        Err(DiagnosticError::Timeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagnostic_simulator::{SimulatorBehavior, SimulatorScenario, SimulatorSource};
    use transport_replay::{PlaybackMode, ReplaySource};

    const ROUTE: &str = "synthetic-can";

    #[test]
    fn golden_simulator_multiframe_to_typed_uds_result() {
        let request = UdsRequest::read_data_by_identifier(0x1234);
        let response_id = CanId::standard(0x708).unwrap();
        let mut simulator = SimulatorSource::script(
            SimulatorScenario {
                expected_request: request.clone(),
                behavior: SimulatorBehavior::Positive {
                    response_payload: vec![0x12, 0x34, 0xde, 0xad, 0xbe, 0xef, 1, 2, 3],
                },
            },
            &request,
            ROUTE,
            response_id,
        )
        .unwrap();
        let result = run_transaction(&mut simulator, &request, ROUTE, response_id, 10_000).unwrap();
        assert_eq!(result.source_kind, CanSourceKind::Simulator);
        assert_eq!(
            result.value,
            TypedDiagnosticResult::ReadDataByIdentifier {
                identifier: 0x1234,
                data: vec![0xde, 0xad, 0xbe, 0xef, 1, 2, 3],
            }
        );
        assert!(result.completed_timestamp_us > 0);
    }

    #[test]
    fn golden_replay_fixture_is_deterministic() {
        let fixture = include_str!("../../../fixtures/synthetic/f4_replay_uds_multiframe.json");
        let request = UdsRequest::read_data_by_identifier(0x1234);
        let response_id = CanId::standard(0x708).unwrap();
        let run = || {
            let mut replay = ReplaySource::from_json(fixture, PlaybackMode::Deterministic).unwrap();
            run_transaction(&mut replay, &request, ROUTE, response_id, 10_000).unwrap()
        };
        let first = run();
        let second = run();
        assert_eq!(first, second);
        assert_eq!(first.source_kind, CanSourceKind::Replay);
        assert_eq!(
            first.value,
            TypedDiagnosticResult::ReadDataByIdentifier {
                identifier: 0x1234,
                data: vec![0xde, 0xad, 0xbe, 0xef, 1, 2, 3],
            }
        );
    }

    #[test]
    fn simulator_timeout_and_malformed_sequence_are_observable() {
        let request = UdsRequest::read_data_by_identifier(0x1234);
        let response_id = CanId::standard(0x708).unwrap();
        let create = |behavior| {
            SimulatorSource::script(
                SimulatorScenario {
                    expected_request: request.clone(),
                    behavior,
                },
                &request,
                ROUTE,
                response_id,
            )
            .unwrap()
        };
        let mut timeout = create(SimulatorBehavior::Timeout);
        assert!(matches!(
            run_transaction(&mut timeout, &request, ROUTE, response_id, 10_000),
            Err(DiagnosticError::Timeout)
        ));
        let mut malformed = create(SimulatorBehavior::MalformedSequence {
            response_payload: vec![0x12, 0x34, 1, 2, 3, 4, 5, 6, 7],
        });
        assert!(matches!(
            run_transaction(&mut malformed, &request, ROUTE, response_id, 10_000),
            Err(DiagnosticError::IsoTp(
                isotp::IsoTpError::SequenceMismatch { .. }
            ))
        ));
    }
}
