use crate::{evidence_class_for, preferred_segment, qualified_applicability, validation_state_for};
use knowledge::{
    ClaimKey, DimensionConstraint, EntityKind, EvidenceId, EvidenceRecord, IngestionAdapter,
    IngestionBatch, KnowledgeEntity, KnowledgeError, KnowledgeRecord, KnowledgeValue,
    SourceLocator, SourceRecord, ValidationState, KNOWLEDGE_SCHEMA_VERSION,
};
use std::collections::BTreeMap;

pub const IVS_PARSER_ID: &str = "sdd-ivs-part-lineage";
const IVS_PARSER_VERSION: &str = "1.0.0";

/// Claim carrying one assembly's lineage as JLR's IVS catalogue names it
/// (ADR-0033): the assembly part number, the identifier it answers on, the
/// catalogue's own date, and the parts that belong inside it as three
/// parallel lists. An escaped `key=value;…` text, the shape `ADR-0028` uses.
pub const IVS_ASSEMBLY_CLAIM: &str = "sdd_ivs_assembly";

/// Adapter for the SDD `COMMON_JLR_SMPACK_XML/IVS` part-lineage documents.
///
/// # This reads a catalogue; it does not programme anything
///
/// IVS is JLR's in-vehicle software system, and its documents are the source
/// of truth for which software and calibration part numbers belong to which
/// assembly. That is exactly what a read-only product needs to answer "is
/// this the software this unit is supposed to carry" — and nothing more of
/// it is taken. The component also carries service actions and coordinated
/// flash lists, which are programming orchestration: this adapter reads none
/// of them, refuses any document that contains one, and refuses any document
/// SDD does not stamp as production data (`ADR-0033`, decisions 1 and 2).
/// Reflashing stays excluded by `ADR-0005`; this is a comparison of numbers.
#[derive(Clone, Debug)]
pub struct IvsLineageAdapter {
    source: SourceRecord,
}

impl IvsLineageAdapter {
    pub fn new(source: SourceRecord) -> Result<Self, KnowledgeError> {
        source.validate()?;
        Ok(Self { source })
    }

    /// Whether SDD stamps this document as production data that carries no
    /// programming orchestration.
    ///
    /// The stamp is in the document's own header, so this reads the head of
    /// the text rather than parsing tens of megabytes of XML; the search for
    /// service actions is over the whole text, because a refusal must not
    /// depend on where they sit. The exporter asks before handing a document
    /// over, so that SDD's own test files are skipped rather than rejected.
    pub fn is_production(input: &str) -> bool {
        let head = &input[..input.len().min(4096)];
        head.contains("<Environment>Production</Environment>")
            && head.contains("<ValidationStatus>Yes</ValidationStatus>")
            && !input.contains("<ServiceActions>")
            && !input.contains("<CoordinatedFlashList>")
    }
}

impl IngestionAdapter for IvsLineageAdapter {
    fn parser_id(&self) -> &'static str {
        IVS_PARSER_ID
    }

    fn parser_version(&self) -> &'static str {
        IVS_PARSER_VERSION
    }

    fn parse(&self, input: &str) -> Result<IngestionBatch, KnowledgeError> {
        if !Self::is_production(input) {
            return Err(KnowledgeError::Parse(
                "the document is not SDD's production part lineage, or carries service actions; \
                 it is not ingested (ADR-0033)"
                    .into(),
            ));
        }
        let document = roxmltree::Document::parse(input)
            .map_err(|error| KnowledgeError::Parse(error.to_string()))?;
        let root = document.root_element();
        if root.tag_name().name() != "ShowVehicleServiceActions" {
            return Err(KnowledgeError::Parse(format!(
                "expected a <ShowVehicleServiceActions> root, found <{}>",
                root.tag_name().name()
            )));
        }

        let text_of = |node: roxmltree::Node<'_, '_>, name: &str| -> Option<String> {
            node.descendants()
                .find(|child| child.is_element() && child.tag_name().name() == name)
                .and_then(|child| child.text())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        };

        let program = text_of(root, "ProgramCode").ok_or_else(|| {
            KnowledgeError::Parse("the document names no <ProgramCode>".to_string())
        })?;
        let program_slug = preferred_segment(&program, "ProgramCode")?;
        let baseline = text_of(root, "ModelYear").unwrap_or_default();
        // The catalogue ages from its own date, and every comparison made
        // against it says so (ADR-0033, decision 3).
        let dated = text_of(root, "CreationDateTime").unwrap_or_default();

        // One record per assembly: SDD declares the same assembly once per
        // node and the parts differ between them, so they are gathered.
        let mut assemblies: BTreeMap<String, Assembly> = BTreeMap::new();
        for lineage in document
            .descendants()
            .filter(|node| node.is_element() && node.tag_name().name() == "PartLineage")
        {
            let Some(module) = text_of(lineage, "ECUAcronym") else {
                continue;
            };
            let Some(assembly) = text_of(lineage, "AssyPN") else {
                continue;
            };
            let module_slug = preferred_segment(&module, "ECUAcronym")?;
            let key = format!("{module_slug}.{}", part_key(&assembly));
            let entry = assemblies.entry(key).or_insert_with(|| Assembly {
                module,
                assembly: assembly.clone(),
                as_delivered: String::new(),
                parts: Vec::new(),
            });
            if entry.as_delivered.is_empty() {
                if let Some(pid) = text_of(lineage, "AsDeliveredPID") {
                    entry.as_delivered = pid;
                }
            }
            for part in lineage.descendants().filter(|node| {
                node.is_element()
                    && matches!(
                        node.tag_name().name(),
                        "HardwareComponentPart" | "SoftwareComponentPart"
                    )
            }) {
                let hardware = part.tag_name().name() == "HardwareComponentPart";
                let number = if hardware {
                    text_of(part, "HardwareType")
                } else {
                    text_of(part, "FilePN")
                };
                // A part the catalogue does not tie to an identifier cannot
                // be compared with anything a module answers, so it is not
                // recorded. Supporting parts — bootloaders — are such parts,
                // and so are the 2,518 whose identifier is `SECX` or `N/A`:
                // an identifier is four hex digits or it is not one.
                let (Some(number), Some(pid)) = (number, text_of(part, "FilePNPid")) else {
                    continue;
                };
                if pid.len() != 4 || !pid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    continue;
                }
                let kind = if hardware {
                    "Hardware".to_string()
                } else {
                    text_of(part, "PartType").unwrap_or_else(|| "Software".to_string())
                };
                let row = (pid, number, kind);
                if !entry.parts.contains(&row) {
                    entry.parts.push(row);
                }
            }
        }

        let mut evidence = BTreeMap::new();
        let mut records = BTreeMap::new();
        for (key, entry) in assemblies {
            if entry.parts.is_empty() {
                continue;
            }
            let mut applicability = qualified_applicability();
            applicability.vehicle_program = DimensionConstraint::one_of([program.clone()])?;
            applicability.ecu_family = DimensionConstraint::one_of([entry.module.clone()])?;
            applicability.validate()?;

            let entity = KnowledgeEntity {
                kind: EntityKind::ModuleAssembly,
                id: format!(
                    "IVS-{program_slug}-{}-{}",
                    preferred_segment(&entry.module, "ECUAcronym")?,
                    part_key(&entry.assembly)
                ),
            };
            entity.validate()?;

            let record_id = format!("{}.ivs.{program_slug}.{key}", self.source.id.0);
            let evidence_id = format!("{}.ev.{record_id}", self.source.id.0);
            evidence.insert(
                evidence_id.clone(),
                EvidenceRecord {
                    id: EvidenceId::new(evidence_id.clone())?,
                    source_id: self.source.id.clone(),
                    evidence_class: None,
                    locator: SourceLocator {
                        description: format!("PartLineage[@AssyPN='{}']", entry.assembly),
                        document_page: None,
                        document_section: Some("VehicleServiceActionDetail".into()),
                        record_key: Some(format!("{program}/{}/{}", entry.module, entry.assembly)),
                        capture_timestamp_us: None,
                    },
                    excerpt: Some(truncate(&format!(
                        "{} {} carries {} part(s)",
                        entry.module,
                        entry.assembly,
                        entry.parts.len()
                    ))),
                    notes: None,
                },
            );

            let pids = join(entry.parts.iter().map(|(pid, _, _)| pid.as_str()));
            let parts = join(entry.parts.iter().map(|(_, number, _)| number.as_str()));
            let types = join(entry.parts.iter().map(|(_, _, kind)| kind.as_str()));
            let value = format!(
                "assembly={};as_delivered={};baseline={};dated={};pids={pids};parts={parts};types={types}",
                escape(&entry.assembly),
                escape(&entry.as_delivered),
                escape(&baseline),
                escape(&dated),
            );

            records.insert(
                record_id.clone(),
                KnowledgeRecord {
                    id: record_id,
                    entity,
                    key: ClaimKey::Custom {
                        name: IVS_ASSEMBLY_CLAIM.to_string(),
                    },
                    value: KnowledgeValue::Text { value },
                    applicability,
                    evidence_ids: vec![EvidenceId::new(evidence_id)?],
                    // Overwritten once the source type is known.
                    validation_state: ValidationState::Unverified,
                },
            );
        }

        if records.is_empty() {
            return Err(KnowledgeError::Parse(format!(
                "no part lineage was found for {program}"
            )));
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

/// One assembly and the parts the catalogue names inside it.
struct Assembly {
    module: String,
    assembly: String,
    as_delivered: String,
    /// `(identifier, part number, part type)`, in the catalogue's order.
    parts: Vec<(String, String, String)>,
}

/// A part number as a record-id segment: the normalised form of `ADR-0033`,
/// which is also what a comparison is made on.
fn part_key(value: &str) -> String {
    let key = knowledge::part_number::normalised(value);
    if key.is_empty() {
        "unnamed".to_string()
    } else {
        key
    }
}

fn join<'a>(values: impl Iterator<Item = &'a str>) -> String {
    values.map(escape).collect::<Vec<_>>().join("|")
}

/// The escaping of `ADR-0028`, so a value carrying a separator survives.
fn escape(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace(';', "%3B")
        .replace('|', "%7C")
        .replace('=', "%3D")
}

fn truncate(value: &str) -> String {
    value.chars().take(500).collect()
}
