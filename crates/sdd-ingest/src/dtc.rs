use crate::{
    child_element, child_text, evidence_class_for, preferred_segment, qualified_applicability,
    require_attribute, slugify, validation_state_for, ModelYearTimeline,
};
use knowledge::{
    Applicability, ClaimKey, DimensionConstraint, EntityKind, EvidenceId, EvidenceRecord,
    IngestionAdapter, IngestionBatch, KnowledgeEntity, KnowledgeError, KnowledgeRecord,
    KnowledgeValue, SourceLocator, SourceRecord, ValidationState, KNOWLEDGE_SCHEMA_VERSION,
};
use std::collections::BTreeMap;

pub const DTC_HELP_PARSER_ID: &str = "sdd-dtc-help";
const DTC_HELP_PARSER_VERSION: &str = "1.0.0";

/// Custom applicability dimension holding an SDD model-year designation.
///
/// SDD qualifies a DTC description with a designation such as `MY06`, and
/// enumerates each one separately rather than expressing a range. Half-year
/// designations such as `MY04_5` also occur. Mapping a designation onto a
/// calendar year is a small inference, but it is still an inference, so the
/// designation is recorded verbatim on its own dimension and `model_year` is
/// left unknown. A caller that knows the designation can match on it exactly.
///
/// This is deliberately the same treatment as
/// [`crate::YEAR_BREAKPOINT_DIMENSION`], kept separate because a designation
/// and a breakpoint are different things and must not be conflated.
pub const MODEL_YEAR_DESIGNATION_DIMENSION: &str = "sdd_model_year";

/// Custom applicability dimension holding the SDD DTC type, such as `BASE`.
pub const DTC_TYPE_DIMENSION: &str = "sdd_dtc_type";

/// Custom applicability dimension holding the SDD module data-name variant.
pub const MODULE_DATA_NAME_DIMENSION: &str = "sdd_module_data_name";

/// Custom applicability dimension holding SDD's own fault type for a help
/// screen — the same number the failure-type byte of a fault record carries.
pub const DTC_FAULT_TYPE_DIMENSION: &str = "sdd_fault_type";

/// Claim under which the help screen a car is given for a code is recorded.
/// Its value is the screen's name; the screen's own text is a claim of its
/// own, so a screen forty cars select is written once, not forty times.
pub const DTC_HELP_CLAIM: &str = "sdd_help";

/// Claim prefix under which one help screen's text is recorded, the screen's
/// name following the dot.
pub const DTC_HELP_SCREEN_CLAIM_PREFIX: &str = "sdd_help_screen.";

/// Adapter for a single SDD `rdsDtcHelp0x____.xml` document.
///
/// Each document is self-contained: it carries the fault code, the description
/// texts, and the qualifiers that say which module, model, and model-year
/// designation each description belongs to. Descriptions are recorded per
/// qualifier so applicability stays exact rather than being widened to the
/// union of everything the file mentions.
#[derive(Clone, Debug)]
pub struct DtcHelpAdapter {
    source: SourceRecord,
    timeline: Option<ModelYearTimeline>,
}

impl DtcHelpAdapter {
    pub fn new(source: SourceRecord) -> Result<Self, KnowledgeError> {
        source.validate()?;
        Ok(Self {
            source,
            timeline: None,
        })
    }

    /// Opt in to deriving calendar year ranges from designations. See `ADR-0011`.
    pub fn with_timeline(mut self, timeline: ModelYearTimeline) -> Self {
        self.timeline = Some(timeline);
        self
    }
}

impl IngestionAdapter for DtcHelpAdapter {
    fn parser_id(&self) -> &'static str {
        DTC_HELP_PARSER_ID
    }

    fn parser_version(&self) -> &'static str {
        DTC_HELP_PARSER_VERSION
    }

    fn parse(&self, input: &str) -> Result<IngestionBatch, KnowledgeError> {
        let document = roxmltree::Document::parse(input)
            .map_err(|error| KnowledgeError::Parse(error.to_string()))?;
        let root = document.root_element();
        if root.tag_name().name() != "dtcHelp" {
            return Err(KnowledgeError::Parse(format!(
                "expected a <dtcHelp> root, found <{}>",
                root.tag_name().name()
            )));
        }

        let code = child_text(root, "dtcCode")
            .ok_or_else(|| KnowledgeError::Parse("<dtcHelp> has no <dtcCode>".into()))?;
        let code_slug = slugify(code, "dtcCode")?;

        // Description texts are keyed by id and referenced by the qualifiers.
        let mut descriptions = BTreeMap::new();
        if let Some(list) = child_element(root, "dtcDescriptionList") {
            for entry in list
                .children()
                .filter(|node| node.is_element() && node.tag_name().name() == "dtcDescription")
            {
                let id = require_attribute(entry, "id")?;
                let text = entry.text().map(str::trim).unwrap_or_default();
                if text.is_empty() {
                    continue;
                }
                descriptions.insert(id.to_string(), text.to_string());
            }
        }

        let mut evidence = BTreeMap::new();
        let mut records = BTreeMap::new();
        // The raw code stays verbatim in the evidence locator and excerpt, so a
        // malformed identifier normalises rather than discarding the document.
        let entity = KnowledgeEntity {
            kind: EntityKind::DiagnosticTroubleCode,
            id: format!("DTC-{}", preferred_segment(code, "dtcCode")?),
        };
        entity.validate()?;

        if let Some(qualification) = child_element(root, "dtcDescriptionQualification") {
            for selection in qualification.children().filter(|node| {
                node.is_element() && node.tag_name().name() == "dtcDescriptionSelection"
            }) {
                let description_id = require_attribute(selection, "dtcDescriptionId")?;
                // A qualifier pointing at a description the file does not define
                // would leave a record with no meaning, so it is skipped rather
                // than recorded as an empty claim.
                let Some(text) = descriptions.get(description_id) else {
                    continue;
                };

                for (index, qualifier) in selection
                    .children()
                    .filter(|node| node.is_element() && node.tag_name().name() == "dtcQualifier")
                    .enumerate()
                {
                    let applicability =
                        applicability_from_qualifier(qualifier, self.timeline.as_ref())?;
                    let record_id = format!(
                        "{}.dtc.{code_slug}.{}.{index}",
                        self.source.id.0,
                        slugify(description_id, "dtcDescriptionId")?
                    );
                    let evidence_id = format!("{}.ev.{record_id}", self.source.id.0);

                    evidence.insert(
                        evidence_id.clone(),
                        EvidenceRecord {
                            id: EvidenceId::new(evidence_id.clone())?,
                            source_id: self.source.id.clone(),
                            evidence_class: None,
                            locator: SourceLocator {
                                description: format!(
                                    "dtcDescriptionSelection[@dtcDescriptionId='{description_id}']/dtcQualifier[{index}]"
                                ),
                                document_page: None,
                                document_section: Some("dtcDescriptionQualification".into()),
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
                            entity: entity.clone(),
                            key: ClaimKey::Alias,
                            value: KnowledgeValue::Text {
                                value: text.clone(),
                            },
                            applicability,
                            evidence_ids: vec![EvidenceId::new(evidence_id)?],
                            // Overwritten once the source type is known.
                            validation_state: ValidationState::Unverified,
                        },
                    );
                }
            }
        }

        // ------------------------------------------------------------------
        // The help layer.
        //
        // Beside the descriptions, the document carries screens of help text:
        // possible causes, actions required, monitoring conditions, in SDD's
        // own words. A screen is chosen by model, model year and fault type;
        // it names mnemonics, and the document's own string table holds their
        // text. Two thirds of the corpus's codes carry one.
        //
        // The chain is recorded as it stands — which screen a car is given,
        // and separately what each screen says — because one screen serves
        // dozens of model-and-year pairs and its text would otherwise be
        // written dozens of times.
        let mnemonics = mnemonic_texts(root);
        let screens = help_screens(root);
        let data_names = help_screen_selections(root);
        let mut used_screens: BTreeMap<String, String> = BTreeMap::new();

        if let Some(qualification) = child_element(root, "dtcDescriptionQualification") {
            for selection in qualification.children().filter(|node| {
                node.is_element() && node.tag_name().name() == "dtcDescriptionSelection"
            }) {
                // The fault-type screen is the one that carries causes and
                // actions; the general one stands in when there is none.
                let fault_type = selection.attribute("faultType");
                let Some(data_name_id) = selection
                    .attribute("faultTypeHelpScreenDataNameId")
                    .or_else(|| selection.attribute("helpScreenDataNameId"))
                else {
                    continue;
                };
                let Some(choices) = data_names.get(data_name_id) else {
                    continue;
                };

                for qualifier in selection
                    .children()
                    .filter(|node| node.is_element() && node.tag_name().name() == "dtcQualifier")
                {
                    let (Some(model), Some(year)) =
                        (qualifier.attribute("model"), qualifier.attribute("year"))
                    else {
                        continue;
                    };
                    let Some(screen_id) = choices
                        .iter()
                        .find(|(_, pairs)| {
                            pairs.iter().any(|(pair_model, pair_year)| {
                                pair_model == model && pair_year == year
                            })
                        })
                        .map(|(screen_id, _)| screen_id)
                    else {
                        continue;
                    };
                    let Some((screen_name, items)) = screens.get(screen_id) else {
                        continue;
                    };
                    let text = items
                        .iter()
                        .filter_map(|item| mnemonics.get(item))
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n");
                    // A screen with nothing in it is no help; SDD has many.
                    if text.trim().is_empty() {
                        continue;
                    }

                    let mut applicability =
                        applicability_from_qualifier(qualifier, self.timeline.as_ref())?;
                    if let Some(fault_type) = fault_type {
                        applicability.other.insert(
                            DTC_FAULT_TYPE_DIMENSION.to_string(),
                            DimensionConstraint::one_of([fault_type.to_string()])?,
                        );
                    }
                    applicability.validate()?;

                    let module = qualifier.attribute("module").unwrap_or("any");
                    let record_id = format!(
                        "{}.dtc.{code_slug}.help.{}",
                        self.source.id.0,
                        slugify(
                            &format!("{module}-{model}-{year}-{}", fault_type.unwrap_or("any")),
                            "help qualifier"
                        )?
                    );
                    let evidence_id = format!(
                        "{}.ev.{}.dtc.{code_slug}.helpdata.{}",
                        self.source.id.0,
                        self.source.id.0,
                        slugify(data_name_id, "helpScreenDataNameId")?
                    );
                    evidence
                        .entry(evidence_id.clone())
                        .or_insert_with(|| EvidenceRecord {
                            id: EvidenceId::new(evidence_id.clone()).expect("a validated id"),
                            source_id: self.source.id.clone(),
                            evidence_class: None,
                            locator: SourceLocator {
                                description: format!(
                                "helpScreenQualification/helpScreenDataName[@id='{data_name_id}']"
                            ),
                                document_page: None,
                                document_section: Some("helpScreenQualification".into()),
                                record_key: Some(code.to_string()),
                                capture_timestamp_us: None,
                            },
                            excerpt: Some(truncate(&format!("{code}: {screen_name}"))),
                            notes: None,
                        });
                    records.insert(
                        record_id.clone(),
                        KnowledgeRecord {
                            id: record_id,
                            entity: entity.clone(),
                            key: ClaimKey::Custom {
                                name: DTC_HELP_CLAIM.to_string(),
                            },
                            value: KnowledgeValue::Text {
                                value: screen_name.clone(),
                            },
                            applicability,
                            evidence_ids: vec![EvidenceId::new(evidence_id)?],
                            validation_state: ValidationState::Unverified,
                        },
                    );
                    used_screens.insert(screen_name.clone(), text);
                }
            }
        }

        // What each selected screen says, written once whatever selects it.
        for (screen_name, text) in used_screens {
            let screen_slug = slugify(&screen_name, "helpScreen name")?;
            let record_id = format!(
                "{}.dtc.{code_slug}.helpscreen.{screen_slug}",
                self.source.id.0
            );
            let evidence_id = format!("{}.ev.{record_id}", self.source.id.0);
            evidence.insert(
                evidence_id.clone(),
                EvidenceRecord {
                    id: EvidenceId::new(evidence_id.clone())?,
                    source_id: self.source.id.clone(),
                    evidence_class: None,
                    locator: SourceLocator {
                        description: format!("helpScreenList/helpScreen[@name='{screen_name}']"),
                        document_page: None,
                        document_section: Some("helpScreenList".into()),
                        record_key: Some(code.to_string()),
                        capture_timestamp_us: None,
                    },
                    excerpt: Some(truncate(&text)),
                    notes: None,
                },
            );
            records.insert(
                record_id.clone(),
                KnowledgeRecord {
                    id: record_id,
                    entity: entity.clone(),
                    key: ClaimKey::Custom {
                        name: format!("{DTC_HELP_SCREEN_CLAIM_PREFIX}{screen_name}"),
                    },
                    value: KnowledgeValue::Text { value: text },
                    // A screen's text is the same wherever it is selected; the
                    // selection above is what a car has to match.
                    applicability: qualified_applicability(),
                    evidence_ids: vec![EvidenceId::new(evidence_id)?],
                    validation_state: ValidationState::Unverified,
                },
            );
        }

        if records.is_empty() {
            return Err(KnowledgeError::Parse(format!(
                "no qualified descriptions were found for {code}"
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

/// The document's own string table: mnemonic name to the text it stands for.
/// The text is CDATA, so every text descendant is taken, not just the first.
fn mnemonic_texts(root: roxmltree::Node<'_, '_>) -> BTreeMap<String, String> {
    let mut texts = BTreeMap::new();
    let Some(list) = child_element(root, "helpScreenMnemonicList") else {
        return texts;
    };
    for entry in list
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == "mnemonic")
    {
        let Some(name) = entry.attribute("name") else {
            continue;
        };
        let text: String = entry
            .descendants()
            .filter(|node| node.is_text())
            .filter_map(|node| node.text())
            .collect();
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        texts.insert(name.to_string(), text.to_string());
    }
    texts
}

/// Screen id to its name and the mnemonics it shows, in the order it shows
/// them: a heading, then what belongs under it.
fn help_screens(root: roxmltree::Node<'_, '_>) -> BTreeMap<String, (String, Vec<String>)> {
    let mut screens = BTreeMap::new();
    let Some(list) = child_element(root, "helpScreenList") else {
        return screens;
    };
    for screen in list
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == "helpScreen")
    {
        let (Some(id), Some(name)) = (screen.attribute("id"), screen.attribute("name")) else {
            continue;
        };
        let items: Vec<String> = screen
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "helpScreenItem")
            .filter_map(|node| node.attribute("mnemonicName").map(str::to_string))
            .collect();
        screens.insert(id.to_string(), (name.to_string(), items));
    }
    screens
}

/// Help-screen data name id to the screens it offers, each with the model and
/// model-year pairs that choose it.
#[allow(clippy::type_complexity)]
fn help_screen_selections(
    root: roxmltree::Node<'_, '_>,
) -> BTreeMap<String, Vec<(String, Vec<(String, String)>)>> {
    let mut data_names = BTreeMap::new();
    let Some(qualification) = child_element(root, "helpScreenQualification") else {
        return data_names;
    };
    for data_name in qualification
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == "helpScreenDataName")
    {
        let Some(id) = data_name.attribute("id") else {
            continue;
        };
        let mut selections = Vec::new();
        for selection in data_name
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "helpScreenSelection")
        {
            let Some(screen_id) = selection.attribute("helpScreenId") else {
                continue;
            };
            let pairs: Vec<(String, String)> = selection
                .children()
                .filter(|node| node.is_element() && node.tag_name().name() == "helpScreenQualifier")
                .filter_map(|node| {
                    Some((
                        node.attribute("model")?.to_string(),
                        node.attribute("year")?.to_string(),
                    ))
                })
                .collect();
            selections.push((screen_id.to_string(), pairs));
        }
        data_names.insert(id.to_string(), selections);
    }
    data_names
}

/// Map one `dtcQualifier` onto applicability.
///
/// An attribute the parser does not understand is rejected rather than dropped,
/// because a silently ignored qualifier would widen the claim beyond its source.
fn applicability_from_qualifier(
    qualifier: roxmltree::Node<'_, '_>,
    timeline: Option<&ModelYearTimeline>,
) -> Result<Applicability, KnowledgeError> {
    let mut applicability = qualified_applicability();
    let mut designation = None;
    let mut program = None;
    for attribute in qualifier.attributes() {
        let value = attribute.value().trim();
        if value.is_empty() {
            continue;
        }
        match attribute.name() {
            "module" => {
                applicability.ecu_family = DimensionConstraint::one_of([value.to_string()])?
            }
            "model" => {
                program = Some(value);
                applicability.vehicle_program = DimensionConstraint::one_of([value.to_string()])?
            }
            "year" => {
                designation = Some(value);
                applicability.other.insert(
                    MODEL_YEAR_DESIGNATION_DIMENSION.to_string(),
                    DimensionConstraint::one_of([value.to_string()])?,
                );
            }
            "dtcType" => {
                applicability.other.insert(
                    DTC_TYPE_DIMENSION.to_string(),
                    DimensionConstraint::one_of([value.to_string()])?,
                );
            }
            "moduleDataName" => {
                applicability.other.insert(
                    MODULE_DATA_NAME_DIMENSION.to_string(),
                    DimensionConstraint::one_of([value.to_string()])?,
                );
            }
            other => {
                return Err(KnowledgeError::Parse(format!(
                    "dtcQualifier carries an unrecognised attribute '{other}'"
                )))
            }
        }
    }
    if applicability == qualified_applicability() {
        return Err(KnowledgeError::Parse(
            "dtcQualifier carries no usable qualification".into(),
        ));
    }
    // The verbatim designation above is always kept; the derived range is an
    // opt-in re-expression of it.
    if let (Some(timeline), Some(program), Some(designation)) = (timeline, program, designation) {
        if let Some(range) = timeline.range_for(program, designation)? {
            applicability.model_year = range;
        }
    }
    applicability.validate()?;
    Ok(applicability)
}

fn truncate(value: &str) -> String {
    value.chars().take(500).collect()
}
