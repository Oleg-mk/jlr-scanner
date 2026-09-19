//! Compilation of evidence-backed UDS intent, and its offline execution.
//!
//! The sibling of `diagnostic-execution` decided in ADR-0012: the same one-way
//! boundary from an F6 resolved plan into a prepared transaction, for ISO
//! 14229. The reads - ReadDataByIdentifier and ReadDTCInformation - compile to
//! a [`PreparedUdsTransaction`] and run offline here or live on the adapter.
//! Since ADR-0036 the first service operation, ClearDiagnosticInformation,
//! compiles to a [`PreparedUdsService`]: the same route, a class that is not
//! read-only, and the session it may need. It runs live only, after the
//! person's confirmation, and the read path refuses it by its class. There is
//! still no arbitrary payload and no CAN transmission API.

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
/// The capability a prepared clear carries (`ADR-0036`): this product's own
/// name for the service operation. Nothing in the knowledge base claims a
/// module accepts a clear; ISO 14229 does.
pub const CLEAR_DIAGNOSTIC_INFORMATION_CAPABILITY: &str =
    "uds.service14.clear_diagnostic_information.service_routine";
/// The operation name `SAFETY_BOUNDARIES.md` and the report use for a clear.
pub const DTC_CLEAR_OPERATION: &str = "DTC_CLEAR";
/// The routine control a prepared service carries (`ADR-0036`, step 2).
pub const ROUTINE_CONTROL_CAPABILITY: &str = "uds.service31.routine_control.service_routine";
/// The operation's name in the safety table and the report.
pub const ROUTINE_RUN_OPERATION: &str = "ROUTINE_RUN";
/// The on-demand self test: the one routine of stage 2's second step.
pub const SELF_TEST_ROUTINE: u16 = 0x0202;
/// Every routine identifier this product will send, in every step of
/// stage 2 so far - the closed set the architecture guard reads.
pub const STAGE_2_ROUTINES: &[u16] = &[SELF_TEST_ROUTINE];
/// The group that means every code, re-exported so the shell needs no protocol crate.
pub use uds::ALL_DTC_GROUPS;
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

/// A routine of stage 2 as a closed type: one value per routine the
/// record allows, so that a routine identifier is never a number the
/// interface chose (`ADR-0036`, step 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageTwoRoutine {
    /// `0x0202`, the on-demand self test.
    SelfTest,
}

impl StageTwoRoutine {
    pub fn identifier(self) -> u16 {
        match self {
            Self::SelfTest => SELF_TEST_ROUTINE,
        }
    }
}

/// What a routine control asks: the three sub-functions of `0x31`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoutineStep {
    Start,
    Stop,
    RequestResults,
}

impl RoutineStep {
    pub fn sub_function(self) -> u8 {
        match self {
            Self::Start => uds::ROUTINE_CONTROL_START,
            Self::Stop => uds::ROUTINE_CONTROL_STOP,
            Self::RequestResults => uds::ROUTINE_CONTROL_REQUEST_RESULTS,
        }
    }
}

/// The service operations of stage 2 (`ADR-0036`), one per step. Each names
/// its target and what it asks; the class and the session come with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceUdsIntent {
    /// `0x14`: clear the diagnostic information of a group of codes -
    /// [`uds::ALL_DTC_GROUPS`] for every code the module holds.
    ClearDiagnosticInformation {
        target: DiagnosticTargetIdentity,
        group: u32,
    },
    /// `0x31`: one step of a routine the module declares - start it, ask
    /// its results, or stop it (`ADR-0036`, step 2).
    RoutineControl {
        target: DiagnosticTargetIdentity,
        routine: StageTwoRoutine,
        step: RoutineStep,
    },
}

impl ServiceUdsIntent {
    pub fn clear_diagnostic_information(target: DiagnosticTargetIdentity, group: u32) -> Self {
        Self::ClearDiagnosticInformation { target, group }
    }

    pub fn routine_control(
        target: DiagnosticTargetIdentity,
        routine: StageTwoRoutine,
        step: RoutineStep,
    ) -> Self {
        Self::RoutineControl {
            target,
            routine,
            step,
        }
    }

    fn target(&self) -> &DiagnosticTargetIdentity {
        match self {
            Self::ClearDiagnosticInformation { target, .. } => target,
            Self::RoutineControl { target, .. } => target,
        }
    }

    /// The operation's name in the safety table and the report.
    pub fn operation(&self) -> &'static str {
        match self {
            Self::ClearDiagnosticInformation { .. } => DTC_CLEAR_OPERATION,
            Self::RoutineControl { .. } => ROUTINE_RUN_OPERATION,
        }
    }

    pub fn safety_class(&self) -> TransactionSafetyClass {
        match self {
            Self::ClearDiagnosticInformation { .. } | Self::RoutineControl { .. } => {
                TransactionSafetyClass::ServiceRoutine
            }
        }
    }

    /// How the operation uses the extended diagnostic session.
    pub fn session_use(&self) -> SessionUse {
        match self {
            // ISO 14229 allows a clear in the default session; a module that
            // refuses it there is asked again in the extended one.
            Self::ClearDiagnosticInformation { .. } => SessionUse::DefaultThenExtended,
            // The data requires the extended session for every routine it
            // declares: opened at the start and held while the routine runs;
            // the request for results and the stop each end the run, so each
            // brings the module back to the default session.
            Self::RoutineControl {
                step: RoutineStep::Start,
                ..
            } => SessionUse::OpenAndHold,
            Self::RoutineControl { .. } => SessionUse::KeepAliveThenLeave,
        }
    }
}

/// How an operation uses the extended diagnostic session (`ADR-0036`,
/// decision 4). Whatever it uses, the session is left when it ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionUse {
    /// Try in the default session; on `serviceNotSupportedInActiveSession`
    /// open the extended one and try once more.
    DefaultThenExtended,
    /// Open the extended session first: the data requires it.
    Extended,
    /// Open the extended session first and leave it open: the operation
    /// starts a routine the module keeps running, and the executor's
    /// keep-alive holds the session until the run ends (`ADR-0036`, step 2).
    OpenAndHold,
    /// A keep-alive first, then the operation, then the default session
    /// back: the last step of a routine run, whatever it answers.
    KeepAliveThenLeave,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransactionSafetyClass {
    ReadOnly,
    /// A validated service procedure (`ADR-0036`): the clear of the codes.
    ServiceRoutine,
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

/// One service operation, compiled from a resolved plan (`ADR-0036`). It
/// carries the same route a read does and a class that is not read-only;
/// the live path takes it through its own function and the read function
/// refuses it by its class.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedUdsService {
    transaction: PreparedUdsTransaction,
    operation: &'static str,
    session_use: SessionUse,
}

impl PreparedUdsService {
    pub fn transaction(&self) -> &PreparedUdsTransaction {
        &self.transaction
    }

    /// A read dressed as a service, for the tests that prove the live path
    /// refuses it by its class. Never built by the product.
    #[doc(hidden)]
    pub fn from_read_for_test(transaction: PreparedUdsTransaction) -> Self {
        Self {
            transaction,
            operation: DTC_CLEAR_OPERATION,
            session_use: SessionUse::DefaultThenExtended,
        }
    }

    pub fn operation(&self) -> &'static str {
        self.operation
    }

    pub fn session_use(&self) -> SessionUse {
        self.session_use
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
    MissingDiagnosticImplementation,
    UnsupportedProtocol(String),
    UnsupportedCapability(String),
    InvalidRoute(&'static str),
    InvalidBitrate(u32),
    /// The plan names no CAN identifier format; UDS over ISO-TP has one.
    MissingCanIdFormat,
    UnsupportedAddressingMode(String),
    InvalidCanId {
        field: &'static str,
        value: u32,
    },
    IdentifierNotReadable {
        identifier: u16,
    },
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
            Self::MissingCanIdFormat => {
                formatter.write_str("the plan names no CAN identifier format")
            }
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
    let mut route = compile_route(
        resolution,
        intent.target().clone(),
        Some(intent.required_capability()),
    )?;
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
    if let Some(entry) = &identifier {
        route
            .provenance
            .traces
            .insert(ProvenanceField::Identifier, entry.evidence.clone());
    }
    let capability_id = route.capability_id.clone();
    Ok(route.into_transaction(
        TransactionSafetyClass::ReadOnly,
        capability_id,
        request,
        identifier,
    ))
}

/// Compile one service operation against a RESOLVED plan (`ADR-0036`): the
/// same route a read takes, the request ISO 14229 names for it, the class
/// that keeps it out of the read path, and how it uses the extended session.
/// The plan's read capability is not what admits it - a clear is a standard
/// service - but the plan must still be resolved, ISO 14229, for this module.
pub fn prepare_service_transaction(
    resolution: &DiagnosticEnvironmentResolution,
    intent: ServiceUdsIntent,
) -> Result<PreparedUdsService, PreparationError> {
    let route = compile_route(resolution, intent.target().clone(), None)?;
    let (request, capability_id) = match &intent {
        ServiceUdsIntent::ClearDiagnosticInformation { group, .. } => (
            UdsRequest::clear_diagnostic_information(*group),
            CLEAR_DIAGNOSTIC_INFORMATION_CAPABILITY.to_string(),
        ),
        ServiceUdsIntent::RoutineControl { routine, step, .. } => (
            UdsRequest::routine_control(step.sub_function(), routine.identifier()),
            ROUTINE_CONTROL_CAPABILITY.to_string(),
        ),
    };
    Ok(PreparedUdsService {
        transaction: route.into_transaction(intent.safety_class(), capability_id, request, None),
        operation: intent.operation(),
        session_use: intent.session_use(),
    })
}

/// Everything a prepared UDS transaction takes from the plan, before the
/// request itself is chosen.
struct CompiledRoute {
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
    provenance: ExecutionProvenance,
}

impl CompiledRoute {
    fn into_transaction(
        self,
        safety_class: TransactionSafetyClass,
        capability_id: String,
        request: UdsRequest,
        identifier: Option<ReadableIdentifier>,
    ) -> PreparedUdsTransaction {
        PreparedUdsTransaction {
            safety_class,
            target: self.target,
            logical_network: self.logical_network,
            physical_connector: self.physical_connector,
            physical_pins: self.physical_pins,
            backend_route: self.backend_route,
            bitrate_bps: self.bitrate_bps,
            protocol_family: self.protocol_family,
            addressing_mode: self.addressing_mode,
            physical_request_id: self.physical_request_id,
            expected_response_id: self.expected_response_id,
            functional_request_id: self.functional_request_id,
            capability_id,
            request,
            identifier,
            provenance: self.provenance,
        }
    }
}

/// The plan checked and its route taken: resolved, for this module, ISO
/// 14229 in an addressing mode this crate frames, every mandatory field
/// present, the identifiers valid for the declared width.
/// `required_capability` is the read-only capability a read must find in the
/// plan; a service passes `None`, because the plan's read capability does not
/// admit it and nothing in the knowledge base claims a module accepts a clear.
fn compile_route(
    resolution: &DiagnosticEnvironmentResolution,
    target: DiagnosticTargetIdentity,
    required_capability: Option<&str>,
) -> Result<CompiledRoute, PreparationError> {
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
    if let Some(required) = required_capability {
        if plan.read_only_capability.value.id != required {
            return Err(PreparationError::UnsupportedCapability(
                plan.read_only_capability.value.id.clone(),
            ));
        }
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

    let Some(can_id_format) = plan.can_id_format.as_ref() else {
        return Err(PreparationError::MissingCanIdFormat);
    };
    let can_id = |field, value| match can_id_format.value {
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
    traces.insert(ProvenanceField::CanIdFormat, can_id_format.evidence.clone());
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

    Ok(CompiledRoute {
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

/// What the module answered to a service operation (`ADR-0036`). A
/// negative response is an answer, not a failure of this crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UdsServiceOutcome {
    /// The module cleared the group it was asked to clear.
    Cleared {
        group: u32,
    },
    /// The module answered a routine control: the step and the routine
    /// echoed, and the status record as the module wrote it.
    RoutineControlled {
        sub_function: u8,
        routine: u16,
        status: Vec<u8>,
    },
    Negative(NegativeResponse),
}

/// Decode one complete response to a prepared service, for the live backend.
/// `Ok(None)` is ResponsePending: the module is still working.
pub fn decode_service_response(
    service: &PreparedUdsService,
    payload: &[u8],
) -> Result<Option<UdsServiceOutcome>, ExecutionError> {
    let request = &service.transaction.request;
    let response = uds::parse_response(request, payload)?;
    if let UdsResponse::Negative(negative) = &response {
        if negative.code == NegativeResponseCode::ResponsePending {
            return Ok(None);
        }
    }
    let outcome = match uds::typed_result(request, response)? {
        TypedDiagnosticResult::ClearDiagnosticInformation { group } => {
            UdsServiceOutcome::Cleared { group }
        }
        TypedDiagnosticResult::RoutineControl {
            sub_function,
            routine,
            status,
        } => UdsServiceOutcome::RoutineControlled {
            sub_function,
            routine,
            status,
        },
        TypedDiagnosticResult::Negative(negative) => UdsServiceOutcome::Negative(negative),
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
