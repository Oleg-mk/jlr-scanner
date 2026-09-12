//! The module passport (ADR-0027): what a module says it is.
//!
//! The first thing a workshop asks of a module after "does it answer" is
//! "which one is it" — the part number that was fitted, the software it
//! runs, its serial. SDD's platform documents name, per module, the
//! identifiers that hold those texts, and the library carries them in the
//! catalogue's own shape, so reading a passport is reading identifiers: one
//! request per module and identifier, once each, the text shown as the text
//! it is. Nothing is parsed out of a part number.
//!
//! Since `ADR-0033` each number is also set against JLR's own IVS part
//! lineage, which the library carries: the number agrees with the catalogue,
//! the catalogue names a different one, it names none for that identifier,
//! or it does not carry the assembly at all. That is a comparison and not a
//! verdict — no word such as *outdated* appears here or on screen, and this
//! product could not act on one if it did.
//!
//! The shape is the mileage survey's: the shell owns the plan and the rules,
//! the interface asks for one step at a time and can stop between any two;
//! every read leaves the record a single read leaves, for the intake.

use crate::module_read_service::{ModuleReadService, PreparedModuleRead};
use crate::read_record::{read_record, ReadIdentity, ReadOutcome};
use app_contracts::{
    AdapterInfo, DiagnosticError, DiagnosticErrorCategory, DiagnosticExecutionStage,
    ModulePassportSnapshot, ModulePassportState, ModuleReadKind, ModuleReadReport,
    ModuleReadRequest, ModuleReadState, PassportReading, VehicleContextInput,
};
use diagnostic_session::decode::decode_parameters;
use diagnostic_session::{
    catalogue_comparisons, vehicle_context, CatalogueAssembly, KnowledgeLibrary,
};
use mongoose_jlr::MongooseUdsReadResult;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uds_execution::{decode_response, PreparedUdsTransaction, UdsReadOutcome};

/// How long one module is given to answer one identifier.
pub const PASSPORT_READ_TIMEOUT: Duration = Duration::from_secs(2);
pub const PASSPORT_SCHEMA: &str = "prowlone.module-passport";
pub const PASSPORT_OPERATION: &str = "MODULE_PASSPORT";
pub const PASSPORT_SAFETY_CLASS: &str = "READ_ONLY";

static RUN_COUNTER: AtomicU64 = AtomicU64::new(1);

/// One read the run still owes, with the transaction prepared at planning.
pub struct PassportDue {
    pub index: usize,
    pub transaction: PreparedUdsTransaction,
}

struct Entry {
    ecu_family: String,
    identifier: u16,
    /// SDD's own name for the identifier.
    parameter: String,
    route_id: String,
    route_validation: String,
    transaction: PreparedUdsTransaction,
    done: bool,
}

struct Run {
    id: u64,
    state: ModulePassportState,
    started_unix_ms: u128,
    context: VehicleContextInput,
    adapter: Option<AdapterInfo>,
    entries: Vec<Entry>,
    readings: Vec<PassportReading>,
    /// What JLR's catalogue names for each module of this run (ADR-0033),
    /// read once at planning: the comparison is made as answers arrive, and
    /// the library is not in hand by then.
    catalogue: BTreeMap<String, Vec<CatalogueAssembly>>,
    /// One record per read made, answered or not, in the shape a single
    /// read leaves — what the intake turns into evidence.
    records: Vec<ModuleReadReport>,
    /// Modules asked for but not planned, with the resolver's reason.
    refused: Vec<(String, String)>,
    asked: u32,
    answered: u32,
    synthetic: bool,
    error: Option<DiagnosticError>,
}

#[derive(Default)]
pub struct PassportService {
    run: Option<Run>,
    last_report: Option<Value>,
}

impl PassportService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> ModulePassportSnapshot {
        match &self.run {
            Some(run) => run.snapshot(self.last_report.is_some()),
            None => idle(self.last_report.is_some()),
        }
    }

    /// No run begins, and the reason is the real one — no adapter — not a
    /// claim about the data.
    pub fn refuse(&mut self, error: DiagnosticError) -> ModulePassportSnapshot {
        self.run = None;
        let mut snapshot = idle(self.last_report.is_some());
        snapshot.state = ModulePassportState::Finished;
        snapshot.error = Some(error);
        snapshot
    }

    /// Plan a run: for each module, every identification identifier the
    /// loaded data declares for it on this car, one read each. A module the
    /// data gives no identification is passed over without a word; a module
    /// whose route cannot be planned is listed with the resolver's reason.
    pub fn start(
        &mut self,
        library: &KnowledgeLibrary,
        adapter: Option<&AdapterInfo>,
        context: &VehicleContextInput,
        families: &[String],
    ) -> ModulePassportSnapshot {
        let resolved = vehicle_context(context);
        let mut entries: Vec<Entry> = Vec::new();
        let mut refused = Vec::new();
        let mut catalogue: BTreeMap<String, Vec<CatalogueAssembly>> = BTreeMap::new();

        for family in families {
            let known = library.catalogue_assemblies(&resolved, family);
            if !known.is_empty() {
                catalogue.insert(family.clone(), known);
            }
            for (identifier, parameter) in library.identification_identifiers(&resolved, family) {
                let request = ModuleReadRequest {
                    ecu_family: family.clone(),
                    kind: ModuleReadKind::Identifier,
                    identifier: Some(format!("0x{identifier:04X}")),
                    context: context.clone(),
                };
                match ModuleReadService::prepare(library, &request) {
                    Ok(prepared) => {
                        entries.push(Entry::new(family, identifier, parameter, prepared))
                    }
                    Err(error) => {
                        let reason = error.technical_details.unwrap_or(error.message);
                        // One line per module, not one per identifier.
                        if !refused
                            .iter()
                            .any(|(known, _): &(String, String)| known == family)
                        {
                            refused.push((family.clone(), reason));
                        }
                    }
                }
            }
        }

        let planned = !entries.is_empty();
        let mut run = Run {
            id: RUN_COUNTER.fetch_add(1, Ordering::Relaxed),
            state: if planned {
                ModulePassportState::Running
            } else {
                ModulePassportState::Finished
            },
            started_unix_ms: unix_ms(),
            context: context.clone(),
            adapter: adapter.cloned(),
            entries,
            readings: Vec::new(),
            catalogue,
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
                message: "The loaded data names no identification for these modules".into(),
                technical_details: Some(
                    "no chosen module lists an identifier whose payload the platform declares as a text — a part number, a serial, a software level"
                        .into(),
                ),
                stage: DiagnosticExecutionStage::Preparation,
            });
        }
        self.run = Some(run);
        self.last_report = None;
        self.snapshot()
    }

    /// The next read the run owes, or `None` when it owes none.
    pub fn next_due(&mut self) -> Option<PassportDue> {
        let run = self.run.as_mut()?;
        if !matches!(run.state, ModulePassportState::Running) {
            return None;
        }
        let index = run.entries.iter().position(|entry| !entry.done)?;
        run.entries[index].done = true;
        run.asked += 1;
        Some(PassportDue {
            index,
            transaction: run.entries[index].transaction.clone(),
        })
    }

    /// Record what one module answered for one identifier, or that it did
    /// not.
    pub fn record(
        &mut self,
        index: usize,
        result: Result<MongooseUdsReadResult, DiagnosticError>,
    ) -> ModulePassportSnapshot {
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
    pub fn finish(&mut self) -> ModulePassportSnapshot {
        if let Some(run) = self.run.as_mut() {
            run.finish();
        }
        self.rebuild_report();
        self.snapshot()
    }

    /// The bench answered this run (ADR-0020): every reading is synthetic.
    pub fn mark_synthetic(&mut self) -> ModulePassportSnapshot {
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
            .is_some_and(|run| matches!(run.state, ModulePassportState::Finished));
        if !finished {
            return;
        }
        self.last_report = self.run.as_ref().map(build_report);
    }

    pub fn report_json(&self) -> Result<String, String> {
        let report = self
            .last_report
            .as_ref()
            .ok_or_else(|| "No module passport has been recorded yet".to_owned())?;
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

impl Entry {
    fn new(family: &str, identifier: u16, parameter: String, prepared: PreparedModuleRead) -> Self {
        Self {
            ecu_family: family.to_string(),
            identifier,
            parameter,
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
            operation: PASSPORT_OPERATION,
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

        let mut row = PassportReading {
            ecu_family: entry.ecu_family.clone(),
            identifier: format!("0x{:04X}", entry.identifier),
            parameter: entry.parameter.clone(),
            state: ModuleReadState::Failed,
            value: None,
            route_id: entry.route_id.clone(),
            route_validation: entry.route_validation.clone(),
            raw_response_hex: None,
            negative_response: None,
            note: None,
            reason: None,
            catalogue: None,
        };

        match result {
            Ok(raw) => {
                row.raw_response_hex = Some(hex(&raw.raw_diagnostic_response));
                match decode_response(&entry.transaction, &raw.raw_diagnostic_response) {
                    Ok(Some(UdsReadOutcome::DataByIdentifier { data, .. })) => {
                        let parameters = entry
                            .transaction
                            .readable_identifier()
                            .map(|identifier| identifier.parameters.clone())
                            .unwrap_or_default();
                        let decoded = decode_parameters(&parameters, &data);
                        // The identifier may carry a catalogue parameter
                        // beside its text; the text is the passport's row.
                        match decoded.iter().find(|value| value.name == entry.parameter) {
                            Some(value) => {
                                row.state = ModuleReadState::Succeeded;
                                row.value = value.value.clone();
                                row.note = value.note.clone();
                                self.answered += 1;
                            }
                            None => {
                                row.reason =
                                    Some("the answer does not carry this parameter".to_string());
                            }
                        }
                    }
                    Ok(Some(UdsReadOutcome::Negative(negative))) => {
                        row.state = ModuleReadState::Succeeded;
                        row.negative_response = Some(format!("{:?}", negative.code));
                    }
                    Ok(Some(UdsReadOutcome::DtcReport(_))) => {
                        row.reason =
                            Some("a fault-code report answered an identifier read".to_string());
                    }
                    Ok(None) => {
                        row.reason = Some("the final answer was still ResponsePending".to_string());
                    }
                    Err(error) => row.reason = Some(error.to_string()),
                }
            }
            Err(error) => {
                row.reason = Some(
                    error
                        .technical_details
                        .clone()
                        .unwrap_or_else(|| error.message.clone()),
                );
            }
        }

        self.readings.push(row);
        self.compare_with_catalogue();
    }

    /// Set every reading against the catalogue (ADR-0033). Recomputed from
    /// scratch each time, because the assembly a module reports can arrive
    /// after a number it explains.
    fn compare_with_catalogue(&mut self) {
        if self.catalogue.is_empty() {
            return;
        }
        let comparisons = catalogue_comparisons(&self.catalogue, &self.readings);
        for (reading, comparison) in self.readings.iter_mut().zip(comparisons) {
            reading.catalogue = comparison;
        }
    }

    fn finish(&mut self) {
        self.state = ModulePassportState::Finished;
    }

    fn modules(&self) -> u32 {
        self.entries
            .iter()
            .map(|entry| entry.ecu_family.as_str())
            .collect::<BTreeSet<_>>()
            .len() as u32
    }

    fn snapshot(&self, report_available: bool) -> ModulePassportSnapshot {
        ModulePassportSnapshot {
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
            error: self.error.clone(),
            report_available,
        }
    }
}

fn build_report(run: &Run) -> Value {
    json!({
        "schema": PASSPORT_SCHEMA,
        "schema_version": 1,
        "passport_id": format!("passport-{}", run.id),
        "application_version": env!("CARGO_PKG_VERSION"),
        "operation": PASSPORT_OPERATION,
        "safety_class": PASSPORT_SAFETY_CLASS,
        "route_validation": if run.synthetic { "SYNTHETIC" } else { "SOURCE_BACKED" },
        "validation": "what each module holds in its identification identifiers, as the text it holds; nothing is parsed out of it. Where the library carries JLR's IVS part lineage for the module, each number is also set against it (ADR-0033): AGREES, DIFFERS, NOT_NAMED or NO_ASSEMBLY, with the catalogue's own date. A comparison, not a verdict.",
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
        "not_planned": run.refused.iter().map(|(family, reason)| json!({
            "ecu_family": family,
            "reason": reason,
        })).collect::<Vec<_>>(),
    })
}

fn idle(report_available: bool) -> ModulePassportSnapshot {
    ModulePassportSnapshot {
        state: ModulePassportState::Idle,
        readings: Vec::new(),
        planned: 0,
        asked: 0,
        answered: 0,
        modules: 0,
        route_validation: String::new(),
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
