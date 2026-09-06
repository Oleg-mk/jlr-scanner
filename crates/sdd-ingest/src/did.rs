use crate::{
    child_element, child_text, evidence_class_for, qualified_applicability, require_attribute,
    slugify, validation_state_for, ConverterCatalogue, ConverterInfo, ModelYearTimeline,
};
use knowledge::{
    Applicability, ClaimKey, DimensionConstraint, EntityKind, EvidenceId, EvidenceRecord,
    IngestionAdapter, IngestionBatch, KnowledgeEntity, KnowledgeError, KnowledgeRecord,
    KnowledgeValue, SourceLocator, SourceRecord, ValidationState, YearConstraint,
    KNOWLEDGE_SCHEMA_VERSION,
};
use std::collections::BTreeMap;

pub const DID_FORMATTING_PARSER_ID: &str = "sdd-did-formatting";
const DID_FORMATTING_PARSER_VERSION: &str = "1.0.0";

/// Custom applicability dimension holding an SDD model-year breakpoint.
///
/// SDD qualifies data by a breakpoint such as `MY10` rather than by a year
/// range. Whether a breakpoint means "this model year" or "this model year and
/// later" is not stated in the published data, so the marker is recorded
/// verbatim on its own dimension and `model_year` stays unknown. Establishing
/// breakpoint semantics is a separate evidence question.
pub const YEAR_BREAKPOINT_DIMENSION: &str = "sdd_year_breakpoint";

/// Adapter for the SDD DID formatting catalogues under `gradex/Snapshot`.
///
/// Every parameter in these files is a `ReadParameter`, so the corpus is
/// read-only by construction and is stage-1 material by nature rather than by
/// filtering. Any other parameter element is rejected instead of ignored.
#[derive(Clone, Debug)]
pub struct DidFormattingAdapter {
    source: SourceRecord,
    converters: ConverterCatalogue,
    timeline: Option<ModelYearTimeline>,
}

impl DidFormattingAdapter {
    pub fn new(
        source: SourceRecord,
        converters: ConverterCatalogue,
    ) -> Result<Self, KnowledgeError> {
        source.validate()?;
        Ok(Self {
            source,
            converters,
            timeline: None,
        })
    }

    /// Opt in to deriving calendar year ranges from breakpoints.
    ///
    /// See `ADR-0011`. Without this the marker is recorded verbatim and
    /// `model_year` stays unknown.
    pub fn with_timeline(mut self, timeline: ModelYearTimeline) -> Self {
        self.timeline = Some(timeline);
        self
    }
}

impl IngestionAdapter for DidFormattingAdapter {
    fn parser_id(&self) -> &'static str {
        DID_FORMATTING_PARSER_ID
    }

    fn parser_version(&self) -> &'static str {
        DID_FORMATTING_PARSER_VERSION
    }

    fn parse(&self, input: &str) -> Result<IngestionBatch, KnowledgeError> {
        let document = roxmltree::Document::parse(input)
            .map_err(|error| KnowledgeError::Parse(error.to_string()))?;
        let component = document.root_element();
        if component.tag_name().name() != "COMPONENT" {
            return Err(KnowledgeError::Parse(format!(
                "expected a <COMPONENT> root, found <{}>",
                component.tag_name().name()
            )));
        }

        let mut evidence = BTreeMap::new();
        let mut records = BTreeMap::new();
        for container in component.children().filter(|node| {
            node.is_element() && node.tag_name().name() == "KeyedDataRecordContainer"
        }) {
            for keyed in container
                .children()
                .filter(|node| node.is_element() && node.tag_name().name() == "KeyedData")
            {
                self.add_keyed_data(keyed, &mut evidence, &mut records)?;
            }
        }
        if records.is_empty() {
            return Err(KnowledgeError::Parse(
                "no DID parameter definitions were found".into(),
            ));
        }

        let class = evidence_class_for(self.source.source_type);
        let state = validation_state_for(self.source.source_type);
        Ok(IngestionBatch {
            schema_version: KNOWLEDGE_SCHEMA_VERSION,
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

impl DidFormattingAdapter {
    fn add_keyed_data(
        &self,
        keyed: roxmltree::Node<'_, '_>,
        evidence: &mut BTreeMap<String, EvidenceRecord>,
        records: &mut BTreeMap<String, KnowledgeRecord>,
    ) -> Result<(), KnowledgeError> {
        let keyed_id = require_attribute(keyed, "id")?;
        let properties = child_element(keyed, "Properties").ok_or_else(|| {
            KnowledgeError::Parse(format!("<KeyedData id='{keyed_id}'> has no <Properties>"))
        })?;

        let key_size = child_text(properties, "keySize")
            .ok_or_else(|| {
                KnowledgeError::Parse(format!("<KeyedData id='{keyed_id}'> has no keySize"))
            })?
            .parse::<u32>()
            .map_err(|_| {
                KnowledgeError::Parse(format!(
                    "<KeyedData id='{keyed_id}'> has a non-numeric keySize"
                ))
            })?;
        let identifier = format!("0x{key_size:04X}");
        verify_identifier_matches_id(keyed_id, key_size, &identifier)?;

        let data_size = child_text(properties, "dataSize")
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|size| *size > 0);

        let applicability = match child_element(properties, "enable") {
            Some(enable) => applicability_from_enable(enable, keyed_id, self.timeline.as_ref())?,
            None => Applicability::default(),
        };

        let keyed_slug = slugify(keyed_id, "KeyedData id")?;
        let entity = KnowledgeEntity {
            kind: EntityKind::IdentifierParameter,
            id: format!("DID-{identifier}"),
        };

        // Anything that is not a ReadParameter would be a non-read access path.
        // Rejecting rather than skipping keeps the read-only guarantee honest.
        for child in keyed.children().filter(|node| node.is_element()) {
            let name = child.tag_name().name();
            if name != "Properties" && name != "ReadParameter" {
                return Err(KnowledgeError::Parse(format!(
                    "<KeyedData id='{keyed_id}'> contains unsupported non-read element <{name}>"
                )));
            }
        }

        for parameter in keyed
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "ReadParameter")
        {
            let parameter_id = require_attribute(parameter, "id")?;
            let name = child_element(parameter, "Properties")
                .and_then(|properties| child_text(properties, "name"))
                .ok_or_else(|| {
                    KnowledgeError::Parse(format!(
                        "ReadParameter {parameter_id} of '{keyed_id}' has no name"
                    ))
                })?;
            let converter_id = child_element(parameter, "SHORTCUT")
                .and_then(|shortcut| shortcut.attribute("id"))
                .map(str::trim)
                .filter(|value| !value.is_empty());

            let record_id = format!(
                "{}.did.{keyed_slug}.p{}",
                self.source.id.0,
                slugify(parameter_id, "ReadParameter id")?
            );
            let evidence_id = format!("{}.ev.{record_id}", self.source.id.0);

            let converter = converter_id.and_then(|id| self.converters.get(id));
            let unit = converter.and_then(|info| info.out_unit.clone());
            let encoding = encoding_descriptor(parameter, data_size, converter_id, converter);

            insert_unique(
                evidence,
                evidence_id.clone(),
                EvidenceRecord {
                    id: EvidenceId::new(evidence_id.clone())?,
                    source_id: self.source.id.clone(),
                    evidence_class: None,
                    locator: SourceLocator {
                        description: format!(
                            "KeyedData[@id='{keyed_id}']/ReadParameter[@id='{parameter_id}']"
                        ),
                        document_page: None,
                        document_section: Some("Snapshot DID formatting".into()),
                        record_key: Some(keyed_id.to_string()),
                        capture_timestamp_us: None,
                    },
                    excerpt: Some(truncate(&format!("{identifier} {parameter_id}: {name}"))),
                    notes: None,
                },
                "evidence",
            )?;
            insert_unique(
                records,
                record_id.clone(),
                KnowledgeRecord {
                    id: record_id,
                    entity: entity.clone(),
                    key: ClaimKey::ParameterDefinition {
                        parameter: name.to_string(),
                    },
                    value: KnowledgeValue::IdentifierDefinition {
                        identifier: identifier.clone(),
                        encoding: Some(encoding),
                        unit,
                    },
                    applicability: applicability.clone(),
                    evidence_ids: vec![EvidenceId::new(evidence_id)?],
                    // Overwritten once the source type is known.
                    validation_state: ValidationState::Unverified,
                },
                "knowledge record",
            )?;
        }
        Ok(())
    }
}

/// Compact, deterministic description of where a parameter sits in the payload.
fn encoding_descriptor(
    parameter: roxmltree::Node<'_, '_>,
    data_size: Option<i64>,
    converter_id: Option<&str>,
    converter: Option<&ConverterInfo>,
) -> String {
    let mut parts = Vec::new();
    if let (Some(lsb), Some(msb)) = (
        parameter.attribute("lsbNumber"),
        parameter.attribute("msbNumber"),
    ) {
        parts.push(format!("bytes={}..{}", lsb.trim(), msb.trim()));
    }
    if let Some(size) = data_size {
        parts.push(format!("size={size}"));
    }
    if let Some(mask) = parameter.attribute("mask").map(str::trim) {
        // SDD writes a full-width mask as the signed value -1.
        match mask.parse::<i64>() {
            Ok(-1) => parts.push("mask=all".into()),
            Ok(value) if value >= 0 => parts.push(format!("mask=0x{value:x}")),
            _ => parts.push(format!("mask={mask}")),
        }
    }
    if let Some(converter) = converter_id {
        parts.push(format!("converter={converter}"));
    }
    // The converter's arithmetic, verbatim, so a value can be presented as
    // SDD presents it (ADR-0014 M1). Nothing is derived here.
    if let Some(info) = converter {
        if !info.map_points.is_empty() {
            let points: Vec<String> = info
                .map_points
                .iter()
                .map(|(x, y)| format!("{x}:{y}"))
                .collect();
            parts.push(format!("map={}", points.join("|")));
        } else if let Some(multiplier) = &info.multiplier {
            parts.push(format!("scale={multiplier}"));
            if let Some(offset) = &info.offset {
                parts.push(format!("offset={offset}"));
            }
            if let Some(offset_first) = info.offset_first {
                parts.push(format!("offset_first={offset_first}"));
            }
        }
        if !info.states.is_empty() {
            let states: Vec<String> = info
                .states
                .iter()
                .map(|state| {
                    format!(
                        "{}..{}={}",
                        state.low,
                        state.high,
                        escape_state_name(&state.name)
                    )
                })
                .collect();
            parts.push(format!("states={}", states.join("|")));
        }
    }
    parts.join(";")
}

/// Keep a state name from breaking the descriptor's own separators. The
/// decoder reverses exactly these four replacements.
fn escape_state_name(name: &str) -> String {
    name.replace('%', "%25")
        .replace(';', "%3B")
        .replace('|', "%7C")
        .replace('=', "%3D")
}

/// Build applicability from an SDD `enable` expression.
///
/// Only a conjunction of string equality tests is understood. Any other
/// operator or an unrecognised tactic is rejected, because silently ignoring a
/// qualification would widen applicability beyond what the source states.
fn applicability_from_enable(
    enable: roxmltree::Node<'_, '_>,
    keyed_id: &str,
    timeline: Option<&ModelYearTimeline>,
) -> Result<Applicability, KnowledgeError> {
    let mut applicability = qualified_applicability();
    let mut module = None;
    let mut model = None;
    let mut powertrain = None;
    let mut variant = None;
    let mut breakpoint = None;

    for node in enable.descendants().filter(|node| node.is_element()) {
        match node.tag_name().name() {
            "OR" | "NOT" | "XOR" => {
                return Err(KnowledgeError::Parse(format!(
                    "<KeyedData id='{keyed_id}'> uses an unsupported <{}> qualification",
                    node.tag_name().name()
                )))
            }
            "STRING-TEST-TACTIC" => {
                let tactic = require_attribute(node, "id")?;
                let value = child_element(node, "Properties")
                    .and_then(|properties| child_text(properties, "string2"))
                    .ok_or_else(|| {
                        KnowledgeError::Parse(format!(
                            "tactic '{tactic}' of '{keyed_id}' has no comparison value"
                        ))
                    })?;
                let slot = match tactic {
                    "module" => &mut module,
                    "model" => &mut model,
                    "type" => &mut powertrain,
                    "subType" => &mut variant,
                    "year" => &mut breakpoint,
                    other => {
                        return Err(KnowledgeError::Parse(format!(
                        "<KeyedData id='{keyed_id}'> uses an unrecognised qualification '{other}'"
                    )))
                    }
                };
                match slot {
                    Some(existing) if existing == &value => {}
                    Some(_) => {
                        return Err(KnowledgeError::Parse(format!(
                            "<KeyedData id='{keyed_id}'> qualifies '{tactic}' twice with different values"
                        )))
                    }
                    None => *slot = Some(value),
                }
            }
            _ => {}
        }
    }

    if let Some(value) = module {
        applicability.ecu_family = DimensionConstraint::one_of([value.to_string()])?;
    }
    if let Some(value) = model {
        applicability.vehicle_program = DimensionConstraint::one_of([value.to_string()])?;
    }
    if let Some(value) = powertrain {
        applicability.powertrain = DimensionConstraint::one_of([value.to_string()])?;
    }
    if let Some(value) = variant {
        applicability.variant = DimensionConstraint::one_of([value.to_string()])?;
    }
    if let Some(value) = breakpoint {
        applicability.other.insert(
            YEAR_BREAKPOINT_DIMENSION.to_string(),
            DimensionConstraint::one_of([value.to_string()])?,
        );
        // The verbatim marker above is always kept; the derived range is an
        // opt-in re-expression of it, so the derivation stays reversible.
        if let (Some(timeline), Some(program)) = (timeline, model) {
            if let Some(range) = timeline.range_for(program, value)? {
                applicability.model_year = range;
            }
        }
    }
    // An enable expression is a predicate: a qualification it does not test is
    // one it does not constrain. With no year test the entry is enabled for
    // every model year, which is a statement, not the absence of one.
    if breakpoint.is_none() {
        applicability.model_year = YearConstraint::Any;
    }
    applicability.validate()?;
    Ok(applicability)
}

/// Cross-check the decimal `keySize` against the hex identifier in the element id.
fn verify_identifier_matches_id(
    keyed_id: &str,
    key_size: u32,
    identifier: &str,
) -> Result<(), KnowledgeError> {
    let Some(token) = keyed_id
        .split_whitespace()
        .find(|token| token.starts_with("0x") || token.starts_with("0X"))
    else {
        return Ok(());
    };
    let declared = u32::from_str_radix(token.trim_start_matches("0x").trim_start_matches("0X"), 16)
        .map_err(|_| {
            KnowledgeError::Parse(format!("'{keyed_id}' has an unparsable identifier token"))
        })?;
    if declared != key_size {
        return Err(KnowledgeError::Parse(format!(
            "'{keyed_id}' declares {token} but keySize is {key_size} ({identifier})"
        )));
    }
    Ok(())
}

fn insert_unique<T: PartialEq>(
    map: &mut BTreeMap<String, T>,
    id: String,
    value: T,
    kind: &str,
) -> Result<(), KnowledgeError> {
    match map.get(&id) {
        Some(existing) if existing == &value => Ok(()),
        Some(_) => Err(KnowledgeError::Parse(format!(
            "conflicting {kind} generated for id {id}"
        ))),
        None => {
            map.insert(id, value);
            Ok(())
        }
    }
}

fn truncate(value: &str) -> String {
    value.chars().take(500).collect()
}
