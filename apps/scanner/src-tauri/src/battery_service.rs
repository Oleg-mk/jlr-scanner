//! The battery (ADR-0030): what the battery monitor holds, read from the
//! modules SDD says hold it.
//!
//! A diagnostic session runs on the car's own battery and is exactly when it
//! is weakest — ignition on, engine off, a tester drawing from the same rail
//! — so every silence a module gives can have one cause that has nothing to
//! do with the module. This reads the battery monitor's own dataset: how
//! full it is, what it is doing now, what it leaks while parked, how it has
//! aged, what the car remembers about it, and, on a hybrid, the traction
//! battery from its own module.
//!
//! The shape is the module passport's: the shell owns the plan and the
//! rules, the interface asks for one step at a time and can stop between any
//! two, and every read leaves the record a single read leaves, for the
//! intake. Nothing is judged here: a value the loaded data describes is the
//! number it describes, a value it does not is the bytes it is, and no
//! threshold of this product's own is applied to either.

use crate::module_read_service::{ModuleReadService, PreparedModuleRead};
use crate::read_record::{read_record, ReadIdentity, ReadOutcome};
use app_contracts::{
    AdapterInfo, BatteryReadSnapshot, BatteryReadState, BatteryReading, DiagnosticError,
    DiagnosticErrorCategory, DiagnosticExecutionStage, ModuleReadKind, ModuleReadReport,
    ModuleReadRequest, ModuleReadState, ModuleRefusal, VehicleContextInput,
};
use diagnostic_session::decode::decode_parameters;
use diagnostic_session::{vehicle_context, KnowledgeLibrary};
use mongoose_jlr::MongooseUdsReadResult;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uds_execution::{decode_response, PreparedUdsTransaction, UdsReadOutcome};

/// How long one module is given to answer one battery identifier.
pub const BATTERY_READ_TIMEOUT: Duration = Duration::from_secs(2);
pub const BATTERY_SCHEMA: &str = "prowlone.battery-state";
pub const BATTERY_OPERATION: &str = "BATTERY_STATE";
pub const BATTERY_SAFETY_CLASS: &str = "READ_ONLY";

static RUN_COUNTER: AtomicU64 = AtomicU64::new(1);

/// One read the run still owes, with the transaction prepared at planning.
pub struct BatteryDue {
    pub index: usize,
    pub transaction: PreparedUdsTransaction,
}

/// One parameter the identifier carries: a record of bits carries several,
/// and each is its own row on the card.
struct ParameterRow {
    /// SDD's own name for the parameter.
    parameter: String,
    role: String,
    headline: bool,
    unit: Option<String>,
}

/// One read the run makes: one module, one identifier, and every parameter
/// that identifier carries for it.
struct Entry {
    ecu_family: String,
    identifier: u16,
    parameters: Vec<ParameterRow>,
    route_id: String,
    route_validation: String,
    transaction: PreparedUdsTransaction,
    done: bool,
}

struct Run {
    id: u64,
    state: BatteryReadState,
    started_unix_ms: u128,
    context: VehicleContextInput,
    adapter: Option<AdapterInfo>,
    entries: Vec<Entry>,
    readings: Vec<BatteryReading>,
    /// One record per read made, answered or not, in the shape a single
    /// read leaves — what the intake turns into evidence.
    records: Vec<ModuleReadReport>,
    /// Modules asked for but not planned, with the resolver's reason.
    refused: Vec<ModuleRefusal>,
    asked: u32,
    answered: u32,
    synthetic: bool,
    error: Option<DiagnosticError>,
}

#[derive(Default)]
pub struct BatteryService {
    run: Option<Run>,
    last_report: Option<Value>,
}

impl BatteryService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> BatteryReadSnapshot {
        match &self.run {
            Some(run) => run.snapshot(self.last_report.is_some()),
            None => idle(self.last_report.is_some()),
        }
    }

    /// No run begins, and the reason is the real one — no adapter — not a
    /// claim about the data.
    pub fn refuse(&mut self, error: DiagnosticError) -> BatteryReadSnapshot {
        self.run = None;
        let mut snapshot = idle(self.last_report.is_some());
        snapshot.state = BatteryReadState::Finished;
        snapshot.error = Some(error);
        snapshot
    }

    /// Plan a run: for every module, each battery parameter the loaded data
    /// declares for it on this car, one read per identifier. A module the
    /// data gives no battery parameter is passed over without a word; a
    /// module whose route cannot be planned is listed with the resolver's
    /// reason.
    pub fn start(
        &mut self,
        library: &KnowledgeLibrary,
        adapter: Option<&AdapterInfo>,
        context: &VehicleContextInput,
        families: &[String],
    ) -> BatteryReadSnapshot {
        let resolved = vehicle_context(context);
        let mut entries: Vec<Entry> = Vec::new();
        let mut refused: Vec<ModuleRefusal> = Vec::new();

        for family in families {
            for parameter in library.battery_parameters(&resolved, family) {
                // One request per module and identifier, however many
                // parameters that identifier carries: the rest are read
                // from the same answer.
                if let Some(entry) = entries.iter_mut().find(|entry| {
                    entry.ecu_family == *family && entry.identifier == parameter.identifier
                }) {
                    entry.parameters.push(ParameterRow::new(&parameter));
                    continue;
                }
                let request = ModuleReadRequest {
                    ecu_family: family.clone(),
                    kind: ModuleReadKind::Identifier,
                    identifier: Some(format!("0x{:04X}", parameter.identifier)),
                    context: context.clone(),
                };
                match ModuleReadService::prepare(library, &request) {
                    Ok(prepared) => entries.push(Entry::new(family, &parameter, prepared)),
                    Err(error) => {
                        let reason = error.technical_details.unwrap_or(error.message);
                        // One line per module, not one per parameter.
                        if !refused.iter().any(|known| &known.ecu_family == family) {
                            refused.push(ModuleRefusal {
                                ecu_family: family.clone(),
                                reason,
                            });
                        }
                    }
                }
            }
        }

        let planned = !entries.is_empty();
        let mut run = Run {
            id: RUN_COUNTER.fetch_add(1, Ordering::Relaxed),
            state: if planned {
                BatteryReadState::Running
            } else {
                BatteryReadState::Finished
            },
            started_unix_ms: unix_ms(),
            context: context.clone(),
            adapter: adapter.cloned(),
            entries,
            readings: Vec::new(),
            records: Vec::new(),
            refused,
            asked: 0,
            answered: 0,
            synthetic: false,
            error: None,
        };
        if !planned {
            run.error = Some(DiagnosticError {
                category: DiagnosticErrorCategory::UnsupportedVehicleProfile,
                message: "The loaded data names no battery parameter for this car".into(),
                technical_details: Some(
                    "no module of this car declares an identifier the battery rule recognises (ADR-0030)"
                        .into(),
                ),
                stage: DiagnosticExecutionStage::Preparation,
            });
        }
        self.run = Some(run);
        self.last_report = None;
        self.snapshot()
    }

    /// The next read the run owes, or `None` when it owes none. An
    /// identifier already asked of the same module is not asked again: its
    /// other parameters are decoded from the same answer.
    pub fn next_due(&mut self) -> Option<BatteryDue> {
        let run = self.run.as_mut()?;
        if !matches!(run.state, BatteryReadState::Running) {
            return None;
        }
        let index = run.entries.iter().position(|entry| !entry.done)?;
        run.entries[index].done = true;
        run.asked += 1;
        Some(BatteryDue {
            index,
            transaction: run.entries[index].transaction.clone(),
        })
    }

    /// Record what one module answered for one identifier, or that it did
    /// not. Every parameter that identifier carries is decoded from the one
    /// answer.
    pub fn record(
        &mut self,
        index: usize,
        result: Result<MongooseUdsReadResult, DiagnosticError>,
    ) -> BatteryReadSnapshot {
        if let Some(run) = self.run.as_mut() {
            run.record(index, result);
            if run.entries.iter().all(|entry| entry.done) {
                run.finish();
            }
        }
        self.rebuild_report();
        self.snapshot()
    }

    /// Stop before the end; what was read stays.
    pub fn finish(&mut self) -> BatteryReadSnapshot {
        if let Some(run) = self.run.as_mut() {
            run.finish();
        }
        self.rebuild_report();
        self.snapshot()
    }

    /// The bench answered this run (ADR-0020): every reading is synthetic.
    pub fn mark_synthetic(&mut self) -> BatteryReadSnapshot {
        if let Some(run) = self.run.as_mut() {
            run.synthetic = true;
        }
        self.rebuild_report();
        self.snapshot()
    }

    fn rebuild_report(&mut self) {
        let finished = self
            .run
            .as_ref()
            .is_some_and(|run| matches!(run.state, BatteryReadState::Finished));
        if !finished {
            return;
        }
        self.last_report = self.run.as_ref().map(build_report);
    }

    pub fn report_json(&self) -> Result<String, String> {
        let report = self
            .last_report
            .as_ref()
            .ok_or_else(|| "No battery reading has been recorded yet".to_owned())?;
        serde_json::to_string_pretty(report).map_err(|error| error.to_string())
    }

    /// Handed to the session bundle once, so a second finish cannot record
    /// the same run twice.
    pub fn take_report_json(&mut self) -> Option<String> {
        let json = self.report_json().ok();
        self.last_report = None;
        json
    }
}

impl ParameterRow {
    fn new(parameter: &diagnostic_session::BatteryParameter) -> Self {
        Self {
            parameter: parameter.parameter.clone(),
            role: parameter.role.as_str().to_string(),
            headline: parameter.headline,
            unit: parameter.unit.clone(),
        }
    }
}

impl Entry {
    fn new(
        family: &str,
        parameter: &diagnostic_session::BatteryParameter,
        prepared: PreparedModuleRead,
    ) -> Self {
        Self {
            ecu_family: family.to_string(),
            identifier: parameter.identifier,
            parameters: vec![ParameterRow::new(parameter)],
            route_id: prepared.transaction.backend_route().to_string(),
            route_validation: prepared.route_validation,
            transaction: prepared.transaction,
            done: false,
        }
    }
}

impl Run {
    fn record(&mut self, index: usize, result: Result<MongooseUdsReadResult, DiagnosticError>) {
        let Some(entry) = self.entries.get(index) else {
            return;
        };

        // The record first, from what was sent and what came back.
        let identity = ReadIdentity {
            context: &self.context,
            ecu_family: &entry.ecu_family,
            operation: BATTERY_OPERATION,
            route_validation: if self.synthetic {
                "SYNTHETIC"
            } else {
                entry.route_validation.as_str()
            },
            adapter: self.adapter.as_ref(),
        };
        let outcome = match &result {
            Ok(raw) => ReadOutcome::Answered(raw),
            Err(error) => ReadOutcome::Failed(error),
        };
        let record = read_record(identity, &entry.transaction, outcome, None);
        self.records.push(record);

        let decoded = match &result {
            Ok(raw) => {
                let entry = &self.entries[index];
                #[allow(clippy::let_and_return)]
                match decode_response(&entry.transaction, &raw.raw_diagnostic_response) {
                    Ok(Some(UdsReadOutcome::DataByIdentifier { data, .. })) => {
                        let parameters = entry
                            .transaction
                            .readable_identifier()
                            .map(|identifier| identifier.parameters.clone())
                            .unwrap_or_default();
                        Decoded::Values(decode_parameters(&parameters, &data))
                    }
                    Ok(Some(UdsReadOutcome::Negative(negative))) => {
                        Decoded::Negative(format!("{:?}", negative.code))
                    }
                    Ok(Some(UdsReadOutcome::DtcReport(_))) => Decoded::Reason(
                        "a fault-code report answered an identifier read".to_string(),
                    ),
                    Ok(None) => {
                        Decoded::Reason("the final answer was still ResponsePending".to_string())
                    }
                    Err(error) => Decoded::Reason(error.to_string()),
                }
            }
            Err(error) => Decoded::Reason(
                error
                    .technical_details
                    .clone()
                    .unwrap_or_else(|| error.message.clone()),
            ),
        };
        let raw_hex = result
            .as_ref()
            .ok()
            .map(|raw| hex(&raw.raw_diagnostic_response));

        // Every parameter this identifier carries is a row, read from the
        // one answer; the read itself is counted once.
        let entry = &self.entries[index];
        let mut answered_this_read = false;
        let mut rows = Vec::new();
        for parameter in &entry.parameters {
            let mut row = BatteryReading {
                ecu_family: entry.ecu_family.clone(),
                identifier: format!("0x{:04X}", entry.identifier),
                parameter: parameter.parameter.clone(),
                role: parameter.role.clone(),
                headline: parameter.headline,
                state: ModuleReadState::Failed,
                value: None,
                unit: parameter.unit.clone(),
                route_id: entry.route_id.clone(),
                route_validation: entry.route_validation.clone(),
                raw_response_hex: raw_hex.clone(),
                negative_response: None,
                note: None,
                reason: None,
            };
            match &decoded {
                Decoded::Values(values) => {
                    match values
                        .iter()
                        .find(|value| value.name == parameter.parameter)
                    {
                        Some(value) => {
                            row.state = ModuleReadState::Succeeded;
                            row.value = value.value.clone();
                            row.note = value.note.clone();
                            answered_this_read = true;
                        }
                        None => {
                            row.reason =
                                Some("the answer does not carry this parameter".to_string());
                        }
                    }
                }
                Decoded::Negative(code) => {
                    row.state = ModuleReadState::Succeeded;
                    row.negative_response = Some(code.clone());
                }
                Decoded::Reason(reason) => row.reason = Some(reason.clone()),
            }
            rows.push(row);
        }
        // `asked` and `answered` both count reads, never parameters: the
        // lesson the mileage survey taught on 2026-09-12.
        if answered_this_read {
            self.answered += 1;
        }
        self.entries[index].done = true;
        self.readings.extend(rows);
    }

    fn finish(&mut self) {
        self.state = BatteryReadState::Finished;
    }

    fn modules(&self) -> u32 {
        self.entries
            .iter()
            .map(|entry| entry.ecu_family.as_str())
            .collect::<BTreeSet<_>>()
            .len() as u32
    }

    fn snapshot(&self, report_available: bool) -> BatteryReadSnapshot {
        BatteryReadSnapshot {
            state: self.state,
            readings: self
                .readings
                .iter()
                .cloned()
                .map(|mut row| {
                    if self.synthetic {
                        row.route_validation = "SYNTHETIC".into();
                    }
                    row
                })
                .collect(),
            planned: self.entries.len() as u32,
            asked: self.asked,
            answered: self.answered,
            modules: self.modules(),
            route_validation: if self.synthetic {
                "SYNTHETIC".into()
            } else {
                self.entries
                    .first()
                    .map(|entry| entry.route_validation.clone())
                    .unwrap_or_default()
            },
            read_unix_ms: u64::try_from(self.started_unix_ms).ok(),
            refused: self.refused.clone(),
            error: self.error.clone(),
            report_available,
        }
    }
}

/// What one answer said, shared by every parameter of that identifier.
enum Decoded {
    Values(Vec<diagnostic_session::decode::DecodedParameter>),
    Negative(String),
    Reason(String),
}

fn build_report(run: &Run) -> Value {
    json!({
        "schema": BATTERY_SCHEMA,
        "schema_version": 1,
        "battery_read_id": format!("battery-{}", run.id),
        "application_version": env!("CARGO_PKG_VERSION"),
        "operation": BATTERY_OPERATION,
        "safety_class": BATTERY_SAFETY_CLASS,
        "route_validation": if run.synthetic { "SYNTHETIC" } else { "SOURCE_BACKED" },
        "validation": "what the battery monitor holds, read from the modules the loaded data names; a value the data describes is the number it describes, a value it does not is the bytes it is, and no threshold of this product's own is applied to either",
        "started_unix_ms": run.started_unix_ms,
        "vehicle_context": run.context,
        "adapter": run.adapter,
        "modules": run.modules(),
        "planned": run.entries.len(),
        "asked": run.asked,
        "answered": run.answered,
        // Every row says what its value is worth, on the bench as on
        // screen (ADR-0020, decision 6).
        "readings": run.readings.iter().cloned().map(|mut row| {
            if run.synthetic {
                row.route_validation = "SYNTHETIC".into();
            }
            row
        }).collect::<Vec<_>>(),
        // Every read as a single read records it, for the intake.
        "reads": run.records.iter().cloned().map(|mut record| {
            if run.synthetic {
                record.route_validation = "SYNTHETIC".into();
            }
            record
        }).collect::<Vec<_>>(),
        "not_planned": run.refused,
    })
}

fn idle(report_available: bool) -> BatteryReadSnapshot {
    BatteryReadSnapshot {
        state: BatteryReadState::Idle,
        readings: Vec::new(),
        planned: 0,
        asked: 0,
        answered: 0,
        modules: 0,
        route_validation: String::new(),
        read_unix_ms: None,
        refused: Vec::new(),
        error: None,
        report_available,
    }
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
