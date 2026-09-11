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
    ConverterCatalogue, DidFormattingAdapter, DtcDescriptionAdapter, ModelYearTimeline,
    ModuleTextAdapter, PlatformAdapter, VinDecodeAdapter,
};
use transport_api::{BenchBus, BenchRoute, CanFrame, CanId};

const PLATFORM: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_platform.xml");
const DIDS: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_did_formatting.xml");
const CONVERTER: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_converter.xml");
const CONVERTER_KM: &str =
    include_str!("../../../fixtures/knowledge/synthetic/f9_converter_km.xml");
const MODULE_TEXT: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_module_text.xml");
const VIN_DECODE: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_vin_decode.xml");
/// Fault-code wording, so the bench has something to draw its codes from:
/// `P0100` stated for any module, `B1250` stated for SYNTHMOD.
const DTC_DESCRIPTIONS: &str =
    include_str!("../../../fixtures/knowledge/synthetic/f9_dtc_descriptions.xml");

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
    converters.insert_from_xml(CONVERTER_KM).unwrap();
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
    let dtc_batch = DtcDescriptionAdapter::new(synthetic_source("bench-dtc", DTC_DESCRIPTIONS))
        .unwrap()
        .parse(DTC_DESCRIPTIONS)
        .unwrap();
    let manifests = [
        (
            "platform.json".to_string(),
            serde_json::to_string(&platform_batch).unwrap(),
        ),
        (
            "bundle.json".to_string(),
            serde_json::to_string(&vec![did_batch, text_batch, vin_batch, dtc_batch]).unwrap(),
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
    let bench = BenchVehicle::from_library(
        &library,
        &vehicle(),
        Some("SAJTEST0000000001"),
        bench_vehicle::SCENARIO_DEFAULT,
    );
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
    let mut bench =
        BenchVehicle::from_library(&library, &vehicle(), None, bench_vehicle::SCENARIO_DEFAULT);
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
    let mut bench =
        BenchVehicle::from_library(&library, &vehicle(), None, bench_vehicle::SCENARIO_DEFAULT);
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
    let mut bench = BenchVehicle::from_library(
        &library,
        &vehicle(),
        Some("SAJTEST0000000001"),
        bench_vehicle::SCENARIO_DEFAULT,
    );
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
    let mut bench =
        BenchVehicle::from_library(&library, &vehicle(), None, bench_vehicle::SCENARIO_DEFAULT);
    let faults = reported_faults(&mut bench, 0x7E0);
    assert!(!faults.is_empty(), "SYNTHMOD reports codes on scenario 1");
    assert!(
        faults.iter().all(|(.., status)| *status == 0x09),
        "every record: test failed and confirmed"
    );

    let first = bench.on_frame(
        BenchRoute::HsCan,
        &request(BenchRoute::HsCan, 0x7E0, &[0x02, 0x09, 0x04]),
    );
    assert_eq!(&first[0].data[..5], &[0x10, 0x13, 0x49, 0x04, 0x01]);
}

/// The fault codes a module answers with, read off the wire as the tester's
/// own stack reads them — a single frame, or a first frame and the
/// consecutive ones that follow the flow control, because two codes no longer
/// fit in eight bytes. Code, failure type and status per record.
fn reported_faults(bench: &mut BenchVehicle, request_id: u16) -> Vec<(String, u8, u8)> {
    let answers = bench.on_frame(
        BenchRoute::HsCan,
        &request(BenchRoute::HsCan, request_id, &[0x03, 0x19, 0x02, 0x08]),
    );
    let first = &answers[0].data;
    let payload = match first[0] >> 4 {
        0x0 => first[1..1 + usize::from(first[0] & 0x0F)].to_vec(),
        0x1 => {
            let length = (usize::from(first[0] & 0x0F) << 8) | usize::from(first[1]);
            let mut payload = first[2..].to_vec();
            for frame in bench.on_frame(
                BenchRoute::HsCan,
                &request(BenchRoute::HsCan, request_id, &[0x30, 0x00, 0x00]),
            ) {
                payload.extend_from_slice(&frame.data[1..]);
            }
            payload.truncate(length);
            payload
        }
        other => panic!("neither a single nor a first frame: {other:#X}"),
    };
    assert_eq!(
        &payload[..3],
        &[0x59, 0x02, 0xFF],
        "the module answered at all"
    );
    payload[3..]
        .chunks(4)
        .map(|record| {
            let value = u16::from_be_bytes([record[0], record[1]]);
            let letter = ["P", "C", "B", "U"][usize::from(value >> 14)];
            (
                format!("{letter}{:04X}", value & 0x3FFF),
                record[2],
                record[3],
            )
        })
        .collect()
}

#[test]
fn the_healthy_scenario_leaves_every_module_quiet_and_still_answering() {
    let library = library();
    let mut bench =
        BenchVehicle::from_library(&library, &vehicle(), None, bench_vehicle::SCENARIO_HEALTHY);
    // Not silence: the module replies, and the reply carries no code. A car in
    // good order looks like this, and the application has to show it.
    assert!(reported_faults(&mut bench, 0x7E0).is_empty());
}

#[test]
fn a_number_paints_one_picture_every_time_and_another_number_a_different_one() {
    let library = library();
    let picture = |scenario| {
        let mut bench = BenchVehicle::from_library(&library, &vehicle(), None, scenario);
        reported_faults(&mut bench, 0x7E0)
    };
    assert_eq!(
        picture(1),
        picture(1),
        "the same number draws the same codes"
    );
    assert!(
        (2..=8).any(|scenario| picture(scenario) != picture(1)),
        "another number draws something else"
    );
}

#[test]
fn every_code_the_bench_reports_is_one_the_library_describes_for_that_module() {
    let library = library();
    let mut bench =
        BenchVehicle::from_library(&library, &vehicle(), None, bench_vehicle::SCENARIO_DEFAULT);
    let faults = reported_faults(&mut bench, 0x7E0);
    assert!(!faults.is_empty(), "scenario 1 gives SYNTHMOD codes");
    for (code, failure_type, _) in faults {
        assert!(
            library
                .describe_dtc(&code, failure_type, "SYNTHMOD")
                .description
                .is_some(),
            "{code} reported with no wording to show"
        );
    }
}

/// The legislated services, decoded by the codec after they crossed the
/// bench's wire: what a tester sees in the standard OBD-II panel.
fn legislated(
    bench: &mut BenchVehicle,
    route: BenchRoute,
    request_id: u16,
    asked: obd_j1979::J1979Request,
) -> Result<obd_j1979::J1979Response, obd_j1979::J1979Error> {
    let mut data = vec![asked.encoded().len() as u8];
    data.extend(asked.encoded());
    let answers = bench.on_frame(route, &request(route, request_id, &data));
    assert!(
        !answers.is_empty(),
        "the bench answered nothing to {asked:?}"
    );
    let first = &answers[0].data;
    let payload = match first[0] >> 4 {
        0x0 => first[1..1 + usize::from(first[0] & 0x0F)].to_vec(),
        0x1 => {
            let length = (usize::from(first[0] & 0x0F) << 8) | usize::from(first[1]);
            let mut payload = first[2..].to_vec();
            for frame in bench.on_frame(route, &request(route, request_id, &[0x30, 0x00, 0x00])) {
                payload.extend_from_slice(&frame.data[1..]);
            }
            payload.truncate(length);
            payload
        }
        other => panic!("neither a single nor a first frame: {other:#X}"),
    };
    obd_j1979::decode_response(asked, &payload)
}

#[test]
fn the_engine_controller_speaks_the_legislated_services_and_nobody_else_does() {
    use obd_j1979::{J1979Request, J1979Response, ParameterValue, VehicleInformation};
    let library = library();
    let mut bench = BenchVehicle::from_library(
        &library,
        &vehicle(),
        Some("SAJTEST0000000001"),
        bench_vehicle::SCENARIO_DEFAULT,
    );

    // Current data at the engine controller: idle, warm, at rest.
    match legislated(
        &mut bench,
        BenchRoute::HsCan,
        0x7E0,
        J1979Request::current_data(&[0x0C, 0x0D, 0x05]).unwrap(),
    )
    .unwrap()
    {
        J1979Response::CurrentData(values) => {
            assert_eq!(values[0].value, ParameterValue::Number(750.0));
            assert_eq!(values[1].value, ParameterValue::Number(0.0));
            assert_eq!(values[2].value, ParameterValue::Number(83.0));
        }
        other => panic!("{other:?}"),
    }
    // The support map names what the bench reports, and says a next map exists.
    match legislated(
        &mut bench,
        BenchRoute::HsCan,
        0x7E0,
        J1979Request::current_data(&[0x00]).unwrap(),
    )
    .unwrap()
    {
        J1979Response::CurrentData(values) => match &values[0].value {
            ParameterValue::Supported(pids) => {
                assert!(pids.contains(&0x0C) && pids.contains(&0x01) && pids.contains(&0x20));
            }
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
    // Stored codes are the scenario's own, and the lamp follows them.
    let stored = match legislated(
        &mut bench,
        BenchRoute::HsCan,
        0x7E0,
        J1979Request::stored_dtcs(),
    )
    .unwrap()
    {
        J1979Response::Dtcs { codes, .. } => codes,
        other => panic!("{other:?}"),
    };
    assert!(!stored.is_empty(), "scenario 1 gives SYNTHMOD codes");
    match legislated(
        &mut bench,
        BenchRoute::HsCan,
        0x7E0,
        J1979Request::current_data(&[0x01]).unwrap(),
    )
    .unwrap()
    {
        J1979Response::CurrentData(values) => {
            assert_eq!(values[0].value, ParameterValue::Flag(true), "lamp on");
            assert_eq!(values[1].value, ParameterValue::Number(stored.len() as f64));
        }
        other => panic!("{other:?}"),
    }
    // The VIN is the session's, over three frames.
    match legislated(
        &mut bench,
        BenchRoute::HsCan,
        0x7E0,
        J1979Request::vehicle_information(0x02).unwrap(),
    )
    .unwrap()
    {
        J1979Response::VehicleInformation(VehicleInformation::Vin(vin)) => {
            assert_eq!(vin, "SAJTEST0000000001");
        }
        other => panic!("{other:?}"),
    }
    // The functional request reaches the same controller.
    assert!(legislated(
        &mut bench,
        BenchRoute::HsCan,
        0x7DF,
        J1979Request::current_data(&[0x0C]).unwrap()
    )
    .is_ok());
    // A module outside the standard's addresses refuses the service — asked
    // on its own bus, because a module on the medium-speed pair hears nothing
    // sent on the high-speed one, and that is silence, not a refusal.
    let survey = library.survey(&vehicle());
    let other = survey
        .modules
        .iter()
        .find(|module| module.ecu_family == "OTHERMOD")
        .expect("OTHERMOD is surveyed");
    let other_id = other
        .request_id
        .as_deref()
        .map(|id| u16::from_str_radix(id.trim_start_matches("0x"), 16).unwrap())
        .expect("OTHERMOD has a request identifier");
    let other_route = match other.backend_route.as_deref() {
        Some("ms-can") => BenchRoute::MsCan,
        _ => BenchRoute::HsCan,
    };
    let refused = legislated(
        &mut bench,
        other_route,
        other_id,
        J1979Request::current_data(&[0x0C]).unwrap(),
    )
    .unwrap_err();
    assert!(matches!(
        refused,
        obd_j1979::J1979Error::NegativeResponse {
            service: 0x01,
            code: 0x11
        }
    ));
}

#[test]
fn the_healthy_scenario_has_no_codes_no_lamp_and_no_freeze_frame() {
    use obd_j1979::{J1979Request, J1979Response, ParameterValue};
    let library = library();
    let mut bench =
        BenchVehicle::from_library(&library, &vehicle(), None, bench_vehicle::SCENARIO_HEALTHY);
    match legislated(
        &mut bench,
        BenchRoute::HsCan,
        0x7E0,
        J1979Request::stored_dtcs(),
    )
    .unwrap()
    {
        J1979Response::Dtcs { codes, .. } => assert!(codes.is_empty()),
        other => panic!("{other:?}"),
    }
    match legislated(
        &mut bench,
        BenchRoute::HsCan,
        0x7E0,
        J1979Request::current_data(&[0x01]).unwrap(),
    )
    .unwrap()
    {
        J1979Response::CurrentData(values) => {
            assert_eq!(values[0].value, ParameterValue::Flag(false));
        }
        other => panic!("{other:?}"),
    }
    assert!(legislated(
        &mut bench,
        BenchRoute::HsCan,
        0x7E0,
        J1979Request::freeze_frame(0x0C, 0)
    )
    .is_err());
}

#[test]
fn the_freeze_frame_answers_its_map_and_its_code_whether_or_not_a_code_froze_one() {
    use obd_j1979::{J1979Request, J1979Response, ParameterValue};
    let library = library();
    let frame = |bench: &mut BenchVehicle, pid: u8| match legislated(
        bench,
        BenchRoute::HsCan,
        0x7E0,
        J1979Request::freeze_frame(pid, 0),
    ) {
        Ok(J1979Response::FreezeFrame {
            frame,
            mut parameters,
        }) => {
            assert_eq!(frame, 0);
            Some(parameters.remove(0).value)
        }
        Ok(other) => panic!("{other:?}"),
        Err(_) => None,
    };

    // Healthy: the map and "no code" answer; the frame's values do not exist.
    let mut bench =
        BenchVehicle::from_library(&library, &vehicle(), None, bench_vehicle::SCENARIO_HEALTHY);
    match frame(&mut bench, 0x00) {
        Some(ParameterValue::Supported(items)) => {
            assert!(items.contains(&0x02) && items.contains(&0x0C), "{items:?}");
            assert!(
                !items.contains(&0x01),
                "monitor status is no part of a frame"
            );
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        frame(&mut bench, 0x02),
        Some(ParameterValue::Text("none".into()))
    );
    assert_eq!(frame(&mut bench, 0x0C), None);

    // With a code: the code itself, then the standing engine behind it.
    let mut bench =
        BenchVehicle::from_library(&library, &vehicle(), None, bench_vehicle::SCENARIO_DEFAULT);
    match frame(&mut bench, 0x02) {
        Some(ParameterValue::Text(code)) => {
            assert!(code.starts_with(['P', 'C', 'B', 'U']), "{code}")
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(frame(&mut bench, 0x0C), Some(ParameterValue::Number(750.0)));
}
