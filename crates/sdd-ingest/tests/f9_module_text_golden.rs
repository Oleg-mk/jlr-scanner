//! F9 golden tests for SDD's module-description text items.

use knowledge::{
    sha256_bytes, ClaimKey, ContentFingerprint, DimensionConstraint, EntityKind, KnowledgeStore,
    KnowledgeValue, RedistributionStatus, SourceId, SourceRecord, SourceType,
};
use sdd_ingest::{
    is_failure_type_id, is_module_description_id, ModuleTextAdapter,
    FAILURE_TYPE_NAME_CLAIM_PREFIX, LEGACY_MODULE_NAME_CLAIM_PREFIX, MODULE_NAME_CLAIM_PREFIX,
};

const TEXT: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_module_text.xml");

fn source(body: &str) -> SourceRecord {
    SourceRecord {
        id: SourceId::new("f9-text").unwrap(),
        title: "F9 module text fixture".into(),
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

#[test]
fn each_language_becomes_a_name_claim_on_the_ecu_family() {
    let adapter = ModuleTextAdapter::new(source(TEXT)).unwrap();
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter, TEXT).unwrap();

    let english = store
        .get_record("f9-text.module_name.SYNTHMOD.eng")
        .unwrap();
    assert_eq!(english.entity.kind, EntityKind::EcuFamily);
    assert_eq!(english.entity.id, "SYNTHMOD");
    assert_eq!(
        english.key,
        ClaimKey::Custom {
            name: format!("{MODULE_NAME_CLAIM_PREFIX}eng")
        }
    );
    assert_eq!(
        english.value,
        KnowledgeValue::Text {
            value: "Synthetic control module".into()
        }
    );
    // Vehicle-independent: the family's name holds wherever the family does.
    assert_eq!(
        english.applicability.vehicle_program,
        DimensionConstraint::Unknown
    );
    let russian = store
        .get_record("f9-text.module_name.SYNTHMOD.rus")
        .unwrap();
    assert_eq!(
        russian.value,
        KnowledgeValue::Text {
            value: "Синтетический блок управления".into()
        }
    );
    // An empty translation unit is not a name.
    assert!(store
        .get_record("f9-text.module_name.SYNTHMOD.fra")
        .is_none());
}

#[test]
fn the_earlier_text_family_is_kept_apart_and_other_items_are_refused() {
    let legacy = TEXT.replace("@J_14229_M_DESC_SYNTHMOD", "@J_M_DESC_SYNTHMOD");
    let adapter = ModuleTextAdapter::new(source(&legacy)).unwrap();
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter, &legacy).unwrap();
    let english = store
        .get_record("f9-text.module_name.SYNTHMOD.eng")
        .unwrap();
    assert_eq!(
        english.key,
        ClaimKey::Custom {
            name: format!("{LEGACY_MODULE_NAME_CLAIM_PREFIX}eng")
        }
    );

    assert!(is_module_description_id("@J_14229_M_DESC_PCM"));
    assert!(is_module_description_id("@J_M_DESC_ABS"));
    assert!(!is_module_description_id("@J_14229_M_ACRO_PCM"));
    let other = TEXT.replace("@J_14229_M_DESC_SYNTHMOD", "@J_COUNTRY_UA");
    let adapter = ModuleTextAdapter::new(source(&other)).unwrap();
    let mut store = KnowledgeStore::new();
    let error = store.ingest(&adapter, &other).unwrap_err();
    assert!(error.to_string().contains("not a module description"));
}

#[test]
fn a_failure_type_item_lands_on_the_decimal_ftb_entity() {
    // The text database numbers failure types in hexadecimal; 0x17 is 23.
    let item = TEXT
        .replace("@J_14229_M_DESC_SYNTHMOD", "@J_I_ISO15031_FAULT_TYPE_17")
        .replace(
            "Synthetic control module",
            "General electrical failure - circuit voltage above threshold.",
        );
    assert!(is_failure_type_id("@J_I_ISO15031_FAULT_TYPE_17"));
    let adapter = ModuleTextAdapter::new(source(&item)).unwrap();
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter, &item).unwrap();
    let english = store.get_record("f9-text.failure_type.23.eng").unwrap();
    assert_eq!(english.entity.kind, EntityKind::DiagnosticTroubleCode);
    assert_eq!(english.entity.id, "FTB-23");
    assert_eq!(
        english.key,
        ClaimKey::Custom {
            name: format!("{FAILURE_TYPE_NAME_CLAIM_PREFIX}eng")
        }
    );
    assert!(store.get_record("f9-text.failure_type.23.rus").is_some());
}
