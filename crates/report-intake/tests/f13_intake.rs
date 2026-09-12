//! F13 intake golden tests, on synthetic data only.
//!
//! A synthetic module sits on a bus whose adapter route is one of the
//! built-in research hypotheses. Before any report, the survey shows it as a
//! hypothesis. A session report in which the module answered becomes a
//! captured manifest; loaded with the rest, it makes the route reachable and
//! capture-validated. Silence, captures and an unexpected responder are
//! recorded as ADR-0016 says, and nothing else.

use app_contracts::{
    DiagnosticErrorCategory, DiagnosticExecutionStage, ModuleReadReport, RouteStatus,
    VehicleContextInput,
};
use diagnostic_session::KnowledgeLibrary;
use knowledge::{
    sha256_bytes, ContentFingerprint, IngestionAdapter, RedistributionStatus, SourceId,
    SourceRecord, SourceType, ValidationState,
};
use report_intake::{intake, CAPTURED_BUS_ACTIVITY_CLAIM, CAPTURED_READ_ATTEMPT_CLAIM};
use sdd_ingest::{ConverterCatalogue, DidFormattingAdapter, ModelYearTimeline, PlatformAdapter};

const PLATFORM: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_platform.xml");
const DIDS: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_did_formatting.xml");
const CONVERTER: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_converter.xml");
const CONVERTER_KM: &str =
    include_str!("../../../fixtures/knowledge/synthetic/f9_converter_km.xml");

/// A synthetic module on `PT_HSCAN`, the bus ADR-0015 binds to hs-can only
/// as a hypothesis. Written the way the platform adapter writes its records.
const RELAY_MODULE: &str = r#"{
  "schema_version": 1,
  "source": {
    "id": "f13-synthetic-relay-module",
    "title": "F13 synthetic module on a hypothesised bus",
    "source_type": "synthetic",
    "origin": "F13 golden test",
    "source_locator": "crates/report-intake/tests/f13_intake.rs",
    "content_fingerprint": null,
    "acquired_on": null,
    "declared_vehicle_programs": ["SYNTHA"],
    "provenance": "Synthetic fixture; not JLR evidence and not promotable",
    "redistribution_status": "permitted",
    "notes": null
  },
  "evidence": [
    {
      "id": "f13-relay.ev",
      "source_id": "f13-synthetic-relay-module",
      "evidence_class": "synthetic_test",
      "locator": { "description": "synthetic", "document_page": null, "document_section": null, "record_key": "RELAYMOD", "capture_timestamp_us": null },
      "excerpt": null,
      "notes": null
    }
  ],
  "records": [
    {
      "id": "f13-relay.addressing",
      "entity": { "kind": "ecu_family", "id": "RELAYMOD" },
      "key": { "kind": "diagnostic_addressing" },
      "value": { "kind": "diagnostic_addressing", "request_id": 1830, "response_id": 1838, "functional_request_id": null, "can_id_format": "standard11_bit", "addressing_mode": "normal" },
      "applicability": APPLICABILITY,
      "evidence_ids": ["f13-relay.ev"],
      "validation_state": "unverified"
    },
    {
      "id": "f13-relay.bus",
      "entity": { "kind": "ecu_family", "id": "RELAYMOD" },
      "key": { "kind": "network_route" },
      "value": { "kind": "network_route", "logical_name": "PT_HSCAN", "connector": null, "pins": [], "bitrate_bps": null },
      "applicability": APPLICABILITY,
      "evidence_ids": ["f13-relay.ev"],
      "validation_state": "unverified"
    },
    {
      "id": "f13-relay.network",
      "entity": { "kind": "ecu_family", "id": "RELAYMOD" },
      "key": { "kind": "custom", "name": "sdd_network" },
      "value": { "kind": "text", "value": "PT_HSCAN" },
      "applicability": APPLICABILITY,
      "evidence_ids": ["f13-relay.ev"],
      "validation_state": "unverified"
    },
    {
      "id": "f13-relay.protocol",
      "entity": { "kind": "ecu_family", "id": "RELAYMOD" },
      "key": { "kind": "uses_protocol_family" },
      "value": { "kind": "protocol_family", "name": "ISO14229" },
      "applicability": APPLICABILITY,
      "evidence_ids": ["f13-relay.ev"],
      "validation_state": "unverified"
    },
    {
      "id": "f13-relay.capability",
      "entity": { "kind": "ecu_family", "id": "RELAYMOD" },
      "key": { "kind": "supports_capability", "capability": "uds.service19.read_dtc_information.read_only" },
      "value": { "kind": "capability", "name": "uds.service19.read_dtc_information.read_only", "supported": true, "safety_class": "read_only" },
      "applicability": APPLICABILITY,
      "evidence_ids": ["f13-relay.ev"],
      "validation_state": "unverified"
    }
  ]
}"#;

const APPLICABILITY: &str = r#"{
      "vehicle_program": { "state": "one_of", "values": ["SYNTHA"] },
      "model_year": { "state": "range", "model_year_from": 2010, "model_year_to": 2011 },
      "architecture_generation": { "state": "any" },
      "ecu_family": { "state": "any" },
      "powertrain": { "state": "any" },
      "variant": { "state": "any" },
      "market": { "state": "any" },
      "diagnostic_implementation": { "state": "any" },
      "other": { "sdd_year_breakpoint": { "state": "one_of", "values": ["MY10"] } }
    }"#;

fn synthetic_source(id: &str, text: &str) -> SourceRecord {
    SourceRecord {
        id: SourceId::new(id).unwrap(),
        title: format!("F13 fixture {id}"),
        source_type: SourceType::Synthetic,
        origin: "F13 golden test".into(),
        source_locator: format!("fixtures/knowledge/synthetic/{id}.xml"),
        content_fingerprint: Some(
            ContentFingerprint::sha256(sha256_bytes(text.as_bytes())).unwrap(),
        ),
        acquired_on: None,
        declared_vehicle_programs: vec![],
        provenance: "Synthetic fixture reproducing the SDD element shape only".into(),
        redistribution_status: RedistributionStatus::Permitted,
        notes: None,
    }
}

fn base_manifests() -> Vec<(String, String)> {
    let mut timeline = ModelYearTimeline::new();
    for marker in ["MY08", "MY10", "MY12"] {
        timeline.observe("SYNTHA", marker).unwrap();
    }
    let platform = PlatformAdapter::new(synthetic_source("f13-plat", PLATFORM))
        .unwrap()
        .with_timeline(timeline.clone());
    let mut converters = ConverterCatalogue::new();
    converters.insert_from_xml(CONVERTER).unwrap();
    converters.insert_from_xml(CONVERTER_KM).unwrap();
    let dids = DidFormattingAdapter::new(synthetic_source("f13-did", DIDS), converters)
        .unwrap()
        .with_timeline(timeline);
    vec![
        (
            "platform.json".into(),
            serde_json::to_string(&platform.parse(PLATFORM).unwrap()).unwrap(),
        ),
        (
            "did.json".into(),
            serde_json::to_string(&dids.parse(DIDS).unwrap()).unwrap(),
        ),
        (
            "relay.json".into(),
            RELAY_MODULE.replace("APPLICABILITY", APPLICABILITY),
        ),
    ]
}

fn library(extra: &[(String, String)]) -> KnowledgeLibrary {
    let mut manifests = base_manifests();
    manifests.extend(extra.iter().cloned());
    KnowledgeLibrary::from_manifests(
        manifests
            .iter()
            .map(|(name, text)| (name.as_str(), text.as_str())),
    )
}

fn vehicle() -> VehicleContextInput {
    VehicleContextInput {
        vehicle_program: "SYNTHA".into(),
        model_year: Some(2010),
        powertrain: None,
        variant: None,
        market: None,
        year_breakpoint: Some("MY10".into()),
    }
}

fn read_report(responder: Option<&str>, response: Option<&str>) -> ModuleReadReport {
    ModuleReadReport {
        schema_version: 1,
        application_version: "0.0.0-test".into(),
        timestamp_unix_ms: 1_788_480_000_000,
        session_id: "f13-test".into(),
        execution_source: "mongoose-jlr".into(),
        adapter_model: "MongoosePro JLR".into(),
        usb_vid: "18E1".into(),
        usb_pid: "0104".into(),
        adapter_board_info_result: "0x8109".into(),
        vehicle: vehicle(),
        ecu_family: "RELAYMOD".into(),
        operation: "Read confirmed fault codes".into(),
        logical_bus: "PT_HSCAN".into(),
        backend_route: "hs-can".into(),
        physical_route: "J1962 pins 6/14".into(),
        bitrate_bps: Some(500_000),
        route_validation: "UNVERIFIED".into(),
        protocol: "ISO14229".into(),
        request_id: "0x726".into(),
        expected_response_id: "0x72E".into(),
        capability: "uds.service19.read_dtc_information.read_only".into(),
        request_payload: "19 02 08".into(),
        actual_responder: responder.map(str::to_string),
        raw_diagnostic_response: response.map(str::to_string),
        decoded_result: response.map(|_| "no confirmed fault codes".to_string()),
        pending_responses: 0,
        timeout_or_error_category: if responder.is_none() {
            Some(DiagnosticErrorCategory::NoResponseFromEcu)
        } else {
            None
        },
        execution_stage: if responder.is_none() {
            DiagnosticExecutionStage::ResponseReception
        } else {
            DiagnosticExecutionStage::Completed
        },
    }
}

fn session_report(reads: Vec<ModuleReadReport>, with_capture: bool) -> String {
    session_bundle(
        reads,
        with_capture,
        serde_json::json!([]),
        serde_json::json!([]),
        serde_json::json!([]),
    )
}

/// A bundle with live-read runs, mileage surveys and module passports in it
/// as the shell writes them (ADR-0022, ADR-0024, ADR-0027): a run's set
/// entries each carry the record of their last completed request; a survey
/// and a passport carry their reads.
fn session_bundle(
    reads: Vec<ModuleReadReport>,
    with_capture: bool,
    live_read_runs: serde_json::Value,
    mileage_surveys: serde_json::Value,
    module_passports: serde_json::Value,
) -> String {
    session_bundle_with_ccf(
        reads,
        with_capture,
        live_read_runs,
        mileage_surveys,
        module_passports,
        serde_json::json!([]),
    )
}

fn session_bundle_with_ccf(
    reads: Vec<ModuleReadReport>,
    with_capture: bool,
    live_read_runs: serde_json::Value,
    mileage_surveys: serde_json::Value,
    module_passports: serde_json::Value,
    ccf_reads: serde_json::Value,
) -> String {
    let captures = if with_capture {
        serde_json::json!([{
            "schema_version": 1,
            "fixture_class": "captured",
            "name": "capture-hs-can-1",
            "metadata": { "vehicle_program": "SYNTHA", "model_year": 2010, "evidence": "listen-only capture on route hs-can" },
            "frames": [
                { "timestamp_us": 10, "route": "hs-can", "id": 0x7E8, "extended": false, "dlc": 8, "data": [1,2,3,4,5,6,7,8] },
                { "timestamp_us": 20, "route": "hs-can", "id": 0x7E8, "extended": false, "dlc": 8, "data": [1,2,3,4,5,6,7,8] },
                { "timestamp_us": 30, "route": "hs-can", "id": 0x18DAF110u32, "extended": true, "dlc": 8, "data": [0,0,0,0,0,0,0,0] }
            ]
        }])
    } else {
        serde_json::json!([])
    };
    serde_json::json!({
        "schema": "jlr-scanner.session-report",
        "schema_version": 1,
        "application_version": "0.0.0-test",
        "session_started_unix_ms": 1_788_479_000_000u64,
        "saved_unix_ms": 1_788_480_000_000u64,
        "validation": "session bundle",
        "adapter": null,
        "library": null,
        "survey": null,
        "captures": captures,
        "module_reads": reads,
        "calibration_reads": [],
        "live_read_runs": live_read_runs,
        "mileage_surveys": mileage_surveys,
        "module_passports": module_passports,
        "ccf_reads": ccf_reads
    })
    .to_string()
}

/// A configuration read (ADR-0028) is read like the passport: its block
/// reads are module reads and confirm what one read confirms; the decoded
/// configuration is not evidence and is not recorded.
#[test]
fn a_configuration_read_confirms_what_one_read_confirms_and_records_no_value() {
    let before = library(&[]);
    let answered = read_report(Some("0x72E"), Some("59 02 FF"));
    let bundle = session_bundle_with_ccf(
        vec![],
        false,
        serde_json::json!([]),
        serde_json::json!([]),
        serde_json::json!([]),
        serde_json::json!([{
            "schema": "prowlone.ccf-read",
            "master_module": "RELAYMOD",
            "reads": [ answered ],
            "readings": [ { "parameter": "PARAM_SYNTH_BRAND", "valueEn": "Alpha" } ]
        }]),
    );
    let outcome = intake(&bundle, "session.json", &before).unwrap();
    assert_eq!(outcome.batch.records.len(), 7, "{:#?}", outcome.summary);
    assert!(outcome
        .summary
        .confirmations
        .iter()
        .any(|line| line.starts_with("ccf_reads[0].reads[0] RELAYMOD")));
    let manifest = serde_json::to_string(&outcome.batch).unwrap();
    assert!(manifest.contains(".ccf.00.000."));
    assert!(
        !manifest.contains("PARAM_SYNTH_BRAND") && !manifest.contains("Alpha"),
        "a configuration value is not evidence about a route"
    );
    let after = library(&[("captured.json".into(), manifest)]);
    assert_eq!(
        status_of(&after, "RELAYMOD"),
        (RouteStatus::Reachable, "CAPTURE_VALIDATED".into())
    );
}

fn status_of(library: &KnowledgeLibrary, family: &str) -> (RouteStatus, String) {
    let survey = library.survey(&vehicle());
    let module = survey
        .modules
        .iter()
        .find(|module| module.ecu_family == family)
        .unwrap_or_else(|| panic!("{family} surveyed"));
    (module.dtc_read.status, module.route_validation.clone())
}

#[test]
fn an_answered_read_turns_a_hypothesised_route_into_a_capture_validated_one() {
    let before = library(&[]);
    assert_eq!(
        status_of(&before, "RELAYMOD"),
        (RouteStatus::Hypothesis, "UNVERIFIED".into())
    );

    let report = session_report(vec![read_report(Some("0x72E"), Some("59 02 FF"))], false);
    let outcome = intake(&report, "session.json", &before).unwrap();
    let batch = &outcome.batch;
    assert_eq!(batch.source.source_type, SourceType::Captured);
    assert_eq!(
        batch.source.declared_vehicle_programs,
        vec!["SYNTHA".to_string()]
    );
    assert_eq!(batch.source.acquired_on.as_deref(), Some("2026-09-04"));
    assert!(batch
        .records
        .iter()
        .all(|record| record.validation_state == ValidationState::CaptureValidated));
    // Addressing, bus, physical route, adapter route, protocol, capability,
    // and the observation.
    assert_eq!(batch.records.len(), 7);
    assert_eq!(outcome.summary.confirmations.len(), 1);
    assert!(outcome.summary.confirmations[0].contains("6 fact(s) confirmed"));
    assert!(outcome.summary.skipped.is_empty());

    let manifest = serde_json::to_string(batch).unwrap();
    let after = library(&[("captured.json".into(), manifest)]);
    assert_eq!(after.snapshot().manifests_failed, 0);
    assert_eq!(
        status_of(&after, "RELAYMOD"),
        (RouteStatus::Reachable, "CAPTURE_VALIDATED".into())
    );
    let survey = after.survey(&vehicle());
    assert_eq!(survey.hypothesis, 0);
}

#[test]
fn silence_and_a_capture_are_observations_that_confirm_nothing() {
    let before = library(&[]);
    let report = session_report(vec![read_report(None, None)], true);
    let outcome = intake(&report, "session.json", &before).unwrap();
    let keys: Vec<String> = outcome
        .batch
        .records
        .iter()
        .map(|record| format!("{:?}", record.key))
        .collect();
    assert_eq!(outcome.batch.records.len(), 2);
    assert!(keys
        .iter()
        .any(|key| key.contains(CAPTURED_READ_ATTEMPT_CLAIM)));
    assert!(keys
        .iter()
        .any(|key| key.contains(CAPTURED_BUS_ACTIVITY_CLAIM)));
    assert!(outcome.summary.confirmations.is_empty());
    assert_eq!(outcome.summary.observations.len(), 2);

    let manifest = serde_json::to_string(&outcome.batch).unwrap();
    let after = library(&[("captured.json".into(), manifest)]);
    assert_eq!(
        status_of(&after, "RELAYMOD"),
        (RouteStatus::Hypothesis, "UNVERIFIED".into())
    );
}

#[test]
fn an_unexpected_responder_is_recorded_as_seen_and_surfaces_as_a_conflict() {
    let before = library(&[]);
    let report = session_report(vec![read_report(Some("0x7A8"), Some("59 02 FF"))], false);
    let outcome = intake(&report, "session.json", &before).unwrap();
    assert!(outcome
        .summary
        .skipped
        .iter()
        .any(|line| line.contains("responder 0x7A8 is not the expected 0x72E")));

    let manifest = serde_json::to_string(&outcome.batch).unwrap();
    let after = library(&[("captured.json".into(), manifest)]);
    let (status, _) = status_of(&after, "RELAYMOD");
    assert_eq!(status, RouteStatus::Conflict);
}

#[test]
fn a_report_of_another_schema_or_with_nothing_to_record_is_refused() {
    let library = library(&[]);
    let other =
        session_report(vec![], false).replace("jlr-scanner.session-report", "something-else");
    assert!(intake(&other, "x.json", &library)
        .unwrap_err()
        .to_string()
        .contains("expected a jlr-scanner.session-report"));
    let empty = session_report(vec![], false);
    assert!(intake(&empty, "x.json", &library)
        .unwrap_err()
        .to_string()
        .contains("nothing this intake can record"));
}

/// A live read and a mileage survey make the same request a module read
/// makes, through the same path, and since the review of 2026-09-12 they
/// leave the same record. The intake reads it the same way, under the name of
/// the place it sat, so a tester's live session confirms what a single read
/// confirms — and no more, because repeating a request proves nothing new.
#[test]
fn a_live_read_run_and_a_mileage_survey_confirm_what_one_read_confirms() {
    let before = library(&[]);
    assert_eq!(
        status_of(&before, "RELAYMOD"),
        (RouteStatus::Hypothesis, "UNVERIFIED".into())
    );

    let answered = read_report(Some("0x72E"), Some("59 02 FF"));
    let silent = read_report(None, None);
    let bundle = session_bundle(
        vec![],
        false,
        serde_json::json!([{
            "schema": "prowlone.live-read-run",
            "set": [
                { "ecu_family": "RELAYMOD", "identifier": "0x1945", "reads": 12, "dropped": false, "record": answered },
                { "ecu_family": "RELAYMOD", "identifier": "0x0347", "reads": 0, "dropped": true, "record": silent }
            ],
            "samples": []
        }]),
        serde_json::json!([{
            "schema": "prowlone.mileage-survey",
            "reads": [ answered ],
            "readings": []
        }]),
        serde_json::json!([{
            "schema": "prowlone.module-passport",
            "reads": [ answered ],
            "readings": []
        }]),
    );
    let outcome = intake(&bundle, "session.json", &before).unwrap();
    let batch = &outcome.batch;
    // Three answered records confirm six facts each and record one
    // observation each; the silent one is an attempt. 7 + 7 + 7 + 1.
    assert_eq!(batch.records.len(), 22, "{:#?}", outcome.summary);
    assert_eq!(outcome.summary.confirmations.len(), 3);
    assert!(outcome
        .summary
        .confirmations
        .iter()
        .any(|line| line.starts_with("module_passports[0].reads[0] RELAYMOD")));
    assert!(outcome
        .summary
        .confirmations
        .iter()
        .any(|line| line.starts_with("live_read_runs[0].set[0] RELAYMOD")));
    assert!(outcome
        .summary
        .confirmations
        .iter()
        .any(|line| line.starts_with("mileage_surveys[0].reads[0] RELAYMOD")));
    assert!(outcome
        .summary
        .observations
        .iter()
        .any(|line| line.starts_with("live_read_runs[0].set[1] RELAYMOD")));
    // Record ids stay apart: the same fact confirmed twice is two records
    // from two places, not one overwritten by the other.
    let ids: std::collections::BTreeSet<&str> = batch
        .records
        .iter()
        .map(|record| record.id.as_str())
        .collect();
    assert_eq!(ids.len(), batch.records.len());
    assert!(ids.iter().any(|id| id.contains(".live.00.000.")));
    assert!(ids.iter().any(|id| id.contains(".mileage.00.000.")));
    assert!(ids.iter().any(|id| id.contains(".passport.00.000.")));

    let manifest = serde_json::to_string(batch).unwrap();
    let after = library(&[("captured.json".into(), manifest)]);
    assert_eq!(
        status_of(&after, "RELAYMOD"),
        (RouteStatus::Reachable, "CAPTURE_VALIDATED".into())
    );

    // A bundle that holds only an empty run is still nothing to record, and
    // the refusal names the kinds it looked for.
    let empty = session_bundle(
        vec![],
        false,
        serde_json::json!([{ "set": [] }]),
        serde_json::json!([]),
        serde_json::json!([{ "reads": [] }]),
    );
    let error = intake(&empty, "session.json", &before)
        .unwrap_err()
        .to_string();
    assert!(error.contains("live-read run"), "{error}");
    assert!(error.contains("module passport"), "{error}");
    assert!(error.contains("configuration read"), "{error}");
}
