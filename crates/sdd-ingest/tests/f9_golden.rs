//! F9 golden tests for SDD knowledge ingestion.
//!
//! The fixture is synthetic and proves parser behaviour only. The classification
//! assertions below are the point of the test: the same parser must produce
//! `SyntheticTest` / `Unverified` for a synthetic source and
//! `OemDocumentation` / `SourceBacked` for a documented one, so a fixture can
//! never be laundered into JLR evidence.

use knowledge::{
    sha256_bytes, ApplicabilityResolution, ContentFingerprint, EntityKind, EvidenceClass,
    IngestionAdapter, KnowledgeQuery, KnowledgeStore, KnowledgeValue, RedistributionStatus,
    SourceId, SourceRecord, SourceType, ValidationState, VehicleContext, YearConstraint,
};
use sdd_ingest::{CanLinkMonitorAdapter, ModelYearTimeline};
use std::collections::BTreeMap;

const FIXTURE: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_can_link_monitor.xml");

fn source(id: &str, source_type: SourceType) -> SourceRecord {
    SourceRecord {
        id: SourceId::new(id).unwrap(),
        title: "F9 CAN link monitor fixture".into(),
        source_type,
        origin: "F9 golden test".into(),
        source_locator: "fixtures/knowledge/synthetic/f9_can_link_monitor.xml".into(),
        content_fingerprint: source_type
            .is_real_evidence()
            .then(|| ContentFingerprint::sha256(sha256_bytes(FIXTURE.as_bytes())).unwrap()),
        acquired_on: None,
        declared_vehicle_programs: vec![],
        provenance: "Synthetic fixture reproducing the SDD element shape only".into(),
        redistribution_status: RedistributionStatus::Permitted,
        notes: None,
    }
}

fn ingest(source_type: SourceType) -> KnowledgeStore {
    let adapter = CanLinkMonitorAdapter::new(source("f9-fixture", source_type)).unwrap();
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter, FIXTURE).unwrap();
    store
}

fn context(program: &str, model_year: u16) -> VehicleContext {
    VehicleContext {
        vehicle_program: Some(program.into()),
        model_year: Some(model_year),
        architecture_generation: None,
        ecu_family: None,
        powertrain: None,
        variant: None,
        market: None,
        diagnostic_implementation: None,
        other: BTreeMap::new(),
    }
}

#[test]
fn addressing_width_and_model_year_are_parsed_and_blank_entries_are_skipped() {
    let store = ingest(SourceType::Synthetic);

    let record = store.get_record("f9-fixture.vehicle.SYNTHA.all").unwrap();
    assert_eq!(record.entity.kind, EntityKind::VehicleProgram);
    let KnowledgeValue::DiagnosticAddressing {
        can_id_format,
        addressing_mode,
        request_id,
        ..
    } = &record.value
    else {
        panic!("expected a diagnostic addressing value");
    };
    assert_eq!(*can_id_format, Some(knowledge::CanIdFormat::Standard11Bit));
    assert_eq!(addressing_mode.as_deref(), Some("11bit"));
    // The file states a width, never a request identifier. Nothing may invent one.
    assert_eq!(*request_id, None);

    let extended = store.get_record("f9-fixture.vehicle.SYNTHB.2014").unwrap();
    let KnowledgeValue::DiagnosticAddressing { can_id_format, .. } = &extended.value else {
        panic!("expected a diagnostic addressing value");
    };
    assert_eq!(*can_id_format, Some(knowledge::CanIdFormat::Extended29Bit));

    // A blank AddressingType is skipped rather than guessed, and a module
    // without a description is not turned into a claim.
    assert!(store.get_record("f9-fixture.vehicle.SYNTHB.2015").is_none());
    assert!(store.get_record("f9-fixture.module.7E1").is_none());
}

#[test]
fn prose_model_year_markers_are_kept_but_never_resolve_on_a_guess() {
    let store = ingest(SourceType::Synthetic);

    // "Post MY10" is retained as a claim, because discarding it would lose real
    // information, but its boundary is not stated by the source.
    let record = store
        .get_record("f9-fixture.vehicle.SYNTHB.post-my10")
        .expect("prose model-year markers are retained");
    assert_eq!(record.applicability.model_year, YearConstraint::Unknown);

    // Because the boundary was not invented, it must not resolve for a year.
    let resolved: Vec<_> = store
        .query(&KnowledgeQuery::for_vehicle(context("SYNTHB", 2014)))
        .records
        .into_iter()
        .map(|entry| entry.record.id)
        .collect();
    assert_eq!(resolved, vec!["f9-fixture.vehicle.SYNTHB.2014"]);
}

#[test]
fn global_module_table_does_not_resolve_for_a_specific_vehicle() {
    let store = ingest(SourceType::Synthetic);

    let module = store.get_record("f9-fixture.module.7E0").unwrap();
    assert_eq!(module.entity.kind, EntityKind::DiagnosticAddressing);
    assert_eq!(module.entity.id, "7E0");

    // The vehicle-scoped claim resolves; the global module alias must not,
    // because the source never states which program it belongs to.
    let result = store.query(&KnowledgeQuery::for_vehicle(context("SYNTHA", 2010)));
    let resolved: Vec<_> = result
        .records
        .iter()
        .map(|entry| entry.record.id.as_str())
        .collect();
    assert_eq!(resolved, vec!["f9-fixture.vehicle.SYNTHA.all"]);

    let permissive = store
        .query(&KnowledgeQuery::for_vehicle(context("SYNTHA", 2010)).include_indeterminate(true));
    let module_entry = permissive
        .records
        .iter()
        .find(|entry| entry.record.id == "f9-fixture.module.7E0")
        .expect("module alias is retrievable when indeterminate results are requested");
    assert_eq!(
        module_entry.applicability_resolution,
        ApplicabilityResolution::InsufficientEvidence
    );
}

#[test]
fn a_reused_mnemonic_surfaces_as_a_conflict_instead_of_overwriting() {
    let store = ingest(SourceType::Synthetic);

    // The exact repeat adds nothing and must not create a second record.
    // The differing description must, so the ambiguity stays visible.
    let first = store.get_record("f9-fixture.module.10").unwrap();
    let second = store.get_record("f9-fixture.module.10.2").unwrap();
    assert_eq!(first.entity, second.entity);
    assert_eq!(first.key, second.key);
    assert_ne!(first.value, second.value);
    assert!(store.get_record("f9-fixture.module.10.3").is_none());

    let conflicts = store
        .query(&KnowledgeQuery::for_vehicle(context("SYNTHA", 2010)).include_indeterminate(true))
        .conflicts;
    let reused = conflicts
        .iter()
        .find(|conflict| conflict.entity.id == "10")
        .expect("a reused mnemonic is reported as a conflict");
    assert_eq!(
        reused.record_ids,
        vec!["f9-fixture.module.10", "f9-fixture.module.10.2"]
    );
    assert_eq!(reused.values.len(), 2);
}

#[test]
fn classification_follows_the_source_type_and_cannot_be_laundered() {
    let synthetic = ingest(SourceType::Synthetic);
    let trace = synthetic
        .trace_back("f9-fixture.vehicle.SYNTHA.all")
        .unwrap();
    assert_eq!(
        trace[0].evidence.evidence_class,
        Some(EvidenceClass::SyntheticTest)
    );
    assert_eq!(
        synthetic
            .get_record("f9-fixture.vehicle.SYNTHA.all")
            .unwrap()
            .validation_state,
        ValidationState::Unverified
    );

    let documented = ingest(SourceType::Documented);
    let trace = documented
        .trace_back("f9-fixture.vehicle.SYNTHA.all")
        .unwrap();
    assert_eq!(
        trace[0].evidence.evidence_class,
        Some(EvidenceClass::OemDocumentation)
    );
    assert_eq!(
        documented
            .get_record("f9-fixture.vehicle.SYNTHA.all")
            .unwrap()
            .validation_state,
        ValidationState::SourceBacked
    );
    // A single source never corroborates itself.
    assert_ne!(
        documented
            .get_record("f9-fixture.vehicle.SYNTHA.all")
            .unwrap()
            .validation_state,
        ValidationState::Corroborated
    );
}

#[test]
fn ingestion_is_deterministic_and_idempotent() {
    let adapter = CanLinkMonitorAdapter::new(source("f9-fixture", SourceType::Documented)).unwrap();
    let mut store = KnowledgeStore::new();
    let first = store.ingest(&adapter, FIXTURE).unwrap();
    let second = store.ingest(&adapter, FIXTURE).unwrap();
    assert_eq!(first, second);

    let mut fresh = KnowledgeStore::new();
    assert_eq!(fresh.ingest(&adapter, FIXTURE).unwrap(), first);
    assert_eq!(first.parser_id, adapter.parser_id());
}

#[test]
fn malformed_input_is_rejected_without_partial_write() {
    let adapter = CanLinkMonitorAdapter::new(source("f9-fixture", SourceType::Documented)).unwrap();
    let mut store = KnowledgeStore::new();

    assert!(store.ingest(&adapter, "<configuration>").is_err());
    assert!(store.ingest(&adapter, "<other><Modules/></other>").is_err());
    assert!(store
        .ingest(&adapter, "<configuration><Modules/></configuration>")
        .is_err());
    assert!(store
        .ingest(
            &adapter,
            &FIXTURE.replace("<AddressingType>11bit", "<AddressingType>7bit")
        )
        .is_err());
    assert_eq!(store.sources().iter().count(), 0);
}

#[test]
fn a_prose_marker_resolves_only_when_the_derivation_is_opted_into() {
    // Without a timeline the marker is kept verbatim and no year is claimed.
    let store = ingest(SourceType::Documented);
    let plain = store
        .get_record("f9-fixture.vehicle.SYNTHB.post-my10")
        .expect("prose markers are retained");
    assert_eq!(plain.applicability.model_year, YearConstraint::Unknown);

    // Opting in resolves it. A prose marker carries its own boundary, so an
    // empty timeline suffices; the marker stays on its own dimension either way.
    let adapter = CanLinkMonitorAdapter::new(source("f9-fixture", SourceType::Documented))
        .unwrap()
        .with_timeline(ModelYearTimeline::new());
    let mut resolved = KnowledgeStore::new();
    resolved.ingest(&adapter, FIXTURE).unwrap();
    let derived = resolved
        .get_record("f9-fixture.vehicle.SYNTHB.post-my10")
        .unwrap();
    assert_eq!(
        derived.applicability.model_year,
        YearConstraint::Range {
            model_year_from: Some(2010),
            model_year_to: None
        }
    );
}
