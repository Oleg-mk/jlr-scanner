//! Cut an exported library to named vehicle programmes, and if asked to
//! English only (`ADR-0040`, decision 4): for a copy that has to fit a small
//! machine, or a tester who sees one model.
//!
//! A record stays when it applies to any programme or to one of the named
//! ones; the evidence its records cite stays with it; a manifest left with
//! nothing goes. With `--english-only` the manifests of the Russian packs
//! (sources named `…-rus`) go too. Each bundle is read one manifest at a
//! time and written as it goes, so the tool needs little memory itself.
//! The cut copy is unstamped: stamp it like any other, with the scope in
//! the name it is issued to.
//!
//! Usage: `cut_library <library dir> <output dir> <programme>[,<programme>…] [--english-only]`

use diagnostic_session::manifest_stream::for_each_manifest;
use knowledge::{manifest_file, DimensionConstraint, IngestionBatch};
use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;

const USAGE: &str =
    "usage: cut_library <library dir> <output dir> <programme>[,<programme>...] [--english-only]";

/// Sources of SDD's Russian packs are named `<stem>-rus` by the exporter.
const RUSSIAN_SOURCE_SUFFIX: &str = "-rus";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let english_only = args.iter().any(|arg| arg == "--english-only");
    let positional: Vec<&String> = args.iter().filter(|arg| !arg.starts_with("--")).collect();
    let (Some(library), Some(out), Some(programmes)) =
        (positional.first(), positional.get(1), positional.get(2))
    else {
        eprintln!("{USAGE}");
        std::process::exit(2);
    };
    let wanted: BTreeSet<String> = programmes
        .split(',')
        .map(|programme| programme.trim().to_ascii_uppercase())
        .filter(|programme| !programme.is_empty())
        .collect();
    if wanted.is_empty() {
        eprintln!("{USAGE}");
        std::process::exit(2);
    }
    let library = Path::new(library);
    let out = Path::new(out);
    std::fs::create_dir_all(out)?;

    let mut files: Vec<_> = std::fs::read_dir(library)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    manifest_file::is_manifest(name) && name != diagnostic_session::ISSUE_STAMP_FILE
                })
        })
        .collect();
    files.sort();
    if files.is_empty() {
        return Err(format!("no manifests in {}", library.display()).into());
    }

    let (mut records_in, mut records_out, mut manifests_out) = (0usize, 0usize, 0usize);
    for path in &files {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("manifest")
            .to_string();
        let mut writer = BundleWriter::create(&out.join(manifest_file::compressed_name(&name)))?;
        let (mut file_in, mut file_out) = (0usize, 0usize);
        let mut failures: Vec<String> = Vec::new();
        for_each_manifest(manifest_file::open(path)?, |index, text| {
            let mut batch: IngestionBatch = match serde_json::from_str(text) {
                Ok(batch) => batch,
                Err(error) => {
                    failures.push(format!("{name}#{index}: {error}"));
                    return;
                }
            };
            file_in += batch.records.len();
            if english_only && batch.source.id.0.ends_with(RUSSIAN_SOURCE_SUFFIX) {
                return;
            }
            batch
                .records
                .retain(|record| applies(&record.applicability.vehicle_program, &wanted));
            if batch.records.is_empty() {
                return;
            }
            let cited: BTreeSet<&str> = batch
                .records
                .iter()
                .flat_map(|record| record.evidence_ids.iter().map(|id| id.0.as_str()))
                .collect();
            batch
                .evidence
                .retain(|evidence| cited.contains(evidence.id.0.as_str()));
            file_out += batch.records.len();
            if let Err(error) = writer.push(&batch) {
                failures.push(format!("{name}#{index}: {error}"));
            }
        })
        .map_err(|error| format!("{name}: {error}"))?;
        if let Some(first) = failures.first() {
            return Err(format!(
                "{} manifest(s) could not be cut; the first: {first}",
                failures.len()
            )
            .into());
        }
        let written = writer.finish()?;
        if written == 0 {
            std::fs::remove_file(out.join(manifest_file::compressed_name(&name)))?;
        }
        records_in += file_in;
        records_out += file_out;
        manifests_out += written;
        println!("{name:28} {file_in:7} -> {file_out:7} records in {written} manifests");
    }
    println!(
        "cut to {}{}: {records_in} -> {records_out} records in {manifests_out} manifests, written to {}",
        wanted.iter().cloned().collect::<Vec<_>>().join(", "),
        if english_only { ", English only" } else { "" },
        out.display()
    );
    Ok(())
}

/// Whether a record scoped by this constraint belongs in a copy for these
/// programmes: one that names none of them does not; every other does.
fn applies(constraint: &DimensionConstraint, wanted: &BTreeSet<String>) -> bool {
    match constraint {
        DimensionConstraint::OneOf { values } => values
            .iter()
            .any(|value| wanted.contains(&value.to_ascii_uppercase())),
        DimensionConstraint::Any | DimensionConstraint::Unknown => true,
    }
}

/// A bundle written packed as its manifests come, the way the exporter
/// writes one.
struct BundleWriter {
    file: flate2::write::GzEncoder<std::io::BufWriter<std::fs::File>>,
    manifests: usize,
}

impl BundleWriter {
    fn create(path: &Path) -> std::io::Result<Self> {
        let mut file = flate2::write::GzEncoder::new(
            std::io::BufWriter::new(std::fs::File::create(path)?),
            flate2::Compression::default(),
        );
        file.write_all(b"[")?;
        Ok(Self { file, manifests: 0 })
    }

    fn push(&mut self, batch: &IngestionBatch) -> Result<(), Box<dyn std::error::Error>> {
        if self.manifests > 0 {
            self.file.write_all(b",\n")?;
        }
        serde_json::to_writer(&mut self.file, batch)?;
        self.manifests += 1;
        Ok(())
    }

    fn finish(mut self) -> std::io::Result<usize> {
        self.file.write_all(b"]\n")?;
        self.file.finish()?.flush()?;
        Ok(self.manifests)
    }
}
