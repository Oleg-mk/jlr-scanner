//! The bench end to end (ADR-0020): the shell's own services over the bench
//! transport, reading the SYNTHA vehicle the F10 fixtures describe. No serial
//! port is opened, and nothing here is evidence of any vehicle.

use super::adapter_service::{AdapterService, SystemAdapterBackend, BENCH_PORT, BENCH_TRANSPORT};
use super::battery_service::{BatteryService, BATTERY_READ_TIMEOUT};
use super::capture_service::CaptureService;
use super::ccf_service::{CcfService, CCF_READ_TIMEOUT};
use super::live_read_service::{LiveReadService, LIVE_READ_TIMEOUT};
use super::mileage_service::{MileageService, MILEAGE_READ_TIMEOUT};
use super::module_read_service::{map_live_error, ModuleReadService, PreparedModuleRead};
use super::passport_service::{PassportService, PASSPORT_READ_TIMEOUT};
use super::refresh_bench;
use super::session_report_service::{SessionReportService, SESSION_MODE_BENCH, SESSION_MODE_REAL};
use super::session_service::SessionService;
use super::standard_obd_service::{StandardObdService, STANDARD_OBD_TIMEOUT};
use app_contracts::{
    AdapterErrorCode, AdapterInfo, AdapterState, BatteryReadState, CcfReadState,
    LiveReadEntryRequest, LiveReadRequest, LiveReadState, MileageKind, MileageSurveyState,
    ModulePassportState, ModuleReadKind, ModuleReadRequest, ModuleReadState, StandardObdReadKind,
    StandardObdRequest, VehicleContextInput,
};
use diagnostic_session::KnowledgeLibrary;
use knowledge::{
    sha256_bytes, ContentFingerprint, IngestionAdapter, RedistributionStatus, SourceId,
    SourceRecord, SourceType,
};
use mongoose_jlr::bench::share_bus;
use mongoose_jlr::VehicleRouteId;
use sdd_ingest::{
    BatteryFormatting, CcfAdapter, ConverterCatalogue, DidFormattingAdapter, ModelYearTimeline,
    ModuleTextAdapter, PlatformAdapter, VinDecodeAdapter,
};
use std::time::Duration;
use transport_api::EmptyBench;

const PLATFORM: &str = include_str!("../../../../fixtures/knowledge/synthetic/f9_platform.xml");
const DIDS: &str = include_str!("../../../../fixtures/knowledge/synthetic/f9_did_formatting.xml");
const CONVERTER: &str = include_str!("../../../../fixtures/knowledge/synthetic/f9_converter.xml");
const CONVERTER_KM: &str =
    include_str!("../../../../fixtures/knowledge/synthetic/f9_converter_km.xml");
const MODULE_TEXT: &str =
    include_str!("../../../../fixtures/knowledge/synthetic/f9_module_text.xml");
const VIN_DECODE: &str = include_str!("../../../../fixtures/knowledge/synthetic/f9_vin_decode.xml");
const CCF: &str = include_str!("../../../../fixtures/knowledge/synthetic/f9_ccf_data.xml");

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
    // The battery join of ADR-0030: the platform names the module, the
    // formatting document describes the bytes. The exporter hands the one to
    // the other, and so does the bench.
    let mut battery_formatting = BatteryFormatting::new();
    battery_formatting.insert(
        0x4028,
        "Vehicle Battery Estimated State of Charge",
        Some("size=1;mask=0xff;converter=CVT_N_PCT_OFF_0_RES_1;scale=1;offset=0;offset_first=true"),
        Some("pct"),
    );
    let platform = PlatformAdapter::new(synthetic_source("bench-plat", PLATFORM))
        .unwrap()
        .with_timeline(timeline.clone())
        .with_battery_formatting(std::sync::Arc::new(battery_formatting));
    let ccf_adapter = CcfAdapter::new(synthetic_source("bench-ccf", CCF))
        .unwrap()
        .with_timeline(timeline.clone());
    let mut converters = ConverterCatalogue::new();
    converters.insert_from_xml(CONVERTER).unwrap();
    converters.insert_from_xml(CONVERTER_KM).unwrap();
    let dids = DidFormattingAdapter::new(synthetic_source("bench-did", DIDS), converters)
        .unwrap()
        .with_timeline(timeline);
    let platform_batch = platform.parse(PLATFORM).unwrap();
    let did_batch = dids.parse(DIDS).unwrap();
    let ccf_batch = ccf_adapter.parse(CCF).unwrap();
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
            serde_json::to_string(&vec![did_batch, text_batch, vin_batch, ccf_batch]).unwrap(),
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
    let live_identifier = identifier.clone();
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

    // The legislated services (ADR-0022, decision 7): prepared from the
    // standard, executed over the adapter protocol, decoded by the codec —
    // engine speed and vehicle speed from the engine controller at 0x7E0 —
    // then marked synthetic like everything the bench answers.
    let legislated = StandardObdRequest {
        kind: StandardObdReadKind::CurrentData,
        responder: 0,
        items: vec!["0x0C".into(), "0x0D".into()],
        context: vehicle(),
    };
    let prepared = StandardObdService::prepare(&legislated).expect("the standard prepares it");
    assert_eq!(prepared.transaction.encoded_payload(), [0x01, 0x0C, 0x0D]);
    let result = adapter
        .execute_j1979_read(&prepared.transaction, STANDARD_OBD_TIMEOUT)
        .expect("the bench is connected")
        .map_err(map_live_error);
    let mut standard = StandardObdService::new();
    let read = standard.finish(
        &legislated,
        Some(&prepared),
        Some(&info),
        Some(session.library()),
        result,
    );
    assert_eq!(read.state, ModuleReadState::Succeeded, "{read:?}");
    assert_eq!(read.responder, "0x7E0 → 0x7E8");
    assert_eq!(read.values[0].name, "engine speed");
    assert_eq!(read.values[0].number, Some(750.0));
    assert_eq!(read.values[0].unit, "rpm");
    assert_eq!(read.values[1].name, "vehicle speed");
    assert_eq!(read.route_validation, "SOURCE_BACKED");
    assert_eq!(standard.mark_synthetic().route_validation, "SYNTHETIC");
    let standard_json = standard.report_json().unwrap();
    assert!(standard_json.contains("\"route_validation\": \"SYNTHETIC\""));
    // The stored codes are the scenario's, worded in the tester's languages.
    let codes = StandardObdRequest {
        kind: StandardObdReadKind::StoredDtcs,
        responder: 0,
        items: Vec::new(),
        context: vehicle(),
    };
    let prepared = StandardObdService::prepare(&codes).unwrap();
    let result = adapter
        .execute_j1979_read(&prepared.transaction, STANDARD_OBD_TIMEOUT)
        .unwrap()
        .map_err(map_live_error);
    let read = standard.finish(
        &codes,
        Some(&prepared),
        Some(&info),
        Some(session.library()),
        result,
    );
    assert_eq!(read.state, ModuleReadState::Succeeded, "{read:?}");
    assert_eq!(read.dtc_kind.as_deref(), Some("stored"));
    assert!(
        !read.dtcs.is_empty(),
        "scenario 1 gives the engine controller codes"
    );

    // Live reading (ADR-0022): a set of two, one of which the library cannot
    // plan; the loop steps, the floor refuses a step that comes too soon, and
    // every sample is the bench's, so synthetic.
    let mut live = LiveReadService::new();
    let live_request = LiveReadRequest {
        entries: vec![
            LiveReadEntryRequest {
                ecu_family: "SYNTHMOD".into(),
                identifier: live_identifier.clone(),
            },
            LiveReadEntryRequest {
                ecu_family: "SYNTHMOD".into(),
                identifier: "0xDEAD".into(),
            },
        ],
        context: vehicle(),
    };
    let started = live.start(session.library(), Some(&info), &live_request);
    assert_eq!(started.state, LiveReadState::Running, "{started:?}");
    assert_eq!(started.entries.len(), 2);
    assert!(
        started.entries[1].dropped && started.entries[1].reason.is_some(),
        "an identifier the library does not list never joins the set: {:?}",
        started.entries[1]
    );
    assert_eq!(started.cadence_floor_ms, 100);

    let step =
        |live: &mut LiveReadService, adapter: &mut AdapterService<SystemAdapterBackend>| match live
            .next_due()
        {
            Some(due) => {
                let result = adapter
                    .execute_uds_read(&due.transaction, LIVE_READ_TIMEOUT)
                    .expect("the bench is connected")
                    .map_err(map_live_error);
                live.record(due.index, result);
                live.mark_synthetic()
            }
            None => live.snapshot(),
        };

    let first = step(&mut live, &mut adapter);
    assert_eq!(first.samples, 1, "{first:?}");
    assert_eq!(first.route_validation, "SYNTHETIC");
    assert!(!first.values.is_empty(), "the catalogue decoded the answer");
    let too_soon = step(&mut live, &mut adapter);
    assert_eq!(
        too_soon.samples, 1,
        "a step sooner than the floor allows is refused, not queued"
    );
    std::thread::sleep(super::live_read_service::LIVE_READ_FLOOR);
    let second = step(&mut live, &mut adapter);
    assert_eq!(second.samples, 2, "{second:?}");
    assert!(
        second.rounds >= 1,
        "the one live entry is a round of its own"
    );
    assert_eq!(second.values[0].samples, 2);

    let stopped = live.stop("stopped by the tester");
    assert_eq!(stopped.state, LiveReadState::Stopped);
    assert_eq!(
        stopped.stopped_reason.as_deref(),
        Some("stopped by the tester")
    );
    let live_json = live.report_json().expect("a stopped run is a report");
    assert!(live_json.contains("prowlone.live-read-run"), "{live_json}");
    assert!(live_json.contains("\"safety_class\": \"READ_ONLY\""));
    assert!(live_json.contains("\"route_validation\": \"SYNTHETIC\""));
    assert!(
        live_json.contains("\"at_ms\""),
        "the series carries its clock"
    );
    // Handed over once: a second stop cannot record the same run twice.
    assert!(live.take_report_json().is_some());
    assert!(live.take_report_json().is_none());
    // The odometer read from every module (ADR-0024): the catalogue names a
    // distance for SYNTHMOD, and the legislated counter beside it must not be
    // mistaken for one.
    let mut mileage = MileageService::new();
    let started = mileage.start(
        session.library(),
        Some(&info),
        &vehicle(),
        &["SYNTHMOD".to_string()],
    );
    assert_eq!(started.state, MileageSurveyState::Running, "{started:?}");
    assert_eq!(started.planned, 1, "one identifier carries a distance");

    while let Some(due) = mileage.next_due() {
        let result = adapter
            .execute_uds_read(&due.transaction, MILEAGE_READ_TIMEOUT)
            .expect("the bench is connected")
            .map_err(map_live_error);
        mileage.record(due.index, result);
        mileage.mark_synthetic();
    }
    let surveyed = mileage.snapshot();
    assert_eq!(surveyed.state, MileageSurveyState::Finished);
    assert_eq!(surveyed.asked, 1);
    assert_eq!(
        surveyed.readings.len(),
        1,
        "only the running total is a mileage; the lamp counter beside it is not: {:?}",
        surveyed.readings
    );
    let row = &surveyed.readings[0];
    assert_eq!(row.parameter, "Total distance");
    assert_eq!(row.kind, MileageKind::Current);
    assert_eq!(row.unit.as_deref(), Some("km"));
    assert_eq!(row.route_validation, "SYNTHETIC");
    // One reading is its own highest, so the difference from it is zero.
    assert_eq!(row.difference, Some(0.0));
    assert_eq!(surveyed.highest_module.as_deref(), Some("SYNTHMOD"));
    let mileage_json = mileage
        .report_json()
        .expect("a finished survey is a report");
    assert!(mileage_json.contains("prowlone.mileage-survey"));
    assert!(mileage_json.contains("\"safety_class\": \"READ_ONLY\""));
    // Every row of the report says what its value is worth, not only the
    // head (ADR-0020, decision 6): a bench row must never read as real.
    assert!(
        !mileage_json.contains("SOURCE_BACKED"),
        "a bench reading is marked as real inside the report: {mileage_json}"
    );
    assert!(
        mileage_json.matches("\"SYNTHETIC\"").count() >= 2,
        "the head and every row carry the mark"
    );
    // Readings, never a verdict: no word for what a difference might mean.
    for verdict in ["rolled", "tamper", "fraud", "clocked"] {
        assert!(
            !mileage_json.to_lowercase().contains(verdict),
            "the report must not say {verdict}"
        );
    }

    // The module passport (ADR-0027): SYNTHMOD's identification identifiers
    // are the platform's own list for it — the NET set's F1xx members and
    // the PDI set; the two software lists a qualifier this car does not state
    // chooses stay out. The bench answers each with a part-number-shaped text
    // and the VIN identifier with its VIN, every row marked synthetic.
    let mut passport = PassportService::new();
    let started = passport.start(
        session.library(),
        Some(&info),
        &vehicle(),
        &["SYNTHMOD".to_string(), "OTHERMOD".to_string()],
    );
    assert_eq!(started.state, ModulePassportState::Running, "{started:?}");
    assert_eq!(started.planned, 5, "{started:?}");
    assert_eq!(started.modules, 1, "OTHERMOD's route cannot be planned");
    while let Some(due) = passport.next_due() {
        let result = adapter
            .execute_uds_read(&due.transaction, PASSPORT_READ_TIMEOUT)
            .expect("the bench is connected")
            .map_err(map_live_error);
        passport.record(due.index, result);
        passport.mark_synthetic();
    }
    let identified = passport.snapshot();
    assert_eq!(identified.state, ModulePassportState::Finished);
    assert_eq!(identified.asked, 5);
    assert_eq!(identified.answered, 5, "{:?}", identified.readings);
    let identifiers: Vec<&str> = identified
        .readings
        .iter()
        .map(|row| row.identifier.as_str())
        .collect();
    assert_eq!(
        identifiers,
        vec!["0xF111", "0xF188", "0xF18C", "0xF190", "0xF1A0"]
    );
    for row in &identified.readings {
        assert_eq!(row.state, ModuleReadState::Succeeded, "{row:?}");
        assert_eq!(row.route_validation, "SYNTHETIC");
        assert_eq!(row.note, None, "the bench answers printable text: {row:?}");
        let value = row.value.as_deref().expect("a text");
        if row.identifier == "0xF190" {
            assert_eq!(
                value.len(),
                17,
                "the VIN identifier answers the VIN: {value}"
            );
        } else {
            assert!(
                value.starts_with("BN"),
                "a bench prefix no real part carries: {value}"
            );
        }
    }
    let core = identified
        .readings
        .iter()
        .find(|row| row.identifier == "0xF111")
        .unwrap();
    assert_eq!(core.parameter, "ECU Core Assembly Number");
    let passport_json = passport.report_json().expect("a finished run is a report");
    assert!(passport_json.contains("prowlone.module-passport"));
    assert!(passport_json.contains("\"safety_class\": \"READ_ONLY\""));
    assert!(
        !passport_json.contains("SOURCE_BACKED"),
        "a bench reading is marked as real inside the report: {passport_json}"
    );
    assert!(passport_json.contains("\"not_planned\""));
    // Read, never judged: no word for what a part number might mean.
    for verdict in ["outdated", "obsolete", "wrong part", "superseded"] {
        assert!(
            !passport_json.to_lowercase().contains(verdict),
            "the report must not say {verdict}"
        );
    }

    // The car configuration file (ADR-0028): SYNTHMOD keeps the master copy
    // and answers 0xF106 with the CCF block and 0xF105 with VB and RES at
    // two offsets; the copy on OTHERMOD cannot be planned and is listed with
    // the reason. The bench's blocks land every parameter on a listed option
    // and every row is synthetic; nothing is judged.
    let mut ccf = CcfService::new();
    let started = ccf.start(session.library(), Some(&info), &vehicle());
    assert_eq!(started.state, CcfReadState::Running, "{started:?}");
    assert_eq!(started.scheme.as_deref(), Some("did"));
    assert_eq!(started.master_module.as_deref(), Some("SYNTHMOD"));
    assert_eq!(
        started.planned, 2,
        "0xF106 and 0xF105 of SYNTHMOD: {started:?}"
    );
    while let Some(due) = ccf.next_due() {
        let result = adapter
            .execute_uds_read(&due.transaction, CCF_READ_TIMEOUT)
            .expect("the bench is connected")
            .map_err(map_live_error);
        ccf.record(due.index, result);
        ccf.mark_synthetic();
    }
    let configured = ccf.snapshot();
    assert_eq!(configured.state, CcfReadState::Finished);
    assert_eq!(configured.answered, 2, "{:?}", configured.reads);
    assert_eq!(configured.readings.len(), 8, "{:?}", configured.readings);
    let brand = configured
        .readings
        .iter()
        .find(|row| row.parameter == "PARAM_SYNTH_BRAND")
        .unwrap();
    assert!(brand.option_name.is_some(), "{brand:?}");
    assert_eq!(brand.note, None);
    assert!(brand.display);
    assert_eq!(brand.role, "sync");
    let res = configured
        .readings
        .iter()
        .find(|row| row.parameter == "SYNTH_RES")
        .unwrap();
    assert!(
        res.raw.is_some(),
        "the second block of 0xF105 is sliced at its offset: {res:?}"
    );
    assert_eq!(configured.hidden, 5, "{:?}", configured.readings);
    assert!(configured.differences.is_empty(), "no copy was read");
    for read in &configured.reads {
        assert_eq!(read.route_validation, "SYNTHETIC");
    }
    let ccf_json = ccf.report_json().expect("a finished run is a report");
    assert!(ccf_json.contains("prowlone.ccf-read"));
    assert!(ccf_json.contains("\"safety_class\": \"READ_ONLY\""));
    assert!(
        !ccf_json.contains("SOURCE_BACKED"),
        "a bench reading is marked as real inside the report: {ccf_json}"
    );
    assert!(
        ccf_json.contains("OTHERMOD"),
        "the refused copy is listed: {ccf_json}"
    );
    for verdict in ["corrupt", "wrong", "should be", "invalid"] {
        assert!(
            !ccf_json.to_lowercase().contains(verdict),
            "the report must not say {verdict}"
        );
    }

    // The battery (ADR-0030): the modules whose sets name the battery
    // monitor answer its identifiers, the card gets a level and a current,
    // and every row of it is synthetic. Nothing in the report judges the
    // battery, and the turbocharger parameter whose name merely contains
    // "charge" is not among the reads.
    let mut battery = BatteryService::new();
    let families: Vec<String> = survey
        .modules
        .iter()
        .map(|module| module.ecu_family.clone())
        .collect();
    let started = battery.start(session.library(), Some(&info), &vehicle(), &families);
    assert_eq!(started.state, BatteryReadState::Running, "{started:?}");
    assert_eq!(
        started.planned, 4,
        "the four battery identifiers SYNTHMOD declares: {started:?}"
    );
    // OTHERMOD declares the same set, so the data says it serves them too;
    // its route cannot be planned, and it is listed with that reason rather
    // than passed over.
    assert!(
        started
            .refused
            .iter()
            .any(|refusal| refusal.ecu_family == "OTHERMOD"),
        "{:?}",
        started.refused
    );
    while let Some(due) = battery.next_due() {
        let result = adapter
            .execute_uds_read(&due.transaction, BATTERY_READ_TIMEOUT)
            .expect("the bench is connected")
            .map_err(map_live_error);
        battery.record(due.index, result);
        battery.mark_synthetic();
    }
    let charged = battery.snapshot();
    assert_eq!(charged.state, BatteryReadState::Finished);
    assert_eq!(charged.answered, 4, "{:?}", charged.readings);
    let charge = charged
        .readings
        .iter()
        .find(|row| row.identifier == "0x4028")
        .expect("the state of charge is read");
    assert_eq!(charge.role, "CHARGE");
    assert!(charge.headline);
    assert_eq!(
        charge.value.as_deref(),
        Some("78"),
        "the bench answers a believable state of charge, decoded through the \
         formatting document own scale: {charge:?}"
    );
    assert_eq!(charge.unit.as_deref(), Some("pct"));
    // The ones no document describes are the bytes they are, with the
    // decoder own reason beside them.
    let resets = charged
        .readings
        .iter()
        .find(|row| row.identifier == "0x4020")
        .expect("the monitor resets are read");
    assert!(resets.note.is_some(), "{resets:?}");
    let current = charged
        .readings
        .iter()
        .find(|row| row.identifier == "0x4090")
        .expect("the dedicated battery set is read too");
    assert_eq!(current.role, "CURRENT");
    assert!(
        charged
            .readings
            .iter()
            .all(|row| row.identifier != "0xDE05"),
        "a turbocharger parameter is not the battery: {:?}",
        charged.readings
    );
    for row in &charged.readings {
        assert_eq!(row.route_validation, "SYNTHETIC");
    }
    let battery_json = battery.report_json().expect("a finished run is a report");
    assert!(battery_json.contains("prowlone.battery-state"));
    assert!(battery_json.contains("\"safety_class\": \"READ_ONLY\""));
    assert!(
        !battery_json.contains("SOURCE_BACKED"),
        "a bench reading is marked as real inside the report: {battery_json}"
    );
    for verdict in ["good", "bad", "poor", "healthy", "replace", "failing"] {
        assert!(
            !battery_json.to_lowercase().contains(verdict),
            "the report must not say {verdict}"
        );
    }

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
    report.add_standard_obd_read(&standard_json).unwrap();
    assert_eq!(report.snapshot().standard_obd_reads, 1);
    report.add_live_read_run(&live_json).unwrap();
    assert_eq!(report.snapshot().live_read_runs, 1);
    report.add_mileage_survey(&mileage_json).unwrap();
    assert_eq!(report.snapshot().mileage_surveys, 1);
    report.add_module_passport(&passport_json).unwrap();
    assert_eq!(report.snapshot().module_passports, 1);
    report.add_ccf_read(&ccf_json).unwrap();
    report.add_battery_read(&battery_json).unwrap();
    assert_eq!(report.snapshot().ccf_reads, 1);
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

// ---------------------------------------------------------------------------
// The cadence on the bench (ADR-0022, decision 3)
// ---------------------------------------------------------------------------

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Rounds over the set for `seconds`, through the very path a live read
/// would take: the prepared transaction executed over the adapter protocol,
/// the answer decoded and marked. Requests made, requests that succeeded,
/// rounds completed, time spent.
fn rounds_for(
    adapter: &mut AdapterService<SystemAdapterBackend>,
    reads: &mut ModuleReadService,
    info: &AdapterInfo,
    library: &KnowledgeLibrary,
    set: &[(ModuleReadRequest, PreparedModuleRead)],
    seconds: u64,
) -> (u64, u64, u32, Duration) {
    let started = std::time::Instant::now();
    let (mut requests, mut succeeded, mut rounds) = (0u64, 0u64, 0u32);
    while started.elapsed() < Duration::from_secs(seconds) {
        for (request, prepared) in set {
            let result = adapter
                .execute_uds_read(&prepared.transaction, Duration::from_secs(2))
                .expect("the bench is connected")
                .map_err(map_live_error);
            let read = reads.finish(request, Some(prepared), Some(info), Some(library), result);
            requests += 1;
            if read.state == ModuleReadState::Succeeded {
                succeeded += 1;
            }
        }
        rounds += 1;
    }
    (requests, succeeded, rounds, started.elapsed())
}

fn report(
    label: &str,
    entries: usize,
    (requests, succeeded, rounds, elapsed): (u64, u64, u32, Duration),
) {
    let seconds = elapsed.as_secs_f64();
    println!(
        "{label}: {entries} entr{} — {requests} requests in {seconds:.1} s, {succeeded} succeeded; \
         {:.1} requests/s, {:.2} ms per request, {rounds} rounds of {:.0} ms",
        if entries == 1 { "y" } else { "ies" },
        requests as f64 / seconds,
        seconds * 1000.0 / requests.max(1) as f64,
        seconds * 1000.0 / rounds.max(1) as f64,
    );
}

/// Not a test but a measurement, run on purpose: how many reads a second the
/// product's own path carries — prepare once, execute over the adapter
/// protocol, decode — with the bench answering. ADR-0022 decides that the
/// cadence is measured before any gauge is drawn; this is the bench half, a
/// ceiling set by the software alone. The hardware half is a live session.
///
///   cargo test -p prowlone-shell cadence_on_the_bench -- --ignored --nocapture
///
/// With `PROWLONE_LIBRARY` naming an issued copy, `PROWLONE_PROGRAM`,
/// `PROWLONE_MODEL_YEAR`, `PROWLONE_POWERTRAIN`, `PROWLONE_VARIANT` and
/// `PROWLONE_BREAKPOINT` describe the car; without them, the SYNTHA fixtures.
#[test]
#[ignore]
fn cadence_on_the_bench() {
    let (library, context) = match std::env::var("PROWLONE_LIBRARY") {
        Ok(directory) => (
            KnowledgeLibrary::load_issued_directory(std::path::Path::new(&directory)),
            VehicleContextInput {
                vehicle_program: env_or("PROWLONE_PROGRAM", "X250"),
                model_year: std::env::var("PROWLONE_MODEL_YEAR")
                    .ok()
                    .and_then(|year| year.parse().ok()),
                powertrain: std::env::var("PROWLONE_POWERTRAIN").ok(),
                variant: std::env::var("PROWLONE_VARIANT").ok(),
                market: None,
                year_breakpoint: std::env::var("PROWLONE_BREAKPOINT").ok(),
            },
        ),
        Err(_) => (library(), vehicle()),
    };
    let loaded = library.snapshot();
    println!(
        "library: {:?}, {} records — {}",
        loaded.state, loaded.records, loaded.message
    );

    let bench = share_bus(Box::new(EmptyBench));
    let mut adapter = AdapterService::new(SystemAdapterBackend::new(bench.clone()));
    let connected = adapter.connect_bench(bench_vehicle::SCENARIO_DEFAULT);
    assert_eq!(
        connected.state,
        AdapterState::Connected,
        "{:?}",
        connected.error
    );
    let info = adapter.connected_adapter().expect("the bench is connected");

    let mut session = SessionService::with_library(library);
    let survey = session.survey(&context);
    refresh_bench(&bench, &session, bench_vehicle::SCENARIO_DEFAULT);
    println!(
        "vehicle: {} {} — {} modules surveyed, {} reachable; bench: {}",
        context.vehicle_program,
        context.year_breakpoint.as_deref().unwrap_or(""),
        survey.modules.len(),
        survey.reachable,
        bench.lock().unwrap().describe()
    );

    // The set the ADR allows at most: one identifier per module first, then
    // a second one per module, until sixteen.
    let mut requests: Vec<ModuleReadRequest> = Vec::new();
    for pass in 0..8 {
        for module in &survey.modules {
            if requests.len() >= 16 {
                break;
            }
            if let Some(entry) = module.readable_identifiers.get(pass) {
                requests.push(ModuleReadRequest {
                    ecu_family: module.ecu_family.clone(),
                    kind: ModuleReadKind::Identifier,
                    identifier: Some(entry.identifier.clone()),
                    context: context.clone(),
                });
            }
        }
    }
    let mut set: Vec<(ModuleReadRequest, PreparedModuleRead)> = Vec::new();
    for request in requests {
        match ModuleReadService::prepare(session.library(), &request) {
            Ok(prepared) => set.push((request, prepared)),
            Err(error) => println!(
                "  not in the set: {} {} — {error:?}",
                request.ecu_family,
                request.identifier.as_deref().unwrap_or("")
            ),
        }
    }
    assert!(!set.is_empty(), "nothing to read");
    for (request, _) in &set {
        println!(
            "  {} {}",
            request.ecu_family,
            request.identifier.as_deref().unwrap_or("")
        );
    }

    let mut reads = ModuleReadService::new();
    // A warm-up round, so the first allocations do not count.
    rounds_for(&mut adapter, &mut reads, &info, session.library(), &set, 1);
    report(
        "full set",
        set.len(),
        rounds_for(&mut adapter, &mut reads, &info, session.library(), &set, 5),
    );
    let one = &set[..1];
    report(
        "one identifier",
        1,
        rounds_for(&mut adapter, &mut reads, &info, session.library(), one, 3),
    );
}
