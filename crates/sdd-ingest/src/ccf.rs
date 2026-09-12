//! SDD's car configuration file, as its own data describes it (ADR-0028).
//!
//! `CCF_DATA_<PROGRAM>[_<YEAR>].xml` says, for one programme and model-year
//! marker, which module keeps the configuration and which hold copies; how
//! each block of it is read — a data identifier, an offset inside that
//! identifier's payload, a length; and what every byte and bit means —
//! groups with a title, parameters with a span, a mask and a type, options
//! with a value, a name, sometimes a sales code, and a text.
//!
//! Two things enter the knowledge base. The identifiers, in the DID
//! catalogue's shape and with the encoding `ccf=<BLOCK>;offset=<n>;length=<n>`,
//! so a configuration read is a module read the gate admits. And the layout,
//! one record per parameter, as its own kind: a layout, never a value. The
//! write service SDD names beside every block, the as-built VBF names and the
//! memory-read sources are not recorded at all. A document that pages its
//! configuration through VDF blocks records its scheme and its layout and no
//! identifier, so the read is refused with the reason rather than attempted.

use crate::platform::{module_quals, parse_hex, qual_suffix, qual_text, qualified_by_module, slug};
use crate::{
    child_element, child_text, evidence_class_for, qualified_applicability, require_attribute,
    validation_state_for, ModelYearTimeline, YEAR_BREAKPOINT_DIMENSION,
};
use knowledge::{
    Applicability, ClaimKey, DimensionConstraint, EntityKind, EvidenceId, EvidenceRecord,
    IngestionAdapter, IngestionBatch, KnowledgeEntity, KnowledgeError, KnowledgeRecord,
    KnowledgeValue, SourceLocator, SourceRecord, ValidationState,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

pub const CCF_PARSER_ID: &str = "sdd-ccf-data";
const PARSER_VERSION: &str = "1.0.0";

/// Claim on the programme: how its configuration is read — `did` when every
/// block is one identifier, an offset and a length; `vdf` when the document
/// pages it through VDF blocks, which this product does not read.
pub const CCF_SCHEME_CLAIM: &str = "sdd_ccf_scheme";
/// Claim prefix on the programme, the module's name after the dot: `sync`
/// for the module that keeps the master copy, `copy` for one that holds a
/// copy.
pub const CCF_SOURCE_CLAIM_PREFIX: &str = "sdd_ccf_source.";
/// Claim on a `ConfigurationParameter` entity: the layout of one parameter,
/// encoded as `key=value;…` (see `encode_parameter`).
pub const CCF_PARAMETER_CLAIM: &str = "sdd_ccf_parameter";
/// Encoding prefix of a block identifier's parameter in the catalogue's
/// shape: `ccf=<BLOCK>;offset=<n>;length=<n>`.
pub const CCF_BLOCK_ENCODING_PREFIX: &str = "ccf=";

const READ_SERVICE: u32 = 0x22;
/// Names SDD lists as sources that are not modules: the factory file, the
/// car as it is, a placeholder.
const NOT_MODULES: [&str; 3] = ["AS_BUILT", "AS_IS", "OTHER"];

/// SDD's text database as far as the configuration needs it: one English
/// and one Russian text per item id, the two languages the interface shows
/// SDD's data text in.
#[derive(Clone, Debug, Default)]
pub struct TextLookup {
    texts: HashMap<String, (Option<String>, Option<String>)>,
}

impl TextLookup {
    pub fn new() -> Self {
        Self::default()
    }

    /// One text item document, `<tm id="@…"><tu xmlns:lang="eng">…</tu>…`.
    pub fn insert_from_xml(&mut self, input: &str) -> Result<(), KnowledgeError> {
        // SDD writes the language as `xmlns:lang="eng"`, which an XML parser
        // takes for a namespace declaration. It is a plain attribute here.
        let input = input.replace("xmlns:lang=", "lang=");
        let document = roxmltree::Document::parse(&input)
            .map_err(|error| KnowledgeError::Parse(error.to_string()))?;
        let root = document.root_element();
        if root.tag_name().name() != "tm" {
            return Err(KnowledgeError::Parse(format!(
                "expected a <tm> root, found <{}>",
                root.tag_name().name()
            )));
        }
        let id = require_attribute(root, "id")?.trim().to_string();
        let mut english = None;
        let mut russian = None;
        for unit in root
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "tu")
        {
            let text = unit.text().map(str::trim).unwrap_or_default();
            if text.is_empty() {
                continue;
            }
            match unit.attribute("lang").map(str::trim) {
                Some("eng") => english = Some(text.to_string()),
                Some("rus") => russian = Some(text.to_string()),
                _ => {}
            }
        }
        self.insert(&id, english.as_deref(), russian.as_deref());
        Ok(())
    }

    pub fn insert(&mut self, id: &str, english: Option<&str>, russian: Option<&str>) {
        self.texts.insert(
            id.to_string(),
            (english.map(str::to_string), russian.map(str::to_string)),
        );
    }

    pub fn english(&self, id: &str) -> Option<&str> {
        self.texts
            .get(id)
            .and_then(|(english, _)| english.as_deref())
    }

    pub fn russian(&self, id: &str) -> Option<&str> {
        self.texts
            .get(id)
            .and_then(|(_, russian)| russian.as_deref())
    }

    pub fn len(&self) -> usize {
        self.texts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.texts.is_empty()
    }
}

/// Adapter for one `CCF_DATA_<PROGRAM>[_<YEAR>].xml` document.
#[derive(Clone, Debug)]
pub struct CcfAdapter {
    source: SourceRecord,
    timeline: Option<ModelYearTimeline>,
    texts: Arc<TextLookup>,
}

impl CcfAdapter {
    pub fn new(source: SourceRecord) -> Result<Self, KnowledgeError> {
        source.validate()?;
        Ok(Self {
            source,
            timeline: None,
            texts: Arc::new(TextLookup::new()),
        })
    }

    /// Also express the marker as a model-year range (ADR-0011).
    pub fn with_timeline(mut self, timeline: ModelYearTimeline) -> Self {
        self.timeline = Some(timeline);
        self
    }

    /// Resolve the titles and options through SDD's text database. Without
    /// it every text stays the mnemonic, as SDD itself shows when it has
    /// no text.
    pub fn with_texts(mut self, texts: Arc<TextLookup>) -> Self {
        self.texts = texts;
        self
    }

    fn applicability(&self, program: &str, marker: &str) -> Result<Applicability, KnowledgeError> {
        let mut applicability = qualified_applicability();
        applicability.vehicle_program = DimensionConstraint::one_of([program.to_string()])?;
        applicability.other.insert(
            YEAR_BREAKPOINT_DIMENSION.to_string(),
            DimensionConstraint::one_of([marker.to_string()])?,
        );
        if let Some(timeline) = &self.timeline {
            if let Some(range) = timeline.range_for(program, marker)? {
                applicability.model_year = range;
            }
        }
        applicability.validate()?;
        Ok(applicability)
    }

    #[allow(clippy::too_many_arguments)]
    fn push(
        &self,
        record_id: String,
        locator: String,
        excerpt: String,
        entity: KnowledgeEntity,
        key: ClaimKey,
        value: KnowledgeValue,
        applicability: Applicability,
        evidence: &mut BTreeMap<String, EvidenceRecord>,
        records: &mut BTreeMap<String, KnowledgeRecord>,
    ) -> Result<(), KnowledgeError> {
        entity.validate()?;
        let evidence_id = format!("{}.ev.{record_id}", self.source.id.0);
        evidence.insert(
            evidence_id.clone(),
            EvidenceRecord {
                id: EvidenceId::new(evidence_id.clone())?,
                source_id: self.source.id.clone(),
                evidence_class: None,
                locator: SourceLocator {
                    description: locator,
                    document_page: None,
                    document_section: Some("ccf".into()),
                    record_key: Some(entity.id.clone()),
                    capture_timestamp_us: None,
                },
                excerpt: Some(excerpt.chars().take(500).collect()),
                notes: None,
            },
        );
        records.insert(
            record_id.clone(),
            KnowledgeRecord {
                id: record_id,
                entity,
                key,
                value,
                applicability,
                evidence_ids: vec![EvidenceId::new(evidence_id)?],
                validation_state: ValidationState::Unverified,
            },
        );
        Ok(())
    }

    /// The texts an item id resolves to, English and Russian; empty when
    /// SDD has none, and the caller keeps the mnemonic.
    fn text(&self, id: Option<&str>) -> (String, String) {
        let Some(id) = id.map(str::trim).filter(|id| !id.is_empty()) else {
            return (String::new(), String::new());
        };
        (
            self.texts.english(id).unwrap_or_default().to_string(),
            self.texts.russian(id).unwrap_or_default().to_string(),
        )
    }

    /// One `<address>` of a readable block: the identifier the module
    /// answers the block with, in the catalogue's shape.
    #[allow(clippy::too_many_arguments)]
    fn add_block_address(
        &self,
        address: roxmltree::Node<'_, '_>,
        block: &str,
        program: &str,
        marker: &str,
        base: &Applicability,
        evidence: &mut BTreeMap<String, EvidenceRecord>,
        records: &mut BTreeMap<String, KnowledgeRecord>,
    ) -> Result<(), KnowledgeError> {
        let module = require_attribute(address, "module")?.trim().to_string();
        if NOT_MODULES.contains(&module.as_str()) {
            return Ok(());
        }
        // An address whose own qualifier names another model-year marker
        // than the document's applies to that year: the X250 document of
        // 2013 stamps its reserved block's addresses MY12, and the source's
        // statement is kept rather than the file's.
        let own_year = child_element(address, "qualifier")
            .and_then(|qualifier| qualifier.attribute("year"))
            .map(str::trim)
            .filter(|year| !year.is_empty() && *year != marker);
        let year_base = match own_year {
            Some(year) => self.applicability(program, year)?,
            None => base.clone(),
        };
        // A memory-read source is not a service this product makes; a
        // block with no identifier is padding.
        let Some(did) = address.attribute("parameterId").and_then(parse_hex) else {
            return Ok(());
        };
        if did == 0 || did > 0xFFFF {
            return Ok(());
        }
        let offset = address
            .attribute("parameterIdOffset")
            .and_then(parse_hex)
            .unwrap_or(0);
        let (Some(start), Some(stop)) = (
            address.attribute("start_address").and_then(parse_hex),
            address.attribute("stop_address").and_then(parse_hex),
        ) else {
            return Err(KnowledgeError::Parse(format!(
                "block '{block}' address for {module} states no byte range"
            )));
        };
        if stop < start {
            return Err(KnowledgeError::Parse(format!(
                "block '{block}' address for {module} ends before it starts"
            )));
        }
        let length = stop - start + 1;
        let quals = module_quals(address);
        let mut applicability = qualified_by_module(&year_base, &quals)?;
        applicability.ecu_family = DimensionConstraint::one_of([module.clone()])?;
        applicability.validate()?;
        let mut qualifier_note = if quals.is_empty() {
            String::new()
        } else {
            format!(" qualifier={}", qual_text(&quals))
        };
        if let Some(year) = own_year {
            qualifier_note.push_str(&format!(" year={year}"));
        }
        let identifier = format!("0x{did:04X}");
        // A digest rather than the words: an evidence id repeats the source
        // id inside itself, and the readable form would push the longest
        // rows over the id limit. The words are in the evidence note.
        let record_id = format!(
            "{}.ccf.i.{}",
            self.source.id.0,
            digest(&format!(
                "{block}/{module}/{identifier}{}{}",
                qual_suffix(&quals),
                own_year.map(|year| format!("/{year}")).unwrap_or_default()
            ))
        );
        self.push(
            record_id,
            format!("configuration_data/block[@name='{block}']/address[@module='{module}']"),
            format!(
                "{program} {marker} {block}: {module} {identifier} offset {offset} length {length} service 0x22{qualifier_note}"
            ),
            KnowledgeEntity {
                kind: EntityKind::IdentifierParameter,
                id: format!("DID-{identifier}"),
            },
            ClaimKey::ParameterDefinition {
                parameter: format!("Car configuration block {block}"),
            },
            KnowledgeValue::IdentifierDefinition {
                identifier,
                encoding: Some(format!(
                    "{CCF_BLOCK_ENCODING_PREFIX}{block};offset={offset};length={length}"
                )),
                unit: None,
            },
            applicability,
            evidence,
            records,
        )
    }

    /// The layout of one block: every parameter of every group, one record
    /// each, applicable to the programme and marker.
    #[allow(clippy::too_many_arguments)]
    fn add_layout(
        &self,
        block_node: roxmltree::Node<'_, '_>,
        block: &str,
        program: &str,
        marker: &str,
        base: &Applicability,
        evidence: &mut BTreeMap<String, EvidenceRecord>,
        records: &mut BTreeMap<String, KnowledgeRecord>,
    ) -> Result<(), KnowledgeError> {
        for group in block_node
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "group")
        {
            let group_name = require_attribute(group, "name")?.trim().to_string();
            let group_title = child_element(group, "title")
                .and_then(|title| child_element(title, "tm"))
                .and_then(|tm| tm.attribute("id"));
            let (group_en, group_ru) = self.text(group_title);
            for parameter in group
                .children()
                .filter(|node| node.is_element() && node.tag_name().name() == "parameter")
            {
                let id = require_attribute(parameter, "id")?.trim();
                let name = require_attribute(parameter, "name")?.trim().to_string();
                let mask = require_attribute(parameter, "mask")?.trim().to_string();
                let kind = require_attribute(parameter, "type")?.trim().to_string();
                // A span that cannot be read — SDD's X351 document of 2016
                // writes one parameter's id with the stop byte before the
                // start — is a parameter this product cannot place, so it is
                // left unrecorded rather than guessed, and the document's
                // other parameters stand.
                let Some(span) = parse_span(id) else {
                    continue;
                };
                let category = child_element(parameter, "category");
                let flag = |attribute: &str| {
                    category
                        .and_then(|node| node.attribute(attribute))
                        .map(str::trim)
                        .unwrap_or("false")
                        .to_string()
                };
                let scope = category
                    .and_then(|node| node.attribute("scope"))
                    .map(str::trim)
                    .unwrap_or_default()
                    .to_string();
                let title = child_element(parameter, "parameter_title")
                    .and_then(|title| child_element(title, "tm"))
                    .and_then(|tm| tm.attribute("id"));
                let (title_en, title_ru) = self.text(title);
                let mut options = Vec::new();
                if let Some(select) = child_element(parameter, "select") {
                    for option in select
                        .children()
                        .filter(|node| node.is_element() && node.tag_name().name() == "option")
                    {
                        let value = require_attribute(option, "value")?.trim().to_string();
                        let option_name = option
                            .attribute("name")
                            .map(str::trim)
                            .unwrap_or_default()
                            .to_string();
                        let code = option
                            .attribute("code")
                            .map(str::trim)
                            .unwrap_or_default()
                            .to_string();
                        let (option_en, option_ru) = self
                            .text(child_element(option, "tm").and_then(|tm| tm.attribute("id")));
                        options.push(format!(
                            "{}={}={}={}={}",
                            escape(&value),
                            escape(&option_name),
                            escape(&code),
                            escape(&option_en),
                            escape(&option_ru)
                        ));
                    }
                }
                let encoded = format!(
                    "block={};bytes={}..{};bits={}..{};mask={};type={};display={};edit={};scope={};group={};group_title={};group_title_ru={};title={};title_ru={};options={}",
                    escape(block),
                    span.0,
                    span.1,
                    span.2,
                    span.3,
                    escape(&mask),
                    escape(&kind),
                    flag("display"),
                    flag("edit"),
                    escape(&scope),
                    escape(&group_name),
                    escape(&group_en),
                    escape(&group_ru),
                    escape(&title_en),
                    escape(&title_ru),
                    options.join("|")
                );
                // A name reused inside one block is kept apart by its span,
                // so neither row silently overwrites the other. The record
                // id is a digest for the same reason as the identifiers'.
                let mut entity_id = format!("CCF-{}-{}", stable(block), stable(&name));
                let mut record_id = format!(
                    "{}.ccf.p.{}",
                    self.source.id.0,
                    digest(&format!("{block}/{name}"))
                );
                if records.contains_key(&record_id) {
                    let suffix = format!("-{}_{}_{}_{}", span.0, span.1, span.2, span.3);
                    entity_id.push_str(&suffix);
                    record_id = format!(
                        "{}.ccf.p.{}",
                        self.source.id.0,
                        digest(&format!("{block}/{name}{suffix}"))
                    );
                }
                self.push(
                    record_id,
                    format!(
                        "configuration_data/block[@name='{block}']/group[@name='{group_name}']/parameter[@id='{id}']"
                    ),
                    format!("{program} {marker} {block} {name} {id} mask {mask} {kind}"),
                    KnowledgeEntity {
                        kind: EntityKind::ConfigurationParameter,
                        id: entity_id,
                    },
                    ClaimKey::Custom {
                        name: CCF_PARAMETER_CLAIM.into(),
                    },
                    KnowledgeValue::Text { value: encoded },
                    base.clone(),
                    evidence,
                    records,
                )?;
            }
        }
        Ok(())
    }
}

/// Vehicle program and model-year marker of a configuration document, from
/// its qualifiers: exactly one program, and the marker most of the
/// qualifiers name. A document may stamp a few addresses with a neighbouring
/// year (the X250 document of 2013 stamps its reserved block MY12); those
/// addresses keep their own year, the document keeps the majority's.
fn ccf_qualification(root: roxmltree::Node<'_, '_>) -> Result<(String, String), KnowledgeError> {
    let mut programs = BTreeSet::new();
    let mut markers: BTreeMap<String, usize> = BTreeMap::new();
    for qualifier in root
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "qualifier")
    {
        if let Some(model) = qualifier.attribute("model").map(str::trim) {
            if !model.is_empty() {
                programs.insert(model.to_string());
            }
        }
        if let Some(year) = qualifier.attribute("year").map(str::trim) {
            if !year.is_empty() {
                *markers.entry(year.to_string()).or_default() += 1;
            }
        }
    }
    if programs.len() != 1 || markers.is_empty() {
        return Err(KnowledgeError::Parse(format!(
            "<configuration_data> must name exactly one program and at least one model-year marker; found {programs:?} and {:?}",
            markers.keys().collect::<Vec<_>>()
        )));
    }
    let most = markers.values().copied().max().unwrap_or(0);
    let mut leading: Vec<&String> = markers
        .iter()
        .filter(|(_, count)| **count == most)
        .map(|(marker, _)| marker)
        .collect();
    if leading.len() != 1 {
        return Err(KnowledgeError::Parse(format!(
            "<configuration_data> names {:?} equally often; no marker leads",
            leading
        )));
    }
    Ok((
        programs.into_iter().next().expect("checked"),
        leading.remove(0).clone(),
    ))
}

/// `SSS_EEE_bbb_BBB`: the start byte, the stop byte, the first and the last
/// bit of the span.
fn parse_span(id: &str) -> Option<(u32, u32, u32, u32)> {
    let mut parts = id.split('_').map(|part| part.trim().parse::<u32>().ok());
    let span = (
        parts.next()??,
        parts.next()??,
        parts.next()??,
        parts.next()??,
    );
    if parts.next().is_some() || span.1 < span.0 || span.3 < span.2 {
        return None;
    }
    Some(span)
}

/// Twelve hexadecimal characters of the digest of a text: a record id
/// segment that is stable, short and distinct for distinct words.
fn digest(text: &str) -> String {
    knowledge::sha256_bytes(text.as_bytes())[..12].to_string()
}

/// The separators of the encoded layout, escaped inside a text.
fn escape(text: &str) -> String {
    text.replace('%', "%25")
        .replace(';', "%3B")
        .replace('|', "%7C")
        .replace('=', "%3D")
}

/// A stable entity id segment: what SDD wrote, with anything an id may not
/// carry replaced.
fn stable(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

impl IngestionAdapter for CcfAdapter {
    fn parser_id(&self) -> &'static str {
        CCF_PARSER_ID
    }

    fn parser_version(&self) -> &'static str {
        PARSER_VERSION
    }

    fn parse(&self, input: &str) -> Result<IngestionBatch, KnowledgeError> {
        let document = roxmltree::Document::parse(input)
            .map_err(|error| KnowledgeError::Parse(error.to_string()))?;
        let root = document.root_element();
        if root.tag_name().name() != "configuration_data" {
            return Err(KnowledgeError::Parse(format!(
                "expected a <configuration_data> root, found <{}>",
                root.tag_name().name()
            )));
        }
        let (program, marker) = ccf_qualification(root)?;
        let base = self.applicability(&program, &marker)?;
        let mut evidence = BTreeMap::new();
        let mut records = BTreeMap::new();
        let programme = KnowledgeEntity {
            kind: EntityKind::VehicleProgram,
            id: program.clone(),
        };

        // The header: who keeps the configuration and who holds copies.
        let header = child_element(root, "ccf").ok_or_else(|| {
            KnowledgeError::Parse("<configuration_data> has no <ccf> header".into())
        })?;
        let sync = child_text(header, "sync")
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_string);
        let mut modules: Vec<String> = Vec::new();
        for module in sync.iter().cloned().chain(
            header
                .children()
                .filter(|node| node.is_element() && node.tag_name().name() == "source")
                .filter_map(|node| node.text())
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(str::to_string),
        ) {
            if !NOT_MODULES.contains(&module.as_str()) && !modules.contains(&module) {
                modules.push(module);
            }
        }

        // The scheme: one identifier, an offset and a length per block, or
        // VDF paging this product does not read.
        let vdf = root.descendants().any(|node| {
            node.is_element()
                && node.tag_name().name() == "address"
                && (node.has_attribute("vdf_type") || node.has_attribute("start_vdf_block"))
        });
        let scheme = if vdf { "vdf" } else { "did" };
        self.push(
            format!("{}.ccf.scheme", self.source.id.0),
            "configuration_data/block/address".into(),
            format!("{program} {marker}: configuration read scheme {scheme}"),
            programme.clone(),
            ClaimKey::Custom {
                name: CCF_SCHEME_CLAIM.into(),
            },
            KnowledgeValue::Text {
                value: scheme.into(),
            },
            base.clone(),
            &mut evidence,
            &mut records,
        )?;
        for module in &modules {
            let role = if sync.as_deref() == Some(module.as_str()) {
                "sync"
            } else {
                "copy"
            };
            self.push(
                format!("{}.ccf.source.{}", self.source.id.0, slug(module)),
                "configuration_data/ccf".into(),
                format!("{program} {marker}: {module} holds the configuration as {role}"),
                programme.clone(),
                ClaimKey::Custom {
                    name: format!("{CCF_SOURCE_CLAIM_PREFIX}{module}"),
                },
                KnowledgeValue::Text { value: role.into() },
                base.clone(),
                &mut evidence,
                &mut records,
            )?;
        }

        // The blocks: only those read with 0x22; padding names service 0x00.
        let mut seen_blocks = BTreeSet::new();
        for block in root
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "block")
        {
            let name = require_attribute(block, "name")?.trim().to_string();
            let read_service = block.attribute("serviceIdRd").and_then(parse_hex);
            if read_service != Some(READ_SERVICE) {
                continue;
            }
            if !seen_blocks.insert(name.clone()) {
                return Err(KnowledgeError::Parse(format!(
                    "block '{name}' is declared twice"
                )));
            }
            if !vdf {
                for address in block
                    .children()
                    .filter(|node| node.is_element() && node.tag_name().name() == "address")
                {
                    self.add_block_address(
                        address,
                        &name,
                        &program,
                        &marker,
                        &base,
                        &mut evidence,
                        &mut records,
                    )?;
                }
            }
            self.add_layout(
                block,
                &name,
                &program,
                &marker,
                &base,
                &mut evidence,
                &mut records,
            )?;
        }

        // Class and state follow the registered source type, never the
        // parser (ADR-0009).
        let class = evidence_class_for(self.source.source_type);
        let state = validation_state_for(self.source.source_type);
        Ok(IngestionBatch {
            schema_version: knowledge::KNOWLEDGE_SCHEMA_VERSION,
            source: self.source.clone(),
            evidence: evidence
                .into_values()
                .map(|mut record: EvidenceRecord| {
                    record.evidence_class = Some(class);
                    record
                })
                .collect(),
            records: records
                .into_values()
                .map(|mut record: KnowledgeRecord| {
                    record.validation_state = state;
                    record
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_span_is_four_numbers_in_order() {
        assert_eq!(parse_span("003_019_000_135"), Some((3, 19, 0, 135)));
        assert_eq!(parse_span("064_064_006_007"), Some((64, 64, 6, 7)));
        assert_eq!(parse_span("064_063_006_007"), None);
        assert_eq!(parse_span("064_064_007_006"), None);
        assert_eq!(parse_span("064_064_006"), None);
        assert_eq!(parse_span("a_b_c_d"), None);
    }

    #[test]
    fn a_text_lookup_reads_english_and_russian_and_nothing_else() {
        let mut lookup = TextLookup::new();
        lookup
            .insert_from_xml(
                r#"<tm id="@SYNTH_A"><tu xmlns:lang="eng">Alpha</tu><tu xmlns:lang="rus">Альфа</tu><tu xmlns:lang="deu">Alpha (de)</tu></tm>"#,
            )
            .unwrap();
        assert_eq!(lookup.english("@SYNTH_A"), Some("Alpha"));
        assert_eq!(lookup.russian("@SYNTH_A"), Some("Альфа"));
        assert_eq!(lookup.english("@SYNTH_B"), None);
        assert_eq!(lookup.len(), 1);
    }

    #[test]
    fn the_separators_are_escaped_inside_a_text() {
        assert_eq!(escape("a;b|c=d%e"), "a%3Bb%7Cc%3Dd%25e");
        assert_eq!(stable("PARAM_CCF_VIN"), "PARAM_CCF_VIN");
        assert_eq!(stable("ODD NAME/1"), "ODD_NAME_1");
    }
}
