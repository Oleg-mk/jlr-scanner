//! The fault-code help SDD carries, chosen for the car in front of us.
//!
//! SDD writes a different help screen for the same code depending on the
//! module, the model, the model year and the fault type. Two thirds of the
//! corpus's codes carry one; the rest carry a placeholder. What matters here
//! is that the right screen is chosen or none is: the neighbouring model
//! year's causes and actions would send someone to the wrong part.
//!
//! Since `ADR-0034` the words are SDD's own: English from the English pack,
//! and the same screen in Russian from the Russian one, joined line to line
//! by the mnemonic's name.

use knowledge::{
    sha256_bytes, ContentFingerprint, IngestionAdapter, RedistributionStatus, SourceId,
    SourceRecord, SourceType, VehicleContext,
};
use sdd_ingest::{DtcHelpAdapter, ModelYearTimeline, DTC_HELP_LANGUAGE_RUSSIAN};

const FIXTURE: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_dtc_help.xml");
const FIXTURE_RUS: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_dtc_help_rus.xml");

fn source(id: &str, locator: &str, text: &str) -> SourceRecord {
    SourceRecord {
        id: SourceId::new(id).unwrap(),
        title: format!("DTC help fixture {id}"),
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

fn timeline() -> ModelYearTimeline {
    // The designations the fixture uses, so a calendar year can tell MY06 from
    // MY07 (ADR-0011); MY08 bounds MY07 from above.
    let mut timeline = ModelYearTimeline::new();
    for marker in ["MY06", "MY07", "MY08"] {
        timeline.observe("SYNTHA", marker).unwrap();
    }
    timeline
}

/// The English pack alone — what every library issued before 2026-09-13 is.
fn english_library() -> diagnostic_session::KnowledgeLibrary {
    let batch = DtcHelpAdapter::new(source(
        "dtc-help-test",
        "fixtures/knowledge/synthetic/f9_dtc_help.xml",
        FIXTURE,
    ))
    .unwrap()
    .with_timeline(timeline())
    .parse(FIXTURE)
    .unwrap();
    diagnostic_session::KnowledgeLibrary::from_manifests([(
        "dtc_help.json",
        serde_json::to_string(&batch).unwrap().as_str(),
    )])
}

/// Both packs, as the exporter writes them since `ADR-0034`: one bundle,
/// the Russian text beside the English under its own source.
fn library() -> diagnostic_session::KnowledgeLibrary {
    let english = DtcHelpAdapter::new(source(
        "dtc-help-test",
        "fixtures/knowledge/synthetic/f9_dtc_help.xml",
        FIXTURE,
    ))
    .unwrap()
    .with_timeline(timeline())
    .parse(FIXTURE)
    .unwrap();
    let russian = DtcHelpAdapter::new(source(
        "dtc-help-test-rus",
        "fixtures/knowledge/synthetic/f9_dtc_help_rus.xml",
        FIXTURE_RUS,
    ))
    .unwrap()
    .with_timeline(timeline())
    .with_language(DTC_HELP_LANGUAGE_RUSSIAN)
    .unwrap()
    .parse(FIXTURE_RUS)
    .unwrap();
    let english = serde_json::to_string(&english).unwrap();
    let russian = serde_json::to_string(&russian).unwrap();
    diagnostic_session::KnowledgeLibrary::from_manifests([
        ("dtc_help.json", english.as_str()),
        ("dtc_help.json", russian.as_str()),
    ])
}

fn car(model_year: Option<u16>) -> VehicleContext {
    VehicleContext {
        vehicle_program: Some("SYNTHA".into()),
        model_year,
        ..VehicleContext::default()
    }
}

#[test]
fn the_model_year_chooses_the_screen_and_the_fault_type_chooses_the_wording() {
    let library = library();

    // A 2006 car gets the screen SDD gives MY06 for fault type 17, in SDD's
    // own English, colon and all (`ADR-0034`).
    let described = library.describe_dtc_with_help("0x0000", 17, "SYNTHMOD", &car(Some(2006)));
    assert_eq!(
        described.help,
        vec![
            "Possible causes:".to_string(),
            "Synthetic sensor circuit open, resistance above 2.30 ohms between -40°C and +85°C"
                .to_string(),
            "Actions required:".to_string(),
            "Refer to the synthetic circuit diagrams and test the sensor circuit.".to_string(),
        ]
    );
    assert!(described.help_note.is_none());

    // A 2007 car gets the other one — the same code, the same fault type.
    let later = library.describe_dtc_with_help("0x0000", 17, "SYNTHMOD", &car(Some(2007)));
    assert_eq!(
        later.help,
        vec![
            "Possible causes:".to_string(),
            "Synthetic sensor circuit – short to ground".to_string(),
        ]
    );
}

#[test]
fn the_russian_pack_gives_the_same_screen_line_for_line_by_name() {
    let library = library();

    // The MY07 screen: both lines have Russian.
    let later = library.describe_dtc_with_help("0x0000", 17, "SYNTHMOD", &car(Some(2007)));
    assert_eq!(
        later.help_texts.get("rus"),
        Some(&vec![
            "Возможные причины:".to_string(),
            "Синтетическая цепь датчика – короткое замыкание на массу".to_string(),
        ]),
        "{:?}",
        later.help_texts
    );
    // Russian is the only other language the library has; nothing is
    // invented for a language SDD does not ship.
    assert_eq!(later.help_texts.len(), 1);

    // The MY06 screen: the Russian pack has no text for SYNTH_ACTION_1, so
    // that line keeps its English and the list stays the screen's length.
    let described = library.describe_dtc_with_help("0x0000", 17, "SYNTHMOD", &car(Some(2006)));
    let russian = described.help_texts.get("rus").expect("the Russian screen");
    assert_eq!(russian.len(), described.help.len());
    assert_eq!(russian[0], "Возможные причины:");
    assert_eq!(russian[2], "Необходимые действия:");
    assert_eq!(
        russian[3],
        "Refer to the synthetic circuit diagrams and test the sensor circuit."
    );
}

#[test]
fn a_library_without_the_russian_pack_reads_as_it_always_did() {
    let library = english_library();
    let described = library.describe_dtc_with_help("0x0000", 17, "SYNTHMOD", &car(Some(2006)));
    assert_eq!(described.help.len(), 4);
    assert!(
        described.help_texts.is_empty(),
        "{:?}",
        described.help_texts
    );
}

#[test]
fn a_car_described_too_loosely_is_told_so_rather_than_given_the_wrong_screen() {
    let library = library();
    // No model year: both screens survive, and neither is shown.
    let vague = library.describe_dtc_with_help("0x0000", 17, "SYNTHMOD", &car(None));
    assert!(vague.help.is_empty());
    assert!(vague.help_texts.is_empty());
    assert!(
        vague
            .help_note
            .as_deref()
            .is_some_and(|note| note.contains("model year")),
        "{:?}",
        vague.help_note
    );
}

#[test]
fn help_belongs_to_the_module_and_the_fault_type_that_carry_it() {
    let library = library();

    // Another module's read of the same code gets no help from this one.
    let elsewhere = library.describe_dtc_with_help("0x0000", 17, "OTHERMOD", &car(Some(2006)));
    assert!(elsewhere.help.is_empty());
    assert!(elsewhere.help_note.is_none());

    // A fault type the data has no screen for gets nothing, not the nearest
    // screen: the code is the same, the fault is not.
    let other_fault = library.describe_dtc_with_help("0x0000", 18, "SYNTHMOD", &car(Some(2006)));
    assert!(other_fault.help.is_empty());

    // A code the data does not carry at all.
    let unknown = library.describe_dtc_with_help("P0301", 0, "SYNTHMOD", &car(Some(2006)));
    assert!(unknown.help.is_empty());
    assert!(unknown.help_note.is_none());
}
