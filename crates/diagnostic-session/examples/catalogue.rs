//! Print what a loaded library lets a tester choose from — programmes, their
//! model-year markers and engines — and show the fault-code wording join on a
//! few codes. Offline; nothing touches an adapter.
//!
//! ```text
//! cargo run --release -p diagnostic-session --example catalogue -- <library dir> [MODULE CODE]...
//! ```

use diagnostic_session::KnowledgeLibrary;
use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(directory) = args.next() else {
        eprintln!("usage: catalogue <library dir> [MODULE CODE]...");
        std::process::exit(2);
    };
    let library = KnowledgeLibrary::load_directory(Path::new(&directory));
    let snapshot = library.snapshot();
    println!(
        "library: {:?}, {} manifests, {} sources, {} records",
        snapshot.state, snapshot.manifests_loaded, snapshot.sources, snapshot.records
    );

    let catalogue = library.catalogue();
    println!("\n{} programmes:", catalogue.programmes.len());
    for programme in &catalogue.programmes {
        let markers: Vec<String> = programme
            .markers
            .iter()
            .map(
                |marker| match (marker.model_year_from, marker.model_year_to) {
                    (Some(from), Some(to)) if from != to => {
                        format!("{}={from}-{to}", marker.marker)
                    }
                    (Some(from), _) => format!("{}={from}", marker.marker),
                    _ => marker.marker.clone(),
                },
            )
            .collect();
        println!(
            "  {:<6} {} marker(s): {}",
            programme.program,
            programme.markers.len(),
            markers.join(", ")
        );
        if !programme.powertrains.is_empty() {
            println!(
                "         {} engine(s): {}",
                programme.powertrains.len(),
                programme.powertrains.join(", ")
            );
        }
        // Only where the data splits an engine further; most programmes
        // split nothing and print nothing.
        if !programme.variants.is_empty() {
            println!(
                "         {} variant(s): {}",
                programme.variants.len(),
                programme.variants.join(", ")
            );
        }
    }

    let mut pairs: Vec<String> = args.collect();
    // `--vin <VIN>...` after the module/code pairs decodes VINs with the
    // library's tables.
    if let Some(index) = pairs.iter().position(|arg| arg == "--vin") {
        let vins = pairs.split_off(index + 1);
        pairs.pop();
        println!(
            "
VIN decoding (tables: {}):",
            if library.has_vin_tables() {
                "loaded"
            } else {
                "none"
            }
        );
        for vin in vins {
            let decoded = library.decode_vin(&vin);
            println!("  {}: {}", decoded.vin, decoded.message);
            for attribute in &decoded.attributes {
                println!("      {:<13} {}", attribute.name, attribute.value);
            }
            if let Some(version) = &decoded.table_version {
                println!("      ({version})");
            }
        }
    }
    if !pairs.is_empty() {
        println!("\nfault-code wording:");
        for pair in pairs.chunks(2) {
            let (module, code) = (
                pair[0].as_str(),
                pair.get(1).map(String::as_str).unwrap_or(""),
            );
            let described = library.describe_dtc(code, 0, module);
            println!(
                "  {module} {code}: {} [{}]; failure type 00: {}",
                described.description.as_deref().unwrap_or("no wording"),
                described.description_scope.as_deref().unwrap_or("-"),
                described
                    .failure_type_text
                    .as_deref()
                    .unwrap_or("no wording"),
            );
        }
    }
}
