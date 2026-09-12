//! F13 intake: a tester's session report becomes captured evidence.
//!
//! The rules are ADR-0016's. One report — the bundle the application saves,
//! `jlr-scanner.session-report` version 1 — becomes one F5 manifest of
//! `Captured` source type with `DirectObservation` evidence and
//! `CaptureValidated` records, applicable to the vehicle the report
//! describes and no wider. A module's answer confirms the facts its request
//! was built from, copied from the library so agreement is exact; silence
//! and listen-only captures are recorded as observations and confirm
//! nothing; fault codes are not knowledge. Whatever the tool cannot decide
//! it names in the summary and leaves unwritten.

use app_contracts::{
    DiagnosticErrorCategory, DiagnosticExecutionStage, DiagnosticReport, ModuleReadReport,
    VehicleContextInput,
};
use diagnostic_session::{vehicle_context, KnowledgeLibrary};
use knowledge::{
    sha256_bytes, Applicability, ClaimKey, ContentFingerprint, DimensionConstraint, EntityKind,
    EvidenceClass, EvidenceId, EvidenceRecord, IngestionBatch, KnowledgeEntity, KnowledgeError,
    KnowledgeQuery, KnowledgeRecord, KnowledgeValue, RedistributionStatus, SourceId, SourceLocator,
    SourceRecord, SourceType, ValidationState, YearConstraint, KNOWLEDGE_SCHEMA_VERSION,
};
use serde::Deserialize;
use std::collections::BTreeMap;

pub const SESSION_REPORT_SCHEMA: &str = "jlr-scanner.session-report";
pub const SESSION_REPORT_SCHEMA_VERSION: u32 = 1;
pub const INTAKE_PARSER_ID: &str = "jlr-scanner-report-intake";
pub const INTAKE_PARSER_VERSION: &str = "1.0.0";

/// Observation claims (ADR-0016 §3–5): recorded, never used to resolve a plan.
pub const CAPTURED_READ_CLAIM: &str = "captured_read";
pub const CAPTURED_READ_ATTEMPT_CLAIM: &str = "captured_read_attempt";
pub const CAPTURED_BUS_ACTIVITY_CLAIM: &str = "captured_bus_activity";
pub const CAPTURED_CALIBRATION_ID_CLAIM: &str = "captured_calibration_id";
/// The claim the route bindings and hypotheses use for the adapter route of
/// a bus (`fixtures/knowledge/documented/mongoose_jlr_route_bindings.json`).
pub const BACKEND_ROUTE_CLAIM: &str = "backend_route";

/// The parts of a session report the intake reads. Unknown fields are
/// ignored so a newer report with more in it still yields what this version
/// understands; a different schema is refused.
#[derive(Debug, Deserialize)]
struct SessionReport {
    schema: String,
    schema_version: u32,
    #[serde(default)]
    application_version: Option<String>,
    /// The commit CI built, since build 0.9.1; older reports carry none.
    #[serde(default)]
    application_build: Option<String>,
    /// `bench` or `real` (ADR-0020); a bench report is refused by construction.
    #[serde(default)]
    session_mode: Option<String>,
    #[serde(default)]
    saved_unix_ms: Option<u64>,
    #[serde(default)]
    captures: Vec<CaptureFixture>,
    #[serde(default)]
    module_reads: Vec<ModuleReadReport>,
    #[serde(default)]
    calibration_reads: Vec<DiagnosticReport>,
    /// Live reading (`ADR-0022`): each entry of the set keeps the record of
    /// its last completed request, in the shape a single read leaves.
    #[serde(default)]
    live_read_runs: Vec<LiveReadRun>,
    /// The mileage survey (`ADR-0024`): one record per read made.
    #[serde(default)]
    mileage_surveys: Vec<MileageSurvey>,
    /// The module passport (`ADR-0027`): one record per read made.
    #[serde(default)]
    module_passports: Vec<ModulePassport>,
    /// The configuration read (`ADR-0028`): one record per block read.
    #[serde(default)]
    ccf_reads: Vec<CcfRead>,
}

/// As much of a configuration read as the intake reads: the reads it made.
/// The configuration itself is what a car holds, not evidence about a route,
/// and is not recorded here.
#[derive(Debug, Deserialize)]
struct CcfRead {
    #[serde(default)]
    reads: Vec<ModuleReadReport>,
}

/// As much of a live-read run as the intake reads: the set, each entry with
/// the record of its last completed request. Samples are not evidence of
/// anything the record is not already evidence of.
#[derive(Debug, Deserialize)]
struct LiveReadRun {
    #[serde(default)]
    set: Vec<LiveReadSetEntry>,
}

#[derive(Debug, Deserialize)]
struct LiveReadSetEntry {
    #[serde(default)]
    record: Option<ModuleReadReport>,
}

/// As much of a mileage survey as the intake reads: the reads it made.
#[derive(Debug, Deserialize)]
struct MileageSurvey {
    #[serde(default)]
    reads: Vec<ModuleReadReport>,
}

/// As much of a module passport as the intake reads: the reads it made. The
/// texts themselves — part numbers, serials — are what a module holds, not
/// evidence about the route, so they are not recorded here.
#[derive(Debug, Deserialize)]
struct ModulePassport {
    #[serde(default)]
    reads: Vec<ModuleReadReport>,
}

/// The listen-only capture as the application saves it (a replay fixture).
#[derive(Debug, Deserialize)]
struct CaptureFixture {
    name: String,
    #[serde(default)]
    metadata: CaptureMetadata,
    #[serde(default)]
    frames: Vec<CaptureFrame>,
}

#[derive(Debug, Default, Deserialize)]
struct CaptureMetadata {
    #[serde(default)]
    vehicle_program: Option<String>,
    #[serde(default)]
    model_year: Option<u16>,
    #[serde(default)]
    powertrain: Option<String>,
    #[serde(default)]
    evidence: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CaptureFrame {
    #[serde(default)]
    timestamp_us: u64,
    #[serde(default)]
    route: String,
    id: u32,
    #[serde(default)]
    extended: bool,
}

/// What the intake did with one report.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IntakeSummary {
    /// Facts confirmed by a module's answer, as `entity key` lines.
    pub confirmations: Vec<String>,
    /// Observations recorded without confirming anything.
    pub observations: Vec<String>,
    /// Items the tool could not turn into a record, each with the reason.
    pub skipped: Vec<String>,
}

#[derive(Debug)]
pub struct IntakeOutcome {
    pub batch: IngestionBatch,
    pub summary: IntakeSummary,
}

/// Turn one session report into one captured manifest, copying confirmed
/// values from `library` so that agreement with the data is exact.
pub fn intake(
    report_text: &str,
    report_file_name: &str,
    library: &KnowledgeLibrary,
) -> Result<IntakeOutcome, KnowledgeError> {
    let report: SessionReport = serde_json::from_str(report_text).map_err(|error| {
        KnowledgeError::Parse(format!("session report is not readable: {error}"))
    })?;
    if report.schema != SESSION_REPORT_SCHEMA
        || report.schema_version != SESSION_REPORT_SCHEMA_VERSION
    {
        return Err(KnowledgeError::Parse(format!(
            "expected a {SESSION_REPORT_SCHEMA} version {SESSION_REPORT_SCHEMA_VERSION} report, found '{}' version {}",
            report.schema, report.schema_version
        )));
    }
    // A bench session (ADR-0020) holds synthetic values only: refused here,
    // by construction, whatever else the report says about itself.
    if report.session_mode.as_deref() == Some("bench") {
        return Err(KnowledgeError::Parse(
            "this is a bench session: every value in it is synthetic, so the intake refuses it"
                .into(),
        ));
    }

    let digest = sha256_bytes(report_text.as_bytes());
    let fingerprint = ContentFingerprint::sha256(digest.clone())?;
    let short = &fingerprint.value[..16];
    let source_id = format!("capture-session-{short}");
    let mut programs: Vec<String> = Vec::new();
    let every_read = report
        .module_reads
        .iter()
        .chain(
            report
                .live_read_runs
                .iter()
                .flat_map(|run| run.set.iter().filter_map(|entry| entry.record.as_ref())),
        )
        .chain(
            report
                .mileage_surveys
                .iter()
                .flat_map(|survey| survey.reads.iter()),
        )
        .chain(
            report
                .module_passports
                .iter()
                .flat_map(|passport| passport.reads.iter()),
        )
        .chain(report.ccf_reads.iter().flat_map(|ccf| ccf.reads.iter()));
    for read in every_read {
        let program = read.vehicle.vehicle_program.trim();
        if !program.is_empty() && !programs.iter().any(|known| known == program) {
            programs.push(program.to_string());
        }
    }
    let source = SourceRecord {
        id: SourceId::new(source_id.clone())?,
        title: format!("Tester session report {short}"),
        source_type: SourceType::Captured,
        origin: format!(
            "ProwlOne {}{} session report saved by a tester",
            report.application_version.as_deref().unwrap_or("(version not recorded)"),
            report
                .application_build
                .as_deref()
                .map(|build| format!(" ({build})"))
                .unwrap_or_default()
        ),
        source_locator: report_file_name.to_string(),
        content_fingerprint: Some(fingerprint),
        acquired_on: report.saved_unix_ms.map(date_of_unix_ms),
        declared_vehicle_programs: programs,
        provenance: "F13 intake of a tester's session report (ADR-0016); the report itself is kept by the programme, this manifest holds only derived claims and observations".into(),
        redistribution_status: RedistributionStatus::Permitted,
        notes: None,
    };
    source.validate()?;

    let mut builder = Builder {
        source_id: source_id.clone(),
        evidence: BTreeMap::new(),
        records: BTreeMap::new(),
        summary: IntakeSummary::default(),
    };

    for (index, read) in report.module_reads.iter().enumerate() {
        builder.module_read(index, read, library)?;
    }
    for (index, capture) in report.captures.iter().enumerate() {
        builder.capture(index, capture)?;
    }
    for (index, calibration) in report.calibration_reads.iter().enumerate() {
        builder.calibration(index, calibration)?;
    }
    // A live read and a mileage survey make the same request a module read
    // makes, through the same path, and leave the same record; the record
    // is read the same way, under the name of the place it sat.
    for (run_index, run) in report.live_read_runs.iter().enumerate() {
        for (entry_index, entry) in run.set.iter().enumerate() {
            if let Some(record) = &entry.record {
                builder.read(
                    format!("live_read_runs[{run_index}].set[{entry_index}]"),
                    format!("live.{run_index:02}.{entry_index:03}"),
                    record,
                    library,
                )?;
            }
        }
    }
    for (survey_index, survey) in report.mileage_surveys.iter().enumerate() {
        for (read_index, record) in survey.reads.iter().enumerate() {
            builder.read(
                format!("mileage_surveys[{survey_index}].reads[{read_index}]"),
                format!("mileage.{survey_index:02}.{read_index:03}"),
                record,
                library,
            )?;
        }
    }
    for (passport_index, passport) in report.module_passports.iter().enumerate() {
        for (read_index, record) in passport.reads.iter().enumerate() {
            builder.read(
                format!("module_passports[{passport_index}].reads[{read_index}]"),
                format!("passport.{passport_index:02}.{read_index:03}"),
                record,
                library,
            )?;
        }
    }
    for (ccf_index, ccf) in report.ccf_reads.iter().enumerate() {
        for (read_index, record) in ccf.reads.iter().enumerate() {
            builder.read(
                format!("ccf_reads[{ccf_index}].reads[{read_index}]"),
                format!("ccf.{ccf_index:02}.{read_index:03}"),
                record,
                library,
            )?;
        }
    }

    if builder.records.is_empty() {
        return Err(KnowledgeError::Parse(
            "the report holds nothing this intake can record: no module read, capture, calibration read, live-read run, mileage survey, module passport or configuration read".into(),
        ));
    }
    Ok(IntakeOutcome {
        batch: IngestionBatch {
            schema_version: KNOWLEDGE_SCHEMA_VERSION,
            source,
            evidence: builder.evidence.into_values().collect(),
            records: builder.records.into_values().collect(),
        },
        summary: builder.summary,
    })
}

struct Builder {
    source_id: String,
    evidence: BTreeMap<String, EvidenceRecord>,
    records: BTreeMap<String, KnowledgeRecord>,
    summary: IntakeSummary,
}

impl Builder {
    // One evidence record and one knowledge record per call; the arguments
    // are the record's own fields, and bundling them would only rename them.
    #[allow(clippy::too_many_arguments)]
    fn add(
        &mut self,
        record_id: String,
        entity: KnowledgeEntity,
        key: ClaimKey,
        value: KnowledgeValue,
        applicability: Applicability,
        locator: String,
        excerpt: String,
        timestamp_us: Option<u64>,
    ) -> Result<(), KnowledgeError> {
        entity.validate()?;
        key.validate()?;
        let evidence_id = format!("{}.ev.{record_id}", self.source_id);
        self.evidence.insert(
            evidence_id.clone(),
            EvidenceRecord {
                id: EvidenceId::new(evidence_id.clone())?,
                source_id: SourceId::new(self.source_id.clone())?,
                evidence_class: Some(EvidenceClass::DirectObservation),
                locator: SourceLocator {
                    description: locator,
                    document_page: None,
                    document_section: Some("session report".into()),
                    record_key: Some(entity.id.clone()),
                    capture_timestamp_us: timestamp_us,
                },
                excerpt: Some(excerpt.chars().take(500).collect()),
                notes: None,
            },
        );
        self.records.insert(
            record_id.clone(),
            KnowledgeRecord {
                id: record_id,
                entity,
                key,
                value,
                applicability,
                evidence_ids: vec![EvidenceId::new(evidence_id)?],
                validation_state: ValidationState::CaptureValidated,
            },
        );
        Ok(())
    }

    fn module_read(
        &mut self,
        index: usize,
        read: &ModuleReadReport,
        library: &KnowledgeLibrary,
    ) -> Result<(), KnowledgeError> {
        self.read(
            format!("module_reads[{index}]"),
            format!("read.{index:03}"),
            read,
            library,
        )
    }

    /// One read's record, wherever in the bundle it sat: `locator` names the
    /// place, `stem` makes the record ids unique within the source.
    fn read(
        &mut self,
        locator: String,
        stem: String,
        read: &ModuleReadReport,
        library: &KnowledgeLibrary,
    ) -> Result<(), KnowledgeError> {
        let family = read.ecu_family.trim();
        let what = format!("{locator} {family} {}", read.operation);
        if family.is_empty() {
            self.summary
                .skipped
                .push(format!("{what}: no ECU family named"));
            return Ok(());
        }
        if read.vehicle.vehicle_program.trim().is_empty() {
            self.summary
                .skipped
                .push(format!("{what}: no vehicle programme in the report"));
            return Ok(());
        }
        let applicability = applicability_for(&read.vehicle)?;
        let timestamp = Some(read.timestamp_unix_ms.saturating_mul(1000));
        let base = format!("{}.{stem}", self.source_id);
        let family_entity = KnowledgeEntity {
            kind: EntityKind::EcuFamily,
            id: family.to_string(),
        };
        let exchange = format!(
            "{} {} -> {} : {}",
            read.request_id,
            read.request_payload,
            read.actual_responder.as_deref().unwrap_or("no answer"),
            read.raw_diagnostic_response.as_deref().unwrap_or("-")
        );

        let answered = read.execution_stage == DiagnosticExecutionStage::Completed
            && read.actual_responder.is_some()
            && read.raw_diagnostic_response.is_some()
            && !matches!(
                read.timeout_or_error_category,
                Some(DiagnosticErrorCategory::NoResponseFromEcu)
                    | Some(DiagnosticErrorCategory::UnexpectedResponder)
            );
        if !answered {
            let outcome = match &read.timeout_or_error_category {
                Some(category) => format!("{category:?}"),
                None => "no answer".into(),
            };
            self.add(
                format!("{base}.attempt"),
                family_entity,
                ClaimKey::Custom {
                    name: CAPTURED_READ_ATTEMPT_CLAIM.into(),
                },
                KnowledgeValue::Text {
                    value: format!(
                        "{}: {} on {} ({}) -> {outcome}; {exchange}",
                        read.operation, read.capability, read.backend_route, read.physical_route
                    ),
                },
                applicability,
                locator,
                exchange,
                timestamp,
            )?;
            self.summary
                .observations
                .push(format!("{what}: {outcome}, recorded as an attempt"));
            return Ok(());
        }

        // The module answered. Copy the facts the request was built from,
        // as the library resolves them for this vehicle, so agreement is exact.
        let context = vehicle_context(&read.vehicle);
        let query = KnowledgeQuery::for_vehicle(context).with_ecu_family(family.to_string());
        let result = library.store().query(&query);
        let mut by_key: BTreeMap<String, Vec<&KnowledgeRecord>> = BTreeMap::new();
        for resolved in &result.records {
            let record = &resolved.record;
            if record.entity.kind == EntityKind::EcuFamily && record.entity.id == family {
                by_key
                    .entry(key_name(&record.key))
                    .or_default()
                    .push(record);
            }
        }
        let single_value = |records: Option<&Vec<&KnowledgeRecord>>| -> Option<KnowledgeValue> {
            let records = records?;
            let mut values: Vec<&KnowledgeValue> = records.iter().map(|r| &r.value).collect();
            values.dedup();
            let first = values.first()?;
            values
                .iter()
                .all(|value| value == first)
                .then(|| (*first).clone())
        };

        let positive = read
            .raw_diagnostic_response
            .as_deref()
            .map(|hex| !hex.trim_start().starts_with("7F"))
            .unwrap_or(false);
        let mut confirmed = 0usize;

        // Addressing: the observed responder must be the one the data expects.
        match single_value(by_key.get("diagnostic_addressing")) {
            Some(KnowledgeValue::DiagnosticAddressing {
                request_id,
                response_id,
                functional_request_id,
                can_id_format,
                addressing_mode,
            }) => {
                let observed = parse_hex_id(read.actual_responder.as_deref().unwrap_or(""));
                let expected = parse_hex_id(&read.expected_response_id);
                let value = if observed.is_some() && observed == expected && observed == response_id
                {
                    KnowledgeValue::DiagnosticAddressing {
                        request_id,
                        response_id,
                        functional_request_id,
                        can_id_format,
                        addressing_mode,
                    }
                } else {
                    // The vehicle answered from somewhere else: record what was
                    // seen and let the store report the disagreement.
                    self.summary.skipped.push(format!(
                        "{what}: responder {} is not the expected {}; recorded as observed, which the survey will show as a conflict",
                        read.actual_responder.as_deref().unwrap_or("?"),
                        read.expected_response_id
                    ));
                    KnowledgeValue::DiagnosticAddressing {
                        request_id: parse_hex_id(&read.request_id),
                        response_id: observed,
                        functional_request_id: None,
                        can_id_format,
                        addressing_mode,
                    }
                };
                self.add(
                    format!("{base}.addressing"),
                    family_entity.clone(),
                    ClaimKey::DiagnosticAddressing,
                    value,
                    applicability.clone(),
                    locator.clone(),
                    exchange.clone(),
                    timestamp,
                )?;
                confirmed += 1;
            }
            _ => self.summary.skipped.push(format!(
                "{what}: the library has no single addressing value for {family} on this vehicle; addressing not confirmed"
            )),
        }

        // The module's bus, and the bus's physical route on the adapter.
        match single_value(by_key.get("network_route")) {
            Some(value @ KnowledgeValue::NetworkRoute { .. }) => {
                let logical = match &value {
                    KnowledgeValue::NetworkRoute { logical_name, .. } => logical_name.clone(),
                    _ => unreachable!(),
                };
                self.add(
                    format!("{base}.bus"),
                    family_entity.clone(),
                    ClaimKey::NetworkRoute,
                    value,
                    applicability.clone(),
                    locator.clone(),
                    exchange.clone(),
                    timestamp,
                )?;
                confirmed += 1;
                // The physical route the answer travelled, and the adapter
                // route it was sent on: the library's own records for the
                // bus, hypothesis or documented alike.
                let bus_query = KnowledgeQuery::for_vehicle(vehicle_context(&read.vehicle));
                let bus_result = library.store().query(&bus_query);
                let bus_records: Vec<&KnowledgeRecord> = bus_result
                    .records
                    .iter()
                    .map(|resolved| &resolved.record)
                    .filter(|record| {
                        record.entity.kind == EntityKind::NetworkRoute && record.entity.id == logical
                    })
                    .collect();
                let physical: Vec<&KnowledgeRecord> = bus_records
                    .iter()
                    .copied()
                    .filter(|record| record.key == ClaimKey::NetworkRoute)
                    .collect();
                match single_value(Some(&physical)) {
                    Some(KnowledgeValue::NetworkRoute {
                        logical_name,
                        connector,
                        pins,
                        bitrate_bps,
                    }) if !pins.is_empty() => {
                        if read.physical_route.contains(&pins_text(&pins)) {
                            self.add(
                                format!("{base}.route"),
                                KnowledgeEntity {
                                    kind: EntityKind::NetworkRoute,
                                    id: logical.clone(),
                                },
                                ClaimKey::NetworkRoute,
                                KnowledgeValue::NetworkRoute {
                                    logical_name,
                                    connector,
                                    pins: pins.clone(),
                                    bitrate_bps,
                                },
                                applicability.clone(),
                                locator.clone(),
                                exchange.clone(),
                                timestamp,
                            )?;
                            confirmed += 1;
                        } else {
                            self.summary.skipped.push(format!(
                                "{what}: the report's physical route '{}' does not name pins {}; route not confirmed",
                                read.physical_route,
                                pins_text(&pins)
                            ));
                        }
                    }
                    _ => self.summary.skipped.push(format!(
                        "{what}: the library has no single physical route for bus {logical}; route not confirmed"
                    )),
                }
                let backend_key = ClaimKey::Custom {
                    name: BACKEND_ROUTE_CLAIM.into(),
                };
                let backends: Vec<&KnowledgeRecord> = bus_records
                    .iter()
                    .copied()
                    .filter(|record| record.key == backend_key)
                    .collect();
                match single_value(Some(&backends)) {
                    Some(KnowledgeValue::BackendRoute { backend, route_id }) => {
                        if read.backend_route.trim() == route_id {
                            self.add(
                                format!("{base}.backend"),
                                KnowledgeEntity {
                                    kind: EntityKind::NetworkRoute,
                                    id: logical.clone(),
                                },
                                backend_key,
                                KnowledgeValue::BackendRoute { backend, route_id },
                                applicability.clone(),
                                locator.clone(),
                                exchange.clone(),
                                timestamp,
                            )?;
                            confirmed += 1;
                        } else {
                            self.summary.skipped.push(format!(
                                "{what}: the report's adapter route '{}' is not the library's '{route_id}' for bus {logical}; adapter route not confirmed",
                                read.backend_route
                            ));
                        }
                    }
                    _ => self.summary.skipped.push(format!(
                        "{what}: the library has no single adapter route for bus {logical}; adapter route not confirmed"
                    )),
                }
            }
            _ => self.summary.skipped.push(format!(
                "{what}: the library has no single bus for {family} on this vehicle; bus not confirmed"
            )),
        }

        match single_value(by_key.get("uses_protocol_family")) {
            Some(value @ KnowledgeValue::ProtocolFamily { .. }) => {
                self.add(
                    format!("{base}.protocol"),
                    family_entity.clone(),
                    ClaimKey::UsesProtocolFamily,
                    value,
                    applicability.clone(),
                    locator.clone(),
                    exchange.clone(),
                    timestamp,
                )?;
                confirmed += 1;
            }
            _ => self.summary.skipped.push(format!(
                "{what}: the library has no single protocol for {family} on this vehicle; protocol not confirmed"
            )),
        }

        if positive {
            let capability = read.capability.trim().to_string();
            let key = ClaimKey::SupportsCapability {
                capability: capability.clone(),
            };
            let matching: Vec<&KnowledgeRecord> = result
                .records
                .iter()
                .map(|resolved| &resolved.record)
                .filter(|record| {
                    record.entity.kind == EntityKind::EcuFamily
                        && record.entity.id == family
                        && record.key == key
                })
                .collect();
            match single_value(Some(&matching)) {
                Some(value @ KnowledgeValue::Capability { .. }) => {
                    self.add(
                        format!("{base}.capability"),
                        family_entity.clone(),
                        key,
                        value,
                        applicability.clone(),
                        locator.clone(),
                        exchange.clone(),
                        timestamp,
                    )?;
                    confirmed += 1;
                }
                _ => self.summary.skipped.push(format!(
                    "{what}: the library has no single capability record for '{capability}'; capability not confirmed"
                )),
            }
        }

        // The observation itself, whatever was confirmed.
        self.add(
            format!("{base}.observed"),
            family_entity,
            ClaimKey::Custom {
                name: CAPTURED_READ_CLAIM.into(),
            },
            KnowledgeValue::Text {
                value: format!(
                    "{}: {} on {} ({}) -> {}; {exchange}",
                    read.operation,
                    read.capability,
                    read.backend_route,
                    read.physical_route,
                    if positive {
                        "positive response"
                    } else {
                        "negative response"
                    }
                ),
            },
            applicability,
            locator,
            exchange,
            timestamp,
        )?;
        self.summary.confirmations.push(format!(
            "{what}: {} response from {}; {confirmed} fact(s) confirmed",
            if positive { "positive" } else { "negative" },
            read.actual_responder.as_deref().unwrap_or("?")
        ));
        Ok(())
    }

    fn capture(&mut self, index: usize, capture: &CaptureFixture) -> Result<(), KnowledgeError> {
        let what = format!("captures[{index}] {}", capture.name);
        let Some(program) = capture
            .metadata
            .vehicle_program
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            self.summary
                .skipped
                .push(format!("{what}: no vehicle programme in the capture"));
            return Ok(());
        };
        let route = capture
            .frames
            .first()
            .map(|frame| frame.route.clone())
            .unwrap_or_else(|| "unknown route".into());
        let mut ids: BTreeMap<(bool, u32), usize> = BTreeMap::new();
        for frame in &capture.frames {
            *ids.entry((frame.extended, frame.id)).or_default() += 1;
        }
        let mut top: Vec<_> = ids.iter().collect();
        top.sort_by(|left, right| right.1.cmp(left.1).then(left.0.cmp(right.0)));
        let top_text: Vec<String> = top
            .iter()
            .take(8)
            .map(|((extended, id), count)| {
                if *extended {
                    format!("0x{id:08X}x{count}")
                } else {
                    format!("0x{id:03X}x{count}")
                }
            })
            .collect();
        let mut applicability = Applicability {
            vehicle_program: DimensionConstraint::one_of([program.to_string()])?,
            ..unconstrained()
        };
        if let Some(year) = capture.metadata.model_year {
            applicability.model_year = YearConstraint::Range {
                model_year_from: Some(year),
                model_year_to: Some(year),
            };
        }
        if let Some(powertrain) = capture.metadata.powertrain.as_deref().map(str::trim) {
            if !powertrain.is_empty() {
                applicability.powertrain = DimensionConstraint::one_of([powertrain.to_string()])?;
            }
        }
        let summary = format!(
            "listen-only on {route}: {} frames, {} distinct identifiers; top {}; {}",
            capture.frames.len(),
            ids.len(),
            top_text.join(" "),
            capture
                .metadata
                .evidence
                .as_deref()
                .unwrap_or("no capture notes")
        );
        self.add(
            format!("{}.capture.{index:03}", self.source_id),
            KnowledgeEntity {
                kind: EntityKind::VehicleProgram,
                id: program.to_string(),
            },
            ClaimKey::Custom {
                name: CAPTURED_BUS_ACTIVITY_CLAIM.into(),
            },
            KnowledgeValue::Text {
                value: summary.clone(),
            },
            applicability,
            format!("captures[{index}]"),
            summary,
            capture.frames.first().map(|frame| frame.timestamp_us),
        )?;
        self.summary.observations.push(format!(
            "{what}: {} frames on {route}, recorded as bus activity; binds nothing",
            capture.frames.len()
        ));
        Ok(())
    }

    fn calibration(
        &mut self,
        index: usize,
        report: &DiagnosticReport,
    ) -> Result<(), KnowledgeError> {
        let what = format!("calibration_reads[{index}] {}", report.ecu_target);
        let Some(decoded) = report
            .decoded_result
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            self.summary.observations.push(format!(
                "{what}: no calibration identifier decoded; nothing recorded"
            ));
            return Ok(());
        };
        let profile = &report.selected_vehicle_profile;
        let applicability = Applicability {
            vehicle_program: DimensionConstraint::one_of([profile.vehicle_program.clone()])?,
            model_year: YearConstraint::Range {
                model_year_from: Some(profile.model_year),
                model_year_to: Some(profile.model_year),
            },
            ..unconstrained()
        };
        let exchange = format!(
            "{} {} -> {} : {}",
            report.request_id,
            report.request_payload,
            report.actual_responder.as_deref().unwrap_or("no answer"),
            report.raw_diagnostic_response.as_deref().unwrap_or("-")
        );
        self.add(
            format!("{}.calibration.{index:03}", self.source_id),
            KnowledgeEntity {
                kind: EntityKind::EcuFamily,
                id: report.ecu_target.trim().to_string(),
            },
            ClaimKey::Custom {
                name: CAPTURED_CALIBRATION_ID_CLAIM.into(),
            },
            KnowledgeValue::Text {
                value: format!(
                    "{decoded}; {} on {}; {exchange}",
                    report.protocol, report.physical_route
                ),
            },
            applicability,
            format!("calibration_reads[{index}]"),
            exchange,
            Some(report.timestamp_unix_ms.saturating_mul(1000)),
        )?;
        self.summary
            .observations
            .push(format!("{what}: calibration identifier {decoded} recorded"));
        Ok(())
    }
}

/// Applicability with every dimension `Any`: the dimensions a report does
/// not state are not constraints, unlike the model's `Unknown`, which would
/// make the record apply to nothing.
fn unconstrained() -> Applicability {
    use DimensionConstraint::Any;
    Applicability {
        vehicle_program: Any,
        model_year: YearConstraint::Any,
        architecture_generation: Any,
        ecu_family: Any,
        powertrain: Any,
        variant: Any,
        market: Any,
        diagnostic_implementation: Any,
        other: BTreeMap::new(),
    }
}

fn key_name(key: &ClaimKey) -> String {
    match key {
        ClaimKey::Alias => "alias".into(),
        ClaimKey::UsesProtocolFamily => "uses_protocol_family".into(),
        ClaimKey::NetworkRoute => "network_route".into(),
        ClaimKey::DiagnosticAddressing => "diagnostic_addressing".into(),
        ClaimKey::SupportsCapability { capability } => format!("supports_capability:{capability}"),
        ClaimKey::IdentifierDefinition { namespace } => {
            format!("identifier_definition:{namespace}")
        }
        ClaimKey::ParameterDefinition { parameter } => format!("parameter_definition:{parameter}"),
        ClaimKey::Custom { name } => format!("custom:{name}"),
    }
}

/// The report's vehicle context, as the applicability of what it proved:
/// that programme, that model year, that engine and marker when stated.
fn applicability_for(vehicle: &VehicleContextInput) -> Result<Applicability, KnowledgeError> {
    let mut applicability = Applicability {
        vehicle_program: DimensionConstraint::one_of([vehicle.vehicle_program.trim().to_string()])?,
        ..unconstrained()
    };
    if let Some(year) = vehicle.model_year {
        applicability.model_year = YearConstraint::Range {
            model_year_from: Some(year),
            model_year_to: Some(year),
        };
    }
    if let Some(powertrain) = vehicle.powertrain.as_deref().map(str::trim) {
        if !powertrain.is_empty() {
            applicability.powertrain = DimensionConstraint::one_of([powertrain.to_string()])?;
        }
    }
    if let Some(marker) = vehicle.year_breakpoint.as_deref().map(str::trim) {
        if !marker.is_empty() {
            applicability.other.insert(
                diagnostic_session::SDD_YEAR_BREAKPOINT_DIMENSION.to_string(),
                DimensionConstraint::one_of([marker.to_string()])?,
            );
        }
    }
    Ok(applicability)
}

fn parse_hex_id(text: &str) -> Option<u32> {
    let trimmed = text.trim();
    let digits = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    u32::from_str_radix(digits, 16).ok()
}

fn pins_text(pins: &[u8]) -> String {
    pins.iter().map(u8::to_string).collect::<Vec<_>>().join("/")
}

/// `YYYY-MM-DD` of a Unix millisecond timestamp, civil calendar (Howard
/// Hinnant's algorithm), UTC.
fn date_of_unix_ms(ms: u64) -> String {
    let days = (ms / 86_400_000) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_milliseconds_become_a_civil_date() {
        assert_eq!(date_of_unix_ms(0), "1970-01-01");
        assert_eq!(date_of_unix_ms(1_788_480_000_000), "2026-09-04");
    }

    #[test]
    fn identifiers_parse_with_or_without_a_prefix() {
        assert_eq!(parse_hex_id("0x7E8"), Some(0x7E8));
        assert_eq!(parse_hex_id("7e8"), Some(0x7E8));
        assert_eq!(parse_hex_id("no answer"), None);
    }
}
