//! F9 golden tests for JLR's IVS part lineage (`ADR-0033`).
//!
//! The load-bearing test here is the safety one. The IVS component is where
//! JLR keeps programming orchestration — service actions and coordinated
//! flash lists — and this product takes part *numbers* from it and nothing
//! else. A document carrying orchestration, or one SDD does not stamp as
//! production data, is refused rather than filtered.

use knowledge::{
    sha256_bytes, ClaimKey, ContentFingerprint, DimensionConstraint, EntityKind, EvidenceClass,
    KnowledgeQuery, KnowledgeStore, KnowledgeValue, RedistributionStatus, SourceId, SourceRecord,
    SourceType, ValidationState,
};
use sdd_ingest::{IvsLineageAdapter, IVS_ASSEMBLY_CLAIM};

const FIXTURE: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_ivs_lineage.xml");

fn source(source_type: SourceType) -> SourceRecord {
    SourceRecord {
        id: SourceId::new("f9-ivs").unwrap(),
        title: "F9 IVS fixture".into(),
        source_type,
        origin: "F9 golden test".into(),
        source_locator: "fixtures/knowledge/synthetic/f9_ivs_lineage.xml".into(),
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
    let adapter = IvsLineageAdapter::new(source(source_type)).unwrap();
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter, FIXTURE).unwrap();
    store
}

fn reject(input: &str) -> String {
    let adapter = IvsLineageAdapter::new(source(SourceType::Documented)).unwrap();
    let mut store = KnowledgeStore::new();
    let error = store
        .ingest(&adapter, input)
        .expect_err("input must be rejected");
    assert_eq!(store.sources().iter().count(), 0, "no partial write");
    error.to_string()
}

fn text_of(store: &KnowledgeStore, id: &str) -> String {
    let record = store.get_record(id).expect("the record exists");
    let KnowledgeValue::Text { value } = &record.value else {
        panic!("expected a text");
    };
    value.clone()
}

#[test]
fn nothing_that_could_programme_a_module_is_ingested() {
    let store = ingest(SourceType::Documented);
    let all = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    assert!(!all.records.is_empty());
    for entry in &all.records {
        // Every record is one assembly's numbers under one claim. There is no
        // capability here, so nothing can be planned, offered or executed.
        assert_eq!(entry.record.entity.kind, EntityKind::ModuleAssembly);
        assert_eq!(
            entry.record.key,
            ClaimKey::Custom {
                name: IVS_ASSEMBLY_CLAIM.into()
            }
        );
        let KnowledgeValue::Text { value } = &entry.record.value else {
            panic!("a lineage record is a text");
        };
        for forbidden in [
            "ServiceAction",
            "CoordinatedFlash",
            "CFL",
            "SOTA",
            "ProgInSvc",
            "Certification",
        ] {
            assert!(
                !value.contains(forbidden),
                "{forbidden} must not reach the library: {value}"
            );
        }
    }
}

#[test]
fn a_document_sdd_does_not_stamp_as_production_is_refused() {
    let test_environment = FIXTURE.replace(
        "<Environment>Production</Environment>",
        "<Environment>Test</Environment>",
    );
    assert!(reject(&test_environment).contains("production"));

    let unvalidated = FIXTURE.replace(
        "<ValidationStatus>Yes</ValidationStatus>",
        "<ValidationStatus>No</ValidationStatus>",
    );
    assert!(reject(&unvalidated).contains("production"));

    // A production stamp does not save a document that carries orchestration.
    let with_actions = FIXTURE.replace(
        "<VehicleServiceActionDetail>",
        "<ServiceActions><SvcActionID>SRV0000068</SvcActionID></ServiceActions>\n      <VehicleServiceActionDetail>",
    );
    assert!(reject(&with_actions).contains("service actions"));

    let with_flash = FIXTURE.replace(
        "<VehicleServiceActionDetail>",
        "<CoordinatedFlashList><CFLId>CFL0000071</CFLId></CoordinatedFlashList>\n      <VehicleServiceActionDetail>",
    );
    assert!(reject(&with_flash).contains("service actions"));

    assert!(IvsLineageAdapter::is_production(FIXTURE));
}

#[test]
fn an_assembly_carries_the_parts_the_catalogue_names_inside_it() {
    let store = ingest(SourceType::Documented);

    // The same assembly declared once per node is one record, with the parts
    // of both nodes and the hardware part beside them.
    let value = text_of(&store, "f9-ivs.ivs.SYNTHA.SYNTHMOD.8X2318C808CE");
    assert_eq!(
        value,
        "assembly=8X23-18C808-CE;as_delivered=F113;baseline=2007;dated=Oct-17-2022 22:16:38;\
         pids=F191|F188|F124;parts=6H52-14C524-CB|6H52-14C526-CD|8X23-14C527-CE;\
         types=Hardware|Strategy|Calibration"
    );

    // A second assembly of the same module is its own record.
    assert!(
        text_of(&store, "f9-ivs.ivs.SYNTHA.SYNTHMOD.8X2318C808DA").contains("parts=9X23-14C526-AB")
    );

    // A module with no parts under its assembly says nothing.
    assert!(store
        .get_record("f9-ivs.ivs.SYNTHA.EMPTYMOD.8X2319C999AA")
        .is_none());

    // A supporting part the catalogue ties to no identifier is not recorded:
    // there is nothing a module could answer to be compared with it.
    let all = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    for entry in &all.records {
        let KnowledgeValue::Text { value } = &entry.record.value else {
            continue;
        };
        assert!(!value.contains("6H52-14C528-CB"), "{value}");
        // Nor one whose identifier is not an identifier at all.
        assert!(!value.contains("SECX"), "{value}");
        assert!(!value.contains("8X23-14C529-AA"), "{value}");
    }
}

#[test]
fn a_lineage_belongs_to_one_programme_and_one_module() {
    let store = ingest(SourceType::Documented);
    let record = store
        .get_record("f9-ivs.ivs.SYNTHA.OTHERMOD.8X2319C200AA")
        .expect("the other module is recorded");
    assert_eq!(
        record.applicability.vehicle_program,
        DimensionConstraint::one_of(["SYNTHA".to_string()]).unwrap()
    );
    assert_eq!(
        record.applicability.ecu_family,
        DimensionConstraint::one_of(["OTHERMOD".to_string()]).unwrap()
    );
    // The baseline year is written into the text, never matched: SDD's year
    // markers sit on a dimension a session does not state (ADR-0032).
    assert!(record.applicability.other.is_empty());
    assert_eq!(record.entity.id, "IVS-SYNTHA-OTHERMOD-8X2319C200AA");
}

#[test]
fn classification_follows_the_source_type_and_ingestion_is_idempotent() {
    let synthetic = ingest(SourceType::Synthetic);
    let id = "f9-ivs.ivs.SYNTHA.SYNTHMOD.8X2318C808CE";
    assert_eq!(
        synthetic.get_record(id).unwrap().validation_state,
        ValidationState::Unverified
    );
    assert_eq!(
        synthetic.trace_back(id).unwrap()[0].evidence.evidence_class,
        Some(EvidenceClass::SyntheticTest)
    );

    let documented = ingest(SourceType::Documented);
    assert_eq!(
        documented.get_record(id).unwrap().validation_state,
        ValidationState::SourceBacked
    );

    let adapter = IvsLineageAdapter::new(source(SourceType::Documented)).unwrap();
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter, FIXTURE).unwrap();
    let first = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    let before = first.records.len();
    store.ingest(&adapter, FIXTURE).unwrap();
    let after = store
        .query(&KnowledgeQuery::default().include_indeterminate(true))
        .records
        .len();
    assert_eq!(before, after, "ingesting twice changes nothing");
}

#[test]
fn a_document_without_a_programme_or_lineage_is_refused_rather_than_half_read() {
    let no_program = FIXTURE.replace("<ProgramCode>SYNTHA</ProgramCode>", "");
    assert!(reject(&no_program).contains("ProgramCode"));

    let wrong_root = FIXTURE.replace("ShowVehicleServiceActions", "SomethingElse");
    assert!(reject(&wrong_root).contains("ShowVehicleServiceActions"));
}
