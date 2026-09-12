//! F10 composition layer: the diagnostic data library and the vehicle survey.
//!
//! This is the data-only orchestration the application shell calls. It loads
//! F5 knowledge manifests, asks F6 and F10 what a vehicle carries and how far
//! each module can be reached, and turns the answers into `app-contracts`
//! snapshots the UI can show. It opens no transport and transmits nothing.
//!
//! The knowledge base, not SDD, is the runtime dependency: SDD-derived
//! manifests are produced by the owner's own tooling and loaded from a
//! directory of their choosing. Only the two documented manifests in this
//! repository are built in.

pub mod decode;
pub mod dtc_text;
pub mod help_text;
pub mod issue;
pub mod mileage;
pub mod parameter_text;
pub mod vin;

pub use issue::ISSUE_STAMP_FILE;
/// The context every question to the library is asked in. Re-exported so a
/// caller needs the library's crate and not the store's.
pub use knowledge::VehicleContext;

use app_contracts::{
    LibraryIssue, LibraryIssueIntegrity, LibrarySnapshot, LibraryState, ManifestFailure,
    MarkerEntry, ModuleApplicability, ModuleSurveyEntry, ProgrammeEntry, ReadableIdentifierSummary,
    RouteStatus, RouteSummary, VehicleCatalogueSnapshot, VehicleContextInput,
    VehicleSurveySnapshot, VinDecodeSnapshot,
};
use diagnostic_environment::{
    DiagnosticEnvironmentPlan, DiagnosticEnvironmentResolution, DiagnosticEnvironmentResolver,
    EnvironmentField, PartialDiagnosticEnvironment, QueryError,
};
use knowledge::{
    manifest_file, ApplicabilityResolution, ClaimKey, DimensionConstraint, EntityKind,
    EvidenceClass, JsonManifestAdapter, KnowledgeQuery, KnowledgeStore, KnowledgeValue,
    YearConstraint,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use uds_execution::{READ_DATA_BY_IDENTIFIER_CAPABILITY, READ_DTC_INFORMATION_CAPABILITY};
use vin::{VinRule, VinTables};

/// Custom applicability dimension carrying SDD's model-year breakpoint marker.
/// The SDD ingestion adapters write it under this name; a test asserts the two
/// literals agree.
pub const SDD_YEAR_BREAKPOINT_DIMENSION: &str = "sdd_year_breakpoint";

/// Manifests that ship with the application: the documented adapter route
/// bindings (ADR-0013), the X250 CCP connector route, and the relayed-route
/// hypotheses for the 2014-and-later high-speed buses (ADR-0015). They
/// describe the adapter, public wiring documentation and this project's own
/// reasoning, never SDD content.
const BUILT_IN_MANIFESTS: [(&str, &str); 5] = [
    (
        "built-in:mongoose_jlr_route_bindings.json",
        include_str!("../../../fixtures/knowledge/documented/mongoose_jlr_route_bindings.json"),
    ),
    (
        "built-in:x250_ccp_route.json",
        include_str!("../../../fixtures/knowledge/documented/x250_ccp_route.json"),
    ),
    (
        "built-in:iso15765_normal_fixed_addressing.json",
        include_str!(
            "../../../fixtures/knowledge/documented/iso15765_normal_fixed_addressing.json"
        ),
    ),
    (
        "built-in:mongoose_jlr_relayed_route_hypotheses.json",
        include_str!(
            "../../../fixtures/knowledge/research/mongoose_jlr_relayed_route_hypotheses.json"
        ),
    ),
    (
        "built-in:normal_fixed_tester_address_hypothesis.json",
        include_str!(
            "../../../fixtures/knowledge/research/normal_fixed_tester_address_hypothesis.json"
        ),
    ),
];

/// At most this many per-manifest failures travel in a snapshot; the counts
/// are always complete.
const REPORTED_FAILURES: usize = 25;

/// Turn what the user stated into an F5 vehicle context. Empty strings mean
/// "not stated", never "any".
pub fn vehicle_context(input: &VehicleContextInput) -> VehicleContext {
    let text = |value: &str| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    };
    let optional = |value: &Option<String>| value.as_deref().and_then(text);
    let mut other = BTreeMap::new();
    if let Some(marker) = optional(&input.year_breakpoint) {
        other.insert(SDD_YEAR_BREAKPOINT_DIMENSION.to_string(), marker);
    }
    VehicleContext {
        vehicle_program: text(&input.vehicle_program),
        model_year: input.model_year,
        powertrain: optional(&input.powertrain),
        variant: optional(&input.variant),
        market: optional(&input.market),
        other,
        ..VehicleContext::default()
    }
}

/// SDD's wording for a fault code and its failure type byte, as far as the
/// library holds it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DtcDescription {
    pub description: Option<String>,
    pub description_scope: Option<String>,
    pub failure_type_text: Option<String>,
    /// The failure type wording by SDD language code, from the text
    /// database (`eng`, `rus`, …).
    pub failure_type_texts: BTreeMap<String, String>,
    /// The code's own wording in the interface's languages, by the same
    /// language codes. SDD holds fault-code descriptions in English only, so
    /// for the standard codes this is our own text (`dtc_text`); `eng` is
    /// never here, because English stays `description`.
    pub description_texts: BTreeMap<String, String>,
    /// The help for this code on this car, line by line as the screen shows
    /// it: possible causes, actions required, monitoring conditions. In
    /// English, and in this project's own words where it has them
    /// (`ADR-0026`). Empty when the loaded data holds none, or when it holds
    /// several and the vehicle is not described closely enough to choose
    /// between them.
    pub help: Vec<String>,
    /// The same screen in the interface's languages, by SDD's language code,
    /// each list the same length and order as `help`. A line this project has
    /// no wording for stands in English inside them. Empty when no line on
    /// the screen has any.
    pub help_texts: BTreeMap<String, Vec<String>>,
    /// Why there is no help, when the reason is worth saying.
    pub help_note: Option<String>,
}

/// One help screen SDD gives a car for a code: what a car must be to be given
/// it, which fault type it answers, and the screen's name.
#[derive(Debug)]
struct HelpSelection {
    applicability: knowledge::Applicability,
    fault_type: Option<String>,
    screen: String,
}

/// Fault-code wording indexed at load: descriptions by `DTC-<code>` entity,
/// each with the module it is scoped to when it is, and failure-type wording
/// by `FTB-<number>`.
#[derive(Debug, Default)]
struct DtcIndex {
    descriptions: BTreeMap<String, Vec<(Option<String>, String)>>,
    failure_types: BTreeMap<String, String>,
    /// `FTB-<n>` → language code → wording, from the text database.
    failure_type_texts: BTreeMap<String, BTreeMap<String, String>>,
    /// `DTC-<code>` → the help screens the data offers for it.
    help: BTreeMap<String, Vec<HelpSelection>>,
    /// `DTC-<code>` → screen name → what that screen says, line by line.
    help_screens: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    /// `DTC-<code>` → screen name → the screen's items, each the mnemonic's
    /// name and its text on one line. Present in libraries exported since
    /// 2026-09-12; absent before, when `help_screens` alone is used.
    help_screen_items: BTreeMap<String, BTreeMap<String, Vec<(String, String)>>>,
}

/// Everything indexed in one pass over the store after a load.
#[derive(Default)]
struct Indexes {
    catalogue: VehicleCatalogueSnapshot,
    dtc_index: DtcIndex,
    /// Module family → language code → SDD's name; ISO 14229 wording first,
    /// the earlier text family only where the former has none.
    module_names: BTreeMap<String, BTreeMap<String, String>>,
    vin_tables: VinTables,
}

/// The loaded knowledge and an honest account of how it was loaded.
pub struct KnowledgeLibrary {
    store: KnowledgeStore,
    snapshot: LibrarySnapshot,
    indexes: Indexes,
}

impl Default for KnowledgeLibrary {
    fn default() -> Self {
        Self::built_in()
    }
}

impl KnowledgeLibrary {
    /// Only the manifests built into the application.
    pub fn built_in() -> Self {
        let mut library = Self {
            store: KnowledgeStore::new(),
            indexes: Indexes::default(),
            snapshot: LibrarySnapshot {
                issue: None,
                state: LibraryState::NotLoaded,
                directory: None,
                manifests_loaded: 0,
                manifests_failed: 0,
                sources: 0,
                records: 0,
                failures: Vec::new(),
                message: String::new(),
            },
        };
        for (name, text) in BUILT_IN_MANIFESTS {
            library.ingest_text(name, text);
        }
        library.refresh_counts();
        library.snapshot.message = format!(
            "Built-in data only: {} sources, {} records. Load a directory of exported manifests to describe vehicles.",
            library.snapshot.sources, library.snapshot.records
        );
        library
    }

    /// Built-in manifests plus every manifest or manifest bundle found
    /// directly in `directory`, packed (`.json.gz`) or not (`.json`). A file that fails is reported and skipped; the
    /// rest still load, because a library with one bad file is not an empty
    /// library. The issue stamp is reported, not enforced: this is the loader
    /// for tools, examples and tests.
    pub fn load_directory(directory: &Path) -> Self {
        Self::load_directory_with(
            directory,
            &issue::today_utc(),
            issue::TRUSTED_ISSUER_KEYS,
            false,
        )
    }

    /// The application's loader (ADR-0019): the directory is ingested only
    /// if its stamp is present, signed by a trusted key, matches every
    /// bundle and is not past its date. Otherwise the built-in data stays,
    /// the state is `Failed`, and the issue says why.
    pub fn load_issued_directory(directory: &Path) -> Self {
        Self::load_directory_with(
            directory,
            &issue::today_utc(),
            issue::TRUSTED_ISSUER_KEYS,
            true,
        )
    }

    /// The loader with its date and trusted keys supplied, for tests.
    pub fn load_directory_with(
        directory: &Path,
        today: &str,
        trusted_keys: &[&str],
        enforce_issue: bool,
    ) -> Self {
        let mut library = Self::built_in();
        library.snapshot.directory = Some(directory.display().to_string());

        let entries = match std::fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) => {
                library.snapshot.state = LibraryState::Failed;
                library.snapshot.message = format!("Cannot read {}: {error}", directory.display());
                return library;
            }
        };
        // A manifest is JSON, packed or not (ADR-0023); the rule that keeps
        // a Mac's AppleDouble sidecars out of the count lives with it.
        let mut files: Vec<_> = entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|name| {
                        manifest_file::is_manifest(name) && name != ISSUE_STAMP_FILE
                    })
            })
            .collect();
        files.sort();
        let stamp = std::fs::read_to_string(directory.join(ISSUE_STAMP_FILE))
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());

        let mut texts = Vec::new();
        for path in &files {
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("manifest")
                .to_string();
            // The stamp covers what a manifest says, not how it is packed,
            // so a packed one is decompressed before it is hashed or read.
            match manifest_file::read(path) {
                Ok(text) => texts.push((name, text)),
                Err(error) => library.record_failure(&name, &error.to_string()),
            }
        }
        let mut verdict = issue::verify(stamp.as_ref(), &texts, trusted_keys, today);
        if enforce_issue {
            let accepted = verdict
                .as_ref()
                .is_some_and(|issue| issue.integrity == LibraryIssueIntegrity::Matches);
            if !accepted {
                let issue = verdict.take().unwrap_or(LibraryIssue {
                    issued_to: String::new(),
                    issued_on: String::new(),
                    issue_code: String::new(),
                    valid_until: String::new(),
                    days_left: -1,
                    issuer: String::new(),
                    integrity: LibraryIssueIntegrity::NoStamp,
                });
                library.snapshot.message = refusal_message(&issue);
                library.snapshot.issue = Some(issue);
                library.snapshot.state = LibraryState::Failed;
                return library;
            }
        }
        for (name, text) in &texts {
            library.ingest_text(name, text);
        }
        library.snapshot.issue = verdict;
        library.finish_directory_load(files.len());
        library
    }

    /// Built-in manifests plus the given `(name, text)` manifests, for callers
    /// that hold the JSON already. Used by tests and by tooling.
    pub fn from_manifests<'a>(manifests: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        let mut library = Self::built_in();
        let mut count = 0usize;
        for (name, text) in manifests {
            count += 1;
            library.ingest_text(name, text);
        }
        library.finish_directory_load(count);
        library
    }

    pub fn snapshot(&self) -> &LibrarySnapshot {
        &self.snapshot
    }

    pub fn store(&self) -> &KnowledgeStore {
        &self.store
    }

    /// The programmes, markers and engines a vehicle can be described with.
    pub fn catalogue(&self) -> &VehicleCatalogueSnapshot {
        &self.indexes.catalogue
    }

    /// SDD's name for a module family in a language (`eng`, `rus`, `deu`,
    /// …), exactly as the text database has it. A programme-decorated
    /// mnemonic such as `LR_ABS_L316` is looked up as `ABS` when it has no
    /// entry of its own; nothing else is inferred.
    pub fn module_name(&self, mnemonic: &str, language: &str) -> Option<String> {
        let lookup = |key: &str| {
            self.indexes
                .module_names
                .get(key)
                .and_then(|names| names.get(language))
                .cloned()
        };
        lookup(mnemonic).or_else(|| {
            let base = base_mnemonic(mnemonic);
            (base != mnemonic).then(|| lookup(base)).flatten()
        })
    }

    /// Every language's name for a module family, by SDD language code.
    pub fn module_names(&self, mnemonic: &str) -> BTreeMap<String, String> {
        let lookup = |key: &str| self.indexes.module_names.get(key).cloned();
        lookup(mnemonic)
            .or_else(|| {
                let base = base_mnemonic(mnemonic);
                (base != mnemonic).then(|| lookup(base)).flatten()
            })
            .unwrap_or_default()
    }

    /// Whether the loaded data holds SDD's VIN tables at all.
    pub fn has_vin_tables(&self) -> bool {
        !self.indexes.vin_tables.is_empty()
    }

    /// Decode a VIN with SDD's tables; the snapshot says what was and was
    /// not established.
    pub fn decode_vin(&self, vin: &str) -> VinDecodeSnapshot {
        self.indexes.vin_tables.decode(vin)
    }

    /// SDD's wording for a fault code as read from `module`: the module-scoped
    /// entry when one exists, otherwise the programme-independent one, plus
    /// the failure type byte's wording. Absent wording stays absent.
    pub fn describe_dtc(&self, code: &str, failure_type: u8, module: &str) -> DtcDescription {
        let mut description = DtcDescription::default();
        if let Some(entries) = self
            .indexes
            .dtc_index
            .descriptions
            .get(&format!("DTC-{code}"))
        {
            let scoped = entries
                .iter()
                .find(|(scope, _)| scope.as_deref() == Some(module));
            let generic = entries.iter().find(|(scope, _)| scope.is_none());
            if let Some((_, text)) = scoped {
                description.description = Some(text.clone());
                description.description_scope = Some("module".into());
            } else if let Some((_, text)) = generic {
                description.description = Some(text.clone());
                description.description_scope = Some("generic".into());
            }
        }
        let ftb = format!("FTB-{failure_type}");
        description.failure_type_text = self.indexes.dtc_index.failure_types.get(&ftb).cloned();
        if let Some(texts) = self.indexes.dtc_index.failure_type_texts.get(&ftb) {
            description.failure_type_texts = texts.clone();
            if description.failure_type_text.is_none() {
                description.failure_type_text = texts.get("eng").cloned();
            }
        }
        // Our own wording for the codes the standard defines. It never
        // replaces `description`, which stays the loaded data's English and
        // is what every report carries.
        description.description_texts = dtc_text::standard_texts(code);
        description
    }

    /// The same, with SDD's own help for the car in front of us.
    ///
    /// The help a code carries depends on the module, the vehicle programme,
    /// the model year and the fault type, because SDD writes a different
    /// screen for each. The fault type's own screen is preferred; a screen
    /// that names no fault type stands in. When the loaded data offers
    /// several and the vehicle is not described closely enough to choose,
    /// none is shown and the reason is said — a screen from the neighbouring
    /// model year is worse than no screen.
    pub fn describe_dtc_with_help(
        &self,
        code: &str,
        failure_type: u8,
        module: &str,
        context: &VehicleContext,
    ) -> DtcDescription {
        let mut description = self.describe_dtc(code, failure_type, module);
        let entity = format!("DTC-{code}");
        let Some(selections) = self.indexes.dtc_index.help.get(&entity) else {
            return description;
        };
        // The module is part of what chooses a screen, so it is part of the
        // context the record is resolved against.
        let mut context = context.clone();
        context.ecu_family = Some(module.to_string());

        let wanted = failure_type.to_string();
        let mut screens: Vec<&str> = Vec::new();
        for exact in [true, false] {
            for selection in selections {
                let names_it = selection.fault_type.as_deref() == Some(wanted.as_str());
                let names_none = selection.fault_type.is_none();
                if exact && !names_it {
                    continue;
                }
                if !exact && !names_none {
                    continue;
                }
                if matches!(
                    selection.applicability.resolve(&context),
                    ApplicabilityResolution::NotApplicable
                ) {
                    continue;
                }
                if !screens.contains(&selection.screen.as_str()) {
                    screens.push(&selection.screen);
                }
            }
            if !screens.is_empty() {
                break;
            }
        }

        let Some(screen) = screens.first() else {
            return description;
        };
        if screens.len() > 1 {
            description.help_note = Some(
                "the loaded data holds several help screens for this code; describe the vehicle's model year to choose between them".into(),
            );
            return description;
        }
        // The library says which lines this car is given; the words are this
        // project's own (`ADR-0026`), and a line it has no wording for keeps
        // the library's English. A library that names the screen's items
        // is read by name, the unit a translation is keyed by; an older one
        // is read by the text of each line.
        if let Some(items) = self
            .indexes
            .dtc_index
            .help_screen_items
            .get(&entity)
            .and_then(|screens| screens.get(*screen))
        {
            let (shown, texts) = help_text::screen_by_names(items);
            description.help = shown;
            description.help_texts = texts;
        } else if let Some(lines) = self
            .indexes
            .dtc_index
            .help_screens
            .get(&entity)
            .and_then(|screens| screens.get(*screen))
        {
            let (shown, texts) = help_text::screen(lines);
            description.help = shown;
            description.help_texts = texts;
        }
        description
    }

    /// The mileage a module can be asked for (ADR-0024): every identifier
    /// the loaded data lists for this family whose parameter is a distance
    /// the car has actually travelled, with the parameter's own name and
    /// what kind of reading it is. The legislated counters that reset with
    /// the codes are not here — see `mileage::mileage_kind`.
    pub fn mileage_identifiers(
        &self,
        context: &VehicleContext,
        ecu_family: &str,
    ) -> Vec<(u16, String, mileage::MileageKind)> {
        let mut found = Vec::new();
        for identifier in
            DiagnosticEnvironmentResolver::readable_identifiers(self.store(), context, ecu_family)
        {
            for parameter in &identifier.parameters {
                if let Some(kind) = mileage::mileage_kind(&parameter.name) {
                    found.push((identifier.identifier, parameter.name.clone(), kind));
                }
            }
        }
        found
    }

    /// The identification identifiers the loaded data declares for this
    /// module (ADR-0027) — the ones whose payload is a text: a part number,
    /// a serial, a software level — each with SDD's own name for it, in
    /// identifier order.
    pub fn identification_identifiers(
        &self,
        context: &VehicleContext,
        ecu_family: &str,
    ) -> Vec<(u16, String)> {
        let mut found = Vec::new();
        for identifier in
            DiagnosticEnvironmentResolver::readable_identifiers(self.store(), context, ecu_family)
        {
            if !decode::is_text(&identifier.parameters) {
                continue;
            }
            let name = identifier
                .parameters
                .first()
                .map(|parameter| parameter.name.clone())
                .unwrap_or_else(|| format!("0x{:04X}", identifier.identifier));
            found.push((identifier.identifier, name));
        }
        found
    }

    /// Every fault code the loaded data describes for this module family:
    /// those SDD scopes to the family itself first, then the ones it states
    /// without a module, each code once and in a fixed order. `describe_dtc`
    /// answers for all of them, which is what makes the list worth drawing
    /// from — the bench picks its codes here so that whatever it reports has
    /// wording to show.
    pub fn dtc_codes_for(&self, module: &str) -> Vec<&str> {
        let mut scoped = Vec::new();
        let mut generic = Vec::new();
        for (id, entries) in &self.indexes.dtc_index.descriptions {
            let Some(code) = id.strip_prefix("DTC-") else {
                continue;
            };
            if entries
                .iter()
                .any(|(scope, _)| scope.as_deref() == Some(module))
            {
                scoped.push(code);
            } else if entries.iter().any(|(scope, _)| scope.is_none()) {
                generic.push(code);
            }
        }
        scoped.append(&mut generic);
        scoped
    }

    /// What the loaded knowledge says a vehicle carries and how far each
    /// module can be reached over the adapter.
    pub fn survey(&self, input: &VehicleContextInput) -> VehicleSurveySnapshot {
        survey_vehicle(&self.store, input, &|mnemonic| self.module_names(mnemonic))
    }

    /// Ingest one manifest file's text. A bundle — a JSON array of manifests —
    /// is taken apart and each element ingested on its own, so one bad element
    /// costs only itself.
    fn ingest_text(&mut self, name: &str, text: &str) {
        let value: serde_json::Value = match serde_json::from_str(text) {
            Ok(value) => value,
            Err(error) => {
                self.record_failure(name, &format!("not valid JSON: {error}"));
                return;
            }
        };
        let items = match value {
            serde_json::Value::Array(items) => items,
            other => vec![other],
        };
        let bundled = items.len() > 1;
        for (index, item) in items.into_iter().enumerate() {
            let label = if bundled {
                format!("{name}#{index}")
            } else {
                name.to_string()
            };
            match self.store.ingest(&JsonManifestAdapter, &item.to_string()) {
                Ok(_) => self.snapshot.manifests_loaded += 1,
                Err(error) => self.record_failure(&label, &error.to_string()),
            }
        }
    }

    fn record_failure(&mut self, file: &str, message: &str) {
        self.snapshot.manifests_failed += 1;
        if self.snapshot.failures.len() < REPORTED_FAILURES {
            self.snapshot.failures.push(ManifestFailure {
                file: file.to_string(),
                message: message.to_string(),
            });
        }
    }

    fn refresh_counts(&mut self) {
        self.snapshot.sources = self.store.sources().iter().count() as u32;
        self.snapshot.records = self.store.record_count() as u32;
        self.indexes = build_indexes(&self.store);
    }

    fn finish_directory_load(&mut self, files: usize) {
        self.refresh_counts();
        let built_in = BUILT_IN_MANIFESTS.len() as u32;
        let loaded = self.snapshot.manifests_loaded.saturating_sub(built_in);
        let failed = self.snapshot.manifests_failed;
        let (state, message) = if files == 0 {
            (
                LibraryState::NotLoaded,
                "No .json manifests were found in that directory; built-in data only.".to_string(),
            )
        } else if loaded == 0 {
            (
                LibraryState::Failed,
                format!("None of the {files} files could be loaded; see the details below."),
            )
        } else if failed > 0 {
            (
                LibraryState::PartiallyLoaded,
                format!(
                    "Loaded {loaded} manifests ({} sources, {} records); {failed} could not be loaded — see the details below.",
                    self.snapshot.sources, self.snapshot.records
                ),
            )
        } else {
            (
                LibraryState::Loaded,
                format!(
                    "Loaded {loaded} manifests: {} sources, {} records.",
                    self.snapshot.sources, self.snapshot.records
                ),
            )
        };
        self.snapshot.state = state;
        self.snapshot.message = message;
    }
}

/// `LR_ABS_L316` → `ABS`: the programme decoration some platform documents
/// put around a mnemonic, and nothing else.
fn base_mnemonic(mnemonic: &str) -> &str {
    let base = mnemonic.strip_prefix("LR_").unwrap_or(mnemonic);
    match base.rfind("_L") {
        Some(index)
            if base[index + 2..].len() == 3
                && base[index + 2..].bytes().all(|b| b.is_ascii_digit()) =>
        {
            &base[..index]
        }
        _ => base,
    }
}

/// One pass over the store for what the UI needs to offer before any vehicle
/// is described: the programmes, their markers with derived model years and
/// their engines; and the fault-code wording a read will be joined to.
fn build_indexes(store: &KnowledgeStore) -> Indexes {
    type Markers = BTreeMap<String, (Option<u16>, Option<u16>)>;
    let mut programmes: BTreeMap<String, (Markers, BTreeSet<String>, BTreeSet<String>)> =
        BTreeMap::new();
    let mut dtc_index = DtcIndex::default();
    let mut module_names: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut legacy_names: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut vin_rules: BTreeMap<String, VinRule> = BTreeMap::new();
    let mut vin_tables = VinTables::default();

    let result = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    for entry in &result.records {
        let record = &entry.record;
        match record.entity.kind {
            EntityKind::VinDecodeModel => {
                let (KnowledgeValue::Text { value }, ClaimKey::Custom { name }) =
                    (&record.value, &record.key)
                else {
                    continue;
                };
                let Some(model) = record
                    .entity
                    .id
                    .strip_prefix("VIN-MODEL-")
                    .and_then(|id| id.parse::<u32>().ok())
                else {
                    continue;
                };
                if name == "sdd_vin_rule" {
                    if let Some(tests) = vin::parse_rule(value) {
                        vin_rules.insert(record.id.clone(), VinRule { model, tests });
                    }
                } else if let Some(attribute) = name.strip_prefix("sdd_vin_attribute.") {
                    if let Some(parsed) = vin::parse_attribute(value) {
                        vin_tables
                            .attributes
                            .entry(model)
                            .or_default()
                            .insert(attribute.to_string(), parsed);
                        if let Some(note) = record
                            .evidence_ids
                            .iter()
                            .filter_map(|id| store.get_evidence(id))
                            .find_map(|evidence| evidence.notes.clone())
                        {
                            vin_tables.versions.entry(model).or_insert(note);
                        }
                    }
                }
            }
            EntityKind::EcuFamily | EntityKind::IdentifierParameter => {
                if record.entity.kind == EntityKind::EcuFamily {
                    if let (ClaimKey::Custom { name }, KnowledgeValue::Text { value }) =
                        (&record.key, &record.value)
                    {
                        if let Some(language) = name.strip_prefix("sdd_module_name.") {
                            module_names
                                .entry(record.entity.id.clone())
                                .or_default()
                                .entry(language.to_string())
                                .or_insert_with(|| value.clone());
                            continue;
                        }
                        if let Some(language) = name.strip_prefix("sdd_module_name_legacy.") {
                            legacy_names
                                .entry(record.entity.id.clone())
                                .or_default()
                                .entry(language.to_string())
                                .or_insert_with(|| value.clone());
                            continue;
                        }
                    }
                }
                let DimensionConstraint::OneOf { values } = &record.applicability.vehicle_program
                else {
                    continue;
                };
                let [program] = values.as_slice() else {
                    continue;
                };
                let slot = programmes.entry(program.clone()).or_default();
                if record.entity.kind == EntityKind::EcuFamily {
                    if let Some(DimensionConstraint::OneOf { values }) = record
                        .applicability
                        .other
                        .get(SDD_YEAR_BREAKPOINT_DIMENSION)
                    {
                        let years = match record.applicability.model_year {
                            YearConstraint::Range {
                                model_year_from,
                                model_year_to,
                            } => (model_year_from, model_year_to),
                            _ => (None, None),
                        };
                        for marker in values {
                            slot.0.entry(marker.clone()).or_insert(years);
                        }
                    }
                }
                if let DimensionConstraint::OneOf { values } = &record.applicability.powertrain {
                    slot.1.extend(values.iter().cloned());
                }
                if let DimensionConstraint::OneOf { values } = &record.applicability.variant {
                    slot.2.extend(values.iter().cloned());
                }
            }
            EntityKind::DiagnosticTroubleCode => {
                let KnowledgeValue::Text { value } = &record.value else {
                    continue;
                };
                let id = record.entity.id.as_str();
                if id.starts_with("DTC-") && record.key == ClaimKey::Alias {
                    let module = match &record.applicability.ecu_family {
                        DimensionConstraint::OneOf { values } if values.len() == 1 => {
                            Some(values[0].clone())
                        }
                        _ => None,
                    };
                    dtc_index
                        .descriptions
                        .entry(id.to_string())
                        .or_default()
                        .push((module, value.clone()));
                } else if id.starts_with("DTC-") {
                    // The help layer (ADR-0022 is the live read; this is the
                    // DTC help SDD carries). A car is given a screen by name;
                    // what the screen says is a claim of its own, so a screen
                    // dozens of cars select is carried once.
                    if let ClaimKey::Custom { name } = &record.key {
                        if name == "sdd_help" {
                            let fault_type = match record.applicability.other.get("sdd_fault_type")
                            {
                                Some(DimensionConstraint::OneOf { values })
                                    if values.len() == 1 =>
                                {
                                    Some(values[0].clone())
                                }
                                _ => None,
                            };
                            dtc_index
                                .help
                                .entry(id.to_string())
                                .or_default()
                                .push(HelpSelection {
                                    applicability: record.applicability.clone(),
                                    fault_type,
                                    screen: value.clone(),
                                });
                        } else if let Some(screen) = name.strip_prefix("sdd_help_screen_items.") {
                            // One line per item: the name, U+001F, the text.
                            dtc_index
                                .help_screen_items
                                .entry(id.to_string())
                                .or_default()
                                .entry(screen.to_string())
                                .or_insert_with(|| {
                                    value
                                        .lines()
                                        .filter_map(|line| {
                                            let (name, text) = line.split_once('\u{1F}')?;
                                            Some((name.to_string(), text.to_string()))
                                        })
                                        .collect()
                                });
                        } else if let Some(screen) = name.strip_prefix("sdd_help_screen.") {
                            dtc_index
                                .help_screens
                                .entry(id.to_string())
                                .or_default()
                                .entry(screen.to_string())
                                .or_insert_with(|| value.lines().map(str::to_string).collect());
                        }
                    }
                } else if id.starts_with("FTB-") {
                    if let ClaimKey::Custom { name } = &record.key {
                        if name == "sdd_failure_type" {
                            dtc_index
                                .failure_types
                                .entry(id.to_string())
                                .or_insert_with(|| value.clone());
                        } else if let Some(language) = name.strip_prefix("sdd_failure_type.") {
                            dtc_index
                                .failure_type_texts
                                .entry(id.to_string())
                                .or_default()
                                .entry(language.to_string())
                                .or_insert_with(|| value.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    for (mnemonic, names) in legacy_names {
        let slot = module_names.entry(mnemonic).or_default();
        for (language, name) in names {
            slot.entry(language).or_insert(name);
        }
    }
    // Record ids carry the document order (`vin.rule.001`, …), which is
    // SDD's matching order.
    vin_tables.rules = vin_rules.into_values().collect();

    let catalogue = VehicleCatalogueSnapshot {
        programmes: programmes
            .into_iter()
            .filter(|(_, (markers, _, _))| !markers.is_empty())
            .map(
                |(program, (markers, powertrains, variants))| ProgrammeEntry {
                    program,
                    markers: markers
                        .into_iter()
                        .map(|(marker, (from, to))| MarkerEntry {
                            marker,
                            model_year_from: from,
                            model_year_to: to,
                        })
                        .collect(),
                    powertrains: powertrains.into_iter().collect(),
                    variants: variants.into_iter().collect(),
                },
            )
            .collect(),
    };
    Indexes {
        catalogue,
        dtc_index,
        module_names,
        vin_tables,
    }
}

/// Survey a vehicle against a store: every ECU family the knowledge associates
/// with it, each resolved for identifier reads and for fault-code reads, with
/// the resolver's own reasons wherever a route is not usable. Nothing is
/// hidden: an unreachable module is a row with its reasons, never an omission.
pub fn survey_vehicle(
    store: &KnowledgeStore,
    input: &VehicleContextInput,
    module_names: &dyn Fn(&str) -> BTreeMap<String, String>,
) -> VehicleSurveySnapshot {
    let context = vehicle_context(input);
    if context.vehicle_program.is_none() {
        return VehicleSurveySnapshot {
            context: input.clone(),
            modules: Vec::new(),
            reachable: 0,
            hypothesis: 0,
            unreachable: 0,
            message: "State the vehicle programme as SDD names it, for example X250.".into(),
        };
    }

    let mut families = DiagnosticEnvironmentResolver::enumerate_ecu_families(store, &context);
    families.sort_by(|left, right| left.ecu_family.cmp(&right.ecu_family));

    let mut modules = Vec::new();
    for presence in families {
        let applicability = match presence.resolution {
            ApplicabilityResolution::Applicable => ModuleApplicability::Applicable,
            ApplicabilityResolution::InsufficientContext => {
                ModuleApplicability::InsufficientContext
            }
            ApplicabilityResolution::InsufficientEvidence => {
                ModuleApplicability::InsufficientEvidence
            }
            ApplicabilityResolution::NotApplicable => continue,
        };
        let names = module_names(&presence.ecu_family);
        let mut entry = ModuleSurveyEntry {
            ecu_family: presence.ecu_family.clone(),
            name: names.get("eng").cloned(),
            names,
            applicability,
            logical_network: presence.logical_network.clone(),
            request_id: None,
            response_id: None,
            backend_route: None,
            pins: None,
            bitrate_bps: None,
            protocol: None,
            route_validation: String::new(),
            identifier_read: not_applicable(),
            dtc_read: not_applicable(),
            readable_identifiers: Vec::new(),
        };

        if applicability != ModuleApplicability::Applicable {
            let reason = match applicability {
                ModuleApplicability::InsufficientContext => {
                    "the vehicle description lacks a detail this module's data is qualified by"
                }
                _ => "the data for this module does not state which vehicles it applies to",
            };
            entry.identifier_read.reasons.push(reason.to_string());
            entry.dtc_read.reasons.push(reason.to_string());
            modules.push(entry);
            continue;
        }

        let family = presence.ecu_family.as_str();
        entry.identifier_read = route_summary(
            &mut entry,
            DiagnosticEnvironmentResolver::resolve_ecu_family(
                store,
                &context,
                family,
                READ_DATA_BY_IDENTIFIER_CAPABILITY,
            ),
        );
        entry.dtc_read = route_summary(
            &mut entry,
            DiagnosticEnvironmentResolver::resolve_ecu_family(
                store,
                &context,
                family,
                READ_DTC_INFORMATION_CAPABILITY,
            ),
        );
        entry.readable_identifiers =
            DiagnosticEnvironmentResolver::readable_identifiers(store, &context, family)
                .into_iter()
                .map(|identifier| ReadableIdentifierSummary {
                    identifier: format!("0x{:04X}", identifier.identifier),
                    parameters: identifier
                        .parameters
                        .into_iter()
                        .map(|parameter| parameter.name)
                        .collect(),
                })
                .collect();
        modules.push(entry);
    }

    let reachable = modules
        .iter()
        .filter(|module| {
            module.identifier_read.status == RouteStatus::Reachable
                || module.dtc_read.status == RouteStatus::Reachable
        })
        .count() as u32;
    let hypothesis = modules
        .iter()
        .filter(|module| {
            module.identifier_read.status == RouteStatus::Hypothesis
                || module.dtc_read.status == RouteStatus::Hypothesis
        })
        .count() as u32;
    let unreachable = modules.len() as u32 - reachable - hypothesis;
    let message = if modules.is_empty() {
        "No modules are known for this vehicle in the loaded data. Check the programme name and the SDD breakpoint marker, and that a library is loaded.".to_string()
    } else {
        format!(
            "{} modules known: {reachable} reachable over the adapter, {hypothesis} on a hypothesised route, {unreachable} not — each with the reason.",
            modules.len()
        )
    };
    VehicleSurveySnapshot {
        context: input.clone(),
        modules,
        reachable,
        hypothesis,
        unreachable,
        message,
    }
}

fn not_applicable() -> RouteSummary {
    RouteSummary {
        status: RouteStatus::NotApplicable,
        reasons: Vec::new(),
    }
}

fn route_summary(
    entry: &mut ModuleSurveyEntry,
    resolution: Result<DiagnosticEnvironmentResolution, QueryError>,
) -> RouteSummary {
    match resolution {
        Err(error) => RouteSummary {
            status: RouteStatus::Indeterminate,
            reasons: vec![format!("query rejected: {error}")],
        },
        Ok(DiagnosticEnvironmentResolution::Resolved(plan)) => {
            fill_from_plan(entry, &plan);
            entry.route_validation = validation_label(plan.validation_state).to_string();
            if route_is_hypothesis(&plan) {
                RouteSummary {
                    status: RouteStatus::Hypothesis,
                    reasons: vec![
                        "the adapter route for this bus is an unverified hypothesis (ADR-0015); a first read-only request confirms or refutes it"
                            .to_string(),
                    ],
                }
            } else {
                RouteSummary {
                    status: RouteStatus::Reachable,
                    reasons: Vec::new(),
                }
            }
        }
        Ok(DiagnosticEnvironmentResolution::Indeterminate {
            partial,
            unresolved_facts,
        }) => {
            fill_from_partial(entry, &partial);
            RouteSummary {
                status: RouteStatus::Indeterminate,
                reasons: unresolved_facts
                    .iter()
                    .map(|fact| format!("{}: {}", field_label(fact.field), fact.reason))
                    .collect(),
            }
        }
        Ok(DiagnosticEnvironmentResolution::Conflict { partial, conflicts }) => {
            fill_from_partial(entry, &partial);
            RouteSummary {
                status: RouteStatus::Conflict,
                reasons: conflicts
                    .iter()
                    .map(|conflict| {
                        format!(
                            "{}: {} evidence-backed values disagree",
                            field_label(conflict.field),
                            conflict.candidates.len()
                        )
                    })
                    .collect(),
            }
        }
    }
}

/// A route rests on a hypothesis when any fact that places the module on an
/// adapter route — the backend route or the connector pins — is backed only
/// by unverified research.
/// A route is a hypothesis only while every trace behind its physical and
/// backend route is unverified research; one direct observation or one
/// document ends the hypothesis (ADR-0016 §7).
fn route_is_hypothesis(plan: &DiagnosticEnvironmentPlan) -> bool {
    let traces: Vec<_> = plan
        .backend_route
        .evidence
        .iter()
        .chain(plan.physical_route.evidence.iter())
        .collect();
    !traces.is_empty()
        && traces
            .iter()
            .any(|trace| trace.evidence_class == Some(EvidenceClass::UnverifiedResearch))
        && !traces.iter().any(|trace| {
            matches!(
                trace.evidence_class,
                Some(EvidenceClass::DirectObservation)
                    | Some(EvidenceClass::OemDocumentation)
                    | Some(EvidenceClass::StandardDocumentation)
            )
        })
}

/// The resolver's validation state in the words the UI and reports use.
pub fn validation_label(state: diagnostic_environment::ValidationState) -> &'static str {
    use diagnostic_environment::ValidationState;
    match state {
        ValidationState::Unverified => "UNVERIFIED",
        ValidationState::SourceBacked => "SOURCE_BACKED",
        ValidationState::Corroborated => "CORROBORATED",
        ValidationState::CaptureValidated => "CAPTURE_VALIDATED",
        ValidationState::Contradicted => "CONTRADICTED",
        ValidationState::Deprecated => "DEPRECATED",
    }
}

fn fill_from_plan(entry: &mut ModuleSurveyEntry, plan: &DiagnosticEnvironmentPlan) {
    entry.logical_network = Some(plan.logical_network.value.clone());
    entry.request_id = Some(format!("0x{:X}", plan.physical_request_id.value));
    entry.response_id = Some(format!("0x{:X}", plan.physical_response_id.value));
    entry.backend_route = Some(plan.backend_route.value.route_id.clone());
    entry.pins = Some(pins_text(&plan.physical_route.value.pins));
    entry.bitrate_bps = Some(plan.bitrate_bps.value);
    entry.protocol = Some(plan.protocol_family.value.clone());
}

fn fill_from_partial(entry: &mut ModuleSurveyEntry, partial: &PartialDiagnosticEnvironment) {
    if entry.logical_network.is_none() {
        entry.logical_network = partial
            .logical_network
            .as_ref()
            .map(|field| field.value.clone());
    }
    if entry.request_id.is_none() {
        entry.request_id = partial
            .physical_request_id
            .as_ref()
            .map(|field| format!("0x{:X}", field.value));
    }
    if entry.response_id.is_none() {
        entry.response_id = partial
            .physical_response_id
            .as_ref()
            .map(|field| format!("0x{:X}", field.value));
    }
    if entry.backend_route.is_none() {
        entry.backend_route = partial
            .backend_route
            .as_ref()
            .map(|field| field.value.route_id.clone());
    }
    if entry.pins.is_none() {
        entry.pins = partial
            .physical_route
            .as_ref()
            .map(|field| pins_text(&field.value.pins));
    }
    if entry.bitrate_bps.is_none() {
        entry.bitrate_bps = partial.bitrate_bps.as_ref().map(|field| field.value);
    }
    if entry.protocol.is_none() {
        entry.protocol = partial
            .protocol_family
            .as_ref()
            .map(|field| field.value.clone());
    }
}

fn pins_text(pins: &[u8]) -> String {
    pins.iter().map(u8::to_string).collect::<Vec<_>>().join("/")
}

/// The resolver's field names in the words a person reading the survey needs.
fn field_label(field: EnvironmentField) -> String {
    match field {
        EnvironmentField::VehicleApplicability => "vehicle applicability".into(),
        EnvironmentField::EcuFamily => "module family".into(),
        EnvironmentField::DiagnosticImplementation => "software build".into(),
        EnvironmentField::LogicalNetwork => "bus".into(),
        EnvironmentField::PhysicalRoute => "connector pins".into(),
        EnvironmentField::BackendRoute => "adapter route".into(),
        EnvironmentField::Bitrate => "bit rate".into(),
        EnvironmentField::ProtocolFamily => "diagnostic protocol".into(),
        EnvironmentField::AddressingMode => "addressing mode".into(),
        EnvironmentField::CanIdFormat => "CAN identifier width".into(),
        EnvironmentField::PhysicalRequestId => "request identifier".into(),
        EnvironmentField::PhysicalResponseId => "response identifier".into(),
        EnvironmentField::FunctionalRequestId => "functional request identifier".into(),
        EnvironmentField::ReadOnlyCapability => "read-only capability".into(),
        EnvironmentField::EvidenceClassification => "evidence classification".into(),
        EnvironmentField::ImplementationMarker(kind) => {
            format!("implementation marker {kind:?}")
        }
    }
}

/// Why the application did not load a copy (ADR-0019), in the words the
/// status line shows; the panel says the same in the interface language.
fn refusal_message(issue: &LibraryIssue) -> String {
    match issue.integrity {
        LibraryIssueIntegrity::NoStamp => "This folder carries no issue stamp. The application loads only a copy issued to a named person; ask for one.".to_string(),
        LibraryIssueIntegrity::Unsigned => "The stamp of this copy is not signed. The library was not loaded; ask for a new copy.".to_string(),
        LibraryIssueIntegrity::BadSignature => "The stamp's signature is not the owner's. The library was not loaded; ask for a new copy.".to_string(),
        LibraryIssueIntegrity::Mismatch => format!("The data does not match its stamp (copy {}). The library was not loaded; ask for a new copy.", issue.issue_code),
        LibraryIssueIntegrity::StampRemoved => format!("The stamp file is missing; the data carries copy {}. The library was not loaded; ask for a new copy.", issue.issue_code),
        LibraryIssueIntegrity::Expired => format!("This copy, issued to {} until {}, has expired. The library was not loaded; ask for a new copy.", issue.issued_to, issue.valid_until),
        LibraryIssueIntegrity::Matches => String::new(),
    }
}
