use crate::{
    child_element, evidence_class_for, preferred_segment, qualified_applicability,
    require_attribute, validation_state_for, MODEL_YEAR_DESIGNATION_DIMENSION,
};
use knowledge::{
    Applicability, ClaimKey, DiagnosticSafetyClass, DimensionConstraint, EntityKind, EvidenceId,
    EvidenceRecord, IngestionAdapter, IngestionBatch, KnowledgeEntity, KnowledgeError,
    KnowledgeRecord, KnowledgeValue, SourceLocator, SourceRecord, ValidationState,
    KNOWLEDGE_SCHEMA_VERSION,
};
use std::collections::BTreeMap;

use crate::dtc::{iso_code_for, DTC_HELP_ITEM_SEPARATOR};

pub const ODST_PARSER_ID: &str = "sdd-odst-info";
const ODST_PARSER_VERSION: &str = "1.1.0";

/// Prefix for custom dimensions holding an SDD `qual` qualifier verbatim.
pub const QUAL_DIMENSION_PREFIX: &str = "sdd_qual_";

/// Claim carrying one self test as SDD declares it (ADR-0032): the
/// identifier, SDD's own name, how long the test runs and how long the
/// tester waits. An escaped `key=value;…` text, the shape `ADR-0028` uses.
pub const ODST_TEST_CLAIM: &str = "sdd_odst_test";
/// Claim under which the help screen a car is given for a test is recorded;
/// its value is the screen's name. The screen's own text is a claim of its
/// own, so a screen dozens of cars select is written once (`ADR-0026`'s
/// shape).
pub const ODST_HELP_CLAIM_PREFIX: &str = "sdd_odst_help.";
/// Claim prefix under which one screen's text is recorded, the screen's name
/// following the dot.
pub const ODST_SCREEN_CLAIM_PREFIX: &str = "sdd_odst_screen.";
/// The same screen as the names of its items, one per line — the mnemonic's
/// name, U+001F, its text — so a line can be joined to the same line in
/// another language's pack (`ADR-0034`). A pack in another language writes
/// the two screen claims with its code last, `sdd_odst_screen.<NAME>.rus`.
pub const ODST_SCREEN_ITEMS_CLAIM_PREFIX: &str = "sdd_odst_screen_items.";

/// Adapter for the SDD `rds-odst-info-<MODULE>.xml` documents.
///
/// # These are not read-only operations
///
/// An on-demand self test commands an ECU to run a routine. It is not a read,
/// and it is therefore **stage 2 material** under `docs/ROADMAP.md`, outside the
/// current scope. `ADR-0009` permits such material to be recorded as
/// known-but-unavailable, never as an available operation, and that is exactly
/// what this adapter does: every record it produces carries
/// [`DiagnosticSafetyClass::ServiceRoutine`].
///
/// Nothing can execute these. The application execution surface accepts only
/// typed read-only intent, and the architecture checker rejects any attempt to
/// add another path. Knowing which self tests a module declares is useful for
/// planning stage 2; it does not bring stage 2 forward.
#[derive(Clone, Debug)]
pub struct OdstInfoAdapter {
    source: SourceRecord,
    /// `None` for the English pack, which carries the tests, their
    /// timings and which screen a car is given; a language code for a pack
    /// that carries only the text of the same screens (`ADR-0034`).
    language: Option<String>,
}

impl OdstInfoAdapter {
    pub fn new(source: SourceRecord) -> Result<Self, KnowledgeError> {
        source.validate()?;
        Ok(Self {
            source,
            language: None,
        })
    }

    /// Read this pack as the same screens in another language: only the
    /// screen text claims are written, suffixed with the code, and the
    /// document must declare that language itself (`ADR-0034`).
    pub fn with_language(mut self, language: &str) -> Result<Self, KnowledgeError> {
        if iso_code_for(language).is_none() {
            return Err(KnowledgeError::Parse(format!(
                "no self-test pack is known for language code {language:?}"
            )));
        }
        self.language = Some(language.to_string());
        Ok(self)
    }

    fn language_suffix(&self) -> String {
        self.language
            .as_deref()
            .map(|code| format!(".{code}"))
            .unwrap_or_default()
    }
}

impl IngestionAdapter for OdstInfoAdapter {
    fn parser_id(&self) -> &'static str {
        ODST_PARSER_ID
    }

    fn parser_version(&self) -> &'static str {
        ODST_PARSER_VERSION
    }

    fn parse(&self, input: &str) -> Result<IngestionBatch, KnowledgeError> {
        let document = roxmltree::Document::parse(input)
            .map_err(|error| KnowledgeError::Parse(error.to_string()))?;
        let root = document.root_element();
        if root.tag_name().name() != "odstInfo" {
            return Err(KnowledgeError::Parse(format!(
                "expected an <odstInfo> root, found <{}>",
                root.tag_name().name()
            )));
        }
        let module = require_attribute(root, "moduleType")?;
        let module_slug = preferred_segment(module, "moduleType")?;

        // The Russian pack has the English pack's file names, so a root
        // mounted under the wrong name would write one language's text under
        // the other's claim: the document must say which it is.
        let expected_iso = self
            .language
            .as_deref()
            .and_then(iso_code_for)
            .unwrap_or("en");
        if let Some(declared) =
            child_element(root, "language").and_then(|node| node.attribute("isoCode"))
        {
            if declared != expected_iso {
                return Err(KnowledgeError::Parse(format!(
                    "{module}: the document declares language {declared:?}, this pack is read as {expected_iso:?}"
                )));
            }
        }

        // The document carries its own screens and its own string table, so
        // a test's prose needs no other pack (ADR-0032).
        let mnemonics = mnemonic_texts(root);
        let screens = help_screens(root, &mnemonics);

        // Help-screen data names give each test a stable SDD name.
        let mut test_names = BTreeMap::new();
        if let Some(qualification) = child_element(root, "helpScreenQualification") {
            for data_name in qualification
                .children()
                .filter(|node| node.is_element() && node.tag_name().name() == "helpScreenDataName")
            {
                if let (Ok(id), Some(name)) = (
                    require_attribute(data_name, "id"),
                    data_name.attribute("name").map(str::trim),
                ) {
                    if !name.is_empty() {
                        test_names.insert(id.to_string(), name.to_string());
                    }
                }
            }
        }

        let mut evidence = BTreeMap::new();
        let mut records = BTreeMap::new();
        // Which test each help-screen data name belongs to, so a screen is
        // recorded against the test it describes.
        let mut tests_by_data_name: BTreeMap<String, String> = BTreeMap::new();

        for qualification in root
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "odstTestQualification")
        {
            for selection in qualification
                .children()
                .filter(|node| node.is_element() && node.tag_name().name() == "odstTestSelection")
            {
                let test_id = require_attribute(selection, "testID")?;
                let test_slug = preferred_segment(test_id, "testID")?;
                if let Some(data_id) = selection.attribute("helpScreenDataNameId").map(str::trim) {
                    tests_by_data_name.insert(data_id.to_string(), test_slug.clone());
                }
                let name = selection
                    .attribute("helpScreenDataNameId")
                    .and_then(|id| test_names.get(id.trim()))
                    .cloned()
                    .unwrap_or_else(|| format!("ODST test {test_id}"));

                let entity = KnowledgeEntity {
                    kind: EntityKind::DiagnosticCapability,
                    id: format!("ODST-{module_slug}-{test_slug}"),
                };
                entity.validate()?;

                // What the test is, in one escaped text: the identifier, the
                // name, and SDD's own timings in milliseconds (ADR-0032).
                let described = format!(
                    "test={};name={};time_ms={};timeout_ms={}",
                    escape(test_id),
                    escape(&name),
                    escape(selection.attribute("time").unwrap_or_default().trim()),
                    escape(selection.attribute("timeout").unwrap_or_default().trim()),
                );

                for (index, qualifier) in selection
                    .children()
                    .filter(|node| {
                        node.is_element() && node.tag_name().name() == "odstTestQualifier"
                    })
                    .enumerate()
                {
                    // The tests and their qualifiers are the English pack's
                    // to say; another language adds only its words.
                    if self.language.is_some() {
                        continue;
                    }
                    let applicability = applicability_from_qualifier(qualifier, module)?;
                    let record_id = format!(
                        "{}.odst.{module_slug}.{test_slug}.{index}",
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
                                description: format!(
                                    "odstTestSelection[@testID='{test_id}']/odstTestQualifier[{index}]"
                                ),
                                document_page: None,
                                document_section: Some("odstTestQualification".into()),
                                record_key: Some(format!("{module}/{test_id}")),
                                capture_timestamp_us: None,
                            },
                            excerpt: Some(truncate(&format!("{module} test {test_id}: {name}"))),
                            notes: None,
                        },
                    );
                    // The test itself, in words a person reads, under the
                    // same applicability as the capability beside it.
                    let described_id = format!("{record_id}.described");
                    records.insert(
                        described_id.clone(),
                        KnowledgeRecord {
                            id: described_id,
                            entity: entity.clone(),
                            key: ClaimKey::Custom {
                                name: ODST_TEST_CLAIM.to_string(),
                            },
                            value: KnowledgeValue::Text {
                                value: described.clone(),
                            },
                            applicability: applicability.clone(),
                            evidence_ids: vec![EvidenceId::new(evidence_id.clone())?],
                            validation_state: ValidationState::Unverified,
                        },
                    );
                    records.insert(
                        record_id.clone(),
                        KnowledgeRecord {
                            id: record_id,
                            entity: entity.clone(),
                            key: ClaimKey::SupportsCapability {
                                capability: name.clone(),
                            },
                            value: KnowledgeValue::Capability {
                                name: name.clone(),
                                supported: true,
                                // An on-demand self test commands the ECU to act.
                                // It is never READ_ONLY, and stamping it here is
                                // what keeps it out of stage 1.
                                safety_class: Some(DiagnosticSafetyClass::ServiceRoutine),
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

        // Which screen a car is given for a test, and what each screen
        // says: two claims, so one screen serving dozens of cars is written
        // once (ADR-0032, decision 2).
        let mut used_screens: BTreeMap<String, (String, String)> = BTreeMap::new();
        if let Some(qualification) = child_element(root, "helpScreenQualification") {
            for data_name in qualification
                .children()
                .filter(|node| node.is_element() && node.tag_name().name() == "helpScreenDataName")
            {
                let Some(data_id) = data_name.attribute("id").map(str::trim) else {
                    continue;
                };
                let Some(test_slug) = tests_by_data_name.get(data_id) else {
                    // A screen no test on this module points at says nothing
                    // about this car and is not recorded.
                    continue;
                };
                let entity = KnowledgeEntity {
                    kind: EntityKind::DiagnosticCapability,
                    id: format!("ODST-{module_slug}-{test_slug}"),
                };
                for (index, selection) in data_name
                    .children()
                    .filter(|node| {
                        node.is_element() && node.tag_name().name() == "helpScreenSelection"
                    })
                    .enumerate()
                {
                    let Some(screen) = selection.attribute("helpScreenName").map(str::trim) else {
                        continue;
                    };
                    let Some((text, item_lines)) = screens.get(screen) else {
                        continue;
                    };
                    if text.trim().is_empty() {
                        continue;
                    }
                    used_screens.insert(screen.to_string(), (text.clone(), item_lines.clone()));
                    // Which car is given which screen is the English pack's
                    // to say as well.
                    if self.language.is_some() {
                        continue;
                    }
                    for (position, qualifier) in selection
                        .children()
                        .filter(|node| {
                            node.is_element() && node.tag_name().name() == "helpScreenQualifier"
                        })
                        .enumerate()
                    {
                        let applicability = applicability_from_qualifier(qualifier, module)?;
                        let record_id = format!(
                            "{}.odst.{module_slug}.{test_slug}.help.{index}.{position}",
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
                                    description: format!(
                                        "helpScreenDataName[@id='{data_id}']/helpScreenSelection[{index}]"
                                    ),
                                    document_page: None,
                                    document_section: Some("helpScreenQualification".into()),
                                    record_key: Some(format!("{module}/{screen}")),
                                    capture_timestamp_us: None,
                                },
                                excerpt: Some(truncate(text)),
                                notes: None,
                            },
                        );
                        records.insert(
                            record_id.clone(),
                            KnowledgeRecord {
                                id: record_id,
                                entity: entity.clone(),
                                key: ClaimKey::Custom {
                                    name: format!("{ODST_HELP_CLAIM_PREFIX}{screen}"),
                                },
                                value: KnowledgeValue::Text {
                                    value: screen.to_string(),
                                },
                                applicability,
                                evidence_ids: vec![EvidenceId::new(evidence_id)?],
                                validation_state: ValidationState::Unverified,
                            },
                        );
                    }
                }
            }
        }

        // What each used screen says — and, for a pack in another language,
        // the only thing written. The record id carries the language last;
        // the evidence id of a language pack does not repeat the source id,
        // which is longer by the code, so it stays within the id limit.
        let suffix = self.language_suffix();
        for (screen, (text, item_lines)) in used_screens {
            let record_id = format!(
                "{}.odst.{module_slug}.screen.{screen}{suffix}",
                self.source.id.0
            );
            let evidence_id = match &self.language {
                None => format!("{}.ev.{record_id}", self.source.id.0),
                Some(_) => format!(
                    "{}.ev.odst.{module_slug}.screen.{screen}{suffix}",
                    self.source.id.0
                ),
            };
            evidence.insert(
                evidence_id.clone(),
                EvidenceRecord {
                    id: EvidenceId::new(evidence_id.clone())?,
                    source_id: self.source.id.clone(),
                    evidence_class: None,
                    locator: SourceLocator {
                        description: format!("helpScreenList/helpScreen[@name='{screen}']"),
                        document_page: None,
                        document_section: Some("helpScreenList".into()),
                        record_key: Some(screen.clone()),
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
                    entity: KnowledgeEntity {
                        kind: EntityKind::EcuFamily,
                        id: module.to_string(),
                    },
                    key: ClaimKey::Custom {
                        name: format!("{ODST_SCREEN_CLAIM_PREFIX}{screen}{suffix}"),
                    },
                    value: KnowledgeValue::Text { value: text },
                    applicability: Applicability::default(),
                    evidence_ids: vec![EvidenceId::new(evidence_id.clone())?],
                    validation_state: ValidationState::Unverified,
                },
            );
            // The same screen as the names of its items, on the same
            // evidence, so a line is joined to its twin in another language
            // by name (`ADR-0034`).
            let items_record_id = format!(
                "{}.odst.{module_slug}.screenitems.{screen}{suffix}",
                self.source.id.0
            );
            records.insert(
                items_record_id.clone(),
                KnowledgeRecord {
                    id: items_record_id,
                    entity: KnowledgeEntity {
                        kind: EntityKind::EcuFamily,
                        id: module.to_string(),
                    },
                    key: ClaimKey::Custom {
                        name: format!("{ODST_SCREEN_ITEMS_CLAIM_PREFIX}{screen}{suffix}"),
                    },
                    value: KnowledgeValue::Text { value: item_lines },
                    applicability: Applicability::default(),
                    evidence_ids: vec![EvidenceId::new(evidence_id)?],
                    validation_state: ValidationState::Unverified,
                },
            );
        }

        // A module whose screens no test points at has nothing for a pack in
        // another language to add: an empty batch, not a fault.
        if records.is_empty() && self.language.is_none() {
            return Err(KnowledgeError::Parse(format!(
                "no qualified self tests were found for {module}"
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

/// Map one `odstTestQualifier` and its `qual` children onto applicability.
///
/// `TYPE` and `SUBTYPE` carry the same meaning as the DID catalogue's `type`
/// and `subType`, so they use the same dimensions. Every other qualifier is
/// recorded verbatim under its own dimension rather than being interpreted, and
/// an unrecognised attribute stops ingestion.
fn applicability_from_qualifier(
    qualifier: roxmltree::Node<'_, '_>,
    module: &str,
) -> Result<Applicability, KnowledgeError> {
    let mut applicability = qualified_applicability();
    applicability.ecu_family = DimensionConstraint::one_of([module.to_string()])?;

    for attribute in qualifier.attributes() {
        let value = attribute.value().trim();
        if value.is_empty() {
            continue;
        }
        match attribute.name() {
            "model" => {
                applicability.vehicle_program = DimensionConstraint::one_of([value.to_string()])?
            }
            "year" => {
                applicability.other.insert(
                    MODEL_YEAR_DESIGNATION_DIMENSION.to_string(),
                    DimensionConstraint::one_of([value.to_string()])?,
                );
            }
            other => {
                return Err(KnowledgeError::Parse(format!(
                    "odstTestQualifier carries an unrecognised attribute '{other}'"
                )))
            }
        }
    }

    for qual in qualifier
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == "qual")
    {
        let kind = require_attribute(qual, "type")?;
        let value = require_attribute(qual, "value")?;
        match kind {
            "TYPE" => {
                applicability.powertrain = DimensionConstraint::one_of([value.to_string()])?;
            }
            "SUBTYPE" => {
                applicability.variant = DimensionConstraint::one_of([value.to_string()])?;
            }
            other => {
                let dimension = format!("{QUAL_DIMENSION_PREFIX}{}", other.to_ascii_lowercase());
                applicability
                    .other
                    .insert(dimension, DimensionConstraint::one_of([value.to_string()])?);
            }
        }
    }

    applicability.validate()?;
    Ok(applicability)
}

fn truncate(value: &str) -> String {
    value.chars().take(500).collect()
}

/// The document's own string table: mnemonic name → its text, flattened.
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
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if text.is_empty() {
            continue;
        }
        texts.insert(name.to_string(), text);
    }
    texts
}

/// Each screen as the lines it is made of, in the order SDD lists them —
/// once as the text and once as the items, the mnemonic's name, U+001F, its
/// text, one per line. An item the string table has no text for is left out
/// rather than shown as its mnemonic; a screen of nothing but such items
/// yields no text at all.
fn help_screens(
    root: roxmltree::Node<'_, '_>,
    mnemonics: &BTreeMap<String, String>,
) -> BTreeMap<String, (String, String)> {
    let mut screens = BTreeMap::new();
    let Some(list) = child_element(root, "helpScreenList") else {
        return screens;
    };
    for screen in list
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == "helpScreen")
    {
        let Some(name) = screen.attribute("name").map(str::trim) else {
            continue;
        };
        let items: Vec<(&str, &str)> = screen
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "helpScreenItem")
            .filter_map(|item| item.attribute("mnemonicName"))
            .filter_map(|mnemonic| {
                mnemonics
                    .get(mnemonic)
                    .map(|text| (mnemonic, text.as_str()))
            })
            .filter(|(_, line)| !line.trim().is_empty())
            .collect();
        if items.is_empty() {
            continue;
        }
        let text = items
            .iter()
            .map(|(_, line)| *line)
            .collect::<Vec<_>>()
            .join("\n");
        let item_lines = items
            .iter()
            .map(|(mnemonic, line)| format!("{mnemonic}{DTC_HELP_ITEM_SEPARATOR}{line}"))
            .collect::<Vec<_>>()
            .join("\n");
        screens.insert(name.to_string(), (text, item_lines));
    }
    screens
}

/// The escaping of `ADR-0028`: the separators a value may not carry raw.
fn escape(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace(';', "%3B")
        .replace('|', "%7C")
        .replace('=', "%3D")
}
