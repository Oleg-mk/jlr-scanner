//! Ingest the SDD platform files and report the addressing they declare.
//!
//! Runs across every `PLATFORM_*.xml` in a directory tree, not one program.
//! Writes nothing.
//!
//! Usage: `cargo run -p sdd-ingest --example ingest_platform -- <platform XML dir>`

use knowledge::{
    sha256_bytes, ClaimKey, ContentFingerprint, DimensionConstraint, KnowledgeQuery,
    KnowledgeStore, KnowledgeValue, RedistributionStatus, SourceId, SourceRecord, SourceType,
};
use sdd_ingest::PlatformAdapter;
use std::collections::{BTreeMap, BTreeSet};

fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(&path, out)?;
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("PLATFORM_") && name.ends_with(".xml"))
        {
            out.push(path);
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args()
        .nth(1)
        .ok_or("usage: ingest_platform <path to the SDD platform XML directory>")?;
    let mut paths = Vec::new();
    walk(std::path::Path::new(&root), &mut paths)?;
    paths.sort();

    let mut store = KnowledgeStore::new();
    let mut rejected: BTreeMap<String, usize> = BTreeMap::new();
    let mut accepted = 0usize;

    for path in &paths {
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown")
            .to_ascii_lowercase();
        let slug: String = stem
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let text = std::fs::read_to_string(path)?;
        let source = SourceRecord {
            id: SourceId::new(format!("sdd-169-{slug}"))?,
            title: format!("SDD {stem}"),
            source_type: SourceType::Documented,
            origin: "Jaguar Land Rover SDD 169.00.001".into(),
            source_locator: format!("CURRENT_JLR_XCL_XML_DATA_XML/Xml/{stem}.xml"),
            content_fingerprint: Some(ContentFingerprint::sha256(sha256_bytes(text.as_bytes()))?),
            acquired_on: Some("2026-09-02".into()),
            declared_vehicle_programs: vec![],
            provenance:
                "Platform documents of the official installer; see docs/research/sdd/PROVENANCE.md"
                    .into(),
            redistribution_status: RedistributionStatus::RestrictedMetadataOnly,
            notes: None,
        };
        let adapter = PlatformAdapter::new(source)?;
        match store.ingest(&adapter, &text) {
            Ok(_) => accepted += 1,
            Err(error) => *rejected.entry(error.to_string()).or_default() += 1,
        }
    }

    println!("platform files found   : {}", paths.len());
    println!("ingested               : {accepted}");
    if rejected.is_empty() {
        println!("rejected               : 0");
    } else {
        println!("rejected:");
        for (message, count) in rejected.iter().take(6) {
            println!("  {count:>4}  {message}");
        }
    }

    let all = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    let mut programs = BTreeSet::new();
    let mut modules = BTreeSet::new();
    let mut networks = BTreeSet::new();
    let mut addressing = 0usize;
    let mut rates: BTreeMap<u32, usize> = BTreeMap::new();

    for entry in &all.records {
        if let DimensionConstraint::OneOf { values } = &entry.record.applicability.vehicle_program {
            programs.extend(values.iter().cloned());
        }
        match (&entry.record.key, &entry.record.value) {
            (ClaimKey::DiagnosticAddressing, KnowledgeValue::DiagnosticAddressing { .. }) => {
                addressing += 1;
                modules.insert(entry.record.entity.id.clone());
            }
            (
                ClaimKey::NetworkRoute,
                KnowledgeValue::NetworkRoute {
                    logical_name,
                    bitrate_bps,
                    ..
                },
            ) => {
                networks.insert(logical_name.clone());
                if let Some(rate) = bitrate_bps {
                    *rates.entry(*rate).or_default() += 1;
                }
            }
            _ => {}
        }
    }

    println!("\nknowledge records      : {}", all.records.len());
    println!("module addressing claims: {addressing}");
    println!("distinct modules       : {}", modules.len());
    println!("distinct programs      : {}", programs.len());
    println!("distinct networks      : {:?}", networks);
    println!("bus rates seen         :");
    for (rate, count) in &rates {
        println!("  {count:>4} x {rate} bit/s");
    }
    println!("conflicts reported     : {}", all.conflicts.len());
    Ok(())
}
