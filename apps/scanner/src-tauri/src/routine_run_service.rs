//! Running a routine a module declares (`ADR-0036`, step 2): the on-demand
//! self test, `0x0202`, and no other routine in this slice.
//!
//! The shell's side of it, beside `dtc_clear_service`: the run is planned
//! from the same survey and the same resolved plan a read uses, and sent only
//! as prepared services - the start with the extended session opened and
//! held, a keep-alive on every step while the test runs, the request for
//! results once the data's time has run, and the way back to the default
//! session with whichever step ends the run. Each step is one request over
//! the adapter, taken and released, so a stop gets through between two. The
//! result is the bytes the module answered with; nothing is read into them.
//! Nothing here decides whether the run was a good idea; the person did, in
//! the confirmation, and the record says so.

use crate::module_read_service::{hex, map_live_error, preparation_error};
use app_contracts::{
    AdapterInfo, DiagnosticError, DiagnosticErrorCategory, DiagnosticExecutionStage, DtcSummary,
    ModuleReadReport, ModuleReadSnapshot, ModuleSurveyEntry, RoutineRunRequest, RoutineRunSnapshot,
    RoutineRunState,
};
use diagnostic_environment::{DiagnosticEnvironmentResolution, DiagnosticEnvironmentResolver};
use diagnostic_session::{validation_label, vehicle_context, KnowledgeLibrary};
use mongoose_jlr::{MongooseDiagnosticError, MongooseUdsServiceResult};
use serde_json::{json, Value};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uds_execution::{
    decode_service_response, prepare_service_transaction, DiagnosticTargetIdentity,
    PreparedUdsService, RoutineStep, ServiceUdsIntent, StageTwoRoutine, UdsServiceOutcome,
    READ_DTC_INFORMATION_CAPABILITY, SELF_TEST_ROUTINE,
};

/// How long one step is given: the start, a keep-alive, the stop. The
/// request for results waits what the data allows, never less than this.
pub const ROUTINE_STEP_TIMEOUT: Duration = Duration::from_secs(2);
/// A keep-alive at most this long after the last exchange: ISO 14229's
/// server keeps the extended session five seconds without one.
pub const ROUTINE_KEEP_ALIVE_EVERY: Duration = Duration::from_millis(1500);
/// The record's schema in the session bundle (`ADR-0036`, decision 5).
pub const ROUTINE_RUN_SCHEMA: &str = "prowlone.routine-run";
pub const ROUTINE_RUN_SCHEMA_VERSION: u16 = 1;
/// The operation's name in `SAFETY_BOUNDARIES.md` and in every record.
pub const ROUTINE_RUN_OPERATION: &str = "ROUTINE_RUN";
pub const ROUTINE_RUN_SAFETY_CLASS: &str = "SERVICE_ROUTINE";
/// The identifiers the ODST pack gives the self test: the digits of `0x0202`.
const SELF_TEST_IDS: &[&str] = &["202", "0202"];

/// The three prepared steps of one run, from one plan, and what the data
/// says of the test they run.
pub struct PreparedRoutineRun {
    pub start: PreparedUdsService,
    pub results: PreparedUdsService,
    pub stop: PreparedUdsService,
    pub route_validation: String,
    pub test_name: String,
    pub time_ms: u32,
    pub timeout_ms: u32,
}

impl PreparedRoutineRun {
    /// How long the request for results may wait: what the data allows
    /// beyond the test's own time, never less than a step.
    pub fn results_timeout(&self) -> Duration {
        Duration::from_millis(u64::from(self.timeout_ms.saturating_sub(self.time_ms)))
            .max(ROUTINE_STEP_TIMEOUT)
    }
}

/// What the next step should do, by the clock and the data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoutineDue {
    /// Hold the session: a keep-alive.
    KeepAlive,
    /// The data's time has run: ask for the result, and leave the session.
    RequestResults,
}

pub struct RoutineRunService {
    snapshot: RoutineRunSnapshot,
    prepared: Option<PreparedRoutineRun>,
    request: Option<RoutineRunRequest>,
    adapter: Option<AdapterInfo>,
    started: Option<Instant>,
    last_exchange: Option<Instant>,
    exchanges: Vec<(Vec<u8>, Vec<u8>)>,
    /// The fault-code read made before the run in this session, if any:
    /// what the test's codes are set against.
    before: Option<(ModuleReadSnapshot, ModuleReadReport)>,
    /// The record of a run that ended, until the session report takes it.
    finished: Option<Value>,
}

impl Default for RoutineRunService {
    fn default() -> Self {
        Self::new()
    }
}

impl RoutineRunService {
    pub fn new() -> Self {
        Self {
            snapshot: idle(),
            prepared: None,
            request: None,
            adapter: None,
            started: None,
            last_exchange: None,
            exchanges: Vec::new(),
            before: None,
            finished: None,
        }
    }

    pub fn snapshot(&self) -> RoutineRunSnapshot {
        let mut snapshot = self.snapshot.clone();
        if snapshot.state == RoutineRunState::Running {
            if let Some(started) = self.started {
                snapshot.elapsed_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
            }
        }
        snapshot
    }

    pub fn is_running(&self) -> bool {
        self.snapshot.state == RoutineRunState::Running
    }

    /// The run ran and ended: completed, stopped, or timed out. A refusal
    /// at the start ran nothing, and a failure may have run anything.
    pub fn has_ended(&self) -> bool {
        matches!(
            self.snapshot.state,
            RoutineRunState::Completed | RoutineRunState::Stopped | RoutineRunState::TimedOut
        )
    }

    /// A run has ended and its record has not been taken yet: the moment
    /// the module is read again, once.
    pub fn record_pending(&self) -> bool {
        self.finished.is_some()
    }

    pub fn request(&self) -> Option<&RoutineRunRequest> {
        self.request.as_ref()
    }

    pub fn prepared_start(&self) -> Option<&PreparedUdsService> {
        self.prepared.as_ref().map(|prepared| &prepared.start)
    }

    pub fn prepared_results(&self) -> Option<&PreparedUdsService> {
        self.prepared.as_ref().map(|prepared| &prepared.results)
    }

    pub fn prepared_stop(&self) -> Option<&PreparedUdsService> {
        self.prepared.as_ref().map(|prepared| &prepared.stop)
    }

    pub fn results_timeout(&self) -> Duration {
        self.prepared
            .as_ref()
            .map(PreparedRoutineRun::results_timeout)
            .unwrap_or(ROUTINE_STEP_TIMEOUT)
    }

    /// The three steps, from the plan the read uses, where the two records
    /// agree that this module runs this test (`ADR-0036`, step 2, decision
    /// 1): the module's index declares `0x0202` without a security level,
    /// and the ODST pack gives this car the test with its time and timeout.
    pub fn prepare(
        library: &KnowledgeLibrary,
        module: &ModuleSurveyEntry,
        request: &RoutineRunRequest,
    ) -> Result<PreparedRoutineRun, DiagnosticError> {
        if module.ecu_family != request.ecu_family {
            return Err(not_offered("the survey entry is another module's"));
        }
        if !SELF_TEST_IDS.contains(&request.test_id.as_str()) {
            return Err(not_offered(
                "only the on-demand self test, routine 0x0202, is run in this step",
            ));
        }
        let declared = module.accepted_operations.iter().any(|operation| {
            operation.kind == "ROUTINE"
                && parse_hex_identifier(&operation.identifier) == Some(SELF_TEST_ROUTINE)
                && operation.security.is_none()
        });
        if !declared {
            return Err(not_offered(
                "the module's index does not declare routine 0x0202 for this car, or names a security level for it",
            ));
        }
        let test = module
            .self_tests
            .iter()
            .find(|test| test.test_id == request.test_id)
            .ok_or_else(|| {
                not_offered(
                    "the ODST pack gives this car no screen for the self test of this module",
                )
            })?;
        let (Some(time_ms), Some(timeout_ms)) = (test.time_ms, test.timeout_ms) else {
            return Err(not_offered(
                "the data does not say how long the test runs, so the product does not run it",
            ));
        };

        let context = vehicle_context(&request.context);
        let resolution = DiagnosticEnvironmentResolver::resolve_ecu_family(
            library.store(),
            &context,
            &request.ecu_family,
            READ_DTC_INFORMATION_CAPABILITY,
        )
        .map_err(|error| preparation_error(error.to_string()))?;
        let route_validation = match &resolution {
            DiagnosticEnvironmentResolution::Resolved(plan) => {
                validation_label(plan.validation_state).to_string()
            }
            _ => String::new(),
        };
        let target = DiagnosticTargetIdentity::family(&request.ecu_family)
            .map_err(|error| preparation_error(error.to_string()))?;
        let step = |step: RoutineStep| {
            prepare_service_transaction(
                &resolution,
                ServiceUdsIntent::routine_control(target.clone(), StageTwoRoutine::SelfTest, step),
            )
            .map_err(|error| preparation_error(error.to_string()))
        };
        Ok(PreparedRoutineRun {
            start: step(RoutineStep::Start)?,
            results: step(RoutineStep::RequestResults)?,
            stop: step(RoutineStep::Stop)?,
            route_validation,
            test_name: test.name.clone(),
            time_ms,
            timeout_ms,
        })
    }

    /// Nothing was sent: the shell refused before the plan, or the plan
    /// could not be made. Said in the snapshot, recorded nowhere - an
    /// action that never started is not a record.
    pub fn refuse(
        &mut self,
        request: &RoutineRunRequest,
        error: DiagnosticError,
    ) -> RoutineRunSnapshot {
        let mut snapshot = idle();
        snapshot.ecu_family = request.ecu_family.clone();
        snapshot.test_id = request.test_id.clone();
        snapshot.state = RoutineRunState::Failed;
        snapshot.error = Some(error);
        self.snapshot = snapshot;
        self.prepared = None;
        self.request = None;
        self.started = None;
        self.before = None;
        self.exchanges.clear();
        self.snapshot()
    }

    /// The start was sent: what the module answered, and the run from here.
    pub fn start(
        &mut self,
        request: &RoutineRunRequest,
        prepared: PreparedRoutineRun,
        adapter: Option<&AdapterInfo>,
        before: Option<&(ModuleReadSnapshot, ModuleReadReport)>,
        synthetic: bool,
        result: Result<MongooseUdsServiceResult, MongooseDiagnosticError>,
    ) -> RoutineRunSnapshot {
        let mut snapshot = idle();
        snapshot.ecu_family = request.ecu_family.clone();
        snapshot.test_id = request.test_id.clone();
        snapshot.test_name = prepared.test_name.clone();
        snapshot.route_id = prepared.start.transaction().backend_route().to_string();
        snapshot.route_validation = if synthetic {
            "SYNTHETIC".into()
        } else {
            prepared.route_validation.clone()
        };
        snapshot.time_ms = prepared.time_ms;
        snapshot.timeout_ms = prepared.timeout_ms;
        snapshot.started_unix_ms = Some(unix_ms());
        snapshot.codes_before = before
            .map(|(read, _)| read.dtcs.clone())
            .unwrap_or_default();
        self.before = before.cloned();
        self.exchanges.clear();
        self.request = Some(request.clone());
        self.adapter = adapter.cloned();
        self.started = Some(Instant::now());
        self.last_exchange = self.started;

        match result {
            Ok(raw) => {
                snapshot.session = Some(format!("0x{:02X}", raw.session));
                self.exchanges.extend(raw.exchanges.iter().cloned());
                match decode_service_response(&prepared.start, &raw.raw_diagnostic_response) {
                    Ok(Some(UdsServiceOutcome::RoutineControlled { .. })) => {
                        snapshot.state = RoutineRunState::Running;
                    }
                    Ok(Some(UdsServiceOutcome::Negative(negative))) => {
                        snapshot.state = RoutineRunState::Refused;
                        snapshot.refusal = Some(format!("{:?}", negative.code));
                    }
                    Ok(Some(UdsServiceOutcome::Cleared { .. })) => {
                        fail(
                            &mut snapshot,
                            "a clear answered a routine start".to_string(),
                        );
                    }
                    Ok(None) => fail(
                        &mut snapshot,
                        "the final answer was still ResponsePending".to_string(),
                    ),
                    Err(error) => fail(&mut snapshot, error.to_string()),
                }
            }
            Err(error) => {
                snapshot.state = RoutineRunState::Failed;
                snapshot.error = Some(map_live_error(error));
            }
        }
        snapshot.exchanges = self.exchanges_hex();
        self.prepared = Some(prepared);
        self.snapshot = snapshot;
        if !self.is_running() {
            self.close();
        }
        self.snapshot()
    }

    /// What the next step is, by the clock: nothing while the run is not on
    /// or the last exchange is recent enough; a keep-alive while the test
    /// runs; the request for results once the data's time has run.
    pub fn next_due(&self) -> Option<RoutineDue> {
        if !self.is_running() {
            return None;
        }
        let started = self.started?;
        let time = Duration::from_millis(u64::from(self.snapshot.time_ms));
        if started.elapsed() >= time {
            return Some(RoutineDue::RequestResults);
        }
        let since = self
            .last_exchange
            .map(|at| at.elapsed())
            .unwrap_or(Duration::MAX);
        (since >= ROUTINE_KEEP_ALIVE_EVERY).then_some(RoutineDue::KeepAlive)
    }

    /// A keep-alive answered, or not: a session that cannot be held is a run
    /// that cannot go on.
    pub fn record_keep_alive(
        &mut self,
        result: Result<MongooseUdsServiceResult, DiagnosticError>,
    ) -> RoutineRunSnapshot {
        self.last_exchange = Some(Instant::now());
        match result {
            Ok(raw) => {
                self.exchanges.extend(raw.exchanges.iter().cloned());
                self.snapshot.exchanges = self.exchanges_hex();
            }
            Err(error) => {
                self.snapshot.state = RoutineRunState::Failed;
                self.snapshot.error = Some(error);
                self.close();
            }
        }
        self.snapshot()
    }

    /// The request for results answered: the run's result, as the module
    /// wrote it, and the session left behind it whatever it said.
    pub fn record_results(
        &mut self,
        result: Result<MongooseUdsServiceResult, MongooseDiagnosticError>,
    ) -> RoutineRunSnapshot {
        self.last_exchange = Some(Instant::now());
        match result {
            Ok(raw) => {
                self.exchanges.extend(raw.exchanges.iter().cloned());
                let decoded = self
                    .prepared_results()
                    .map(|results| decode_service_response(results, &raw.raw_diagnostic_response));
                match decoded {
                    Some(Ok(Some(UdsServiceOutcome::RoutineControlled { status, .. }))) => {
                        self.snapshot.state = RoutineRunState::Completed;
                        self.snapshot.result_hex = Some(hex(&status));
                    }
                    Some(Ok(Some(UdsServiceOutcome::Negative(negative)))) => {
                        self.snapshot.state = RoutineRunState::Refused;
                        self.snapshot.refusal = Some(format!("{:?}", negative.code));
                    }
                    Some(Ok(Some(UdsServiceOutcome::Cleared { .. }))) => fail(
                        &mut self.snapshot,
                        "a clear answered a request for results".to_string(),
                    ),
                    Some(Ok(None)) => fail(
                        &mut self.snapshot,
                        "the final answer was still ResponsePending".to_string(),
                    ),
                    Some(Err(error)) => fail(&mut self.snapshot, error.to_string()),
                    None => fail(
                        &mut self.snapshot,
                        "no prepared request for results".to_string(),
                    ),
                }
            }
            Err(MongooseDiagnosticError::Timeout) => {
                self.snapshot.state = RoutineRunState::TimedOut;
                self.snapshot.error = Some(map_live_error(MongooseDiagnosticError::Timeout));
            }
            Err(error) => {
                self.snapshot.state = RoutineRunState::Failed;
                self.snapshot.error = Some(map_live_error(error));
            }
        }
        self.close();
        self.snapshot()
    }

    /// The person stopped the run: the stop's answer is recorded, and the
    /// run is stopped whatever the module said to it.
    pub fn record_stop(
        &mut self,
        result: Result<MongooseUdsServiceResult, DiagnosticError>,
    ) -> RoutineRunSnapshot {
        self.last_exchange = Some(Instant::now());
        match result {
            Ok(raw) => {
                self.exchanges.extend(raw.exchanges.iter().cloned());
                if let Some(Ok(Some(UdsServiceOutcome::Negative(negative)))) = self
                    .prepared_stop()
                    .map(|stop| decode_service_response(stop, &raw.raw_diagnostic_response))
                {
                    self.snapshot.refusal = Some(format!("{:?}", negative.code));
                }
            }
            Err(error) => {
                self.snapshot.error = Some(error);
            }
        }
        self.snapshot.state = RoutineRunState::Stopped;
        self.close();
        self.snapshot()
    }

    /// The module was read again once the run had ended: what it holds
    /// now, kept beside what it held before, and the difference - what the
    /// test logged - said on its own. The record takes both reads.
    pub fn attach_after(
        &mut self,
        after: &(ModuleReadSnapshot, ModuleReadReport),
    ) -> RoutineRunSnapshot {
        let found: Vec<DtcSummary> = after
            .0
            .dtcs
            .iter()
            .filter(|dtc| {
                !self
                    .snapshot
                    .codes_before
                    .iter()
                    .any(|held| held.code == dtc.code && held.failure_type == dtc.failure_type)
            })
            .cloned()
            .collect();
        self.snapshot.codes_after = Some(after.0.dtcs.clone());
        self.snapshot.codes_found = found;
        if let Some(record) = self.finished.as_mut() {
            record["codes_after"] = json!(after.0.dtcs);
            record["after"] = json!(after.1);
            record["codes_found"] = json!(self.snapshot.codes_found);
        }
        self.snapshot()
    }

    /// The record of a run that ended, once; nothing while one is on.
    pub fn take_report_json(&mut self) -> Option<String> {
        let record = self.finished.take()?;
        serde_json::to_string_pretty(&record).ok()
    }

    fn exchanges_hex(&self) -> Vec<(String, String)> {
        self.exchanges
            .iter()
            .map(|(request, response)| (hex(request), hex(response)))
            .collect()
    }

    /// The run ended: the snapshot is frozen and the record written, with
    /// every exchange and what each answered (`ADR-0036`, step 2, decision 5).
    fn close(&mut self) {
        if let Some(started) = self.started {
            self.snapshot.elapsed_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
        }
        self.snapshot.exchanges = self.exchanges_hex();
        self.snapshot.report_available = true;
        let Some(request) = &self.request else {
            return;
        };
        let answers: Vec<String> = self
            .exchanges
            .iter()
            .map(|(_, response)| answer_name(response))
            .collect();
        self.finished = Some(json!({
            "schema": ROUTINE_RUN_SCHEMA,
            "schema_version": ROUTINE_RUN_SCHEMA_VERSION,
            "operation": ROUTINE_RUN_OPERATION,
            "safety_class": ROUTINE_RUN_SAFETY_CLASS,
            "application_version": env!("CARGO_PKG_VERSION"),
            "timestamp_unix_ms": unix_ms(),
            "vehicle": request.context,
            "ecu_family": request.ecu_family,
            "routine": self.snapshot.routine,
            "test_id": self.snapshot.test_id,
            "test_name": self.snapshot.test_name,
            "adapter_model": self.adapter.as_ref().map(|adapter| adapter.name.clone()),
            "route_id": self.snapshot.route_id,
            "route_validation": self.snapshot.route_validation,
            "session": self.snapshot.session,
            "time_ms": self.snapshot.time_ms,
            "timeout_ms": self.snapshot.timeout_ms,
            "started_unix_ms": self.snapshot.started_unix_ms,
            "elapsed_ms": self.snapshot.elapsed_ms,
            "state": self.snapshot.state,
            "result_hex": self.snapshot.result_hex,
            "refusal": self.snapshot.refusal,
            "error": self.snapshot.error,
            "codes_before": self.snapshot.codes_before,
            "before": self.before.as_ref().map(|(_, report)| report),
            "codes_after": Value::Null,
            "after": Value::Null,
            "codes_found": Vec::<DtcSummary>::new(),
            "exchanges": self.snapshot.exchanges,
            "answers": answers,
            "validation": "a service operation of stage 2; the person confirmed it, the module answered, the result is the bytes it answered with and nothing is read into them; nothing here is vehicle-confirmed by being here",
        }));
        self.started = None;
    }
}

/// `0x0202` as the survey writes an identifier, or any other text, as a number.
fn parse_hex_identifier(text: &str) -> Option<u16> {
    let digits = text
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    u16::from_str_radix(digits, 16).ok()
}

/// What an answer was, for the record: positive, or the refusal by its code.
fn answer_name(response: &[u8]) -> String {
    match response {
        [0x7F, service, code, ..] => format!("refused: service 0x{service:02X}, NRC 0x{code:02X}"),
        [] => "no answer".to_string(),
        _ => "positive".to_string(),
    }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn fail(snapshot: &mut RoutineRunSnapshot, details: String) {
    snapshot.state = RoutineRunState::Failed;
    snapshot.error = Some(DiagnosticError {
        category: DiagnosticErrorCategory::MalformedDiagnosticResponse,
        message: "Malformed diagnostic response".into(),
        technical_details: Some(details),
        stage: DiagnosticExecutionStage::DiagnosticDecode,
    });
}

/// The refusal the shell gives before anything is sent.
pub fn not_offered(details: &str) -> DiagnosticError {
    DiagnosticError {
        category: DiagnosticErrorCategory::UnsupportedVehicleProfile,
        message: "The routine is not offered".into(),
        technical_details: Some(details.into()),
        stage: DiagnosticExecutionStage::Preparation,
    }
}

fn idle() -> RoutineRunSnapshot {
    RoutineRunSnapshot {
        state: RoutineRunState::Idle,
        ecu_family: String::new(),
        routine: format!("0x{SELF_TEST_ROUTINE:04X}"),
        test_id: String::new(),
        test_name: String::new(),
        operation: ROUTINE_RUN_OPERATION.into(),
        safety_class: ROUTINE_RUN_SAFETY_CLASS.into(),
        route_id: String::new(),
        route_validation: String::new(),
        session: None,
        time_ms: 0,
        timeout_ms: 0,
        started_unix_ms: None,
        elapsed_ms: 0,
        result_hex: None,
        refusal: None,
        codes_before: Vec::new(),
        codes_after: None,
        codes_found: Vec::new(),
        exchanges: Vec::new(),
        error: None,
        report_available: false,
    }
}
