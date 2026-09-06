//! Turn a tester's session report into a captured manifest.
//!
//! ```text
//! cargo run -p report-intake --example intake -- <report.json> <library dir> [out dir]
//! ```
//!
//! The library directory is read to copy confirmed values exactly; the
//! manifest is written to `out dir` (default: the library directory, where
//! the application will load it with the rest) as `captured-<id>.json`. The
//! summary says what was confirmed, what was only observed, and what was
//! skipped and why. Nothing is written when nothing can be recorded.

use diagnostic_session::KnowledgeLibrary;
use report_intake::intake;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(report_path), Some(library_dir)) = (args.first(), args.get(1)) else {
        eprintln!("usage: intake <report.json> <library dir> [out dir]");
        std::process::exit(2);
    };
    let out_dir = args
        .get(2)
        .map(String::as_str)
        .unwrap_or(library_dir.as_str());
    let text = match std::fs::read_to_string(report_path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("cannot read {report_path}: {error}");
            std::process::exit(1);
        }
    };
    let file_name = Path::new(report_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("session-report.json");
    let library = KnowledgeLibrary::load_directory(Path::new(library_dir));
    let snapshot = library.snapshot();
    println!(
        "library: {:?}, {} manifests, {} records",
        snapshot.state, snapshot.manifests_loaded, snapshot.records
    );

    let outcome = match intake(&text, file_name, &library) {
        Ok(outcome) => outcome,
        Err(error) => {
            eprintln!("intake refused: {error}");
            std::process::exit(1);
        }
    };
    let manifest_name = format!("{}.json", outcome.batch.source.id.0);
    let target = Path::new(out_dir).join(&manifest_name);
    let json = serde_json::to_string_pretty(&outcome.batch).expect("batch serialises");
    if let Err(error) = std::fs::write(&target, json) {
        eprintln!("cannot write {}: {error}", target.display());
        std::process::exit(1);
    }

    println!(
        "written {} ({} records, {} evidence) as source {}",
        target.display(),
        outcome.batch.records.len(),
        outcome.batch.evidence.len(),
        outcome.batch.source.id.0
    );
    let summary = outcome.summary;
    println!(
        "\nconfirmed by a module's answer ({}):",
        summary.confirmations.len()
    );
    for line in &summary.confirmations {
        println!("  {line}");
    }
    println!(
        "\nobserved, confirming nothing ({}):",
        summary.observations.len()
    );
    for line in &summary.observations {
        println!("  {line}");
    }
    println!("\nskipped ({}):", summary.skipped.len());
    for line in &summary.skipped {
        println!("  {line}");
    }
}
