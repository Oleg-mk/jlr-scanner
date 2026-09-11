//! Count the parameter names a library carries that the translation table
//! does not (`ADR-0025`).
//!
//! A parameter renamed by a future SDD release loses its translation and
//! falls back to English. That is the intended behaviour, but it is silent,
//! so it is counted here rather than hoped for: survey every programme the
//! catalogue names, collect every name the interface could show, and report
//! the ones with no row. The number belongs in `docs/CURRENT_STATE.md`.
//!
//! Usage: `cargo run -p diagnostic-session --example untranslated_names -- <library dir>`

use app_contracts::VehicleContextInput;
use diagnostic_session::{parameter_text, KnowledgeLibrary};
use std::collections::BTreeSet;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(directory) = args.first() else {
        eprintln!("usage: untranslated_names <library dir>");
        std::process::exit(2);
    };
    let library = KnowledgeLibrary::load_directory(Path::new(directory));
    println!("{}", library.snapshot().message);

    let mut names: BTreeSet<String> = BTreeSet::new();
    let mut surveys = 0usize;
    for programme in &library.catalogue().programmes {
        // One survey per declared breakpoint: a module's identifiers can be
        // qualified by year, so a single year would miss names.
        for marker in &programme.markers {
            let survey = library.survey(&VehicleContextInput {
                vehicle_program: programme.program.clone(),
                model_year: marker.model_year_from,
                year_breakpoint: Some(marker.marker.clone()),
                powertrain: None,
                variant: None,
                market: None,
            });
            surveys += 1;
            for module in &survey.modules {
                for identifier in &module.readable_identifiers {
                    names.extend(identifier.parameters.iter().cloned());
                }
            }
        }
    }

    let missing: Vec<&String> = names
        .iter()
        .filter(|name| parameter_text::parameter_texts(name).is_empty())
        .collect();

    println!("surveys run          : {surveys}");
    println!("distinct names shown : {}", names.len());
    println!("table covers         : {}", parameter_text::covered_names());
    println!("without a row        : {}", missing.len());
    for name in missing.iter().take(40) {
        println!("  {name}");
    }
    if missing.len() > 40 {
        println!("  … and {} more", missing.len() - 40);
    }
}
