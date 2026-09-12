use crate::{
    child_element, child_text, evidence_class_for, preferred_segment, qualified_applicability,
    require_attribute, validation_state_for, ModelYearTimeline, YEAR_BREAKPOINT_DIMENSION,
};
use knowledge::{
    Applicability, CanIdFormat, ClaimKey, DimensionConstraint, EntityKind, EvidenceId,
    EvidenceRecord, IngestionAdapter, IngestionBatch, KnowledgeEntity, KnowledgeError,
    KnowledgeRecord, KnowledgeValue, SourceLocator, SourceRecord, ValidationState,
    KNOWLEDGE_SCHEMA_VERSION,
};
use std::collections::BTreeMap;

pub const PLATFORM_PARSER_ID: &str = "sdd-platform";
const PLATFORM_PARSER_VERSION: &str = "1.0.0";

/// Claim key under which a module's SDD network membership is recorded.
pub const NETWORK_CLAIM: &str = "sdd_network";
/// Custom claim holding a module's physical (node) address verbatim, for the
/// addressing schemes — `normal_fixed`, `enhanced` — whose CAN identifiers are
/// derived from a prefix and this address rather than stated. The derivation
/// is not performed here: the module is recorded so that it is *seen*, and it
/// resolves as unreachable until a later slice derives identifiers with the
/// standard as evidence.
pub const PHYSICAL_ADDRESS_CLAIM: &str = "sdd_physical_address";
/// Encoding descriptor of an identification identifier (ADR-0027): the whole
/// payload is a text — a part number, a serial, a VIN. The decoder shows
/// printable ASCII as text and anything else as bytes.
pub const IDENTIFICATION_ENCODING: &str = "text=ascii";
/// The module's data-identifier sets whose members are a module's
/// identification: its own set, the software part numbers, the pre-delivery
/// list. The module's own set also carries data that is not identification;
/// only its `0xF100`–`0xF1FF` members are taken (ADR-0027).
const IDENTIFICATION_SET_TYPES: [&str; 3] = ["NET", "SWDL", "PDI"];
const IDENTIFICATION_RANGE: std::ops::RangeInclusive<u16> = 0xF100..=0xF1FF;
/// The one service an identification identifier is read with.
const READ_DATA_BY_IDENTIFIER_SERVICE: &str = "0x22";

/// One `<did>` of a named data-identifier set, as the platform declares it.
#[derive(Clone, Debug)]
struct DeclaredIdentifier {
    identifier: u16,
    /// SDD's human text for it (`ECU Core Assembly Number`), or its
    /// attribute name when the text is absent.
    name: String,
    /// The service SDD reads it with, verbatim (`0x22`).
    service: Option<String>,
}

/// Adapter for the SDD `PLATFORM_<PROGRAM>_<YEAR>.xml` documents.
///
/// These carry the vehicle's network architecture and its module fitment list:
/// which buses exist, at what rate and identifier width, and which diagnostic
/// CAN identifiers reach each module. It is the addressing layer the rest of the
/// SDD corpus does not state.
///
/// # Only diagnostic-session addressing is ingested
///
/// Each module declares addresses for both a `diag` and a `prog` session.
/// Programming-session addressing is a route into software download, which is
/// outside every current stage, so it is **not ingested at all** rather than
/// recorded and hoped to be ignored. A route that exists in the knowledge base
/// is a route something can be built on.
#[derive(Clone, Debug)]
pub struct PlatformAdapter {
    source: SourceRecord,
    timeline: Option<ModelYearTimeline>,
    derive_normal_fixed: bool,
}

impl PlatformAdapter {
    pub fn new(source: SourceRecord) -> Result<Self, KnowledgeError> {
        source.validate()?;
        Ok(Self {
            source,
            timeline: None,
            derive_normal_fixed: false,
        })
    }

    /// Opt in to deriving a calendar year range from the platform's marker.
    pub fn with_timeline(mut self, timeline: ModelYearTimeline) -> Self {
        self.timeline = Some(timeline);
        self
    }

    /// Opt in to deriving 29-bit CAN identifiers for physically addressed
    /// modules on `normal_fixed` buses (ADR-0017). The derived records cite
    /// the built-in standards and research manifests by evidence identifier,
    /// so the store must hold those manifests before such a batch is
    /// ingested; the application and the exporter both load them first.
    pub fn with_derived_normal_fixed_identifiers(mut self) -> Self {
        self.derive_normal_fixed = true;
        self
    }
}

/// Evidence the derived normal-fixed identifiers cite, from the built-in
/// manifests `fixtures/knowledge/documented/iso15765_normal_fixed_addressing.json`
/// and `fixtures/knowledge/research/normal_fixed_tester_address_hypothesis.json`.
pub const ISO15765_NORMAL_FIXED_LAYOUT_EVIDENCE: &str = "evidence-iso15765-2-normal-fixed-layout";
pub const NORMAL_FIXED_TESTER_ADDRESS_EVIDENCE: &str =
    "evidence-normal-fixed-tester-address-hypothesis";
/// The tester's source address the derivation assumes (ISO 15765-4's external
/// test equipment address); a hypothesis for JLR enhanced diagnostics.
pub const NORMAL_FIXED_TESTER_ADDRESS: u32 = 0xF1;
const NORMAL_FIXED_MODE: &str = "normal_fixed";

/// One declared bus, as the platform states it.
#[derive(Clone, Debug)]
struct Network {
    id: String,
    bitrate_bps: Option<u32>,
    can_id_format: Option<CanIdFormat>,
    addressing_mode: Option<String>,
    /// Application protocol the bus declares, such as `ISO14229`.
    diagnostic_protocol: Option<String>,
    /// The physical-addressing identifier prefix a `normal_fixed` bus
    /// declares (`0x18DA`), verbatim as a number.
    physical_prefix: Option<u32>,
}

/// Diagnostic protocol name SDD declares for buses spoken to with ISO 14229.
pub const UDS_DIAGNOSTIC_PROTOCOL: &str = "ISO14229";
/// Read-only UDS capabilities recorded for every module on such a bus
/// (ADR-0013). The literals are repeated by the UDS bridge, which cannot
/// depend on this crate; its tests assert that the two agree.
pub const UDS_READ_DATA_BY_IDENTIFIER_CAPABILITY: &str =
    "uds.service22.read_data_by_identifier.read_only";
pub const UDS_READ_DTC_INFORMATION_CAPABILITY: &str =
    "uds.service19.read_dtc_information.read_only";

impl IngestionAdapter for PlatformAdapter {
    fn parser_id(&self) -> &'static str {
        PLATFORM_PARSER_ID
    }

    fn parser_version(&self) -> &'static str {
        PLATFORM_PARSER_VERSION
    }

    fn parse(&self, input: &str) -> Result<IngestionBatch, KnowledgeError> {
        let document = roxmltree::Document::parse(input)
            .map_err(|error| KnowledgeError::Parse(error.to_string()))?;
        let root = document.root_element();
        if root.tag_name().name() != "platform" {
            return Err(KnowledgeError::Parse(format!(
                "expected a <platform> root, found <{}>",
                root.tag_name().name()
            )));
        }

        let (program, marker) = qualification(root)?;
        let networks = networks(root)?;
        if networks.is_empty() {
            return Err(KnowledgeError::Parse(
                "<platform> declares no networks".into(),
            ));
        }

        let mut evidence = BTreeMap::new();
        let mut records = BTreeMap::new();
        let base = self.applicability(&program, &marker)?;
        let sets = identifier_sets(root)?;

        self.add_connector(root, &program, &base, &mut evidence, &mut records)?;
        for network in networks.values() {
            self.add_network(network, &program, &base, &mut evidence, &mut records)?;
        }
        self.add_modules(
            root,
            &networks,
            &sets,
            &program,
            &base,
            &mut evidence,
            &mut records,
        )?;

        if records.is_empty() {
            return Err(KnowledgeError::Parse(
                "<platform> yielded no usable claims".into(),
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
                    // A record citing evidence outside this batch is a derived
                    // one (ADR-0017) and keeps the state it was given.
                    let cites_only_own = record
                        .evidence_ids
                        .iter()
                        .all(|id| id.0.starts_with(&format!("{}.ev.", self.source.id.0)));
                    if cites_only_own {
                        record.validation_state = state;
                    }
                    record
                })
                .collect(),
        })
    }
}

fn parse_hex(text: &str) -> Option<u32> {
    let digits = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .unwrap_or(text);
    u32::from_str_radix(digits, 16).ok()
}

impl PlatformAdapter {
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
                    document_section: Some("platform".into()),
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

    fn add_network(
        &self,
        network: &Network,
        program: &str,
        base: &Applicability,
        evidence: &mut BTreeMap<String, EvidenceRecord>,
        records: &mut BTreeMap<String, KnowledgeRecord>,
    ) -> Result<(), KnowledgeError> {
        let segment = preferred_segment(&network.id, "net_id")?;
        // The platform states a rate and an identifier width but never the
        // J1962 connector pins, so pins stay empty rather than assumed.
        self.push(
            format!("{}.network.{segment}", self.source.id.0),
            format!("network_architecture/network[@net_id='{}']", network.id),
            format!(
                "{program} {} rate={:?} width={:?}",
                network.id, network.bitrate_bps, network.can_id_format
            ),
            KnowledgeEntity {
                kind: EntityKind::NetworkRoute,
                id: format!("{program}-{segment}"),
            },
            ClaimKey::NetworkRoute,
            KnowledgeValue::NetworkRoute {
                logical_name: network.id.clone(),
                connector: None,
                pins: vec![],
                bitrate_bps: network.bitrate_bps,
            },
            base.clone(),
            evidence,
            records,
        )?;

        // The bus also states its application protocol, which F6 needs and the
        // rest of the corpus never supplies.
        if let Some(protocol) = &network.diagnostic_protocol {
            self.push(
                format!("{}.network.{segment}.protocol", self.source.id.0),
                format!(
                    "network_architecture/network[@net_id='{}']/protocol[@type='diagnostic']",
                    network.id
                ),
                format!("{program} {} diagnostic protocol {protocol}", network.id),
                KnowledgeEntity {
                    kind: EntityKind::NetworkRoute,
                    id: format!("{program}-{segment}"),
                },
                ClaimKey::UsesProtocolFamily,
                KnowledgeValue::ProtocolFamily {
                    name: protocol.clone(),
                },
                base.clone(),
                evidence,
                records,
            )?;
        }
        Ok(())
    }

    /// Record the diagnostic connector and the pins the platform declares.
    ///
    /// Pins are recorded as the document lists them and are **not** assigned to
    /// individual buses. The pin names imply an assignment, but implying is not
    /// stating, and a mis-assigned pin is a wrong physical connection.
    fn add_connector(
        &self,
        root: roxmltree::Node<'_, '_>,
        program: &str,
        base: &Applicability,
        evidence: &mut BTreeMap<String, EvidenceRecord>,
        records: &mut BTreeMap<String, KnowledgeRecord>,
    ) -> Result<(), KnowledgeError> {
        let Some(architecture) = child_element(root, "network_architecture") else {
            return Ok(());
        };
        let Some(connector) = child_element(architecture, "connector") else {
            return Ok(());
        };
        let Some(name) = child_text(connector, "name") else {
            return Ok(());
        };

        let mut pins = Vec::new();
        let mut described = Vec::new();
        for pin in connector
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "pin")
        {
            let (Some(pin_name), Some(number)) =
                (child_text(pin, "name"), child_text(pin, "number"))
            else {
                continue;
            };
            let number = number.parse::<u8>().map_err(|_| {
                KnowledgeError::Parse(format!(
                    "connector pin '{pin_name}' has a non-numeric number"
                ))
            })?;
            // The knowledge model only represents connector pins 1 to 16.
            if !(1..=16).contains(&number) {
                return Err(KnowledgeError::Parse(format!(
                    "connector pin '{pin_name}' is outside the 1-16 range"
                )));
            }
            pins.push(number);
            described.push(format!("{pin_name}={number}"));
        }
        if pins.is_empty() {
            return Ok(());
        }
        pins.sort_unstable();
        pins.dedup();

        self.push(
            format!("{}.connector", self.source.id.0),
            "network_architecture/connector".into(),
            format!("{program} {name}: {}", described.join(" ")),
            KnowledgeEntity {
                kind: EntityKind::NetworkRoute,
                id: format!("{program}-connector"),
            },
            ClaimKey::NetworkRoute,
            KnowledgeValue::NetworkRoute {
                logical_name: name.to_string(),
                connector: Some(name.to_string()),
                pins,
                bitrate_bps: None,
            },
            base.clone(),
            evidence,
            records,
        )
    }

    /// ADR-0017: a physically addressed module on a `normal_fixed` 29-bit bus
    /// gets the identifiers ISO 15765-2 composes from the bus's prefix, the
    /// module's address and the tester's address. The record cites the
    /// module's own physical-address evidence, the standard, and the tester
    /// address hypothesis, and is therefore `Unverified` until a module
    /// answers. Nothing is derived when the bus is not `normal_fixed`, is not
    /// 29-bit, declares no prefix, or the address is not one byte.
    #[allow(clippy::too_many_arguments)]
    fn derive_normal_fixed_addressing(
        &self,
        segment: &str,
        suffix: &str,
        acronym: &str,
        program: &str,
        physical: &str,
        network: &Network,
        entity: &KnowledgeEntity,
        base: &Applicability,
        evidence: &mut BTreeMap<String, EvidenceRecord>,
        records: &mut BTreeMap<String, KnowledgeRecord>,
    ) -> Result<(), KnowledgeError> {
        if network.addressing_mode.as_deref() != Some(NORMAL_FIXED_MODE)
            || network.can_id_format != Some(CanIdFormat::Extended29Bit)
        {
            return Ok(());
        }
        let Some(prefix) = network.physical_prefix else {
            return Ok(());
        };
        let Some(address) = parse_hex(physical).filter(|value| *value <= 0xFF) else {
            return Ok(());
        };
        let request = (prefix << 16) | (address << 8) | NORMAL_FIXED_TESTER_ADDRESS;
        let response = (prefix << 16) | (NORMAL_FIXED_TESTER_ADDRESS << 8) | address;

        let record_id = format!("{}.module.{segment}.addressing{suffix}", self.source.id.0);
        let own_evidence_id = format!(
            "{}.ev.{}.module.{segment}.physical_address{suffix}",
            self.source.id.0, self.source.id.0
        );
        if !evidence.contains_key(&own_evidence_id) {
            return Err(KnowledgeError::Parse(format!(
                "derived addressing for {acronym} has no physical-address evidence to cite"
            )));
        }
        records.insert(
            record_id.clone(),
            KnowledgeRecord {
                id: record_id,
                entity: entity.clone(),
                key: ClaimKey::DiagnosticAddressing,
                value: KnowledgeValue::DiagnosticAddressing {
                    request_id: Some(request),
                    response_id: Some(response),
                    functional_request_id: None,
                    can_id_format: Some(CanIdFormat::Extended29Bit),
                    addressing_mode: Some(NORMAL_FIXED_MODE.to_string()),
                },
                applicability: base.clone(),
                evidence_ids: {
                    // The model wants evidence identifiers sorted and unique.
                    let mut ids = vec![
                        EvidenceId::new(own_evidence_id)?,
                        EvidenceId::new(ISO15765_NORMAL_FIXED_LAYOUT_EVIDENCE)?,
                        EvidenceId::new(NORMAL_FIXED_TESTER_ADDRESS_EVIDENCE)?,
                    ];
                    ids.sort();
                    ids
                },
                // Research evidence is among the sources: the store admits
                // nothing stronger, and nothing stronger is claimed.
                validation_state: ValidationState::Unverified,
            },
        );
        let _ = program;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn add_modules(
        &self,
        root: roxmltree::Node<'_, '_>,
        networks: &BTreeMap<String, Network>,
        sets: &BTreeMap<String, Vec<DeclaredIdentifier>>,
        program: &str,
        base: &Applicability,
        evidence: &mut BTreeMap<String, EvidenceRecord>,
        records: &mut BTreeMap<String, KnowledgeRecord>,
    ) -> Result<(), KnowledgeError> {
        let Some(fitment) = child_element(root, "module_fitment") else {
            return Ok(());
        };
        for module in fitment
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "module")
        {
            let Some(code_name) = child_element(module, "module_code_name") else {
                continue;
            };
            let acronym = require_attribute(code_name, "acronym")?;
            let segment = preferred_segment(acronym, "module acronym")?;
            // One acronym can appear several times under different tests, each
            // with its own address; the suffix keeps the rows apart and the
            // narrowed applicability says which car each row is for.
            let quals = module_quals(module);
            let suffix = qual_suffix(&quals);
            let qualifier_note = if quals.is_empty() {
                "none".to_string()
            } else {
                qual_text(&quals)
            };
            let base = &qualified_by_module(base, &quals)?;
            let network_id = child_text(module, "network").unwrap_or_default();
            let network = networks.get(network_id);

            // Only the diagnostic session is ingested; see the type comment.
            let request = address(module, "can_tx", "diag")?;
            let response = address(module, "can_rx", "diag")?;
            let physical = address_text(module, "phys", "diag");
            if request.is_none() && response.is_none() && physical.is_none() {
                continue;
            }

            let entity = KnowledgeEntity {
                kind: EntityKind::EcuFamily,
                id: segment.clone(),
            };
            let fitment_note = child_text(module, "fitment").unwrap_or("unstated");

            if request.is_some() || response.is_some() {
                self.push(
                format!("{}.module.{segment}.addressing{suffix}", self.source.id.0),
                format!("module_fitment/module[module_code_name/@acronym='{acronym}']"),
                format!(
                    "{program} {acronym} tx={request:?} rx={response:?} net={network_id} fitment={fitment_note} qualifier={qualifier_note}"
                ),
                entity.clone(),
                ClaimKey::DiagnosticAddressing,
                KnowledgeValue::DiagnosticAddressing {
                    request_id: request,
                    response_id: response,
                    functional_request_id: None,
                    can_id_format: network.and_then(|network| network.can_id_format),
                    addressing_mode: network.and_then(|network| network.addressing_mode.clone()),
                },
                base.clone(),
                evidence,
                records,
            )?;
            }
            if let Some(physical) = physical.as_deref() {
                self.push(
                    format!("{}.module.{segment}.physical_address{suffix}", self.source.id.0),
                    format!(
                        "module_fitment/module[module_code_name/@acronym='{acronym}']/address[@type='phys'][@session='diag']"
                    ),
                    format!(
                        "{program} {acronym} physical address {physical} on {network_id}; CAN identifiers follow the bus addressing scheme and are not derived"
                    ),
                    entity.clone(),
                    ClaimKey::Custom {
                        name: PHYSICAL_ADDRESS_CLAIM.to_string(),
                    },
                    KnowledgeValue::Text {
                        value: physical.to_string(),
                    },
                    base.clone(),
                    evidence,
                    records,
                )?;
            }
            if self.derive_normal_fixed && request.is_none() && response.is_none() {
                if let (Some(physical), Some(network)) = (physical.as_deref(), network) {
                    self.derive_normal_fixed_addressing(
                        &segment, &suffix, acronym, program, physical, network, &entity, base,
                        evidence, records,
                    )?;
                }
            }

            if !network_id.is_empty() {
                self.push(
                    format!("{}.module.{segment}.network{suffix}", self.source.id.0),
                    format!("module_fitment/module[module_code_name/@acronym='{acronym}']/network"),
                    format!("{program} {acronym} on {network_id}"),
                    entity.clone(),
                    ClaimKey::Custom {
                        name: NETWORK_CLAIM.to_string(),
                    },
                    KnowledgeValue::Text {
                        value: network_id.to_string(),
                    },
                    base.clone(),
                    evidence,
                    records,
                )?;
            }
            if let Some(network) = network {
                self.add_module_bus_facts(
                    network, &entity, acronym, &suffix, program, base, evidence, records,
                )?;
            }
            self.add_identification(
                module, sets, acronym, &segment, &suffix, program, base, evidence, records,
            )?;
        }
        Ok(())
    }

    /// The module's identification identifiers (ADR-0027): the members of
    /// its `SWDL` and `PDI` sets and the `0xF100`–`0xF1FF` members of its own
    /// `NET` set, each recorded in the DID catalogue's own shape so that the
    /// resolver's readable list and the transaction gate admit them as they
    /// admit a catalogue parameter. A set the document references but never
    /// defines states nothing and yields nothing; a member read with any
    /// service but `0x22` is not a read this product makes and is left out.
    #[allow(clippy::too_many_arguments)]
    fn add_identification(
        &self,
        module: roxmltree::Node<'_, '_>,
        sets: &BTreeMap<String, Vec<DeclaredIdentifier>>,
        acronym: &str,
        segment: &str,
        suffix: &str,
        program: &str,
        base: &Applicability,
        evidence: &mut BTreeMap<String, EvidenceRecord>,
        records: &mut BTreeMap<String, KnowledgeRecord>,
    ) -> Result<(), KnowledgeError> {
        for reference in module.children().filter(|node| {
            node.is_element()
                && node.tag_name().name() == "data_identifier_set"
                && node
                    .attribute("type")
                    .is_some_and(|kind| IDENTIFICATION_SET_TYPES.contains(&kind))
        }) {
            let kind = reference.attribute("type").unwrap_or_default();
            // The set's name is the element's own text; a qualifier may
            // follow it as a child element.
            let Some(set_name) = reference
                .text()
                .map(str::trim)
                .filter(|name| !name.is_empty())
            else {
                continue;
            };
            let Some(members) = sets.get(set_name) else {
                continue;
            };
            let set_quals = module_quals(reference);
            let set_suffix = qual_suffix(&set_quals);
            let mut applicability = qualified_by_module(base, &set_quals)?;
            applicability.ecu_family = DimensionConstraint::one_of([acronym.to_string()])?;
            applicability.validate()?;
            let qualifier_note = if set_quals.is_empty() {
                String::new()
            } else {
                format!(" set qualifier={}", qual_text(&set_quals))
            };

            for member in members {
                if member.service.as_deref() != Some(READ_DATA_BY_IDENTIFIER_SERVICE) {
                    continue;
                }
                if kind == "NET" && !IDENTIFICATION_RANGE.contains(&member.identifier) {
                    continue;
                }
                let identifier = format!("0x{:04X}", member.identifier);
                let record_id = format!(
                    "{}.module.{segment}.identification.{}{suffix}{set_suffix}",
                    self.source.id.0,
                    identifier.to_ascii_lowercase()
                );
                // The same identifier reached through two of the module's
                // sets is one identifier; the first set to name it speaks.
                if records.contains_key(&record_id) {
                    continue;
                }
                self.push(
                    record_id,
                    format!(
                        "module_fitment/module[module_code_name/@acronym='{acronym}']/data_identifier_set[@type='{kind}']; data_identifier_set[@id='{set_name}']/did[@id='{identifier}']"
                    ),
                    format!(
                        "{program} {acronym} {kind} {set_name}: {identifier} {} service 0x22{qualifier_note}",
                        member.name
                    ),
                    KnowledgeEntity {
                        kind: EntityKind::IdentifierParameter,
                        id: format!("DID-{identifier}"),
                    },
                    ClaimKey::ParameterDefinition {
                        parameter: member.name.clone(),
                    },
                    KnowledgeValue::IdentifierDefinition {
                        identifier,
                        encoding: Some(IDENTIFICATION_ENCODING.to_string()),
                        unit: None,
                    },
                    applicability.clone(),
                    evidence,
                    records,
                )?;
            }
        }
        Ok(())
    }

    /// Bus facts SDD states once per network, recorded on the module that sits
    /// on it (ADR-0013). Width and addressing mode were already copied this
    /// way; the bus name, its diagnostic protocol and the read-only UDS
    /// capabilities that protocol carries now follow. The bus's own rate stays
    /// on the network record: a plan carries the route's rate (ADR-0015). Both facts come from
    /// this one document, so each evidence locator cites both elements.
    #[allow(clippy::too_many_arguments)]
    fn add_module_bus_facts(
        &self,
        network: &Network,
        entity: &KnowledgeEntity,
        acronym: &str,
        variant: &str,
        program: &str,
        base: &Applicability,
        evidence: &mut BTreeMap<String, EvidenceRecord>,
        records: &mut BTreeMap<String, KnowledgeRecord>,
    ) -> Result<(), KnowledgeError> {
        let segment = &entity.id;
        let module_locator =
            format!("module_fitment/module[module_code_name/@acronym='{acronym}']/network");
        let network_locator = format!("network_architecture/network[@net_id='{}']", network.id);

        self.push(
            format!("{}.module.{segment}.bus{variant}", self.source.id.0),
            format!("{module_locator}; {network_locator}"),
            format!("{program} {acronym} on {}", network.id),
            entity.clone(),
            ClaimKey::NetworkRoute,
            KnowledgeValue::NetworkRoute {
                logical_name: network.id.clone(),
                connector: None,
                pins: vec![],
                bitrate_bps: None,
            },
            base.clone(),
            evidence,
            records,
        )?;

        let Some(protocol) = &network.diagnostic_protocol else {
            return Ok(());
        };
        self.push(
            format!("{}.module.{segment}.protocol{variant}", self.source.id.0),
            format!("{module_locator}; {network_locator}/protocol[@type='diagnostic']"),
            format!(
                "{program} {acronym} on {} diagnostic protocol {protocol}",
                network.id
            ),
            entity.clone(),
            ClaimKey::UsesProtocolFamily,
            KnowledgeValue::ProtocolFamily {
                name: protocol.clone(),
            },
            base.clone(),
            evidence,
            records,
        )?;

        if protocol != UDS_DIAGNOSTIC_PROTOCOL {
            return Ok(());
        }
        for (suffix, capability) in [
            (
                "read_data_by_identifier",
                UDS_READ_DATA_BY_IDENTIFIER_CAPABILITY,
            ),
            ("read_dtc_information", UDS_READ_DTC_INFORMATION_CAPABILITY),
        ] {
            self.push(
                format!(
                    "{}.module.{segment}.capability.{suffix}{variant}",
                    self.source.id.0
                ),
                format!("{module_locator}; {network_locator}/protocol[@type='diagnostic']"),
                format!("{program} {acronym}: {protocol} read service {capability}"),
                entity.clone(),
                ClaimKey::SupportsCapability {
                    capability: capability.to_string(),
                },
                KnowledgeValue::Capability {
                    name: capability.to_string(),
                    supported: true,
                    // A read service of the protocol SDD declares for this
                    // module. Stamping READ_ONLY is what admits it to stage 1.
                    safety_class: Some(knowledge::DiagnosticSafetyClass::ReadOnly),
                },
                base.clone(),
                evidence,
                records,
            )?;
        }
        Ok(())
    }
}

/// The `<qual>` tests a module's own `<qualifier>` carries, in document order.
///
/// SDD uses these to say that one acronym means different hardware depending
/// on the engine, the build year or the market — and gives each its own
/// diagnostic address. They must reach the record, or the rows collide and
/// only one address of several survives.
fn module_quals(module: roxmltree::Node<'_, '_>) -> Vec<(String, String)> {
    let Some(qualifier) = child_element(module, "qualifier") else {
        return Vec::new();
    };
    qualifier
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == "qual")
        .filter_map(|qual| {
            let kind = qual.attribute("type")?.trim();
            let value = qual.attribute("value")?.trim();
            (!kind.is_empty() && !value.is_empty()).then(|| (kind.to_string(), value.to_string()))
        })
        .collect()
}

/// The tests as one canonical line, for the evidence note and the digest.
fn qual_text(quals: &[(String, String)]) -> String {
    let mut parts: Vec<String> = quals
        .iter()
        .map(|(kind, value)| format!("{kind}={value}"))
        .collect();
    parts.sort();
    parts.dedup();
    parts.join(",")
}

/// The record-id suffix that keeps two qualified rows apart; empty when the
/// module carries no test, so the ids of the unqualified majority do not move.
///
/// A digest rather than the words themselves: a stable id may be 128
/// characters and an evidence id repeats the source id inside itself, so the
/// readable form pushed the longest rows over the limit. The words stay in
/// the evidence note, where they cost nothing.
fn qual_suffix(quals: &[(String, String)]) -> String {
    if quals.is_empty() {
        return String::new();
    }
    let digest = knowledge::sha256_bytes(qual_text(quals).as_bytes());
    format!(".q{}", &digest[..8])
}

fn slug(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// `base` narrowed by the module's own qualifier.
///
/// SDD's `type` is the powertrain and its `SubType` refines it — 4.2L against
/// 5L of the same V8 — which is what `variant` is for. Anything naming a
/// market is the market. Everything else keeps SDD's own name under `other`,
/// because inventing a dimension for it would claim to understand more than
/// the document says.
fn qualified_by_module(
    base: &Applicability,
    quals: &[(String, String)],
) -> Result<Applicability, KnowledgeError> {
    if quals.is_empty() {
        return Ok(base.clone());
    }
    let mut applicability = base.clone();
    for (kind, value) in quals {
        let constraint = DimensionConstraint::one_of([value.to_string()])?;
        if kind.eq_ignore_ascii_case("type") {
            applicability.powertrain = constraint;
        } else if kind.eq_ignore_ascii_case("subtype") {
            applicability.variant = constraint;
        } else if kind.to_ascii_uppercase().contains("MARKET") {
            applicability.market = constraint;
        } else {
            applicability
                .other
                .insert(format!("sdd_qual_{}", slug(kind)), constraint);
        }
    }
    applicability.validate()?;
    Ok(applicability)
}

/// Vehicle program and model-year marker, taken from the document's qualifiers.
///
/// Derived from the content rather than the file name, and required to be
/// unambiguous: a platform describing more than one program or marker would
/// make every claim in it ambiguous, so it is rejected instead of guessed.
fn qualification(root: roxmltree::Node<'_, '_>) -> Result<(String, String), KnowledgeError> {
    let mut programs = std::collections::BTreeSet::new();
    let mut markers = std::collections::BTreeSet::new();
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
                markers.insert(year.to_string());
            }
        }
    }
    match (programs.len(), markers.len()) {
        (1, 1) => Ok((
            programs.into_iter().next().expect("checked"),
            markers.into_iter().next().expect("checked"),
        )),
        _ => Err(KnowledgeError::Parse(format!(
            "<platform> must name exactly one program and one model-year marker; found {programs:?} and {markers:?}"
        ))),
    }
}

/// The named data-identifier sets a platform defines at its root, each a
/// list of `<did>` elements with the identifier, SDD's attribute name, the
/// service it is read with and, usually, a human text.
fn identifier_sets(
    root: roxmltree::Node<'_, '_>,
) -> Result<BTreeMap<String, Vec<DeclaredIdentifier>>, KnowledgeError> {
    let mut sets = BTreeMap::new();
    for set in root.children().filter(|node| {
        node.is_element()
            && node.tag_name().name() == "data_identifier_set"
            && node.has_attribute("id")
    }) {
        let id = require_attribute(set, "id")?.to_string();
        let mut members = Vec::new();
        for did in set
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "did")
        {
            let raw = require_attribute(did, "id")?;
            let Some(identifier) = parse_hex(raw).and_then(|value| u16::try_from(value).ok())
            else {
                return Err(KnowledgeError::Parse(format!(
                    "data_identifier_set '{id}' declares a did '{raw}' that is not a 16-bit identifier"
                )));
            };
            let attribute_name = did.attribute("name").map(str::trim).unwrap_or_default();
            let name = child_text(did, "tm")
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .unwrap_or(attribute_name)
                .to_string();
            if name.is_empty() {
                return Err(KnowledgeError::Parse(format!(
                    "data_identifier_set '{id}' declares did {raw} with no name"
                )));
            }
            members.push(DeclaredIdentifier {
                identifier,
                name,
                service: did
                    .attribute("service_id")
                    .map(|value| value.trim().to_string()),
            });
        }
        sets.insert(id, members);
    }
    Ok(sets)
}

fn networks(root: roxmltree::Node<'_, '_>) -> Result<BTreeMap<String, Network>, KnowledgeError> {
    let mut networks = BTreeMap::new();
    let Some(architecture) = child_element(root, "network_architecture") else {
        return Ok(networks);
    };
    for node in architecture
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == "network")
    {
        let id = require_attribute(node, "net_id")?.to_string();
        let can = child_element(node, "can");
        let bitrate_bps = can
            .and_then(|can| child_text(can, "rate"))
            .and_then(|rate| rate.parse::<u32>().ok())
            // SDD states the rate in kbit/s.
            .and_then(|rate| rate.checked_mul(1000));
        let can_id_format = match can.and_then(|can| child_text(can, "identifier")) {
            Some("11") => Some(CanIdFormat::Standard11Bit),
            Some("29") => Some(CanIdFormat::Extended29Bit),
            Some(other) => {
                return Err(KnowledgeError::Parse(format!(
                    "network '{id}' declares an unsupported identifier width '{other}'"
                )))
            }
            None => None,
        };
        let diagnostic_protocol = node
            .children()
            .find(|child| {
                child.is_element()
                    && child.tag_name().name() == "protocol"
                    && child.attribute("type") == Some("diagnostic")
            })
            .and_then(|child| child.text())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let scheme = child_element(node, "addressing")
            .and_then(|addressing| child_element(addressing, "can_iso15765"))
            .and_then(|iso| iso.children().find(|child| child.is_element()));
        let addressing_mode = scheme.map(|child| child.tag_name().name().to_string());
        let physical_prefix = scheme
            .filter(|child| child.tag_name().name() == NORMAL_FIXED_MODE)
            .and_then(|fixed| {
                fixed
                    .children()
                    .find(|child| {
                        child.is_element()
                            && child.tag_name().name() == "can_id_prefix"
                            && child.attribute("type") == Some("phys")
                    })
                    .and_then(|prefix| prefix.text())
                    .and_then(|text| parse_hex(text.trim()))
            });
        networks.insert(
            id.clone(),
            Network {
                id,
                bitrate_bps,
                can_id_format,
                addressing_mode,
                diagnostic_protocol,
                physical_prefix,
            },
        );
    }
    Ok(networks)
}

/// The verbatim text of one `<address>` value, for schemes whose identifiers
/// this adapter does not derive.
fn address_text(module: roxmltree::Node<'_, '_>, direction: &str, session: &str) -> Option<String> {
    module
        .children()
        .find(|child| {
            child.is_element()
                && child.tag_name().name() == "address"
                && child.attribute("type") == Some(direction)
                && child.attribute("session") == Some(session)
        })
        .and_then(|node| node.text())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

/// One `<address>` value for the given direction and session.
fn address(
    module: roxmltree::Node<'_, '_>,
    direction: &str,
    session: &str,
) -> Result<Option<u32>, KnowledgeError> {
    let Some(node) = module.children().find(|child| {
        child.is_element()
            && child.tag_name().name() == "address"
            && child.attribute("type") == Some(direction)
            && child.attribute("session") == Some(session)
    }) else {
        return Ok(None);
    };
    let Some(text) = node.text().map(str::trim).filter(|text| !text.is_empty()) else {
        return Ok(None);
    };
    let parsed = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .map(|hex| u32::from_str_radix(hex, 16))
        .unwrap_or_else(|| text.parse::<u32>())
        .map_err(|_| {
            KnowledgeError::Parse(format!("address '{text}' is not a usable identifier"))
        })?;
    Ok(Some(parsed))
}

#[cfg(test)]
mod qualifier_tests {
    use super::*;

    #[test]
    fn the_suffix_is_short_stable_and_order_independent() {
        let one = vec![
            ("type".to_string(), "V6DIESEL".to_string()),
            ("SubType".to_string(), "2.7L".to_string()),
        ];
        let other = vec![
            ("SubType".to_string(), "2.7L".to_string()),
            ("type".to_string(), "V6DIESEL".to_string()),
        ];
        let suffix = qual_suffix(&one);
        assert_eq!(suffix, qual_suffix(&other), "attribute order cannot matter");
        assert_eq!(suffix.len(), 10, "short enough for a stable id: {suffix}");
        assert!(
            qual_suffix(&[]).is_empty(),
            "an unqualified module keeps its id"
        );
        assert_ne!(
            suffix,
            qual_suffix(&[("type".to_string(), "V8SC".to_string())]),
            "different tests, different rows"
        );
    }
}
