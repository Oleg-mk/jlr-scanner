//! F9 golden tests for a help pack in another language (`ADR-0034`).
//!
//! SDD ships the fault-code help once per language, the same 6,171 files
//! with the same names, screens and selections, and only the human text
//! changed. The adapter read as Russian contributes that text and nothing
//! else — under the English claim's name with the language last — and it
//! refuses a document that does not say it is in that language.

use knowledge::{
    sha256_bytes, ClaimKey, ContentFingerprint, KnowledgeQuery, KnowledgeStore, KnowledgeValue,
    RedistributionStatus, SourceId, SourceRecord, SourceType,
};
use sdd_ingest::{
    DtcHelpAdapter, DTC_HELP_CLAIM, DTC_HELP_LANGUAGE_RUSSIAN, DTC_HELP_SCREEN_CLAIM_PREFIX,
    DTC_HELP_SCREEN_ITEMS_CLAIM_PREFIX,
};

const ENGLISH: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_dtc_help.xml");
const RUSSIAN: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_dtc_help_rus.xml");

fn source(id: &str, text: &str) -> SourceRecord {
    SourceRecord {
        id: SourceId::new(id).unwrap(),
        title: format!("F9 DTC help fixture {id}"),
        source_type: SourceType::Documented,
        origin: "F9 golden test".into(),
        source_locator: "fixtures/knowledge/synthetic/f9_dtc_help_rus.xml".into(),
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

fn russian_adapter(id: &str, text: &str) -> DtcHelpAdapter {
    DtcHelpAdapter::new(source(id, text))
        .unwrap()
        .with_language(DTC_HELP_LANGUAGE_RUSSIAN)
        .unwrap()
}

#[test]
fn a_russian_pack_writes_the_text_of_each_screen_and_nothing_else() {
    let mut store = KnowledgeStore::new();
    store
        .ingest(&russian_adapter("f9-dtc-rus", RUSSIAN), RUSSIAN)
        .unwrap();

    let all = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    let mut ids: Vec<String> = all
        .records
        .iter()
        .map(|entry| entry.record.id.clone())
        .collect();
    ids.sort();
    // Two screens are selected by some car; each is written as its text and
    // as its items, and every id ends in the language. The empty screen the
    // fixture keeps produces no claim, as it does in English.
    assert_eq!(
        ids,
        vec![
            "f9-dtc-rus.dtc.0x0000.helpscreen.flt-type-17-synth-default-hlp-001.rus",
            "f9-dtc-rus.dtc.0x0000.helpscreen.flt-type-17-synth-default-hlp-002.rus",
            "f9-dtc-rus.dtc.0x0000.helpscreenitems.flt-type-17-synth-default-hlp-001.rus",
            "f9-dtc-rus.dtc.0x0000.helpscreenitems.flt-type-17-synth-default-hlp-002.rus",
        ]
    );

    // No description, no alias, no selection: those are the English pack's.
    assert!(all.records.iter().all(|entry| {
        !matches!(entry.record.key, ClaimKey::Alias)
            && !matches!(&entry.record.key, ClaimKey::Custom { name } if name == DTC_HELP_CLAIM)
    }));

    // The claim names are the English ones with the language last.
    let text = store
        .get_record("f9-dtc-rus.dtc.0x0000.helpscreen.flt-type-17-synth-default-hlp-002.rus")
        .unwrap();
    assert_eq!(
        text.key,
        ClaimKey::Custom {
            name: format!("{DTC_HELP_SCREEN_CLAIM_PREFIX}FLT_TYPE_17_SYNTH_DEFAULT_HLP_002.rus"),
        }
    );
    assert_eq!(
        text.value,
        KnowledgeValue::Text {
            value: "Возможные причины:\nСинтетическая цепь датчика – короткое замыкание на массу"
                .into(),
        }
    );
    let items = store
        .get_record("f9-dtc-rus.dtc.0x0000.helpscreenitems.flt-type-17-synth-default-hlp-001.rus")
        .unwrap();
    assert_eq!(
        items.key,
        ClaimKey::Custom {
            name: format!(
                "{DTC_HELP_SCREEN_ITEMS_CLAIM_PREFIX}FLT_TYPE_17_SYNTH_DEFAULT_HLP_001.rus"
            ),
        }
    );
    // SYNTH_ACTION_1 has no Russian text, so the screen's items are the
    // three the pack does carry: a missing line is missing, never invented.
    let KnowledgeValue::Text { value } = &items.value else {
        panic!("items are text");
    };
    let names: Vec<&str> = value
        .lines()
        .filter_map(|line| line.split_once('\u{1F}').map(|(name, _)| name))
        .collect();
    assert_eq!(
        names,
        vec![
            "J_I_POSSIBLE_CAUSES",
            "SYNTH_CAUSE_1",
            "J_I_ACTIONS_REQUIRED"
        ]
    );
}

#[test]
fn the_two_packs_load_side_by_side_without_a_collision() {
    let mut store = KnowledgeStore::new();
    let english = DtcHelpAdapter::new(source("f9-dtc", ENGLISH)).unwrap();
    store.ingest(&english, ENGLISH).unwrap();
    store
        .ingest(&russian_adapter("f9-dtc-rus", RUSSIAN), RUSSIAN)
        .unwrap();
    // The English screen text is untouched by the Russian one beside it.
    let english_text = store
        .get_record("f9-dtc.dtc.0x0000.helpscreen.flt-type-17-synth-default-hlp-002")
        .unwrap();
    assert_eq!(
        english_text.key,
        ClaimKey::Custom {
            name: format!("{DTC_HELP_SCREEN_CLAIM_PREFIX}FLT_TYPE_17_SYNTH_DEFAULT_HLP_002"),
        }
    );
    assert_eq!(store.sources().iter().count(), 2);
}

#[test]
fn a_document_in_the_wrong_language_is_refused() {
    // The English document read as Russian: the same file names exist in
    // both packs, so a root mounted under the wrong name would otherwise
    // write English under the Russian claim.
    let mut store = KnowledgeStore::new();
    let error = store
        .ingest(&russian_adapter("f9-dtc-rus", ENGLISH), ENGLISH)
        .expect_err("English is not Russian");
    assert!(
        error.to_string().contains("declares language \"en\""),
        "{error}"
    );
    assert_eq!(store.sources().iter().count(), 0, "no partial write");

    // And the Russian document read as English.
    let english = DtcHelpAdapter::new(source("f9-dtc", RUSSIAN)).unwrap();
    let error = store
        .ingest(&english, RUSSIAN)
        .expect_err("Russian is not English");
    assert!(
        error.to_string().contains("declares language \"ru\""),
        "{error}"
    );
}

/// A third of the corpus's documents select no help screen at all. In
/// English such a document still carries its descriptions; in Russian it
/// carries nothing this adapter writes, and that is an empty batch, not a
/// rejected document.
#[test]
fn a_russian_document_with_no_selected_screen_is_an_empty_batch_not_an_error() {
    use knowledge::IngestionAdapter;
    let bare = r#"<?xml version="1.0" encoding="UTF-8"?>
<dtcHelp>
<dtcCode>0x0001</dtcCode>
<language isoCode="ru">Russian</language>
<dtcDescriptionList>
<dtcDescription id="1001">Синтетичный текст без экрана</dtcDescription>
</dtcDescriptionList>
</dtcHelp>"#;
    let batch = russian_adapter("f9-dtc-rus-bare", bare)
        .parse(bare)
        .expect("nothing to write is not a fault");
    assert!(batch.records.is_empty());
    assert!(batch.evidence.is_empty());

    // The same document in English is still refused: it should have
    // carried qualified descriptions and did not.
    let english = DtcHelpAdapter::new(source("f9-dtc-bare", bare)).unwrap();
    let bare_english = bare.replace(r#"isoCode="ru">Russian"#, r#"isoCode="en">English"#);
    assert!(english.parse(&bare_english).is_err());
}

#[test]
fn a_language_sdd_ships_no_pack_for_is_refused_up_front() {
    let error = DtcHelpAdapter::new(source("f9-dtc-deu", RUSSIAN))
        .unwrap()
        .with_language("deu")
        .expect_err("no German pack is known");
    assert!(error.to_string().contains("deu"), "{error}");
}
