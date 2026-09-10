//! Offline-only compilation and execution of evidence-backed read-only intent.
//!
//! This is the one-way application boundary from an F6 resolved plan into
//! protocol-specific offline replay/simulator execution. It exposes neither a
//! live source nor an arbitrary payload or CAN transmission API.

use diagnostic_environment::{
    CanIdFormat, DiagnosticEnvironmentResolution, ImplementationMarkerKind, PlanEvidenceTrace,
    ResolutionConflict, UnresolvedFact,
};
use diagnostic_simulator::SimulatorSource;
use obd_j1979::{
    decode_calibration_identification, CalibrationIdentificationResult, J1979Error, J1979Request,
};
use std::collections::BTreeMap;
use std::fmt;
use transport_api::{CanFrameSource, CanId, CanSourceError, CanSourceKind};
use transport_replay::{FixtureClass, ReplaySource};

pub const CALIBRATION_IDENTIFICATION_CAPABILITY: &str =
    "obd.service09.infotype04.calibration_id.read_only";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticTargetIdentity {
    ecu_family: String,
    diagnostic_implementation: String,
}

impl DiagnosticTargetIdentity {
    pub fn new(
        ecu_family: impl Into<String>,
        diagnostic_implementation: impl Into<String>,
    ) -> Result<Self, PreparationError> {
        let target = Self {
            ecu_family: ecu_family.into(),
            diagnostic_implementation: diagnostic_implementation.into(),
        };
        if target.ecu_family.trim().is_empty() || target.diagnostic_implementation.trim().is_empty()
        {
            return Err(PreparationError::EmptyTargetIdentity);
        }
        Ok(target)
    }

    pub fn ecu_family(&self) -> &str {
        &self.ecu_family
    }

    pub fn diagnostic_implementation(&self) -> &str {
        &self.diagnostic_implementation
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadOnlyDiagnosticIntent {
    CalibrationIdentification { target: DiagnosticTargetIdentity },
}

impl ReadOnlyDiagnosticIntent {
    pub fn calibration_identification(target: DiagnosticTargetIdentity) -> Self {
        Self::CalibrationIdentification { target }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransactionSafetyClass {
    ReadOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProvenanceField {
    VehicleApplicability,
    EcuFamily,
    DiagnosticImplementation,
    LogicalNetwork,
    PhysicalRoute,
    BackendRoute,
    Bitrate,
    ProtocolFamily,
    AddressingMode,
    CanIdFormat,
    PhysicalRequest,
    ExpectedResponse,
    FunctionalRequest,
    Capability,
    ObservedCalibration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionProvenance {
    traces: BTreeMap<ProvenanceField, Vec<PlanEvidenceTrace>>,
}

impl ExecutionProvenance {
    pub fn traces_for(&self, field: ProvenanceField) -> &[PlanEvidenceTrace] {
        self.traces.get(&field).map(Vec::as_slice).unwrap_or(&[])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedDiagnosticTransaction {
    safety_class: TransactionSafetyClass,
    target: DiagnosticTargetIdentity,
    logical_network: String,
    physical_connector: String,
    physical_pins: Vec<u8>,
    backend_route: String,
    bitrate_bps: u32,
    protocol_family: String,
    addressing_mode: String,
    physical_request_id: CanId,
    expected_response_id: CanId,
    functional_request_id: Option<CanId>,
    capability_id: String,
    observed_calibration: String,
    request: J1979Request,
    encoded_payload: Vec<u8>,
    provenance: ExecutionProvenance,
}

impl PreparedDiagnosticTransaction {
    pub fn safety_class(&self) -> TransactionSafetyClass {
        self.safety_class
    }

    pub fn target(&self) -> &DiagnosticTargetIdentity {
        &self.target
    }

    pub fn logical_network(&self) -> &str {
        &self.logical_network
    }

    pub fn physical_connector(&self) -> &str {
        &self.physical_connector
    }

    pub fn physical_pins(&self) -> &[u8] {
        &self.physical_pins
    }

    pub fn backend_route(&self) -> &str {
        &self.backend_route
    }

    pub fn bitrate_bps(&self) -> u32 {
        self.bitrate_bps
    }

    pub fn protocol_family(&self) -> &str {
        &self.protocol_family
    }

    pub fn addressing_mode(&self) -> &str {
        &self.addressing_mode
    }

    pub fn physical_request_id(&self) -> CanId {
        self.physical_request_id
    }

    pub fn expected_response_id(&self) -> CanId {
        self.expected_response_id
    }

    pub fn functional_request_id(&self) -> Option<CanId> {
        self.functional_request_id
    }

    pub fn capability_id(&self) -> &str {
        &self.capability_id
    }

    pub fn observed_calibration(&self) -> &str {
        &self.observed_calibration
    }

    pub fn protocol_request(&self) -> J1979Request {
        self.request
    }

    pub fn provenance(&self) -> &ExecutionProvenance {
        &self.provenance
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreparationError {
    EnvironmentIndeterminate(Vec<UnresolvedFact>),
    EnvironmentConflict(Vec<ResolutionConflict>),
    EmptyTargetIdentity,
    TargetEcuMismatch { expected: String, actual: String },
    TargetImplementationMismatch { expected: String, actual: String },
    UnsupportedProtocol(String),
    UnsupportedCapability(String),
    InvalidRoute(&'static str),
    InvalidBitrate(u32),
    UnsupportedAddressingMode(String),
    InvalidCanId { field: &'static str, value: u32 },
    MissingObservedCalibration,
    MissingDiagnosticImplementation,
}

impl fmt::Display for PreparationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EnvironmentIndeterminate(_) => {
                formatter.write_str("diagnostic environment is INDETERMINATE")
            }
            Self::EnvironmentConflict(_) => {
                formatter.write_str("diagnostic environment is CONFLICT")
            }
            Self::EmptyTargetIdentity => formatter.write_str("diagnostic target is empty"),
            Self::TargetEcuMismatch { expected, actual } => write!(
                formatter,
                "target ECU mismatch: expected {expected}, plan has {actual}"
            ),
            Self::TargetImplementationMismatch { expected, actual } => write!(
                formatter,
                "target implementation mismatch: expected {expected}, plan has {actual}"
            ),
            Self::UnsupportedProtocol(protocol) => {
                write!(formatter, "unsupported diagnostic protocol: {protocol}")
            }
            Self::UnsupportedCapability(capability) => {
                write!(formatter, "unsupported read-only capability: {capability}")
            }
            Self::InvalidRoute(field) => {
                write!(formatter, "invalid mandatory route field: {field}")
            }
            Self::InvalidBitrate(value) => write!(formatter, "invalid CAN bitrate: {value}"),
            Self::UnsupportedAddressingMode(mode) => {
                write!(formatter, "unsupported addressing mode: {mode}")
            }
            Self::InvalidCanId { field, value } => {
                write!(
                    formatter,
                    "invalid {field} for declared CAN ID width: {value:#x}"
                )
            }
            Self::MissingObservedCalibration => {
                formatter.write_str("resolved plan lacks an observed calibration marker")
            }
            Self::MissingDiagnosticImplementation => formatter
                .write_str("resolved plan is family-level and names no diagnostic implementation"),
        }
    }
}

impl std::error::Error for PreparationError {}

pub fn prepare_read_only_transaction(
    resolution: &DiagnosticEnvironmentResolution,
    intent: ReadOnlyDiagnosticIntent,
) -> Result<PreparedDiagnosticTransaction, PreparationError> {
    let plan = match resolution {
        DiagnosticEnvironmentResolution::Resolved(plan) => plan,
        DiagnosticEnvironmentResolution::Indeterminate {
            unresolved_facts, ..
        } => {
            return Err(PreparationError::EnvironmentIndeterminate(
                unresolved_facts.clone(),
            ))
        }
        DiagnosticEnvironmentResolution::Conflict { conflicts, .. } => {
            return Err(PreparationError::EnvironmentConflict(conflicts.clone()))
        }
    };

    let target = match intent {
        ReadOnlyDiagnosticIntent::CalibrationIdentification { target } => target,
    };
    if target.ecu_family != plan.ecu_family.value {
        return Err(PreparationError::TargetEcuMismatch {
            expected: target.ecu_family,
            actual: plan.ecu_family.value.clone(),
        });
    }
    // Calibration identification is evidence about one software build, so a
    // family-level plan (ADR-0013) is not enough here.
    let implementation = plan
        .diagnostic_implementation
        .as_ref()
        .ok_or(PreparationError::MissingDiagnosticImplementation)?;
    if target.diagnostic_implementation != implementation.value {
        return Err(PreparationError::TargetImplementationMismatch {
            expected: target.diagnostic_implementation,
            actual: implementation.value.clone(),
        });
    }
    if plan.protocol_family.value != "ISO15765-4 / SAE J1979" {
        return Err(PreparationError::UnsupportedProtocol(
            plan.protocol_family.value.clone(),
        ));
    }
    if plan.read_only_capability.value.id != CALIBRATION_IDENTIFICATION_CAPABILITY {
        return Err(PreparationError::UnsupportedCapability(
            plan.read_only_capability.value.id.clone(),
        ));
    }
    if plan.logical_network.value.trim().is_empty() {
        return Err(PreparationError::InvalidRoute("logical_network"));
    }
    if plan.physical_route.value.connector.trim().is_empty() {
        return Err(PreparationError::InvalidRoute("physical_connector"));
    }
    if plan.physical_route.value.pins.is_empty() {
        return Err(PreparationError::InvalidRoute("physical_pins"));
    }
    if plan.backend_route.value.route_id.trim().is_empty() {
        return Err(PreparationError::InvalidRoute("backend_route"));
    }
    if plan.bitrate_bps.value == 0 {
        return Err(PreparationError::InvalidBitrate(0));
    }
    if plan.addressing_mode.value != "normal_physical" {
        return Err(PreparationError::UnsupportedAddressingMode(
            plan.addressing_mode.value.clone(),
        ));
    }

    let can_id = |field, value| match plan.can_id_format.value {
        CanIdFormat::Standard11Bit => u16::try_from(value)
            .ok()
            .and_then(|value| CanId::standard(value).ok())
            .ok_or(PreparationError::InvalidCanId { field, value }),
        CanIdFormat::Extended29Bit => {
            CanId::extended(value).map_err(|_| PreparationError::InvalidCanId { field, value })
        }
    };
    let physical_request_id = can_id("physical_request_id", plan.physical_request_id.value)?;
    let expected_response_id = can_id("physical_response_id", plan.physical_response_id.value)?;
    let functional_request_id = plan
        .functional_request_id
        .as_ref()
        .map(|field| can_id("functional_request_id", field.value))
        .transpose()?;

    let observed_calibration = plan
        .implementation_markers
        .get(&ImplementationMarkerKind::CalibrationId)
        .ok_or(PreparationError::MissingObservedCalibration)?;

    let mut traces = BTreeMap::new();
    traces.insert(
        ProvenanceField::VehicleApplicability,
        plan.vehicle_applicability.evidence.clone(),
    );
    traces.insert(ProvenanceField::EcuFamily, plan.ecu_family.evidence.clone());
    traces.insert(
        ProvenanceField::DiagnosticImplementation,
        implementation.evidence.clone(),
    );
    traces.insert(
        ProvenanceField::LogicalNetwork,
        plan.logical_network.evidence.clone(),
    );
    traces.insert(
        ProvenanceField::PhysicalRoute,
        plan.physical_route.evidence.clone(),
    );
    traces.insert(
        ProvenanceField::BackendRoute,
        plan.backend_route.evidence.clone(),
    );
    traces.insert(ProvenanceField::Bitrate, plan.bitrate_bps.evidence.clone());
    traces.insert(
        ProvenanceField::ProtocolFamily,
        plan.protocol_family.evidence.clone(),
    );
    traces.insert(
        ProvenanceField::AddressingMode,
        plan.addressing_mode.evidence.clone(),
    );
    traces.insert(
        ProvenanceField::CanIdFormat,
        plan.can_id_format.evidence.clone(),
    );
    traces.insert(
        ProvenanceField::PhysicalRequest,
        plan.physical_request_id.evidence.clone(),
    );
    traces.insert(
        ProvenanceField::ExpectedResponse,
        plan.physical_response_id.evidence.clone(),
    );
    if let Some(functional) = &plan.functional_request_id {
        traces.insert(
            ProvenanceField::FunctionalRequest,
            functional.evidence.clone(),
        );
    }
    traces.insert(
        ProvenanceField::Capability,
        plan.read_only_capability.evidence.clone(),
    );
    traces.insert(
        ProvenanceField::ObservedCalibration,
        observed_calibration.evidence.clone(),
    );

    let request = J1979Request::calibration_identification();
    let encoded_payload = request.encoded();

    Ok(PreparedDiagnosticTransaction {
        safety_class: TransactionSafetyClass::ReadOnly,
        target,
        logical_network: plan.logical_network.value.clone(),
        physical_connector: plan.physical_route.value.connector.clone(),
        physical_pins: plan.physical_route.value.pins.clone(),
        backend_route: plan.backend_route.value.route_id.clone(),
        bitrate_bps: plan.bitrate_bps.value,
        protocol_family: plan.protocol_family.value.clone(),
        addressing_mode: plan.addressing_mode.value.clone(),
        physical_request_id,
        expected_response_id,
        functional_request_id,
        capability_id: plan.read_only_capability.value.id.clone(),
        observed_calibration: observed_calibration.value.clone(),
        request,
        encoded_payload,
        provenance: ExecutionProvenance { traces },
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfflineExecutionSource {
    Simulator,
    Replay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionFixtureClass {
    Synthetic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionFixtureProvenance {
    pub name: String,
    pub fixture_class: ExecutionFixtureClass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalibrationIdentificationExecutionResult {
    pub source: OfflineExecutionSource,
    pub target: DiagnosticTargetIdentity,
    pub physical_request_id: CanId,
    pub responder: CanId,
    pub functional_request_id: Option<CanId>,
    pub request_payload: Vec<u8>,
    pub raw_diagnostic_response: Vec<u8>,
    pub calibration_ids: Vec<String>,
    pub environment_provenance: ExecutionProvenance,
    pub execution_fixture: ExecutionFixtureProvenance,
    pub completed_timestamp_us: u64,
}

#[derive(Debug)]
pub enum ExecutionError {
    Source(CanSourceError),
    IsoTp(isotp::IsoTpError),
    J1979(J1979Error),
    UnexpectedResponder { expected: CanId, actual: CanId },
    Timeout,
    TruncatedIsoTp,
    InvalidFixtureName,
    NonSyntheticReplayFixture,
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => write!(formatter, "{error}"),
            Self::IsoTp(error) => write!(formatter, "{error}"),
            Self::J1979(error) => write!(formatter, "{error}"),
            Self::UnexpectedResponder { expected, actual } => write!(
                formatter,
                "unexpected responder: expected {:#x}, got {:#x}",
                expected.value(),
                actual.value()
            ),
            Self::Timeout => formatter.write_str("offline diagnostic response timed out"),
            Self::TruncatedIsoTp => {
                formatter.write_str("offline CAN source ended during ISO-TP message")
            }
            Self::InvalidFixtureName => formatter.write_str("execution fixture name is empty"),
            Self::NonSyntheticReplayFixture => {
                formatter.write_str("F7 replay fixture must be explicitly synthetic")
            }
        }
    }
}

impl std::error::Error for ExecutionError {}

impl From<CanSourceError> for ExecutionError {
    fn from(value: CanSourceError) -> Self {
        Self::Source(value)
    }
}

impl From<isotp::IsoTpError> for ExecutionError {
    fn from(value: isotp::IsoTpError) -> Self {
        Self::IsoTp(value)
    }
}

impl From<J1979Error> for ExecutionError {
    fn from(value: J1979Error) -> Self {
        Self::J1979(value)
    }
}

pub fn execute_simulator(
    transaction: &PreparedDiagnosticTransaction,
    source: &mut SimulatorSource,
    fixture_name: impl Into<String>,
    timeout_us: u64,
) -> Result<CalibrationIdentificationExecutionResult, ExecutionError> {
    let name = fixture_name.into();
    if name.trim().is_empty() {
        return Err(ExecutionError::InvalidFixtureName);
    }
    source.validate_expected_request_payload(&transaction.encoded_payload)?;
    execute_source(
        transaction,
        source,
        OfflineExecutionSource::Simulator,
        ExecutionFixtureProvenance {
            name,
            fixture_class: ExecutionFixtureClass::Synthetic,
        },
        timeout_us,
    )
}

pub fn execute_replay(
    transaction: &PreparedDiagnosticTransaction,
    source: &mut ReplaySource,
    timeout_us: u64,
) -> Result<CalibrationIdentificationExecutionResult, ExecutionError> {
    if source.fixture().fixture_class != FixtureClass::Synthetic {
        return Err(ExecutionError::NonSyntheticReplayFixture);
    }
    let fixture = ExecutionFixtureProvenance {
        name: source.fixture().name.clone(),
        fixture_class: ExecutionFixtureClass::Synthetic,
    };
    execute_source(
        transaction,
        source,
        OfflineExecutionSource::Replay,
        fixture,
        timeout_us,
    )
}

fn execute_source(
    transaction: &PreparedDiagnosticTransaction,
    source: &mut impl CanFrameSource,
    execution_source: OfflineExecutionSource,
    execution_fixture: ExecutionFixtureProvenance,
    timeout_us: u64,
) -> Result<CalibrationIdentificationExecutionResult, ExecutionError> {
    let required_kind = match execution_source {
        OfflineExecutionSource::Simulator => CanSourceKind::Simulator,
        OfflineExecutionSource::Replay => CanSourceKind::Replay,
    };
    debug_assert_eq!(source.source_kind(), required_kind);
    let mut reassembler = isotp::Reassembler::new();
    let mut first_timestamp = None;
    let mut last_timestamp = None;

    while let Some(frame) = source.next_frame()? {
        if frame.route != transaction.backend_route {
            continue;
        }
        if frame.id != transaction.expected_response_id {
            return Err(ExecutionError::UnexpectedResponder {
                expected: transaction.expected_response_id,
                actual: frame.id,
            });
        }
        let start = *first_timestamp.get_or_insert(frame.timestamp_us);
        if frame.timestamp_us.saturating_sub(start) > timeout_us {
            return Err(ExecutionError::Timeout);
        }
        if last_timestamp.is_some() {
            reassembler.expire(frame.timestamp_us, timeout_us)?;
        }
        last_timestamp = Some(frame.timestamp_us);
        if let Some(payload) = reassembler.accept(&frame.data, frame.timestamp_us)? {
            let CalibrationIdentificationResult { calibration_ids } =
                decode_calibration_identification(transaction.request, &payload)?;
            return Ok(CalibrationIdentificationExecutionResult {
                source: execution_source,
                target: transaction.target.clone(),
                physical_request_id: transaction.physical_request_id,
                responder: transaction.expected_response_id,
                functional_request_id: transaction.functional_request_id,
                request_payload: transaction.encoded_payload.to_vec(),
                raw_diagnostic_response: payload,
                calibration_ids,
                environment_provenance: transaction.provenance.clone(),
                execution_fixture,
                completed_timestamp_us: frame.timestamp_us,
            });
        }
    }

    if reassembler.is_pending() {
        Err(ExecutionError::TruncatedIsoTp)
    } else {
        Err(ExecutionError::Timeout)
    }
}
