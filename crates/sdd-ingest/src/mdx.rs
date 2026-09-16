//! What a module says it will accept, from SDD's own per-module index
//! (`ADR-0035`).
//!
//! `MDX_<ECU>.xml` describes one module on one programme-year. Beside every
//! data identifier it declares how that identifier may be reached — read,
//! written, controlled — naming the service that would carry it, the
//! diagnostic session it requires, and the security level standing in front
//! of it where there is one. Beside those it lists the routines the module
//! declares it can run, each with SDD's own name and number.
//!
//! This adapter takes the halves the read path never needed: the writeable
//! and controllable identifiers, and the routines. They enter as knowledge
//! and nothing sends them — no request constructor exists for `0x2E`,
//! `0x2F` or `0x31`, the architecture guard still fails the build on them,
//! and `SAFETY_BOUNDARIES.md` is unchanged. A declaration here is SDD
//! saying a module accepts something; whether a car answers is a different
//! state, and no car has answered yet.

use crate::platform::{parse_hex, slug};
use crate::{
    child_element, child_text, evidence_class_for, qualified_applicability, validation_state_for,
    ModelYearTimeline, YEAR_BREAKPOINT_DIMENSION,
};
use knowledge::{
    Applicability, ClaimKey, DimensionConstraint, EntityKind, EvidenceId, EvidenceRecord,
    IngestionAdapter, IngestionBatch, KnowledgeEntity, KnowledgeError, KnowledgeRecord,
    KnowledgeValue, SourceLocator, SourceRecord, ValidationState,
};
use roxmltree::Node;
use std::collections::BTreeMap;

pub const MODULE_ACCESS_PARSER_ID: &str = "sdd-module-access";
const PARSER_VERSION: &str = "1.0.0";

/// Namespace of an identifier a module declares writeable — service `0x2E`.
pub const MODULE_WRITE_NAMESPACE: &str = "sdd_module_write";
/// Namespace of an identifier a module declares controllable — service `0x2F`.
pub const MODULE_CONTROL_NAMESPACE: &str = "sdd_module_control";
/// Namespace of a routine a module declares it can run — service `0x31`.
pub const MODULE_ROUTINE_NAMESPACE: &str = "sdd_module_routine";

/// The accesses this adapter takes. The readable half is already in the
/// library from the platform and snapshot catalogues and is not repeated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Access {
    Write,
    Control,
}

impl Access {
    fn element(self) -> &'static str {
        match self {
            Self::Write => "WRITEABLE",
            Self::Control => "CONTROLLABLE",
        }
    }

    fn namespace(self) -> &'static str {
        match self {
            Self::Write => MODULE_WRITE_NAMESPACE,
            Self::Control => MODULE_CONTROL_NAMESPACE,
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Self::Write => "wr",
            Self::Control => "ct",
        }
    }
}

/// One module's index, read for what the module will accept.
///
/// The document names the module and nothing else about the car, so the
/// programme and the model-year marker are given to the adapter: they come
/// from the directory the file sits in, `<PROGRAM>_<YEAR>`.
#[derive(Clone, Debug)]
pub struct ModuleAccessAdapter {
    source: SourceRecord,
    program: String,
    marker: String,
    timeline: Option<ModelYearTimeline>,
}

impl ModuleAccessAdapter {
    pub fn new(
        source: SourceRecord,
        program: impl Into<String>,
        marker: impl Into<String>,
    ) -> Result<Self, KnowledgeError> {
        let program = program.into();
        let marker = marker.into();
        if program.trim().is_empty() || marker.trim().is_empty() {
            return Err(KnowledgeError::InvalidRecord(
                "a module index needs the programme and the model-year marker of its directory"
                    .into(),
            ));
        }
        Ok(Self {
            source,
            program,
            marker,
            timeline: None,
        })
    }

    pub fn with_model_year_timeline(mut self, timeline: ModelYearTimeline) -> Self {
        self.timeline = Some(timeline);
        self
    }

    fn applicability(&self, module: &str) -> Result<Applicability, KnowledgeError> {
        let mut applicability = qualified_applicability();
        applicability.vehicle_program = DimensionConstraint::one_of([self.program.clone()])?;
        applicability.ecu_family = DimensionConstraint::one_of([module.to_string()])?;
        applicability.other.insert(
            YEAR_BREAKPOINT_DIMENSION.to_string(),
            DimensionConstraint::one_of([self.marker.clone()])?,
        );
        if let Some(timeline) = &self.timeline {
            if let Some(range) = timeline.range_for(&self.program, &self.marker)? {
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
        namespace: &str,
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
                    document_section: Some("mdx".into()),
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
                key: ClaimKey::IdentifierDefinition {
                    namespace: namespace.to_string(),
                },
                value,
                applicability,
                evidence_ids: vec![EvidenceId::new(evidence_id)?],
                validation_state: ValidationState::Unverified,
            },
        );
        Ok(())
    }
}

/// `key=value;…`, the shape the rest of the library encodes a definition in.
/// A value carrying a semicolon would break the line, so it loses it: these
/// are SDD's own names, and none of them needs one.
fn encode(fields: &[(&str, String)]) -> String {
    fields
        .iter()
        .filter(|(_, value)| !value.trim().is_empty())
        .map(|(key, value)| format!("{key}={}", value.replace(';', ",").trim()))
        .collect::<Vec<_>>()
        .join(";")
}

/// The sessions an access names, as bare numbers: `session_01 session_03`
/// becomes `01 03`. SDD writes them as references; the number is the
/// diagnostic session of ISO 14229, and that is what a reader wants.
fn sessions(node: Node<'_, '_>) -> String {
    node.attribute("SESSION_REFS")
        .unwrap_or_default()
        .split_whitespace()
        .map(|reference| reference.trim_start_matches("session_"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The security levels standing in front of an access, without SDD's prefix.
fn security(node: Node<'_, '_>) -> String {
    node.attribute("SECURITY_REFS")
        .unwrap_or_default()
        .split_whitespace()
        .map(|reference| reference.trim_start_matches("security_"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The services an access names, as service numbers: `service_2E` is `0x2E`.
fn service(node: Node<'_, '_>) -> String {
    node.attribute("SERVICE_REFS")
        .unwrap_or_default()
        .split_whitespace()
        .map(|reference| {
            format!(
                "0x{}",
                reference.trim_start_matches("service_").to_uppercase()
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl IngestionAdapter for ModuleAccessAdapter {
    fn parser_id(&self) -> &'static str {
        MODULE_ACCESS_PARSER_ID
    }

    fn parser_version(&self) -> &'static str {
        PARSER_VERSION
    }

    fn parse(&self, input: &str) -> Result<IngestionBatch, KnowledgeError> {
        let document = roxmltree::Document::parse(input)
            .map_err(|error| KnowledgeError::Parse(error.to_string()))?;
        let root = document.root_element();
        if root.tag_name().name() != "MDX" {
            return Err(KnowledgeError::Parse(format!(
                "expected an <MDX> root, found <{}>",
                root.tag_name().name()
            )));
        }
        let administration = child_element(root, "ADMINISTRATION")
            .ok_or_else(|| KnowledgeError::Parse("<MDX> has no <ADMINISTRATION>".into()))?;
        let module = child_text(administration, "SHORTNAME")
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| KnowledgeError::Parse("<ADMINISTRATION> names no <SHORTNAME>".into()))?
            .to_string();
        let applicability = self.applicability(&module)?;
        let module_slug = slug(&module);

        let mut evidence = BTreeMap::new();
        let mut records = BTreeMap::new();
        // Every section, not the first of each. Seven documents in the
        // corpus carry a second `<DATA_IDENTIFIERS>` or a second
        // `<ROUTINE_IDENTIFIERS>`, and reading only the first dropped what
        // stood in the others without saying so.
        let sections = |name: &'static str| {
            root.children()
                .filter(|node| node.is_element() && node.tag_name().name() == "ECU_DATA")
                .flat_map(move |data| {
                    data.children()
                        .filter(move |node| node.is_element() && node.tag_name().name() == name)
                })
        };
        if !root
            .children()
            .any(|node| node.tag_name().name() == "ECU_DATA")
        {
            return Err(KnowledgeError::Parse("<MDX> has no <ECU_DATA>".into()));
        }

        // The identifiers a module declares it will take, not the ones it
        // will give: the readable half is already in the library.
        for identifiers in sections("DATA_IDENTIFIERS") {
            for did in identifiers
                .children()
                .filter(|node| node.is_element() && node.tag_name().name() == "DID")
            {
                let Some(parameters) = child_element(did, "ACCESS_PARAMETERS") else {
                    continue;
                };
                let number = child_text(did, "NUMBER").map(str::trim).unwrap_or_default();
                let Some(identifier) = parse_hex(number) else {
                    continue;
                };
                let name = child_text(did, "NAME").map(str::trim).unwrap_or_default();
                let byte_size = child_text(did, "BYTE_SIZE")
                    .map(str::trim)
                    .unwrap_or_default();
                let did_type = child_text(did, "DID_TYPE")
                    .map(str::trim)
                    .unwrap_or_default();
                for access in [Access::Write, Access::Control] {
                    let Some(node) = child_element(parameters, access.element()) else {
                        continue;
                    };
                    let identifier_text = format!("0x{identifier:04X}");
                    let encoding = encode(&[
                        ("name", name.to_string()),
                        ("service", service(node)),
                        ("session", sessions(node)),
                        ("security", security(node)),
                        ("bytes", byte_size.to_string()),
                        ("type", did_type.to_string()),
                        (
                            "iocp",
                            node.attribute("IOCP_REFS").unwrap_or_default().to_string(),
                        ),
                    ]);
                    self.push(
                        format!(
                            "{}.{module_slug}.{}.{identifier:04x}",
                            self.source.id.0,
                            access.slug()
                        ),
                        format!("{module} {} {identifier_text}", access.element()),
                        excerpt(did),
                        KnowledgeEntity {
                            kind: EntityKind::IdentifierParameter,
                            id: format!("DID-{identifier_text}"),
                        },
                        access.namespace(),
                        KnowledgeValue::IdentifierDefinition {
                            identifier: identifier_text,
                            encoding: Some(encoding),
                            unit: None,
                        },
                        applicability.clone(),
                        &mut evidence,
                        &mut records,
                    )?;
                }
            }
        }

        // The routines it declares it can run. A routine carries no security
        // reference anywhere in this corpus: the session is the only gate SDD
        // names beside one, and it is recorded as SDD names it.
        for routines in sections("ROUTINE_IDENTIFIERS") {
            for routine in routines
                .children()
                .filter(|node| node.is_element() && node.tag_name().name() == "ROUTINE")
            {
                let number = child_text(routine, "NUMBER")
                    .map(str::trim)
                    .unwrap_or_default();
                let Some(identifier) = parse_hex(number) else {
                    continue;
                };
                let identifier_text = format!("0x{identifier:04X}");
                let encoding = encode(&[
                    (
                        "name",
                        child_text(routine, "NAME")
                            .map(str::trim)
                            .unwrap_or_default()
                            .to_string(),
                    ),
                    ("service", "0x31".to_string()),
                    ("session", sessions(routine)),
                    (
                        "max_run_time",
                        child_text(routine, "MAX_ROUTINE_RUN_TIME")
                            .map(str::trim)
                            .unwrap_or_default()
                            .to_string(),
                    ),
                    (
                        "restart_while_running",
                        child_text(routine, "RESTART_WHILE_RUNNING")
                            .map(str::trim)
                            .unwrap_or_default()
                            .to_string(),
                    ),
                ]);
                self.push(
                    format!("{}.{module_slug}.ro.{identifier:04x}", self.source.id.0),
                    format!("{module} ROUTINE {identifier_text}"),
                    excerpt(routine),
                    KnowledgeEntity {
                        kind: EntityKind::DiagnosticCapability,
                        id: format!("ROUTINE-{identifier_text}"),
                    },
                    MODULE_ROUTINE_NAMESPACE,
                    KnowledgeValue::IdentifierDefinition {
                        identifier: identifier_text,
                        encoding: Some(encoding),
                        unit: None,
                    },
                    applicability.clone(),
                    &mut evidence,
                    &mut records,
                )?;
            }
        }

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

/// The element as it stands in the document, for the evidence record.
fn excerpt(node: Node<'_, '_>) -> String {
    node.document().input_text()[node.range()]
        .chars()
        .take(500)
        .collect()
}
