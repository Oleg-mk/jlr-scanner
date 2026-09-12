//! The car configuration file, read as SDD reads it (ADR-0028).
//!
//! The configuration is where a car says what it is fitted with and how it
//! is set. SDD's data names the module that keeps the master copy and the
//! modules holding copies, says which identifier answers which block at
//! which offset, and describes every byte and bit. This service plans one
//! `ReadDataByIdentifier` per module and identifier, slices each block out
//! of the answer, and decodes it with the layout the library carries: an
//! option's text for an enumeration or a boolean, a number, a string, digits,
//! bytes. A value is what the type says; no word judges it. A copy's block is
//! compared with the master's parameter by parameter, and a difference is two
//! readings side by side.
//!
//! The shape is the passport's: the shell owns the plan, the interface steps
//! and can stop between any two reads, every read leaves the record a single
//! read leaves, and the whole run joins the session bundle as `ccf_reads`.

use crate::module_read_service::{ModuleReadService, PreparedModuleRead};
use crate::read_record::{read_record, ReadIdentity, ReadOutcome};
use app_contracts::{
    AdapterInfo, CcfBlockRead, CcfDifference, CcfReadSnapshot, CcfReadState, CcfReading,
    DiagnosticError, DiagnosticErrorCategory, DiagnosticExecutionStage, ModuleReadKind,
    ModuleReadReport, ModuleReadRequest, ModuleReadState, VehicleContextInput,
};
use diagnostic_session::ccf::{decode, CcfBlockRef, CcfParameter, CcfValue};
use diagnostic_session::{vehicle_context, KnowledgeLibrary};
use mongoose_jlr::MongooseUdsReadResult;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uds_execution::{decode_response, PreparedUdsTransaction, UdsReadOutcome};

/// How long a module is given to answer a block; the largest is 784 bytes
/// over many consecutive frames.
pub const CCF_READ_TIMEOUT: Duration = Duration::from_secs(5);
pub const CCF_SCHEMA: &str = "prowlone.ccf-read";
pub const CCF_OPERATION: &str = "CCF_READ";
pub const CCF_SAFETY_CLASS: &str = "READ_ONLY";

static RUN_COUNTER: AtomicU64 = AtomicU64::new(1);

/// One read the run still owes, with the transaction prepared at planning.
pub struct CcfDue {
    pub index: usize,
    pub transaction: PreparedUdsTransaction,
}

struct Entry {
    ecu_family: String,
    role: String,
    identifier: u16,
    blocks: Vec<CcfBlockRef>,
    route_validation: String,
    transaction: PreparedUdsTransaction,
    done: bool,
}

struct Run {
    id: u64,
    state: CcfReadState,
    started_unix_ms: u128,
    context: VehicleContextInput,
    adapter: Option<AdapterInfo>,
    scheme: Option<String>,
    master: Option<String>,
    layout: Vec<CcfParameter>,
    entries: Vec<Entry>,
    block_reads: Vec<CcfBlockRead>,
    /// The bytes of each block each module answered with.
    blocks: BTreeMap<(String, String), Vec<u8>>,
    /// One record per read made, answered or not, in the shape a single
    /// read leaves — what the intake turns into evidence.
    records: Vec<ModuleReadReport>,
    /// Modules the data names but whose route could not be planned.
    refused: Vec<(String, String)>,
    asked: u32,
    answered: u32,
    synthetic: bool,
    error: Option<DiagnosticError>,
}

#[derive(Default)]
pub struct CcfService {
    run: Option<Run>,
    last_report: Option<Value>,
}

impl CcfService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> CcfReadSnapshot {
        match &self.run {
            Some(run) => run.snapshot(self.last_report.is_some()),
            None => idle(self.last_report.is_some()),
        }
    }

    /// No run begins, and the reason is the real one — no adapter — not a
    /// claim about the data.
    pub fn refuse(&mut self, error: DiagnosticError) -> CcfReadSnapshot {
        self.run = None;
        let mut snapshot = idle(self.last_report.is_some());
        snapshot.state = CcfReadState::Finished;
        snapshot.error = Some(error);
        snapshot
    }

    /// Plan the run: the keeper of the master copy first, then each module
    /// holding a copy; one read per module and identifier, however many
    /// blocks the identifier carries. A paged (VDF) configuration is refused
    /// with the reason; a car the data describes no configuration for, too.
    pub fn start(
        &mut self,
        library: &KnowledgeLibrary,
        adapter: Option<&AdapterInfo>,
        context: &VehicleContextInput,
    ) -> CcfReadSnapshot {
        let resolved = vehicle_context(context);
        let scheme = library.ccf_scheme(&resolved);
        let sources = library.ccf_sources(&resolved);
        let layout = library.ccf_layout(&resolved);
        let master = sources
            .iter()
            .find(|(_, role)| role == "sync")
            .map(|(module, _)| module.clone());

        let mut entries: Vec<Entry> = Vec::new();
        let mut refused = Vec::new();
        let mut error = None;
        if scheme.as_deref() == Some("vdf") {
            error = Some(DiagnosticError {
                category: DiagnosticErrorCategory::UnsupportedVehicleProfile,
                message: "This vehicle's configuration is paged; this version does not read it"
                    .into(),
                technical_details: Some(
                    "SDD addresses the CCF block of this car through VDF blocks (0xEE00 with vdf_type and vdf_offset); the request sequence is not established and the read is not attempted (ADR-0028)"
                        .into(),
                ),
                stage: DiagnosticExecutionStage::Preparation,
            });
        } else if sources.is_empty() || layout.is_empty() {
            error = Some(DiagnosticError {
                category: DiagnosticErrorCategory::UnsupportedVehicleProfile,
                message: "The loaded data describes no configuration for this vehicle".into(),
                technical_details: Some(
                    "no CCF_DATA document applies to the described programme and model year".into(),
                ),
                stage: DiagnosticExecutionStage::Preparation,
            });
        } else {
            for (module, role) in &sources {
                let blocks = library.ccf_blocks(&resolved, module);
                if blocks.is_empty() {
                    refused.push((
                        module.clone(),
                        "the data names this module as a holder of the configuration but declares no readable block for it".into(),
                    ));
                    continue;
                }
                for (identifier, refs) in blocks {
                    let request = ModuleReadRequest {
                        ecu_family: module.clone(),
                        kind: ModuleReadKind::Identifier,
                        identifier: Some(format!("0x{identifier:04X}")),
                        context: context.clone(),
                    };
                    match ModuleReadService::prepare(library, &request) {
                        Ok(prepared) => {
                            entries.push(Entry::new(module, role, identifier, refs, prepared))
                        }
                        Err(error) => {
                            let reason = error.technical_details.unwrap_or(error.message);
                            if !refused
                                .iter()
                                .any(|(known, _): &(String, String)| known == module)
                            {
                                refused.push((module.clone(), reason));
                            }
                        }
                    }
                }
            }
            if entries.is_empty() {
                error = Some(DiagnosticError {
                    category: DiagnosticErrorCategory::UnsupportedVehicleProfile,
                    message: "No module holding the configuration can be reached".into(),
                    technical_details: Some(
                        refused
                            .iter()
                            .map(|(module, reason)| format!("{module}: {reason}"))
                            .collect::<Vec<_>>()
                            .join("; "),
                    ),
                    stage: DiagnosticExecutionStage::Preparation,
                });
            }
        }

        let planned = !entries.is_empty();
        self.run = Some(Run {
            id: RUN_COUNTER.fetch_add(1, Ordering::Relaxed),
            state: if planned {
                CcfReadState::Running
            } else {
                CcfReadState::Finished
            },
            started_unix_ms: unix_ms(),
            context: context.clone(),
            adapter: adapter.cloned(),
            scheme,
            master,
            layout,
            entries,
            block_reads: Vec::new(),
            blocks: BTreeMap::new(),
            records: Vec::new(),
            refused,
            asked: 0,
            answered: 0,
            synthetic: false,
            error,
        });
        self.last_report = None;
        self.rebuild_report();
        self.snapshot()
    }

    /// The next read the run owes, or `None` when it owes none.
    pub fn next_due(&mut self) -> Option<CcfDue> {
        let run = self.run.as_mut()?;
        if !matches!(run.state, CcfReadState::Running) {
            return None;
        }
        let index = run.entries.iter().position(|entry| !entry.done)?;
        run.entries[index].done = true;
        run.asked += 1;
        Some(CcfDue {
            index,
            transaction: run.entries[index].transaction.clone(),
        })
    }

    /// Record what one module answered for one identifier, or that it did
    /// not; slice the blocks out of the answer.
    pub fn record(
        &mut self,
        index: usize,
        result: Result<MongooseUdsReadResult, DiagnosticError>,
    ) -> CcfReadSnapshot {
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
    pub fn finish(&mut self) -> CcfReadSnapshot {
        if let Some(run) = self.run.as_mut() {
            run.finish();
        }
        self.rebuild_report();
        self.snapshot()
    }

    /// The bench answered this run (ADR-0020): every reading is synthetic.
    pub fn mark_synthetic(&mut self) -> CcfReadSnapshot {
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
            .is_some_and(|run| matches!(run.state, CcfReadState::Finished));
        if !finished {
            return;
        }
        self.last_report = self.run.as_ref().map(build_report);
    }

    pub fn report_json(&self) -> Result<String, String> {
        let report = self
            .last_report
            .as_ref()
            .ok_or_else(|| "No configuration read has been recorded yet".to_owned())?;
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
    fn new(
        module: &str,
        role: &str,
        identifier: u16,
        blocks: Vec<CcfBlockRef>,
        prepared: PreparedModuleRead,
    ) -> Self {
        Self {
            ecu_family: module.to_string(),
            role: role.to_string(),
            identifier,
            blocks,
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
        let identity = ReadIdentity {
            context: &self.context,
            ecu_family: &entry.ecu_family,
            operation: CCF_OPERATION,
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
        self.records
            .push(read_record(identity, &entry.transaction, outcome, None));

        let mut row = CcfBlockRead {
            ecu_family: entry.ecu_family.clone(),
            role: entry.role.clone(),
            identifier: format!("0x{:04X}", entry.identifier),
            blocks: entry
                .blocks
                .iter()
                .map(|block| block.block.clone())
                .collect(),
            state: ModuleReadState::Failed,
            bytes: None,
            route_validation: entry.route_validation.clone(),
            raw_response_hex: None,
            negative_response: None,
            reason: None,
        };
        let mut sliced: Vec<((String, String), Vec<u8>)> = Vec::new();
        match result {
            Ok(raw) => {
                row.raw_response_hex = Some(hex(&raw.raw_diagnostic_response));
                match decode_response(&entry.transaction, &raw.raw_diagnostic_response) {
                    Ok(Some(UdsReadOutcome::DataByIdentifier { data, .. })) => {
                        row.state = ModuleReadState::Succeeded;
                        row.bytes = Some(data.len() as u32);
                        let mut short = Vec::new();
                        for block in &entry.blocks {
                            let end = block.offset + block.length;
                            if data.len() >= end {
                                sliced.push((
                                    (entry.ecu_family.clone(), block.block.clone()),
                                    data[block.offset..end].to_vec(),
                                ));
                            } else {
                                short.push(format!(
                                    "{} needs {} byte(s) at offset {}",
                                    block.block, block.length, block.offset
                                ));
                            }
                        }
                        if !short.is_empty() {
                            row.reason = Some(format!(
                                "the answer holds {} byte(s); {}",
                                data.len(),
                                short.join(", ")
                            ));
                        }
                        self.answered += 1;
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
        self.block_reads.push(row);
        for (key, bytes) in sliced {
            self.blocks.insert(key, bytes);
        }
    }

    fn finish(&mut self) {
        self.state = CcfReadState::Finished;
    }

    /// The master's configuration, decoded parameter by parameter.
    fn readings(&self) -> Vec<CcfReading> {
        let Some(master) = &self.master else {
            return Vec::new();
        };
        self.readings_of(master, "sync")
    }

    fn readings_of(&self, module: &str, role: &str) -> Vec<CcfReading> {
        let mut rows = Vec::new();
        for parameter in &self.layout {
            let Some(block) = self
                .blocks
                .get(&(module.to_string(), parameter.block.clone()))
            else {
                continue;
            };
            rows.push(reading(module, role, parameter, &decode(parameter, block)));
        }
        rows
    }

    /// Where a copy holds a parameter differently from the master: two
    /// readings side by side, nothing said about which is right.
    fn differences(&self) -> Vec<CcfDifference> {
        let Some(master) = &self.master else {
            return Vec::new();
        };
        let mut found = Vec::new();
        let copies: Vec<&str> = self
            .entries
            .iter()
            .map(|entry| entry.ecu_family.as_str())
            .filter(|module| module != master)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        for parameter in &self.layout {
            let Some(master_block) = self.blocks.get(&(master.clone(), parameter.block.clone()))
            else {
                continue;
            };
            let master_value = decode(parameter, master_block);
            for copy in &copies {
                let Some(copy_block) = self
                    .blocks
                    .get(&(copy.to_string(), parameter.block.clone()))
                else {
                    continue;
                };
                let copy_value = decode(parameter, copy_block);
                if canonical(&master_value) != canonical(&copy_value) {
                    found.push(CcfDifference {
                        block: parameter.block.clone(),
                        parameter: parameter.name.clone(),
                        master_module: master.clone(),
                        master_value: canonical(&master_value),
                        copy_module: copy.to_string(),
                        copy_value: canonical(&copy_value),
                    });
                }
            }
        }
        found
    }

    fn snapshot(&self, report_available: bool) -> CcfReadSnapshot {
        let readings = self.readings();
        let hidden = readings.iter().filter(|row| !row.display).count() as u32;
        CcfReadSnapshot {
            state: self.state,
            scheme: self.scheme.clone(),
            master_module: self.master.clone(),
            reads: self
                .block_reads
                .iter()
                .cloned()
                .map(|mut row| {
                    if self.synthetic {
                        row.route_validation = "SYNTHETIC".into();
                    }
                    row
                })
                .collect(),
            readings,
            differences: self.differences(),
            planned: self.entries.len() as u32,
            asked: self.asked,
            answered: self.answered,
            hidden,
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

/// One row of the configuration as a module holds it.
fn reading(module: &str, role: &str, parameter: &CcfParameter, value: &CcfValue) -> CcfReading {
    CcfReading {
        ecu_family: module.to_string(),
        role: role.to_string(),
        block: parameter.block.clone(),
        parameter: parameter.name.clone(),
        group: parameter.group.clone(),
        group_title_en: parameter.group_title_en.clone(),
        group_title_ru: parameter.group_title_ru.clone(),
        title_en: parameter.title_en.clone(),
        title_ru: parameter.title_ru.clone(),
        kind: parameter.kind.clone(),
        display: parameter.display,
        scope: parameter.scope.clone(),
        value_en: value.text_en.clone(),
        value_ru: value.text_ru.clone(),
        option_name: value.option_name.clone(),
        option_code: value.option_code.clone(),
        raw: value.raw,
        hex: value.hex.clone(),
        note: value.note.clone(),
    }
}

/// The one string two readings are compared by: the option's name, else the
/// text, else the bytes.
fn canonical(value: &CcfValue) -> Option<String> {
    value
        .option_name
        .clone()
        .or_else(|| value.text_en.clone())
        .or_else(|| value.hex.clone())
}

fn build_report(run: &Run) -> Value {
    let mut copies: Vec<CcfReading> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for entry in &run.entries {
        if Some(&entry.ecu_family) == run.master.as_ref() || !seen.insert(entry.ecu_family.clone())
        {
            continue;
        }
        copies.extend(run.readings_of(&entry.ecu_family, &entry.role));
    }
    json!({
        "schema": CCF_SCHEMA,
        "schema_version": 1,
        "ccf_read_id": format!("ccf-{}", run.id),
        "application_version": env!("CARGO_PKG_VERSION"),
        "operation": CCF_OPERATION,
        "safety_class": CCF_SAFETY_CLASS,
        "route_validation": if run.synthetic { "SYNTHETIC" } else { "SOURCE_BACKED" },
        "validation": "the configuration as the modules hold it, decoded with SDD's own layout; every value is what its type says, a difference between copies is two readings side by side, and nothing is judged",
        "started_unix_ms": run.started_unix_ms,
        "vehicle_context": run.context,
        "adapter": run.adapter,
        "scheme": run.scheme,
        "master_module": run.master,
        "planned": run.entries.len(),
        "asked": run.asked,
        "answered": run.answered,
        "block_reads": run.block_reads.iter().cloned().map(|mut row| {
            if run.synthetic {
                row.route_validation = "SYNTHETIC".into();
            }
            row
        }).collect::<Vec<_>>(),
        "readings": run.readings(),
        "copy_readings": copies,
        "differences": run.differences(),
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

fn idle(report_available: bool) -> CcfReadSnapshot {
    CcfReadSnapshot {
        state: CcfReadState::Idle,
        scheme: None,
        master_module: None,
        reads: Vec::new(),
        readings: Vec::new(),
        differences: Vec::new(),
        planned: 0,
        asked: 0,
        answered: 0,
        hidden: 0,
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
