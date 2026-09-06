//! SDD's multilingual text database, as far as it names modules.
//!
//! `CURRENT_PAG_MCP_TEXT_XML/Xml/Text/@J/` holds one small document per text
//! item: `<tm id="@J_14229_M_DESC_PCM">` with a `<tu>` per language, tagged
//! `xmlns:lang="eng"`, `"rus"`, `"deu"`, … (twelve languages; no Ukrainian).
//! Two families describe modules: `@J_14229_M_DESC_<mnemonic>` for the
//! ISO 14229 era and `@J_M_DESC_<mnemonic>` for the earlier one. The
//! mnemonic is the ECU family the platform documents use, so each language's
//! wording becomes a claim on that family. A third family,
//! `@J_I_ISO15031_FAULT_TYPE_<hex>`, words the failure type byte of a fault
//! code; its suffix is the byte in hexadecimal (`17` is 0x17, "circuit
//! voltage above threshold"), where `dtcFaultTypes.xml` counts in decimal, so
//! the claim lands on the same `FTB-<decimal>` entity. Nothing else in the
//! text database is read here.

use crate::dtc_index::{finish, truncate};
use crate::require_attribute;
use knowledge::{
    Applicability, ClaimKey, EntityKind, EvidenceId, EvidenceRecord, IngestionAdapter,
    IngestionBatch, KnowledgeEntity, KnowledgeError, KnowledgeRecord, KnowledgeValue,
    SourceLocator, SourceRecord, ValidationState,
};
use std::collections::BTreeMap;

pub const MODULE_TEXT_PARSER_ID: &str = "sdd-module-text";
const PARSER_VERSION: &str = "1.0.0";

/// Claim key prefix for a module's name in one language: the ISO 639-2 code
/// SDD tags the text with follows the dot (`sdd_module_name.eng`).
pub const MODULE_NAME_CLAIM_PREFIX: &str = "sdd_module_name.";
/// The same for the earlier text family, kept apart so the ISO 14229 wording
/// is preferred where both exist and neither is silently replaced.
pub const LEGACY_MODULE_NAME_CLAIM_PREFIX: &str = "sdd_module_name_legacy.";
/// Claim key prefix for the wording of a failure type byte in one language,
/// on the `FTB-<decimal>` entity `dtcFaultTypes.xml` also describes.
pub const FAILURE_TYPE_NAME_CLAIM_PREFIX: &str = "sdd_failure_type.";

const ISO14229_PREFIX: &str = "@J_14229_M_DESC_";
const LEGACY_PREFIX: &str = "@J_M_DESC_";
const FAULT_TYPE_PREFIX: &str = "@J_I_ISO15031_FAULT_TYPE_";

/// Adapter for one module-description text document.
#[derive(Clone, Debug)]
pub struct ModuleTextAdapter {
    source: SourceRecord,
}

impl ModuleTextAdapter {
    pub fn new(source: SourceRecord) -> Result<Self, KnowledgeError> {
        source.validate()?;
        Ok(Self { source })
    }
}

/// Whether a text document id names a module description this adapter reads.
pub fn is_module_description_id(id: &str) -> bool {
    id.starts_with(ISO14229_PREFIX) || id.starts_with(LEGACY_PREFIX)
}

/// Whether a text document id names a failure type wording this adapter reads.
pub fn is_failure_type_id(id: &str) -> bool {
    id.starts_with(FAULT_TYPE_PREFIX)
}

/// Whether a text document id is one of the items this adapter reads.
pub fn is_text_item_id(id: &str) -> bool {
    is_module_description_id(id) || is_failure_type_id(id)
}

impl IngestionAdapter for ModuleTextAdapter {
    fn parser_id(&self) -> &'static str {
        MODULE_TEXT_PARSER_ID
    }

    fn parser_version(&self) -> &'static str {
        PARSER_VERSION
    }

    fn parse(&self, input: &str) -> Result<IngestionBatch, KnowledgeError> {
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
        let id = require_attribute(root, "id")?;
        let (subject, prefix, entity, what) = if let Some(rest) = id.strip_prefix(ISO14229_PREFIX) {
            let mnemonic = module_mnemonic(id, rest)?;
            (
                mnemonic.clone(),
                MODULE_NAME_CLAIM_PREFIX,
                KnowledgeEntity {
                    kind: EntityKind::EcuFamily,
                    id: mnemonic,
                },
                "module_name",
            )
        } else if let Some(rest) = id.strip_prefix(LEGACY_PREFIX) {
            let mnemonic = module_mnemonic(id, rest)?;
            (
                mnemonic.clone(),
                LEGACY_MODULE_NAME_CLAIM_PREFIX,
                KnowledgeEntity {
                    kind: EntityKind::EcuFamily,
                    id: mnemonic,
                },
                "module_name",
            )
        } else if let Some(rest) = id.strip_prefix(FAULT_TYPE_PREFIX) {
            // The suffix is the failure type byte in hexadecimal.
            let byte = u8::from_str_radix(rest.trim(), 16).map_err(|_| {
                KnowledgeError::Parse(format!(
                    "text item '{id}' does not name a failure type byte in hexadecimal"
                ))
            })?;
            (
                byte.to_string(),
                FAILURE_TYPE_NAME_CLAIM_PREFIX,
                KnowledgeEntity {
                    kind: EntityKind::DiagnosticTroubleCode,
                    id: format!("FTB-{byte}"),
                },
                "failure_type",
            )
        } else {
            return Err(KnowledgeError::Parse(format!(
                "text item '{id}' is not a module description or a failure type"
            )));
        };
        entity.validate()?;

        let mut evidence = BTreeMap::new();
        let mut records = BTreeMap::new();
        for unit in root
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "tu")
        {
            let language = require_attribute(unit, "lang")?.trim().to_ascii_lowercase();
            if language.is_empty() || !language.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
                return Err(KnowledgeError::Parse(format!(
                    "text item '{id}' carries a language tag that is not a plain code"
                )));
            }
            let Some(text) = unit.text().map(str::trim).filter(|text| !text.is_empty()) else {
                continue;
            };
            let record_id = format!("{}.{what}.{subject}.{language}", self.source.id.0);
            let evidence_id = format!("{}.ev.{record_id}", self.source.id.0);
            evidence.insert(
                evidence_id.clone(),
                EvidenceRecord {
                    id: EvidenceId::new(evidence_id.clone())?,
                    source_id: self.source.id.clone(),
                    evidence_class: None,
                    locator: SourceLocator {
                        description: format!("tm[@id='{id}']/tu[@lang='{language}']"),
                        document_page: None,
                        document_section: Some("tm".into()),
                        record_key: Some(subject.clone()),
                        capture_timestamp_us: None,
                    },
                    excerpt: Some(truncate(&format!("{subject} ({language}): {text}"))),
                    notes: None,
                },
            );
            records.insert(
                record_id.clone(),
                KnowledgeRecord {
                    id: record_id,
                    entity: entity.clone(),
                    key: ClaimKey::Custom {
                        name: format!("{prefix}{language}"),
                    },
                    value: KnowledgeValue::Text {
                        value: text.to_string(),
                    },
                    // The name of a module family holds wherever the family
                    // does; nothing here is vehicle-specific.
                    applicability: Applicability::default(),
                    evidence_ids: vec![EvidenceId::new(evidence_id)?],
                    validation_state: ValidationState::Unverified,
                },
            );
        }
        finish(&self.source, evidence, records, "text items")
    }
}

fn module_mnemonic(id: &str, rest: &str) -> Result<String, KnowledgeError> {
    let mnemonic = rest.trim();
    if mnemonic.is_empty() {
        return Err(KnowledgeError::Parse(format!(
            "text item '{id}' names no module"
        )));
    }
    Ok(mnemonic.to_string())
}
