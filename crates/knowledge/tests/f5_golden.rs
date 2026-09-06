use knowledge::{
    ApplicabilityResolution, EntityKind, JsonManifestAdapter, KnowledgeQuery, KnowledgeStore,
    SourceType, ValidationState, VehicleContext,
};
use std::collections::BTreeMap;

const SYNTHETIC: &str = include_str!("../../../fixtures/knowledge/synthetic/f5_architecture.json");
const REAL_DOCUMENTED: &str =
    include_str!("../../../fixtures/knowledge/documented/x250_ccp_route.json");

#[test]
fn synthetic_source_to_query_is_deterministic_and_never_real() {
    let mut store = KnowledgeStore::new();
    let receipt = store.ingest(&JsonManifestAdapter, SYNTHETIC).unwrap();
    assert_eq!(
        receipt.record_ids,
        vec!["record-synthetic-implementation-a"]
    );

    let context = VehicleContext {
        vehicle_program: Some("PROGRAM-B".into()),
        model_year: Some(2012),
        architecture_generation: Some("ARCH-A".into()),
        ecu_family: Some("ECU-FAMILY-A".into()),
        powertrain: None,
        variant: Some("VARIANT-A".into()),
        market: None,
        diagnostic_implementation: Some("IMPL-A".into()),
        other: BTreeMap::new(),
    };
    let result = store.query(
        &KnowledgeQuery::for_vehicle(context)
            .with_ecu_family("ECU-FAMILY-A")
            .with_diagnostic_implementation("IMPL-A"),
    );
    assert_eq!(result.records.len(), 1);
    assert_eq!(
        result.records[0].applicability_resolution,
        ApplicabilityResolution::Applicable
    );
    assert_eq!(
        result.records[0].record.validation_state,
        ValidationState::Unverified
    );
    assert_eq!(
        result.records[0].evidence[0].source.source_type,
        SourceType::Synthetic
    );
}

#[test]
fn real_jlr_documented_source_has_exact_end_to_end_trace_back() {
    let mut store = KnowledgeStore::new();
    let receipt = store.ingest(&JsonManifestAdapter, REAL_DOCUMENTED).unwrap();
    assert_eq!(receipt.source_id.0, "jlr-x250-ewd-2008-13-56-10-1e");
    assert_eq!(
        receipt.record_ids,
        vec!["record-x250-diagnostic-connector-ccp-route"]
    );

    let context = VehicleContext {
        vehicle_program: Some("X250".into()),
        ..VehicleContext::default()
    };
    let result = store.query(&KnowledgeQuery::for_vehicle(context).include_indeterminate(true));
    assert_eq!(result.records.len(), 1);
    let resolved = &result.records[0];
    assert_eq!(resolved.record.entity.kind, EntityKind::NetworkRoute);
    assert_eq!(
        resolved.applicability_resolution,
        ApplicabilityResolution::InsufficientEvidence
    );
    assert_eq!(
        resolved.record.validation_state,
        ValidationState::SourceBacked
    );

    let trace = store
        .trace_back("record-x250-diagnostic-connector-ccp-route")
        .unwrap();
    assert_eq!(trace.len(), 1);
    assert_eq!(trace[0].source.source_type, SourceType::Documented);
    assert_eq!(
        trace[0].source.content_fingerprint.as_ref().unwrap().value,
        "406e07bfc3a8afc2caaa7384795210d911d44b3e23ca53ed7aab663c2782bee3"
    );
    assert_eq!(trace[0].evidence.locator.document_page, Some(181));
    assert_eq!(
        trace[0].evidence.locator.record_key.as_deref(),
        Some("C2DB04B/12,C2DB04B/13")
    );
}
