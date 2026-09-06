//! Load a directory of exported manifests and survey one vehicle, exactly as
//! the application does. Opens no transport; prints what the UI would show.
//!
//! Usage: `cargo run -p diagnostic-session --example survey -- <library dir> <programme> [model year] [breakpoint] [powertrain]`

use app_contracts::{RouteStatus, VehicleContextInput};
use diagnostic_session::KnowledgeLibrary;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(directory), Some(program)) = (args.first(), args.get(1)) else {
        eprintln!("usage: survey <library dir> <programme> [model year] [breakpoint] [powertrain]");
        std::process::exit(2);
    };
    let started = std::time::Instant::now();
    let library = KnowledgeLibrary::load_directory(Path::new(directory));
    let snapshot = library.snapshot();
    println!(
        "{} ({:?}, {:.1}s)",
        snapshot.message,
        snapshot.state,
        started.elapsed().as_secs_f32()
    );
    for failure in &snapshot.failures {
        println!("  ! {}: {}", failure.file, failure.message);
    }

    let input = VehicleContextInput {
        vehicle_program: program.clone(),
        model_year: args.get(2).and_then(|value| value.parse().ok()),
        year_breakpoint: args.get(3).cloned(),
        powertrain: args.get(4).cloned(),
        variant: None,
        market: None,
    };
    let survey = library.survey(&input);
    println!("\n{}", survey.message);
    for module in &survey.modules {
        let status = |summary: &app_contracts::RouteSummary| match summary.status {
            RouteStatus::Reachable => "reachable",
            RouteStatus::Hypothesis => "hypothesis",
            RouteStatus::Indeterminate => "indeterminate",
            RouteStatus::Conflict => "CONFLICT",
            RouteStatus::NotApplicable => "n/a",
        };
        println!(
            "  {:<10} bus={:<14} route={:<8} pins={:<6} rx/tx={}/{} ids={:<13} dtc={:<13} readable={}",
            module.ecu_family,
            module.logical_network.as_deref().unwrap_or("?"),
            module.backend_route.as_deref().unwrap_or("-"),
            module.pins.as_deref().unwrap_or("-"),
            module.request_id.as_deref().unwrap_or("?"),
            module.response_id.as_deref().unwrap_or("?"),
            status(&module.identifier_read),
            status(&module.dtc_read),
            module.readable_identifiers.len()
        );
        for reason in module
            .identifier_read
            .reasons
            .iter()
            .chain(module.dtc_read.reasons.iter())
            .collect::<std::collections::BTreeSet<_>>()
        {
            println!("             - {reason}");
        }
    }
}
