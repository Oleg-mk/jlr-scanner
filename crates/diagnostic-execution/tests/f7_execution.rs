use diagnostic_environment::{
    CanIdFormat, DiagnosticEnvironmentQuery, DiagnosticEnvironmentResolution,
    DiagnosticEnvironmentResolver, DiagnosticTarget, EnvironmentField, ImplementationMarkerKind,
    PartialDiagnosticEnvironment, ResolutionConflict,
};
use diagnostic_execution::{
    execute_replay, execute_simulator, prepare_read_only_transaction, DiagnosticTargetIdentity,
    ExecutionError, ExecutionFixtureClass, OfflineExecutionSource, PreparationError,
    ProvenanceField, ReadOnlyDiagnosticIntent, TransactionSafetyClass,
};
use diagnostic_simulator::{PayloadSimulatorBehavior, PayloadSimulatorScenario, SimulatorSource};
use knowledge::{
    sha256_bytes, Applicability, ClaimKey, ContentFingerprint, DiagnosticSafetyClass,
    DimensionConstraint, EntityKind, EvidenceClass, EvidenceId, EvidenceRecord, KnowledgeEntity,
    KnowledgeRecord, KnowledgeStore, KnowledgeValue, RedistributionStatus, SourceId, SourceLocator,
    SourceRecord, SourceType, ValidationState, VehicleContext, YearConstraint,
};
use obd_j1979::{
    encode_calibration_identification_response, CalibrationId, J1979Error, J1979Request,
};
use std::collections::BTreeMap;
use transport_api::{CanId, CanSourceKind};
use transport_replay::{FixtureClass, PlaybackMode, ReplaySource};

const CAPABILITY: &str = "obd.service09.infotype04.calibration_id.read_only";
const ECU: &str = "jlr.x250.ecm";
const IMPLEMENTATION: &str = "jlr.x250.ecm.cx23-14c204-zad";
const CALIBRATION: &str = "CX23-14C204-ZAD";
const OBSERVATION_SHA256: &str = "310ff355a053adc3430269c4903084c619b95a38e4d765ff87a767eac579641b";
const OBSERVATION: &[u8] =
    include_bytes!("../../../fixtures/knowledge/captured/x250_obd_fusion_mode09.normalized.txt");
const REPLAY: &str =
    include_str!("../../../fixtures/synthetic/f7_x250_mode09_calibration_multiframe.json");

fn context() -> VehicleContext {
    VehicleContext {
        vehicle_program: Some("X250".into()),
        model_year: Some(2010),
        architecture_generation: Some("X250".into()),
        ecu_family: Some(ECU.into()),
        powertrain: Some("AJ133-5.0L-SC".into()),
        variant: Some("Supercharged".into()),
        market: Some("NAS".into()),
        diagnostic_implementation: Some(IMPLEMENTATION.into()),
        other: BTreeMap::new(),
    }
}

fn applicability() -> Applicability {
    Applicability {
        vehicle_program: DimensionConstraint::one_of(vec!["X250".into()]).unwrap(),
        model_year: YearConstraint::Range {
            model_year_from: Some(2010),
            model_year_to: Some(2010),
        },
        architecture_generation: DimensionConstraint::one_of(vec!["X250".into()]).unwrap(),
        ecu_family: DimensionConstraint::one_of(vec![ECU.into()]).unwrap(),
        powertrain: DimensionConstraint::one_of(vec!["AJ133-5.0L-SC".into()]).unwrap(),
        variant: DimensionConstraint::one_of(vec!["Supercharged".into()]).unwrap(),
        market: DimensionConstraint::one_of(vec!["NAS".into()]).unwrap(),
        diagnostic_implementation: DimensionConstraint::one_of(vec![IMPLEMENTATION.into()])
            .unwrap(),
        other: BTreeMap::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn register_source(
    store: &mut KnowledgeStore,
    id: &str,
    title: &str,
    source_type: SourceType,
    origin: &str,
    locator: &str,
    sha256: &str,
    programs: &[&str],
    provenance: &str,
) {
    store
        .register_source(SourceRecord {
            id: SourceId::new(id).unwrap(),
            title: title.into(),
            source_type,
            origin: origin.into(),
            source_locator: locator.into(),
            content_fingerprint: Some(ContentFingerprint::sha256(sha256).unwrap()),
            acquired_on: Some("2026-08-31".into()),
            declared_vehicle_programs: programs.iter().map(|value| (*value).into()).collect(),
            provenance: provenance.into(),
            redistribution_status: RedistributionStatus::RestrictedMetadataOnly,
            notes: Some("F6 source identity reused by the F7 offline golden".into()),
        })
        .unwrap();
}

#[allow(clippy::too_many_arguments)]
fn add_evidence(
    store: &mut KnowledgeStore,
    id: &str,
    source_id: &str,
    class: EvidenceClass,
    description: &str,
    page: Option<u32>,
    section: Option<&str>,
    key: Option<&str>,
    excerpt: &str,
    notes: &str,
) -> EvidenceId {
    let evidence_id = EvidenceId::new(id).unwrap();
    store
        .add_evidence(EvidenceRecord {
            id: evidence_id.clone(),
            source_id: SourceId::new(source_id).unwrap(),
            evidence_class: Some(class),
            locator: SourceLocator {
                description: description.into(),
                document_page: page,
                document_section: section.map(str::to_owned),
                record_key: key.map(str::to_owned),
                capture_timestamp_us: None,
            },
            excerpt: Some(excerpt.into()),
            notes: Some(notes.into()),
        })
        .unwrap();
    evidence_id
}

fn add_record(
    store: &mut KnowledgeStore,
    id: &str,
    key: ClaimKey,
    value: KnowledgeValue,
    mut evidence_ids: Vec<EvidenceId>,
    validation_state: ValidationState,
) {
    evidence_ids.sort();
    evidence_ids.dedup();
    store
        .add_record(KnowledgeRecord {
            id: id.into(),
            entity: KnowledgeEntity {
                kind: EntityKind::DiagnosticImplementation,
                id: IMPLEMENTATION.into(),
            },
            key,
            value,
            applicability: applicability(),
            evidence_ids,
            validation_state,
        })
        .unwrap();
}

fn real_x250_resolution() -> DiagnosticEnvironmentResolution {
    assert_eq!(sha256_bytes(OBSERVATION), OBSERVATION_SHA256);
    let mut store = KnowledgeStore::new();
    register_source(
        &mut store,
        "public-jaguarforums-x250-obd-fusion-2016-03",
        "2010 Jaguar XF Supercharged OBD Fusion diagnostic reports",
        SourceType::Captured,
        "JaguarForums public posts 159551 and 159757",
        "repository:fixtures/knowledge/captured/x250_obd_fusion_mode09.normalized.txt",
        OBSERVATION_SHA256,
        &["X250"],
        "Minimal normalized public-index snapshot; original attachment unavailable",
    );
    register_source(
        &mut store,
        "jlr-jtb00244nas1-2012-02-24",
        "Jaguar Technical Bulletin JTB00244NAS1",
        SourceType::Documented,
        "Jaguar Land Rover North America, LLC",
        "https://static.nhtsa.gov/odi/tsbs/2012/SB-10043530-3423.pdf",
        "ff4d405e212b5e1f973c41c08aabd929043b7ec084da5d212af19b9ca4347202",
        &["X250"],
        "Official Jaguar bulletin dated 24 FEB 2012",
    );
    register_source(
        &mut store,
        "charm-jaguar-2010-x250-v8-5.0l-sc",
        "2010 Jaguar XF (X250) V8-5.0L SC service manual offline bundle",
        SourceType::Documented,
        "Operation CHARM exact Jaguar 2010 X250 5.0L SC bundle",
        "https://charm.li/bundle/Jaguar/2010/XF%20%28X250%29%20V8-5.0L%20SC/",
        "e09b54db2571a984190a504ab7c7f4f38eefb842d496f0286dc6d0676ff3106e",
        &["X250"],
        "Exact offline bundle link and archive fingerprint",
    );
    register_source(
        &mut store,
        "california-bar-dad-2012-v2.5",
        "BAR OBD Inspection System Data Acquisition Device Specification",
        SourceType::Documented,
        "California Bureau of Automotive Repair",
        "https://www.bar.ca.gov/pdf/publications/DAD-2012.pdf",
        "9138a95fbfef44483c7efd1149405d93e98736f89b31a2eb9b9670b0bd4d2a9c",
        &[],
        "October 2012 V2.5 public regulatory specification",
    );
    register_source(
        &mut store,
        "elm-electronics-elm327dsj",
        "ELM327DSJ OBD to RS232 Interpreter datasheet",
        SourceType::Documented,
        "Elm Electronics",
        "https://www.elmelectronics.com/wp-content/uploads/2016/07/ELM327DS.pdf",
        "43cab005c9e1678325a3ba64066db93e09e2a78a23eb5fa360ee1eaf9a585428",
        &[],
        "Official datasheet used for ISO15765-4 and Mode 09 message semantics",
    );
    register_source(
        &mut store,
        "jlr-scanner-mongoose-jlr-passive-routes-8af1491",
        "JLR Scanner Mongoose JLR production route descriptors",
        SourceType::Documented,
        "JLR Scanner source code",
        "repository:crates/mongoose-jlr/src/passive.rs:74,79-84",
        "79180a11e16f2398cec8d08702ba369e0935517e92da8835774e327b3d16998c",
        &["X250"],
        "Receive-only production route descriptor",
    );

    let observed = add_evidence(
        &mut store,
        "evidence-x250-obd-fusion-mode09-7e8",
        "public-jaguarforums-x250-obd-fusion-2016-03",
        EvidenceClass::DirectObservation,
        "Mode $09 report dated 2016-03-15 and repeated 2016-03-19",
        None,
        Some("Mode $09 - Vehicle Information"),
        Some("VIN/Calibration ID - $7E8"),
        "VIN SAJWA0HE4AMR59890; Calibration ID - $7E8 CX23-14C204-ZAD",
        "No request frame or execution bytes were observed",
    );
    let tsb = add_evidence(
        &mut store,
        "evidence-jtb00244nas1-x250-ecm",
        "jlr-jtb00244nas1-2012-02-24",
        EvidenceClass::OemDocumentation,
        "PDF pages 1-2",
        Some(1),
        Some("AFFECTED VEHICLE RANGE / REPAIR PROCEDURE"),
        Some("XF X250 5.0L MY2010-2012 / ECM"),
        "XF (X250) 5.0L only, MY2010-2012; Engine control module through IDS/SDD",
        "Applicability and ECM relationship only",
    );
    let topology = add_evidence(
        &mut store,
        "evidence-charm-x250-ecm-hs-can",
        "charm-jaguar-2010-x250-v8-5.0l-sc",
        EvidenceClass::OemDocumentation,
        "pages/7013.html; control diagram 461775832.jpeg",
        None,
        Some("CONTROL DIAGRAM - HIGH SPEED CAN BUS"),
        Some("items 3 Diagnostic socket and 9 ECM"),
        "ECM and diagnostic socket participate on high speed CAN",
        "Exact 2010 X250 V8-5.0L SC bundle",
    );
    let bitrate = add_evidence(
        &mut store,
        "evidence-charm-x250-hs-can-bitrate",
        "charm-jaguar-2010-x250-v8-5.0l-sc",
        EvidenceClass::OemDocumentation,
        "pages/7012.html; table image 460639383.png",
        None,
        Some("Communications Network / OVERVIEW"),
        Some("High speed CAN bus / Baud Rate"),
        "High speed CAN bus: 500 kbits/s",
        "Exact-model network table",
    );
    let route = add_evidence(
        &mut store,
        "evidence-charm-x250-hs-can-dlc-route",
        "charm-jaguar-2010-x250-v8-5.0l-sc",
        EvidenceClass::OemDocumentation,
        "pages/7014.html and pages/7024.html; wiring image 468175520.jpeg",
        None,
        Some("CAN bus - high speed - Part 1"),
        Some("C2DB04B/6,C2DB04B/14"),
        "Diagnostic connector C2DB04B/6 HS_CAN_POS; C2DB04B/14 HS_CAN_NEG",
        "J1962 accessibility is stated separately on pages/7014.html",
    );
    let mode09 = add_evidence(
        &mut store,
        "evidence-bar-mode09-infotype04",
        "california-bar-dad-2012-v2.5",
        EvidenceClass::StandardDocumentation,
        "PDF page 14 / printed page 13, requirement 3.2.46",
        Some(14),
        Some("Mode $09 InfoType $04 Calibration Identification"),
        Some("3.2.46"),
        "CAL ID is Mode $09 InfoType $04 in accordance with SAE J1979",
        "Standard meaning only",
    );
    let addressing = add_evidence(
        &mut store,
        "evidence-elm-iso15765-11bit-addressing",
        "elm-electronics-elm327dsj",
        EvidenceClass::StandardDocumentation,
        "PDF page 41 of 94",
        Some(41),
        Some("11 bit ISO15765-4 CAN"),
        Some("7DF/7En/7E0-to-7E8"),
        "7DF functional; 7E0 physical for ECU responding as 7E8",
        "Response 0x7E8 remains independently observed",
    );
    let backend = add_evidence(
        &mut store,
        "evidence-mongoose-jlr-hs-can-route",
        "jlr-scanner-mongoose-jlr-passive-routes-8af1491",
        EvidenceClass::SourceCode,
        "crates/mongoose-jlr/src/passive.rs lines 74 and 79-84",
        None,
        Some("VEHICLE_ROUTES"),
        Some("VehicleRouteId::HsCan"),
        "route hs-can; pins 6/14; bitrate 500000; passive ready",
        "Descriptor is not opened or executed in F7",
    );

    add_record(
        &mut store,
        "record-x250-positive-ecm-marker",
        ClaimKey::Custom {
            name: "implementation_marker.module_acronym".into(),
        },
        KnowledgeValue::ImplementationMarker {
            marker_kind: ImplementationMarkerKind::ModuleAcronym,
            value: "ECM".into(),
        },
        vec![tsb],
        ValidationState::SourceBacked,
    );
    add_record(
        &mut store,
        "record-x250-positive-hs-can-route",
        ClaimKey::NetworkRoute,
        KnowledgeValue::NetworkRoute {
            logical_name: "HS-CAN".into(),
            connector: Some("J1962/C2DB04B".into()),
            pins: vec![6, 14],
            bitrate_bps: Some(500_000),
        },
        vec![topology, bitrate, route],
        ValidationState::SourceBacked,
    );
    add_record(
        &mut store,
        "record-x250-positive-backend-route",
        ClaimKey::Custom {
            name: "backend_route".into(),
        },
        KnowledgeValue::BackendRoute {
            backend: "mongoose-jlr".into(),
            route_id: "hs-can".into(),
        },
        vec![backend],
        ValidationState::SourceBacked,
    );
    add_record(
        &mut store,
        "record-x250-positive-protocol",
        ClaimKey::UsesProtocolFamily,
        KnowledgeValue::ProtocolFamily {
            name: "ISO15765-4 / SAE J1979".into(),
        },
        vec![addressing.clone()],
        ValidationState::SourceBacked,
    );
    add_record(
        &mut store,
        "record-x250-positive-standard-addressing",
        ClaimKey::DiagnosticAddressing,
        KnowledgeValue::DiagnosticAddressing {
            request_id: Some(0x7e0),
            response_id: None,
            functional_request_id: Some(0x7df),
            can_id_format: Some(CanIdFormat::Standard11Bit),
            addressing_mode: Some("normal_physical".into()),
        },
        vec![addressing],
        ValidationState::SourceBacked,
    );
    add_record(
        &mut store,
        "record-x250-positive-observed-response",
        ClaimKey::DiagnosticAddressing,
        KnowledgeValue::DiagnosticAddressing {
            request_id: None,
            response_id: Some(0x7e8),
            functional_request_id: None,
            can_id_format: None,
            addressing_mode: None,
        },
        vec![observed.clone()],
        ValidationState::CaptureValidated,
    );
    add_record(
        &mut store,
        "record-x250-positive-mode09-calibration-id",
        ClaimKey::SupportsCapability {
            capability: CAPABILITY.into(),
        },
        KnowledgeValue::Capability {
            name: CAPABILITY.into(),
            supported: true,
            safety_class: Some(DiagnosticSafetyClass::ReadOnly),
        },
        vec![mode09],
        ValidationState::SourceBacked,
    );
    add_record(
        &mut store,
        "record-x250-positive-calibration",
        ClaimKey::Custom {
            name: "implementation_marker.calibration_id".into(),
        },
        KnowledgeValue::ImplementationMarker {
            marker_kind: ImplementationMarkerKind::CalibrationId,
            value: CALIBRATION.into(),
        },
        vec![observed],
        ValidationState::CaptureValidated,
    );

    let target = DiagnosticTarget::implementation(ECU, IMPLEMENTATION, CAPABILITY).unwrap();
    let query = DiagnosticEnvironmentQuery::new(context(), target).unwrap();
    DiagnosticEnvironmentResolver::resolve(&store, &query)
}

fn intent() -> ReadOnlyDiagnosticIntent {
    ReadOnlyDiagnosticIntent::calibration_identification(
        DiagnosticTargetIdentity::new(ECU, IMPLEMENTATION).unwrap(),
    )
}

fn prepared() -> diagnostic_execution::PreparedDiagnosticTransaction {
    prepare_read_only_transaction(&real_x250_resolution(), intent()).unwrap()
}

fn response_payload() -> Vec<u8> {
    encode_calibration_identification_response(&[CalibrationId::new(CALIBRATION).unwrap()]).unwrap()
}

fn simulator(payload: Vec<u8>, response_id: u16) -> SimulatorSource {
    let request = J1979Request::calibration_identification().encoded();
    SimulatorSource::script_payload(
        PayloadSimulatorScenario {
            expected_request_payload: request.to_vec(),
            behavior: PayloadSimulatorBehavior::Response {
                response_payload: payload,
            },
        },
        &request,
        "hs-can",
        CanId::standard(response_id).unwrap(),
    )
    .unwrap()
}

#[test]
fn real_resolved_plan_compiles_to_targeted_read_only_transaction() {
    let transaction = prepared();
    assert_eq!(transaction.safety_class(), TransactionSafetyClass::ReadOnly);
    assert_eq!(transaction.target().ecu_family(), ECU);
    assert_eq!(transaction.logical_network(), "HS-CAN");
    assert_eq!(transaction.physical_connector(), "J1962/C2DB04B");
    assert_eq!(transaction.physical_pins(), &[6, 14]);
    assert_eq!(transaction.backend_route(), "hs-can");
    assert_eq!(transaction.bitrate_bps(), 500_000);
    assert_eq!(transaction.physical_request_id().value(), 0x7e0);
    assert_eq!(transaction.expected_response_id().value(), 0x7e8);
    assert_eq!(transaction.functional_request_id().unwrap().value(), 0x7df);
    assert_eq!(transaction.protocol_request().encoded(), [0x09, 0x04]);
    assert_eq!(transaction.observed_calibration(), CALIBRATION);
    assert!(transaction
        .provenance()
        .traces_for(ProvenanceField::PhysicalRequest)
        .iter()
        .all(|trace| trace.evidence_class == Some(EvidenceClass::StandardDocumentation)));
    assert!(transaction
        .provenance()
        .traces_for(ProvenanceField::ExpectedResponse)
        .iter()
        .all(|trace| trace.evidence_class == Some(EvidenceClass::DirectObservation)));
}

#[test]
fn indeterminate_and_conflict_are_rejected_with_distinct_reasons() {
    let indeterminate = DiagnosticEnvironmentResolution::Indeterminate {
        partial: PartialDiagnosticEnvironment::default(),
        unresolved_facts: vec![diagnostic_environment::UnresolvedFact {
            field: EnvironmentField::Bitrate,
            reason: "missing".into(),
        }],
    };
    assert!(matches!(
        prepare_read_only_transaction(&indeterminate, intent()),
        Err(PreparationError::EnvironmentIndeterminate(facts))
            if facts[0].field == EnvironmentField::Bitrate
    ));

    let conflict = DiagnosticEnvironmentResolution::Conflict {
        partial: PartialDiagnosticEnvironment::default(),
        conflicts: vec![ResolutionConflict {
            field: EnvironmentField::PhysicalRequestId,
            candidates: vec![],
        }],
    };
    assert!(matches!(
        prepare_read_only_transaction(&conflict, intent()),
        Err(PreparationError::EnvironmentConflict(conflicts))
            if conflicts[0].field == EnvironmentField::PhysicalRequestId
    ));
}

#[test]
fn preparation_rejects_capability_protocol_route_address_and_target_mismatch() {
    let DiagnosticEnvironmentResolution::Resolved(base) = real_x250_resolution() else {
        panic!("real F6 environment did not resolve")
    };

    let mut capability = base.clone();
    capability.read_only_capability.value.id = "unsupported".into();
    assert!(matches!(
        prepare_read_only_transaction(
            &DiagnosticEnvironmentResolution::Resolved(capability),
            intent()
        ),
        Err(PreparationError::UnsupportedCapability(_))
    ));

    let mut protocol = base.clone();
    protocol.protocol_family.value = "UDS".into();
    assert!(matches!(
        prepare_read_only_transaction(
            &DiagnosticEnvironmentResolution::Resolved(protocol),
            intent()
        ),
        Err(PreparationError::UnsupportedProtocol(_))
    ));

    let mut route = base.clone();
    route.physical_route.value.pins.clear();
    assert!(matches!(
        prepare_read_only_transaction(&DiagnosticEnvironmentResolution::Resolved(route), intent()),
        Err(PreparationError::InvalidRoute("physical_pins"))
    ));

    let mut bitrate = base.clone();
    bitrate.bitrate_bps.value = 0;
    assert!(matches!(
        prepare_read_only_transaction(
            &DiagnosticEnvironmentResolution::Resolved(bitrate),
            intent()
        ),
        Err(PreparationError::InvalidBitrate(0))
    ));

    let mut address = base.clone();
    address.physical_request_id.value = 0x18da_10f1;
    assert!(matches!(
        prepare_read_only_transaction(
            &DiagnosticEnvironmentResolution::Resolved(address),
            intent()
        ),
        Err(PreparationError::InvalidCanId { .. })
    ));

    let wrong_target = ReadOnlyDiagnosticIntent::calibration_identification(
        DiagnosticTargetIdentity::new("jlr.x250.tcm", IMPLEMENTATION).unwrap(),
    );
    assert!(matches!(
        prepare_read_only_transaction(
            &DiagnosticEnvironmentResolution::Resolved(base),
            wrong_target
        ),
        Err(PreparationError::TargetEcuMismatch { .. })
    ));
}

#[test]
fn simulator_multiframe_golden_returns_exact_typed_calibration() {
    let transaction = prepared();
    let payload = response_payload();
    assert_eq!(isotp::segment(&payload, None).unwrap().len(), 3);
    let mut source = simulator(payload, 0x7e8);
    let result = execute_simulator(
        &transaction,
        &mut source,
        "synthetic-f7-mode09-simulator",
        10_000,
    )
    .unwrap();
    assert_eq!(result.source, OfflineExecutionSource::Simulator);
    assert_eq!(result.physical_request_id.value(), 0x7e0);
    assert_eq!(result.responder.value(), 0x7e8);
    assert_eq!(result.functional_request_id.unwrap().value(), 0x7df);
    assert_eq!(result.calibration_ids, [CALIBRATION]);
    assert_eq!(
        result.execution_fixture.fixture_class,
        ExecutionFixtureClass::Synthetic
    );
    assert!(!result
        .environment_provenance
        .traces_for(ProvenanceField::ObservedCalibration)
        .is_empty());
}

#[test]
fn deterministic_replay_golden_uses_synthetic_frames_and_returns_exact_calibration() {
    let transaction = prepared();
    let run = || {
        let mut replay = ReplaySource::from_json(REPLAY, PlaybackMode::Deterministic).unwrap();
        assert_eq!(replay.fixture().fixture_class, FixtureClass::Synthetic);
        execute_replay(&transaction, &mut replay, 10_000).unwrap()
    };
    let first = run();
    let second = run();
    assert_eq!(first, second);
    assert_eq!(first.source, OfflineExecutionSource::Replay);
    assert_eq!(first.calibration_ids, [CALIBRATION]);
    assert_eq!(
        first.execution_fixture.fixture_class,
        ExecutionFixtureClass::Synthetic
    );
}

#[test]
fn wrong_responder_is_rejected_instead_of_becoming_a_target_result() {
    let transaction = prepared();
    let mut source = simulator(response_payload(), 0x7e9);
    assert!(matches!(
        execute_simulator(&transaction, &mut source, "wrong-responder", 10_000),
        Err(ExecutionError::UnexpectedResponder { expected, actual })
            if expected.value() == 0x7e8 && actual.value() == 0x7e9
    ));
}

#[test]
fn integration_rejects_wrong_sid_info_type_malformed_and_invalid_isotp_sequence() {
    let transaction = prepared();
    let mut wrong_sid = response_payload();
    wrong_sid[0] = 0x48;
    assert!(matches!(
        execute_simulator(
            &transaction,
            &mut simulator(wrong_sid, 0x7e8),
            "wrong-sid",
            10_000
        ),
        Err(ExecutionError::J1979(
            J1979Error::UnexpectedPositiveSid { .. }
        ))
    ));

    let mut wrong_info = response_payload();
    wrong_info[1] = 0x02;
    assert!(matches!(
        execute_simulator(
            &transaction,
            &mut simulator(wrong_info, 0x7e8),
            "wrong-info",
            10_000
        ),
        Err(ExecutionError::J1979(J1979Error::UnexpectedInfoType { .. }))
    ));

    let mut malformed = response_payload();
    malformed[2] = 2;
    assert!(matches!(
        execute_simulator(
            &transaction,
            &mut simulator(malformed, 0x7e8),
            "malformed",
            10_000
        ),
        Err(ExecutionError::J1979(
            J1979Error::InvalidResponseLength { .. }
        ))
    ));

    let request = J1979Request::calibration_identification().encoded();
    let mut invalid_sequence = SimulatorSource::script_payload(
        PayloadSimulatorScenario {
            expected_request_payload: request.to_vec(),
            behavior: PayloadSimulatorBehavior::MalformedSequence {
                response_payload: response_payload(),
            },
        },
        &request,
        "hs-can",
        CanId::standard(0x7e8).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        execute_simulator(
            &transaction,
            &mut invalid_sequence,
            "invalid-isotp-sequence",
            10_000
        ),
        Err(ExecutionError::IsoTp(
            isotp::IsoTpError::SequenceMismatch { .. }
        ))
    ));
}

#[test]
fn simulator_expectation_must_match_prepared_typed_request() {
    let transaction = prepared();
    let wrong_request = [0x09, 0x02];
    let mut source = SimulatorSource::script_payload(
        PayloadSimulatorScenario {
            expected_request_payload: wrong_request.to_vec(),
            behavior: PayloadSimulatorBehavior::Response {
                response_payload: response_payload(),
            },
        },
        &wrong_request,
        "hs-can",
        CanId::standard(0x7e8).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        execute_simulator(&transaction, &mut source, "request-mismatch", 10_000),
        Err(ExecutionError::Source(_))
    ));
}

#[test]
fn simulator_timeout_is_reported() {
    let transaction = prepared();
    let request = J1979Request::calibration_identification().encoded();
    let mut timeout = SimulatorSource::script_payload(
        PayloadSimulatorScenario {
            expected_request_payload: request.to_vec(),
            behavior: PayloadSimulatorBehavior::Timeout,
        },
        &request,
        "hs-can",
        CanId::standard(0x7e8).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        execute_simulator(&transaction, &mut timeout, "timeout", 10_000),
        Err(ExecutionError::Timeout)
    ));
}

#[test]
fn public_execution_surface_has_no_raw_tx_or_non_read_only_intent() {
    let source = include_str!("../src/lib.rs");
    for forbidden in [
        "pub fn execute_raw",
        "pub fn send_frame",
        "pub fn send_diagnostic",
        "pub fn send_can",
        "WriteDataByIdentifier",
        "SecurityAccess",
        "RoutineControl",
        "EcuReset",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden public surface: {forbidden}"
        );
    }
    assert!(!source.contains("wrapping_add(8)"));
    assert!(!source.contains("physical_request_id + 8"));
    assert_eq!(CanSourceKind::Simulator, CanSourceKind::Simulator);
}
