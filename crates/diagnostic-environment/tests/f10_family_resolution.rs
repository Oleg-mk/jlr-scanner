//! F10 golden tests for family-level resolution (ADR-0013).
//!
//! A module SDD describes has no software-build identity, so its plan records
//! the implementation as absent. Bus-level knowledge — which adapter route
//! reaches the bus — is related through the bus entity the target names.
//! Anything the store does not state stays unresolved and named.

use diagnostic_environment::{
    DiagnosticEnvironmentQuery, DiagnosticEnvironmentResolution, DiagnosticEnvironmentResolver,
    DiagnosticTarget, EnvironmentField, SDD_NETWORK_CLAIM,
};
use knowledge::{
    sha256_bytes, Applicability, ClaimKey, ContentFingerprint, DiagnosticSafetyClass,
    DimensionConstraint, EntityKind, EvidenceClass, EvidenceId, EvidenceRecord, KnowledgeEntity,
    KnowledgeRecord, KnowledgeStore, KnowledgeValue, RedistributionStatus, SourceId, SourceLocator,
    SourceRecord, SourceType, ValidationState, VehicleContext, YearConstraint,
};
use std::collections::BTreeMap;

const CAPABILITY: &str = "uds.service22.read_data_by_identifier.read_only";

fn programme_only(program: &str) -> Applicability {
    Applicability {
        vehicle_program: DimensionConstraint::one_of([program.to_string()]).unwrap(),
        model_year: YearConstraint::Any,
        architecture_generation: DimensionConstraint::Any,
        ecu_family: DimensionConstraint::Any,
        powertrain: DimensionConstraint::Any,
        variant: DimensionConstraint::Any,
        market: DimensionConstraint::Any,
        diagnostic_implementation: DimensionConstraint::Any,
        other: BTreeMap::new(),
    }
}

fn fleet_wide() -> Applicability {
    Applicability {
        vehicle_program: DimensionConstraint::Any,
        ..programme_only("unused")
    }
}

fn pcm() -> KnowledgeEntity {
    KnowledgeEntity {
        kind: EntityKind::EcuFamily,
        id: "PCM".into(),
    }
}

fn bus(name: &str) -> KnowledgeEntity {
    KnowledgeEntity {
        kind: EntityKind::NetworkRoute,
        id: name.into(),
    }
}

fn push(
    store: &mut KnowledgeStore,
    id: &str,
    entity: KnowledgeEntity,
    key: ClaimKey,
    value: KnowledgeValue,
    applicability: Applicability,
) {
    let evidence_id = format!("ev-{id}");
    store
        .add_evidence(EvidenceRecord {
            id: EvidenceId::new(&evidence_id).unwrap(),
            source_id: SourceId::new("f10-src").unwrap(),
            evidence_class: Some(EvidenceClass::OemDocumentation),
            locator: SourceLocator {
                description: id.into(),
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
            id: id.into(),
            entity,
            key,
            value,
            applicability,
            evidence_ids: vec![EvidenceId::new(&evidence_id).unwrap()],
            validation_state: ValidationState::SourceBacked,
        })
        .unwrap();
}

fn module_store() -> KnowledgeStore {
    let mut store = KnowledgeStore::new();
    store
        .register_source(SourceRecord {
            id: SourceId::new("f10-src").unwrap(),
            title: "F10 family resolution fixture".into(),
            source_type: SourceType::Documented,
            origin: "F10 golden test".into(),
            source_locator: "inline:f10-family".into(),
            content_fingerprint: Some(
                ContentFingerprint::sha256(sha256_bytes(b"f10-family")).unwrap(),
            ),
            acquired_on: None,
            declared_vehicle_programs: vec![],
            provenance: "Constructed in-test; proves resolver behaviour only".into(),
            redistribution_status: RedistributionStatus::Permitted,
            notes: None,
        })
        .unwrap();

    // What the platform adapter states about PCM on programme PROGA.
    push(
        &mut store,
        "pcm-addressing",
        pcm(),
        ClaimKey::DiagnosticAddressing,
        KnowledgeValue::DiagnosticAddressing {
            request_id: Some(0x7E0),
            response_id: Some(0x7E8),
            functional_request_id: None,
            can_id_format: Some(knowledge::CanIdFormat::Standard11Bit),
            addressing_mode: Some("normal".into()),
        },
        programme_only("PROGA"),
    );
    push(
        &mut store,
        "pcm-network",
        pcm(),
        ClaimKey::Custom {
            name: SDD_NETWORK_CLAIM.into(),
        },
        KnowledgeValue::Text {
            value: "CAN_HS".into(),
        },
        programme_only("PROGA"),
    );
    push(
        &mut store,
        "pcm-bus",
        pcm(),
        ClaimKey::NetworkRoute,
        KnowledgeValue::NetworkRoute {
            logical_name: "CAN_HS".into(),
            connector: None,
            pins: vec![],
            bitrate_bps: None,
        },
        programme_only("PROGA"),
    );
    push(
        &mut store,
        "pcm-protocol",
        pcm(),
        ClaimKey::UsesProtocolFamily,
        KnowledgeValue::ProtocolFamily {
            name: "ISO14229".into(),
        },
        programme_only("PROGA"),
    );
    push(
        &mut store,
        "pcm-capability",
        pcm(),
        ClaimKey::SupportsCapability {
            capability: CAPABILITY.into(),
        },
        KnowledgeValue::Capability {
            name: CAPABILITY.into(),
            supported: true,
            safety_class: Some(DiagnosticSafetyClass::ReadOnly),
        },
        programme_only("PROGA"),
    );
    store
}

fn bind_can_hs(store: &mut KnowledgeStore) {
    push(
        store,
        "binding-can-hs-route",
        bus("CAN_HS"),
        ClaimKey::NetworkRoute,
        KnowledgeValue::NetworkRoute {
            logical_name: "CAN_HS".into(),
            connector: Some("J1962".into()),
            pins: vec![6, 14],
            bitrate_bps: Some(500_000),
        },
        fleet_wide(),
    );
    push(
        store,
        "binding-can-hs-backend",
        bus("CAN_HS"),
        ClaimKey::Custom {
            name: "backend_route".into(),
        },
        KnowledgeValue::BackendRoute {
            backend: "mongoose-jlr".into(),
            route_id: "hs-can".into(),
        },
        fleet_wide(),
    );
}

fn vehicle() -> VehicleContext {
    VehicleContext {
        vehicle_program: Some("PROGA".into()),
        model_year: Some(2010),
        ..VehicleContext::default()
    }
}

#[test]
fn a_family_resolves_without_an_implementation() {
    let mut store = module_store();
    bind_can_hs(&mut store);

    let resolution =
        DiagnosticEnvironmentResolver::resolve_ecu_family(&store, &vehicle(), "PCM", CAPABILITY)
            .unwrap();
    let DiagnosticEnvironmentResolution::Resolved(plan) = resolution else {
        panic!("expected RESOLVED, got {resolution:?}");
    };
    assert_eq!(plan.diagnostic_implementation, None);
    assert_eq!(plan.ecu_family.value, "PCM");
    assert_eq!(
        plan.vehicle_applicability.value.ecu_family.as_deref(),
        Some("PCM")
    );
    assert_eq!(plan.logical_network.value, "CAN_HS");
    assert_eq!(plan.physical_route.value.pins, vec![6, 14]);
    assert_eq!(plan.backend_route.value.route_id, "hs-can");
    assert_eq!(plan.bitrate_bps.value, 500_000);
    // The rate is the route's and comes from the binding alone (ADR-0015).
    assert_eq!(plan.bitrate_bps.evidence.len(), 1);
    assert_eq!(plan.validation_state, ValidationState::SourceBacked);
}

#[test]
fn naming_an_implementation_keeps_it_mandatory() {
    let mut store = module_store();
    bind_can_hs(&mut store);

    let target = DiagnosticTarget::implementation("PCM", "build-x", CAPABILITY).unwrap();
    let query = DiagnosticEnvironmentQuery::new(vehicle(), target).unwrap();
    let DiagnosticEnvironmentResolution::Indeterminate {
        unresolved_facts, ..
    } = DiagnosticEnvironmentResolver::resolve(&store, &query)
    else {
        panic!("a named implementation with no evidence must not resolve");
    };
    assert!(unresolved_facts
        .iter()
        .any(|fact| fact.field == EnvironmentField::DiagnosticImplementation));
}

#[test]
fn an_unbound_bus_leaves_the_route_unresolved_and_named() {
    let store = module_store();

    let DiagnosticEnvironmentResolution::Indeterminate {
        partial,
        unresolved_facts,
    } = DiagnosticEnvironmentResolver::resolve_ecu_family(&store, &vehicle(), "PCM", CAPABILITY)
        .unwrap()
    else {
        panic!("a bus with no adapter route must not resolve");
    };
    let fields: Vec<_> = unresolved_facts.iter().map(|fact| fact.field).collect();
    assert!(fields.contains(&EnvironmentField::BackendRoute));
    assert!(fields.contains(&EnvironmentField::PhysicalRoute));
    // The bus itself is known; only how to reach it is not.
    assert_eq!(
        partial
            .logical_network
            .as_ref()
            .map(|field| field.value.as_str()),
        Some("CAN_HS")
    );
}

#[test]
fn two_stated_buses_are_a_conflict_not_a_choice() {
    let mut store = module_store();
    bind_can_hs(&mut store);
    push(
        &mut store,
        "pcm-network-2",
        pcm(),
        ClaimKey::Custom {
            name: SDD_NETWORK_CLAIM.into(),
        },
        KnowledgeValue::Text {
            value: "CAN_MS".into(),
        },
        programme_only("PROGA"),
    );
    push(
        &mut store,
        "pcm-bus-2",
        pcm(),
        ClaimKey::NetworkRoute,
        KnowledgeValue::NetworkRoute {
            logical_name: "CAN_MS".into(),
            connector: None,
            pins: vec![],
            bitrate_bps: None,
        },
        programme_only("PROGA"),
    );

    let DiagnosticEnvironmentResolution::Conflict { conflicts, .. } =
        DiagnosticEnvironmentResolver::resolve_ecu_family(&store, &vehicle(), "PCM", CAPABILITY)
            .unwrap()
    else {
        panic!("two buses must surface as CONFLICT");
    };
    assert!(conflicts
        .iter()
        .any(|conflict| conflict.field == EnvironmentField::LogicalNetwork));
}
