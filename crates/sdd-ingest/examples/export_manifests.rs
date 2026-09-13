//! Export the SDD-derived knowledge as F5 manifest bundles for the application.
//!
//! The SDD tree is never committed and never a runtime dependency; what the
//! application loads is the knowledge base itself, as JSON manifests. This
//! example walks one or more locally extracted SDD roots, ingests every file
//! the F9 adapters understand — validating the whole set as one store, exactly
//! as the application will — and writes one JSON bundle per adapter kind into
//! the output directory. A bundle is a JSON array of manifests; the
//! application accepts single manifests and bundles alike.
//!
//! Model-year ranges are derived the way `resolve_model_years` does it: pass
//! one observes every breakpoint marker the platform and DID documents
//! declare, pass two ingests with the resulting timeline (ADR-0011).
//!
//! Usage: `cargo run -p sdd-ingest --example export_manifests -- <out dir> <root>...`

use knowledge::{
    sha256_bytes, ContentFingerprint, DimensionConstraint, IngestionAdapter, IngestionBatch,
    JsonManifestAdapter, KnowledgeError, KnowledgeStore, RedistributionStatus, SourceId,
    SourceRecord, SourceType,
};
use sdd_ingest::{
    is_text_item_id, BatteryFormatting, CanLinkMonitorAdapter, CcfAdapter, ConverterCatalogue,
    DidFormattingAdapter, DtcDescriptionAdapter, DtcFaultTypeAdapter, DtcHelpAdapter,
    IvsLineageAdapter, ModelYearTimeline, ModuleTextAdapter, OdstInfoAdapter, PlatformAdapter,
    TextLookup, VinDecodeAdapter, DTC_HELP_LANGUAGE_RUSSIAN, YEAR_BREAKPOINT_DIMENSION,
};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Default)]
struct Corpus {
    platforms: Vec<Found>,
    converters: Vec<Found>,
    snapshots: Vec<Found>,
    dtc_help: Vec<Found>,
    /// The same help documents from the Russian pack (`ADR-0034`): text
    /// only, the selections come from the English ones.
    dtc_help_rus: Vec<Found>,
    dtc_index: Vec<Found>,
    odst: Vec<Found>,
    /// JLR's IVS part lineage: which software belongs to which assembly
    /// (ADR-0033).
    ivs: Vec<Found>,
    link_monitor: Vec<Found>,
    vin: Vec<Found>,
    module_text: Vec<Found>,
    /// The car configuration descriptions (ADR-0028).
    ccf: Vec<Found>,
    /// Every item of SDD's text database, for the configuration's titles
    /// and options.
    texts: Vec<Found>,
}

/// A corpus file and where it sits relative to the root it was found under,
/// which is what the source locator records.
struct Found {
    path: PathBuf,
    relative: String,
}

/// The language a corpus root is in, from its name: SDD ships one folder per
/// language, `COMMON_SDD_DATA_DTC_HELP_LANG_EN`, `..._LANG_RU`. English and
/// a root that names no language are read as English; Russian is read for
/// the help text alone (`ADR-0034`); any other language is left where it is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RootLanguage {
    English,
    Russian,
    Other,
}

fn root_language(root: &Path) -> RootLanguage {
    let name = root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    match name.rsplit_once("_LANG_") {
        None => RootLanguage::English,
        Some((_, "EN")) => RootLanguage::English,
        Some((_, "RU")) => RootLanguage::Russian,
        Some(_) => RootLanguage::Other,
    }
}

fn walk(root: &Path, dir: &Path, corpus: &mut Corpus) -> std::io::Result<()> {
    let language = root_language(root);
    if language == RootLanguage::Other {
        return Ok(());
    }
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            walk(root, &path, corpus)?;
            continue;
        }
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_string();
        let parent = path
            .parent()
            .and_then(|value| value.file_name())
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_string();
        let is_xml = name.to_ascii_lowercase().ends_with(".xml");
        if !is_xml {
            continue;
        }
        let relative = format!(
            "{}/{}",
            root.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("root"),
            path.strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/")
        );
        let found = Found { path, relative };
        // A Russian root has the same file names as the English one; only
        // the per-code help documents are taken from it, and the index
        // files beside them are left, because the English ones carry the
        // descriptions and the selections.
        if language == RootLanguage::Russian {
            if name.starts_with("rdsDtcHelp") {
                corpus.dtc_help_rus.push(found);
            }
            continue;
        }
        if name.starts_with("PLATFORM_") {
            corpus.platforms.push(found);
        } else if parent == "Converters" {
            corpus.converters.push(found);
        } else if parent == "Snapshot" {
            corpus.snapshots.push(found);
        } else if name.starts_with("rdsDtcHelp") {
            corpus.dtc_help.push(found);
        } else if matches!(
            name.as_str(),
            "dtcDescriptions.xml" | "dtcModuleDescriptions.xml" | "dtcFaultTypes.xml"
        ) {
            corpus.dtc_index.push(found);
        } else if parent == "rds-odst-info" {
            corpus.odst.push(found);
        } else if parent == "IVS" {
            corpus.ivs.push(found);
        } else if name == "CANLinkMonitorData.xml" {
            corpus.link_monitor.push(found);
        } else if name == "VINDecode.xml" {
            corpus.vin.push(found);
        } else if name.starts_with("CCF_DATA_") {
            corpus.ccf.push(found);
        } else if is_text_item_id(name.trim_end_matches(".xml")) {
            corpus.module_text.push(found);
        } else if name.starts_with('@') && parent.starts_with('@') {
            corpus.texts.push(found);
        }
    }
    Ok(())
}

fn slug(text: &str) -> String {
    let slug: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    slug.trim_matches('-').to_string()
}

/// The same source identity the F9 examples construct, so records exported
/// here carry the ids and locators the F9 documents describe.
fn source(found: &Found, text: &str) -> Result<SourceRecord, KnowledgeError> {
    source_in(found, text, None)
}

/// The same, for a file from a pack in another language: the id and the
/// title name the language, so the Russian `rdsDtcHelp0x0000.xml` is a
/// source of its own beside the English one and their records cannot
/// collide.
fn source_in(
    found: &Found,
    text: &str,
    language: Option<&str>,
) -> Result<SourceRecord, KnowledgeError> {
    let stem = found
        .path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown");
    let (id, title) = match language {
        Some(code) => (
            format!("sdd-169-{}-{code}", slug(stem)),
            format!("SDD {stem} ({code})"),
        ),
        None => (format!("sdd-169-{}", slug(stem)), format!("SDD {stem}")),
    };
    Ok(SourceRecord {
        id: SourceId::new(id)?,
        title,
        source_type: SourceType::Documented,
        origin: "Jaguar Land Rover SDD 169.00.001".into(),
        source_locator: found.relative.clone(),
        content_fingerprint: Some(ContentFingerprint::sha256(sha256_bytes(text.as_bytes()))?),
        acquired_on: Some("2026-09-02".into()),
        declared_vehicle_programs: vec![],
        provenance: "Extracted from the official installer; see docs/research/sdd/PROVENANCE.md"
            .into(),
        // SDD content is not redistributable; only derived claims are retained.
        redistribution_status: RedistributionStatus::RestrictedMetadataOnly,
        notes: None,
    })
}

/// The manifests the application carries built in, in the order it loads
/// them; the exporter's store starts with the same so that what is exported
/// validates the way it will load.
const BUILT_IN_MANIFESTS: [(&str, &str); 6] = [
    (
        "mongoose_jlr_route_bindings.json",
        include_str!("../../../fixtures/knowledge/documented/mongoose_jlr_route_bindings.json"),
    ),
    (
        "x250_ccp_route.json",
        include_str!("../../../fixtures/knowledge/documented/x250_ccp_route.json"),
    ),
    (
        "iso15765_normal_fixed_addressing.json",
        include_str!(
            "../../../fixtures/knowledge/documented/iso15765_normal_fixed_addressing.json"
        ),
    ),
    (
        "mongoose_jlr_relayed_route_hypotheses.json",
        include_str!(
            "../../../fixtures/knowledge/research/mongoose_jlr_relayed_route_hypotheses.json"
        ),
    ),
    (
        "mongoose_jlr_kline_route_hypotheses.json",
        include_str!(
            "../../../fixtures/knowledge/research/mongoose_jlr_kline_route_hypotheses.json"
        ),
    ),
    (
        "normal_fixed_tester_address_hypothesis.json",
        include_str!(
            "../../../fixtures/knowledge/research/normal_fixed_tester_address_hypothesis.json"
        ),
    ),
];

struct Bundle {
    file: flate2::write::GzEncoder<std::io::BufWriter<std::fs::File>>,
    name: String,
    manifests: usize,
    records: usize,
}

impl Bundle {
    /// Written packed (ADR-0023): the same records, a twentieth of the bytes.
    fn create(out: &Path, name: &str) -> std::io::Result<Self> {
        let name = knowledge::manifest_file::compressed_name(name);
        let mut file = flate2::write::GzEncoder::new(
            std::io::BufWriter::new(std::fs::File::create(out.join(&name))?),
            flate2::Compression::default(),
        );
        file.write_all(b"[")?;
        Ok(Self {
            file,
            name,
            manifests: 0,
            records: 0,
        })
    }

    fn push(&mut self, batch: &IngestionBatch) -> Result<(), Box<dyn std::error::Error>> {
        if self.manifests > 0 {
            self.file.write_all(b",\n")?;
        }
        serde_json::to_writer(&mut self.file, batch)?;
        self.manifests += 1;
        self.records += batch.records.len();
        Ok(())
    }

    fn finish(mut self) -> std::io::Result<(String, usize, usize)> {
        self.file.write_all(b"]\n")?;
        self.file.finish()?.flush()?;
        Ok((self.name, self.manifests, self.records))
    }
}

#[derive(Default)]
struct Rejections {
    by_reason: BTreeMap<String, usize>,
}

impl Rejections {
    fn note(&mut self, kind: &str, error: impl std::fmt::Display) {
        *self
            .by_reason
            .entry(format!("{kind}: {error}"))
            .or_default() += 1;
    }
}

/// Parse with the adapter, validate against the shared store as the
/// application will, and only then write the batch into the bundle.
fn export(
    store: &mut KnowledgeStore,
    bundle: &mut Bundle,
    adapter: &impl IngestionAdapter,
    text: &str,
    kind: &str,
    rejections: &mut Rejections,
) -> Result<(), Box<dyn std::error::Error>> {
    let batch = match adapter.parse(text) {
        Ok(batch) => batch,
        Err(error) => {
            rejections.note(kind, error);
            return Ok(());
        }
    };
    // A batch with nothing in it — a Russian help document whose screens
    // no car selects (`ADR-0034`) — is not a rejection and not a manifest.
    if batch.records.is_empty() {
        return Ok(());
    }
    match store.ingest_batch(adapter.parser_id(), adapter.parser_version(), batch.clone()) {
        Ok(_) => bundle.push(&batch),
        Err(error) => {
            rejections.note(kind, error);
            Ok(())
        }
    }
}

fn read(found: &Found) -> std::io::Result<String> {
    std::fs::read_to_string(&found.path)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(out), roots) = (args.first(), &args[1.min(args.len())..]) else {
        return Err("usage: export_manifests <out dir> <root>...".into());
    };
    if roots.is_empty() {
        return Err("usage: export_manifests <out dir> <root>...".into());
    }
    let out = Path::new(out);
    std::fs::create_dir_all(out)?;

    let mut corpus = Corpus::default();
    for root in roots {
        let root = Path::new(root);
        walk(root, root, &mut corpus)?;
    }
    println!(
        "found: {} platform, {} converter, {} DID snapshot, {} DTC help, {} DTC help (rus), {} DTC index, {} ODST, {} IVS, {} link monitor, {} VIN decode, {} module text, {} CCF, {} other text items",
        corpus.platforms.len(),
        corpus.converters.len(),
        corpus.snapshots.len(),
        corpus.dtc_help.len(),
        corpus.dtc_help_rus.len(),
        corpus.dtc_index.len(),
        corpus.odst.len(),
        corpus.ivs.len(),
        corpus.link_monitor.len(),
        corpus.vin.len(),
        corpus.module_text.len(),
        corpus.ccf.len(),
        corpus.texts.len()
    );

    let mut converters = ConverterCatalogue::new();
    for found in &corpus.converters {
        let _ = converters.insert_from_xml(&read(found)?);
    }

    // Pass one: observe every breakpoint marker the platform and DID
    // documents declare, without deriving anything yet.
    let mut timeline = ModelYearTimeline::new();
    let mut observe = |batch: &IngestionBatch| -> Result<(), KnowledgeError> {
        for record in &batch.records {
            let program = match &record.applicability.vehicle_program {
                DimensionConstraint::OneOf { values } if values.len() == 1 => &values[0],
                _ => continue,
            };
            if let Some(DimensionConstraint::OneOf { values }) =
                record.applicability.other.get(YEAR_BREAKPOINT_DIMENSION)
            {
                for marker in values {
                    timeline.observe(program, marker)?;
                }
            }
        }
        Ok(())
    };
    for found in &corpus.platforms {
        let text = read(found)?;
        if let Ok(batch) = PlatformAdapter::new(source(found, &text)?)?.parse(&text) {
            observe(&batch)?;
        }
    }
    // The same pass collects how SDD's DID formatting document describes the
    // bytes of the battery identifiers (ADR-0030). That document names no
    // module, so its rows reach nothing on their own; joined to the platform
    // document, which names the module, they make a readable parameter.
    let mut battery_formatting = BatteryFormatting::new();
    for found in &corpus.snapshots {
        let text = read(found)?;
        if let Ok(batch) =
            DidFormattingAdapter::new(source(found, &text)?, converters.clone())?.parse(&text)
        {
            for record in &batch.records {
                let (
                    knowledge::ClaimKey::ParameterDefinition { parameter },
                    knowledge::KnowledgeValue::IdentifierDefinition {
                        identifier,
                        encoding,
                        unit,
                    },
                ) = (&record.key, &record.value)
                else {
                    continue;
                };
                let Some(number) = identifier
                    .strip_prefix("0x")
                    .or_else(|| identifier.strip_prefix("0X"))
                    .and_then(|digits| u16::from_str_radix(digits, 16).ok())
                else {
                    continue;
                };
                battery_formatting.insert(number, parameter, encoding.as_deref(), unit.as_deref());
            }
            observe(&batch)?;
        }
    }
    let battery_formatting = std::sync::Arc::new(battery_formatting);
    println!(
        "battery: {} identifiers described byte by byte",
        battery_formatting.identifiers()
    );
    println!(
        "timeline: {} programmes with a breakpoint sequence",
        timeline.programs().count()
    );

    // Pass two: ingest everything into one store, deriving year ranges, and
    // write what the store accepted. The built-in manifests go in first: the
    // derived normal-fixed identifiers (ADR-0017) cite their evidence.
    let mut store = KnowledgeStore::new();
    for (name, text) in BUILT_IN_MANIFESTS {
        store
            .ingest(&JsonManifestAdapter, text)
            .map_err(|error| format!("built-in manifest {name}: {error}"))?;
    }
    let mut rejections = Rejections::default();
    let mut summary = Vec::new();

    let mut bundle = Bundle::create(out, "platform.json")?;
    for found in &corpus.platforms {
        let text = read(found)?;
        let adapter = PlatformAdapter::new(source(found, &text)?)?
            .with_timeline(timeline.clone())
            .with_derived_normal_fixed_identifiers()
            .with_battery_formatting(battery_formatting.clone());
        export(
            &mut store,
            &mut bundle,
            &adapter,
            &text,
            "platform",
            &mut rejections,
        )?;
    }
    summary.push(bundle.finish()?);

    let mut bundle = Bundle::create(out, "did_catalogue.json")?;
    for found in &corpus.snapshots {
        let text = read(found)?;
        let adapter = DidFormattingAdapter::new(source(found, &text)?, converters.clone())?
            .with_timeline(timeline.clone());
        export(
            &mut store,
            &mut bundle,
            &adapter,
            &text,
            "did",
            &mut rejections,
        )?;
    }
    summary.push(bundle.finish()?);

    let mut bundle = Bundle::create(out, "dtc_index.json")?;
    for found in &corpus.dtc_index {
        let text = read(found)?;
        let name = found
            .path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if name == "dtcFaultTypes.xml" {
            let adapter = DtcFaultTypeAdapter::new(source(found, &text)?)?;
            export(
                &mut store,
                &mut bundle,
                &adapter,
                &text,
                "dtc-index",
                &mut rejections,
            )?;
        } else {
            let adapter = DtcDescriptionAdapter::new(source(found, &text)?)?;
            export(
                &mut store,
                &mut bundle,
                &adapter,
                &text,
                "dtc-index",
                &mut rejections,
            )?;
        }
    }
    summary.push(bundle.finish()?);

    let mut bundle = Bundle::create(out, "dtc_help.json")?;
    for found in &corpus.dtc_help {
        let text = read(found)?;
        let adapter = DtcHelpAdapter::new(source(found, &text)?)?.with_timeline(timeline.clone());
        export(
            &mut store,
            &mut bundle,
            &adapter,
            &text,
            "dtc-help",
            &mut rejections,
        )?;
    }
    // The same screens in Russian, from JLR's own pack (`ADR-0034`): the
    // text of each screen a car can be given, written beside the English.
    for found in &corpus.dtc_help_rus {
        let text = read(found)?;
        let adapter =
            DtcHelpAdapter::new(source_in(found, &text, Some(DTC_HELP_LANGUAGE_RUSSIAN))?)?
                .with_timeline(timeline.clone())
                .with_language(DTC_HELP_LANGUAGE_RUSSIAN)?;
        export(
            &mut store,
            &mut bundle,
            &adapter,
            &text,
            "dtc-help-rus",
            &mut rejections,
        )?;
    }
    summary.push(bundle.finish()?);

    let mut bundle = Bundle::create(out, "odst.json")?;
    for found in &corpus.odst {
        let text = read(found)?;
        let adapter = OdstInfoAdapter::new(source(found, &text)?)?;
        export(
            &mut store,
            &mut bundle,
            &adapter,
            &text,
            "odst",
            &mut rejections,
        )?;
    }
    summary.push(bundle.finish()?);

    // JLR's part lineage (ADR-0033). Only what SDD stamps as production
    // data is ingested: its own test documents are the ones carrying service
    // actions and coordinated flash lists, and they are skipped by name of
    // that stamp rather than by file name. Three of the production files are
    // byte-identical copies of one document, so the same content is ingested
    // once.
    let mut bundle = Bundle::create(out, "ivs_lineage.json")?;
    let mut skipped_ivs = 0usize;
    let mut seen_ivs: BTreeSet<String> = BTreeSet::new();
    for found in &corpus.ivs {
        let text = read(found)?;
        if !IvsLineageAdapter::is_production(&text) {
            skipped_ivs += 1;
            continue;
        }
        if !seen_ivs.insert(sha256_bytes(text.as_bytes())) {
            skipped_ivs += 1;
            continue;
        }
        let adapter = IvsLineageAdapter::new(source(found, &text)?)?;
        export(
            &mut store,
            &mut bundle,
            &adapter,
            &text,
            "ivs",
            &mut rejections,
        )?;
    }
    summary.push(bundle.finish()?);
    if skipped_ivs > 0 {
        println!(
            "IVS: {skipped_ivs} document(s) skipped — SDD's own test files and repeated copies (ADR-0033)"
        );
    }

    let mut bundle = Bundle::create(out, "can_link_monitor.json")?;
    for found in &corpus.link_monitor {
        let text = read(found)?;
        let adapter =
            CanLinkMonitorAdapter::new(source(found, &text)?)?.with_timeline(timeline.clone());
        export(
            &mut store,
            &mut bundle,
            &adapter,
            &text,
            "link-monitor",
            &mut rejections,
        )?;
    }
    summary.push(bundle.finish()?);

    let mut bundle = Bundle::create(out, "vin_decode.json")?;
    for found in &corpus.vin {
        let text = read(found)?;
        let adapter = VinDecodeAdapter::new(source(found, &text)?)?;
        export(
            &mut store,
            &mut bundle,
            &adapter,
            &text,
            "vin-decode",
            &mut rejections,
        )?;
    }
    summary.push(bundle.finish()?);

    let mut bundle = Bundle::create(out, "sdd_text.json")?;
    for found in &corpus.module_text {
        let text = read(found)?;
        let adapter = ModuleTextAdapter::new(source(found, &text)?)?;
        export(
            &mut store,
            &mut bundle,
            &adapter,
            &text,
            "module-text",
            &mut rejections,
        )?;
    }
    summary.push(bundle.finish()?);

    // The configuration descriptions (ADR-0028), their titles and options
    // resolved through the whole text database in English and Russian.
    let mut lookup = TextLookup::new();
    for found in corpus.texts.iter().chain(corpus.module_text.iter()) {
        if let Err(error) = lookup.insert_from_xml(&read(found)?) {
            rejections.note("text-item", error);
        }
    }
    let lookup = std::sync::Arc::new(lookup);
    let mut bundle = Bundle::create(out, "ccf.json")?;
    for found in &corpus.ccf {
        let text = read(found)?;
        let adapter = CcfAdapter::new(source(found, &text)?)?
            .with_timeline(timeline.clone())
            .with_texts(lookup.clone());
        export(
            &mut store,
            &mut bundle,
            &adapter,
            &text,
            "ccf",
            &mut rejections,
        )?;
    }
    summary.push(bundle.finish()?);

    println!("\nwritten to {}:", out.display());
    for (name, manifests, records) in &summary {
        println!("  {name:<24} {manifests:>6} manifests {records:>8} records");
    }
    println!(
        "\nstore: {} sources, {} evidence, {} records",
        store.sources().iter().count(),
        store.evidence_count(),
        store.record_count()
    );
    if rejections.by_reason.is_empty() {
        println!("rejected: 0");
    } else {
        let total: usize = rejections.by_reason.values().sum();
        println!("rejected: {total} (by reason)");
        let mut reasons: Vec<_> = rejections.by_reason.iter().collect();
        reasons.sort_by(|left, right| right.1.cmp(left.1));
        for (reason, count) in reasons.iter().take(15) {
            println!("  {count:>6}  {reason}");
        }
    }
    Ok(())
}
