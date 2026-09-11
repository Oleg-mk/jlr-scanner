//! The odometer, read from every module (ADR-0024).
//!
//! A car keeps its mileage in dozens of places, and a rolled-back car is
//! rolled back only where the tool could reach. This asks every module the
//! loaded data says holds a distance, once each, and puts the answers side by
//! side with the difference from the highest running total already worked out
//! — arithmetic over two numbers this product read itself.
//!
//! It draws no conclusion. No word for what a difference might mean appears
//! here or on screen; a module that says nothing is recorded as saying
//! nothing rather than as a zero, and the legislated counter that resets with
//! the codes is kept out by `diagnostic_session::mileage`.
//!
//! The shape is the live read's: the shell owns the plan and the rules, the
//! interface asks for one step at a time.

use crate::module_read_service::{ModuleReadService, PreparedModuleRead};
use app_contracts::{
    AdapterInfo, DiagnosticError, DiagnosticErrorCategory, DiagnosticExecutionStage, MileageKind,
    MileageReading, MileageSurveySnapshot, MileageSurveyState, ModuleReadKind, ModuleReadRequest,
    ModuleReadState, VehicleContextInput,
};
use diagnostic_session::decode::decode_parameters;
use diagnostic_session::{mileage, vehicle_context, KnowledgeLibrary};
use mongoose_jlr::MongooseUdsReadResult;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uds_execution::{decode_response, PreparedUdsTransaction, UdsReadOutcome};

/// How long one module is given to answer.
pub const MILEAGE_READ_TIMEOUT: Duration = Duration::from_secs(2);
pub const MILEAGE_SCHEMA: &str = "prowlone.mileage-survey";
pub const MILEAGE_OPERATION: &str = "MILEAGE_SURVEY";
pub const MILEAGE_SAFETY_CLASS: &str = "READ_ONLY";

static RUN_COUNTER: AtomicU64 = AtomicU64::new(1);

/// One read the survey still owes, with the transaction prepared at planning.
pub struct MileageDue {
    pub index: usize,
    pub transaction: PreparedUdsTransaction,
}

struct Entry {
    ecu_family: String,
    identifier: u16,
    /// Which of the identifier's parameters are mileages, by SDD's name.
    wanted: Vec<(String, MileageKind)>,
    route_id: String,
    route_validation: String,
    transaction: PreparedUdsTransaction,
    done: bool,
}

struct Run {
    id: u64,
    state: MileageSurveyState,
    started_unix_ms: u128,
    context: VehicleContextInput,
    adapter: Option<AdapterInfo>,
    entries: Vec<Entry>,
    readings: Vec<MileageReading>,
    /// Modules the data describes but whose route could not be planned.
    refused: Vec<(String, String)>,
    asked: u32,
    answered: u32,
    synthetic: bool,
    error: Option<DiagnosticError>,
}

#[derive(Default)]
pub struct MileageService {
    run: Option<Run>,
    last_report: Option<Value>,
}

impl MileageService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> MileageSurveySnapshot {
        match &self.run {
            Some(run) => run.snapshot(self.last_report.is_some()),
            None => idle(self.last_report.is_some()),
        }
    }

    /// Plan a survey: every module the survey can reach, every identifier the
    /// loaded data says carries a distance for it.
    pub fn start(
        &mut self,
        library: &KnowledgeLibrary,
        adapter: Option<&AdapterInfo>,
        context: &VehicleContextInput,
        families: &[String],
    ) -> MileageSurveySnapshot {
        let resolved = vehicle_context(context);
        let mut entries: Vec<Entry> = Vec::new();
        let mut refused = Vec::new();

        for family in families {
            let wanted = library.mileage_identifiers(&resolved, family);
            if wanted.is_empty() {
                continue;
            }
            // One read per identifier, however many of its parameters are
            // mileages.
            let mut by_identifier: std::collections::BTreeMap<u16, Vec<(String, MileageKind)>> =
                std::collections::BTreeMap::new();
            for (identifier, parameter, kind) in wanted {
                // The library's kind and the contract's are the same idea on
                // two sides of a boundary; the crossing is spelled out.
                let kind = match kind {
                    mileage::MileageKind::Current => MileageKind::Current,
                    mileage::MileageKind::Event => MileageKind::Event,
                };
                by_identifier
                    .entry(identifier)
                    .or_default()
                    .push((parameter, kind));
            }
            for (identifier, parameters) in by_identifier {
                let request = ModuleReadRequest {
                    ecu_family: family.clone(),
                    kind: ModuleReadKind::Identifier,
                    identifier: Some(format!("0x{identifier:04X}")),
                    context: context.clone(),
                };
                match ModuleReadService::prepare(library, &request) {
                    Ok(prepared) => {
                        entries.push(Entry::new(family, identifier, parameters, prepared))
                    }
                    Err(error) => refused.push((
                        family.clone(),
                        error.technical_details.unwrap_or(error.message),
                    )),
                }
            }
        }

        let planned = !entries.is_empty();
        let mut run = Run {
            id: RUN_COUNTER.fetch_add(1, Ordering::Relaxed),
            state: if planned {
                MileageSurveyState::Running
            } else {
                MileageSurveyState::Finished
            },
            started_unix_ms: unix_ms(),
            context: context.clone(),
            adapter: adapter.cloned(),
            entries,
            readings: Vec::new(),
            refused,
            asked: 0,
            answered: 0,
            synthetic: false,
            error: None,
        };
        if !planned {
            run.error = Some(DiagnosticError {
                category: DiagnosticErrorCategory::UnsupportedVehicleProfile,
                message: "The loaded data names no mileage for this vehicle".into(),
                technical_details: Some(
                    "no surveyed module lists an identifier whose parameter is a distance the car has travelled"
                        .into(),
                ),
                stage: DiagnosticExecutionStage::Preparation,
            });
        }
        self.run = Some(run);
        self.last_report = None;
        self.snapshot()
    }

    /// The next read the survey owes, or `None` when it owes none.
    pub fn next_due(&mut self) -> Option<MileageDue> {
        let run = self.run.as_mut()?;
        if !matches!(run.state, MileageSurveyState::Running) {
            return None;
        }
        let index = run.entries.iter().position(|entry| !entry.done)?;
        run.entries[index].done = true;
        run.asked += 1;
        Some(MileageDue {
            index,
            transaction: run.entries[index].transaction.clone(),
        })
    }

    /// Record what one module answered, or that it did not.
    pub fn record(
        &mut self,
        index: usize,
        result: Result<MongooseUdsReadResult, DiagnosticError>,
    ) -> MileageSurveySnapshot {
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
    pub fn finish(&mut self) -> MileageSurveySnapshot {
        if let Some(run) = self.run.as_mut() {
            run.finish();
        }
        self.rebuild_report();
        self.snapshot()
    }

    /// The bench answered this survey (ADR-0020): every reading is synthetic.
    pub fn mark_synthetic(&mut self) -> MileageSurveySnapshot {
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
            .is_some_and(|run| matches!(run.state, MileageSurveyState::Finished));
        if !finished {
            return;
        }
        self.last_report = self.run.as_ref().map(build_report);
    }

    pub fn report_json(&self) -> Result<String, String> {
        let report = self
            .last_report
            .as_ref()
            .ok_or_else(|| "No mileage survey has been recorded yet".to_owned())?;
        serde_json::to_string_pretty(report).map_err(|error| error.to_string())
    }

    /// Handed to the session bundle once, so a second finish cannot record
    /// the same survey twice.
    pub fn take_report_json(&mut self) -> Option<String> {
        let json = self.report_json().ok();
        self.last_report = None;
        json
    }
}

impl Entry {
    fn new(
        family: &str,
        identifier: u16,
        wanted: Vec<(String, MileageKind)>,
        prepared: PreparedModuleRead,
    ) -> Self {
        Self {
            ecu_family: family.to_string(),
            identifier,
            wanted,
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
        let identifier = format!("0x{:04X}", entry.identifier);
        let mut rows: Vec<MileageReading> = entry
            .wanted
            .iter()
            .map(|(parameter, kind)| MileageReading {
                ecu_family: entry.ecu_family.clone(),
                identifier: identifier.clone(),
                parameter: parameter.clone(),
                kind: *kind,
                state: ModuleReadState::Failed,
                value: None,
                unit: None,
                number: None,
                raw: None,
                difference: None,
                route_id: entry.route_id.clone(),
                route_validation: entry.route_validation.clone(),
                raw_response_hex: None,
                negative_response: None,
                note: None,
                reason: None,
            })
            .collect();

        match result {
            Ok(raw) => {
                let response_hex = hex(&raw.raw_diagnostic_response);
                for row in rows.iter_mut() {
                    row.raw_response_hex = Some(response_hex.clone());
                }
                match decode_response(&entry.transaction, &raw.raw_diagnostic_response) {
                    Ok(Some(UdsReadOutcome::DataByIdentifier { data, .. })) => {
                        let parameters = entry
                            .transaction
                            .readable_identifier()
                            .map(|identifier| identifier.parameters.clone())
                            .unwrap_or_default();
                        let decoded = decode_parameters(&parameters, &data);
                        for row in rows.iter_mut() {
                            match decoded.iter().find(|value| value.name == row.parameter) {
                                Some(value) => {
                                    row.state = ModuleReadState::Succeeded;
                                    row.value = value.value.clone();
                                    row.unit = value.unit.clone();
                                    row.raw = value.raw;
                                    row.note = value.note.clone();
                                    row.number = value
                                        .value
                                        .as_deref()
                                        .and_then(|text| text.parse::<f64>().ok());
                                    self.answered += 1;
                                }
                                None => {
                                    row.reason =
                                        Some("the answer does not carry this parameter".to_string())
                                }
                            }
                        }
                    }
                    Ok(Some(UdsReadOutcome::Negative(negative))) => {
                        let text = format!("{:?}", negative.code);
                        for row in rows.iter_mut() {
                            row.state = ModuleReadState::Succeeded;
                            row.negative_response = Some(text.clone());
                        }
                    }
                    Ok(Some(UdsReadOutcome::DtcReport(_))) => {
                        set_reason(&mut rows, "a fault-code report answered an identifier read");
                    }
                    Ok(None) => {
                        set_reason(&mut rows, "the final answer was still ResponsePending");
                    }
                    Err(error) => set_reason(&mut rows, &error.to_string()),
                }
            }
            Err(error) => {
                let text = error
                    .technical_details
                    .clone()
                    .unwrap_or_else(|| error.message.clone());
                set_reason(&mut rows, &text);
            }
        }

        self.readings.extend(rows);
        self.recompute_differences();
    }

    /// The reference is the highest running total on the car, not the
    /// cluster's: the cluster is precisely what gets rewritten.
    fn recompute_differences(&mut self) {
        let highest = self
            .readings
            .iter()
            .filter(|row| row.kind == MileageKind::Current)
            .filter_map(|row| row.number)
            .fold(None::<f64>, |best, value| {
                Some(best.map_or(value, |best| best.max(value)))
            });
        let Some(highest) = highest else {
            for row in self.readings.iter_mut() {
                row.difference = None;
            }
            return;
        };
        for row in self.readings.iter_mut() {
            row.difference = row.number.map(|value| value - highest);
        }
    }

    fn finish(&mut self) {
        self.state = MileageSurveyState::Finished;
    }

    fn highest(&self) -> Option<(&MileageReading, f64)> {
        self.readings
            .iter()
            .filter(|row| row.kind == MileageKind::Current)
            .filter_map(|row| row.number.map(|number| (row, number)))
            .fold(None, |best, (row, number)| match best {
                Some((_, seen)) if seen >= number => best,
                _ => Some((row, number)),
            })
    }

    fn snapshot(&self, report_available: bool) -> MileageSurveySnapshot {
        let highest = self.highest();
        MileageSurveySnapshot {
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
            highest: highest.map(|(_, number)| number),
            highest_module: highest.map(|(row, _)| row.ecu_family.clone()),
            unit: highest.and_then(|(row, _)| row.unit.clone()),
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

fn set_reason(rows: &mut [MileageReading], reason: &str) {
    for row in rows.iter_mut() {
        row.reason = Some(reason.to_string());
    }
}

fn build_report(run: &Run) -> Value {
    json!({
        "schema": MILEAGE_SCHEMA,
        "schema_version": 1,
        "survey_id": format!("mileage-{}", run.id),
        "application_version": env!("CARGO_PKG_VERSION"),
        "operation": MILEAGE_OPERATION,
        "safety_class": MILEAGE_SAFETY_CLASS,
        "route_validation": if run.synthetic { "SYNTHETIC" } else { "SOURCE_BACKED" },
        "validation": "readings, not a verdict: each row is what one module answered when asked, and a difference is arithmetic over two of those readings",
        "started_unix_ms": run.started_unix_ms,
        "vehicle_context": run.context,
        "adapter": run.adapter,
        "planned": run.entries.len(),
        "asked": run.asked,
        "answered": run.answered,
        "highest": run.highest().map(|(row, number)| json!({
            "ecu_family": row.ecu_family,
            "value": number,
            "unit": row.unit,
        })),
        "readings": run.readings,
        "not_planned": run.refused.iter().map(|(family, reason)| json!({
            "ecu_family": family,
            "reason": reason,
        })).collect::<Vec<_>>(),
    })
}

fn idle(report_available: bool) -> MileageSurveySnapshot {
    MileageSurveySnapshot {
        state: MileageSurveyState::Idle,
        readings: Vec::new(),
        planned: 0,
        asked: 0,
        answered: 0,
        highest: None,
        highest_module: None,
        unit: None,
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
