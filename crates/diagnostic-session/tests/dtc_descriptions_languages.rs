//! A fault code's description in SDD's own languages (`ADR-0034`, amended):
//! the English alias as before, and the Russian beside it with the same
//! scope — the module's entry when that is what the English is, the generic
//! one otherwise, and nothing when the Russian pack has no entry of that
//! scope.

use knowledge::{
    sha256_bytes, ContentFingerprint, IngestionAdapter, RedistributionStatus, SourceId,
    SourceRecord, SourceType,
};
use sdd_ingest::{DtcDescriptionAdapter, DTC_HELP_LANGUAGE_RUSSIAN};

const ENGLISH: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_dtc_descriptions.xml");
const RUSSIAN: &str =
    include_str!("../../../fixtures/knowledge/synthetic/f9_dtc_descriptions_rus.xml");

fn source(id: &str, text: &str) -> SourceRecord {
    SourceRecord {
        id: SourceId::new(id).unwrap(),
        title: format!("DTC index fixture {id}"),
        source_type: SourceType::Synthetic,
        origin: "diagnostic-session test".into(),
        source_locator: "fixtures/knowledge/synthetic".into(),
        content_fingerprint: Some(
            ContentFingerprint::sha256(sha256_bytes(text.as_bytes())).unwrap(),
        ),
        acquired_on: None,
        declared_vehicle_programs: vec![],
        provenance: "Synthetic fixture reproducing the SDD element shape only".into(),
        redistribution_status: RedistributionStatus::Permitted,
        notes: None,
    }
}

fn library(with_russian: bool) -> diagnostic_session::KnowledgeLibrary {
    let english = DtcDescriptionAdapter::new(source("desc-test", ENGLISH))
        .unwrap()
        .parse(ENGLISH)
        .unwrap();
    let english = serde_json::to_string(&english).unwrap();
    let mut manifests = vec![("dtc_index.json", english)];
    if with_russian {
        let russian = DtcDescriptionAdapter::new(source("desc-test-rus", RUSSIAN))
            .unwrap()
            .with_language(DTC_HELP_LANGUAGE_RUSSIAN)
            .unwrap()
            .parse(RUSSIAN)
            .unwrap();
        manifests.push(("dtc_index.json", serde_json::to_string(&russian).unwrap()));
    }
    diagnostic_session::KnowledgeLibrary::from_manifests(
        manifests.iter().map(|(name, text)| (*name, text.as_str())),
    )
}

#[test]
fn the_russian_description_stands_beside_the_english_with_the_same_scope() {
    let library = library(true);

    // The generic entry, read from any module.
    let generic = library.describe_dtc("P0100", 0, "ANYMOD");
    assert_eq!(
        generic.description.as_deref(),
        Some("Synthetic generic description")
    );
    assert_eq!(generic.description_scope.as_deref(), Some("generic"));
    assert_eq!(
        generic
            .description_data_texts
            .get("rus")
            .map(String::as_str),
        Some("Синтетическое общее описание"),
        "{:?}",
        generic.description_data_texts
    );
    // Russian is the only other language the library has.
    assert_eq!(generic.description_data_texts.len(), 1);

    // The module-scoped entry, read from that module: the first English
    // wording and its Russian twin.
    let scoped = library.describe_dtc("B1250", 0, "SYNTHMOD");
    assert_eq!(
        scoped.description.as_deref(),
        Some("Synthetic sensor circuit failure")
    );
    assert_eq!(scoped.description_scope.as_deref(), Some("module"));
    assert_eq!(
        scoped.description_data_texts.get("rus").map(String::as_str),
        Some("Синтетическая неисправность цепи датчика")
    );

    // The same code read from another module: the English has no entry of
    // any scope, so there is nothing to translate and nothing is shown.
    let elsewhere = library.describe_dtc("B1250", 0, "OTHERMOD");
    assert!(elsewhere.description.is_none());
    assert!(elsewhere.description_data_texts.is_empty());
}

#[test]
fn a_library_without_the_russian_pack_reads_as_it_always_did() {
    let library = library(false);
    let generic = library.describe_dtc("P0100", 0, "ANYMOD");
    assert_eq!(
        generic.description.as_deref(),
        Some("Synthetic generic description")
    );
    assert!(generic.description_data_texts.is_empty());
}
