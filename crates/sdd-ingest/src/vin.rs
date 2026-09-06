//! SDD's VIN decode tables (`CURRENT_JLR_VIN_DECODE_XML/Xml/VINDecode.xml`).
//!
//! The document has two halves. `<Models>` holds rule blocks — `<VIN
//! DecodeModel="n">` with `<Test CharPos="1,3" Operator="EQUAL"
//! CharValue="SAJ"/>` children — and a VIN belongs to decode model `n` when
//! every test of one of its blocks passes. `<Decodes>` holds, per
//! `<DecodeModel id="n">`, the attributes SDD reads off such a VIN: a
//! constant (`<Attribute Name="Brand" Decode="Jaguar"/>`) or a lookup on
//! given characters (`<Attribute Name="Model" Char="6,7">` with `<Value
//! Value="01" Decode="X200"/>` rows).
//!
//! Both are kept verbatim as text claims on a `VinDecodeModel` entity — the
//! rule as `1..3=SAJ;12!=B`, the attribute as `const=Jaguar` or
//! `chars=6..7;01=X200|02=X200` — so the decoder in `diagnostic-session`
//! applies exactly SDD's tables and nothing is derived at ingest.

use crate::dtc_index::{finish, truncate};
use crate::{child_element, require_attribute};
use knowledge::{
    Applicability, ClaimKey, EntityKind, EvidenceId, EvidenceRecord, IngestionAdapter,
    IngestionBatch, KnowledgeEntity, KnowledgeError, KnowledgeRecord, KnowledgeValue,
    SourceLocator, SourceRecord, ValidationState,
};
use std::collections::BTreeMap;

pub const VIN_DECODE_PARSER_ID: &str = "sdd-vin-decode";
const PARSER_VERSION: &str = "1.0.0";

/// Claim key of a rule block: which VINs belong to the decode model.
pub const VIN_RULE_CLAIM: &str = "sdd_vin_rule";
/// Claim key prefix of an attribute the model decodes; the attribute's name
/// as SDD writes it follows the dot (`sdd_vin_attribute.Model`).
pub const VIN_ATTRIBUTE_CLAIM_PREFIX: &str = "sdd_vin_attribute.";

/// Adapter for `VINDecode.xml`.
#[derive(Clone, Debug)]
pub struct VinDecodeAdapter {
    source: SourceRecord,
}

impl VinDecodeAdapter {
    pub fn new(source: SourceRecord) -> Result<Self, KnowledgeError> {
        source.validate()?;
        Ok(Self { source })
    }
}

fn entity_for(model: u32) -> Result<KnowledgeEntity, KnowledgeError> {
    let entity = KnowledgeEntity {
        kind: EntityKind::VinDecodeModel,
        id: format!("VIN-MODEL-{model}"),
    };
    entity.validate()?;
    Ok(entity)
}

/// `CharPos` is `"10"` or `"1,3"`: one-based, inclusive, within the 17.
fn char_positions(text: &str) -> Result<(usize, usize), KnowledgeError> {
    let parse = |value: &str| {
        value
            .trim()
            .parse::<usize>()
            .map_err(|_| KnowledgeError::Parse(format!("CharPos '{text}' is not a position")))
    };
    let (from, to) = match text.split_once(',') {
        Some((from, to)) => (parse(from)?, parse(to)?),
        None => {
            let single = parse(text)?;
            (single, single)
        }
    };
    if from == 0 || from > to || to > 17 {
        return Err(KnowledgeError::Parse(format!(
            "CharPos '{text}' is outside the 17 VIN positions"
        )));
    }
    Ok((from, to))
}

fn plain_value(value: &str, what: &str) -> Result<String, KnowledgeError> {
    let value = value.trim();
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return Err(KnowledgeError::Parse(format!(
            "{what} '{value}' is not a plain alphanumeric value"
        )));
    }
    Ok(value.to_string())
}

/// Remove a DOCTYPE that declares nothing. A DTD with content is refused,
/// because entities it declared would change the text without being read.
fn without_empty_doctype(input: &str) -> Result<String, KnowledgeError> {
    let Some(start) = input.find("<!DOCTYPE") else {
        return Ok(input.to_string());
    };
    let Some(end) = input[start..].find('>') else {
        return Err(KnowledgeError::Parse("unterminated DOCTYPE".into()));
    };
    let declaration = &input[start..start + end + 1];
    let body = declaration
        .trim_start_matches("<!DOCTYPE")
        .trim_end_matches('>')
        .trim();
    let subset = body.find('[').map(|open| body[open..].trim());
    if matches!(subset, Some(inner) if inner != "[]") {
        return Err(KnowledgeError::Parse(
            "VINDecode carries a DTD with declarations, which is not read".into(),
        ));
    }
    Ok(format!("{}{}", &input[..start], &input[start + end + 1..]))
}

/// Keep a decoded text from breaking the claim's own separators. The decoder
/// reverses exactly these four replacements.
fn escape(text: &str) -> String {
    text.replace('%', "%25")
        .replace(';', "%3B")
        .replace('|', "%7C")
        .replace('=', "%3D")
}

impl IngestionAdapter for VinDecodeAdapter {
    fn parser_id(&self) -> &'static str {
        VIN_DECODE_PARSER_ID
    }

    fn parser_version(&self) -> &'static str {
        PARSER_VERSION
    }

    fn parse(&self, input: &str) -> Result<IngestionBatch, KnowledgeError> {
        // SDD opens the document with `<!DOCTYPE VINDecode []>`, an empty
        // DTD the XML parser refuses on principle; it declares nothing.
        let input = without_empty_doctype(input)?;
        let document = roxmltree::Document::parse(&input)
            .map_err(|error| KnowledgeError::Parse(error.to_string()))?;
        let root = document.root_element();
        if root.tag_name().name() != "VINDecode" {
            return Err(KnowledgeError::Parse(format!(
                "expected a <VINDecode> root, found <{}>",
                root.tag_name().name()
            )));
        }
        let mut evidence = BTreeMap::new();
        let mut records = BTreeMap::new();
        let source_id = self.source.id.0.clone();

        let mut add = |record_id: String,
                       entity: KnowledgeEntity,
                       key: String,
                       value: String,
                       locator: String,
                       excerpt: String,
                       notes: Option<String>|
         -> Result<(), KnowledgeError> {
            let evidence_id = format!("{source_id}.ev.{record_id}");
            evidence.insert(
                evidence_id.clone(),
                EvidenceRecord {
                    id: EvidenceId::new(evidence_id.clone())?,
                    source_id: self.source.id.clone(),
                    evidence_class: None,
                    locator: SourceLocator {
                        description: locator,
                        document_page: None,
                        document_section: Some("VINDecode".into()),
                        record_key: Some(entity.id.clone()),
                        capture_timestamp_us: None,
                    },
                    excerpt: Some(truncate(&excerpt)),
                    notes,
                },
            );
            records.insert(
                record_id.clone(),
                KnowledgeRecord {
                    id: record_id,
                    entity,
                    key: ClaimKey::Custom { name: key },
                    value: KnowledgeValue::Text { value },
                    // VIN rules are programme-independent by nature: they are
                    // how the programme is found.
                    applicability: Applicability::default(),
                    evidence_ids: vec![EvidenceId::new(evidence_id)?],
                    validation_state: ValidationState::Unverified,
                },
            );
            Ok(())
        };

        let models = child_element(root, "Models")
            .ok_or_else(|| KnowledgeError::Parse("VINDecode has no <Models>".into()))?;
        let mut rule_index = 0usize;
        for block in models
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "VIN")
        {
            rule_index += 1;
            let model_text = require_attribute(block, "DecodeModel")?;
            let model = model_text.trim().parse::<u32>().map_err(|_| {
                KnowledgeError::Parse(format!("DecodeModel '{model_text}' is not a number"))
            })?;
            let mut tests = Vec::new();
            for test in block
                .children()
                .filter(|node| node.is_element() && node.tag_name().name() == "Test")
            {
                let (from, to) = char_positions(require_attribute(test, "CharPos")?)?;
                let operator = match require_attribute(test, "Operator")?.trim() {
                    "EQUAL" => "=",
                    "NOT_EQUAL" => "!=",
                    other => {
                        return Err(KnowledgeError::Parse(format!(
                            "VIN test operator '{other}' is not understood"
                        )))
                    }
                };
                let value = plain_value(require_attribute(test, "CharValue")?, "CharValue")?;
                if value.len() != to - from + 1 {
                    return Err(KnowledgeError::Parse(format!(
                        "CharValue '{value}' does not span positions {from}..{to}"
                    )));
                }
                tests.push(format!("{from}..{to}{operator}{value}"));
            }
            if tests.is_empty() {
                continue;
            }
            let text = tests.join(";");
            add(
                format!("{source_id}.vin.rule.{rule_index:03}"),
                entity_for(model)?,
                VIN_RULE_CLAIM.to_string(),
                text.clone(),
                format!("Models/VIN[{rule_index}][@DecodeModel='{model}']"),
                format!("model {model}: {text}"),
                None,
            )?;
        }

        let decodes = child_element(root, "Decodes")
            .ok_or_else(|| KnowledgeError::Parse("VINDecode has no <Decodes>".into()))?;
        for decode in decodes
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "DecodeModel")
        {
            let id_text = require_attribute(decode, "id")?;
            let model = id_text.trim().parse::<u32>().map_err(|_| {
                KnowledgeError::Parse(format!("DecodeModel id '{id_text}' is not a number"))
            })?;
            let version = child_element(decode, "Version")
                .and_then(|node| node.attribute("Ref"))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| format!("SDD table version: {value}"));
            for attribute in decode
                .children()
                .filter(|node| node.is_element() && node.tag_name().name() == "Attribute")
            {
                let name = plain_value(require_attribute(attribute, "Name")?, "Attribute Name")?;
                let text = match attribute.attribute("Char") {
                    Some(chars) => {
                        let (from, to) = char_positions(chars)?;
                        let rows: Vec<String> = attribute
                            .children()
                            .filter(|node| node.is_element() && node.tag_name().name() == "Value")
                            .filter_map(|row| {
                                // A row that decodes to nothing, or whose value
                                // cannot span the characters it is looked up
                                // on, can never answer; it is left out rather
                                // than taken for the whole table's fault.
                                let value = plain_value(row.attribute("Value")?, "Value").ok()?;
                                if value.len() != to - from + 1 {
                                    return None;
                                }
                                let decoded = row.attribute("Decode")?.trim();
                                if decoded.is_empty() {
                                    return None;
                                }
                                Some(format!("{value}={}", escape(decoded)))
                            })
                            .collect();
                        if rows.is_empty() {
                            continue;
                        }
                        format!("chars={from}..{to};{}", rows.join("|"))
                    }
                    None => match attribute.attribute("Decode").map(str::trim) {
                        Some(decoded) if !decoded.is_empty() => {
                            format!("const={}", escape(decoded))
                        }
                        _ => continue,
                    },
                };
                add(
                    format!(
                        "{source_id}.vin.model.{model}.{}",
                        name.to_ascii_lowercase()
                    ),
                    entity_for(model)?,
                    format!("{VIN_ATTRIBUTE_CLAIM_PREFIX}{name}"),
                    text.clone(),
                    format!("Decodes/DecodeModel[@id='{model}']/Attribute[@Name='{name}']"),
                    format!("model {model} {name}: {text}"),
                    version.clone(),
                )?;
            }
        }

        finish(&self.source, evidence, records, "VIN decode rules")
    }
}
