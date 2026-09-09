//! Serializable contracts at the application-to-UI boundary.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdapterState {
    #[default]
    NoAdapter,
    AdapterDetected,
    Connecting,
    Connected,
    Error,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BoardCommunicationState {
    #[default]
    Unavailable,
    Pending,
    Verified,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InterfaceImplementationState {
    Available,
    UnsupportedByAdapter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VehicleValidationState {
    NotYetValidated,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdapterErrorCode {
    AdapterNotFound,
    AdapterSelectionRequired,
    AdapterAlreadyInUse,
    UnableToOpenAdapter,
    AdapterDisconnected,
    BoardCommunicationFailed,
    DiscoveryFailed,
    DisconnectFailed,
    /// The session already holds records of the other kind (bench or real);
    /// start a new session before switching (ADR-0020).
    SessionModeMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterSummary {
    pub name: String,
    pub port: String,
    pub usb_vid: u16,
    pub usb_pid: u16,
    pub serial_number: Option<String>,
    pub driver: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardInfoEvidence {
    pub response_command: String,
    pub raw_response_hex: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterInfo {
    pub name: String,
    pub connection_status: String,
    pub port: String,
    pub usb_vid: u16,
    pub usb_pid: u16,
    pub serial_number: Option<String>,
    pub transport: String,
    pub driver: Option<String>,
    pub backend: String,
    pub board_info: BoardInfoEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleInterfaceCapability {
    pub id: String,
    pub name: String,
    pub pins: String,
    pub nominal_bitrate: Option<u32>,
    pub hardware_confirmed: bool,
    pub fixture_tested: bool,
    pub implementation: InterfaceImplementationState,
    pub vehicle_validation: VehicleValidationState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFacingError {
    pub code: AdapterErrorCode,
    pub message: String,
    pub technical_details: Option<String>,
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterSnapshot {
    pub state: AdapterState,
    pub adapters: Vec<AdapterSummary>,
    pub selected_adapter_port: Option<String>,
    pub adapter: Option<AdapterInfo>,
    pub board_communication: BoardCommunicationState,
    pub capabilities: Vec<VehicleInterfaceCapability>,
    pub selection_required: bool,
    pub error: Option<UserFacingError>,
    pub vehicle_message: String,
    /// The bench scenario the session is connected on (ADR-0020): `0` is the
    /// healthy vehicle, any other number a picture of faults that number
    /// always paints. Absent unless the bench is what is connected.
    #[serde(default)]
    pub bench_scenario: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportedVehicleProfile {
    pub make: String,
    pub model: String,
    pub vehicle_program: String,
    pub model_year: u16,
    pub powertrain: String,
    pub module: String,
    pub ecu_target: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticState {
    Unavailable,
    Ready,
    Running,
    Succeeded,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticErrorCategory {
    AdapterNotFound,
    AdapterCommunicationFailed,
    UnsupportedVehicleProfile,
    CanConnectionFailed,
    NoResponseFromEcu,
    UnexpectedResponder,
    MalformedDiagnosticResponse,
    InternalFailure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticExecutionStage {
    Preparation,
    AdapterValidation,
    CanConnection,
    RequestTransmission,
    ResponseReception,
    IsoTpReassembly,
    DiagnosticDecode,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticError {
    pub category: DiagnosticErrorCategory,
    pub message: String,
    pub technical_details: Option<String>,
    pub stage: DiagnosticExecutionStage,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticSnapshot {
    pub profile: SupportedVehicleProfile,
    pub operation_name: String,
    pub state: DiagnosticState,
    pub calibration_id: Option<String>,
    pub error: Option<DiagnosticError>,
    pub report_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub schema_version: u16,
    pub application_version: String,
    pub os_name: String,
    pub os_version: String,
    pub timestamp_unix_ms: u64,
    pub session_id: String,
    pub execution_source: String,
    pub adapter_model: String,
    pub usb_vid: String,
    pub usb_pid: String,
    pub adapter_board_info_result: String,
    pub selected_vehicle_profile: SupportedVehicleProfile,
    pub ecu_target: String,
    pub logical_bus: String,
    pub physical_route: String,
    pub bitrate_bps: u32,
    pub protocol: String,
    pub request_id: String,
    pub expected_response_id: String,
    pub capability: String,
    pub request_payload: String,
    pub actual_responder: Option<String>,
    pub raw_diagnostic_response: Option<String>,
    pub decoded_result: Option<String>,
    pub timeout_or_error_category: Option<DiagnosticErrorCategory>,
    pub execution_stage: DiagnosticExecutionStage,
}

/// How much of the diagnostic data library the application holds.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LibraryState {
    /// Only the manifests built into the application.
    #[default]
    NotLoaded,
    Loaded,
    PartiallyLoaded,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestFailure {
    pub file: String,
    pub message: String,
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySnapshot {
    pub state: LibraryState,
    pub directory: Option<String>,
    pub manifests_loaded: u32,
    pub manifests_failed: u32,
    pub sources: u32,
    pub records: u32,
    pub failures: Vec<ManifestFailure>,
    pub message: String,
    /// Whose copy of the library this is, when it carries the owner's stamp.
    #[serde(default)]
    pub issue: Option<LibraryIssue>,
}

/// The stamp the owner puts on every library copy he hands out (`G4`): who
/// it was issued to, when, until when, a short code, and whether the copy
/// is what the owner signed. Since ADR-0019 the application loads a copy
/// only when `integrity` is `Matches`; otherwise the state is `Failed` and
/// this says why. The session report carries the issue of a loaded copy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryIssue {
    pub issued_to: String,
    pub issued_on: String,
    pub issue_code: String,
    /// Inclusive last day of validity, `YYYY-MM-DD` UTC; empty on old stamps.
    #[serde(default)]
    pub valid_until: String,
    /// Days from today to `valid_until`; negative once expired or unknown.
    #[serde(default)]
    pub days_left: i64,
    #[serde(default)]
    pub issuer: String,
    pub integrity: LibraryIssueIntegrity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LibraryIssueIntegrity {
    /// Every bundle's hash is the one the stamp names.
    Matches,
    /// A bundle differs from the stamp, or one is missing or added.
    Mismatch,
    /// No stamp file, but the data carries an issue code inside.
    StampRemoved,
    /// No stamp at all; the application refuses such a folder.
    NoStamp,
    /// A stamp without a signature: an older or a hand-made one.
    Unsigned,
    /// A signature no trusted key made, or one over other fields.
    BadSignature,
    /// Signed and matching, but past its `valid_until`.
    Expired,
}

/// What the user states about the vehicle, in the terms the SDD-derived
/// knowledge is qualified by. Until VIN decoding exists this is typed in.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleContextInput {
    pub vehicle_program: String,
    pub model_year: Option<u16>,
    pub powertrain: Option<String>,
    pub variant: Option<String>,
    pub market: Option<String>,
    /// SDD's own model-year breakpoint marker, such as `MY10`.
    pub year_breakpoint: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RouteStatus {
    Reachable,
    /// A route exists on paper only: its binding is an unverified research
    /// hypothesis that a first read-only request confirms or refutes.
    Hypothesis,
    Indeterminate,
    Conflict,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModuleApplicability {
    Applicable,
    InsufficientContext,
    InsufficientEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteSummary {
    pub status: RouteStatus,
    /// Why the route is not usable, one line per unresolved or conflicting
    /// fact, in the resolver's words. Empty when reachable.
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadableIdentifierSummary {
    pub identifier: String,
    pub parameters: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleSurveyEntry {
    pub ecu_family: String,
    /// SDD's own name for the module family in English, when the loaded
    /// data has one.
    pub name: Option<String>,
    /// The same name in every language the loaded data has, by SDD's
    /// language code (`eng`, `rus`, `deu`, …).
    #[serde(default)]
    pub names: BTreeMap<String, String>,
    pub applicability: ModuleApplicability,
    pub logical_network: Option<String>,
    pub request_id: Option<String>,
    pub response_id: Option<String>,
    pub backend_route: Option<String>,
    pub pins: Option<String>,
    pub bitrate_bps: Option<u32>,
    pub protocol: Option<String>,
    /// Weakest validation state of the route's facts, as the resolver reports
    /// it, such as `SOURCE_BACKED` or `UNVERIFIED`.
    pub route_validation: String,
    pub identifier_read: RouteSummary,
    pub dtc_read: RouteSummary,
    pub readable_identifiers: Vec<ReadableIdentifierSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleSurveySnapshot {
    pub context: VehicleContextInput,
    pub modules: Vec<ModuleSurveyEntry>,
    pub reachable: u32,
    pub hypothesis: u32,
    pub unreachable: u32,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CaptureState {
    Idle,
    Completed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureIdCount {
    pub id: String,
    pub extended: bool,
    pub count: u32,
}

/// What a listen-only capture established: counts, never an inference about
/// which vehicle bus was heard. The verdict says so in words.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSnapshot {
    pub state: CaptureState,
    pub route_id: String,
    pub pins: String,
    pub bitrate_bps: Option<u32>,
    pub requested_seconds: u32,
    pub listened_ms: u64,
    pub frames: u32,
    pub frames_per_second: u32,
    pub distinct_ids: u32,
    pub standard_frames: u32,
    pub extended_frames: u32,
    pub dropped_frames: u64,
    pub truncated: bool,
    pub top_ids: Vec<CaptureIdCount>,
    pub verdict: String,
    pub error: Option<String>,
    pub capture_available: bool,
    /// Heard on the bench, not on a car (ADR-0020): every count is synthetic.
    #[serde(default)]
    pub synthetic: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModuleReadState {
    Idle,
    Succeeded,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModuleReadKind {
    FaultCodes,
    Identifier,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleReadRequest {
    pub ecu_family: String,
    pub kind: ModuleReadKind,
    /// Hexadecimal identifier such as `0x1945`; only for `Identifier` reads.
    pub identifier: Option<String>,
    pub context: VehicleContextInput,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DtcSummary {
    pub code: String,
    pub failure_type: String,
    pub status: String,
    /// SDD's wording for the code, when the library holds it.
    pub description: Option<String>,
    /// `module` when the wording is scoped to the module read, `generic`
    /// when it is the programme-independent entry.
    pub description_scope: Option<String>,
    /// SDD's wording for the failure type byte, when the library holds it.
    pub failure_type_text: Option<String>,
    /// The same wording in every language the loaded data has, by SDD's
    /// language code.
    #[serde(default)]
    pub failure_type_texts: BTreeMap<String, String>,
    /// The code's own wording in the interface's languages, by the same
    /// language codes; the English stays in `description`. Empty when the
    /// code has no wording of ours.
    #[serde(default)]
    pub description_texts: BTreeMap<String, String>,
}

/// One decoded parameter of an identifier read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodedParameterSummary {
    pub name: String,
    pub raw: Option<u64>,
    pub value: Option<String>,
    pub unit: Option<String>,
    /// SDD's name for the raw-count range, when the catalogue names it.
    pub state: Option<String>,
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkerEntry {
    pub marker: String,
    pub model_year_from: Option<u16>,
    pub model_year_to: Option<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgrammeEntry {
    pub program: String,
    pub markers: Vec<MarkerEntry>,
    pub powertrains: Vec<String>,
    /// What SDD splits an engine into where it does — the naturally aspirated
    /// V8 of an X250 by displacement, for one — because those halves answer at
    /// different addresses. Empty when the programme's data splits nothing.
    #[serde(default)]
    pub variants: Vec<String>,
}

/// What one session has recorded so far and whether a report can be saved.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionReportSnapshot {
    pub captures: u32,
    pub module_reads: u32,
    pub calibration_reads: u32,
    pub report_available: bool,
    /// `bench` or `real`, once an adapter of either kind took part; a session
    /// is one or the other, never both (ADR-0020).
    #[serde(default)]
    pub mode: Option<String>,
}

/// One attribute SDD's VIN tables read off a VIN, by the table's own name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodedAttribute {
    pub name: String,
    pub value: String,
}

/// What SDD's VIN tables say about a VIN, and what they do not.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VinDecodeSnapshot {
    /// The VIN as decoded: upper case, without spaces or hyphens.
    pub vin: String,
    /// Whether it is a VIN by format at all.
    pub valid: bool,
    pub message: String,
    pub decode_model: Option<u32>,
    /// Every decode model whose rules claim the VIN, in SDD's order.
    pub candidates: Vec<u32>,
    /// The programme, when the tables name it (`Model`).
    pub program: Option<String>,
    pub model_year: Option<u16>,
    pub attributes: Vec<DecodedAttribute>,
    pub table_version: Option<String>,
}

/// What the loaded library can describe: the programmes SDD knows, their
/// breakpoint markers with derived model years, and the engines their data
/// is qualified by. Empty until a library with vehicle data is loaded.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleCatalogueSnapshot {
    pub programmes: Vec<ProgrammeEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleReadSnapshot {
    pub state: ModuleReadState,
    pub ecu_family: String,
    pub operation: String,
    pub route_id: String,
    pub route_validation: String,
    pub request_hex: String,
    pub responder: Option<String>,
    pub raw_response_hex: Option<String>,
    pub data_hex: Option<String>,
    pub parameters: Vec<DecodedParameterSummary>,
    pub dtcs: Vec<DtcSummary>,
    /// The negative response code, when the module declined; a completed
    /// read, not a failure of the application.
    pub negative_response: Option<String>,
    pub pending_responses: u32,
    pub error: Option<DiagnosticError>,
    pub report_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleReadReport {
    pub schema_version: u16,
    pub application_version: String,
    pub timestamp_unix_ms: u64,
    pub session_id: String,
    pub execution_source: String,
    pub adapter_model: String,
    pub usb_vid: String,
    pub usb_pid: String,
    pub adapter_board_info_result: String,
    pub vehicle: VehicleContextInput,
    pub ecu_family: String,
    pub operation: String,
    pub logical_bus: String,
    pub backend_route: String,
    pub physical_route: String,
    pub bitrate_bps: Option<u32>,
    pub route_validation: String,
    pub protocol: String,
    pub request_id: String,
    pub expected_response_id: String,
    pub capability: String,
    pub request_payload: String,
    pub actual_responder: Option<String>,
    pub raw_diagnostic_response: Option<String>,
    pub decoded_result: Option<String>,
    pub pending_responses: u32,
    pub timeout_or_error_category: Option<DiagnosticErrorCategory>,
    pub execution_stage: DiagnosticExecutionStage,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_states_remain_small_and_explicit() {
        let states = [
            AdapterState::NoAdapter,
            AdapterState::AdapterDetected,
            AdapterState::Connecting,
            AdapterState::Connected,
            AdapterState::Error,
        ];
        assert_eq!(states.len(), 5);
    }

    #[test]
    fn vehicle_validation_is_independent_from_implementation() {
        let capability = VehicleInterfaceCapability {
            id: "hs-can".to_owned(),
            name: "HS-CAN".to_owned(),
            pins: "6/14".to_owned(),
            nominal_bitrate: Some(500_000),
            hardware_confirmed: true,
            fixture_tested: true,
            implementation: InterfaceImplementationState::Available,
            vehicle_validation: VehicleValidationState::NotYetValidated,
        };

        assert_eq!(
            capability.implementation,
            InterfaceImplementationState::Available
        );
        assert_eq!(
            capability.vehicle_validation,
            VehicleValidationState::NotYetValidated
        );
    }
}
