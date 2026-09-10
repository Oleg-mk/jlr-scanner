//! The legislated OBD-II reads over the standard's own addressing (ADR-0022,
//! decision 7). No vehicle knowledge and no resolver take part: ISO 15765-4
//! says where an emissions module answers on the high-speed pair, and every
//! OBD-II car answers there, JLR or not.

use diagnostic_execution::{
    execute_j1979_simulator, prepare_standard_obd_transaction, ExecutionError, PreparationError,
    ProvenanceField, StandardObdResponder, TransactionSafetyClass, STANDARD_OBD_CAPABILITY,
};
use diagnostic_simulator::{PayloadSimulatorBehavior, PayloadSimulatorScenario, SimulatorSource};
use knowledge::EvidenceClass;
use obd_j1979::{J1979Error, J1979Request, J1979Response, ParameterValue};
use transport_api::CanId;

fn scripted(request: J1979Request, response_payload: Vec<u8>) -> SimulatorSource {
    SimulatorSource::script_payload(
        PayloadSimulatorScenario {
            expected_request_payload: request.encoded(),
            behavior: PayloadSimulatorBehavior::Response { response_payload },
        },
        &request.encoded(),
        "hs-can",
        CanId::standard(0x7E8).unwrap(),
    )
    .unwrap()
}

#[test]
fn the_standard_plan_is_the_standards_own_numbers() {
    let request = J1979Request::current_data(&[0x0C]).unwrap();
    let transaction =
        prepare_standard_obd_transaction(request, StandardObdResponder::new(0).unwrap()).unwrap();
    assert_eq!(transaction.safety_class(), TransactionSafetyClass::ReadOnly);
    assert_eq!(
        transaction.physical_request_id(),
        CanId::standard(0x7E0).unwrap()
    );
    assert_eq!(
        transaction.expected_response_id(),
        CanId::standard(0x7E8).unwrap()
    );
    assert_eq!(
        transaction.functional_request_id(),
        Some(CanId::standard(0x7DF).unwrap())
    );
    assert_eq!(transaction.physical_pins(), [6, 14]);
    assert_eq!(transaction.bitrate_bps(), 500_000);
    assert_eq!(transaction.backend_route(), "hs-can");
    assert_eq!(transaction.protocol_family(), "ISO15765-4 / SAE J1979");
    assert_eq!(transaction.addressing_mode(), "normal_physical");
    assert_eq!(transaction.capability_id(), STANDARD_OBD_CAPABILITY);
    assert_eq!(transaction.protocol_request(), request);
    assert_eq!(transaction.encoded_payload(), [0x01, 0x0C]);
    // The eighth responder is the last the standard reserves.
    let last =
        prepare_standard_obd_transaction(request, StandardObdResponder::new(7).unwrap()).unwrap();
    assert_eq!(last.physical_request_id(), CanId::standard(0x7E7).unwrap());
    assert_eq!(last.expected_response_id(), CanId::standard(0x7EF).unwrap());
    assert_eq!(
        StandardObdResponder::new(8).unwrap_err(),
        PreparationError::InvalidResponder(8)
    );
}

#[test]
fn the_plan_cites_the_standard_not_a_vehicle() {
    let transaction = prepare_standard_obd_transaction(
        J1979Request::stored_dtcs(),
        StandardObdResponder::new(0).unwrap(),
    )
    .unwrap();
    for field in [
        ProvenanceField::PhysicalRequest,
        ProvenanceField::ExpectedResponse,
        ProvenanceField::FunctionalRequest,
        ProvenanceField::PhysicalRoute,
        ProvenanceField::Bitrate,
        ProvenanceField::Capability,
    ] {
        let traces = transaction.provenance().traces_for(field);
        assert!(!traces.is_empty(), "{field:?} has no trace");
        assert_eq!(
            traces[0].evidence_class,
            Some(EvidenceClass::StandardDocumentation),
            "{field:?}"
        );
        assert!(traces[0].locator.description.contains("ISO 15765-4"));
    }
    // The route into the adapter is the adapter's own descriptor, as for
    // every other read.
    let backend = transaction
        .provenance()
        .traces_for(ProvenanceField::BackendRoute);
    assert_eq!(backend[0].evidence_class, Some(EvidenceClass::SourceCode));
}

#[test]
fn a_scripted_module_answers_engine_speed_through_the_offline_path() {
    let request = J1979Request::current_data(&[0x0C]).unwrap();
    let transaction =
        prepare_standard_obd_transaction(request, StandardObdResponder::new(0).unwrap()).unwrap();
    let mut source = scripted(request, vec![0x41, 0x0C, 0x1A, 0xF8]);
    let result =
        execute_j1979_simulator(&transaction, &mut source, "standard-obd-rpm", 1_000_000).unwrap();
    assert_eq!(result.responder, CanId::standard(0x7E8).unwrap());
    assert_eq!(result.request_payload, [0x01, 0x0C]);
    assert_eq!(result.raw_diagnostic_response, [0x41, 0x0C, 0x1A, 0xF8]);
    match &result.response {
        J1979Response::CurrentData(values) => {
            assert_eq!(values[0].name, "engine speed");
            assert_eq!(values[0].value, ParameterValue::Number(1726.0));
            assert_eq!(values[0].unit, "rpm");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_refusal_comes_back_as_the_modules_own_reason() {
    let request = J1979Request::stored_dtcs();
    let transaction =
        prepare_standard_obd_transaction(request, StandardObdResponder::new(0).unwrap()).unwrap();
    let mut source = scripted(request, vec![0x7F, 0x03, 0x12]);
    let error =
        execute_j1979_simulator(&transaction, &mut source, "standard-obd-refused", 1_000_000)
            .unwrap_err();
    assert!(matches!(
        error,
        ExecutionError::J1979(J1979Error::NegativeResponse {
            service: 0x03,
            code: 0x12
        })
    ));
}

#[test]
fn a_multi_frame_answer_is_reassembled_before_it_is_read() {
    // Twenty-one bytes of VIN: a first frame and two consecutive ones.
    let request = J1979Request::vehicle_information(0x02).unwrap();
    let transaction =
        prepare_standard_obd_transaction(request, StandardObdResponder::new(0).unwrap()).unwrap();
    let mut response = vec![0x49, 0x02, 0x01];
    response.extend_from_slice(b"SAJWA0HP1AMR12345");
    let mut source = scripted(request, response);
    let result =
        execute_j1979_simulator(&transaction, &mut source, "standard-obd-vin", 1_000_000).unwrap();
    match result.response {
        J1979Response::VehicleInformation(obd_j1979::VehicleInformation::Vin(vin)) => {
            assert_eq!(vin, "SAJWA0HP1AMR12345");
        }
        other => panic!("{other:?}"),
    }
}
