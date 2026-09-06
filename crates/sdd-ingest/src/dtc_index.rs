use crate::{evidence_class_for, preferred_segment, require_attribute, validation_state_for};
use knowledge::{
    Applicability, ClaimKey, DimensionConstraint, EntityKind, EvidenceId, EvidenceRecord,
    IngestionAdapter, IngestionBatch, KnowledgeEntity, KnowledgeError, KnowledgeRecord,
    KnowledgeValue, SourceLocator, SourceRecord, ValidationState, KNOWLEDGE_SCHEMA_VERSION,
};
use std::collections::BTreeMap;

pub const DTC_DESCRIPTION_PARSER_ID: &str = "sdd-dtc-descriptions";
pub const DTC_FAULT_TYPE_PARSER_ID: &str = "sdd-dtc-fault-types";
const INDEX_PARSER_VERSION: &str = "1.0.0";

/// Claim key under which an SDD failure type byte description is recorded.
pub const FAILURE_TYPE_CLAIM: &str = "sdd_failure_type";

/// Adapter for the SDD DTC description indexes.
///
/// One adapter serves both `dtcDescriptions.xml` and
/// `dtcModuleDescriptions.xml`: they share the `dtcDescriptionList` root and
/// differ only in whether each entry carries a `module` attribute. An entry
/// without one is a generic description that names no module, so its ECU family
/// stays unknown and it fails closed rather than claiming to hold everywhere.
///
/// Neither index carries model or model-year qualification. These records are
/// therefore deliberately weaker than the per-code help documents parsed by
/// [`crate::DtcHelpAdapter`], and the store reports the difference as a conflict
/// when the two disagree rather than silently preferring one.
#[derive(Clone, Debug)]
pub struct DtcDescriptionAdapter {
    source: SourceRecord,
}

impl DtcDescriptionAdapter {
    pub fn new(source: SourceRecord) -> Result<Self, KnowledgeError> {
        source.validate()?;
        Ok(Self { source })
    }
}

impl IngestionAdapter for DtcDescriptionAdapter {
    fn parser_id(&self) -> &'static str {
        DTC_DESCRIPTION_PARSER_ID
    }

    fn parser_version(&self) -> &'static str {
        INDEX_PARSER_VERSION
    }

    fn parse(&self, input: &str) -> Result<IngestionBatch, KnowledgeError> {
        let document = roxmltree::Document::parse(input)
            .map_err(|error| KnowledgeError::Parse(error.to_string()))?;
        let root = document.root_element();
        if root.tag_name().name() != "dtcDescriptionList" {
            return Err(KnowledgeError::Parse(format!(
                "expected a <dtcDescriptionList> root, found <{}>",
                root.tag_name().name()
            )));
        }

        let mut evidence = BTreeMap::new();
        let mut records = BTreeMap::new();
        // Descriptions already seen per (code, module), in document order. The
        // real index gives the same pair more than one wording, so occurrences
        // get distinct record ids and the store reports a conflict instead of
        // one row silently replacing the other.
        let mut seen: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for entry in root
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "dtc")
        {
            let code = require_attribute(entry, "id")?;
            let Some(text) = entry.text().map(str::trim).filter(|text| !text.is_empty()) else {
                continue;
            };
            let module = entry
                .attribute("module")
                .map(str::trim)
                .filter(|value| !value.is_empty());

            let code_segment = preferred_segment(code, "dtc id")?;
            let base = match module {
                Some(module) => {
                    format!("{code_segment}.{}", preferred_segment(module, "module")?)
                }
                None => code_segment.clone(),
            };

            let occurrences = seen.entry(base.clone()).or_default();
            if occurrences.iter().any(|existing| existing == text) {
                // An exact repeat carries no new information.
                continue;
            }
            occurrences.push(text.to_string());
            let occurrence = occurrences.len();

            let record_id = if occurrence == 1 {
                format!("{}.dtc.{base}", self.source.id.0)
            } else {
                format!("{}.dtc.{base}.{occurrence}", self.source.id.0)
            };
            let evidence_id = format!("{}.ev.{record_id}", self.source.id.0);

            let mut applicability = Applicability::default();
            if let Some(module) = module {
                applicability.ecu_family = DimensionConstraint::one_of([module.to_string()])?;
            }

            // The raw code stays verbatim in the evidence locator and excerpt.
            // Only the technical identifier is normalised, so one malformed
            // code in the source cannot discard thousands of sound entries.
            let entity = KnowledgeEntity {
                kind: EntityKind::DiagnosticTroubleCode,
                id: format!("DTC-{code_segment}"),
            };
            entity.validate()?;

            evidence.insert(
                evidence_id.clone(),
                EvidenceRecord {
                    id: EvidenceId::new(evidence_id.clone())?,
                    source_id: self.source.id.clone(),
                    evidence_class: None,
                    locator: SourceLocator {
                        description: match module {
                            Some(module) => {
                                format!("dtc[@id='{code}'][@module='{module}']")
                            }
                            None => format!("dtc[@id='{code}']"),
                        },
                        document_page: None,
                        document_section: Some("dtcDescriptionList".into()),
                        record_key: Some(code.to_string()),
                        capture_timestamp_us: None,
                    },
                    excerpt: Some(truncate(&format!("{code}: {text}"))),
                    notes: None,
                },
            );
            records.insert(
                record_id.clone(),
                KnowledgeRecord {
                    id: record_id,
                    entity,
                    key: ClaimKey::Alias,
                    value: KnowledgeValue::Text {
                        value: text.to_string(),
                    },
                    applicability,
                    evidence_ids: vec![EvidenceId::new(evidence_id)?],
                    validation_state: ValidationState::Unverified,
                },
            );
        }

        finish(&self.source, evidence, records, "DTC descriptions")
    }
}

/// Adapter for `dtcFaultTypes.xml`, the SDD failure type byte catalogue.
///
/// A failure type byte is part of DTC vocabulary rather than a separate domain
/// object, so entries share `EntityKind::DiagnosticTroubleCode` and are told
/// apart by an `FTB-` identifier prefix and the [`FAILURE_TYPE_CLAIM`] claim
/// key. The catalogue is vehicle-independent, so applicability stays unknown.
#[derive(Clone, Debug)]
pub struct DtcFaultTypeAdapter {
    source: SourceRecord,
}

impl DtcFaultTypeAdapter {
    pub fn new(source: SourceRecord) -> Result<Self, KnowledgeError> {
        source.validate()?;
        Ok(Self { source })
    }
}

impl IngestionAdapter for DtcFaultTypeAdapter {
    fn parser_id(&self) -> &'static str {
        DTC_FAULT_TYPE_PARSER_ID
    }

    fn parser_version(&self) -> &'static str {
        INDEX_PARSER_VERSION
    }

    fn parse(&self, input: &str) -> Result<IngestionBatch, KnowledgeError> {
        let document = roxmltree::Document::parse(input)
            .map_err(|error| KnowledgeError::Parse(error.to_string()))?;
        let root = document.root_element();
        if root.tag_name().name() != "faultTypeList" {
            return Err(KnowledgeError::Parse(format!(
                "expected a <faultTypeList> root, found <{}>",
                root.tag_name().name()
            )));
        }

        let mut evidence = BTreeMap::new();
        let mut records = BTreeMap::new();
        for entry in root
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "faultType")
        {
            let raw_number = require_attribute(entry, "number")?;
            let number = raw_number.parse::<i32>().map_err(|_| {
                KnowledgeError::Parse(format!("faultType number '{raw_number}' is not an integer"))
            })?;
            let Some(text) = entry.text().map(str::trim).filter(|text| !text.is_empty()) else {
                continue;
            };

            // SDD uses -1 for the SAE reserved range. A bare minus sign would
            // read as a separator in an identifier, so it is spelled out.
            let segment = if number < 0 {
                format!("neg{}", number.unsigned_abs())
            } else {
                number.to_string()
            };
            let record_id = format!("{}.ftb.{segment}", self.source.id.0);
            let evidence_id = format!("{}.ev.{record_id}", self.source.id.0);

            let entity = KnowledgeEntity {
                kind: EntityKind::DiagnosticTroubleCode,
                id: format!("FTB-{segment}"),
            };
            entity.validate()?;

            evidence.insert(
                evidence_id.clone(),
                EvidenceRecord {
                    id: EvidenceId::new(evidence_id.clone())?,
                    source_id: self.source.id.clone(),
                    evidence_class: None,
                    locator: SourceLocator {
                        description: format!("faultType[@number='{raw_number}']"),
                        document_page: None,
                        document_section: Some("faultTypeList".into()),
                        record_key: Some(raw_number.to_string()),
                        capture_timestamp_us: None,
                    },
                    excerpt: Some(truncate(&format!("{raw_number}: {text}"))),
                    notes: None,
                },
            );
            records.insert(
                record_id.clone(),
                KnowledgeRecord {
                    id: record_id,
                    entity,
                    key: ClaimKey::Custom {
                        name: FAILURE_TYPE_CLAIM.to_string(),
                    },
                    value: KnowledgeValue::Text {
                        value: text.to_string(),
                    },
                    applicability: Applicability::default(),
                    evidence_ids: vec![EvidenceId::new(evidence_id)?],
                    validation_state: ValidationState::Unverified,
                },
            );
        }

        finish(&self.source, evidence, records, "fault types")
    }
}

pub(crate) fn finish(
    source: &SourceRecord,
    evidence: BTreeMap<String, EvidenceRecord>,
    records: BTreeMap<String, KnowledgeRecord>,
    what: &str,
) -> Result<IngestionBatch, KnowledgeError> {
    if records.is_empty() {
        return Err(KnowledgeError::Parse(format!("no {what} were found")));
    }
    let class = evidence_class_for(source.source_type);
    let state = validation_state_for(source.source_type);
    Ok(IngestionBatch {
        schema_version: KNOWLEDGE_SCHEMA_VERSION,
        source: source.clone(),
        evidence: evidence
            .into_values()
            .map(|mut record| {
                record.evidence_class = Some(class);
                record
            })
            .collect(),
        records: records
            .into_values()
            .map(|mut record| {
                record.validation_state = state;
                record
            })
            .collect(),
    })
}

pub(crate) fn truncate(value: &str) -> String {
    value.chars().take(500).collect()
}
