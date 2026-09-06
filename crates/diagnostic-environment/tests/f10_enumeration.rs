//! F10 golden tests for ECU family enumeration.
//!
//! Enumeration answers "what is fitted and what do we still not know". It must
//! never imply permission: a complete route is still only a route, and the
//! resolver remains the thing that decides whether anything may be prepared.

use diagnostic_environment::{DiagnosticEnvironmentResolver, EcuFamilyPresence};
use knowledge::{
    sha256_bytes, Applicability, ApplicabilityResolution, ClaimKey, ContentFingerprint,
    DimensionConstraint, EntityKind, EvidenceId, EvidenceRecord, KnowledgeEntity, KnowledgeRecord,
    KnowledgeStore, KnowledgeValue, RedistributionStatus, SourceId, SourceLocator, SourceRecord,
    SourceType, ValidationState, VehicleContext, YearConstraint,
};
use std::collections::BTreeMap;

fn store() -> KnowledgeStore {
    let mut store = KnowledgeStore::new();
    store
        .register_source(SourceRecord {
            id: SourceId::new("f10-src").unwrap(),
            title: "F10 enumeration fixture".into(),
            source_type: SourceType::Documented,
            origin: "F10 golden test".into(),
            source_locator: "inline:f10".into(),
            content_fingerprint: Some(
                ContentFingerprint::sha256(sha256_bytes(b"f10-enumeration")).unwrap(),
            ),
            acquired_on: None,
            declared_vehicle_programs: vec![],
            provenance: "Constructed in-test; proves resolver behaviour only".into(),
            redistribution_status: RedistributionStatus::Permitted,
            notes: None,
        })
        .unwrap();

    // PROGA: a complete diagnostic pair plus a stated bus.
    add(
        &mut store,
        "e1",
        "r1",
        "PCM",
        "PROGA",
        Some(0x7E0),
        Some(0x7E8),
    );
    network(&mut store, "e2", "r2", "PCM", "PROGA", "CAN_HS");
    // PROGA: only one direction known.
    add(&mut store, "e3", "r3", "ABS", "PROGA", Some(0x760), None);
    // A different vehicle programme entirely.
    add(
        &mut store,
        "e4",
        "r4",
        "TCM",
        "PROGB",
        Some(0x7E1),
        Some(0x7E9),
    );
    store
}

#[allow(clippy::too_many_arguments)]
fn add(
    store: &mut KnowledgeStore,
    evidence_id: &str,
    record_id: &str,
    family: &str,
    program: &str,
    request_id: Option<u32>,
    response_id: Option<u32>,
) {
    push(
        store,
        evidence_id,
        record_id,
        family,
        program,
        ClaimKey::DiagnosticAddressing,
        KnowledgeValue::DiagnosticAddressing {
            request_id,
            response_id,
            functional_request_id: None,
            can_id_format: Some(knowledge::CanIdFormat::Standard11Bit),
            addressing_mode: Some("normal".into()),
        },
    );
}

fn network(
    store: &mut KnowledgeStore,
    evidence_id: &str,
    record_id: &str,
    family: &str,
    program: &str,
    net: &str,
) {
    push(
        store,
        evidence_id,
        record_id,
        family,
        program,
        ClaimKey::Custom {
            name: "sdd_network".into(),
        },
        KnowledgeValue::Text { value: net.into() },
    );
}

fn push(
    store: &mut KnowledgeStore,
    evidence_id: &str,
    record_id: &str,
    family: &str,
    program: &str,
    key: ClaimKey,
    value: KnowledgeValue,
) {
    store
        .add_evidence(EvidenceRecord {
            id: EvidenceId::new(evidence_id).unwrap(),
            source_id: SourceId::new("f10-src").unwrap(),
            evidence_class: Some(knowledge::EvidenceClass::OemDocumentation),
            locator: SourceLocator {
                description: format!("{program}/{family}"),
                document_page: None,
                document_section: None,
                record_key: None,
                capture_timestamp_us: None,
            },
            excerpt: None,
            notes: None,
        })
        .unwrap();
    store
        .add_record(KnowledgeRecord {
            id: record_id.into(),
            entity: KnowledgeEntity {
                kind: EntityKind::EcuFamily,
                id: family.into(),
            },
            key,
            value,
            applicability: Applicability {
                vehicle_program: DimensionConstraint::one_of([program.to_string()]).unwrap(),
                model_year: YearConstraint::Any,
                architecture_generation: DimensionConstraint::Any,
                ecu_family: DimensionConstraint::Any,
                powertrain: DimensionConstraint::Any,
                variant: DimensionConstraint::Any,
                market: DimensionConstraint::Any,
                diagnostic_implementation: DimensionConstraint::Any,
                other: BTreeMap::new(),
            },
            evidence_ids: vec![EvidenceId::new(evidence_id).unwrap()],
            validation_state: ValidationState::SourceBacked,
        })
        .unwrap();
}

fn vehicle(program: &str) -> VehicleContext {
    VehicleContext {
        vehicle_program: Some(program.into()),
        model_year: Some(2010),
        ..VehicleContext::default()
    }
}

fn find<'a>(found: &'a [EcuFamilyPresence], family: &str) -> Option<&'a EcuFamilyPresence> {
    found.iter().find(|entry| entry.ecu_family == family)
}

#[test]
fn only_families_the_vehicle_can_have_are_enumerated() {
    let store = store();
    let found = DiagnosticEnvironmentResolver::enumerate_ecu_families(&store, &vehicle("PROGA"));

    let names: Vec<_> = found
        .iter()
        .map(|entry| entry.ecu_family.as_str())
        .collect();
    assert_eq!(names, vec!["ABS", "PCM"]);
    // A family belonging to another programme is excluded, not merely ranked low.
    assert!(find(&found, "TCM").is_none());
}

#[test]
fn a_route_is_reported_as_complete_only_when_both_directions_are_known() {
    let store = store();
    let found = DiagnosticEnvironmentResolver::enumerate_ecu_families(&store, &vehicle("PROGA"));

    let pcm = find(&found, "PCM").unwrap();
    assert!(pcm.has_request_id && pcm.has_response_id);
    assert!(pcm.has_complete_route());
    assert_eq!(pcm.logical_network.as_deref(), Some("CAN_HS"));
    assert_eq!(pcm.resolution, ApplicabilityResolution::Applicable);

    // Knowing where to send without knowing what answers is not a route.
    let abs = find(&found, "ABS").unwrap();
    assert!(abs.has_request_id);
    assert!(!abs.has_response_id);
    assert!(!abs.has_complete_route());
    // The bus was never stated for it, so none is assumed.
    assert_eq!(abs.logical_network, None);
}

#[test]
fn an_unknown_vehicle_yields_nothing_rather_than_everything() {
    let store = store();
    let found = DiagnosticEnvironmentResolver::enumerate_ecu_families(&store, &vehicle("PROGZ"));
    assert!(found.is_empty());

    // An empty context cannot select a vehicle, so it must not select modules.
    let bare =
        DiagnosticEnvironmentResolver::enumerate_ecu_families(&store, &VehicleContext::default());
    assert!(bare
        .iter()
        .all(|entry| entry.resolution != ApplicabilityResolution::Applicable));
}
