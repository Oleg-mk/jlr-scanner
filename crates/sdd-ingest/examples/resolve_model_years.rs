//! Two-pass ingestion that derives calendar model-year ranges from SDD markers.
//!
//! Pass one observes which markers each vehicle program declares; pass two
//! ingests with the resulting timeline. See `ADR-0011`. Writes nothing.
//!
//! Usage: `cargo run -p sdd-ingest --example resolve_model_years -- <gradex dir>`

use knowledge::{
    sha256_bytes, ApplicabilityResolution, ContentFingerprint, DimensionConstraint, KnowledgeQuery,
    KnowledgeStore, RedistributionStatus, SourceId, SourceRecord, SourceType, VehicleContext,
    YearConstraint,
};
use sdd_ingest::{ConverterCatalogue, DidFormattingAdapter, ModelYearTimeline};
use std::collections::BTreeMap;
use std::path::Path;

fn source(name: &str, text: &str) -> Result<SourceRecord, knowledge::KnowledgeError> {
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
    Ok(SourceRecord {
        id: SourceId::new(format!("sdd-169-{}", slug.trim_matches('-')))?,
        title: format!("SDD {name}"),
        source_type: SourceType::Documented,
        origin: "Jaguar Land Rover SDD 169.00.001".into(),
        source_locator: format!("COMMON_SDD_DATA_SNAPSHOT_LANG_EN/gradex/Snapshot/{name}.xml"),
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
        .ok_or("usage: resolve_model_years <path to gradex directory>")?;
    let root = Path::new(&root);

    let mut converters = ConverterCatalogue::new();
    for entry in std::fs::read_dir(root.join("Converters"))? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) == Some("xml") {
            let _ = converters.insert_from_xml(&std::fs::read_to_string(&path)?);
        }
    }

    let mut snapshots = Vec::new();
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
        snapshots.push((name, std::fs::read_to_string(&path)?));
    }
    snapshots.sort_by(|left, right| left.0.cmp(&right.0));

    // Pass one: ingest without a timeline and observe declared markers.
    let mut observed = KnowledgeStore::new();
    for (name, text) in &snapshots {
        let adapter = DidFormattingAdapter::new(source(name, text)?, converters.clone())?;
        let _ = observed.ingest(&adapter, text);
    }
    let mut timeline = ModelYearTimeline::new();
    for entry in observed
        .query(&KnowledgeQuery::default().include_indeterminate(true))
        .records
    {
        let program = match &entry.record.applicability.vehicle_program {
            DimensionConstraint::OneOf { values } if values.len() == 1 => values[0].clone(),
            _ => continue,
        };
        if let Some(DimensionConstraint::OneOf { values }) = entry
            .record
            .applicability
            .other
            .get(sdd_ingest::YEAR_BREAKPOINT_DIMENSION)
        {
            for marker in values {
                timeline.observe(&program, marker)?;
            }
        }
    }
    println!(
        "programs with a breakpoint sequence: {}",
        timeline.programs().count()
    );
    for program in ["X250", "L319", "X300"] {
        if let Some(points) = timeline.points(program) {
            let rendered: Vec<_> = points
                .map(|point| format!("{}.{:02}", point.year, point.fraction))
                .collect();
            println!("  {program:<6} {}", rendered.join(" "));
        }
    }

    // Pass two: ingest again, this time deriving ranges.
    let mut resolved = KnowledgeStore::new();
    for (name, text) in &snapshots {
        let adapter = DidFormattingAdapter::new(source(name, text)?, converters.clone())?
            .with_timeline(timeline.clone());
        let _ = resolved.ingest(&adapter, text);
    }

    let all = resolved.query(&KnowledgeQuery::default().include_indeterminate(true));
    let mut with_range = 0usize;
    let mut without = 0usize;
    for entry in &all.records {
        match entry.record.applicability.model_year {
            YearConstraint::Range { .. } => with_range += 1,
            _ => without += 1,
        }
    }
    println!("\nrecords: {}", all.records.len());
    println!("  with a derived year range: {with_range}");
    println!("  still unresolved         : {without}");

    // The point of the exercise: a real vehicle question now gets an answer.
    // Fully qualified records also constrain powertrain, so a context missing it
    // correctly resolves as lacking context rather than as applicable.
    let x250 = |powertrain: Option<&str>| VehicleContext {
        vehicle_program: Some("X250".into()),
        model_year: Some(2010),
        ecu_family: Some("PCM".into()),
        powertrain: powertrain.map(str::to_string),
        other: BTreeMap::from([("sdd_year_breakpoint".to_string(), "MY10".to_string())]),
        ..VehicleContext::default()
    };

    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut sample = None;
    for entry in resolved
        .query(&KnowledgeQuery::for_vehicle(x250(None)).include_indeterminate(true))
        .records
    {
        *kinds
            .entry(format!("{:?}", entry.applicability_resolution))
            .or_default() += 1;
        // Only a record explicitly scoped to X250 illustrates the derivation;
        // unqualified records match any context and would be misleading here.
        if sample.is_none()
            && matches!(&entry.record.applicability.vehicle_program,
                DimensionConstraint::OneOf { values } if values == &["X250".to_string()])
        {
            sample = Some(entry.record.clone());
        }
    }
    println!(
        "
X250 MY2010 PCM, powertrain not supplied:"
    );
    for (kind, count) in &kinds {
        println!("  {count:>5}  {kind}");
    }
    if let Some(record) = &sample {
        println!(
            "  example years={:?} powertrain={:?}",
            record.applicability.model_year, record.applicability.powertrain
        );
    }

    // Supplying the powertrain the source demands completes the answer.
    if let Some(record) = &sample {
        if let DimensionConstraint::OneOf { values } = &record.applicability.powertrain {
            let applicable = resolved
                .query(&KnowledgeQuery::for_vehicle(x250(Some(&values[0]))))
                .records
                .iter()
                .filter(|entry| {
                    entry.applicability_resolution == ApplicabilityResolution::Applicable
                })
                .count();
            println!(
                "
with powertrain={}: {applicable} applicable parameter definitions",
                values[0]
            );
        }
    }
    Ok(())
}
