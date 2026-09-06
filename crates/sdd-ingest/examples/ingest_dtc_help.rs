//! Ingest a local SDD `rds-dtc-help` directory and report what it holds.
//!
//! The SDD tree is never committed, so this takes a path to a locally extracted
//! directory. It writes nothing; it reports what the adapters would store.
//!
//! Usage: `cargo run -p sdd-ingest --example ingest_dtc_help -- <rds-dtc-help dir>`

use knowledge::{
    sha256_bytes, ContentFingerprint, DimensionConstraint, KnowledgeQuery, KnowledgeStore,
    KnowledgeValue, RedistributionStatus, SourceId, SourceRecord, SourceType,
};
use sdd_ingest::{DtcHelpAdapter, MODEL_YEAR_DESIGNATION_DIMENSION};
use std::collections::{BTreeMap, BTreeSet};

fn only(constraint: &DimensionConstraint) -> Option<&str> {
    match constraint {
        DimensionConstraint::OneOf { values } if values.len() == 1 => Some(values[0].as_str()),
        _ => None,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args()
        .nth(1)
        .ok_or("usage: ingest_dtc_help <path to rds-dtc-help directory>")?;

    let mut store = KnowledgeStore::new();
    let mut files = 0usize;
    let mut rejected: BTreeMap<String, usize> = BTreeMap::new();
    let mut empty = 0usize;

    let mut paths: Vec<_> = std::fs::read_dir(&root)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("rdsDtcHelp") && name.ends_with(".xml"))
        })
        .collect();
    paths.sort();

    for path in &paths {
        files += 1;
        let raw_stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown")
            .to_ascii_lowercase();
        // Source ids are this example's own construction, so an unusual
        // filename character is slugged here rather than rejected. A fault code
        // that cannot form a stable entity id is still rejected by the adapter.
        let stem: String = raw_stem
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let text = std::fs::read_to_string(path)?;
        let source = SourceRecord {
            id: SourceId::new(format!("sdd-169-{stem}"))?,
            title: format!("SDD {stem}"),
            source_type: SourceType::Documented,
            origin: "Jaguar Land Rover SDD 169.00.001".into(),
            source_locator: format!("COMMON_SDD_DATA_DTC_HELP_LANG_EN/rds-dtc-help/{raw_stem}.xml"),
            content_fingerprint: Some(ContentFingerprint::sha256(sha256_bytes(text.as_bytes()))?),
            acquired_on: Some("2026-09-02".into()),
            declared_vehicle_programs: vec![],
            provenance:
                "Extracted from the official installer; see docs/research/sdd/PROVENANCE.md".into(),
            redistribution_status: RedistributionStatus::RestrictedMetadataOnly,
            notes: None,
        };
        let adapter = DtcHelpAdapter::new(source)?;
        match store.ingest(&adapter, &text) {
            Ok(_) => {}
            Err(error) => {
                let message = error.to_string();
                if message.contains("no qualified descriptions") {
                    empty += 1;
                } else {
                    *rejected.entry(message).or_default() += 1;
                }
            }
        }
    }

    println!("files scanned            : {files}");
    println!("files with no qualified description: {empty}");
    if rejected.is_empty() {
        println!("files rejected           : 0");
    } else {
        println!("files rejected           :");
        for (message, count) in &rejected {
            println!("  {count:>5}  {message}");
        }
    }

    let result = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    let mut codes = BTreeSet::new();
    let mut modules = BTreeSet::new();
    let mut models = BTreeSet::new();
    let mut designations = BTreeSet::new();
    let mut described = 0usize;
    for entry in &result.records {
        codes.insert(entry.record.entity.id.clone());
        if let Some(value) = only(&entry.record.applicability.ecu_family) {
            modules.insert(value.to_string());
        }
        if let Some(value) = only(&entry.record.applicability.vehicle_program) {
            models.insert(value.to_string());
        }
        if let Some(constraint) = entry
            .record
            .applicability
            .other
            .get(MODEL_YEAR_DESIGNATION_DIMENSION)
        {
            if let Some(value) = only(constraint) {
                designations.insert(value.to_string());
            }
        }
        if matches!(&entry.record.value, KnowledgeValue::Text { .. }) {
            described += 1;
        }
    }

    println!("\nknowledge records        : {}", result.records.len());
    println!("described claims         : {described}");
    println!("distinct fault codes     : {}", codes.len());
    println!("distinct modules         : {}", modules.len());
    println!("distinct vehicle programs: {}", models.len());
    println!("model-year designations  : {}", designations.len());
    println!("conflicts reported       : {}", result.conflicts.len());
    println!(
        "\ndesignations: {}",
        designations.into_iter().collect::<Vec<_>>().join(" ")
    );
    println!(
        "programs: {}",
        models.into_iter().collect::<Vec<_>>().join(" ")
    );
    Ok(())
}
