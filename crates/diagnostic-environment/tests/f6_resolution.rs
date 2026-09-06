use diagnostic_environment::{
    DiagnosticEnvironmentQuery, DiagnosticEnvironmentResolution, DiagnosticEnvironmentResolver,
    DiagnosticTarget, EnvironmentField,
};
use knowledge::{
    sha256_bytes, Applicability, CanIdFormat, ClaimKey, ContentFingerprint, DiagnosticSafetyClass,
    DimensionConstraint, EntityKind, EvidenceClass, EvidenceId, EvidenceRecord,
    ImplementationMarkerKind, JsonManifestAdapter, KnowledgeEntity, KnowledgeRecord,
    KnowledgeStore, KnowledgeValue, RedistributionStatus, SourceId, SourceLocator, SourceRecord,
    SourceType, ValidationState, VehicleContext, YearConstraint,
};
use std::collections::BTreeMap;

const CAPABILITY: &str = "obd.service09.pid04.calibration_id.read_only";
const ECU: &str = "ECU-FAMILY-A";
const IMPLEMENTATION: &str = "IMPL-A";
const X250_CAPABILITY: &str = "obd.service09.infotype04.calibration_id.read_only";
const X250_ECU: &str = "jlr.x250.ecm";
const X250_IMPLEMENTATION: &str = "jlr.x250.ecm.cx23-14c204-zad";
const X250_OBSERVATION_SHA256: &str =
    "310ff355a053adc3430269c4903084c619b95a38e4d765ff87a767eac579641b";
const X250_OBSERVATION: &[u8] =
    include_bytes!("../../../fixtures/knowledge/captured/x250_obd_fusion_mode09.normalized.txt");

fn context(program: &str, year: u16) -> VehicleContext {
    VehicleContext {
        vehicle_program: Some(program.into()),
        model_year: Some(year),
        architecture_generation: None,
        ecu_family: Some(ECU.into()),
        powertrain: None,
        variant: None,
        market: None,
        diagnostic_implementation: Some(IMPLEMENTATION.into()),
        other: BTreeMap::new(),
    }
}

fn applicability(
    programs: &[&str],
    implementation: &str,
    ecu: &str,
    market: DimensionConstraint,
) -> Applicability {
    Applicability {
        vehicle_program: DimensionConstraint::one_of(
            programs.iter().map(|value| (*value).to_owned()),
        )
        .unwrap(),
        model_year: YearConstraint::Range {
            model_year_from: Some(2010),
            model_year_to: Some(2014),
        },
        architecture_generation: DimensionConstraint::Any,
        ecu_family: DimensionConstraint::one_of(vec![ecu.into()]).unwrap(),
        powertrain: DimensionConstraint::Any,
        variant: DimensionConstraint::Any,
        market,
        diagnostic_implementation: DimensionConstraint::one_of(vec![implementation.into()])
            .unwrap(),
        other: BTreeMap::new(),
    }
}

fn source_type(class: EvidenceClass) -> SourceType {
    match class {
        EvidenceClass::DirectObservation => SourceType::Captured,
        EvidenceClass::SyntheticTest => SourceType::Synthetic,
        EvidenceClass::UnverifiedResearch => SourceType::UnverifiedResearch,
        _ => SourceType::Documented,
    }
}

fn validation_state(class: EvidenceClass) -> ValidationState {
    match class {
        EvidenceClass::SyntheticTest | EvidenceClass::UnverifiedResearch => {
            ValidationState::Unverified
        }
        _ => ValidationState::SourceBacked,
    }
}

fn add_record(
    store: &mut KnowledgeStore,
    id: &str,
    value: KnowledgeValue,
    applicability: Applicability,
    evidence_class: Option<EvidenceClass>,
) {
    let class = evidence_class.unwrap_or(EvidenceClass::SyntheticTest);
    let kind = source_type(class);
    let source_id = format!("source-{id}");
    let evidence_id = format!("evidence-{id}");
    let source = SourceRecord {
        id: SourceId::new(&source_id).unwrap(),
        title: format!("Focused F6 test source {id}"),
        source_type: kind,
        origin: "F6 focused test input".into(),
        source_locator: format!("inline:{id}"),
        content_fingerprint: kind
            .is_real_evidence()
            .then(|| ContentFingerprint::sha256(sha256_bytes(id.as_bytes())).unwrap()),
        acquired_on: None,
        declared_vehicle_programs: vec![],
        provenance: "Generic test-only evidence; no JLR factual claim".into(),
        redistribution_status: RedistributionStatus::Permitted,
        notes: None,
    };
    store.register_source(source).unwrap();
    store
        .add_evidence(EvidenceRecord {
            id: EvidenceId::new(&evidence_id).unwrap(),
            source_id: SourceId::new(&source_id).unwrap(),
            evidence_class,
            locator: SourceLocator {
                description: format!("record {id}"),
                document_page: None,
                document_section: None,
                record_key: Some(id.into()),
                capture_timestamp_us: None,
            },
            excerpt: None,
            notes: None,
        })
        .unwrap();
    let key = match &value {
        KnowledgeValue::NetworkRoute { .. } => ClaimKey::NetworkRoute,
        KnowledgeValue::BackendRoute { .. } => ClaimKey::Custom {
            name: "backend_route".into(),
        },
        KnowledgeValue::ProtocolFamily { .. } => ClaimKey::UsesProtocolFamily,
        KnowledgeValue::DiagnosticAddressing { .. } => ClaimKey::DiagnosticAddressing,
        KnowledgeValue::Capability { name, .. } => ClaimKey::SupportsCapability {
            capability: name.clone(),
        },
        _ => ClaimKey::Custom {
            name: "test_fact".into(),
        },
    };
    store
        .add_record(KnowledgeRecord {
            id: format!("record-{id}"),
            entity: KnowledgeEntity {
                kind: EntityKind::DiagnosticImplementation,
                id: IMPLEMENTATION.into(),
            },
            key,
            value,
            applicability,
            evidence_ids: vec![EvidenceId::new(evidence_id).unwrap()],
            validation_state: validation_state(class),
        })
        .unwrap();
}

fn add_complete_environment(
    store: &mut KnowledgeStore,
    prefix: &str,
    programs: &[&str],
    market: DimensionConstraint,
    include_bitrate: bool,
) {
    let applies = || applicability(programs, IMPLEMENTATION, ECU, market.clone());
    add_record(
        store,
        &format!("{prefix}-route"),
        KnowledgeValue::NetworkRoute {
            logical_name: "HS-CAN".into(),
            connector: Some("DLC".into()),
            pins: vec![6, 14],
            bitrate_bps: include_bitrate.then_some(500_000),
        },
        applies(),
        Some(EvidenceClass::SyntheticTest),
    );
    add_record(
        store,
        &format!("{prefix}-backend"),
        KnowledgeValue::BackendRoute {
            backend: "TEST-BACKEND".into(),
            route_id: "hs-can".into(),
        },
        applies(),
        Some(EvidenceClass::SyntheticTest),
    );
    add_record(
        store,
        &format!("{prefix}-protocol"),
        KnowledgeValue::ProtocolFamily {
            name: "ISO15765-4".into(),
        },
        applies(),
        Some(EvidenceClass::SyntheticTest),
    );
    add_record(
        store,
        &format!("{prefix}-address"),
        KnowledgeValue::DiagnosticAddressing {
            request_id: Some(0x7e0),
            response_id: Some(0x7e8),
            functional_request_id: Some(0x7df),
            can_id_format: Some(CanIdFormat::Standard11Bit),
            addressing_mode: Some("normal_physical".into()),
        },
        applies(),
        Some(EvidenceClass::SyntheticTest),
    );
    add_record(
        store,
        &format!("{prefix}-capability"),
        KnowledgeValue::Capability {
            name: CAPABILITY.into(),
            supported: true,
            safety_class: Some(DiagnosticSafetyClass::ReadOnly),
        },
        applies(),
        Some(EvidenceClass::SyntheticTest),
    );
    add_record(
        store,
        &format!("{prefix}-calibration"),
        KnowledgeValue::ImplementationMarker {
            marker_kind: ImplementationMarkerKind::CalibrationId,
            value: "SYNTHETIC-CALIBRATION".into(),
        },
        applies(),
        Some(EvidenceClass::SyntheticTest),
    );
}

fn query(program: &str, year: u16) -> DiagnosticEnvironmentQuery {
    DiagnosticEnvironmentQuery::new(
        context(program, year),
        DiagnosticTarget::implementation(ECU, IMPLEMENTATION, CAPABILITY).unwrap(),
    )
    .unwrap()
}

fn x250_context() -> VehicleContext {
    VehicleContext {
        vehicle_program: Some("X250".into()),
        model_year: Some(2010),
        architecture_generation: Some("X250".into()),
        ecu_family: Some(X250_ECU.into()),
        powertrain: Some("AJ133-5.0L-SC".into()),
        variant: Some("Supercharged".into()),
        market: Some("NAS".into()),
        diagnostic_implementation: Some(X250_IMPLEMENTATION.into()),
        other: BTreeMap::new(),
    }
}

fn x250_applicability() -> Applicability {
    Applicability {
        vehicle_program: DimensionConstraint::one_of(vec!["X250".into()]).unwrap(),
        model_year: YearConstraint::Range {
            model_year_from: Some(2010),
            model_year_to: Some(2010),
        },
        architecture_generation: DimensionConstraint::one_of(vec!["X250".into()]).unwrap(),
        ecu_family: DimensionConstraint::one_of(vec![X250_ECU.into()]).unwrap(),
        powertrain: DimensionConstraint::one_of(vec!["AJ133-5.0L-SC".into()]).unwrap(),
        variant: DimensionConstraint::one_of(vec!["Supercharged".into()]).unwrap(),
        market: DimensionConstraint::one_of(vec!["NAS".into()]).unwrap(),
        diagnostic_implementation: DimensionConstraint::one_of(vec![X250_IMPLEMENTATION.into()])
            .unwrap(),
        other: BTreeMap::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn register_real_source(
    store: &mut KnowledgeStore,
    id: &str,
    title: &str,
    source_type: SourceType,
    origin: &str,
    source_locator: &str,
    sha256: &str,
    declared_vehicle_programs: &[&str],
    provenance: &str,
    redistribution_status: RedistributionStatus,
    notes: &str,
) {
    store
        .register_source(SourceRecord {
            id: SourceId::new(id).unwrap(),
            title: title.into(),
            source_type,
            origin: origin.into(),
            source_locator: source_locator.into(),
            content_fingerprint: Some(ContentFingerprint::sha256(sha256).unwrap()),
            acquired_on: Some("2026-08-31".into()),
            declared_vehicle_programs: declared_vehicle_programs
                .iter()
                .map(|program| (*program).into())
                .collect(),
            provenance: provenance.into(),
            redistribution_status,
            notes: Some(notes.into()),
        })
        .unwrap();
}

#[allow(clippy::too_many_arguments)]
fn add_real_evidence(
    store: &mut KnowledgeStore,
    id: &str,
    source_id: &str,
    evidence_class: EvidenceClass,
    description: &str,
    document_page: Option<u32>,
    document_section: Option<&str>,
    record_key: Option<&str>,
    excerpt: &str,
    notes: &str,
) -> EvidenceId {
    let evidence_id = EvidenceId::new(id).unwrap();
    store
        .add_evidence(EvidenceRecord {
            id: evidence_id.clone(),
            source_id: SourceId::new(source_id).unwrap(),
            evidence_class: Some(evidence_class),
            locator: SourceLocator {
                description: description.into(),
                document_page,
                document_section: document_section.map(str::to_owned),
                record_key: record_key.map(str::to_owned),
                capture_timestamp_us: None,
            },
            excerpt: Some(excerpt.into()),
            notes: Some(notes.into()),
        })
        .unwrap();
    evidence_id
}

fn add_x250_record(
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
                id: X250_IMPLEMENTATION.into(),
            },
            key,
            value,
            applicability: x250_applicability(),
            evidence_ids,
            validation_state,
        })
        .unwrap();
}

#[test]
fn complete_resolution_is_typed_deterministic_and_traced() {
    let mut store = KnowledgeStore::new();
    add_complete_environment(
        &mut store,
        "complete",
        &["PROGRAM-A", "PROGRAM-B"],
        DimensionConstraint::Any,
        true,
    );

    let first = DiagnosticEnvironmentResolver::resolve(&store, &query("PROGRAM-A", 2012));
    let second = DiagnosticEnvironmentResolver::resolve(&store, &query("PROGRAM-A", 2012));
    assert_eq!(first, second);

    let DiagnosticEnvironmentResolution::Resolved(plan) = first else {
        panic!("complete environment did not resolve");
    };
    assert_eq!(plan.logical_network.value, "HS-CAN");
    assert_eq!(plan.physical_route.value.pins, vec![6, 14]);
    assert_eq!(plan.bitrate_bps.value, 500_000);
    assert_eq!(plan.can_id_format.value, CanIdFormat::Standard11Bit);
    assert_eq!(plan.physical_request_id.value, 0x7e0);
    assert_eq!(plan.physical_response_id.value, 0x7e8);
    assert_eq!(plan.functional_request_id.unwrap().value, 0x7df);
    assert_eq!(plan.read_only_capability.value.id, CAPABILITY);
    assert_eq!(
        plan.implementation_markers[&ImplementationMarkerKind::CalibrationId].value,
        "SYNTHETIC-CALIBRATION"
    );

    for evidence in [
        &plan.vehicle_applicability.evidence,
        &plan.ecu_family.evidence,
        &plan.diagnostic_implementation.as_ref().unwrap().evidence,
        &plan.logical_network.evidence,
        &plan.physical_route.evidence,
        &plan.backend_route.evidence,
        &plan.bitrate_bps.evidence,
        &plan.protocol_family.evidence,
        &plan.addressing_mode.evidence,
        &plan.can_id_format.evidence,
        &plan.physical_request_id.evidence,
        &plan.physical_response_id.evidence,
        &plan.read_only_capability.evidence,
        &plan.implementation_markers[&ImplementationMarkerKind::CalibrationId].evidence,
    ] {
        assert!(!evidence.is_empty());
        assert!(evidence.iter().all(|trace| trace.evidence_class.is_some()));
    }

    assert!(matches!(
        DiagnosticEnvironmentResolver::resolve(&store, &query("PROGRAM-B", 2014)),
        DiagnosticEnvironmentResolution::Resolved(_)
    ));

    let capability_only = DiagnosticEnvironmentQuery::new(
        context("PROGRAM-A", 2012),
        DiagnosticTarget {
            capability_id: Some(CAPABILITY.into()),
            ..DiagnosticTarget::default()
        },
    )
    .unwrap();
    assert!(matches!(
        DiagnosticEnvironmentResolver::resolve(&store, &capability_only),
        DiagnosticEnvironmentResolution::Resolved(_)
    ));
}

#[test]
fn missing_bitrate_and_unknown_applicability_fail_closed() {
    let mut missing = KnowledgeStore::new();
    add_complete_environment(
        &mut missing,
        "missing",
        &["PROGRAM-A"],
        DimensionConstraint::Any,
        false,
    );
    let DiagnosticEnvironmentResolution::Indeterminate {
        unresolved_facts, ..
    } = DiagnosticEnvironmentResolver::resolve(&missing, &query("PROGRAM-A", 2012))
    else {
        panic!("missing bitrate did not fail closed");
    };
    assert!(unresolved_facts
        .iter()
        .any(|fact| fact.field == EnvironmentField::Bitrate));

    let mut unknown = KnowledgeStore::new();
    add_complete_environment(
        &mut unknown,
        "unknown",
        &["PROGRAM-A"],
        DimensionConstraint::Unknown,
        true,
    );
    assert!(matches!(
        DiagnosticEnvironmentResolver::resolve(&unknown, &query("PROGRAM-A", 2012)),
        DiagnosticEnvironmentResolution::Indeterminate { .. }
    ));
}

#[test]
fn conflicting_request_ids_are_returned_without_selection() {
    let mut store = KnowledgeStore::new();
    add_complete_environment(
        &mut store,
        "conflict",
        &["PROGRAM-A"],
        DimensionConstraint::Any,
        true,
    );
    add_record(
        &mut store,
        "conflict-address-b",
        KnowledgeValue::DiagnosticAddressing {
            request_id: Some(0x700),
            response_id: Some(0x7e8),
            functional_request_id: Some(0x7df),
            can_id_format: Some(CanIdFormat::Standard11Bit),
            addressing_mode: Some("normal_physical".into()),
        },
        applicability(
            &["PROGRAM-A"],
            IMPLEMENTATION,
            ECU,
            DimensionConstraint::Any,
        ),
        Some(EvidenceClass::SyntheticTest),
    );
    let DiagnosticEnvironmentResolution::Conflict { conflicts, .. } =
        DiagnosticEnvironmentResolver::resolve(&store, &query("PROGRAM-A", 2012))
    else {
        panic!("conflicting address was not reported");
    };
    let request = conflicts
        .iter()
        .find(|conflict| conflict.field == EnvironmentField::PhysicalRequestId)
        .unwrap();
    assert_eq!(request.candidates.len(), 2);
}

#[test]
fn incompatible_implementation_cannot_supply_missing_route() {
    let mut store = KnowledgeStore::new();
    let applies_a = || {
        applicability(
            &["PROGRAM-A"],
            IMPLEMENTATION,
            ECU,
            DimensionConstraint::Any,
        )
    };
    for (id, value) in [
        (
            "protocol-a",
            KnowledgeValue::ProtocolFamily {
                name: "ISO15765-4".into(),
            },
        ),
        (
            "backend-a",
            KnowledgeValue::BackendRoute {
                backend: "TEST-BACKEND".into(),
                route_id: "hs-can".into(),
            },
        ),
    ] {
        add_record(
            &mut store,
            id,
            value,
            applies_a(),
            Some(EvidenceClass::SyntheticTest),
        );
    }
    add_record(
        &mut store,
        "route-b",
        KnowledgeValue::NetworkRoute {
            logical_name: "HS-CAN".into(),
            connector: Some("DLC".into()),
            pins: vec![6, 14],
            bitrate_bps: Some(500_000),
        },
        applicability(
            &["PROGRAM-A"],
            "IMPL-B",
            "ECU-FAMILY-B",
            DimensionConstraint::Any,
        ),
        Some(EvidenceClass::SyntheticTest),
    );
    let DiagnosticEnvironmentResolution::Indeterminate {
        unresolved_facts, ..
    } = DiagnosticEnvironmentResolver::resolve(&store, &query("PROGRAM-A", 2012))
    else {
        panic!("invalid multi-source join was accepted");
    };
    assert!(unresolved_facts
        .iter()
        .any(|fact| fact.field == EnvironmentField::PhysicalRoute));
}

#[test]
fn request_documentation_and_observed_response_keep_distinct_traces() {
    let mut store = KnowledgeStore::new();
    let applies = || {
        applicability(
            &["PROGRAM-A"],
            IMPLEMENTATION,
            ECU,
            DimensionConstraint::Any,
        )
    };
    for (id, value, class) in [
        (
            "observed-route",
            KnowledgeValue::NetworkRoute {
                logical_name: "HS-CAN".into(),
                connector: Some("DLC".into()),
                pins: vec![6, 14],
                bitrate_bps: Some(500_000),
            },
            EvidenceClass::OemDocumentation,
        ),
        (
            "observed-backend",
            KnowledgeValue::BackendRoute {
                backend: "TEST-BACKEND".into(),
                route_id: "hs-can".into(),
            },
            EvidenceClass::SourceCode,
        ),
        (
            "observed-protocol",
            KnowledgeValue::ProtocolFamily {
                name: "ISO15765-4".into(),
            },
            EvidenceClass::StandardDocumentation,
        ),
        (
            "observed-request",
            KnowledgeValue::DiagnosticAddressing {
                request_id: Some(0x7e0),
                response_id: None,
                functional_request_id: None,
                can_id_format: Some(CanIdFormat::Standard11Bit),
                addressing_mode: Some("normal_physical".into()),
            },
            EvidenceClass::StandardDocumentation,
        ),
        (
            "observed-response",
            KnowledgeValue::DiagnosticAddressing {
                request_id: None,
                response_id: Some(0x7e8),
                functional_request_id: None,
                can_id_format: None,
                addressing_mode: None,
            },
            EvidenceClass::DirectObservation,
        ),
        (
            "observed-capability",
            KnowledgeValue::Capability {
                name: CAPABILITY.into(),
                supported: true,
                safety_class: Some(DiagnosticSafetyClass::ReadOnly),
            },
            EvidenceClass::StandardDocumentation,
        ),
    ] {
        add_record(&mut store, id, value, applies(), Some(class));
    }

    let DiagnosticEnvironmentResolution::Resolved(plan) =
        DiagnosticEnvironmentResolver::resolve(&store, &query("PROGRAM-A", 2012))
    else {
        panic!("generic mixed-evidence environment did not resolve");
    };
    assert!(plan
        .physical_request_id
        .evidence
        .iter()
        .all(|trace| trace.evidence_class == Some(EvidenceClass::StandardDocumentation)));
    assert!(plan
        .physical_response_id
        .evidence
        .iter()
        .all(|trace| trace.evidence_class == Some(EvidenceClass::DirectObservation)));
}

#[test]
fn real_x250_mode09_positive_golden_resolves_with_distinct_source_roles() {
    assert_eq!(sha256_bytes(X250_OBSERVATION), X250_OBSERVATION_SHA256);

    let mut store = KnowledgeStore::new();
    register_real_source(
        &mut store,
        "public-jaguarforums-x250-obd-fusion-2016-03",
        "2010 Jaguar XF Supercharged OBD Fusion diagnostic reports",
        SourceType::Captured,
        "JaguarForums public posts 159551 and 159757",
        "repository:fixtures/knowledge/captured/x250_obd_fusion_mode09.normalized.txt",
        X250_OBSERVATION_SHA256,
        &["X250"],
        "Minimal normalized public search-index snapshot; direct forum fetch returned Cloudflare Error 1005 and the original attachment was unavailable",
        RedistributionStatus::RestrictedMetadataOnly,
        "Only VIN, report identity, Mode 09 responder/calibration and CVN facts are retained",
    );
    register_real_source(
        &mut store,
        "jlr-jtb00244nas1-2012-02-24",
        "Jaguar Technical Bulletin JTB00244NAS1",
        SourceType::Documented,
        "Jaguar Land Rover North America, LLC",
        "https://static.nhtsa.gov/odi/tsbs/2012/SB-10043530-3423.pdf",
        "ff4d405e212b5e1f973c41c08aabd929043b7ec084da5d212af19b9ca4347202",
        &["X250"],
        "Official Jaguar bulletin dated 24 FEB 2012, acquired from NHTSA",
        RedistributionStatus::RestrictedMetadataOnly,
        "Original PDF is retained locally but is not redistributed by this repository",
    );
    register_real_source(
        &mut store,
        "charm-jaguar-2010-x250-v8-5.0l-sc",
        "2010 Jaguar XF (X250) V8-5.0L SC service manual offline bundle",
        SourceType::Documented,
        "Operation CHARM exact Jaguar 2010 X250 5.0L SC bundle",
        "https://charm.li/bundle/Jaguar/2010/XF%20%28X250%29%20V8-5.0L%20SC/",
        "e09b54db2571a984190a504ab7c7f4f38eefb842d496f0286dc6d0676ff3106e",
        &["X250"],
        "Exact offline bundle link extracted from the matching CHARM model page",
        RedistributionStatus::RestrictedMetadataOnly,
        "The 697007994-byte copyrighted bundle is retained locally and not committed",
    );
    register_real_source(
        &mut store,
        "california-bar-dad-2012-v2.5",
        "BAR OBD Inspection System Data Acquisition Device Specification",
        SourceType::Documented,
        "California Department of Consumer Affairs, Bureau of Automotive Repair",
        "https://www.bar.ca.gov/pdf/publications/DAD-2012.pdf",
        "9138a95fbfef44483c7efd1149405d93e98736f89b31a2eb9b9670b0bd4d2a9c",
        &[],
        "October 2012 V2.5 public regulatory specification",
        RedistributionStatus::RestrictedMetadataOnly,
        "Original PDF is retained locally and not committed",
    );
    register_real_source(
        &mut store,
        "elm-electronics-elm327dsj",
        "ELM327DSJ OBD to RS232 Interpreter datasheet",
        SourceType::Documented,
        "Elm Electronics",
        "https://www.elmelectronics.com/wp-content/uploads/2016/07/ELM327DS.pdf",
        "43cab005c9e1678325a3ba64066db93e09e2a78a23eb5fa360ee1eaf9a585428",
        &[],
        "Official Elm Electronics datasheet reached through the requested public URL",
        RedistributionStatus::RestrictedMetadataOnly,
        "Used only for ISO15765-4 11-bit addressing semantics",
    );
    register_real_source(
        &mut store,
        "jlr-scanner-mongoose-jlr-passive-routes-8af1491",
        "JLR Scanner Mongoose JLR production route descriptors",
        SourceType::Documented,
        "JLR Scanner source code at F6 evidence acquisition baseline",
        "repository:crates/mongoose-jlr/src/passive.rs:74,79-84",
        "79180a11e16f2398cec8d08702ba369e0935517e92da8835774e327b3d16998c",
        &["X250"],
        "Production source descriptor independently audited against MongoosePro JLR hardware evidence",
        RedistributionStatus::Permitted,
        "The descriptor is receive-only and does not create an application CAN TX API",
    );

    let observed = add_real_evidence(
        &mut store,
        "evidence-x250-obd-fusion-mode09-7e8",
        "public-jaguarforums-x250-obd-fusion-2016-03",
        EvidenceClass::DirectObservation,
        "Primary report 2016-03-15 and repeated report 2016-03-19, Mode $09 Vehicle Information",
        None,
        Some("Mode $09 - Vehicle Information"),
        Some("VIN/Calibration ID - $7E8"),
        "VIN SAJWA0HE4AMR59890; Calibration ID - $7E8 CX23-14C204-ZAD; CVN - $7E8 AE746751",
        "No request frame is observed; 0x7E0 and 0x7DF are not derived from this evidence",
    );
    let tsb = add_real_evidence(
        &mut store,
        "evidence-jtb00244nas1-x250-ecm",
        "jlr-jtb00244nas1-2012-02-24",
        EvidenceClass::OemDocumentation,
        "PDF page 1 affected range/action and PDF page 2 repair procedure steps 3-9",
        Some(1),
        Some("AFFECTED VEHICLE RANGE / REPAIR PROCEDURE"),
        Some("XF X250 5.0L MY2010-2012 / ECM"),
        "XF (X250) 5.0L only, MY2010-2012, VIN R47154-S38706; update Engine control module software through IDS/SDD",
        "The bulletin proves applicability and ECM diagnostic relationship, not CAN route or IDs",
    );
    let charm_topology = add_real_evidence(
        &mut store,
        "evidence-charm-x250-ecm-hs-can",
        "charm-jaguar-2010-x250-v8-5.0l-sc",
        EvidenceClass::OemDocumentation,
        "pages/7013.html; control diagram image 461775832.jpeg; legend 462197295.png",
        None,
        Some("Communications Network / CONTROL DIAGRAM - HIGH SPEED CAN BUS"),
        Some("items 3 Diagnostic socket and 9 ECM"),
        "D = High speed CAN bus; item 3 Diagnostic socket; item 9 ECM",
        "Exact 2010 X250 V8-5.0L SC bundle",
    );
    let charm_bitrate = add_real_evidence(
        &mut store,
        "evidence-charm-x250-hs-can-bitrate",
        "charm-jaguar-2010-x250-v8-5.0l-sc",
        EvidenceClass::OemDocumentation,
        "pages/7012.html; Communications Network table image 460639383.png",
        None,
        Some("Communications Network / OVERVIEW"),
        Some("High speed CAN bus / Baud Rate"),
        "High speed CAN (controller area network) bus: 500 kbits/s",
        "Exact-model network table",
    );
    let charm_route = add_real_evidence(
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
    let bar_mode09 = add_real_evidence(
        &mut store,
        "evidence-bar-mode09-infotype04",
        "california-bar-dad-2012-v2.5",
        EvidenceClass::StandardDocumentation,
        "PDF page 14 / printed page 13, requirement 3.2.46",
        Some(14),
        Some("Acquiring Mode $09, InfoTypes $03 and $04 Calibration Identification"),
        Some("3.2.46"),
        "CAL ID is retrieved with Mode $09 InfoType $04 in accordance with SAE J1979",
        "Standard meaning only; no X250 applicability is inferred from BAR",
    );
    let elm_addressing = add_real_evidence(
        &mut store,
        "evidence-elm-iso15765-11bit-addressing",
        "elm-electronics-elm327dsj",
        EvidenceClass::StandardDocumentation,
        "PDF page 41 of 94, Setting the Headers (continued)",
        Some(41),
        Some("11 bit ISO15765-4 CAN"),
        Some("7DF/7En/7E0-to-7E8"),
        "11-bit ISO15765-4 defines functional request 7DF and physical 7En; 7E0 addresses the ECU responding as 7E8",
        "Address roles are standards-based; response 0x7E8 remains independently observed",
    );
    let backend = add_real_evidence(
        &mut store,
        "evidence-mongoose-jlr-hs-can-route",
        "jlr-scanner-mongoose-jlr-passive-routes-8af1491",
        EvidenceClass::SourceCode,
        "crates/mongoose-jlr/src/passive.rs lines 74 and 79-84",
        None,
        Some("VEHICLE_ROUTES"),
        Some("VehicleRouteId::HsCan"),
        "route hs-can; X250 HS-CAN; OBD pins 6/14; bitrate 500000; PassiveCapability::Ready",
        "Source proves an implemented receive-only backend descriptor, not vehicle validation or TX",
    );

    add_x250_record(
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
    add_x250_record(
        &mut store,
        "record-x250-positive-hs-can-route",
        ClaimKey::NetworkRoute,
        KnowledgeValue::NetworkRoute {
            logical_name: "HS-CAN".into(),
            connector: Some("J1962/C2DB04B".into()),
            pins: vec![6, 14],
            bitrate_bps: Some(500_000),
        },
        vec![charm_topology, charm_bitrate, charm_route],
        ValidationState::SourceBacked,
    );
    add_x250_record(
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
    add_x250_record(
        &mut store,
        "record-x250-positive-protocol",
        ClaimKey::UsesProtocolFamily,
        KnowledgeValue::ProtocolFamily {
            name: "ISO15765-4 / SAE J1979".into(),
        },
        vec![elm_addressing.clone()],
        ValidationState::SourceBacked,
    );
    add_x250_record(
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
        vec![elm_addressing],
        ValidationState::SourceBacked,
    );
    add_x250_record(
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
    add_x250_record(
        &mut store,
        "record-x250-positive-mode09-calibration-id",
        ClaimKey::SupportsCapability {
            capability: X250_CAPABILITY.into(),
        },
        KnowledgeValue::Capability {
            name: X250_CAPABILITY.into(),
            supported: true,
            safety_class: Some(DiagnosticSafetyClass::ReadOnly),
        },
        vec![bar_mode09],
        ValidationState::SourceBacked,
    );
    add_x250_record(
        &mut store,
        "record-x250-positive-calibration",
        ClaimKey::Custom {
            name: "implementation_marker.calibration_id".into(),
        },
        KnowledgeValue::ImplementationMarker {
            marker_kind: ImplementationMarkerKind::CalibrationId,
            value: "CX23-14C204-ZAD".into(),
        },
        vec![observed],
        ValidationState::CaptureValidated,
    );

    let target =
        DiagnosticTarget::implementation(X250_ECU, X250_IMPLEMENTATION, X250_CAPABILITY).unwrap();
    let query = DiagnosticEnvironmentQuery::new(x250_context(), target).unwrap();
    let DiagnosticEnvironmentResolution::Resolved(plan) =
        DiagnosticEnvironmentResolver::resolve(&store, &query)
    else {
        panic!("real X250 Mode 09 positive golden did not resolve");
    };

    assert_eq!(plan.logical_network.value, "HS-CAN");
    assert_eq!(plan.physical_route.value.connector, "J1962/C2DB04B");
    assert_eq!(plan.physical_route.value.pins, vec![6, 14]);
    assert_eq!(plan.bitrate_bps.value, 500_000);
    assert_eq!(plan.backend_route.value.backend, "mongoose-jlr");
    assert_eq!(plan.backend_route.value.route_id, "hs-can");
    assert_eq!(plan.protocol_family.value, "ISO15765-4 / SAE J1979");
    assert_eq!(plan.can_id_format.value, CanIdFormat::Standard11Bit);
    assert_eq!(plan.functional_request_id.as_ref().unwrap().value, 0x7df);
    assert_eq!(plan.physical_request_id.value, 0x7e0);
    assert_eq!(plan.physical_response_id.value, 0x7e8);
    assert_eq!(
        plan.implementation_markers[&ImplementationMarkerKind::CalibrationId].value,
        "CX23-14C204-ZAD"
    );
    assert!(plan
        .physical_request_id
        .evidence
        .iter()
        .all(|trace| trace.evidence_class == Some(EvidenceClass::StandardDocumentation)));
    assert!(plan
        .functional_request_id
        .as_ref()
        .unwrap()
        .evidence
        .iter()
        .all(|trace| trace.evidence_class == Some(EvidenceClass::StandardDocumentation)));
    assert!(plan
        .physical_response_id
        .evidence
        .iter()
        .all(|trace| trace.evidence_class == Some(EvidenceClass::DirectObservation)));
    assert!(
        plan.implementation_markers[&ImplementationMarkerKind::CalibrationId]
            .evidence
            .iter()
            .all(|trace| trace.evidence_class == Some(EvidenceClass::DirectObservation))
    );
    assert!(plan
        .backend_route
        .evidence
        .iter()
        .all(|trace| trace.evidence_class == Some(EvidenceClass::SourceCode)));
}

#[test]
fn real_x250_ccp_route_remains_indeterminate_without_defaults() {
    const CCP: &str = include_str!("../../../fixtures/knowledge/documented/x250_ccp_route.json");
    let mut store = KnowledgeStore::new();
    store.ingest(&JsonManifestAdapter, CCP).unwrap();
    let result = store.query(
        &knowledge::KnowledgeQuery::for_vehicle(VehicleContext {
            vehicle_program: Some("X250".into()),
            ..VehicleContext::default()
        })
        .include_indeterminate(true),
    );
    let route = result
        .records
        .iter()
        .find(|record| record.record.entity.id == "x250.ccp_hs_can.diagnostic_connector")
        .unwrap();
    assert!(matches!(
        &route.record.value,
        KnowledgeValue::NetworkRoute {
            connector: Some(connector),
            pins,
            bitrate_bps: None,
            ..
        } if connector == "C2DB04B" && pins == &vec![12, 13]
    ));

    let query = DiagnosticEnvironmentQuery::new(
        VehicleContext {
            vehicle_program: Some("X250".into()),
            ..VehicleContext::default()
        },
        DiagnosticTarget::entity(KnowledgeEntity {
            kind: EntityKind::NetworkRoute,
            id: "x250.ccp_hs_can.diagnostic_connector".into(),
        })
        .unwrap(),
    )
    .unwrap();
    let DiagnosticEnvironmentResolution::Indeterminate {
        partial,
        unresolved_facts,
    } = DiagnosticEnvironmentResolver::resolve(&store, &query)
    else {
        panic!("incomplete CCP route did not remain indeterminate");
    };
    for field in [
        EnvironmentField::Bitrate,
        EnvironmentField::ProtocolFamily,
        EnvironmentField::PhysicalRequestId,
        EnvironmentField::PhysicalResponseId,
        EnvironmentField::BackendRoute,
    ] {
        assert!(unresolved_facts.iter().any(|fact| fact.field == field));
    }
    assert!(partial.bitrate_bps.is_none());
    assert!(partial.protocol_family.is_none());
    assert!(partial.physical_request_id.is_none());
    assert!(partial.physical_response_id.is_none());
    assert!(partial.backend_route.is_none());
}

#[test]
fn can_id_width_validation_is_explicit() {
    assert!(KnowledgeValue::DiagnosticAddressing {
        request_id: Some(0x800),
        response_id: None,
        functional_request_id: None,
        can_id_format: Some(CanIdFormat::Standard11Bit),
        addressing_mode: Some("normal_physical".into()),
    }
    .validate()
    .is_err());
    assert!(KnowledgeValue::DiagnosticAddressing {
        request_id: Some(0x18da_10f1),
        response_id: None,
        functional_request_id: None,
        can_id_format: Some(CanIdFormat::Extended29Bit),
        addressing_mode: Some("normal_fixed".into()),
    }
    .validate()
    .is_ok());
}
