//! Survey every programme in a library and print fleet-wide coverage: how many
//! modules each programme is known to carry, how many the adapter can reach
//! today, and what blocks the rest. Loads the library once. Opens no transport.
//!
//! Usage: `cargo run -p diagnostic-session --example fleet_coverage -- <library dir> <PROGRAMME,YEAR,MARKER>...`

use app_contracts::{ModuleSurveyEntry, RouteStatus, VehicleContextInput};
use diagnostic_session::KnowledgeLibrary;
use std::collections::BTreeMap;
use std::path::Path;

/// The single fact that stops a module from being reachable, in priority
/// order so that a row is counted once. The order follows the pipeline: no
/// identifier, then no route, then no protocol or capability, then a vehicle
/// description the data cannot be applied to.
fn blocker(module: &ModuleSurveyEntry) -> &'static str {
    let reasons = || {
        module
            .identifier_read
            .reasons
            .iter()
            .chain(module.dtc_read.reasons.iter())
    };
    if reasons().any(|reason| reason.starts_with("request identifier:")) {
        "no identifiers"
    } else if reasons().any(|reason| reason.starts_with("adapter route:")) {
        "bus unbound"
    } else if reasons().any(|reason| {
        reason.starts_with("diagnostic protocol:") || reason.starts_with("read-only capability:")
    }) {
        "no protocol"
    } else if reasons()
        .any(|reason| reason.contains("lacks a detail") || reason.contains("does not state"))
    {
        "vehicle detail"
    } else {
        "other"
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(directory) = args.first() else {
        eprintln!("usage: fleet_coverage <library dir> <PROGRAMME,YEAR,MARKER>...");
        std::process::exit(2);
    };
    let library = KnowledgeLibrary::load_directory(Path::new(directory));
    println!("{}\n", library.snapshot().message);

    println!(
        "{:<8} {:<7} {:>4} {:>7} {:>9} {:>10} {:>11}  blockers",
        "prog", "marker", "year", "modules", "reachable", "hypothesis", "unreachable"
    );
    let mut fleet_modules = 0usize;
    let mut fleet_reachable = 0usize;
    let mut fleet_hypothesis = 0usize;
    let mut fleet_blockers: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut programmes_with_any = 0usize;

    for spec in &args[1..] {
        let parts: Vec<&str> = spec.split(',').collect();
        let (Some(program), Some(year), Some(marker)) = (parts.first(), parts.get(1), parts.get(2))
        else {
            eprintln!("skipping malformed spec {spec}");
            continue;
        };
        let survey = library.survey(&VehicleContextInput {
            vehicle_program: program.to_string(),
            model_year: year.parse().ok(),
            year_breakpoint: Some(marker.to_string()),
            powertrain: None,
            variant: None,
            market: None,
        });
        let mut blockers: BTreeMap<&'static str, usize> = BTreeMap::new();
        for module in &survey.modules {
            let reachable = matches!(
                module.identifier_read.status,
                RouteStatus::Reachable | RouteStatus::Hypothesis
            ) || matches!(
                module.dtc_read.status,
                RouteStatus::Reachable | RouteStatus::Hypothesis
            );
            if !reachable {
                *blockers.entry(blocker(module)).or_default() += 1;
                *fleet_blockers.entry(blocker(module)).or_default() += 1;
            }
        }
        let rendered: Vec<String> = blockers
            .iter()
            .map(|(name, count)| format!("{name}={count}"))
            .collect();
        println!(
            "{:<8} {:<7} {:>4} {:>7} {:>9} {:>10} {:>11}  {}",
            program,
            marker,
            year,
            survey.modules.len(),
            survey.reachable,
            survey.hypothesis,
            survey.unreachable,
            rendered.join(" ")
        );
        fleet_modules += survey.modules.len();
        fleet_reachable += survey.reachable as usize;
        fleet_hypothesis += survey.hypothesis as usize;
        if survey.reachable > 0 || survey.hypothesis > 0 {
            programmes_with_any += 1;
        }
    }

    println!(
        "
fleet: {} programme surveys, {programmes_with_any} with at least one reachable module; {fleet_modules} module rows, {fleet_reachable} reachable, {fleet_hypothesis} on a hypothesised route",
        args.len() - 1
    );
    println!("blockers across the fleet:");
    let mut rows: Vec<_> = fleet_blockers.into_iter().collect();
    rows.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for (name, count) in rows {
        println!("  {count:>5}  {name}");
    }
}
