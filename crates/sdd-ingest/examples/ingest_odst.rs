//! Ingest a local SDD `rds-odst-info` directory and report what it holds.
//!
//! On-demand self tests are stage-2 material. This example writes nothing and
//! asserts that every record it would store is classified as a service routine,
//! never as a read-only operation.
//!
//! Usage: `cargo run -p sdd-ingest --example ingest_odst -- <rds-odst-info dir>`

use knowledge::{
    sha256_bytes, ContentFingerprint, DiagnosticSafetyClass, DimensionConstraint, KnowledgeQuery,
    KnowledgeStore, KnowledgeValue, RedistributionStatus, SourceId, SourceRecord, SourceType,
};
use sdd_ingest::{OdstInfoAdapter, MODEL_YEAR_DESIGNATION_DIMENSION, QUAL_DIMENSION_PREFIX};
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
        .ok_or("usage: ingest_odst <path to rds-odst-info directory>")?;

    let mut store = KnowledgeStore::new();
    let mut files = 0usize;
    let mut rejected: BTreeMap<String, usize> = BTreeMap::new();

    let mut paths: Vec<_> = std::fs::read_dir(&root)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("xml"))
        .collect();
    paths.sort();

    for path in &paths {
        files += 1;
        let raw_stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown")
            .to_ascii_lowercase();
        let stem: String = raw_stem
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let text = std::fs::read_to_string(path)?;
        let source = SourceRecord {
            id: SourceId::new(format!("sdd-169-{stem}"))?,
            title: format!("SDD {raw_stem}"),
            source_type: SourceType::Documented,
            origin: "Jaguar Land Rover SDD 169.00.001".into(),
            source_locator: format!("COMMON_SDD_DATA_ODST_LANG_EN/rds-odst-info/{raw_stem}.xml"),
            content_fingerprint: Some(ContentFingerprint::sha256(sha256_bytes(text.as_bytes()))?),
            acquired_on: Some("2026-09-02".into()),
            declared_vehicle_programs: vec![],
            provenance:
                "Extracted from the official installer; see docs/research/sdd/PROVENANCE.md".into(),
            redistribution_status: RedistributionStatus::RestrictedMetadataOnly,
            notes: None,
        };
        let adapter = OdstInfoAdapter::new(source)?;
        if let Err(error) = store.ingest(&adapter, &text) {
            *rejected.entry(error.to_string()).or_default() += 1;
        }
    }

    println!("files scanned : {files}");
    if rejected.is_empty() {
        println!("files rejected: 0");
    } else {
        println!("files rejected:");
        for (message, count) in &rejected {
            println!("  {count:>4}  {message}");
        }
    }

    let result = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    let mut modules = BTreeSet::new();
    let mut models = BTreeSet::new();
    let mut tests = BTreeSet::new();
    let mut designations = BTreeSet::new();
    let mut quals = BTreeSet::new();
    let mut safety: BTreeMap<String, usize> = BTreeMap::new();

    for entry in &result.records {
        tests.insert(entry.record.entity.id.clone());
        if let Some(value) = only(&entry.record.applicability.ecu_family) {
            modules.insert(value.to_string());
        }
        if let Some(value) = only(&entry.record.applicability.vehicle_program) {
            models.insert(value.to_string());
        }
        for (name, constraint) in &entry.record.applicability.other {
            if name == MODEL_YEAR_DESIGNATION_DIMENSION {
                if let Some(value) = only(constraint) {
                    designations.insert(value.to_string());
                }
            } else if name.starts_with(QUAL_DIMENSION_PREFIX) {
                quals.insert(name.clone());
            }
        }
        if let KnowledgeValue::Capability { safety_class, .. } = &entry.record.value {
            *safety.entry(format!("{safety_class:?}")).or_default() += 1;
        }
    }

    println!("\nknowledge records        : {}", result.records.len());
    println!("distinct self tests      : {}", tests.len());
    println!("distinct modules         : {}", modules.len());
    println!("distinct vehicle programs: {}", models.len());
    println!("model-year designations  : {}", designations.len());
    println!("qualifier dimensions     : {:?}", quals);
    println!("conflicts reported       : {}", result.conflicts.len());
    println!("\nsafety classification:");
    for (class, count) in &safety {
        println!("  {count:>6}  {class}");
    }

    // The whole point of this slice: nothing here may look like stage-1 work.
    let read_only = result.records.iter().any(|entry| {
        matches!(
            &entry.record.value,
            KnowledgeValue::Capability {
                safety_class: Some(DiagnosticSafetyClass::ReadOnly),
                ..
            }
        )
    });
    assert!(
        !read_only,
        "an on-demand self test must never be recorded as READ_ONLY"
    );
    println!("\nno record is classified READ_ONLY: confirmed");
    Ok(())
}
