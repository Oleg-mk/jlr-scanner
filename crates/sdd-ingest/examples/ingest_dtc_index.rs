//! Ingest the SDD DTC index files and report what they hold.
//!
//! Covers `dtcDescriptions.xml`, `dtcModuleDescriptions.xml`, and
//! `dtcFaultTypes.xml`. Writes nothing.
//!
//! Usage: `cargo run -p sdd-ingest --example ingest_dtc_index -- <rds-dtc-help dir>`

use knowledge::{
    sha256_bytes, ClaimKey, ContentFingerprint, DimensionConstraint, IngestionAdapter,
    KnowledgeQuery, KnowledgeStore, RedistributionStatus, SourceId, SourceRecord, SourceType,
};
use sdd_ingest::{DtcDescriptionAdapter, DtcFaultTypeAdapter, FAILURE_TYPE_CLAIM};
use std::collections::BTreeSet;
use std::path::Path;

fn source(id: &str, locator: &str, text: &str) -> Result<SourceRecord, knowledge::KnowledgeError> {
    Ok(SourceRecord {
        id: SourceId::new(id)?,
        title: format!("SDD {locator}"),
        source_type: SourceType::Documented,
        origin: "Jaguar Land Rover SDD 169.00.001".into(),
        source_locator: format!("COMMON_SDD_DATA_DTC_HELP_LANG_EN/rds-dtc-help/{locator}"),
        content_fingerprint: Some(ContentFingerprint::sha256(sha256_bytes(text.as_bytes()))?),
        acquired_on: Some("2026-09-02".into()),
        declared_vehicle_programs: vec![],
        provenance: "Extracted from the official installer; see docs/research/sdd/PROVENANCE.md"
            .into(),
        redistribution_status: RedistributionStatus::RestrictedMetadataOnly,
        notes: None,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args()
        .nth(1)
        .ok_or("usage: ingest_dtc_index <path to rds-dtc-help directory>")?;
    let root = Path::new(&root);
    let mut store = KnowledgeStore::new();

    for (file, id) in [
        ("dtcDescriptions.xml", "sdd-169-dtc-descriptions"),
        (
            "dtcModuleDescriptions.xml",
            "sdd-169-dtc-module-descriptions",
        ),
    ] {
        let text = std::fs::read_to_string(root.join(file))?;
        let adapter = DtcDescriptionAdapter::new(source(id, file, &text)?)?;
        let receipt = store.ingest(&adapter, &text)?;
        println!("{file:<28} -> {} records", receipt.record_ids.len());
    }

    let file = "dtcFaultTypes.xml";
    let text = std::fs::read_to_string(root.join(file))?;
    let adapter = DtcFaultTypeAdapter::new(source("sdd-169-dtc-fault-types", file, &text)?)?;
    let receipt = store.ingest(&adapter, &text)?;
    println!("{file:<28} -> {} records", receipt.record_ids.len());
    println!(
        "parser: {} {}",
        adapter.parser_id(),
        adapter.parser_version()
    );

    let result = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    let mut codes = BTreeSet::new();
    let mut modules = BTreeSet::new();
    let mut fault_types = 0usize;
    let mut module_scoped = 0usize;
    let mut generic = 0usize;
    for entry in &result.records {
        if matches!(&entry.record.key, ClaimKey::Custom { name } if name == FAILURE_TYPE_CLAIM) {
            fault_types += 1;
            continue;
        }
        codes.insert(entry.record.entity.id.clone());
        match &entry.record.applicability.ecu_family {
            DimensionConstraint::OneOf { values } => {
                module_scoped += 1;
                modules.extend(values.iter().cloned());
            }
            _ => generic += 1,
        }
    }

    println!("\ntotal records          : {}", result.records.len());
    println!("distinct fault codes   : {}", codes.len());
    println!("module-scoped claims   : {module_scoped}");
    println!("generic claims         : {generic}");
    println!("distinct modules       : {}", modules.len());
    println!("failure type bytes     : {fault_types}");
    println!("conflicts reported     : {}", result.conflicts.len());
    Ok(())
}
