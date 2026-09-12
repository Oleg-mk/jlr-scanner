//! F9 golden tests for SDD on-demand self test definitions.
//!
//! The load-bearing test here is the safety one. An on-demand self test commands
//! an ECU to act, so it is stage-2 material that may be recorded as known but
//! must never appear as an available read-only operation. Everything else in
//! this file supports that guarantee.

use knowledge::{
    sha256_bytes, ClaimKey, ContentFingerprint, DiagnosticSafetyClass, DimensionConstraint,
    EntityKind, EvidenceClass, KnowledgeQuery, KnowledgeStore, KnowledgeValue,
    RedistributionStatus, SourceId, SourceRecord, SourceType, ValidationState, YearConstraint,
};
use sdd_ingest::{OdstInfoAdapter, MODEL_YEAR_DESIGNATION_DIMENSION, QUAL_DIMENSION_PREFIX};

const FIXTURE: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_odst_info.xml");

fn source(source_type: SourceType) -> SourceRecord {
    SourceRecord {
        id: SourceId::new("f9-odst").unwrap(),
        title: "F9 ODST fixture".into(),
        source_type,
        origin: "F9 golden test".into(),
        source_locator: "fixtures/knowledge/synthetic/f9_odst_info.xml".into(),
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
    let adapter = OdstInfoAdapter::new(source(source_type)).unwrap();
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter, FIXTURE).unwrap();
    store
}

fn reject(input: &str) -> String {
    let adapter = OdstInfoAdapter::new(source(SourceType::Documented)).unwrap();
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
fn every_self_test_is_a_service_routine_and_never_read_only() {
    let store = ingest(SourceType::Documented);
    let all = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    assert!(!all.records.is_empty());

    let mut capabilities = 0;
    for entry in &all.records {
        let KnowledgeValue::Capability {
            safety_class,
            supported,
            ..
        } = &entry.record.value
        else {
            // Beside every capability the adapter records what the test is
            // and what SDD tells the technician about it (ADR-0032). Those
            // are texts; they command nothing.
            assert!(
                matches!(&entry.record.value, KnowledgeValue::Text { .. }),
                "unexpected value for {}",
                entry.record.id
            );
            continue;
        };
        capabilities += 1;
        assert!(*supported);
        assert_eq!(
            *safety_class,
            Some(DiagnosticSafetyClass::ServiceRoutine),
            "{} must be a service routine",
            entry.record.id
        );
        assert_ne!(
            *safety_class,
            Some(DiagnosticSafetyClass::ReadOnly),
            "{} must never be read-only",
            entry.record.id
        );
    }
    assert_eq!(capabilities, 4, "four qualified self tests are declared");
}

#[test]
fn a_named_test_uses_its_help_screen_name_and_an_unnamed_one_falls_back() {
    let store = ingest(SourceType::Documented);

    let named = store.get_record("f9-odst.odst.SYNTHMOD.14.0").unwrap();
    assert_eq!(named.entity.kind, EntityKind::DiagnosticCapability);
    assert_eq!(named.entity.id, "ODST-SYNTHMOD-14");
    let KnowledgeValue::Capability { name, .. } = &named.value else {
        panic!("expected a capability");
    };
    assert_eq!(name, "DR_ODST_14_SYNTHMOD");

    // A selection with no help-screen name still yields a usable claim rather
    // than being dropped or given a fabricated name.
    let unnamed = store.get_record("f9-odst.odst.SYNTHMOD.99.0").unwrap();
    let KnowledgeValue::Capability { name, .. } = &unnamed.value else {
        panic!("expected a capability");
    };
    assert_eq!(name, "ODST test 99");
}

#[test]
fn qualifiers_narrow_applicability_and_vendor_ones_are_kept_verbatim() {
    let store = ingest(SourceType::Documented);

    let bare = store.get_record("f9-odst.odst.SYNTHMOD.14.0").unwrap();
    assert_eq!(bare.applicability.ecu_family, one_of("SYNTHMOD"));
    assert_eq!(bare.applicability.vehicle_program, one_of("SYNTHA"));
    // The qualifier names no engine type, and SDD qualification is a
    // conjunction, so powertrain is unconstrained rather than undetermined.
    assert_eq!(bare.applicability.powertrain, DimensionConstraint::Any);

    let typed = store.get_record("f9-odst.odst.SYNTHMOD.14.1").unwrap();
    assert_eq!(typed.applicability.powertrain, one_of("SYNTHENGINE"));
    assert_eq!(typed.applicability.variant, one_of("SYNTHVARIANT"));

    // A vendor qualifier is recorded under its own dimension rather than being
    // mapped onto powertrain, which would assert a meaning the source never gives.
    let vendor = store.get_record("f9-odst.odst.SYNTHMOD.14.2").unwrap();
    assert_eq!(vendor.applicability.powertrain, DimensionConstraint::Any);
    assert_eq!(
        vendor
            .applicability
            .other
            .get(&format!("{QUAL_DIMENSION_PREFIX}cm_qual_eng_type")),
        Some(&one_of("VAL_ENG_SYNTH"))
    );

    // Model years remain designations, consistently with the DTC slice.
    assert_eq!(bare.applicability.model_year, YearConstraint::Unknown);
    assert_eq!(
        bare.applicability
            .other
            .get(MODEL_YEAR_DESIGNATION_DIMENSION),
        Some(&one_of("MY03"))
    );
}

#[test]
fn unrecognised_qualifier_attributes_and_malformed_input_are_rejected() {
    let unknown = FIXTURE.replace("model=\"SYNTHA\" year=\"MY03\"", "marketRegion=\"NAS\"");
    assert!(reject(&unknown).contains("unrecognised attribute"));

    assert!(reject("<other><odstInfo/></other>").contains("expected an <odstInfo> root"));

    let no_tests = FIXTURE
        .replace("<odstTestQualification>", "<unusedQualification>")
        .replace("</odstTestQualification>", "</unusedQualification>");
    assert!(reject(&no_tests).contains("no qualified self tests"));
}

/// ADR-0032: the list carries SDD's own description of each test, and the
/// screen a car is given is recorded apart from what the screen says, so a
/// screen many cars select is written once.
#[test]
fn a_test_carries_its_timings_and_the_words_sdd_writes_for_whoever_runs_it() {
    let store = ingest(SourceType::Documented);

    let described = store
        .get_record("f9-odst.odst.SYNTHMOD.14.0.described")
        .expect("the test is described");
    let KnowledgeValue::Text { value } = &described.value else {
        panic!("expected a text");
    };
    assert_eq!(
        value,
        "test=14;name=DR_ODST_14_SYNTHMOD;time_ms=40000;timeout_ms=40000"
    );
    assert_eq!(
        described.key,
        ClaimKey::Custom {
            name: "sdd_odst_test".into()
        }
    );

    // Which screen this car is given, on the test's own entity.
    let given = store
        .get_record("f9-odst.odst.SYNTHMOD.14.help.0.0")
        .expect("a car is given a screen");
    assert_eq!(
        given.key,
        ClaimKey::Custom {
            name: "sdd_odst_help.DR_ODST_14_SYNTHMOD_HLP_000".into()
        }
    );

    // What that screen says, written once for the module: SDD's lines in
    // order, the blank item and the item with no text left out.
    let screen = store
        .get_record("f9-odst.odst.SYNTHMOD.screen.DR_ODST_14_SYNTHMOD_HLP_000")
        .expect("the screen says something");
    let KnowledgeValue::Text { value } = &screen.value else {
        panic!("expected a text");
    };
    assert_eq!(
        value,
        "The synthetic module runs its own check and logs what it finds.\nMake sure the ignition is switched on."
    );
    // A screen no test points at is not recorded at all.
    assert!(store
        .get_record("f9-odst.odst.SYNTHMOD.screen.DR_ODST_UNUSED_HLP_000")
        .is_none());
}

#[test]
fn classification_follows_the_source_type_and_ingestion_is_idempotent() {
    let synthetic = ingest(SourceType::Synthetic);
    assert_eq!(
        synthetic
            .get_record("f9-odst.odst.SYNTHMOD.14.0")
            .unwrap()
            .validation_state,
        ValidationState::Unverified
    );
    assert_eq!(
        synthetic.trace_back("f9-odst.odst.SYNTHMOD.14.0").unwrap()[0]
            .evidence
            .evidence_class,
        Some(EvidenceClass::SyntheticTest)
    );

    let adapter = OdstInfoAdapter::new(source(SourceType::Documented)).unwrap();
    let mut store = KnowledgeStore::new();
    let first = store.ingest(&adapter, FIXTURE).unwrap();
    let second = store.ingest(&adapter, FIXTURE).unwrap();
    assert_eq!(first, second);
    // Four qualified capabilities, four descriptions beside them, the
    // screen a car is given and the screen's own text (ADR-0032).
    assert_eq!(first.record_ids.len(), 10);
    assert_eq!(
        store
            .get_record("f9-odst.odst.SYNTHMOD.14.0")
            .unwrap()
            .validation_state,
        ValidationState::SourceBacked
    );
}
