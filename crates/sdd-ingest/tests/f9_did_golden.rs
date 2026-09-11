//! F9 golden tests for the SDD DID formatting catalogue.
//!
//! The fixtures are synthetic and prove parser behaviour only. The rejection
//! tests matter as much as the parsing ones: an unrecognised qualification or a
//! non-read parameter must stop ingestion rather than be quietly dropped, since
//! either would widen what the knowledge base claims beyond its source.

use knowledge::{
    sha256_bytes, ContentFingerprint, DimensionConstraint, EntityKind, EvidenceClass,
    KnowledgeQuery, KnowledgeStore, KnowledgeValue, RedistributionStatus, SourceId, SourceRecord,
    SourceType, ValidationState, YearConstraint,
};
use sdd_ingest::{ConverterCatalogue, DidFormattingAdapter, YEAR_BREAKPOINT_DIMENSION};

const FORMATTING: &str =
    include_str!("../../../fixtures/knowledge/synthetic/f9_did_formatting.xml");
const CONVERTER: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_converter.xml");
const CONVERTER_KM: &str =
    include_str!("../../../fixtures/knowledge/synthetic/f9_converter_km.xml");

fn converters() -> ConverterCatalogue {
    let mut catalogue = ConverterCatalogue::new();
    catalogue.insert_from_xml(CONVERTER).unwrap();
    catalogue.insert_from_xml(CONVERTER_KM).unwrap();
    catalogue
}

fn source(source_type: SourceType) -> SourceRecord {
    SourceRecord {
        id: SourceId::new("f9-did").unwrap(),
        title: "F9 DID formatting fixture".into(),
        source_type,
        origin: "F9 golden test".into(),
        source_locator: "fixtures/knowledge/synthetic/f9_did_formatting.xml".into(),
        content_fingerprint: source_type
            .is_real_evidence()
            .then(|| ContentFingerprint::sha256(sha256_bytes(FORMATTING.as_bytes())).unwrap()),
        acquired_on: None,
        declared_vehicle_programs: vec![],
        provenance: "Synthetic fixture reproducing the SDD element shape only".into(),
        redistribution_status: RedistributionStatus::Permitted,
        notes: None,
    }
}

fn ingest(source_type: SourceType) -> KnowledgeStore {
    let adapter = DidFormattingAdapter::new(source(source_type), converters()).unwrap();
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter, FORMATTING).unwrap();
    store
}

fn reject(input: &str) -> String {
    let adapter = DidFormattingAdapter::new(source(SourceType::Documented), converters()).unwrap();
    let mut store = KnowledgeStore::new();
    let error = store
        .ingest(&adapter, input)
        .expect_err("input must be rejected");
    assert_eq!(store.sources().iter().count(), 0, "no partial write");
    error.to_string()
}

#[test]
fn converter_supplies_the_unit_and_a_missing_one_is_not_invented() {
    let store = ingest(SourceType::Documented);

    let record = store.get_record("f9-did.did.did-0x0301.p1").unwrap();
    assert_eq!(record.entity.kind, EntityKind::IdentifierParameter);
    assert_eq!(record.entity.id, "DID-0x0301");
    let KnowledgeValue::IdentifierDefinition {
        identifier,
        encoding,
        unit,
    } = &record.value
    else {
        panic!("expected an identifier definition");
    };
    assert_eq!(identifier, "0x0301");
    assert_eq!(unit.as_deref(), Some("V"));
    assert_eq!(
        encoding.as_deref(),
        Some("bytes=0..1;size=2;mask=0xffff;converter=CVT_SYNTH_VOLT;scale=1;offset=0;offset_first=true;states=65535..65535=Sensor short%3B value%3Dinvalid")
    );

    // The second packed parameter references a converter the catalogue does not
    // hold. It is still recorded, but no unit is guessed for it.
    let unknown_converter = store.get_record("f9-did.did.did-0x0343.p2").unwrap();
    let KnowledgeValue::IdentifierDefinition { unit, encoding, .. } = &unknown_converter.value
    else {
        panic!("expected an identifier definition");
    };
    assert_eq!(*unit, None);
    assert!(encoding.as_deref().unwrap().contains("mask=all"));
}

#[test]
fn bit_packed_parameters_are_kept_apart_by_byte_range() {
    let store = ingest(SourceType::Documented);

    let first = store.get_record("f9-did.did.did-0x0343.p1").unwrap();
    let second = store.get_record("f9-did.did.did-0x0343.p2").unwrap();
    assert_eq!(first.entity, second.entity);
    assert_ne!(first.key, second.key);

    let KnowledgeValue::IdentifierDefinition { encoding, .. } = &first.value else {
        panic!("expected an identifier definition");
    };
    assert!(encoding.as_deref().unwrap().starts_with("bytes=0..3"));
    let KnowledgeValue::IdentifierDefinition { encoding, .. } = &second.value else {
        panic!("expected an identifier definition");
    };
    assert!(encoding.as_deref().unwrap().starts_with("bytes=4..7"));
}

#[test]
fn qualification_maps_onto_applicability_without_inventing_a_year_range() {
    let store = ingest(SourceType::Documented);

    let module_only = store
        .get_record("f9-did.did.did-0x1945-synthmod.p1")
        .unwrap();
    assert_eq!(
        module_only.applicability.ecu_family,
        DimensionConstraint::one_of(["SYNTHMOD".to_string()]).unwrap()
    );
    // The expression tests module but not model, and SDD qualification is a
    // conjunction, so model is unconstrained rather than undetermined. Any is
    // what lets a module-scoped record resolve at all; Unknown would silently
    // make the whole qualified corpus unusable.
    assert_eq!(
        module_only.applicability.vehicle_program,
        DimensionConstraint::Any
    );
    // The same predicate reading applies to the year: no year test means the
    // entry is enabled for every model year, so it is Any rather than Unknown.
    assert_eq!(module_only.applicability.model_year, YearConstraint::Any);

    let fully = store
        .get_record("f9-did.did.did-0x0347-synthmod-1.p1")
        .unwrap();
    assert_eq!(
        fully.applicability.vehicle_program,
        DimensionConstraint::one_of(["SYNTHA".to_string()]).unwrap()
    );
    assert_eq!(
        fully.applicability.powertrain,
        DimensionConstraint::one_of(["SYNTHENGINE".to_string()]).unwrap()
    );

    // The breakpoint is recorded verbatim on its own dimension, and the model
    // year stays unknown because the source never states what MY10 spans.
    assert_eq!(fully.applicability.model_year, YearConstraint::Unknown);
    assert_eq!(
        fully.applicability.other.get(YEAR_BREAKPOINT_DIMENSION),
        Some(&DimensionConstraint::one_of(["MY10".to_string()]).unwrap())
    );
}

#[test]
fn unsupported_qualification_and_non_read_access_are_rejected() {
    // An unrecognised tactic would silently widen applicability if ignored.
    let unknown_tactic = FORMATTING.replace(
        "<STRING-TEST-TACTIC id=\"type\">",
        "<STRING-TEST-TACTIC id=\"marketRegion\">",
    );
    assert!(reject(&unknown_tactic).contains("unrecognised qualification"));

    // Disjunction is not modelled, so it must stop rather than be flattened.
    // Both tags are swapped so the document stays well formed and it is the
    // qualification guard that rejects it, not the XML parser.
    let disjunction = FORMATTING
        .replace("<AND id=\"Row1\">", "<OR id=\"Row1\">")
        .replace("</AND>", "</OR>");
    assert!(reject(&disjunction).contains("unsupported <OR> qualification"));

    // Anything other than a ReadParameter would be a non-read access path.
    // Both tags are swapped so the document stays well formed.
    let writeable = FORMATTING
        .replace("<ReadParameter", "<WriteParameter")
        .replace("</ReadParameter>", "</WriteParameter>");
    assert!(reject(&writeable).contains("non-read element"));
}

#[test]
fn a_key_size_disagreeing_with_the_declared_identifier_is_rejected() {
    let mismatched = FORMATTING.replace("<keySize>769</keySize>", "<keySize>770</keySize>");
    assert!(reject(&mismatched).contains("keySize"));
}

#[test]
fn classification_follows_the_source_type() {
    let synthetic = ingest(SourceType::Synthetic);
    assert_eq!(
        synthetic
            .get_record("f9-did.did.did-0x0301.p1")
            .unwrap()
            .validation_state,
        ValidationState::Unverified
    );
    assert_eq!(
        synthetic.trace_back("f9-did.did.did-0x0301.p1").unwrap()[0]
            .evidence
            .evidence_class,
        Some(EvidenceClass::SyntheticTest)
    );

    let documented = ingest(SourceType::Documented);
    assert_eq!(
        documented
            .get_record("f9-did.did.did-0x0301.p1")
            .unwrap()
            .validation_state,
        ValidationState::SourceBacked
    );
    assert_eq!(
        documented.trace_back("f9-did.did.did-0x0301.p1").unwrap()[0]
            .evidence
            .evidence_class,
        Some(EvidenceClass::OemDocumentation)
    );
}

#[test]
fn every_parameter_is_recorded_and_ingestion_is_idempotent() {
    let adapter = DidFormattingAdapter::new(source(SourceType::Documented), converters()).unwrap();
    let mut store = KnowledgeStore::new();
    let first = store.ingest(&adapter, FORMATTING).unwrap();
    let second = store.ingest(&adapter, FORMATTING).unwrap();
    assert_eq!(first, second);
    // Five formatting parameters, plus the two on 0xDD01 that the mileage
    // survey reads: the distance itself and the lamp counter beside it.
    assert_eq!(first.record_ids.len(), 7);

    let all = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    assert_eq!(all.records.len(), 7);
}
