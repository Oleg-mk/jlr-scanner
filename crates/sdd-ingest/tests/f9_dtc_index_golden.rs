//! F9 golden tests for the SDD DTC index files.
//!
//! These indexes are weaker than the per-code help documents: they carry no
//! model or model-year qualification. The tests below fix that weakness in
//! place rather than papering over it — a generic entry stays unscoped, and a
//! code the index describes twice becomes two records so the disagreement is
//! reported instead of one wording quietly winning.

use knowledge::{
    sha256_bytes, ClaimKey, ContentFingerprint, DimensionConstraint, EntityKind, EvidenceClass,
    KnowledgeQuery, KnowledgeStore, KnowledgeValue, RedistributionStatus, SourceId, SourceRecord,
    SourceType, ValidationState,
};
use sdd_ingest::{DtcDescriptionAdapter, DtcFaultTypeAdapter, FAILURE_TYPE_CLAIM};

const DESCRIPTIONS: &str =
    include_str!("../../../fixtures/knowledge/synthetic/f9_dtc_descriptions.xml");
const FAULT_TYPES: &str =
    include_str!("../../../fixtures/knowledge/synthetic/f9_dtc_fault_types.xml");

fn source(id: &str, body: &str, source_type: SourceType) -> SourceRecord {
    SourceRecord {
        id: SourceId::new(id).unwrap(),
        title: "F9 DTC index fixture".into(),
        source_type,
        origin: "F9 golden test".into(),
        source_locator: "fixtures/knowledge/synthetic".into(),
        content_fingerprint: source_type
            .is_real_evidence()
            .then(|| ContentFingerprint::sha256(sha256_bytes(body.as_bytes())).unwrap()),
        acquired_on: None,
        declared_vehicle_programs: vec![],
        provenance: "Synthetic fixture reproducing the SDD element shape only".into(),
        redistribution_status: RedistributionStatus::Permitted,
        notes: None,
    }
}

fn descriptions(source_type: SourceType) -> KnowledgeStore {
    let adapter = DtcDescriptionAdapter::new(source("f9-desc", DESCRIPTIONS, source_type)).unwrap();
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter, DESCRIPTIONS).unwrap();
    store
}

fn fault_types() -> KnowledgeStore {
    let adapter =
        DtcFaultTypeAdapter::new(source("f9-ftb", FAULT_TYPES, SourceType::Documented)).unwrap();
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter, FAULT_TYPES).unwrap();
    store
}

#[test]
fn a_generic_entry_stays_unscoped_and_a_module_entry_is_scoped() {
    let store = descriptions(SourceType::Documented);

    let generic = store.get_record("f9-desc.dtc.P0100").unwrap();
    assert_eq!(generic.entity.kind, EntityKind::DiagnosticTroubleCode);
    assert_eq!(generic.entity.id, "DTC-P0100");
    // The index never says which module this belongs to, so it must not claim
    // to hold for every module.
    assert_eq!(
        generic.applicability.ecu_family,
        DimensionConstraint::Unknown
    );

    let scoped = store.get_record("f9-desc.dtc.B1250.SYNTHMOD").unwrap();
    assert_eq!(
        scoped.applicability.ecu_family,
        DimensionConstraint::one_of(["SYNTHMOD".to_string()]).unwrap()
    );
}

#[test]
fn a_second_wording_becomes_a_conflict_and_an_exact_repeat_is_dropped() {
    let store = descriptions(SourceType::Documented);

    let first = store.get_record("f9-desc.dtc.B1250.SYNTHMOD").unwrap();
    let second = store.get_record("f9-desc.dtc.B1250.SYNTHMOD.2").unwrap();
    assert_eq!(first.entity, second.entity);
    assert_eq!(first.key, second.key);
    assert_ne!(first.value, second.value);

    // The third entry repeats the first exactly and adds nothing.
    assert!(store.get_record("f9-desc.dtc.B1250.SYNTHMOD.3").is_none());

    let conflicts = store
        .query(&KnowledgeQuery::default().include_indeterminate(true))
        .conflicts;
    let reported = conflicts
        .iter()
        .find(|conflict| conflict.entity.id == "DTC-B1250")
        .expect("a second wording is reported as a conflict");
    assert_eq!(
        reported.record_ids,
        vec!["f9-desc.dtc.B1250.SYNTHMOD", "f9-desc.dtc.B1250.SYNTHMOD.2"]
    );
}

#[test]
fn an_entry_without_text_is_not_recorded_as_an_empty_claim() {
    let store = descriptions(SourceType::Documented);
    assert!(store.get_record("f9-desc.dtc.B1251.OTHERMOD").is_none());

    let all = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    assert_eq!(all.records.len(), 3);
}

#[test]
fn fault_types_are_catalogued_with_negative_numbers_spelled_out() {
    let store = fault_types();

    // A bare minus sign would read as a separator inside an identifier.
    let reserved = store.get_record("f9-ftb.ftb.neg1").unwrap();
    assert_eq!(reserved.entity.id, "FTB-neg1");
    assert_eq!(
        reserved.key,
        ClaimKey::Custom {
            name: FAILURE_TYPE_CLAIM.into()
        }
    );
    assert_eq!(
        reserved.value,
        KnowledgeValue::Text {
            value: "SAE - reserved".into()
        }
    );

    // A CDATA body containing a bare "<" survives intact.
    let amplitude = store.get_record("f9-ftb.ftb.33").unwrap();
    assert_eq!(
        amplitude.value,
        KnowledgeValue::Text {
            value: "General signal failure - signal amplitude<minimum".into()
        }
    );

    // The catalogue is vehicle-independent, so it must not claim a vehicle.
    assert_eq!(
        reserved.applicability.vehicle_program,
        DimensionConstraint::Unknown
    );

    // The empty entry is skipped.
    assert!(store.get_record("f9-ftb.ftb.99").is_none());
    let all = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    assert_eq!(all.records.len(), 3);
}

#[test]
fn wrong_roots_are_rejected_without_partial_write() {
    let mut store = KnowledgeStore::new();
    let descriptions_adapter =
        DtcDescriptionAdapter::new(source("f9-desc", DESCRIPTIONS, SourceType::Documented))
            .unwrap();
    let fault_adapter =
        DtcFaultTypeAdapter::new(source("f9-ftb", FAULT_TYPES, SourceType::Documented)).unwrap();

    // Each adapter must refuse the other's document rather than silently
    // producing nothing.
    assert!(store.ingest(&descriptions_adapter, FAULT_TYPES).is_err());
    assert!(store.ingest(&fault_adapter, DESCRIPTIONS).is_err());
    assert_eq!(store.sources().iter().count(), 0);
}

#[test]
fn classification_follows_the_source_type() {
    let synthetic = descriptions(SourceType::Synthetic);
    assert_eq!(
        synthetic
            .get_record("f9-desc.dtc.P0100")
            .unwrap()
            .validation_state,
        ValidationState::Unverified
    );
    assert_eq!(
        synthetic.trace_back("f9-desc.dtc.P0100").unwrap()[0]
            .evidence
            .evidence_class,
        Some(EvidenceClass::SyntheticTest)
    );

    let documented = descriptions(SourceType::Documented);
    assert_eq!(
        documented
            .get_record("f9-desc.dtc.P0100")
            .unwrap()
            .validation_state,
        ValidationState::SourceBacked
    );
    assert_eq!(
        documented.trace_back("f9-desc.dtc.P0100").unwrap()[0]
            .evidence
            .evidence_class,
        Some(EvidenceClass::OemDocumentation)
    );
}
