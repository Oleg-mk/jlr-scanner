//! F9 golden tests for the description index in another language
//! (`ADR-0034`, amended). The Russian pack carries the same two indexes as
//! the English one with the words changed; read as Russian, the adapter
//! writes each description as its own claim, `sdd_dtc_description.rus`,
//! beside the English alias. The index has no language element, so the
//! words themselves are checked: a Russian index is mostly Cyrillic.

use knowledge::{
    sha256_bytes, ClaimKey, ContentFingerprint, DimensionConstraint, KnowledgeStore,
    KnowledgeValue, RedistributionStatus, SourceId, SourceRecord, SourceType,
};
use sdd_ingest::{DtcDescriptionAdapter, DTC_DESCRIPTION_CLAIM_PREFIX, DTC_HELP_LANGUAGE_RUSSIAN};

const ENGLISH: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_dtc_descriptions.xml");
const RUSSIAN: &str =
    include_str!("../../../fixtures/knowledge/synthetic/f9_dtc_descriptions_rus.xml");

fn source(id: &str, body: &str) -> SourceRecord {
    SourceRecord {
        id: SourceId::new(id).unwrap(),
        title: "F9 DTC index fixture".into(),
        source_type: SourceType::Documented,
        origin: "F9 golden test".into(),
        source_locator: "fixtures/knowledge/synthetic".into(),
        content_fingerprint: Some(
            ContentFingerprint::sha256(sha256_bytes(body.as_bytes())).unwrap(),
        ),
        acquired_on: None,
        declared_vehicle_programs: vec![],
        provenance: "Synthetic fixture reproducing the SDD element shape only".into(),
        redistribution_status: RedistributionStatus::Permitted,
        notes: None,
    }
}

fn russian(id: &str, body: &str) -> DtcDescriptionAdapter {
    DtcDescriptionAdapter::new(source(id, body))
        .unwrap()
        .with_language(DTC_HELP_LANGUAGE_RUSSIAN)
        .unwrap()
}

#[test]
fn a_russian_index_writes_each_description_as_its_own_claim_with_the_same_scope() {
    let mut store = KnowledgeStore::new();
    store
        .ingest(&russian("f9-desc-rus", RUSSIAN), RUSSIAN)
        .unwrap();

    let generic = store.get_record("f9-desc-rus.dtc.P0100.rus").unwrap();
    assert_eq!(
        generic.key,
        ClaimKey::Custom {
            name: format!("{DTC_DESCRIPTION_CLAIM_PREFIX}rus"),
        }
    );
    assert_eq!(
        generic.value,
        KnowledgeValue::Text {
            value: "Синтетическое общее описание".into(),
        }
    );
    assert_eq!(
        generic.applicability.ecu_family,
        DimensionConstraint::Unknown
    );

    let scoped = store
        .get_record("f9-desc-rus.dtc.B1250.SYNTHMOD.rus")
        .unwrap();
    assert_eq!(
        scoped.applicability.ecu_family,
        DimensionConstraint::one_of(["SYNTHMOD".to_string()]).unwrap()
    );
    // The empty entry produces no claim, as in English.
    assert!(store
        .get_record("f9-desc-rus.dtc.B1251.OTHERMOD.rus")
        .is_none());
    assert_eq!(store.record_count(), 2);
}

#[test]
fn the_two_indexes_load_side_by_side_and_the_english_alias_is_untouched() {
    let mut store = KnowledgeStore::new();
    let english = DtcDescriptionAdapter::new(source("f9-desc", ENGLISH)).unwrap();
    store.ingest(&english, ENGLISH).unwrap();
    store
        .ingest(&russian("f9-desc-rus", RUSSIAN), RUSSIAN)
        .unwrap();
    assert_eq!(
        store.get_record("f9-desc.dtc.P0100").unwrap().key,
        ClaimKey::Alias
    );
    assert!(store.get_record("f9-desc-rus.dtc.P0100.rus").is_some());
    assert_eq!(store.sources().iter().count(), 2);
}

#[test]
fn an_index_in_the_wrong_language_is_refused_by_its_words() {
    // The English index read as Russian: nothing in it is Cyrillic.
    let mut store = KnowledgeStore::new();
    let error = store
        .ingest(&russian("f9-desc-rus", ENGLISH), ENGLISH)
        .expect_err("English is not Russian");
    assert!(
        error.to_string().contains("reads as not Russian"),
        "{error}"
    );
    assert_eq!(store.sources().iter().count(), 0, "no partial write");

    // The Russian index read as English.
    let english = DtcDescriptionAdapter::new(source("f9-desc", RUSSIAN)).unwrap();
    let error = store
        .ingest(&english, RUSSIAN)
        .expect_err("Russian is not English");
    assert!(error.to_string().contains("reads as Russian"), "{error}");
}

#[test]
fn a_language_sdd_ships_no_index_for_is_refused_up_front() {
    let error = DtcDescriptionAdapter::new(source("f9-desc-deu", RUSSIAN))
        .unwrap()
        .with_language("deu")
        .expect_err("no German index is known");
    assert!(error.to_string().contains("deu"), "{error}");
}
