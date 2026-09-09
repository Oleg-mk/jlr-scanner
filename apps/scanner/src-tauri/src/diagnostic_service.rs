// The simulator and replay execution sources are exercised by the tests
// only; outside `cfg(test)` their imports and helpers are unused, and CI
// lints with `-D warnings`.
#![cfg_attr(not(test), allow(dead_code, unused_imports))]

use app_contracts::{
    AdapterInfo, DiagnosticError, DiagnosticErrorCategory, DiagnosticExecutionStage,
    DiagnosticReport, DiagnosticSnapshot, DiagnosticState, SupportedVehicleProfile,
};
use diagnostic_execution::{
    execute_replay, execute_simulator, prepare_read_only_transaction,
    CalibrationIdentificationExecutionResult, DiagnosticTargetIdentity,
    PreparedDiagnosticTransaction, ReadOnlyDiagnosticIntent,
};
use diagnostic_simulator::{PayloadSimulatorBehavior, PayloadSimulatorScenario, SimulatorSource};
use jlr_profiles::{
    x250_2010_supercharged_ecm_environment, DIAGNOSTIC_IMPLEMENTATION, ECU_FAMILY,
    OBSERVED_CALIBRATION_ID, X250_2010_SUPERCHARGED_ECM,
};
use mongoose_jlr::{MongooseCalibrationIdentificationResult, MongooseDiagnosticError};
use obd_j1979::{encode_calibration_identification_response, CalibrationId, J1979Request};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use transport_api::{CanFrameSource, CanId, CanSourceKind};
use transport_replay::{PlaybackMode, ReplaySource};

const OPERATION_NAME: &str = "Read Calibration Identification";
const REPLAY: &str =
    include_str!("../../../../fixtures/synthetic/f7_x250_mode09_calibration_multiframe.json");
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionSource {
    LiveMongoose,
    Simulator,
    Replay,
    /// The bench (ADR-0020): the live path over a stand-in adapter, so the
    /// answer is synthetic whatever it decodes to.
    Bench,
}

impl ExecutionSource {
    fn label(self) -> &'static str {
        match self {
            Self::LiveMongoose => "LIVE_MONGOOSE",
            Self::Simulator => "SIMULATOR",
            Self::Replay => "REPLAY",
            Self::Bench => "BENCH_SYNTHETIC",
        }
    }
}

#[derive(Clone, Debug)]
struct Observation {
    responder: u32,
    request_payload: Vec<u8>,
    raw_response: Vec<u8>,
    calibration_ids: Vec<String>,
}

impl From<MongooseCalibrationIdentificationResult> for Observation {
    fn from(value: MongooseCalibrationIdentificationResult) -> Self {
        Self {
            responder: value.responder,
            request_payload: value.request_payload,
            raw_response: value.raw_diagnostic_response,
            calibration_ids: value.calibration_ids,
        }
    }
}

impl From<CalibrationIdentificationExecutionResult> for Observation {
    fn from(value: CalibrationIdentificationExecutionResult) -> Self {
        Self {
            responder: value.responder.value(),
            request_payload: value.request_payload,
            raw_response: value.raw_diagnostic_response,
            calibration_ids: value.calibration_ids,
        }
    }
}

pub struct DiagnosticService {
    transaction: PreparedDiagnosticTransaction,
    state: DiagnosticState,
    calibration_id: Option<String>,
    error: Option<DiagnosticError>,
    last_report: Option<DiagnosticReport>,
}

impl DiagnosticService {
    pub fn new() -> Self {
        let target = DiagnosticTargetIdentity::new(ECU_FAMILY, DIAGNOSTIC_IMPLEMENTATION)
            .expect("built-in F8 target identity is valid");
        let intent = ReadOnlyDiagnosticIntent::calibration_identification(target);
        let transaction =
            prepare_read_only_transaction(&x250_2010_supercharged_ecm_environment(), intent)
                .expect("built-in F8 evidence-backed environment must prepare");
        Self {
            transaction,
            state: DiagnosticState::Unavailable,
            calibration_id: None,
            error: None,
            last_report: None,
        }
    }

    pub fn transaction(&self) -> PreparedDiagnosticTransaction {
        self.transaction.clone()
    }

    pub fn begin(&mut self) {
        self.state = DiagnosticState::Running;
        self.calibration_id = None;
        self.error = None;
    }

    pub fn snapshot(&self, adapter_ready: bool) -> DiagnosticSnapshot {
        let state = match self.state {
            DiagnosticState::Unavailable | DiagnosticState::Ready => {
                if adapter_ready {
                    DiagnosticState::Ready
                } else {
                    DiagnosticState::Unavailable
                }
            }
            other => other,
        };
        DiagnosticSnapshot {
            profile: profile_contract(),
            operation_name: OPERATION_NAME.into(),
            state,
            calibration_id: self.calibration_id.clone(),
            error: self.error.clone(),
            report_available: self.last_report.is_some(),
        }
    }

    pub fn finish_live(
        &mut self,
        adapter: &AdapterInfo,
        result: Result<MongooseCalibrationIdentificationResult, MongooseDiagnosticError>,
    ) -> DiagnosticSnapshot {
        self.finish(
            Some(adapter),
            ExecutionSource::LiveMongoose,
            result.map(Observation::from).map_err(map_mongoose_error),
        );
        self.snapshot(true)
    }

    /// The bench answered (ADR-0020): decoded like a live answer, recorded
    /// under a source that says synthetic.
    pub fn finish_bench(
        &mut self,
        adapter: &AdapterInfo,
        result: Result<MongooseCalibrationIdentificationResult, MongooseDiagnosticError>,
    ) -> DiagnosticSnapshot {
        self.finish(
            Some(adapter),
            ExecutionSource::Bench,
            result.map(Observation::from).map_err(map_mongoose_error),
        );
        self.snapshot(true)
    }

    pub fn finish_adapter_unavailable(&mut self) -> DiagnosticSnapshot {
        let error = DiagnosticError {
            category: DiagnosticErrorCategory::AdapterNotFound,
            message: "Adapter not found".into(),
            technical_details: None,
            stage: DiagnosticExecutionStage::AdapterValidation,
        };
        self.finish(None, ExecutionSource::LiveMongoose, Err(error));
        self.snapshot(false)
    }

    pub fn report_json(&self) -> Result<String, String> {
        let report = self
            .last_report
            .as_ref()
            .ok_or_else(|| "No diagnostic report is available yet".to_owned())?;
        serde_json::to_string_pretty(report).map_err(|error| error.to_string())
    }

    fn finish(
        &mut self,
        adapter: Option<&AdapterInfo>,
        source: ExecutionSource,
        result: Result<Observation, DiagnosticError>,
    ) {
        let (observation, error) = match result {
            Ok(observation) => (Some(observation), None),
            Err(error) => (None, Some(error)),
        };
        self.calibration_id = observation
            .as_ref()
            .and_then(|value| value.calibration_ids.first().cloned());
        self.state = if observation.is_some() {
            DiagnosticState::Succeeded
        } else {
            DiagnosticState::Failed
        };
        self.error = error.clone();
        self.last_report = Some(build_report(
            &self.transaction,
            adapter,
            source,
            observation.as_ref(),
            error.as_ref(),
        ));
    }

    #[cfg(test)]
    fn run_simulator(&mut self) -> DiagnosticSnapshot {
        self.begin();
        let response = encode_calibration_identification_response(&[CalibrationId::new(
            OBSERVED_CALIBRATION_ID,
        )
        .expect("golden Calibration ID")])
        .expect("golden response");
        let request = J1979Request::calibration_identification().encoded();
        let mut source = SimulatorSource::script_payload(
            PayloadSimulatorScenario {
                expected_request_payload: request.to_vec(),
                behavior: PayloadSimulatorBehavior::Response {
                    response_payload: response,
                },
            },
            &request,
            "hs-can",
            CanId::standard(0x7e8).expect("standard response"),
        )
        .expect("simulator fixture");
        debug_assert_eq!(source.source_kind(), CanSourceKind::Simulator);
        let result = execute_simulator(
            &self.transaction,
            &mut source,
            "synthetic-f8-application-simulator",
            10_000,
        )
        .map(Observation::from)
        .map_err(map_offline_error);
        self.finish(Some(&fixture_adapter()), ExecutionSource::Simulator, result);
        self.snapshot(true)
    }

    #[cfg(test)]
    fn run_replay(&mut self) -> DiagnosticSnapshot {
        self.begin();
        let mut source = ReplaySource::from_json(REPLAY, PlaybackMode::Deterministic)
            .expect("committed F8 replay fixture");
        let result = execute_replay(&self.transaction, &mut source, 10_000)
            .map(Observation::from)
            .map_err(map_offline_error);
        self.finish(Some(&fixture_adapter()), ExecutionSource::Replay, result);
        self.snapshot(true)
    }
}

pub fn profile_contract() -> SupportedVehicleProfile {
    SupportedVehicleProfile {
        make: X250_2010_SUPERCHARGED_ECM.make.into(),
        model: X250_2010_SUPERCHARGED_ECM.model.into(),
        vehicle_program: X250_2010_SUPERCHARGED_ECM.vehicle_program.into(),
        model_year: X250_2010_SUPERCHARGED_ECM.model_year,
        powertrain: X250_2010_SUPERCHARGED_ECM.powertrain.into(),
        module: X250_2010_SUPERCHARGED_ECM.module.into(),
        ecu_target: X250_2010_SUPERCHARGED_ECM.ecu_family.into(),
    }
}

fn build_report(
    transaction: &PreparedDiagnosticTransaction,
    adapter: Option<&AdapterInfo>,
    source: ExecutionSource,
    observation: Option<&Observation>,
    error: Option<&DiagnosticError>,
) -> DiagnosticReport {
    let timestamp_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);
    let counter = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let (os_name, os_version) = os_details();
    let request = transaction.protocol_request().encoded();
    DiagnosticReport {
        schema_version: 1,
        application_version: env!("CARGO_PKG_VERSION").into(),
        os_name,
        os_version,
        timestamp_unix_ms,
        session_id: format!("f8-{timestamp_unix_ms}-{counter}"),
        execution_source: source.label().into(),
        adapter_model: adapter
            .map(|value| value.name.clone())
            .unwrap_or_else(|| "MongoosePro JLR".into()),
        usb_vid: adapter
            .map(|value| format!("{:04X}", value.usb_vid))
            .unwrap_or_else(|| "18E1".into()),
        usb_pid: adapter
            .map(|value| format!("{:04X}", value.usb_pid))
            .unwrap_or_else(|| "0104".into()),
        adapter_board_info_result: adapter
            .map(|value| format!("OK / {}", value.board_info.response_command))
            .unwrap_or_else(|| "NOT_AVAILABLE".into()),
        selected_vehicle_profile: profile_contract(),
        ecu_target: transaction.target().ecu_family().into(),
        logical_bus: transaction.logical_network().into(),
        physical_route: format!(
            "{} pins {}",
            transaction.physical_connector(),
            transaction
                .physical_pins()
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join("/")
        ),
        bitrate_bps: transaction.bitrate_bps(),
        protocol: transaction.protocol_family().into(),
        request_id: format!("0x{:03X}", transaction.physical_request_id().value()),
        expected_response_id: format!("0x{:03X}", transaction.expected_response_id().value()),
        capability: transaction.capability_id().into(),
        request_payload: hex(
            observation.map_or(request.as_slice(), |value| value.request_payload.as_slice())
        ),
        actual_responder: observation.map(|value| format!("0x{:03X}", value.responder)),
        raw_diagnostic_response: observation.map(|value| hex(&value.raw_response)),
        decoded_result: observation
            .and_then(|value| value.calibration_ids.first())
            .map(|value| format!("Calibration ID: {value}")),
        timeout_or_error_category: error.map(|value| value.category),
        execution_stage: error
            .map(|value| value.stage)
            .unwrap_or(DiagnosticExecutionStage::Completed),
    }
}

pub(crate) fn map_mongoose_error(error: MongooseDiagnosticError) -> DiagnosticError {
    let (category, stage, message) = match &error {
        MongooseDiagnosticError::UnsupportedTransaction(_) => (
            DiagnosticErrorCategory::UnsupportedVehicleProfile,
            DiagnosticExecutionStage::Preparation,
            "Unsupported vehicle profile",
        ),
        MongooseDiagnosticError::CanConnection(_) => (
            DiagnosticErrorCategory::CanConnectionFailed,
            DiagnosticExecutionStage::CanConnection,
            "CAN connection failed",
        ),
        MongooseDiagnosticError::RequestTransmission(_) => (
            DiagnosticErrorCategory::AdapterCommunicationFailed,
            DiagnosticExecutionStage::RequestTransmission,
            "Adapter communication failed",
        ),
        MongooseDiagnosticError::ResponseReception(_) => (
            DiagnosticErrorCategory::AdapterCommunicationFailed,
            DiagnosticExecutionStage::ResponseReception,
            "Adapter communication failed",
        ),
        MongooseDiagnosticError::ChannelClose(_) => (
            DiagnosticErrorCategory::AdapterCommunicationFailed,
            DiagnosticExecutionStage::ResponseReception,
            "Adapter communication failed",
        ),
        MongooseDiagnosticError::IsoTp(_) => (
            DiagnosticErrorCategory::MalformedDiagnosticResponse,
            DiagnosticExecutionStage::IsoTpReassembly,
            "Malformed diagnostic response",
        ),
        MongooseDiagnosticError::J1979(_) => (
            DiagnosticErrorCategory::MalformedDiagnosticResponse,
            DiagnosticExecutionStage::DiagnosticDecode,
            "Malformed diagnostic response",
        ),
        MongooseDiagnosticError::UnexpectedResponder { .. } => (
            DiagnosticErrorCategory::UnexpectedResponder,
            DiagnosticExecutionStage::ResponseReception,
            "Unexpected responder",
        ),
        MongooseDiagnosticError::Timeout => (
            DiagnosticErrorCategory::NoResponseFromEcu,
            DiagnosticExecutionStage::ResponseReception,
            "No response from ECU",
        ),
    };
    DiagnosticError {
        category,
        message: message.into(),
        technical_details: Some(error.to_string()),
        stage,
    }
}

fn map_offline_error(error: diagnostic_execution::ExecutionError) -> DiagnosticError {
    let technical = error.to_string();
    let (category, stage, message) = match error {
        diagnostic_execution::ExecutionError::Timeout
        | diagnostic_execution::ExecutionError::TruncatedIsoTp => (
            DiagnosticErrorCategory::NoResponseFromEcu,
            DiagnosticExecutionStage::ResponseReception,
            "No response from ECU",
        ),
        diagnostic_execution::ExecutionError::UnexpectedResponder { .. } => (
            DiagnosticErrorCategory::UnexpectedResponder,
            DiagnosticExecutionStage::ResponseReception,
            "Unexpected responder",
        ),
        diagnostic_execution::ExecutionError::IsoTp(_) => (
            DiagnosticErrorCategory::MalformedDiagnosticResponse,
            DiagnosticExecutionStage::IsoTpReassembly,
            "Malformed diagnostic response",
        ),
        diagnostic_execution::ExecutionError::J1979(_) => (
            DiagnosticErrorCategory::MalformedDiagnosticResponse,
            DiagnosticExecutionStage::DiagnosticDecode,
            "Malformed diagnostic response",
        ),
        _ => (
            DiagnosticErrorCategory::InternalFailure,
            DiagnosticExecutionStage::Preparation,
            "Diagnostic execution failed",
        ),
    };
    DiagnosticError {
        category,
        message: message.into(),
        technical_details: Some(technical),
        stage,
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(windows)]
fn os_details() -> (String, String) {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    let current_version = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion");
    if let Ok(key) = current_version {
        let product: String = key
            .get_value("ProductName")
            .unwrap_or_else(|_| "Windows".into());
        let display: String = key.get_value("DisplayVersion").unwrap_or_default();
        let build: String = key.get_value("CurrentBuildNumber").unwrap_or_default();
        return (product, format!("{display} build {build}").trim().into());
    }
    ("Windows".into(), "version unavailable".into())
}

#[cfg(not(windows))]
fn os_details() -> (String, String) {
    (
        std::env::consts::OS.into(),
        format!("{} / version unavailable", std::env::consts::ARCH),
    )
}

#[cfg(test)]
fn fixture_adapter() -> AdapterInfo {
    use app_contracts::BoardInfoEvidence;
    AdapterInfo {
        name: "MongoosePro JLR".into(),
        connection_status: "Connected".into(),
        port: "FIXTURE".into(),
        usb_vid: 0x18e1,
        usb_pid: 0x0104,
        serial_number: None,
        transport: "fixture".into(),
        driver: None,
        backend: "mongoose-jlr".into(),
        board_info: BoardInfoEvidence {
            response_command: "0x8109".into(),
            raw_response_hex: "fixture".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulator_and_replay_use_real_application_workflow_and_report_exact_result() {
        for run in [
            DiagnosticService::run_simulator as fn(&mut DiagnosticService) -> DiagnosticSnapshot,
            DiagnosticService::run_replay,
        ] {
            let mut service = DiagnosticService::new();
            let snapshot = run(&mut service);
            assert_eq!(snapshot.state, DiagnosticState::Succeeded);
            assert_eq!(
                snapshot.calibration_id.as_deref(),
                Some(OBSERVED_CALIBRATION_ID)
            );
            let report: DiagnosticReport = serde_json::from_str(&service.report_json().unwrap())
                .expect("machine-readable report");
            assert_eq!(report.request_payload, "09 04");
            assert_eq!(report.actual_responder.as_deref(), Some("0x7E8"));
            assert_eq!(
                report.decoded_result.as_deref(),
                Some("Calibration ID: CX23-14C204-ZAD")
            );
            assert_eq!(report.execution_stage, DiagnosticExecutionStage::Completed);
        }
    }

    #[test]
    fn application_service_exposes_only_fixed_profile_and_typed_transaction() {
        let service = DiagnosticService::new();
        let transaction = service.transaction();
        assert_eq!(transaction.backend_route(), "hs-can");
        assert_eq!(transaction.physical_pins(), &[6, 14]);
        assert_eq!(transaction.bitrate_bps(), 500_000);
        assert_eq!(transaction.protocol_request().encoded(), [0x09, 0x04]);
        assert_eq!(transaction.physical_request_id().value(), 0x7e0);
        assert_eq!(transaction.expected_response_id().value(), 0x7e8);
    }
}
