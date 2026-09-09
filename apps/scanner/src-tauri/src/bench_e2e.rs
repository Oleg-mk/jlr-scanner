//! The bench end to end (ADR-0020): the shell's own services over the bench
//! transport, reading the SYNTHA vehicle the F10 fixtures describe. No serial
//! port is opened, and nothing here is evidence of any vehicle.

use super::adapter_service::{AdapterService, SystemAdapterBackend, BENCH_PORT, BENCH_TRANSPORT};
use super::capture_service::CaptureService;
use super::module_read_service::{map_live_error, ModuleReadService};
use super::refresh_bench;
use super::session_report_service::{SessionReportService, SESSION_MODE_BENCH, SESSION_MODE_REAL};
use super::session_service::SessionService;
use app_contracts::{
    AdapterErrorCode, AdapterState, ModuleReadKind, ModuleReadRequest, ModuleReadState,
    VehicleContextInput,
};
use diagnostic_session::KnowledgeLibrary;
use knowledge::{
    sha256_bytes, ContentFingerprint, IngestionAdapter, RedistributionStatus, SourceId,
    SourceRecord, SourceType,
};
use mongoose_jlr::bench::share_bus;
use mongoose_jlr::VehicleRouteId;
use sdd_ingest::{
    ConverterCatalogue, DidFormattingAdapter, ModelYearTimeline, ModuleTextAdapter,
    PlatformAdapter, VinDecodeAdapter,
};
use std::time::Duration;
use transport_api::EmptyBench;

const PLATFORM: &str = include_str!("../../../../fixtures/knowledge/synthetic/f9_platform.xml");
const DIDS: &str = include_str!("../../../../fixtures/knowledge/synthetic/f9_did_formatting.xml");
const CONVERTER: &str = include_str!("../../../../fixtures/knowledge/synthetic/f9_converter.xml");
const MODULE_TEXT: &str =
    include_str!("../../../../fixtures/knowledge/synthetic/f9_module_text.xml");
const VIN_DECODE: &str = include_str!("../../../../fixtures/knowledge/synthetic/f9_vin_decode.xml");

fn synthetic_source(id: &str, text: &str) -> SourceRecord {
    SourceRecord {
        id: SourceId::new(id).unwrap(),
        title: format!("bench fixture {id}"),
        source_type: SourceType::Synthetic,
        origin: "shell bench test".into(),
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

/// The SYNTHA library, built the way the application builds one.
fn library() -> KnowledgeLibrary {
    let mut timeline = ModelYearTimeline::new();
    for marker in ["MY08", "MY10", "MY12"] {
        timeline.observe("SYNTHA", marker).unwrap();
    }
    let platform = PlatformAdapter::new(synthetic_source("bench-plat", PLATFORM))
        .unwrap()
        .with_timeline(timeline.clone());
    let mut converters = ConverterCatalogue::new();
    converters.insert_from_xml(CONVERTER).unwrap();
    let dids = DidFormattingAdapter::new(synthetic_source("bench-did", DIDS), converters)
        .unwrap()
        .with_timeline(timeline);
    let platform_batch = platform.parse(PLATFORM).unwrap();
    let did_batch = dids.parse(DIDS).unwrap();
    let text_batch = ModuleTextAdapter::new(synthetic_source("bench-text", MODULE_TEXT))
        .unwrap()
        .parse(MODULE_TEXT)
        .unwrap();
    let vin_batch = VinDecodeAdapter::new(synthetic_source("bench-vin", VIN_DECODE))
        .unwrap()
        .parse(VIN_DECODE)
        .unwrap();
    let manifests = [
        (
            "platform.json".to_string(),
            serde_json::to_string(&platform_batch).unwrap(),
        ),
        (
            "bundle.json".to_string(),
            serde_json::to_string(&vec![did_batch, text_batch, vin_batch]).unwrap(),
        ),
    ];
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
        powertrain: Some("SYNTHENGINE".into()),
        variant: None,
        market: None,
        year_breakpoint: Some("MY10".into()),
    }
}

#[test]
fn the_bench_connects_without_a_port_reads_the_surveyed_vehicle_and_marks_everything_synthetic() {
    let bench = share_bus(Box::new(EmptyBench));
    let mut adapter = AdapterService::new(SystemAdapterBackend::new(bench.clone()));

    // Connect: no port, no discovery, the stand-in adapter's own board info.
    let connected = adapter.connect_bench(bench_vehicle::SCENARIO_DEFAULT);
    assert_eq!(
        connected.state,
        AdapterState::Connected,
        "{:?}",
        connected.error
    );
    assert!(adapter.is_bench());
    let info = adapter.connected_adapter().expect("the bench is connected");
    assert_eq!(info.transport, BENCH_TRANSPORT);
    assert_eq!(info.port, BENCH_PORT);
    assert!(!info.board_info.raw_response_hex.is_empty());
    // A refresh while on the bench leaves it connected: no port to lose.
    assert_eq!(adapter.discover().state, AdapterState::Connected);

    // The session describes SYNTHA; the bench follows the session.
    let mut session = SessionService::with_library(library());
    let survey = session.survey(&vehicle());
    refresh_bench(&bench, &session, bench_vehicle::SCENARIO_DEFAULT);
    assert!(bench.lock().unwrap().describe().contains("SYNTHA"));

    // One identifier read through the real stack: prepared from the library,
    // executed over the adapter protocol, decoded, then marked.
    let synthmod = survey
        .modules
        .iter()
        .find(|module| module.ecu_family == "SYNTHMOD")
        .expect("SYNTHMOD is surveyed");
    let identifier = synthmod
        .readable_identifiers
        .first()
        .expect("SYNTHMOD has a readable identifier")
        .identifier
        .clone();
    let request = ModuleReadRequest {
        ecu_family: "SYNTHMOD".into(),
        kind: ModuleReadKind::Identifier,
        identifier: Some(identifier),
        context: vehicle(),
    };
    let prepared = ModuleReadService::prepare(session.library(), &request)
        .expect("the read prepares from the library");
    let result = adapter
        .execute_uds_read(&prepared.transaction, Duration::from_secs(2))
        .expect("the bench is connected")
        .map_err(map_live_error);
    let mut reads = ModuleReadService::new();
    let read = reads.finish(
        &request,
        Some(&prepared),
        Some(&info),
        Some(session.library()),
        result,
    );
    assert_eq!(read.state, ModuleReadState::Succeeded, "{read:?}");
    assert_ne!(read.route_validation, "SYNTHETIC");
    assert_eq!(reads.mark_synthetic().route_validation, "SYNTHETIC");
    let read_json = reads.report_json().unwrap();
    assert!(read_json.contains("SYNTHETIC"));

    // Fault codes: the bench answers with codes the library describes.
    let request = ModuleReadRequest {
        ecu_family: "SYNTHMOD".into(),
        kind: ModuleReadKind::FaultCodes,
        identifier: None,
        context: vehicle(),
    };
    let prepared = ModuleReadService::prepare(session.library(), &request).unwrap();
    let result = adapter
        .execute_uds_read(&prepared.transaction, Duration::from_secs(2))
        .unwrap()
        .map_err(map_live_error);
    let faults = reads.finish(
        &request,
        Some(&prepared),
        Some(&info),
        Some(session.library()),
        result,
    );
    assert_eq!(faults.state, ModuleReadState::Succeeded, "{faults:?}");

    // A listen on the bench hears nothing, and is synthetic all the same.
    let listened = adapter
        .capture_route(VehicleRouteId::HsCan, Duration::from_millis(100))
        .expect("the bench is connected")
        .expect("listening on the bench succeeds");
    let mut capture = CaptureService::new();
    let recorded = capture.record(&listened, Duration::from_millis(100), &vehicle(), &info);
    assert!(!recorded.synthetic);
    assert!(capture.mark_synthetic().synthetic);
    let capture_json = capture.capture_json().unwrap();
    assert!(capture_json.contains("synthetic_bench_capture_not_vehicle_evidence"));
    assert!(capture_json.contains("bench-capture-hs-can-"));

    // The session is a bench session: it says so in the bundle, it refuses a
    // real adapter once it holds records, and the intake refuses the bundle.
    let mut report = SessionReportService::new();
    assert!(report.accepts(SESSION_MODE_BENCH));
    assert!(report.accepts(SESSION_MODE_REAL));
    report.set_mode(SESSION_MODE_BENCH);
    report.set_bench_scenario(bench_vehicle::SCENARIO_DEFAULT);
    assert!(
        report.accepts(SESSION_MODE_REAL),
        "an empty session may still switch"
    );
    report.add_module_read(&read_json).unwrap();
    report.add_capture(&capture_json).unwrap();
    assert!(report.is_bench());
    assert!(report.accepts(SESSION_MODE_BENCH));
    assert!(!report.accepts(SESSION_MODE_REAL));
    let bundle = report
        .report_json(
            &adapter.snapshot(),
            &session.library_snapshot(),
            session.last_survey().as_ref(),
        )
        .unwrap();
    assert!(bundle.contains("\"session_mode\": \"bench\""));
    // The number that reproduces this session's codes travels with it.
    assert!(bundle.contains("\"bench_scenario\": 1"), "{bundle}");
    let refused = report_intake::intake(&bundle, "bench-session.json", session.library())
        .expect_err("the intake refuses a bench session");
    assert!(refused.to_string().contains("bench session"), "{refused}");

    // Refusing the switch keeps the bench connected; a new session clears it.
    let refused = adapter.refuse_session_switch(SESSION_MODE_REAL);
    assert_eq!(
        refused.error.map(|error| error.code),
        Some(AdapterErrorCode::SessionModeMismatch)
    );
    assert!(adapter.is_bench(), "a refusal does not drop the bench");
    adapter.clear_session_error();
    assert!(adapter.snapshot().error.is_none());

    // Connecting a real adapter drops the bench link first, whatever the
    // enumeration then finds on this machine.
    adapter.connect(None);
    assert!(!adapter.is_bench());
}
