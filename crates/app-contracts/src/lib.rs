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
    /// SDD's own help for this code on this car: possible causes, actions
    /// required, monitoring conditions, line by line as the screen shows it.
    #[serde(default)]
    pub help: Vec<String>,
    /// Why there is no help, when the reason is worth saying.
    #[serde(default)]
    pub help_note: Option<String>,
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
    /// Legislated OBD-II reads (ADR-0022, decision 7).
    #[serde(default)]
    pub standard_obd_reads: u32,
    /// Live read runs recorded whole, with their series (ADR-0022).
    #[serde(default)]
    pub live_read_runs: u32,
    /// Mileage surveys recorded whole (ADR-0024).
    #[serde(default)]
    pub mileage_surveys: u32,
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

// ---------------------------------------------------------------------------
// The legislated OBD-II services (ADR-0022, decision 7)
// ---------------------------------------------------------------------------

/// Which legislated service to ask for. Two of the standard's services are
/// absent by design: clearing codes (04) changes the vehicle, and control of
/// on-board systems (08) is not a read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StandardObdReadKind {
    /// Mode 01: the PIDs given, or the support map when none is given.
    CurrentData,
    /// Mode 02: one PID of freeze frame 0.
    FreezeFrame,
    /// Mode 03.
    StoredDtcs,
    /// Mode 07.
    PendingDtcs,
    /// Mode 0A.
    PermanentDtcs,
    /// Mode 06: the monitors given, or the support map when none is given.
    MonitorResults,
    /// Mode 09: one InfoType.
    VehicleInformation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandardObdRequest {
    pub kind: StandardObdReadKind,
    /// Which of the standard's eight responders, 0–7; 0 is the engine
    /// controller on nearly every car.
    pub responder: u8,
    /// Hexadecimal bytes such as `0x0C`: PIDs for current data and the freeze
    /// frame, MIDs for monitors, the InfoType for vehicle information. Empty
    /// asks for the support map where the service has one.
    pub items: Vec<String>,
    /// The car the tester described, for the report; the request itself does
    /// not depend on it.
    pub context: VehicleContextInput,
}

/// One value of a legislated answer, as the standard decodes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandardObdValue {
    pub pid: String,
    pub name: String,
    pub unit: String,
    /// The value as text; `None` for a PID whose layout the codec does not
    /// carry, whose bytes are in `raw_hex`.
    pub value: Option<String>,
    /// The same value as a number, for a gauge or a trend, when it is one.
    pub number: Option<f64>,
    pub raw_hex: String,
    /// `number`, `text`, `flag`, `supported` or `raw`.
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandardObdMonitor {
    pub mid: String,
    pub monitor: String,
    pub tid: String,
    pub uas: String,
    pub unit: String,
    pub value: Option<f64>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub raw_value: u16,
    pub raw_minimum: u16,
    pub raw_maximum: u16,
    pub passed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelledValue {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandardObdSnapshot {
    pub state: ModuleReadState,
    pub kind: Option<StandardObdReadKind>,
    /// `0x7E0 → 0x7E8`: the responder asked and the one that answers.
    pub responder: String,
    pub operation: String,
    /// `SOURCE_BACKED` for a live answer — the standard is the source — or
    /// `SYNTHETIC` on the bench.
    pub route_validation: String,
    pub request_hex: String,
    pub raw_response_hex: Option<String>,
    /// From a support map: the PIDs, MIDs or InfoTypes the module answers.
    pub supported: Vec<String>,
    pub values: Vec<StandardObdValue>,
    /// `stored`, `pending` or `permanent` when `dtcs` is a list.
    pub dtc_kind: Option<String>,
    pub dtcs: Vec<DtcSummary>,
    pub freeze_frame: Option<u8>,
    pub monitors: Vec<StandardObdMonitor>,
    pub information: Vec<LabelledValue>,
    /// The module's refusal, code and meaning; an answer, not a failure.
    pub negative_response: Option<String>,
    pub pending_responses: u32,
    pub error: Option<DiagnosticError>,
    pub report_available: bool,
}

// ---------------------------------------------------------------------------
// Live reading (ADR-0022): the same read-only reads, repeated at a stated
// cadence. One entry is one module and one identifier the library lists for
// it; the interface never supplies bytes, services or addresses.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveReadEntryRequest {
    pub ecu_family: String,
    /// Hexadecimal identifier such as `0x1945`, chosen from the survey.
    pub identifier: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveReadRequest {
    pub entries: Vec<LiveReadEntryRequest>,
    pub context: VehicleContextInput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LiveReadState {
    Idle,
    Running,
    Stopped,
}

/// One entry of the running set, and what has become of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveReadEntryStatus {
    pub ecu_family: String,
    pub identifier: String,
    pub route_id: String,
    pub route_validation: String,
    pub reads: u32,
    /// Failures in a row; three drops the entry (ADR-0022, decision 4).
    pub failures: u32,
    pub dropped: bool,
    /// Why it was dropped, or why it never entered the set.
    pub reason: Option<String>,
    pub last_response_hex: Option<String>,
    pub negative_response: Option<String>,
}

/// One parameter as the run has seen it: the latest value the catalogue
/// decodes, and the smallest and largest of the run (ADR-0022, decision 6).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveReadValue {
    pub ecu_family: String,
    pub identifier: String,
    pub name: String,
    pub value: Option<String>,
    pub unit: Option<String>,
    /// SDD's name for the raw-count range, when the catalogue names it.
    pub state: Option<String>,
    pub note: Option<String>,
    pub raw: Option<u64>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub samples: u32,
    /// Milliseconds since the run began, at the last sample.
    pub at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveReadSnapshot {
    pub state: LiveReadState,
    pub entries: Vec<LiveReadEntryStatus>,
    pub values: Vec<LiveReadValue>,
    /// Complete passes over the set.
    pub rounds: u32,
    pub samples: u32,
    pub elapsed_ms: u64,
    /// The round time the run actually achieves, shown beside the values so a
    /// parameter that refreshes every two seconds is shown to (decision 3).
    pub round_ms: Option<u64>,
    pub cadence_floor_ms: u64,
    pub time_cap_ms: u64,
    /// `SOURCE_BACKED` or the route's own state; `SYNTHETIC` on the bench.
    pub route_validation: String,
    pub stopped_reason: Option<String>,
    pub error: Option<DiagnosticError>,
    pub report_available: bool,
}

// ---------------------------------------------------------------------------
// The odometer read from every module (ADR-0024). A car keeps its mileage in
// dozens of places; this asks each of them once and puts the answers side by
// side with the arithmetic done. Nothing is written, and no verdict is drawn.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MileageKind {
    /// The module's own running total.
    Current,
    /// The odometer as it stood when the module recorded something.
    Event,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MileageSurveyState {
    Idle,
    Running,
    Finished,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MileageSurveyRequest {
    pub context: VehicleContextInput,
}

/// One module's answer, or its silence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MileageReading {
    pub ecu_family: String,
    pub identifier: String,
    /// SDD's own name for the parameter.
    pub parameter: String,
    pub kind: MileageKind,
    pub state: ModuleReadState,
    pub value: Option<String>,
    pub unit: Option<String>,
    pub number: Option<f64>,
    pub raw: Option<u64>,
    /// How far this reading is from the highest running total found on the
    /// car. Arithmetic over two numbers this product read itself — never a
    /// verdict about anyone.
    pub difference: Option<f64>,
    pub route_id: String,
    pub route_validation: String,
    pub raw_response_hex: Option<String>,
    /// The module's refusal: an answer, not a failure of the application.
    pub negative_response: Option<String>,
    /// What the decoder says about a value it could not scale.
    pub note: Option<String>,
    /// Why the module said nothing.
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MileageSurveySnapshot {
    pub state: MileageSurveyState,
    pub readings: Vec<MileageReading>,
    /// Reads planned, attempted and answered.
    pub planned: u32,
    pub asked: u32,
    pub answered: u32,
    /// The highest running total found, the module that holds it, and the
    /// unit the catalogue gives it.
    pub highest: Option<f64>,
    pub highest_module: Option<String>,
    pub unit: Option<String>,
    /// `SOURCE_BACKED` or the route's own state; `SYNTHETIC` on the bench.
    pub route_validation: String,
    pub error: Option<DiagnosticError>,
    pub report_available: bool,
}
