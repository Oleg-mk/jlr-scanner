//! F9 golden tests for the SDD VIN decode document.
//!
//! The tables are kept verbatim as text claims; nothing is decoded at ingest.
//! What is fixed here is that every rule block and every attribute survives,
//! in a form the session decoder reads back exactly.

use knowledge::{
    sha256_bytes, ClaimKey, ContentFingerprint, EntityKind, KnowledgeQuery, KnowledgeStore,
    KnowledgeValue, RedistributionStatus, SourceId, SourceRecord, SourceType,
};
use sdd_ingest::{VinDecodeAdapter, VIN_ATTRIBUTE_CLAIM_PREFIX, VIN_RULE_CLAIM};

const VIN_DECODE: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_vin_decode.xml");

fn source() -> SourceRecord {
    SourceRecord {
        id: SourceId::new("f9-vin").unwrap(),
        title: "F9 VIN decode fixture".into(),
        source_type: SourceType::Documented,
        origin: "F9 golden test".into(),
        source_locator: "fixtures/knowledge/synthetic".into(),
        content_fingerprint: Some(
            ContentFingerprint::sha256(sha256_bytes(VIN_DECODE.as_bytes())).unwrap(),
        ),
        acquired_on: None,
        declared_vehicle_programs: vec![],
        provenance: "Synthetic fixture reproducing the SDD element shape only".into(),
        redistribution_status: RedistributionStatus::Permitted,
        notes: None,
    }
}

fn store() -> KnowledgeStore {
    let adapter = VinDecodeAdapter::new(source()).unwrap();
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter, VIN_DECODE).unwrap();
    store
}

fn text(store: &KnowledgeStore, id: &str) -> String {
    let record = store.get_record(id).unwrap();
    let KnowledgeValue::Text { value } = &record.value else {
        panic!("expected text");
    };
    value.clone()
}

#[test]
fn every_rule_block_becomes_one_claim_in_document_order() {
    let store = store();
    assert_eq!(text(&store, "f9-vin.vin.rule.001"), "1..3=SYN;12..12!=B");
    assert_eq!(text(&store, "f9-vin.vin.rule.002"), "1..3=SYN;12..12=B");
    assert_eq!(text(&store, "f9-vin.vin.rule.003"), "1..5=SYNLB");
    let rule = store.get_record("f9-vin.vin.rule.003").unwrap();
    assert_eq!(rule.entity.kind, EntityKind::VinDecodeModel);
    assert_eq!(rule.entity.id, "VIN-MODEL-2");
    assert_eq!(
        rule.key,
        ClaimKey::Custom {
            name: VIN_RULE_CLAIM.into()
        }
    );
}

#[test]
fn attributes_keep_constants_and_lookups_verbatim_with_escaped_text() {
    let store = store();
    assert_eq!(text(&store, "f9-vin.vin.model.1.brand"), "const=Synthetic");
    assert_eq!(
        text(&store, "f9-vin.vin.model.1.model"),
        "chars=6..7;01=SYNTHA|02=SYNTHA"
    );
    assert_eq!(
        text(&store, "f9-vin.vin.model.1.modelname"),
        "chars=6..7;01=Synth A saloon|02=Synth A estate%3B long"
    );
    assert_eq!(text(&store, "f9-vin.vin.model.2.model"), "const=SYNTHB");
    let attribute = store.get_record("f9-vin.vin.model.1.modelyear").unwrap();
    assert_eq!(
        attribute.key,
        ClaimKey::Custom {
            name: format!("{VIN_ATTRIBUTE_CLAIM_PREFIX}ModelYear")
        }
    );
    // The table's version travels with the evidence, not as a claim.
    let evidence = store
        .get_evidence(&attribute.evidence_ids[0])
        .expect("evidence recorded");
    assert_eq!(
        evidence.notes.as_deref(),
        Some("SDD table version: Synthetic VIN chart issue 1")
    );

    let all = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    assert_eq!(all.records.len(), 3 + 5 + 3);
}

#[test]
fn a_test_whose_value_does_not_span_its_positions_is_rejected() {
    let broken = VIN_DECODE.replace(
        r#"<Test CharPos="1,5" Operator="EQUAL" CharValue="SYNLB"/>"#,
        r#"<Test CharPos="1,5" Operator="EQUAL" CharValue="SYN"/>"#,
    );
    let adapter = VinDecodeAdapter::new(source()).unwrap();
    let mut store = KnowledgeStore::new();
    let error = store.ingest(&adapter, &broken).unwrap_err();
    assert!(error.to_string().contains("does not span positions 1..5"));
}
