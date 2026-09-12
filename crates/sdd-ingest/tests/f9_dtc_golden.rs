//! F9 golden tests for the SDD per-code DTC help documents.
//!
//! The fixture is synthetic and proves parser behaviour only. The central
//! assertion is that a model-year designation is never converted into a
//! calendar year: the real corpus spans MY94 to MY17, so mapping `MY##` onto
//! `2000 + ##` would silently place 1990s Jaguars in the 2090s.

use knowledge::{
    sha256_bytes, ApplicabilityResolution, ContentFingerprint, DimensionConstraint, EntityKind,
    EvidenceClass, KnowledgeQuery, KnowledgeStore, KnowledgeValue, RedistributionStatus, SourceId,
    SourceRecord, SourceType, ValidationState, VehicleContext, YearConstraint,
};
use sdd_ingest::{
    DtcHelpAdapter, DTC_FAULT_TYPE_DIMENSION, DTC_HELP_CLAIM, DTC_HELP_ITEM_SEPARATOR,
    DTC_HELP_SCREEN_ITEMS_CLAIM_PREFIX, DTC_TYPE_DIMENSION, MODEL_YEAR_DESIGNATION_DIMENSION,
    MODULE_DATA_NAME_DIMENSION,
};
use std::collections::BTreeMap;

const FIXTURE: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_dtc_help.xml");

fn source(source_type: SourceType) -> SourceRecord {
    SourceRecord {
        id: SourceId::new("f9-dtc").unwrap(),
        title: "F9 DTC help fixture".into(),
        source_type,
        origin: "F9 golden test".into(),
        source_locator: "fixtures/knowledge/synthetic/f9_dtc_help.xml".into(),
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
    let adapter = DtcHelpAdapter::new(source(source_type)).unwrap();
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter, FIXTURE).unwrap();
    store
}

fn reject(input: &str) -> String {
    let adapter = DtcHelpAdapter::new(source(SourceType::Documented)).unwrap();
    let mut store = KnowledgeStore::new();
    let error = store
        .ingest(&adapter, input)
        .expect_err("input must be rejected");
    assert_eq!(store.sources().iter().count(), 0, "no partial write");
    error.to_string()
}

fn one_of(value: &str) -> DimensionConstraint {
    DimensionConstraint::one_of([value.to_string()]).unwrap()
}

#[test]
fn each_qualifier_becomes_its_own_scoped_description() {
    let store = ingest(SourceType::Documented);

    let first = store.get_record("f9-dtc.dtc.0x0000.1001.0").unwrap();
    assert_eq!(first.entity.kind, EntityKind::DiagnosticTroubleCode);
    assert_eq!(first.entity.id, "DTC-0x0000");
    assert_eq!(
        first.value,
        KnowledgeValue::Text {
            value: "Synthetic watchdog reset".into()
        }
    );
    assert_eq!(first.applicability.ecu_family, one_of("SYNTHMOD"));
    assert_eq!(first.applicability.vehicle_program, one_of("SYNTHA"));
    assert_eq!(
        first.applicability.other.get(DTC_TYPE_DIMENSION),
        Some(&one_of("BASE"))
    );
    assert_eq!(
        first.applicability.other.get(MODULE_DATA_NAME_DIMENSION),
        Some(&one_of("SYNTHMOD_SYSTEM_A"))
    );

    // The same description under a second designation is a separate record, so
    // applicability stays exact instead of collapsing to a union.
    let second = store.get_record("f9-dtc.dtc.0x0000.1001.1").unwrap();
    assert_eq!(first.value, second.value);
    assert_eq!(
        first
            .applicability
            .other
            .get(MODEL_YEAR_DESIGNATION_DIMENSION),
        Some(&one_of("MY06"))
    );
    assert_eq!(
        second
            .applicability
            .other
            .get(MODEL_YEAR_DESIGNATION_DIMENSION),
        Some(&one_of("MY07"))
    );
}

#[test]
fn a_model_year_designation_never_becomes_a_calendar_year() {
    let store = ingest(SourceType::Documented);

    for id in ["f9-dtc.dtc.0x0000.1001.0", "f9-dtc.dtc.0x0000.1002.0"] {
        let record = store.get_record(id).unwrap();
        assert_eq!(
            record.applicability.model_year,
            YearConstraint::Unknown,
            "{id} must not carry an inferred year range"
        );
    }

    // The literal BASE occurs in the real corpus and carries no stated meaning,
    // so it is preserved rather than normalised away.
    let base = store.get_record("f9-dtc.dtc.0x0000.1002.0").unwrap();
    assert_eq!(
        base.applicability
            .other
            .get(MODEL_YEAR_DESIGNATION_DIMENSION),
        Some(&one_of("BASE"))
    );
}

#[test]
fn a_designation_filters_correctly_but_never_reaches_applicable() {
    let store = ingest(SourceType::Documented);

    let context = |designation: &str| VehicleContext {
        vehicle_program: Some("SYNTHA".into()),
        ecu_family: Some("SYNTHMOD".into()),
        other: BTreeMap::from([
            (
                MODEL_YEAR_DESIGNATION_DIMENSION.to_string(),
                designation.to_string(),
            ),
            (DTC_TYPE_DIMENSION.to_string(), "BASE".to_string()),
            (
                MODULE_DATA_NAME_DIMENSION.to_string(),
                "SYNTHMOD_SYSTEM_A".to_string(),
            ),
        ]),
        ..VehicleContext::default()
    };

    // The designation dimension filters exactly: MY06 keeps only the MY06
    // record and excludes the MY07 one entirely.
    let matching: Vec<_> = store
        .query(&KnowledgeQuery::for_vehicle(context("MY06")).include_indeterminate(true))
        .records
        .into_iter()
        .map(|entry| (entry.record.id, entry.applicability_resolution))
        .collect();
    // Never Applicable: model_year stays unknown because a designation is not
    // a calendar year, so the record cannot answer a calendar-year question
    // and says so.
    assert!(matching.contains(&(
        "f9-dtc.dtc.0x0000.1001.0".to_string(),
        ApplicabilityResolution::InsufficientEvidence
    )));
    assert!(matching.contains(&(
        "f9-dtc.dtc.0x0000.help.synthmod-syntha-my06-17".to_string(),
        ApplicabilityResolution::InsufficientEvidence
    )));
    assert!(
        !matching.iter().any(|(id, _)| id.contains("my07")),
        "MY06 must not drag in the MY07 record: {matching:?}"
    );

    // A designation the record does not carry excludes it outright. What a
    // help screen says is not qualified by any of this — the text is the text,
    // and it is the selection above that a car has to match — so the screen
    // records stay and nothing that names a car does.
    let non_matching: Vec<_> = store
        .query(&KnowledgeQuery::for_vehicle(context("MY99")).include_indeterminate(true))
        .records
        .into_iter()
        .map(|entry| entry.record.id)
        .collect();
    assert!(
        non_matching
            .iter()
            .all(|id| id.contains(".helpscreen.") || id.contains(".helpscreenitems.")),
        "{non_matching:?}"
    );
}

#[test]
fn a_selection_without_a_defined_description_is_skipped() {
    let store = ingest(SourceType::Documented);
    assert!(store.get_record("f9-dtc.dtc.0x0000.1003.0").is_none());

    // Three descriptions, the two help screens the fault type selects, and
    // what those two screens say.
    let all = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    assert_eq!(
        all.records
            .iter()
            .map(|entry| entry.record.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "f9-dtc.dtc.0x0000.1001.0",
            "f9-dtc.dtc.0x0000.1001.1",
            "f9-dtc.dtc.0x0000.1002.0",
            "f9-dtc.dtc.0x0000.help.synthmod-syntha-my06-17",
            "f9-dtc.dtc.0x0000.help.synthmod-syntha-my07-17",
            "f9-dtc.dtc.0x0000.helpscreen.flt-type-17-synth-default-hlp-001",
            "f9-dtc.dtc.0x0000.helpscreen.flt-type-17-synth-default-hlp-002",
            "f9-dtc.dtc.0x0000.helpscreenitems.flt-type-17-synth-default-hlp-001",
            "f9-dtc.dtc.0x0000.helpscreenitems.flt-type-17-synth-default-hlp-002",
        ]
    );
}

#[test]
fn unrecognised_qualification_and_malformed_input_are_rejected() {
    // A qualifier attribute the parser does not understand would narrow or
    // widen the claim if dropped, so ingestion stops.
    let unknown = FIXTURE.replace("dtcType=\"BASE\"", "marketRegion=\"NAS\"");
    assert!(reject(&unknown).contains("unrecognised attribute"));

    assert!(reject("<dtcHelp>").contains("dtcHelp") || !reject("<dtcHelp>").is_empty());
    assert!(reject("<other><dtcCode>0x1</dtcCode></other>").contains("expected a <dtcHelp> root"));

    let no_qualifiers = FIXTURE
        .replace("<dtcDescriptionQualification>", "<unusedQualification>")
        .replace("</dtcDescriptionQualification>", "</unusedQualification>");
    assert!(reject(&no_qualifiers).contains("no qualified descriptions"));
}

#[test]
fn classification_follows_the_source_type_and_ingestion_is_idempotent() {
    let synthetic = ingest(SourceType::Synthetic);
    assert_eq!(
        synthetic
            .get_record("f9-dtc.dtc.0x0000.1001.0")
            .unwrap()
            .validation_state,
        ValidationState::Unverified
    );
    assert_eq!(
        synthetic.trace_back("f9-dtc.dtc.0x0000.1001.0").unwrap()[0]
            .evidence
            .evidence_class,
        Some(EvidenceClass::SyntheticTest)
    );

    let adapter = DtcHelpAdapter::new(source(SourceType::Documented)).unwrap();
    let mut store = KnowledgeStore::new();
    let first = store.ingest(&adapter, FIXTURE).unwrap();
    let second = store.ingest(&adapter, FIXTURE).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        store
            .get_record("f9-dtc.dtc.0x0000.1001.0")
            .unwrap()
            .validation_state,
        ValidationState::SourceBacked
    );
    assert_eq!(
        store.trace_back("f9-dtc.dtc.0x0000.1001.0").unwrap()[0]
            .evidence
            .evidence_class,
        Some(EvidenceClass::OemDocumentation)
    );
}

/// The help layer (2026-09-10): SDD carries, per code, screens of text chosen
/// by model, model year and fault type — possible causes, actions required,
/// monitoring conditions, in its own words. What is recorded is the chain as
/// it stands: which screen a car is given, and separately what the screen
/// says, so a screen dozens of cars select is written once.
#[test]
fn the_help_screen_a_car_is_given_is_recorded_apart_from_what_the_screen_says() {
    use knowledge::ClaimKey;
    let store = ingest(SourceType::Documented);

    // The same fault type gives MY06 and MY07 different screens.
    let first = store
        .get_record("f9-dtc.dtc.0x0000.help.synthmod-syntha-my06-17")
        .expect("the MY06 car is given a screen");
    assert_eq!(
        first.key,
        ClaimKey::Custom {
            name: DTC_HELP_CLAIM.into()
        }
    );
    assert_eq!(
        first.value,
        KnowledgeValue::Text {
            value: "FLT_TYPE_17_SYNTH_DEFAULT_HLP_001".into()
        }
    );
    assert_eq!(first.applicability.ecu_family, one_of("SYNTHMOD"));
    assert_eq!(first.applicability.vehicle_program, one_of("SYNTHA"));
    assert_eq!(
        first.applicability.other.get(DTC_FAULT_TYPE_DIMENSION),
        Some(&one_of("17")),
        "the fault type is part of what chooses the screen"
    );
    assert_eq!(
        store
            .get_record("f9-dtc.dtc.0x0000.help.synthmod-syntha-my07-17")
            .expect("the MY07 car is given its own screen")
            .value,
        KnowledgeValue::Text {
            value: "FLT_TYPE_17_SYNTH_DEFAULT_HLP_002".into()
        }
    );

    // What the screen says, once, in the order it says it, with the characters
    // the real corpus uses intact.
    let screen = store
        .get_record("f9-dtc.dtc.0x0000.helpscreen.flt-type-17-synth-default-hlp-001")
        .expect("the screen says something");
    let KnowledgeValue::Text { value } = &screen.value else {
        panic!("{:?}", screen.value)
    };
    assert_eq!(
        value.lines().collect::<Vec<_>>(),
        vec![
            "Possible causes:",
            "Synthetic sensor circuit open, resistance above 2.30 ohms between -40°C and +85°C",
            "Actions required:",
            "Refer to the synthetic circuit diagrams and test the sensor circuit.",
        ]
    );

    // A mnemonic the string table does not define, and one whose text is
    // empty, are left out rather than shown as a blank line.
    let sparse = store
        .get_record("f9-dtc.dtc.0x0000.helpscreen.flt-type-17-synth-default-hlp-002")
        .expect("the second screen says something too");
    let KnowledgeValue::Text { value } = &sparse.value else {
        panic!("{:?}", sparse.value)
    };
    assert_eq!(
        value.lines().collect::<Vec<_>>(),
        vec![
            "Possible causes:",
            "Synthetic sensor circuit – short to ground",
        ]
    );

    // The same screen by the names of its items (2026-09-12): the unit a
    // translation is keyed by, one line each whatever the text does, with
    // the name and the text parted by U+001F. A mnemonic the string table
    // does not define is left out here as it is above.
    let items = store
        .get_record("f9-dtc.dtc.0x0000.helpscreenitems.flt-type-17-synth-default-hlp-001")
        .expect("the screen is named item by item");
    assert_eq!(
        items.key,
        ClaimKey::Custom {
            name: format!(
                "{}FLT_TYPE_17_SYNTH_DEFAULT_HLP_001",
                DTC_HELP_SCREEN_ITEMS_CLAIM_PREFIX
            )
        }
    );
    let KnowledgeValue::Text { value } = &items.value else {
        panic!("{:?}", items.value)
    };
    assert_eq!(
        value
            .lines()
            .map(|line| line
                .split_once(DTC_HELP_ITEM_SEPARATOR)
                .expect("name and text"))
            .collect::<Vec<_>>(),
        vec![
            ("J_I_POSSIBLE_CAUSES", "Possible causes:"),
            (
                "SYNTH_CAUSE_1",
                "Synthetic sensor circuit open, resistance above 2.30 ohms between -40°C and +85°C"
            ),
            ("J_I_ACTIONS_REQUIRED", "Actions required:"),
            (
                "SYNTH_ACTION_1",
                "Refer to the synthetic circuit diagrams and test the sensor circuit."
            ),
        ]
    );
    let sparse_items = store
        .get_record("f9-dtc.dtc.0x0000.helpscreenitems.flt-type-17-synth-default-hlp-002")
        .expect("the sparse screen is named too");
    let KnowledgeValue::Text { value } = &sparse_items.value else {
        panic!("{:?}", sparse_items.value)
    };
    assert_eq!(value.lines().count(), 2, "{value}");

    // A screen with no items at all is no help: two thirds of the real corpus
    // is such screens, and they must produce no claim.
    assert!(store
        .get_record("f9-dtc.dtc.0x0000.helpscreen.synth-default-hlp-001")
        .is_none());
    assert!(store
        .get_record("f9-dtc.dtc.0x0000.help.othermod-synthb-base-any")
        .is_none());
}
