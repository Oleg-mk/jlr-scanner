//! Offline-only compilation and execution of evidence-backed read-only UDS
//! intent.
//!
//! The sibling of `diagnostic-execution` decided in ADR-0012: the same one-way
//! boundary from an F6 resolved plan into offline replay or simulator
//! execution, for ISO 14229 reads. Only ReadDataByIdentifier and
//! ReadDTCInformation are reachable. There is no live source, no session
//! control, no arbitrary payload and no CAN transmission API.

use diagnostic_environment::{
    CanIdFormat, DiagnosticEnvironmentResolution, PlanEvidenceTrace, ReadableIdentifier,
    ResolutionConflict, UnresolvedFact,
};
use diagnostic_simulator::SimulatorSource;
use std::collections::BTreeMap;
use std::fmt;
use transport_api::{CanFrameSource, CanId, CanSourceError, CanSourceKind};
use transport_replay::{FixtureClass, ReplaySource};
use uds::{
    DtcStatusReport, NegativeResponse, NegativeResponseCode, TypedDiagnosticResult, UdsError,
    UdsRequest, UdsResponse, SUB_FUNCTION_REPORT_DTC_BY_STATUS_MASK,
};

/// Capability identifiers the knowledge base records for ISO 14229 modules.
/// The SDD platform adapter writes the same literals; its tests assert that.
pub const READ_DATA_BY_IDENTIFIER_CAPABILITY: &str =
    "uds.service22.read_data_by_identifier.read_only";
pub const READ_DTC_INFORMATION_CAPABILITY: &str = "uds.service19.read_dtc_information.read_only";
/// Diagnostic protocol name the SDD platform corpus declares for UDS buses.
pub const UDS_PROTOCOL_FAMILY: &str = "ISO14229";
/// ISO 15765-2 addressing modes this crate can frame: the request and response
/// identifiers are complete CAN identifiers and the first data byte is the
/// N_PCI. SDD's `enhanced` mode prefixes a network address and is refused.
const SUPPORTED_ADDRESSING_MODES: [&str; 2] = ["normal", "normal_fixed"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticTargetIdentity {
    ecu_family: String,
    diagnostic_implementation: Option<String>,
}

impl DiagnosticTargetIdentity {
    /// A module family, as SDD describes it (ADR-0013).
    pub fn family(ecu_family: impl Into<String>) -> Result<Self, PreparationError> {
        Self::build(ecu_family.into(), None)
    }

    /// A specific software build of a family, when the caller knows one.
    pub fn implementation(
        ecu_family: impl Into<String>,
        diagnostic_implementation: impl Into<String>,
    ) -> Result<Self, PreparationError> {
        Self::build(ecu_family.into(), Some(diagnostic_implementation.into()))
    }

    fn build(
        ecu_family: String,
        diagnostic_implementation: Option<String>,
    ) -> Result<Self, PreparationError> {
        if ecu_family.trim().is_empty()
            || diagnostic_implementation
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
        {
            return Err(PreparationError::EmptyTargetIdentity);
        }
        Ok(Self {
            ecu_family,
            diagnostic_implementation,
        })
    }

    pub fn ecu_family(&self) -> &str {
        &self.ecu_family
    }

    pub fn diagnostic_implementation(&self) -> Option<&str> {
        self.diagnostic_implementation.as_deref()
    }
}

/// Which status bits a ReadDTCInformation request asks for. Typed so that no
/// caller composes a raw mask byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DtcStatusMask {
    /// `testFailed`: failing right now.
    TestFailed,
    /// `pendingDTC`: failed during the current or previous operation cycle.
    Pending,
    /// `confirmedDTC`: stored as confirmed.
    Confirmed,
    /// Every DTC with any status bit set.
    AnyStatus,
}

impl DtcStatusMask {
    pub fn byte(self) -> u8 {
        match self {
            Self::TestFailed => 0x01,
            Self::Pending => 0x04,
            Self::Confirmed => 0x08,
            Self::AnyStatus => 0xFF,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadOnlyUdsIntent {
    ReadDataByIdentifier {
        target: DiagnosticTargetIdentity,
        identifier: u16,
    },
    ReadDtcInformation {
        target: DiagnosticTargetIdentity,
        status_mask: DtcStatusMask,
    },
}

impl ReadOnlyUdsIntent {
    pub fn read_data_by_identifier(target: DiagnosticTargetIdentity, identifier: u16) -> Self {
        Self::ReadDataByIdentifier { target, identifier }
    }

    pub fn read_dtc_information(
        target: DiagnosticTargetIdentity,
        status_mask: DtcStatusMask,
    ) -> Self {
        Self::ReadDtcInformation {
            target,
            status_mask,
        }
    }

    fn target(&self) -> &DiagnosticTargetIdentity {
        match self {
            Self::ReadDataByIdentifier { target, .. } | Self::ReadDtcInformation { target, .. } => {
                target
            }
        }
    }

    fn required_capability(&self) -> &'static str {
        match self {
            Self::ReadDataByIdentifier { .. } => READ_DATA_BY_IDENTIFIER_CAPABILITY,
            Self::ReadDtcInformation { .. } => READ_DTC_INFORMATION_CAPABILITY,
        }
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
    /// Why the requested identifier was allowed: the catalogue entries that
    /// list it for this module (ADR-0012 gate).
    Identifier,
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
pub struct PreparedUdsTransaction {
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
    request: UdsRequest,
    identifier: Option<ReadableIdentifier>,
    provenance: ExecutionProvenance,
}

impl PreparedUdsTransaction {
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

    pub fn protocol_request(&self) -> &UdsRequest {
        &self.request
    }

    pub fn encoded_payload(&self) -> &[u8] {
        self.request.as_bytes()
    }

    /// The catalogue entry that admitted a ReadDataByIdentifier request.
    pub fn readable_identifier(&self) -> Option<&ReadableIdentifier> {
        self.identifier.as_ref()
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
    MissingDiagnosticImplementation,
    UnsupportedProtocol(String),
    UnsupportedCapability(String),
    InvalidRoute(&'static str),
    InvalidBitrate(u32),
    UnsupportedAddressingMode(String),
    InvalidCanId { field: &'static str, value: u32 },
    IdentifierNotReadable { identifier: u16 },
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
            Self::MissingDiagnosticImplementation => formatter.write_str(
                "target names an implementation but the plan is family-level and has none",
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
            Self::IdentifierNotReadable { identifier } => write!(
                formatter,
                "identifier {identifier:#06x} is not listed as readable for this module"
            ),
        }
    }
}

impl std::error::Error for PreparationError {}

/// Compile typed read-only UDS intent against a RESOLVED plan.
///
/// `readable` is the identifier catalogue F10 derived for the target module;
/// a ReadDataByIdentifier intent naming anything outside it is refused before
/// a single byte is encoded. ReadDTCInformation needs no identifier.
pub fn prepare_read_only_transaction(
    resolution: &DiagnosticEnvironmentResolution,
    intent: ReadOnlyUdsIntent,
    readable: &[ReadableIdentifier],
) -> Result<PreparedUdsTransaction, PreparationError> {
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

    let target = intent.target().clone();
    if target.ecu_family != plan.ecu_family.value {
        return Err(PreparationError::TargetEcuMismatch {
            expected: target.ecu_family,
            actual: plan.ecu_family.value.clone(),
        });
    }
    match (
        &target.diagnostic_implementation,
        &plan.diagnostic_implementation,
    ) {
        (Some(expected), Some(actual)) if expected != &actual.value => {
            return Err(PreparationError::TargetImplementationMismatch {
                expected: expected.clone(),
                actual: actual.value.clone(),
            })
        }
        (Some(_), None) => return Err(PreparationError::MissingDiagnosticImplementation),
        // A family target accepts any plan for the family; a family-level
        // plan is exactly what a family target asked for.
        _ => {}
    }
    if plan.protocol_family.value != UDS_PROTOCOL_FAMILY {
        return Err(PreparationError::UnsupportedProtocol(
            plan.protocol_family.value.clone(),
        ));
    }
    let required_capability = intent.required_capability();
    if plan.read_only_capability.value.id != required_capability {
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
    if !SUPPORTED_ADDRESSING_MODES.contains(&plan.addressing_mode.value.as_str()) {
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

    let (request, identifier) = match &intent {
        ReadOnlyUdsIntent::ReadDataByIdentifier { identifier, .. } => {
            let entry = readable
                .iter()
                .find(|entry| entry.identifier == *identifier)
                .cloned()
                .ok_or(PreparationError::IdentifierNotReadable {
                    identifier: *identifier,
                })?;
            (
                UdsRequest::read_data_by_identifier(*identifier),
                Some(entry),
            )
        }
        ReadOnlyUdsIntent::ReadDtcInformation { status_mask, .. } => (
            UdsRequest::read_dtc_information(
                SUB_FUNCTION_REPORT_DTC_BY_STATUS_MASK,
                &[status_mask.byte()],
            ),
            None,
        ),
    };

    let mut traces = BTreeMap::new();
    traces.insert(
        ProvenanceField::VehicleApplicability,
        plan.vehicle_applicability.evidence.clone(),
    );
    traces.insert(ProvenanceField::EcuFamily, plan.ecu_family.evidence.clone());
    if let Some(implementation) = &plan.diagnostic_implementation {
        traces.insert(
            ProvenanceField::DiagnosticImplementation,
            implementation.evidence.clone(),
        );
    }
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
    if let Some(entry) = &identifier {
        traces.insert(ProvenanceField::Identifier, entry.evidence.clone());
    }

    Ok(PreparedUdsTransaction {
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
        request,
        identifier,
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

/// What the module answered. A negative response is an answer, not a failure
/// of this crate, so it is a result carrying its code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UdsReadOutcome {
    DataByIdentifier { identifier: u16, data: Vec<u8> },
    DtcReport(DtcStatusReport),
    Negative(NegativeResponse),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UdsExecutionResult {
    pub source: OfflineExecutionSource,
    pub target: DiagnosticTargetIdentity,
    pub physical_request_id: CanId,
    pub responder: CanId,
    pub functional_request_id: Option<CanId>,
    pub request_payload: Vec<u8>,
    pub raw_diagnostic_response: Vec<u8>,
    pub outcome: UdsReadOutcome,
    pub environment_provenance: ExecutionProvenance,
    pub execution_fixture: ExecutionFixtureProvenance,
    pub completed_timestamp_us: u64,
}

#[derive(Debug)]
pub enum ExecutionError {
    Source(CanSourceError),
    IsoTp(isotp::IsoTpError),
    Uds(UdsError),
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
            Self::Uds(error) => write!(formatter, "{error}"),
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
                formatter.write_str("replay fixture must be explicitly synthetic")
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

impl From<UdsError> for ExecutionError {
    fn from(value: UdsError) -> Self {
        Self::Uds(value)
    }
}

pub fn execute_simulator(
    transaction: &PreparedUdsTransaction,
    source: &mut SimulatorSource,
    fixture_name: impl Into<String>,
    timeout_us: u64,
) -> Result<UdsExecutionResult, ExecutionError> {
    let name = fixture_name.into();
    if name.trim().is_empty() {
        return Err(ExecutionError::InvalidFixtureName);
    }
    source.validate_expected_request_payload(transaction.encoded_payload())?;
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
    transaction: &PreparedUdsTransaction,
    source: &mut ReplaySource,
    timeout_us: u64,
) -> Result<UdsExecutionResult, ExecutionError> {
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

/// Decode one complete response to a prepared read, for the offline sources
/// here and for the live backend alike. `Ok(None)` is ResponsePending: the
/// module is still working and the caller should keep waiting.
pub fn decode_response(
    transaction: &PreparedUdsTransaction,
    payload: &[u8],
) -> Result<Option<UdsReadOutcome>, ExecutionError> {
    let response = uds::parse_response(&transaction.request, payload)?;
    if let UdsResponse::Negative(negative) = &response {
        if negative.code == NegativeResponseCode::ResponsePending {
            return Ok(None);
        }
    }
    let outcome = match uds::typed_result(&transaction.request, response)? {
        TypedDiagnosticResult::ReadDataByIdentifier { identifier, data } => {
            UdsReadOutcome::DataByIdentifier { identifier, data }
        }
        TypedDiagnosticResult::ReadDtcInformation { data, .. } => {
            UdsReadOutcome::DtcReport(uds::decode_dtc_by_status_mask(&data)?)
        }
        TypedDiagnosticResult::Negative(negative) => UdsReadOutcome::Negative(negative),
        // The two request constructors cannot yield any other service;
        // anything else is an uncorrelated answer.
        _ => return Err(ExecutionError::Uds(UdsError::CorrelationMismatch)),
    };
    Ok(Some(outcome))
}

fn execute_source(
    transaction: &PreparedUdsTransaction,
    source: &mut impl CanFrameSource,
    execution_source: OfflineExecutionSource,
    execution_fixture: ExecutionFixtureProvenance,
    timeout_us: u64,
) -> Result<UdsExecutionResult, ExecutionError> {
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

        let Some(outcome) = decode_response(transaction, &payload)? else {
            // NRC 0x78: the module is still working. ISO 14229 restarts the
            // response timer on it, so the window restarts here.
            window_start = Some(frame.timestamp_us);
            reassembler = isotp::Reassembler::new();
            continue;
        };
        return Ok(UdsExecutionResult {
            source: execution_source,
            target: transaction.target.clone(),
            physical_request_id: transaction.physical_request_id,
            responder: transaction.expected_response_id,
            functional_request_id: transaction.functional_request_id,
            request_payload: transaction.encoded_payload().to_vec(),
            raw_diagnostic_response: payload,
            outcome,
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
