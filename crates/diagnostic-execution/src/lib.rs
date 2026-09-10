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
use knowledge::{EvidenceClass, SourceLocator, SourceType, ValidationState};
use obd_j1979::{
    decode_calibration_identification, decode_response, CalibrationIdentificationResult,
    J1979Error, J1979Request, J1979Response,
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

    /// The bytes that go on the wire: always one ISO-TP single frame.
    pub fn encoded_payload(&self) -> &[u8] {
        &self.encoded_payload
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
    TargetEcuMismatch {
        expected: String,
        actual: String,
    },
    TargetImplementationMismatch {
        expected: String,
        actual: String,
    },
    UnsupportedProtocol(String),
    UnsupportedCapability(String),
    InvalidRoute(&'static str),
    InvalidBitrate(u32),
    UnsupportedAddressingMode(String),
    InvalidCanId {
        field: &'static str,
        value: u32,
    },
    MissingObservedCalibration,
    MissingDiagnosticImplementation,
    /// ISO 15765-4 reserves eight responders; this is not one of them.
    InvalidResponder(u8),
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
            Self::InvalidResponder(index) => write!(
                formatter,
                "responder {index} is outside the eight ISO 15765-4 reserves (0–7)"
            ),
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

// ---------------------------------------------------------------------------
// The legislated OBD-II reads (ADR-0022, decision 7)
// ---------------------------------------------------------------------------

/// The capability every legislated read carries: SAE J1979 over ISO 15765-4,
/// read-only, on the addressing the standard itself fixes.
pub const STANDARD_OBD_CAPABILITY: &str = "obd.j1979.legislated.read_only";

/// The functional request every emissions responder hears.
pub const STANDARD_OBD_FUNCTIONAL_REQUEST_ID: u16 = 0x7DF;
/// ISO 15765-4 on the high-speed pair: 500 kbit/s, J1962 pins 6 and 14.
pub const STANDARD_OBD_BITRATE_BPS: u32 = 500_000;
pub const STANDARD_OBD_PINS: [u8; 2] = [6, 14];

/// The eight physical responders the standard reserves, request beside
/// answer. Both columns are the standard's own table — neither is derived
/// from the other, here or anywhere.
const STANDARD_OBD_RESPONDER_IDS: [(u16, u16); 8] = [
    (0x7E0, 0x7E8),
    (0x7E1, 0x7E9),
    (0x7E2, 0x7EA),
    (0x7E3, 0x7EB),
    (0x7E4, 0x7EC),
    (0x7E5, 0x7ED),
    (0x7E6, 0x7EE),
    (0x7E7, 0x7EF),
];

/// One of the eight: 0 is the engine controller on nearly every car, 1 is
/// usually the transmission, the rest whatever the maker put there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StandardObdResponder(u8);

impl StandardObdResponder {
    pub fn new(index: u8) -> Result<Self, PreparationError> {
        if usize::from(index) < STANDARD_OBD_RESPONDER_IDS.len() {
            Ok(Self(index))
        } else {
            Err(PreparationError::InvalidResponder(index))
        }
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub fn request_id(self) -> u16 {
        STANDARD_OBD_RESPONDER_IDS[usize::from(self.0)].0
    }

    pub fn response_id(self) -> u16 {
        STANDARD_OBD_RESPONDER_IDS[usize::from(self.0)].1
    }
}

/// The standard as evidence: one trace per thing it fixes.
fn standard_trace(evidence_id: &str, section: &str, key: &str) -> PlanEvidenceTrace {
    PlanEvidenceTrace {
        record_id: "record-iso15765-4-legislated-obd".into(),
        evidence_id: evidence_id.into(),
        source_id: "iso-15765-4".into(),
        source_type: SourceType::Documented,
        evidence_class: Some(EvidenceClass::StandardDocumentation),
        locator: SourceLocator {
            description: "ISO 15765-4, road vehicles, diagnostic communication over CAN, part 4: \
                          requirements for emissions-related systems"
                .into(),
            document_page: None,
            document_section: Some(section.into()),
            record_key: Some(key.into()),
            capture_timestamp_us: None,
        },
        validation_state: ValidationState::SourceBacked,
    }
}

/// The adapter's own route descriptor, as every other read cites it.
fn mongoose_hs_can_trace() -> PlanEvidenceTrace {
    PlanEvidenceTrace {
        record_id: "record-mongoose-hs-can-route".into(),
        evidence_id: "evidence-mongoose-jlr-hs-can-route".into(),
        source_id: "jlr-scanner-mongoose-jlr-routes".into(),
        source_type: SourceType::Documented,
        evidence_class: Some(EvidenceClass::SourceCode),
        locator: SourceLocator {
            description: "ProwlOne production Mongoose route descriptor".into(),
            document_page: None,
            document_section: Some("VEHICLE_ROUTES".into()),
            record_key: Some("VehicleRouteId::HsCan".into()),
            capture_timestamp_us: None,
        },
        validation_state: ValidationState::SourceBacked,
    }
}

/// A read-only legislated request to one of the standard's responders,
/// prepared from the standard alone. No resolver takes part: nothing here
/// depends on which car it is, which is the whole point of the standard.
pub fn prepare_standard_obd_transaction(
    request: J1979Request,
    responder: StandardObdResponder,
) -> Result<PreparedDiagnosticTransaction, PreparationError> {
    let (request_id, response_id) = STANDARD_OBD_RESPONDER_IDS[usize::from(responder.index())];
    let target = DiagnosticTargetIdentity::new(
        format!("obd.responder.{request_id:#05x}"),
        "iso15765-4.legislated",
    )?;
    let can = |field: &'static str, value: u16| {
        CanId::standard(value).map_err(|_| PreparationError::InvalidCanId {
            field,
            value: u32::from(value),
        })
    };
    let physical_request_id = can("physical_request_id", request_id)?;
    let expected_response_id = can("physical_response_id", response_id)?;
    let functional_request_id = Some(can(
        "functional_request_id",
        STANDARD_OBD_FUNCTIONAL_REQUEST_ID,
    )?);

    let addressing = standard_trace(
        "evidence-iso15765-4-addressing",
        "11-bit CAN identifiers for legislated OBD",
        "0x7DF functional; 0x7E0–0x7E7 physical requests; 0x7E8–0x7EF responses",
    );
    let physical = standard_trace(
        "evidence-iso15765-4-physical-layer",
        "physical layer",
        "ISO 11898 high-speed CAN at 500 kbit/s on J1962 pins 6 and 14",
    );
    let services = standard_trace(
        "evidence-sae-j1979-services",
        "SAE J1979 diagnostic services",
        "modes 01, 02, 03, 06, 07, 09 and 0A, read-only",
    );
    let backend = mongoose_hs_can_trace();

    let mut traces = BTreeMap::new();
    for field in [
        ProvenanceField::VehicleApplicability,
        ProvenanceField::EcuFamily,
        ProvenanceField::DiagnosticImplementation,
        ProvenanceField::ProtocolFamily,
        ProvenanceField::AddressingMode,
        ProvenanceField::CanIdFormat,
        ProvenanceField::PhysicalRequest,
        ProvenanceField::ExpectedResponse,
        ProvenanceField::FunctionalRequest,
    ] {
        traces.insert(field, vec![addressing.clone()]);
    }
    for field in [
        ProvenanceField::LogicalNetwork,
        ProvenanceField::PhysicalRoute,
        ProvenanceField::Bitrate,
    ] {
        traces.insert(field, vec![physical.clone()]);
    }
    traces.insert(ProvenanceField::BackendRoute, vec![backend]);
    traces.insert(ProvenanceField::Capability, vec![services]);

    let encoded_payload = request.encoded();
    Ok(PreparedDiagnosticTransaction {
        safety_class: TransactionSafetyClass::ReadOnly,
        target,
        logical_network: "HS-CAN".into(),
        physical_connector: "J1962".into(),
        physical_pins: STANDARD_OBD_PINS.to_vec(),
        backend_route: "hs-can".into(),
        bitrate_bps: STANDARD_OBD_BITRATE_BPS,
        protocol_family: "ISO15765-4 / SAE J1979".into(),
        addressing_mode: "normal_physical".into(),
        physical_request_id,
        expected_response_id,
        functional_request_id,
        capability_id: STANDARD_OBD_CAPABILITY.into(),
        observed_calibration: String::new(),
        request,
        encoded_payload,
        provenance: ExecutionProvenance { traces },
    })
}

/// What a legislated read produced offline: the raw answer and its reading.
#[derive(Clone, Debug, PartialEq)]
pub struct J1979ExecutionResult {
    pub source: OfflineExecutionSource,
    pub target: DiagnosticTargetIdentity,
    pub physical_request_id: CanId,
    pub responder: CanId,
    pub functional_request_id: Option<CanId>,
    pub request_payload: Vec<u8>,
    pub raw_diagnostic_response: Vec<u8>,
    pub response: J1979Response,
    pub environment_provenance: ExecutionProvenance,
    pub execution_fixture: ExecutionFixtureProvenance,
    pub completed_timestamp_us: u64,
}

pub fn execute_j1979_simulator(
    transaction: &PreparedDiagnosticTransaction,
    source: &mut SimulatorSource,
    fixture_name: impl Into<String>,
    timeout_us: u64,
) -> Result<J1979ExecutionResult, ExecutionError> {
    let name = fixture_name.into();
    if name.trim().is_empty() {
        return Err(ExecutionError::InvalidFixtureName);
    }
    source.validate_expected_request_payload(&transaction.encoded_payload)?;
    execute_j1979_source(
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

pub fn execute_j1979_replay(
    transaction: &PreparedDiagnosticTransaction,
    source: &mut ReplaySource,
    timeout_us: u64,
) -> Result<J1979ExecutionResult, ExecutionError> {
    if source.fixture().fixture_class != FixtureClass::Synthetic {
        return Err(ExecutionError::NonSyntheticReplayFixture);
    }
    let fixture = ExecutionFixtureProvenance {
        name: source.fixture().name.clone(),
        fixture_class: ExecutionFixtureClass::Synthetic,
    };
    execute_j1979_source(
        transaction,
        source,
        OfflineExecutionSource::Replay,
        fixture,
        timeout_us,
    )
}

/// The offline loop for any legislated request: the same reassembly the
/// calibration path uses, then the codec's own reading of the answer. A
/// ResponsePending (0x78) restarts the window, as ISO 14229 says it does.
fn execute_j1979_source(
    transaction: &PreparedDiagnosticTransaction,
    source: &mut impl CanFrameSource,
    execution_source: OfflineExecutionSource,
    execution_fixture: ExecutionFixtureProvenance,
    timeout_us: u64,
) -> Result<J1979ExecutionResult, ExecutionError> {
    let required_kind = match execution_source {
        OfflineExecutionSource::Simulator => CanSourceKind::Simulator,
        OfflineExecutionSource::Replay => CanSourceKind::Replay,
    };
    debug_assert_eq!(source.source_kind(), required_kind);
    let mut reassembler = isotp::Reassembler::new();
    let mut window_start = None;
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
        let start = *window_start.get_or_insert(frame.timestamp_us);
        if frame.timestamp_us.saturating_sub(start) > timeout_us {
            return Err(ExecutionError::Timeout);
        }
        if last_timestamp.is_some() {
            reassembler.expire(frame.timestamp_us, timeout_us)?;
        }
        last_timestamp = Some(frame.timestamp_us);
        let Some(payload) = reassembler.accept(&frame.data, frame.timestamp_us)? else {
            continue;
        };
        let response = match decode_response(transaction.request, &payload) {
            Err(J1979Error::NegativeResponse { code: 0x78, .. }) => {
                window_start = Some(frame.timestamp_us);
                reassembler = isotp::Reassembler::new();
                continue;
            }
            other => other?,
        };
        return Ok(J1979ExecutionResult {
            source: execution_source,
            target: transaction.target.clone(),
            physical_request_id: transaction.physical_request_id,
            responder: transaction.expected_response_id,
            functional_request_id: transaction.functional_request_id,
            request_payload: transaction.encoded_payload.clone(),
            raw_diagnostic_response: payload,
            response,
            environment_provenance: transaction.provenance.clone(),
            execution_fixture,
            completed_timestamp_us: frame.timestamp_us,
        });
    }

    if reassembler.is_pending() {
        Err(ExecutionError::TruncatedIsoTp)
    } else {
        Err(ExecutionError::Timeout)
    }
}
