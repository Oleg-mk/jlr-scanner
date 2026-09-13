//! The self tests a module declares, described in SDD's own words — English
//! from the English pack and, since `ADR-0034`, Russian from the Russian
//! one, joined line to line by the mnemonic's name.

use knowledge::{
    sha256_bytes, ContentFingerprint, IngestionAdapter, RedistributionStatus, SourceId,
    SourceRecord, SourceType, VehicleContext,
};
use sdd_ingest::{OdstInfoAdapter, DTC_HELP_LANGUAGE_RUSSIAN};

const ENGLISH: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_odst_info.xml");
const RUSSIAN: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_odst_info_rus.xml");

fn source(id: &str, locator: &str, text: &str) -> SourceRecord {
    SourceRecord {
        id: SourceId::new(id).unwrap(),
        title: format!("ODST fixture {id}"),
        source_type: SourceType::Synthetic,
        origin: "diagnostic-session test".into(),
        source_locator: locator.into(),
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

fn english_batch() -> String {
    let batch = OdstInfoAdapter::new(source(
        "odst-test",
        "fixtures/knowledge/synthetic/f9_odst_info.xml",
        ENGLISH,
    ))
    .unwrap()
    .parse(ENGLISH)
    .unwrap();
    serde_json::to_string(&batch).unwrap()
}

fn russian_batch() -> String {
    let batch = OdstInfoAdapter::new(source(
        "odst-test-rus",
        "fixtures/knowledge/synthetic/f9_odst_info_rus.xml",
        RUSSIAN,
    ))
    .unwrap()
    .with_language(DTC_HELP_LANGUAGE_RUSSIAN)
    .unwrap()
    .parse(RUSSIAN)
    .unwrap();
    serde_json::to_string(&batch).unwrap()
}

fn car() -> VehicleContext {
    VehicleContext {
        vehicle_program: Some("SYNTHA".into()),
        ..VehicleContext::default()
    }
}

#[test]
fn the_russian_pack_describes_the_test_line_for_line_by_name() {
    let english = english_batch();
    let russian = russian_batch();
    let library = diagnostic_session::KnowledgeLibrary::from_manifests([
        ("odst.json", english.as_str()),
        ("odst.json", russian.as_str()),
    ]);
    let tests = library.self_tests(&car(), "SYNTHMOD");
    let named = tests
        .iter()
        .find(|test| test.test_id == "14")
        .expect("the named test is listed");

    // SDD's English, as before.
    assert_eq!(
        named.description,
        [
            "The synthetic module runs its own check and logs what it finds.",
            "Make sure the ignition is switched on.",
        ]
    );
    // The same screen in Russian, the same length: the second line has no
    // Russian in the pack and keeps its English.
    assert_eq!(
        named.description_texts.get("rus").map(Vec::as_slice),
        Some(
            [
                "Синтетический блок выполняет собственную проверку и записывает результат."
                    .to_string(),
                "Make sure the ignition is switched on.".to_string(),
            ]
            .as_slice()
        ),
        "{:?}",
        named.description_texts
    );
    // Russian is the only other language the library has.
    assert_eq!(named.description_texts.len(), 1);

    // A test with no screen has no words in any language.
    let unnamed = tests
        .iter()
        .find(|test| test.test_id == "99")
        .expect("the unnamed test is listed");
    assert!(unnamed.description.is_empty());
    assert!(unnamed.description_texts.is_empty());
}

#[test]
fn a_library_without_the_russian_pack_reads_as_it_always_did() {
    let english = english_batch();
    let library =
        diagnostic_session::KnowledgeLibrary::from_manifests([("odst.json", english.as_str())]);
    let tests = library.self_tests(&car(), "SYNTHMOD");
    let named = tests.iter().find(|test| test.test_id == "14").unwrap();
    assert_eq!(named.description.len(), 2);
    assert!(
        named.description_texts.is_empty(),
        "{:?}",
        named.description_texts
    );
}
