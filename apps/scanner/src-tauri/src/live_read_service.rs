//! Live reading (ADR-0022): the same read-only reads, repeated at a stated
//! cadence, until stopped.
//!
//! The set is chosen from the library — a module and one identifier the
//! catalogue lists for it — and every entry passes the same `prepare` a
//! single read passes, so nothing here can supply bytes, service numbers or
//! addresses. This service owns the rules: the order, the 100 ms floor, the
//! ten-minute cap, the three-failures-and-out, and the samples. It owns no
//! timer: the interface asks for one step, and a step that arrives too soon
//! is refused, so no interface can make the product ask a bus faster than
//! the floor allows.
//!
//! One request is in flight at a time and the adapter is taken for that one
//! request, never for the run — a stop, a disconnect or any other command
//! gets through between two requests. The route is opened and closed with
//! each request, as every other read does, because the device refuses a
//! second open while one is open and refuses to close the transport with a
//! route open; see the ADR's "Built" section.

use crate::module_read_service::{ModuleReadService, PreparedModuleRead};
use crate::read_record::{read_record, ReadIdentity, ReadOutcome};
use app_contracts::{
    AdapterInfo, DecodedParameterSummary, DiagnosticError, DiagnosticErrorCategory,
    DiagnosticExecutionStage, LiveReadEntryRequest, LiveReadEntryStatus, LiveReadRequest,
    LiveReadSnapshot, LiveReadState, LiveReadValue, ModuleReadKind, ModuleReadReport,
    ModuleReadRequest,
};
use diagnostic_session::decode::decode_parameters;
use diagnostic_session::KnowledgeLibrary;
use mongoose_jlr::MongooseUdsReadResult;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uds_execution::{decode_response, PreparedUdsTransaction, UdsReadOutcome};

/// The shortest gap between two consecutive requests (ADR-0022, decision 3).
pub const LIVE_READ_FLOOR: Duration = Duration::from_millis(100);
/// The longest a run may go without being started again.
pub const LIVE_READ_CAP: Duration = Duration::from_secs(600);
/// How long one request waits for its answer.
pub const LIVE_READ_TIMEOUT: Duration = Duration::from_secs(2);
/// At most sixteen entries in a set (decision 2).
pub const LIVE_READ_MAX_ENTRIES: usize = 16;
/// Failures in a row before an entry is dropped from the set (decision 4).
const FAILURES_BEFORE_DROP: u32 = 3;
const EVERY_ENTRY_DROPPED: &str = "every entry of the set was dropped";

pub const LIVE_READ_SCHEMA: &str = "prowlone.live-read-run";
pub const LIVE_READ_OPERATION: &str = "LIVE_READ";
pub const LIVE_READ_SAFETY_CLASS: &str = "READ_ONLY";

static RUN_COUNTER: AtomicU64 = AtomicU64::new(1);

/// The entry whose turn it is, with the transaction prepared when the run
/// started. Handed to the caller so the adapter can be locked alone.
pub struct LiveReadDue {
    pub index: usize,
    pub transaction: PreparedUdsTransaction,
}

struct Entry {
    ecu_family: String,
    identifier: String,
    operation: String,
    route_id: String,
    route_validation: String,
    transaction: PreparedUdsTransaction,
    reads: u32,
    failures: u32,
    dropped: bool,
    reason: Option<String>,
    last_response_hex: Option<String>,
    negative_response: Option<String>,
    /// The record of the last completed request, answered or not, in the
    /// shape a single read leaves — what the intake turns into evidence. One
    /// per entry: the module answering at that address on that route is
    /// proven once, and repeating it adds nothing.
    record: Option<ModuleReadReport>,
}

/// A refused entry: it never joined the set, and the run says why.
struct Refused {
    ecu_family: String,
    identifier: String,
    reason: String,
}

struct Sample {
    at_ms: u64,
    ecu_family: String,
    identifier: String,
    response_hex: Option<String>,
    parameters: Vec<DecodedParameterSummary>,
    negative_response: Option<String>,
    failure: Option<String>,
}

struct Run {
    id: u64,
    state: LiveReadState,
    started: Instant,
    started_unix_ms: u128,
    entries: Vec<Entry>,
    refused: Vec<Refused>,
    values: Vec<LiveReadValue>,
    samples: Vec<Sample>,
    cursor: usize,
    rounds: u32,
    round_started: Option<Instant>,
    round_ms: Option<u64>,
    last_request_at: Option<Instant>,
    stopped_reason: Option<String>,
    synthetic: bool,
    adapter: Option<AdapterInfo>,
    context: app_contracts::VehicleContextInput,
    error: Option<DiagnosticError>,
}

#[derive(Default)]
pub struct LiveReadService {
    run: Option<Run>,
    last_report: Option<Value>,
    /// Why a run could not begin at all, when there is no run to carry it.
    refusal: Option<DiagnosticError>,
}

impl LiveReadService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> LiveReadSnapshot {
        match &self.run {
            Some(run) => run.snapshot(self.last_report.is_some()),
            None => LiveReadSnapshot {
                error: self.refusal.clone(),
                ..idle(self.last_report.is_some())
            },
        }
    }

    /// No run begins: the adapter is not there, or the session is not one a
    /// live read may join.
    pub fn refuse(&mut self, error: DiagnosticError) -> LiveReadSnapshot {
        self.run = None;
        self.refusal = Some(error);
        self.snapshot()
    }

    pub fn is_running(&self) -> bool {
        matches!(
            self.run.as_ref().map(|run| run.state),
            Some(LiveReadState::Running)
        )
    }

    /// Prepare every entry from the loaded library and start a run. An entry
    /// the library cannot plan does not join the set, with the resolver's own
    /// reason; a set that ends up empty is a failure to start.
    pub fn start(
        &mut self,
        library: &KnowledgeLibrary,
        adapter: Option<&AdapterInfo>,
        request: &LiveReadRequest,
    ) -> LiveReadSnapshot {
        self.refusal = None;
        let mut entries = Vec::new();
        let mut refused = Vec::new();
        for entry in request.entries.iter().take(LIVE_READ_MAX_ENTRIES) {
            match prepare_entry(library, &request.context, entry) {
                Ok(prepared) => entries.push(prepared),
                Err(reason) => refused.push(Refused {
                    ecu_family: entry.ecu_family.clone(),
                    identifier: entry.identifier.clone(),
                    reason,
                }),
            }
        }
        if request.entries.len() > LIVE_READ_MAX_ENTRIES {
            for entry in request.entries.iter().skip(LIVE_READ_MAX_ENTRIES) {
                refused.push(Refused {
                    ecu_family: entry.ecu_family.clone(),
                    identifier: entry.identifier.clone(),
                    reason: format!("a live set holds at most {LIVE_READ_MAX_ENTRIES} entries"),
                });
            }
        }

        let mut run = Run {
            id: RUN_COUNTER.fetch_add(1, Ordering::Relaxed),
            state: LiveReadState::Running,
            started: Instant::now(),
            started_unix_ms: unix_ms(),
            entries,
            refused,
            values: Vec::new(),
            samples: Vec::new(),
            cursor: 0,
            rounds: 0,
            round_started: None,
            round_ms: None,
            last_request_at: None,
            stopped_reason: None,
            synthetic: false,
            adapter: adapter.cloned(),
            context: request.context.clone(),
            error: None,
        };
        if run.entries.is_empty() {
            run.state = LiveReadState::Stopped;
            run.stopped_reason =
                Some("no entry of the set can be read with the loaded data".into());
            run.error = Some(DiagnosticError {
                category: DiagnosticErrorCategory::UnsupportedVehicleProfile,
                message: "The live set cannot be read with the loaded data".into(),
                technical_details: run
                    .refused
                    .first()
                    .map(|refused| refused.reason.clone())
                    .or(Some("the set was empty".into())),
                stage: DiagnosticExecutionStage::Preparation,
            });
        } else {
            run.round_started = Some(Instant::now());
        }
        let running = matches!(run.state, LiveReadState::Running);
        self.run = Some(run);
        if !running {
            // Nothing was read, so there is nothing to record.
            self.last_report = None;
        }
        self.snapshot()
    }

    /// The entry whose turn it is, when the floor allows a request now, or
    /// `None` — too soon, not running, or the run has just stopped itself at
    /// the cap or with an empty set.
    pub fn next_due(&mut self) -> Option<LiveReadDue> {
        enum Next {
            Wait,
            Stop(&'static str),
            Due(usize),
        }
        let next = {
            let run = self.run.as_mut()?;
            if !matches!(run.state, LiveReadState::Running) {
                Next::Wait
            } else if run.started.elapsed() >= LIVE_READ_CAP {
                Next::Stop("the run reached its ten-minute cap")
            } else if run
                .last_request_at
                .is_some_and(|last| last.elapsed() < LIVE_READ_FLOOR)
            {
                // Sooner than the floor allows: refused, not queued.
                Next::Wait
            } else {
                match run.next_index() {
                    Some(index) => {
                        run.last_request_at = Some(Instant::now());
                        Next::Due(index)
                    }
                    None => Next::Stop(EVERY_ENTRY_DROPPED),
                }
            }
        };
        match next {
            Next::Due(index) => {
                let transaction = self.run.as_ref()?.entries[index].transaction.clone();
                Some(LiveReadDue { index, transaction })
            }
            Next::Stop(reason) => {
                self.stop(reason);
                None
            }
            Next::Wait => None,
        }
    }

    /// Record what came back for one entry: a sample, or a failure counted
    /// against that entry.
    pub fn record(
        &mut self,
        index: usize,
        result: Result<MongooseUdsReadResult, DiagnosticError>,
    ) -> LiveReadSnapshot {
        let mut all_dropped = false;
        if let Some(run) = self.run.as_mut() {
            let at_ms = elapsed_ms(run.started);
            let context = run.context.clone();
            let adapter = run.adapter.clone();
            let synthetic = run.synthetic;
            let mut recorded: Option<(Sample, Option<Vec<DecodedParameterSummary>>)> = None;
            if let Some(entry) = run.entries.get_mut(index) {
                // The record first, from what was sent and what came back,
                // before the result is taken apart below.
                let identity = ReadIdentity {
                    context: &context,
                    ecu_family: &entry.ecu_family,
                    operation: &entry.operation,
                    route_validation: if synthetic {
                        "SYNTHETIC"
                    } else {
                        entry.route_validation.as_str()
                    },
                    adapter: adapter.as_ref(),
                };
                let outcome = match &result {
                    Ok(raw) => ReadOutcome::Answered(raw),
                    Err(error) => ReadOutcome::Failed(error),
                };
                entry.record = Some(read_record(identity, &entry.transaction, outcome, None));
                let mut sample = Sample {
                    at_ms,
                    ecu_family: entry.ecu_family.clone(),
                    identifier: entry.identifier.clone(),
                    response_hex: None,
                    parameters: Vec::new(),
                    negative_response: None,
                    failure: None,
                };
                let mut decoded: Option<Vec<DecodedParameterSummary>> = None;
                match result {
                    Ok(raw) => {
                        let response_hex = hex(&raw.raw_diagnostic_response);
                        entry.last_response_hex = Some(response_hex.clone());
                        sample.response_hex = Some(response_hex);
                        match decode_response(&entry.transaction, &raw.raw_diagnostic_response) {
                            Ok(Some(UdsReadOutcome::DataByIdentifier { data, .. })) => {
                                let parameters = entry
                                    .transaction
                                    .readable_identifier()
                                    .map(|identifier| identifier.parameters.clone())
                                    .unwrap_or_default();
                                let values: Vec<DecodedParameterSummary> =
                                    decode_parameters(&parameters, &data)
                                        .into_iter()
                                        .map(|value| DecodedParameterSummary {
                                            name: value.name,
                                            raw: value.raw,
                                            value: value.value,
                                            unit: value.unit,
                                            state: value.state,
                                            note: value.note,
                                        })
                                        .collect();
                                entry.reads += 1;
                                entry.failures = 0;
                                entry.negative_response = None;
                                entry.reason = None;
                                sample.parameters = values.clone();
                                decoded = Some(values);
                            }
                            // A refusal is an answer, and three in a row still
                            // drop the entry: it is not going to start working.
                            Ok(Some(UdsReadOutcome::Negative(negative))) => {
                                let text = format!("{:?}", negative.code);
                                entry.negative_response = Some(text.clone());
                                sample.negative_response = Some(text.clone());
                                entry.fail(format!("the module declined: {text}"));
                            }
                            Ok(Some(UdsReadOutcome::DtcReport(_))) => {
                                entry
                                    .fail("a fault-code report answered an identifier read".into());
                                sample.failure = entry.reason.clone();
                            }
                            Ok(None) => {
                                entry.fail("the final answer was still ResponsePending".into());
                                sample.failure = entry.reason.clone();
                            }
                            Err(error) => {
                                entry.fail(error.to_string());
                                sample.failure = entry.reason.clone();
                            }
                        }
                    }
                    Err(error) => {
                        let text = error
                            .technical_details
                            .clone()
                            .unwrap_or_else(|| error.message.clone());
                        entry.fail(text.clone());
                        sample.failure = Some(text);
                    }
                }
                recorded = Some((sample, decoded));
            }
            if let Some((sample, decoded)) = recorded {
                if let Some(values) = decoded {
                    let (family, identifier) =
                        (sample.ecu_family.clone(), sample.identifier.clone());
                    run.remember(&family, &identifier, at_ms, values);
                }
                run.samples.push(sample);
            }
            all_dropped = run.entries.iter().all(|entry| entry.dropped);
        }
        if all_dropped {
            self.stop(EVERY_ENTRY_DROPPED);
        }
        self.snapshot()
    }

    /// Stop the run and keep it as one report. Stopping twice is not an
    /// error: the first reason stands.
    pub fn stop(&mut self, reason: &str) -> LiveReadSnapshot {
        let mut record = false;
        if let Some(run) = self.run.as_mut() {
            if matches!(run.state, LiveReadState::Running) {
                run.state = LiveReadState::Stopped;
                run.stopped_reason = Some(reason.to_string());
                record = !run.samples.is_empty();
            }
        }
        if record {
            let report = self.run.as_ref().map(build_report);
            self.last_report = report;
        }
        self.snapshot()
    }

    /// The bench answered this run (ADR-0020): every sample is `SYNTHETIC`,
    /// in the snapshot and in the report.
    pub fn mark_synthetic(&mut self) -> LiveReadSnapshot {
        if let Some(run) = self.run.as_mut() {
            run.synthetic = true;
        }
        if let Some(report) = self.last_report.as_mut() {
            report["route_validation"] = json!("SYNTHETIC");
        }
        self.snapshot()
    }

    /// The last stopped run as JSON, for the session bundle.
    pub fn report_json(&self) -> Result<String, String> {
        let report = self
            .last_report
            .as_ref()
            .ok_or_else(|| "No live read run has been recorded yet".to_owned())?;
        serde_json::to_string_pretty(report).map_err(|error| error.to_string())
    }

    /// The report is handed to the session bundle once; this forgets it so a
    /// second stop cannot record the same run twice.
    pub fn take_report_json(&mut self) -> Option<String> {
        let json = self.report_json().ok();
        self.last_report = None;
        json
    }
}

impl Entry {
    fn fail(&mut self, reason: String) {
        self.failures += 1;
        self.reason = Some(reason);
        if self.failures >= FAILURES_BEFORE_DROP {
            self.dropped = true;
        }
    }

    fn status(&self, synthetic: bool) -> LiveReadEntryStatus {
        LiveReadEntryStatus {
            ecu_family: self.ecu_family.clone(),
            identifier: self.identifier.clone(),
            route_id: self.route_id.clone(),
            route_validation: if synthetic {
                "SYNTHETIC".into()
            } else {
                self.route_validation.clone()
            },
            reads: self.reads,
            failures: self.failures,
            dropped: self.dropped,
            reason: self.reason.clone(),
            last_response_hex: self.last_response_hex.clone(),
            negative_response: self.negative_response.clone(),
        }
    }
}

impl Run {
    /// The next entry to read, advancing the cursor and counting a round each
    /// time it wraps. `None` when every entry has been dropped.
    fn next_index(&mut self) -> Option<usize> {
        if self.entries.iter().all(|entry| entry.dropped) {
            return None;
        }
        for _ in 0..self.entries.len() {
            let index = self.cursor;
            self.cursor += 1;
            if self.cursor >= self.entries.len() {
                self.cursor = 0;
                self.rounds += 1;
                let now = Instant::now();
                if let Some(started) = self.round_started {
                    self.round_ms = Some(elapsed_ms(started));
                }
                self.round_started = Some(now);
            }
            if !self.entries[index].dropped {
                return Some(index);
            }
        }
        None
    }

    /// The latest value of each parameter, with the smallest and largest the
    /// run has seen. A value the catalogue does not scale keeps its raw
    /// count, and the note that says why.
    fn remember(
        &mut self,
        ecu_family: &str,
        identifier: &str,
        at_ms: u64,
        values: Vec<DecodedParameterSummary>,
    ) {
        for value in values {
            let number = value
                .value
                .as_deref()
                .and_then(|text| text.parse::<f64>().ok())
                .or_else(|| value.raw.map(|raw| raw as f64));
            match self.values.iter_mut().find(|kept| {
                kept.ecu_family == ecu_family
                    && kept.identifier == identifier
                    && kept.name == value.name
            }) {
                Some(kept) => {
                    kept.value = value.value;
                    kept.unit = value.unit;
                    kept.state = value.state;
                    kept.note = value.note;
                    kept.raw = value.raw;
                    kept.samples += 1;
                    kept.at_ms = at_ms;
                    if let Some(number) = number {
                        kept.minimum = Some(kept.minimum.map_or(number, |low| low.min(number)));
                        kept.maximum = Some(kept.maximum.map_or(number, |high| high.max(number)));
                    }
                }
                None => self.values.push(LiveReadValue {
                    ecu_family: ecu_family.to_string(),
                    identifier: identifier.to_string(),
                    name: value.name,
                    value: value.value,
                    unit: value.unit,
                    state: value.state,
                    note: value.note,
                    raw: value.raw,
                    minimum: number,
                    maximum: number,
                    samples: 1,
                    at_ms,
                }),
            }
        }
    }

    fn snapshot(&self, report_available: bool) -> LiveReadSnapshot {
        let mut entries: Vec<LiveReadEntryStatus> = self
            .entries
            .iter()
            .map(|entry| entry.status(self.synthetic))
            .collect();
        entries.extend(self.refused.iter().map(|refused| LiveReadEntryStatus {
            ecu_family: refused.ecu_family.clone(),
            identifier: refused.identifier.clone(),
            route_id: String::new(),
            route_validation: String::new(),
            reads: 0,
            failures: 0,
            dropped: true,
            reason: Some(refused.reason.clone()),
            last_response_hex: None,
            negative_response: None,
        }));
        LiveReadSnapshot {
            state: self.state,
            entries,
            values: self.values.clone(),
            rounds: self.rounds,
            samples: self.samples.len() as u32,
            elapsed_ms: elapsed_ms(self.started),
            round_ms: self.round_ms,
            cadence_floor_ms: LIVE_READ_FLOOR.as_millis() as u64,
            time_cap_ms: LIVE_READ_CAP.as_millis() as u64,
            route_validation: if self.synthetic {
                "SYNTHETIC".into()
            } else {
                self.entries
                    .first()
                    .map(|entry| entry.route_validation.clone())
                    .unwrap_or_default()
            },
            stopped_reason: self.stopped_reason.clone(),
            error: self.error.clone(),
            report_available,
        }
    }
}

/// One entry, planned by the same resolver a single read uses. Only an
/// identifier read: a live loop of fault-code reads is not what this is for.
fn prepare_entry(
    library: &KnowledgeLibrary,
    context: &app_contracts::VehicleContextInput,
    entry: &LiveReadEntryRequest,
) -> Result<Entry, String> {
    let request = ModuleReadRequest {
        ecu_family: entry.ecu_family.clone(),
        kind: ModuleReadKind::Identifier,
        identifier: Some(entry.identifier.clone()),
        context: context.clone(),
    };
    let prepared: PreparedModuleRead = ModuleReadService::prepare(library, &request)
        .map_err(|error| error.technical_details.unwrap_or(error.message))?;
    Ok(Entry {
        ecu_family: entry.ecu_family.clone(),
        identifier: entry.identifier.clone(),
        operation: prepared.operation,
        route_id: prepared.transaction.backend_route().to_string(),
        route_validation: prepared.route_validation,
        transaction: prepared.transaction,
        reads: 0,
        failures: 0,
        dropped: false,
        reason: None,
        last_response_hex: None,
        negative_response: None,
        record: None,
    })
}

fn build_report(run: &Run) -> Value {
    json!({
        "schema": LIVE_READ_SCHEMA,
        "schema_version": 1,
        "run_id": format!("live-{}", run.id),
        "application_version": env!("CARGO_PKG_VERSION"),
        "operation": LIVE_READ_OPERATION,
        "safety_class": LIVE_READ_SAFETY_CLASS,
        "route_validation": if run.synthetic { "SYNTHETIC" } else { "SOURCE_BACKED" },
        "validation": "a live read proves the modules answered at those addresses on that route, as a single read does, and nothing more",
        "started_unix_ms": run.started_unix_ms,
        "elapsed_ms": elapsed_ms(run.started),
        "cadence_floor_ms": LIVE_READ_FLOOR.as_millis() as u64,
        "time_cap_ms": LIVE_READ_CAP.as_millis() as u64,
        "achieved_round_ms": run.round_ms,
        "rounds": run.rounds,
        "stopped_reason": run.stopped_reason,
        "vehicle_context": run.context,
        "adapter": run.adapter,
        "set": run.entries.iter().map(|entry| json!({
            "ecu_family": entry.ecu_family,
            "identifier": entry.identifier,
            "operation": entry.operation,
            "route_id": entry.route_id,
            "route_validation": if run.synthetic { "SYNTHETIC".to_string() } else { entry.route_validation.clone() },
            "request_hex": hex(entry.transaction.encoded_payload()),
            "reads": entry.reads,
            "dropped": entry.dropped,
            "reason": entry.reason,
            // The last completed request as a single read records it, for
            // the intake; SYNTHETIC on the bench like everything else here.
            "record": entry.record.as_ref().map(|record| {
                let mut record = record.clone();
                if run.synthetic {
                    record.route_validation = "SYNTHETIC".into();
                }
                record
            }),
        })).collect::<Vec<_>>(),
        "refused": run.refused.iter().map(|refused| json!({
            "ecu_family": refused.ecu_family,
            "identifier": refused.identifier,
            "reason": refused.reason,
        })).collect::<Vec<_>>(),
        "samples": run.samples.iter().map(|sample| json!({
            "at_ms": sample.at_ms,
            "ecu_family": sample.ecu_family,
            "identifier": sample.identifier,
            "response_hex": sample.response_hex,
            "negative_response": sample.negative_response,
            "failure": sample.failure,
            "parameters": sample.parameters,
        })).collect::<Vec<_>>(),
    })
}

fn idle(report_available: bool) -> LiveReadSnapshot {
    LiveReadSnapshot {
        state: LiveReadState::Idle,
        entries: Vec::new(),
        values: Vec::new(),
        rounds: 0,
        samples: 0,
        elapsed_ms: 0,
        round_ms: None,
        cadence_floor_ms: LIVE_READ_FLOOR.as_millis() as u64,
        time_cap_ms: LIVE_READ_CAP.as_millis() as u64,
        route_validation: String::new(),
        stopped_reason: None,
        error: None,
        report_available,
    }
}

fn elapsed_ms(since: Instant) -> u64 {
    u64::try_from(since.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or(0)
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}
