//! F9 golden tests for what a module declares it will accept (`ADR-0035`).
//!
//! The per-module index names three accesses beside an identifier. This
//! adapter takes two of them — writeable and controllable — and the
//! routines, and leaves the readable half to the catalogues that already
//! hold it. Nothing here sends anything: the records are knowledge, and the
//! service, the session and the security level are recorded as SDD writes
//! them so that a reader can see what an operation would cost before any
//! decision is taken to allow one.

use knowledge::{
    sha256_bytes, ClaimKey, ContentFingerprint, DimensionConstraint, KnowledgeStore,
    KnowledgeValue, RedistributionStatus, SourceId, SourceRecord, SourceType,
};
use sdd_ingest::{
    ModuleAccessAdapter, MODULE_CONTROL_NAMESPACE, MODULE_ROUTINE_NAMESPACE, MODULE_WRITE_NAMESPACE,
};

const MODULE: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_mdx_module.xml");

fn source(id: &str) -> SourceRecord {
    SourceRecord {
        id: SourceId::new(id).unwrap(),
        title: "F9 module index fixture".into(),
        source_type: SourceType::Documented,
        origin: "F9 golden test".into(),
        source_locator: "fixtures/knowledge/synthetic".into(),
        content_fingerprint: Some(
            ContentFingerprint::sha256(sha256_bytes(MODULE.as_bytes())).unwrap(),
        ),
        acquired_on: None,
        declared_vehicle_programs: vec![],
        provenance: "Synthetic fixture reproducing the SDD element shape only".into(),
        redistribution_status: RedistributionStatus::Permitted,
        notes: None,
    }
}

fn adapter(id: &str) -> ModuleAccessAdapter {
    ModuleAccessAdapter::new(source(id), "SYNTHA", "MY10").unwrap()
}

fn encoding(store: &KnowledgeStore, id: &str) -> String {
    match &store.get_record(id).expect("record").value {
        KnowledgeValue::IdentifierDefinition { encoding, .. } => {
            encoding.clone().expect("an encoding")
        }
        other => panic!("expected an identifier definition, found {other:?}"),
    }
}

#[test]
fn a_writeable_identifier_carries_its_service_and_session_and_nothing_sends_it() {
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter("f9-mdx"), MODULE).unwrap();

    let record = store.get_record("f9-mdx.synthmod.wr.0200").unwrap();
    assert_eq!(
        record.key,
        ClaimKey::IdentifierDefinition {
            namespace: MODULE_WRITE_NAMESPACE.into(),
        }
    );
    let encoding = encoding(&store, "f9-mdx.synthmod.wr.0200");
    assert!(
        encoding.contains("name=Synthetic written value"),
        "{encoding}"
    );
    assert!(encoding.contains("service=0x2E"), "{encoding}");
    assert!(encoding.contains("session=03"), "{encoding}");
    assert!(!encoding.contains("security="), "{encoding}");

    // The module and the programme-year it was read for travel with it.
    assert_eq!(
        record.applicability.ecu_family,
        DimensionConstraint::one_of(["SYNTHMOD".to_string()]).unwrap()
    );
    assert_eq!(
        record.applicability.vehicle_program,
        DimensionConstraint::one_of(["SYNTHA".to_string()]).unwrap()
    );
}

#[test]
fn a_controllable_identifier_keeps_the_security_level_standing_in_front_of_it() {
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter("f9-mdx"), MODULE).unwrap();

    let record = store.get_record("f9-mdx.synthmod.ct.0300").unwrap();
    assert_eq!(
        record.key,
        ClaimKey::IdentifierDefinition {
            namespace: MODULE_CONTROL_NAMESPACE.into(),
        }
    );
    let encoding = encoding(&store, "f9-mdx.synthmod.ct.0300");
    assert!(encoding.contains("service=0x2F"), "{encoding}");
    assert!(encoding.contains("security=level_1"), "{encoding}");
    assert!(encoding.contains("iocp=iocp_00 iocp_03"), "{encoding}");
}

#[test]
fn the_routines_a_module_declares_are_listed_with_sdds_own_words() {
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter("f9-mdx"), MODULE).unwrap();

    let record = store.get_record("f9-mdx.synthmod.ro.0400").unwrap();
    assert_eq!(
        record.key,
        ClaimKey::IdentifierDefinition {
            namespace: MODULE_ROUTINE_NAMESPACE.into(),
        }
    );
    let encoding = encoding(&store, "f9-mdx.synthmod.ro.0400");
    assert!(
        encoding.contains("name=Synthetic clear adaptions"),
        "{encoding}"
    );
    assert!(encoding.contains("service=0x31"), "{encoding}");
    assert!(encoding.contains("max_run_time=30"), "{encoding}");
    assert!(encoding.contains("restart_while_running=no"), "{encoding}");

    // The self-test routine is one of these too, and it is recorded like any
    // other: listing it is what ADR-0032 already does, and running it is
    // still stage 2.
    assert!(store.get_record("f9-mdx.synthmod.ro.0202").is_some());
}

#[test]
fn the_readable_half_is_left_to_the_catalogues_that_already_hold_it() {
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter("f9-mdx"), MODULE).unwrap();

    // 0x0100 is readable and nothing else: this adapter writes no record for
    // it, and none of the three namespaces claims a readable access.
    assert!(store.get_record("f9-mdx.synthmod.wr.0100").is_none());
    assert!(store.get_record("f9-mdx.synthmod.ct.0100").is_none());
    // An entry with no number cannot be addressed, so it is passed over
    // rather than recorded with a hole in it.
    assert_eq!(store.record_count(), 6);
}

/// Seven documents in SDD 169 carry a second `<DATA_IDENTIFIERS>` or a
/// second `<ROUTINE_IDENTIFIERS>`. Reading only the first of each dropped
/// what stood in the others and said nothing about it.
#[test]
fn a_second_section_of_either_kind_is_read_as_well_as_the_first() {
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter("f9-mdx"), MODULE).unwrap();

    let write = encoding(&store, "f9-mdx.synthmod.wr.0500");
    assert!(
        write.contains("name=Synthetic value in a second section"),
        "{write}"
    );
    let routine = encoding(&store, "f9-mdx.synthmod.ro.0500");
    assert!(
        routine.contains("name=Synthetic routine in a second section"),
        "{routine}"
    );
}

#[test]
fn a_document_that_is_not_a_module_index_is_refused() {
    let mut store = KnowledgeStore::new();
    let error = store
        .ingest(&adapter("f9-mdx"), "<configuration_data/>")
        .expect_err("only an <MDX> document is a module index");
    assert!(error.to_string().contains("MDX"), "{error}");
    assert_eq!(store.sources().iter().count(), 0, "no partial write");
}

#[test]
fn the_programme_and_the_marker_are_required_because_the_document_names_neither() {
    assert!(ModuleAccessAdapter::new(source("f9-mdx"), "", "MY10").is_err());
    assert!(ModuleAccessAdapter::new(source("f9-mdx"), "SYNTHA", "  ").is_err());
}
