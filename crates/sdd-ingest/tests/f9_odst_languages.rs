//! F9 golden tests for a self-test pack in another language (`ADR-0034`).
//!
//! SDD ships the on-demand self tests once per language, the same 93 files
//! with the same tests, screens and mnemonic names, and only the human text
//! changed. The adapter read as Russian contributes the text of each screen
//! and nothing else — under the English claim's name with the language last
//! — and refuses a document that does not say it is in that language.

use knowledge::{
    sha256_bytes, ClaimKey, ContentFingerprint, IngestionAdapter, KnowledgeQuery, KnowledgeStore,
    KnowledgeValue, RedistributionStatus, SourceId, SourceRecord, SourceType,
};
use sdd_ingest::{
    OdstInfoAdapter, DTC_HELP_LANGUAGE_RUSSIAN, ODST_SCREEN_CLAIM_PREFIX,
    ODST_SCREEN_ITEMS_CLAIM_PREFIX, ODST_TEST_CLAIM,
};

const ENGLISH: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_odst_info.xml");
const RUSSIAN: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_odst_info_rus.xml");

fn source(id: &str, text: &str) -> SourceRecord {
    SourceRecord {
        id: SourceId::new(id).unwrap(),
        title: format!("F9 ODST fixture {id}"),
        source_type: SourceType::Documented,
        origin: "F9 golden test".into(),
        source_locator: "fixtures/knowledge/synthetic/f9_odst_info_rus.xml".into(),
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

fn russian_adapter(id: &str, text: &str) -> OdstInfoAdapter {
    OdstInfoAdapter::new(source(id, text))
        .unwrap()
        .with_language(DTC_HELP_LANGUAGE_RUSSIAN)
        .unwrap()
}

#[test]
fn a_russian_pack_writes_the_text_of_each_used_screen_and_nothing_else() {
    let mut store = KnowledgeStore::new();
    store
        .ingest(&russian_adapter("f9-odst-rus", RUSSIAN), RUSSIAN)
        .unwrap();

    let all = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    let mut ids: Vec<String> = all
        .records
        .iter()
        .map(|entry| entry.record.id.clone())
        .collect();
    ids.sort();
    // One screen is pointed at by a test; it is written as its text and as
    // its items, and both ids end in the language. The unused screen is not
    // recorded, as in English.
    assert_eq!(
        ids,
        vec![
            "f9-odst-rus.odst.SYNTHMOD.screen.DR_ODST_14_SYNTHMOD_HLP_000.rus",
            "f9-odst-rus.odst.SYNTHMOD.screenitems.DR_ODST_14_SYNTHMOD_HLP_000.rus",
        ]
    );
    // No test, no capability, no screen binding: those are the English pack's.
    assert!(all.records.iter().all(|entry| {
        !matches!(entry.record.key, ClaimKey::SupportsCapability { .. })
            && !matches!(&entry.record.key, ClaimKey::Custom { name } if name == ODST_TEST_CLAIM)
            && !matches!(&entry.record.key, ClaimKey::Custom { name } if name.starts_with("sdd_odst_help."))
    }));

    let text = store
        .get_record("f9-odst-rus.odst.SYNTHMOD.screen.DR_ODST_14_SYNTHMOD_HLP_000.rus")
        .unwrap();
    assert_eq!(
        text.key,
        ClaimKey::Custom {
            name: format!("{ODST_SCREEN_CLAIM_PREFIX}DR_ODST_14_SYNTHMOD_HLP_000.rus"),
        }
    );
    // J_A_ENS_IGN_ON has no Russian text, so the screen carries one line:
    // a missing line is missing, never invented.
    assert_eq!(
        text.value,
        KnowledgeValue::Text {
            value: "Синтетический блок выполняет собственную проверку и записывает результат."
                .into(),
        }
    );
    let items = store
        .get_record("f9-odst-rus.odst.SYNTHMOD.screenitems.DR_ODST_14_SYNTHMOD_HLP_000.rus")
        .unwrap();
    assert_eq!(
        items.key,
        ClaimKey::Custom {
            name: format!("{ODST_SCREEN_ITEMS_CLAIM_PREFIX}DR_ODST_14_SYNTHMOD_HLP_000.rus"),
        }
    );
    assert_eq!(
        items.value,
        KnowledgeValue::Text {
            value: "J_I_SYNTH_ODST_DESC\u{1F}Синтетический блок выполняет собственную проверку и записывает результат.".into(),
        }
    );
}

#[test]
fn the_two_packs_load_side_by_side_without_a_collision() {
    let mut store = KnowledgeStore::new();
    let english = OdstInfoAdapter::new(source("f9-odst", ENGLISH)).unwrap();
    store.ingest(&english, ENGLISH).unwrap();
    store
        .ingest(&russian_adapter("f9-odst-rus", RUSSIAN), RUSSIAN)
        .unwrap();
    assert!(store
        .get_record("f9-odst.odst.SYNTHMOD.screen.DR_ODST_14_SYNTHMOD_HLP_000")
        .is_some());
    assert!(store
        .get_record("f9-odst-rus.odst.SYNTHMOD.screen.DR_ODST_14_SYNTHMOD_HLP_000.rus")
        .is_some());
    assert_eq!(store.sources().iter().count(), 2);
}

#[test]
fn a_document_in_the_wrong_language_is_refused() {
    let mut store = KnowledgeStore::new();
    let error = store
        .ingest(&russian_adapter("f9-odst-rus", ENGLISH), ENGLISH)
        .expect_err("English is not Russian");
    assert!(
        error.to_string().contains("declares language \"en\""),
        "{error}"
    );
    assert_eq!(store.sources().iter().count(), 0, "no partial write");

    let english = OdstInfoAdapter::new(source("f9-odst", RUSSIAN)).unwrap();
    let error = store
        .ingest(&english, RUSSIAN)
        .expect_err("Russian is not English");
    assert!(
        error.to_string().contains("declares language \"ru\""),
        "{error}"
    );
}

#[test]
fn a_russian_module_with_no_used_screen_is_an_empty_batch_not_an_error() {
    let bare = r#"<?xml version="1.0" encoding="UTF-8"?>
<odstInfo moduleType="SYNTHMOD">
<language isoCode="ru">Russian</language>
<odstTestQualification>
<odstTestSelection testID="99" timeout="5000" time="5000" flag="1">
<odstTestQualifier model="SYNTHA" year="BASE"/>
</odstTestSelection>
</odstTestQualification>
</odstInfo>"#;
    let batch = russian_adapter("f9-odst-rus-bare", bare)
        .parse(bare)
        .expect("nothing to write is not a fault");
    assert!(batch.records.is_empty());
    assert!(batch.evidence.is_empty());
}

#[test]
fn a_language_sdd_ships_no_pack_for_is_refused_up_front() {
    let error = OdstInfoAdapter::new(source("f9-odst-deu", RUSSIAN))
        .unwrap()
        .with_language("deu")
        .expect_err("no German pack is known");
    assert!(error.to_string().contains("deu"), "{error}");
}
