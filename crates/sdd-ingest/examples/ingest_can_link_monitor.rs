//! Ingest a local SDD `CANLinkMonitorData.xml` and print what was derived.
//!
//! The SDD tree is never committed to this repository, so this example takes a
//! path to a locally extracted copy. It writes nothing; it only reports what the
//! adapter would place in the knowledge store, which is what makes an ingestion
//! run reviewable before it is trusted.
//!
//! Usage: `cargo run -p sdd-ingest --example ingest_can_link_monitor -- <path>`

use knowledge::{
    sha256_file, ApplicabilityResolution, ContentFingerprint, EntityKind, KnowledgeQuery,
    KnowledgeStore, KnowledgeValue, RedistributionStatus, SourceId, SourceRecord, SourceType,
    VehicleContext,
};
use sdd_ingest::{CanLinkMonitorAdapter, ModelYearTimeline};
use std::collections::BTreeMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: ingest_can_link_monitor <path to CANLinkMonitorData.xml>")?;
    let input = std::fs::read_to_string(&path)?;
    let fingerprint = ContentFingerprint::sha256(sha256_file(&path)?)?;

    let source = SourceRecord {
        id: SourceId::new("sdd-169-can-link-monitor")?,
        title: "SDD CANLinkMonitorData.xml".into(),
        source_type: SourceType::Documented,
        origin: "Jaguar Land Rover SDD 169.00.001".into(),
        source_locator: "CURRENT_PAG_UTILS_RUNTIME/CANLinkMonitorData.xml".into(),
        content_fingerprint: Some(fingerprint.clone()),
        acquired_on: Some("2026-09-02".into()),
        declared_vehicle_programs: vec![],
        provenance: "Extracted from the official installer; see docs/research/sdd/PROVENANCE.md"
            .into(),
        // SDD content is not redistributable; only derived claims are retained.
        redistribution_status: RedistributionStatus::RestrictedMetadataOnly,
        notes: None,
    };

    // Prose markers carry their own boundary, so an empty timeline is enough to
    // opt in to resolving them. See ADR-0011.
    let adapter = CanLinkMonitorAdapter::new(source)?.with_timeline(ModelYearTimeline::new());
    let mut store = KnowledgeStore::new();
    let receipt = store.ingest(&adapter, &input)?;

    println!(
        "parser       : {} {}",
        receipt.parser_id, receipt.parser_version
    );
    println!("source sha256: {}", fingerprint.value);
    println!("evidence     : {}", receipt.evidence_ids.len());
    println!("records      : {}", receipt.record_ids.len());

    let mut vehicles = 0usize;
    let mut modules = 0usize;
    for id in &receipt.record_ids {
        match store.get_record(id).map(|record| record.entity.kind) {
            Some(EntityKind::VehicleProgram) => vehicles += 1,
            Some(EntityKind::DiagnosticAddressing) => modules += 1,
            _ => {}
        }
    }
    println!("  vehicle addressing claims: {vehicles}");
    println!("  module alias claims      : {modules}");

    // Spot-check the project's own target vehicle against an independent source.
    let context = VehicleContext {
        vehicle_program: Some("X250".into()),
        model_year: Some(2010),
        architecture_generation: None,
        ecu_family: None,
        powertrain: None,
        variant: None,
        market: None,
        diagnostic_implementation: None,
        other: BTreeMap::new(),
    };
    let result = store.query(&KnowledgeQuery::for_vehicle(context));
    println!("\nX250 MY2010 resolved claims: {}", result.records.len());
    for entry in &result.records {
        if let KnowledgeValue::DiagnosticAddressing {
            can_id_format,
            addressing_mode,
            ..
        } = &entry.record.value
        {
            println!(
                "  {} -> {:?} / {:?}  [{:?}]",
                entry.record.id, can_id_format, addressing_mode, entry.applicability_resolution
            );
        }
    }
    println!("conflicts: {}", result.conflicts.len());

    // The prose markers separate a real engineering change: L319, L320 and L322
    // ran 29-bit addressing before MY10 and 11-bit from MY10.
    println!(
        "
prose-marker programs:"
    );
    for id in &receipt.record_ids {
        let Some(record) = store.get_record(id) else {
            continue;
        };
        if record.entity.kind != EntityKind::VehicleProgram {
            continue;
        }
        let KnowledgeValue::DiagnosticAddressing {
            can_id_format,
            addressing_mode,
            ..
        } = &record.value
        else {
            continue;
        };
        if !id.contains("my10") {
            continue;
        }
        println!(
            "  {:<10} {:<20} {:?}  years={:?}",
            record.entity.id,
            format!("{:?}", addressing_mode.as_deref().unwrap_or("")),
            can_id_format,
            record.applicability.model_year
        );
    }

    // The global module table must never resolve for a specific vehicle alone.
    let pcm = store.get_record("sdd-169-can-link-monitor.module.7E0");
    match pcm {
        Some(record) => {
            println!("\n7E0 alias: {:?}", record.value);
            println!(
                "7E0 resolution for X250: {:?}",
                record.applicability.resolve(&VehicleContext {
                    vehicle_program: Some("X250".into()),
                    model_year: Some(2010),
                    architecture_generation: None,
                    ecu_family: None,
                    powertrain: None,
                    variant: None,
                    market: None,
                    diagnostic_implementation: None,
                    other: BTreeMap::new(),
                })
            );
        }
        None => println!("\n7E0 alias not present"),
    }
    assert_ne!(
        pcm.map(|record| record.applicability.resolve(&VehicleContext::default())),
        Some(ApplicabilityResolution::Applicable),
        "a global module alias must not resolve as applicable on its own"
    );
    Ok(())
}
