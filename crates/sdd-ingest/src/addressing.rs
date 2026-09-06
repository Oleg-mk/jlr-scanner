use crate::{
    child_text, evidence_class_for, require_attribute, slugify, validation_state_for,
    ModelYearTimeline,
};
use knowledge::{
    Applicability, CanIdFormat, ClaimKey, DimensionConstraint, EntityKind, EvidenceId,
    EvidenceRecord, IngestionAdapter, IngestionBatch, KnowledgeEntity, KnowledgeError,
    KnowledgeRecord, KnowledgeValue, SourceLocator, SourceRecord, ValidationState, YearConstraint,
    KNOWLEDGE_SCHEMA_VERSION,
};
use std::collections::BTreeMap;

pub const CAN_LINK_MONITOR_PARSER_ID: &str = "sdd-can-link-monitor";
const CAN_LINK_MONITOR_PARSER_VERSION: &str = "1.0.0";

/// Adapter for the SDD `CANLinkMonitorData.xml` configuration file.
///
/// The file states two independent things: the CAN addressing width used by a
/// vehicle program, and a global map from a diagnostic mnemonic to a module
/// description. Both are recorded exactly as stated. In particular the module
/// table does not say which vehicle a mnemonic applies to, so those records
/// leave `vehicle_program` unknown and fail closed on resolution rather than
/// claiming to hold for every program.
#[derive(Clone, Debug)]
pub struct CanLinkMonitorAdapter {
    source: SourceRecord,
    timeline: Option<ModelYearTimeline>,
}

impl CanLinkMonitorAdapter {
    pub fn new(source: SourceRecord) -> Result<Self, KnowledgeError> {
        source.validate()?;
        Ok(Self {
            source,
            timeline: None,
        })
    }

    /// Opt in to resolving `Pre MY10` / `Post MY10` into year ranges.
    ///
    /// See `ADR-0011`. These markers matter: L319, L320 and L322 use them to
    /// separate 29-bit from 11-bit CAN addressing.
    pub fn with_timeline(mut self, timeline: ModelYearTimeline) -> Self {
        self.timeline = Some(timeline);
        self
    }
}

impl IngestionAdapter for CanLinkMonitorAdapter {
    fn parser_id(&self) -> &'static str {
        CAN_LINK_MONITOR_PARSER_ID
    }

    fn parser_version(&self) -> &'static str {
        CAN_LINK_MONITOR_PARSER_VERSION
    }

    fn parse(&self, input: &str) -> Result<IngestionBatch, KnowledgeError> {
        let document = roxmltree::Document::parse(input)
            .map_err(|error| KnowledgeError::Parse(error.to_string()))?;
        let root = document.root_element();
        if root.tag_name().name() != "configuration" {
            return Err(KnowledgeError::Parse(format!(
                "expected a <configuration> root, found <{}>",
                root.tag_name().name()
            )));
        }

        let mut builder = BatchBuilder::new(&self.source, self.timeline.clone());
        for vehicles in root
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "Vehicles")
        {
            for vehicle in vehicles
                .children()
                .filter(|node| node.is_element() && node.tag_name().name() == "Vehicle")
            {
                for name in vehicle
                    .children()
                    .filter(|node| node.is_element() && node.tag_name().name() == "Name")
                {
                    builder.add_vehicle(name)?;
                }
            }
        }
        for modules in root
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "Modules")
        {
            for module in modules
                .children()
                .filter(|node| node.is_element() && node.tag_name().name() == "Module")
            {
                builder.add_module(module)?;
            }
        }

        builder.finish(&self.source)
    }
}

struct BatchBuilder {
    source_id: String,
    timeline: Option<ModelYearTimeline>,
    evidence: BTreeMap<String, EvidenceRecord>,
    records: BTreeMap<String, KnowledgeRecord>,
    /// Descriptions already seen per mnemonic, in file order.
    ///
    /// The real table reuses a mnemonic for different modules, so occurrences
    /// get distinct record ids while keeping the same entity and claim key.
    /// That is what lets the store report a conflict instead of one row
    /// silently overwriting the other.
    module_descriptions: BTreeMap<String, Vec<String>>,
}

impl BatchBuilder {
    fn new(source: &SourceRecord, timeline: Option<ModelYearTimeline>) -> Self {
        Self {
            source_id: source.id.0.clone(),
            timeline,
            evidence: BTreeMap::new(),
            records: BTreeMap::new(),
            module_descriptions: BTreeMap::new(),
        }
    }

    fn add_vehicle(&mut self, name: roxmltree::Node<'_, '_>) -> Result<(), KnowledgeError> {
        let program = require_attribute(name, "id")?;
        let program_segment = stable_segment(program, "vehicle program")?;
        for model_year in name
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "ModelYear")
        {
            let raw_year = require_attribute(model_year, "id")?;
            let Some(addressing) = child_text(model_year, "AddressingType") else {
                continue;
            };
            let format = parse_addressing_type(addressing)?;
            let (year_segment, mut year_constraint) = parse_model_year(raw_year)?;
            // A prose marker such as "Post MY10" carries its own boundary, so
            // the timeline resolves it without needing this program's sequence.
            if let Some(timeline) = &self.timeline {
                if let Some(range) = timeline.range_for(program, raw_year)? {
                    year_constraint = range;
                }
            }

            let record_id = format!(
                "{}.vehicle.{program_segment}.{year_segment}",
                self.source_id
            );
            let evidence_id = format!("{}.ev.{record_id}", self.source_id);
            self.push(
                &record_id,
                &evidence_id,
                SourceLocator {
                    description: format!(
                        "Vehicles/Vehicle/Name[@id='{program}']/ModelYear[@id='{raw_year}']/AddressingType"
                    ),
                    document_page: None,
                    document_section: Some("Vehicles".into()),
                    record_key: Some(program.to_string()),
                    capture_timestamp_us: None,
                },
                Some(addressing.to_string()),
                KnowledgeEntity {
                    kind: EntityKind::VehicleProgram,
                    id: program.to_string(),
                },
                ClaimKey::DiagnosticAddressing,
                KnowledgeValue::DiagnosticAddressing {
                    request_id: None,
                    response_id: None,
                    functional_request_id: None,
                    can_id_format: Some(format),
                    addressing_mode: Some(addressing.to_string()),
                },
                Applicability {
                    vehicle_program: DimensionConstraint::one_of([program.to_string()])?,
                    model_year: year_constraint,
                    // The addressing width is stated for the program as a whole,
                    // so every other dimension is genuinely unconstrained.
                    architecture_generation: DimensionConstraint::Any,
                    ecu_family: DimensionConstraint::Any,
                    powertrain: DimensionConstraint::Any,
                    variant: DimensionConstraint::Any,
                    market: DimensionConstraint::Any,
                    diagnostic_implementation: DimensionConstraint::Any,
                    other: BTreeMap::new(),
                },
            )?;
        }
        Ok(())
    }

    fn add_module(&mut self, module: roxmltree::Node<'_, '_>) -> Result<(), KnowledgeError> {
        let (Some(mnemonic), Some(description)) = (
            child_text(module, "Mnemonic"),
            child_text(module, "Description"),
        ) else {
            return Ok(());
        };
        let mnemonic = mnemonic.to_ascii_uppercase();
        let mnemonic_segment = stable_segment(&mnemonic, "module mnemonic")?;

        let seen = self
            .module_descriptions
            .entry(mnemonic.clone())
            .or_default();
        if seen.iter().any(|existing| existing == description) {
            // An exact repeat carries no new information.
            return Ok(());
        }
        seen.push(description.to_string());
        let occurrence = seen.len();

        let record_id = if occurrence == 1 {
            format!("{}.module.{mnemonic_segment}", self.source_id)
        } else {
            format!("{}.module.{mnemonic_segment}.{occurrence}", self.source_id)
        };
        let evidence_id = format!("{}.ev.{record_id}", self.source_id);
        self.push(
            &record_id,
            &evidence_id,
            SourceLocator {
                description: format!("Modules/Module[Mnemonic='{mnemonic}']"),
                document_page: None,
                document_section: Some("Modules".into()),
                record_key: Some(mnemonic.clone()),
                capture_timestamp_us: None,
            },
            Some(format!("{mnemonic} = {description}")),
            KnowledgeEntity {
                kind: EntityKind::DiagnosticAddressing,
                id: mnemonic.clone(),
            },
            ClaimKey::Alias,
            KnowledgeValue::Text {
                value: description.to_string(),
            },
            // The table is global. It never states which vehicle program a
            // mnemonic belongs to, so this must not resolve for a specific
            // vehicle on its own.
            Applicability::default(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        record_id: &str,
        evidence_id: &str,
        locator: SourceLocator,
        excerpt: Option<String>,
        entity: KnowledgeEntity,
        key: ClaimKey,
        value: KnowledgeValue,
        applicability: Applicability,
    ) -> Result<(), KnowledgeError> {
        let source_id = knowledge::SourceId::new(self.source_id.clone())?;
        let evidence = EvidenceRecord {
            id: EvidenceId::new(evidence_id.to_string())?,
            source_id,
            evidence_class: None,
            locator,
            excerpt,
            notes: None,
        };
        let record = KnowledgeRecord {
            id: record_id.to_string(),
            entity,
            key,
            value,
            applicability,
            evidence_ids: vec![EvidenceId::new(evidence_id.to_string())?],
            // Overwritten in `finish` once the source type is known.
            validation_state: ValidationState::Unverified,
        };
        insert_unique(
            &mut self.evidence,
            evidence_id.to_string(),
            evidence,
            "evidence",
        )?;
        insert_unique(
            &mut self.records,
            record_id.to_string(),
            record,
            "knowledge record",
        )
    }

    fn finish(self, source: &SourceRecord) -> Result<IngestionBatch, KnowledgeError> {
        if self.records.is_empty() {
            return Err(KnowledgeError::Parse(
                "no vehicle addressing or module entries were found".into(),
            ));
        }
        let class = evidence_class_for(source.source_type);
        let state = validation_state_for(source.source_type);
        let evidence = self
            .evidence
            .into_values()
            .map(|mut record| {
                record.evidence_class = Some(class);
                record
            })
            .collect();
        let records = self
            .records
            .into_values()
            .map(|mut record| {
                record.validation_state = state;
                record
            })
            .collect();
        Ok(IngestionBatch {
            schema_version: KNOWLEDGE_SCHEMA_VERSION,
            source: source.clone(),
            evidence,
            records,
        })
    }
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

fn parse_addressing_type(value: &str) -> Result<CanIdFormat, KnowledgeError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "11bit" => Ok(CanIdFormat::Standard11Bit),
        "29bit" => Ok(CanIdFormat::Extended29Bit),
        other => Err(KnowledgeError::Parse(format!(
            "unsupported AddressingType '{other}'"
        ))),
    }
}

fn parse_model_year(value: &str) -> Result<(String, YearConstraint), KnowledgeError> {
    if value.eq_ignore_ascii_case("ALL") {
        return Ok(("all".into(), YearConstraint::Any));
    }
    match value.parse::<u16>() {
        Ok(year) => Ok((
            year.to_string(),
            YearConstraint::Range {
                model_year_from: Some(year),
                model_year_to: Some(year),
            },
        )),
        // SDD also uses prose markers such as "Post MY10" and "Pre MY10".
        // Whether those include model year 2010 is not stated anywhere in the
        // file, and guessing the boundary would put a wrong year range behind a
        // real vehicle claim. The marker is preserved verbatim in the evidence
        // locator and the constraint stays unknown, so the record is kept but
        // fails closed instead of resolving on an assumption.
        Err(_) => Ok((slugify(value, "model year")?, YearConstraint::Unknown)),
    }
}

fn stable_segment(value: &str, what: &str) -> Result<String, KnowledgeError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || !trimmed
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(KnowledgeError::Parse(format!(
            "{what} '{value}' is not usable as a stable identifier segment"
        )));
    }
    Ok(trimmed.to_string())
}
