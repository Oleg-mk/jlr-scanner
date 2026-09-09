//! The vehicle on the bench answers what the library describes and nothing
//! else: the synthetic SYNTHA vehicle of the F10 fixtures, built the way the
//! application builds a library, read through CAN frames on its routes.

use app_contracts::VehicleContextInput;
use bench_vehicle::BenchVehicle;
use diagnostic_session::KnowledgeLibrary;
use knowledge::{
    sha256_bytes, ContentFingerprint, IngestionAdapter, RedistributionStatus, SourceId,
    SourceRecord, SourceType,
};
use sdd_ingest::{
    ConverterCatalogue, DidFormattingAdapter, ModelYearTimeline, ModuleTextAdapter,
    PlatformAdapter, VinDecodeAdapter,
};
use transport_api::{BenchBus, BenchRoute, CanFrame, CanId};

const PLATFORM: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_platform.xml");
const DIDS: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_did_formatting.xml");
const CONVERTER: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_converter.xml");
const MODULE_TEXT: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_module_text.xml");
const VIN_DECODE: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_vin_decode.xml");

fn synthetic_source(id: &str, text: &str) -> SourceRecord {
    SourceRecord {
        id: SourceId::new(id).unwrap(),
        title: format!("bench fixture {id}"),
        source_type: SourceType::Synthetic,
        origin: "bench-vehicle test".into(),
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

fn request(route: BenchRoute, id: u16, data: &[u8]) -> CanFrame {
    let mut padded = data.to_vec();
    padded.resize(8, 0);
    CanFrame::new(0, route.as_str(), CanId::standard(id).unwrap(), padded).unwrap()
}

#[test]
fn the_vehicle_holds_the_modules_the_survey_can_reach_and_names_itself() {
    let library = library();
    let bench = BenchVehicle::from_library(&library, &vehicle(), Some("SAJTEST0000000001"));
    let families = bench.families();
    assert!(families.contains(&"SYNTHMOD"), "{families:?}");
    assert!(families.contains(&"OTHERMOD"), "{families:?}");
    // No request identifier on a bound route: silent, so not on the bench.
    assert!(!families.contains(&"PROGONLY"), "{families:?}");
    assert!(bench.describe().contains("SYNTHA MY10"));
    assert!(bench.describe().contains("synthetic"));
}

#[test]
fn a_known_identifier_answers_with_the_catalogue_width_on_its_own_route_only() {
    let library = library();
    let mut bench = BenchVehicle::from_library(&library, &vehicle(), None);
    // SYNTHMOD sits on CAN_HS at 0x7E0/0x7E8; ask for the first identifier
    // the survey lists as readable for it, whatever the fixture names.
    let survey = library.survey(&vehicle());
    let synthmod = survey
        .modules
        .iter()
        .find(|module| module.ecu_family == "SYNTHMOD")
        .expect("SYNTHMOD is surveyed");
    let identifier = synthmod
        .readable_identifiers
        .first()
        .map(|entry| u16::from_str_radix(entry.identifier.trim_start_matches("0x"), 16).unwrap())
        .expect("SYNTHMOD has a readable identifier");
    let [hi, lo] = identifier.to_be_bytes();
    let answers = bench.on_frame(
        BenchRoute::HsCan,
        &request(BenchRoute::HsCan, 0x7E0, &[0x03, 0x22, hi, lo]),
    );
    assert_eq!(answers.len(), 1, "single frame for a short identifier");
    assert_eq!(answers[0].id.value(), 0x7E8);
    assert_eq!(&answers[0].data[1..4], &[0x62, hi, lo]);
    assert!(answers[0].data[0] >= 0x04, "positive response carries data");
    // The same module is not on the medium-speed bus.
    let silence = bench.on_frame(
        BenchRoute::MsCan,
        &request(BenchRoute::MsCan, 0x7E0, &[0x03, 0x22, hi, lo]),
    );
    assert!(silence.is_empty());
    // OTHERMOD lives on CAN_MS at 0x760/0x768.
    let answers = bench.on_frame(
        BenchRoute::MsCan,
        &request(BenchRoute::MsCan, 0x760, &[0x03, 0x22, 0xF1, 0x90]),
    );
    assert_eq!(answers[0].id.value(), 0x768);
}

#[test]
fn an_unknown_identifier_draws_request_out_of_range_and_an_unknown_service_not_supported() {
    let library = library();
    let mut bench = BenchVehicle::from_library(&library, &vehicle(), None);
    let answers = bench.on_frame(
        BenchRoute::HsCan,
        &request(BenchRoute::HsCan, 0x7E0, &[0x03, 0x22, 0xDE, 0xAD]),
    );
    assert_eq!(&answers[0].data[..4], &[0x03, 0x7F, 0x22, 0x31]);
    let answers = bench.on_frame(
        BenchRoute::HsCan,
        &request(BenchRoute::HsCan, 0x7E0, &[0x02, 0x10, 0x03]),
    );
    assert_eq!(&answers[0].data[..4], &[0x03, 0x7F, 0x10, 0x11]);
}

#[test]
fn the_vin_comes_as_a_first_frame_and_the_rest_after_flow_control() {
    let library = library();
    let mut bench = BenchVehicle::from_library(&library, &vehicle(), Some("SAJTEST0000000001"));
    let first = bench.on_frame(
        BenchRoute::HsCan,
        &request(BenchRoute::HsCan, 0x7E0, &[0x03, 0x22, 0xF1, 0x90]),
    );
    assert_eq!(first.len(), 1);
    // First frame: length 20 = 62 F1 90 + 17 bytes of VIN.
    assert_eq!(&first[0].data[..5], &[0x10, 0x14, 0x62, 0xF1, 0x90]);
    // Nothing more until the tester's flow control.
    let rest = bench.on_frame(
        BenchRoute::HsCan,
        &request(BenchRoute::HsCan, 0x7E0, &[0x30, 0x00, 0x00]),
    );
    assert_eq!(rest.len(), 2);
    assert_eq!(rest[0].data[0], 0x21);
    assert_eq!(rest[1].data[0], 0x22);
    let mut vin = Vec::new();
    vin.extend_from_slice(&first[0].data[5..]);
    vin.extend_from_slice(&rest[0].data[1..]);
    vin.extend_from_slice(&rest[1].data[1..]);
    assert_eq!(&vin[..17], b"SAJTEST0000000001");
}

#[test]
fn fault_codes_answer_as_confirmed_records_and_calibration_as_one_id() {
    let library = library();
    let mut bench = BenchVehicle::from_library(&library, &vehicle(), None);
    let answers = bench.on_frame(
        BenchRoute::HsCan,
        &request(BenchRoute::HsCan, 0x7E0, &[0x03, 0x19, 0x02, 0x08]),
    );
    let data = &answers[0].data;
    // Single frame: 59 02 FF then one four-byte record at least.
    assert_eq!(&data[1..4], &[0x59, 0x02, 0xFF]);
    let length = usize::from(data[0] & 0x0F);
    assert_eq!((length - 3) % 4, 0);
    assert!(length >= 7);
    assert_eq!(data[7], 0x09, "test failed and confirmed");

    let first = bench.on_frame(
        BenchRoute::HsCan,
        &request(BenchRoute::HsCan, 0x7E0, &[0x02, 0x09, 0x04]),
    );
    assert_eq!(&first[0].data[..5], &[0x10, 0x13, 0x49, 0x04, 0x01]);
}
