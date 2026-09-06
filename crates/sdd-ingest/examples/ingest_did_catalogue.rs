//! Ingest a local SDD `gradex` DID formatting catalogue and report what it holds.
//!
//! The SDD tree is never committed, so this takes a path to a locally extracted
//! `gradex` directory containing `Converters/` and `Snapshot/`. It writes
//! nothing; it reports what the adapter would place in the knowledge store.
//!
//! Usage: `cargo run -p sdd-ingest --example ingest_did_catalogue -- <gradex dir>`

use knowledge::{
    sha256_bytes, ClaimKey, ContentFingerprint, KnowledgeStore, KnowledgeValue,
    RedistributionStatus, SourceId, SourceRecord, SourceType,
};
use sdd_ingest::{ConverterCatalogue, DidFormattingAdapter};
use std::collections::BTreeMap;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args()
        .nth(1)
        .ok_or("usage: ingest_did_catalogue <path to gradex directory>")?;
    let root = Path::new(&root);

    let mut converters = ConverterCatalogue::new();
    let mut converter_failures = 0usize;
    for entry in std::fs::read_dir(root.join("Converters"))? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("xml") {
            continue;
        }
        let text = std::fs::read_to_string(&path)?;
        if converters.insert_from_xml(&text).is_err() {
            converter_failures += 1;
        }
    }
    println!("converters loaded: {}", converters.len());
    if converter_failures > 0 {
        println!("converters rejected: {converter_failures}");
    }

    let mut store = KnowledgeStore::new();
    let mut totals = BTreeMap::new();
    for entry in std::fs::read_dir(root.join("Snapshot"))? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("xml") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown")
            .to_string();
        let text = std::fs::read_to_string(&path)?;
        let slug: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();
        let source = SourceRecord {
            id: SourceId::new(format!("sdd-169-{}", slug.trim_matches('-')))?,
            title: format!("SDD {name}"),
            source_type: SourceType::Documented,
            origin: "Jaguar Land Rover SDD 169.00.001".into(),
            source_locator: format!("COMMON_SDD_DATA_SNAPSHOT_LANG_EN/gradex/Snapshot/{name}.xml"),
            content_fingerprint: Some(ContentFingerprint::sha256(sha256_bytes(text.as_bytes()))?),
            acquired_on: Some("2026-09-02".into()),
            declared_vehicle_programs: vec![],
            provenance:
                "Extracted from the official installer; see docs/research/sdd/PROVENANCE.md".into(),
            redistribution_status: RedistributionStatus::RestrictedMetadataOnly,
            notes: None,
        };
        let adapter = DidFormattingAdapter::new(source, converters.clone())?;
        match store.ingest(&adapter, &text) {
            Ok(receipt) => {
                totals.insert(name, receipt.record_ids.len());
            }
            Err(error) => {
                println!("  {name}: REJECTED -> {error}");
            }
        }
    }

    println!("\nparameters ingested per catalogue:");
    let mut grand = 0usize;
    for (name, count) in &totals {
        println!("  {count:>6}  {name}");
        grand += count;
    }
    println!("  {grand:>6}  TOTAL");

    let mut with_unit = 0usize;
    let mut units: BTreeMap<String, usize> = BTreeMap::new();
    let mut identifiers = std::collections::BTreeSet::new();
    let query = knowledge::KnowledgeQuery::default().include_indeterminate(true);
    for entry in store.query(&query).records {
        if let KnowledgeValue::IdentifierDefinition {
            identifier, unit, ..
        } = &entry.record.value
        {
            identifiers.insert(identifier.clone());
            if let Some(unit) = unit {
                with_unit += 1;
                *units.entry(unit.clone()).or_default() += 1;
            }
        }
    }
    println!("\ndistinct DIDs: {}", identifiers.len());
    println!("parameters carrying a unit: {with_unit}");
    println!("top units:");
    let mut ranked: Vec<_> = units.into_iter().collect();
    ranked.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for (unit, count) in ranked.into_iter().take(8) {
        println!("  {count:>6}  {unit}");
    }

    // Show one fully qualified record so the applicability mapping is visible.
    let qualified = store.query(&query).records.into_iter().find(|entry| {
        matches!(&entry.record.key, ClaimKey::ParameterDefinition { .. })
            && !matches!(
                entry.record.applicability.vehicle_program,
                knowledge::DimensionConstraint::Unknown
            )
    });
    if let Some(entry) = qualified {
        println!("\nexample qualified record: {}", entry.record.id);
        println!(
            "  program : {:?}",
            entry.record.applicability.vehicle_program
        );
        println!("  ecu     : {:?}", entry.record.applicability.ecu_family);
        println!("  year     : {:?}", entry.record.applicability.model_year);
        println!("  other   : {:?}", entry.record.applicability.other);
        println!("  value   : {:?}", entry.record.value);
    }
    Ok(())
}
