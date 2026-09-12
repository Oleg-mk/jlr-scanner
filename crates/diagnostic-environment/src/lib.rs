//! Offline, evidence-backed diagnostic environment resolution.
//!
//! This crate produces data-only plans. It has no protocol, transport,
//! operating-system, backend, or UI dependency and cannot execute diagnostics.

pub use knowledge::{
    ApplicabilityResolution as VehicleApplicabilityResolution, CanIdFormat,
    ImplementationMarkerKind, ValidationState,
};
use knowledge::{
    ApplicabilityResolution, ClaimKey, DiagnosticSafetyClass, DimensionConstraint, EntityKind,
    EvidenceClass, KnowledgeEntity, KnowledgeQuery, KnowledgeStore, KnowledgeValue,
    ResolvedKnowledge, SourceLocator, SourceType, VehicleContext,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Claims the SDD platform ingest writes for a serial line's framing and
/// wake-up (`ADR-0029`). The literals are the ingest's; a test asserts they
/// agree.
const ISO_SETTINGS_CLAIM: &str = "sdd_iso_settings";
const ISO_WAKEUP_CLAIM: &str = "sdd_iso_wakeup";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticEnvironmentQuery {
    pub vehicle_context: VehicleContext,
    pub diagnostic_target: DiagnosticTarget,
}

impl DiagnosticEnvironmentQuery {
    pub fn new(
        vehicle_context: VehicleContext,
        diagnostic_target: DiagnosticTarget,
    ) -> Result<Self, QueryError> {
        diagnostic_target.validate()?;
        Ok(Self {
            vehicle_context,
            diagnostic_target,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiagnosticTarget {
    pub ecu_family: Option<String>,
    pub diagnostic_implementation: Option<String>,
    pub capability_id: Option<String>,
    pub knowledge_entity: Option<KnowledgeEntity>,
}

impl DiagnosticTarget {
    pub fn implementation(
        ecu_family: impl Into<String>,
        diagnostic_implementation: impl Into<String>,
        capability_id: impl Into<String>,
    ) -> Result<Self, QueryError> {
        let target = Self {
            ecu_family: Some(ecu_family.into()),
            diagnostic_implementation: Some(diagnostic_implementation.into()),
            capability_id: Some(capability_id.into()),
            knowledge_entity: None,
        };
        target.validate()?;
        Ok(target)
    }

    /// A family-level target: the ECU family and the capability, no
    /// implementation. The plan will record the implementation as absent.
    pub fn family(
        ecu_family: impl Into<String>,
        capability_id: impl Into<String>,
    ) -> Result<Self, QueryError> {
        let target = Self {
            ecu_family: Some(ecu_family.into()),
            diagnostic_implementation: None,
            capability_id: Some(capability_id.into()),
            knowledge_entity: None,
        };
        target.validate()?;
        Ok(target)
    }

    /// A family-level target that also names the logical bus the family sits
    /// on, so that knowledge recorded about that bus — such as which adapter
    /// route reaches it — counts as related (ADR-0013).
    pub fn family_on_bus(
        ecu_family: impl Into<String>,
        capability_id: impl Into<String>,
        logical_network: impl Into<String>,
    ) -> Result<Self, QueryError> {
        let target = Self {
            ecu_family: Some(ecu_family.into()),
            diagnostic_implementation: None,
            capability_id: Some(capability_id.into()),
            knowledge_entity: Some(KnowledgeEntity {
                kind: knowledge::EntityKind::NetworkRoute,
                id: logical_network.into(),
            }),
        };
        target.validate()?;
        Ok(target)
    }

    pub fn entity(entity: KnowledgeEntity) -> Result<Self, QueryError> {
        let target = Self {
            knowledge_entity: Some(entity),
            ..Self::default()
        };
        target.validate()?;
        Ok(target)
    }

    fn validate(&self) -> Result<(), QueryError> {
        let values = [
            self.ecu_family.as_deref(),
            self.diagnostic_implementation.as_deref(),
            self.capability_id.as_deref(),
        ];
        if values.iter().flatten().any(|value| value.trim().is_empty()) {
            return Err(QueryError::EmptyTargetValue);
        }
        if values.iter().all(Option::is_none) && self.knowledge_entity.is_none() {
            return Err(QueryError::MissingTarget);
        }
        if let Some(entity) = &self.knowledge_entity {
            entity
                .validate()
                .map_err(|_| QueryError::InvalidKnowledgeEntity)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryError {
    MissingTarget,
    EmptyTargetValue,
    InvalidKnowledgeEntity,
}

impl fmt::Display for QueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTarget => formatter.write_str("diagnostic target is required"),
            Self::EmptyTargetValue => {
                formatter.write_str("diagnostic target values must not be empty")
            }
            Self::InvalidKnowledgeEntity => {
                formatter.write_str("knowledge entity target is invalid")
            }
        }
    }
}

impl std::error::Error for QueryError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticEnvironmentResolution {
    Resolved(DiagnosticEnvironmentPlan),
    Indeterminate {
        partial: PartialDiagnosticEnvironment,
        unresolved_facts: Vec<UnresolvedFact>,
    },
    Conflict {
        partial: PartialDiagnosticEnvironment,
        conflicts: Vec<ResolutionConflict>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticEnvironmentPlan {
    pub vehicle_applicability: PlanField<VehicleApplicability>,
    pub ecu_family: PlanField<String>,
    /// Absent for a family-level plan (ADR-0013): the source describes the
    /// module family and names no software build, so none is invented.
    pub diagnostic_implementation: Option<PlanField<String>>,
    pub logical_network: PlanField<String>,
    pub physical_route: PlanField<PhysicalDiagnosticRoute>,
    pub backend_route: PlanField<BackendRoute>,
    pub bitrate_bps: PlanField<u32>,
    pub protocol_family: PlanField<String>,
    pub addressing_mode: PlanField<String>,
    /// Absent under a serial node addressing (ADR-0029): a K-line module has
    /// a node address and no CAN identifier, so none is required or
    /// invented. Required under every CAN addressing mode.
    pub can_id_format: Option<PlanField<CanIdFormat>>,
    /// How the serial line is framed, in the data's own words
    /// (`data_bits=8;parity=even;stop_bits=1`), and how it is woken
    /// (`bmw_ds2`, `kw2000_fast`, `rosco`). Present only where the bus
    /// states them, which in this corpus is the K-line (`ADR-0029`); a CAN
    /// plan carries neither, and neither is ever invented.
    pub serial_framing: Option<PlanField<String>>,
    pub serial_wakeup: Option<PlanField<String>>,
    pub physical_request_id: PlanField<u32>,
    pub physical_response_id: PlanField<u32>,
    pub functional_request_id: Option<PlanField<u32>>,
    pub read_only_capability: PlanField<ReadOnlyCapability>,
    pub implementation_markers: BTreeMap<ImplementationMarkerKind, PlanField<String>>,
    pub validation_state: ValidationState,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartialDiagnosticEnvironment {
    pub vehicle_applicability: Option<PlanField<VehicleApplicability>>,
    pub ecu_family: Option<PlanField<String>>,
    pub diagnostic_implementation: Option<PlanField<String>>,
    pub logical_network: Option<PlanField<String>>,
    pub physical_route: Option<PlanField<PhysicalDiagnosticRoute>>,
    pub backend_route: Option<PlanField<BackendRoute>>,
    pub bitrate_bps: Option<PlanField<u32>>,
    pub protocol_family: Option<PlanField<String>>,
    pub addressing_mode: Option<PlanField<String>>,
    pub can_id_format: Option<PlanField<CanIdFormat>>,
    pub serial_framing: Option<PlanField<String>>,
    pub serial_wakeup: Option<PlanField<String>>,
    pub physical_request_id: Option<PlanField<u32>>,
    pub physical_response_id: Option<PlanField<u32>>,
    pub functional_request_id: Option<PlanField<u32>>,
    pub read_only_capability: Option<PlanField<ReadOnlyCapability>>,
    pub implementation_markers: BTreeMap<ImplementationMarkerKind, PlanField<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanField<T> {
    pub value: T,
    pub evidence: Vec<PlanEvidenceTrace>,
    pub validation_state: ValidationState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanEvidenceTrace {
    pub record_id: String,
    pub evidence_id: String,
    pub source_id: String,
    pub source_type: SourceType,
    pub evidence_class: Option<EvidenceClass>,
    pub locator: SourceLocator,
    pub validation_state: ValidationState,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VehicleApplicability {
    pub vehicle_program: Option<String>,
    pub model_year: Option<u16>,
    pub architecture_generation: Option<String>,
    pub ecu_family: Option<String>,
    pub powertrain: Option<String>,
    pub variant: Option<String>,
    pub market: Option<String>,
    pub diagnostic_implementation: Option<String>,
    pub other: BTreeMap<String, String>,
}

impl From<&VehicleContext> for VehicleApplicability {
    fn from(context: &VehicleContext) -> Self {
        Self {
            vehicle_program: context.vehicle_program.clone(),
            model_year: context.model_year,
            architecture_generation: context.architecture_generation.clone(),
            ecu_family: context.ecu_family.clone(),
            powertrain: context.powertrain.clone(),
            variant: context.variant.clone(),
            market: context.market.clone(),
            diagnostic_implementation: context.diagnostic_implementation.clone(),
            other: context.other.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalDiagnosticRoute {
    pub connector: String,
    pub pins: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BackendRoute {
    pub backend: String,
    pub route_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReadOnlyCapability {
    pub id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnvironmentField {
    VehicleApplicability,
    EcuFamily,
    DiagnosticImplementation,
    LogicalNetwork,
    PhysicalRoute,
    BackendRoute,
    Bitrate,
    ProtocolFamily,
    AddressingMode,
    CanIdFormat,
    SerialFraming,
    SerialWakeup,
    PhysicalRequestId,
    PhysicalResponseId,
    FunctionalRequestId,
    ReadOnlyCapability,
    EvidenceClassification,
    ImplementationMarker(ImplementationMarkerKind),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnresolvedFact {
    pub field: EnvironmentField,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolutionConflict {
    pub field: EnvironmentField,
    pub candidates: Vec<ConflictCandidate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictCandidate {
    pub value: EnvironmentValue,
    pub evidence: Vec<PlanEvidenceTrace>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnvironmentValue {
    Text(String),
    Number(u32),
    CanIdFormat(CanIdFormat),
    VehicleApplicability(VehicleApplicability),
    PhysicalRoute(PhysicalDiagnosticRoute),
    BackendRoute(BackendRoute),
    ReadOnlyCapability(ReadOnlyCapability),
}

pub struct DiagnosticEnvironmentResolver;

impl DiagnosticEnvironmentResolver {
    pub fn resolve(
        store: &KnowledgeStore,
        query: &DiagnosticEnvironmentQuery,
    ) -> DiagnosticEnvironmentResolution {
        let mut effective_target = query.diagnostic_target.clone();
        if effective_target.ecu_family.is_none() {
            effective_target.ecu_family = query.vehicle_context.ecu_family.clone();
        }
        if effective_target.diagnostic_implementation.is_none() {
            effective_target.diagnostic_implementation =
                query.vehicle_context.diagnostic_implementation.clone();
        }
        let result = store.query(
            &KnowledgeQuery::for_vehicle(query.vehicle_context.clone()).include_indeterminate(true),
        );
        let related: Vec<_> = result
            .records
            .into_iter()
            .filter(|record| record_is_related(record, &effective_target))
            .collect();
        let applicable: Vec<_> = related
            .iter()
            .filter(|record| record.applicability_resolution == ApplicabilityResolution::Applicable)
            .cloned()
            .collect();

        let mut unresolved = BTreeSet::new();
        let mut conflicts = Vec::new();

        let mut vehicle = BTreeMap::new();
        if !applicable.is_empty() {
            vehicle.insert(
                VehicleApplicability::from(&query.vehicle_context),
                applicable.clone(),
            );
        }

        let mut ecu_families = BTreeMap::new();
        let mut implementations = BTreeMap::new();
        let mut logical_networks = BTreeMap::new();
        let mut physical_routes = BTreeMap::new();
        let mut backend_routes = BTreeMap::new();
        let mut bitrates = BTreeMap::new();
        let mut protocols = BTreeMap::new();
        let mut addressing_modes = BTreeMap::new();
        let mut id_formats = BTreeMap::new();
        let mut request_ids = BTreeMap::new();
        let mut response_ids = BTreeMap::new();
        let mut functional_ids = BTreeMap::new();
        let mut capabilities = BTreeMap::new();
        // The serial line's framing and wake-up, which the platform ingest
        // records as text on the bus itself (ADR-0029).
        let mut serial_framings = BTreeMap::new();
        let mut serial_wakeups = BTreeMap::new();
        let mut markers: BTreeMap<
            ImplementationMarkerKind,
            BTreeMap<String, Vec<ResolvedKnowledge>>,
        > = BTreeMap::new();

        for record in &applicable {
            if let Some(expected) = &effective_target.ecu_family {
                if record_mentions_ecu(record, expected) {
                    insert_candidate(&mut ecu_families, expected.clone(), record.clone());
                }
            }
            if let Some(expected) = &effective_target.diagnostic_implementation {
                if record_mentions_implementation(record, expected) {
                    insert_candidate(&mut implementations, expected.clone(), record.clone());
                }
            }

            if let (ClaimKey::Custom { name }, KnowledgeValue::Text { value }) =
                (&record.record.key, &record.record.value)
            {
                if name == ISO_SETTINGS_CLAIM {
                    insert_candidate(&mut serial_framings, value.clone(), record.clone());
                } else if name == ISO_WAKEUP_CLAIM {
                    insert_candidate(&mut serial_wakeups, value.clone(), record.clone());
                }
            }

            match &record.record.value {
                KnowledgeValue::NetworkRoute {
                    logical_name,
                    connector,
                    pins,
                    bitrate_bps,
                } => {
                    insert_candidate(&mut logical_networks, logical_name.clone(), record.clone());
                    if let Some(connector) = connector {
                        if !pins.is_empty() {
                            insert_candidate(
                                &mut physical_routes,
                                PhysicalDiagnosticRoute {
                                    connector: connector.clone(),
                                    pins: pins.clone(),
                                },
                                record.clone(),
                            );
                        }
                    }
                    if let Some(bitrate) = bitrate_bps {
                        insert_candidate(&mut bitrates, *bitrate, record.clone());
                    }
                }
                KnowledgeValue::BackendRoute { backend, route_id } => insert_candidate(
                    &mut backend_routes,
                    BackendRoute {
                        backend: backend.clone(),
                        route_id: route_id.clone(),
                    },
                    record.clone(),
                ),
                KnowledgeValue::ProtocolFamily { name } => {
                    insert_candidate(&mut protocols, name.clone(), record.clone());
                }
                KnowledgeValue::DiagnosticAddressing {
                    request_id,
                    response_id,
                    functional_request_id,
                    can_id_format,
                    addressing_mode,
                } => {
                    if let Some(value) = request_id {
                        insert_candidate(&mut request_ids, *value, record.clone());
                    }
                    if let Some(value) = response_id {
                        insert_candidate(&mut response_ids, *value, record.clone());
                    }
                    if let Some(value) = functional_request_id {
                        insert_candidate(&mut functional_ids, *value, record.clone());
                    }
                    if let Some(value) = can_id_format {
                        insert_candidate(&mut id_formats, *value, record.clone());
                    }
                    if let Some(value) = addressing_mode {
                        insert_candidate(&mut addressing_modes, value.clone(), record.clone());
                    }
                }
                KnowledgeValue::Capability {
                    name,
                    supported: true,
                    safety_class: Some(DiagnosticSafetyClass::ReadOnly),
                } if effective_target
                    .capability_id
                    .as_ref()
                    .is_some_and(|expected| expected == name) =>
                {
                    insert_candidate(
                        &mut capabilities,
                        ReadOnlyCapability { id: name.clone() },
                        record.clone(),
                    );
                }
                KnowledgeValue::ImplementationMarker { marker_kind, value } => {
                    insert_candidate(
                        markers.entry(*marker_kind).or_default(),
                        value.clone(),
                        record.clone(),
                    );
                }
                _ => {}
            }
        }

        let implementation_markers = markers
            .into_iter()
            .filter_map(|(kind, candidates)| {
                select_optional_field(
                    EnvironmentField::ImplementationMarker(kind),
                    candidates,
                    &mut unresolved,
                    &mut conflicts,
                )
                .map(|field| (kind, field))
            })
            .collect();

        let addressing_mode = select_field(
            EnvironmentField::AddressingMode,
            addressing_modes,
            &mut unresolved,
            &mut conflicts,
        );
        // ADR-0029: on a serial line the node address is the whole of the
        // addressing, so a CAN identifier format is required only where the
        // addressing is CAN's. Nothing is invented for the K-line; a CAN
        // module without a format stays unresolved as before.
        let serial_line = addressing_mode
            .as_ref()
            .is_some_and(|mode| mode.value == knowledge::ISO9141_NODE_ADDRESSING_MODE);
        let can_id_format = if serial_line {
            select_optional_field(
                EnvironmentField::CanIdFormat,
                id_formats,
                &mut unresolved,
                &mut conflicts,
            )
        } else {
            select_field(
                EnvironmentField::CanIdFormat,
                id_formats,
                &mut unresolved,
                &mut conflicts,
            )
        };

        // Both are the line's own facts: absent on a CAN plan, and absent
        // on a K-line whose document states neither, where the execution
        // layer refuses rather than assuming a framing.
        let serial_framing = select_optional_field(
            EnvironmentField::SerialFraming,
            serial_framings,
            &mut unresolved,
            &mut conflicts,
        );
        let serial_wakeup = select_optional_field(
            EnvironmentField::SerialWakeup,
            serial_wakeups,
            &mut unresolved,
            &mut conflicts,
        );

        let partial = PartialDiagnosticEnvironment {
            vehicle_applicability: select_field(
                EnvironmentField::VehicleApplicability,
                vehicle,
                &mut unresolved,
                &mut conflicts,
            ),
            ecu_family: select_field(
                EnvironmentField::EcuFamily,
                ecu_families,
                &mut unresolved,
                &mut conflicts,
            ),
            // Mandatory only when the target names one; a family-level target
            // records its absence instead of failing (ADR-0013).
            diagnostic_implementation: if effective_target.diagnostic_implementation.is_some() {
                select_field(
                    EnvironmentField::DiagnosticImplementation,
                    implementations,
                    &mut unresolved,
                    &mut conflicts,
                )
            } else {
                select_optional_field(
                    EnvironmentField::DiagnosticImplementation,
                    implementations,
                    &mut unresolved,
                    &mut conflicts,
                )
            },
            logical_network: select_field(
                EnvironmentField::LogicalNetwork,
                logical_networks,
                &mut unresolved,
                &mut conflicts,
            ),
            physical_route: select_field(
                EnvironmentField::PhysicalRoute,
                physical_routes,
                &mut unresolved,
                &mut conflicts,
            ),
            backend_route: select_field(
                EnvironmentField::BackendRoute,
                backend_routes,
                &mut unresolved,
                &mut conflicts,
            ),
            bitrate_bps: select_field(
                EnvironmentField::Bitrate,
                bitrates,
                &mut unresolved,
                &mut conflicts,
            ),
            protocol_family: select_field(
                EnvironmentField::ProtocolFamily,
                protocols,
                &mut unresolved,
                &mut conflicts,
            ),
            addressing_mode,
            can_id_format,
            serial_framing,
            serial_wakeup,
            physical_request_id: select_field(
                EnvironmentField::PhysicalRequestId,
                request_ids,
                &mut unresolved,
                &mut conflicts,
            ),
            physical_response_id: select_field(
                EnvironmentField::PhysicalResponseId,
                response_ids,
                &mut unresolved,
                &mut conflicts,
            ),
            functional_request_id: select_optional_field(
                EnvironmentField::FunctionalRequestId,
                functional_ids,
                &mut unresolved,
                &mut conflicts,
            ),
            read_only_capability: select_field(
                EnvironmentField::ReadOnlyCapability,
                capabilities,
                &mut unresolved,
                &mut conflicts,
            ),
            implementation_markers,
        };

        conflicts.sort_by_key(|conflict| conflict.field);
        if !conflicts.is_empty() {
            return DiagnosticEnvironmentResolution::Conflict { partial, conflicts };
        }

        if related.iter().any(|record| {
            matches!(
                record.applicability_resolution,
                ApplicabilityResolution::InsufficientContext
                    | ApplicabilityResolution::InsufficientEvidence
            )
        }) && applicable.is_empty()
        {
            unresolved.insert(UnresolvedFact {
                field: EnvironmentField::VehicleApplicability,
                reason: "matching evidence has insufficient context or applicability evidence"
                    .into(),
            });
        }

        if !unresolved.is_empty() {
            return DiagnosticEnvironmentResolution::Indeterminate {
                partial,
                unresolved_facts: unresolved.into_iter().collect(),
            };
        }

        DiagnosticEnvironmentResolution::Resolved(partial.into_complete())
    }
}

impl PartialDiagnosticEnvironment {
    fn into_complete(self) -> DiagnosticEnvironmentPlan {
        let mut fields = vec![
            self.vehicle_applicability
                .as_ref()
                .unwrap()
                .validation_state,
            self.ecu_family.as_ref().unwrap().validation_state,
            self.logical_network.as_ref().unwrap().validation_state,
            self.physical_route.as_ref().unwrap().validation_state,
            self.backend_route.as_ref().unwrap().validation_state,
            self.bitrate_bps.as_ref().unwrap().validation_state,
            self.protocol_family.as_ref().unwrap().validation_state,
            self.addressing_mode.as_ref().unwrap().validation_state,
            self.physical_request_id.as_ref().unwrap().validation_state,
            self.physical_response_id.as_ref().unwrap().validation_state,
            self.read_only_capability.as_ref().unwrap().validation_state,
        ];
        if let Some(can_id_format) = &self.can_id_format {
            fields.push(can_id_format.validation_state);
        }
        if let Some(functional) = &self.functional_request_id {
            fields.push(functional.validation_state);
        }
        if let Some(implementation) = &self.diagnostic_implementation {
            fields.push(implementation.validation_state);
        }
        fields.extend(
            self.implementation_markers
                .values()
                .map(|field| field.validation_state),
        );
        DiagnosticEnvironmentPlan {
            vehicle_applicability: self.vehicle_applicability.unwrap(),
            ecu_family: self.ecu_family.unwrap(),
            diagnostic_implementation: self.diagnostic_implementation,
            logical_network: self.logical_network.unwrap(),
            physical_route: self.physical_route.unwrap(),
            backend_route: self.backend_route.unwrap(),
            bitrate_bps: self.bitrate_bps.unwrap(),
            protocol_family: self.protocol_family.unwrap(),
            addressing_mode: self.addressing_mode.unwrap(),
            can_id_format: self.can_id_format,
            serial_framing: self.serial_framing,
            serial_wakeup: self.serial_wakeup,
            physical_request_id: self.physical_request_id.unwrap(),
            physical_response_id: self.physical_response_id.unwrap(),
            functional_request_id: self.functional_request_id,
            read_only_capability: self.read_only_capability.unwrap(),
            implementation_markers: self.implementation_markers,
            validation_state: weakest_validation(fields),
        }
    }
}

fn record_is_related(record: &ResolvedKnowledge, target: &DiagnosticTarget) -> bool {
    if target
        .knowledge_entity
        .as_ref()
        .is_some_and(|entity| entity == &record.record.entity)
    {
        return true;
    }

    let has_identity = target.ecu_family.is_some() || target.diagnostic_implementation.is_some();
    if !has_identity {
        return target
            .capability_id
            .as_ref()
            .is_some_and(|expected| capability_name(record).is_some_and(|name| name == expected));
    }

    if let Some(expected) = &target.ecu_family {
        if !record_mentions_ecu(record, expected) {
            return false;
        }
    }
    if let Some(expected) = &target.diagnostic_implementation {
        if !record_mentions_implementation(record, expected) {
            return false;
        }
    }
    true
}

fn record_mentions_ecu(record: &ResolvedKnowledge, expected: &str) -> bool {
    (record.record.entity.kind == EntityKind::EcuFamily && record.record.entity.id == expected)
        || dimension_mentions(&record.record.applicability.ecu_family, expected)
}

fn record_mentions_implementation(record: &ResolvedKnowledge, expected: &str) -> bool {
    (record.record.entity.kind == EntityKind::DiagnosticImplementation
        && record.record.entity.id == expected)
        || dimension_mentions(
            &record.record.applicability.diagnostic_implementation,
            expected,
        )
}

fn capability_name(record: &ResolvedKnowledge) -> Option<&str> {
    match &record.record.value {
        KnowledgeValue::Capability { name, .. } => Some(name),
        _ => None,
    }
}

fn dimension_mentions(constraint: &DimensionConstraint, expected: &str) -> bool {
    matches!(
        constraint,
        DimensionConstraint::OneOf { values } if values.iter().any(|value| value == expected)
    )
}

fn insert_candidate<T: Ord>(
    candidates: &mut BTreeMap<T, Vec<ResolvedKnowledge>>,
    value: T,
    record: ResolvedKnowledge,
) {
    candidates.entry(value).or_default().push(record);
}

trait CandidateValue: Clone + Ord {
    fn environment_value(&self) -> EnvironmentValue;
}

impl CandidateValue for String {
    fn environment_value(&self) -> EnvironmentValue {
        EnvironmentValue::Text(self.clone())
    }
}

impl CandidateValue for u32 {
    fn environment_value(&self) -> EnvironmentValue {
        EnvironmentValue::Number(*self)
    }
}

impl CandidateValue for CanIdFormat {
    fn environment_value(&self) -> EnvironmentValue {
        EnvironmentValue::CanIdFormat(*self)
    }
}

impl CandidateValue for VehicleApplicability {
    fn environment_value(&self) -> EnvironmentValue {
        EnvironmentValue::VehicleApplicability(self.clone())
    }
}

impl CandidateValue for PhysicalDiagnosticRoute {
    fn environment_value(&self) -> EnvironmentValue {
        EnvironmentValue::PhysicalRoute(self.clone())
    }
}

impl CandidateValue for BackendRoute {
    fn environment_value(&self) -> EnvironmentValue {
        EnvironmentValue::BackendRoute(self.clone())
    }
}

impl CandidateValue for ReadOnlyCapability {
    fn environment_value(&self) -> EnvironmentValue {
        EnvironmentValue::ReadOnlyCapability(self.clone())
    }
}

fn select_field<T: CandidateValue>(
    field: EnvironmentField,
    candidates: BTreeMap<T, Vec<ResolvedKnowledge>>,
    unresolved: &mut BTreeSet<UnresolvedFact>,
    conflicts: &mut Vec<ResolutionConflict>,
) -> Option<PlanField<T>> {
    if candidates.is_empty() {
        unresolved.insert(UnresolvedFact {
            field,
            reason: "no applicable evidence-backed value".into(),
        });
        return None;
    }
    if candidates.len() > 1 {
        conflicts.push(ResolutionConflict {
            field,
            candidates: candidates
                .iter()
                .map(|(value, records)| ConflictCandidate {
                    value: value.environment_value(),
                    evidence: traces(records),
                })
                .collect(),
        });
        return None;
    }
    let (value, records) = candidates.into_iter().next().unwrap();
    let evidence = traces(&records);
    if evidence.iter().any(|trace| trace.evidence_class.is_none()) {
        unresolved.insert(UnresolvedFact {
            field: EnvironmentField::EvidenceClassification,
            reason: format!("{field:?} has evidence without an explicit evidence class"),
        });
    }
    if records.iter().any(|record| {
        matches!(
            record.record.validation_state,
            ValidationState::Contradicted | ValidationState::Deprecated
        )
    }) {
        unresolved.insert(UnresolvedFact {
            field,
            reason: "selected evidence is contradicted or deprecated".into(),
        });
    }
    Some(PlanField {
        value,
        evidence,
        // Records that agree in value back one fact; it is as validated as
        // the best of them (ADR-0016 §7). The plan as a whole still reports
        // the weakest of its fields.
        validation_state: strongest_validation(
            records.iter().map(|record| record.record.validation_state),
        ),
    })
}

fn select_optional_field<T: CandidateValue>(
    field: EnvironmentField,
    candidates: BTreeMap<T, Vec<ResolvedKnowledge>>,
    unresolved: &mut BTreeSet<UnresolvedFact>,
    conflicts: &mut Vec<ResolutionConflict>,
) -> Option<PlanField<T>> {
    if candidates.is_empty() {
        return None;
    }
    if candidates.len() > 1 {
        conflicts.push(ResolutionConflict {
            field,
            candidates: candidates
                .iter()
                .map(|(value, records)| ConflictCandidate {
                    value: value.environment_value(),
                    evidence: traces(records),
                })
                .collect(),
        });
        return None;
    }
    let (value, records) = candidates.into_iter().next().unwrap();
    let evidence = traces(&records);
    if evidence.iter().any(|trace| trace.evidence_class.is_none()) {
        unresolved.insert(UnresolvedFact {
            field: EnvironmentField::EvidenceClassification,
            reason: format!("{field:?} has evidence without an explicit evidence class"),
        });
    }
    if records.iter().any(|record| {
        matches!(
            record.record.validation_state,
            ValidationState::Contradicted | ValidationState::Deprecated
        )
    }) {
        unresolved.insert(UnresolvedFact {
            field,
            reason: "selected evidence is contradicted or deprecated".into(),
        });
    }
    Some(PlanField {
        value,
        evidence,
        // Records that agree in value back one fact; it is as validated as
        // the best of them (ADR-0016 §7). The plan as a whole still reports
        // the weakest of its fields.
        validation_state: strongest_validation(
            records.iter().map(|record| record.record.validation_state),
        ),
    })
}

fn traces(records: &[ResolvedKnowledge]) -> Vec<PlanEvidenceTrace> {
    let mut traces = Vec::new();
    for record in records {
        for trace in &record.evidence {
            traces.push(PlanEvidenceTrace {
                record_id: record.record.id.clone(),
                evidence_id: trace.evidence.id.0.clone(),
                source_id: trace.source.id.0.clone(),
                source_type: trace.source.source_type,
                evidence_class: trace.evidence.evidence_class,
                locator: trace.evidence.locator.clone(),
                validation_state: record.record.validation_state,
            });
        }
    }
    traces.sort_by(|left, right| {
        (
            &left.record_id,
            &left.evidence_id,
            &left.source_id,
            left.evidence_class,
        )
            .cmp(&(
                &right.record_id,
                &right.evidence_id,
                &right.source_id,
                right.evidence_class,
            ))
    });
    traces.dedup_by(|left, right| {
        left.record_id == right.record_id
            && left.evidence_id == right.evidence_id
            && left.source_id == right.source_id
    });
    traces
}

fn validation_rank(value: ValidationState) -> u8 {
    match value {
        ValidationState::Unverified => 0,
        ValidationState::SourceBacked => 1,
        ValidationState::Corroborated => 2,
        ValidationState::CaptureValidated => 3,
        ValidationState::Contradicted => 0,
        ValidationState::Deprecated => 0,
    }
}

fn strongest_validation(values: impl IntoIterator<Item = ValidationState>) -> ValidationState {
    values
        .into_iter()
        .max_by_key(|value| validation_rank(*value))
        .unwrap_or(ValidationState::Unverified)
}

fn weakest_validation(values: impl IntoIterator<Item = ValidationState>) -> ValidationState {
    values
        .into_iter()
        .min_by_key(|value| match value {
            ValidationState::Unverified => 0,
            ValidationState::SourceBacked => 1,
            ValidationState::Corroborated => 2,
            ValidationState::CaptureValidated => 3,
            ValidationState::Contradicted => 0,
            ValidationState::Deprecated => 0,
        })
        .unwrap_or(ValidationState::Unverified)
}

/// What the knowledge base says about one ECU family on a given vehicle.
///
/// This is the enumeration step F10 needs before it can address more than one
/// module: which ECU families the knowledge associates with a vehicle at all,
/// and how far each one is from a usable route. It answers "what is fitted and
/// what do we still not know", never "what is safe to talk to" — that remains
/// the resolver's job, per family.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EcuFamilyPresence {
    /// ECU family identifier as the knowledge base spells it, such as `PCM`.
    pub ecu_family: String,
    /// Strongest applicability any of its claims reached for this vehicle.
    pub resolution: ApplicabilityResolution,
    /// Whether a diagnostic request identifier is known for it.
    pub has_request_id: bool,
    /// Whether a response identifier is known for it.
    pub has_response_id: bool,
    /// Logical bus the knowledge places it on, when stated.
    pub logical_network: Option<String>,
}

impl EcuFamilyPresence {
    /// Whether both directions of a diagnostic route are known.
    ///
    /// A route is not a permission. Even a complete pair still has to pass the
    /// resolver, which checks protocol, bus parameters, capability and
    /// provenance before anything may be prepared.
    pub fn has_complete_route(&self) -> bool {
        self.has_request_id && self.has_response_id
    }
}

impl DiagnosticEnvironmentResolver {
    /// Enumerate the ECU families the knowledge base associates with a vehicle.
    ///
    /// Families whose applicability rules out this vehicle are excluded.
    /// Everything else is returned with its resolution stated plainly, so a
    /// caller can see the difference between "fitted and routable", "mentioned
    /// but unresolved", and "absent" instead of inferring it.
    pub fn enumerate_ecu_families(
        store: &KnowledgeStore,
        vehicle_context: &VehicleContext,
    ) -> Vec<EcuFamilyPresence> {
        let result = store.query(
            &KnowledgeQuery::for_vehicle(vehicle_context.clone()).include_indeterminate(true),
        );

        let mut found: BTreeMap<String, EcuFamilyPresence> = BTreeMap::new();
        for entry in &result.records {
            if entry.record.entity.kind != EntityKind::EcuFamily {
                continue;
            }
            // Only diagnostic claims place a family on a vehicle: how it is
            // addressed, which bus it sits on, what it speaks and can do.
            // A name, a wording or a recorded observation describes a family
            // without fitting it anywhere.
            let places_family = match &entry.record.key {
                ClaimKey::DiagnosticAddressing
                | ClaimKey::NetworkRoute
                | ClaimKey::UsesProtocolFamily
                | ClaimKey::SupportsCapability { .. } => true,
                ClaimKey::Custom { name } => {
                    name == SDD_NETWORK_CLAIM || name == SDD_PHYSICAL_ADDRESS_CLAIM
                }
                _ => false,
            };
            if !places_family {
                continue;
            }
            let presence = found
                .entry(entry.record.entity.id.clone())
                .or_insert_with(|| EcuFamilyPresence {
                    ecu_family: entry.record.entity.id.clone(),
                    resolution: entry.applicability_resolution,
                    has_request_id: false,
                    has_response_id: false,
                    logical_network: None,
                });

            // Keep the strongest applicability any claim about the family
            // reached; a single resolvable claim is what makes it worth asking
            // the resolver about.
            if rank(entry.applicability_resolution) > rank(presence.resolution) {
                presence.resolution = entry.applicability_resolution;
            }

            match &entry.record.value {
                KnowledgeValue::DiagnosticAddressing {
                    request_id,
                    response_id,
                    ..
                } => {
                    presence.has_request_id |= request_id.is_some();
                    presence.has_response_id |= response_id.is_some();
                }
                KnowledgeValue::Text { value } if presence.logical_network.is_none() => {
                    if matches!(&entry.record.key, ClaimKey::Custom { name } if name == SDD_NETWORK_CLAIM)
                    {
                        presence.logical_network = Some(value.clone());
                    }
                }
                _ => {}
            }
        }
        found.into_values().collect()
    }
}

/// Order applicability outcomes from least to most usable.
fn rank(resolution: ApplicabilityResolution) -> u8 {
    match resolution {
        ApplicabilityResolution::NotApplicable => 0,
        ApplicabilityResolution::InsufficientEvidence => 1,
        ApplicabilityResolution::InsufficientContext => 2,
        ApplicabilityResolution::Applicable => 3,
    }
}

/// Claim name under which the SDD platform adapter states the bus a module
/// sits on. Read here so F10 can find a module's bus; never written here.
pub const SDD_NETWORK_CLAIM: &str = "sdd_network";
/// The claim under which the SDD ingester records a module's physical (node)
/// address when the bus derives identifiers from it; it places the module
/// on its vehicle as an addressing claim does.
pub const SDD_PHYSICAL_ADDRESS_CLAIM: &str = "sdd_physical_address";

/// One formatted parameter of a readable identifier, as the DID catalogue
/// describes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadableParameter {
    pub name: String,
    pub encoding: Option<String>,
    pub unit: Option<String>,
}

/// An identifier the knowledge base states is readable from a module.
///
/// Presence in this list is the ADR-0012 gate for ReadDataByIdentifier: the
/// DID catalogue is read-only by construction, so an identifier is readable
/// exactly when the catalogue lists it for the module and the vehicle. The
/// evidence travels with it so a prepared read can show why it was allowed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadableIdentifier {
    pub identifier: u16,
    pub parameters: Vec<ReadableParameter>,
    pub evidence: Vec<PlanEvidenceTrace>,
    pub validation_state: ValidationState,
}

impl DiagnosticEnvironmentResolver {
    /// Resolve a plan for an ECU family the vehicle carries (ADR-0013).
    ///
    /// Two hops: the family's own claims say which bus it sits on; the target
    /// then names that bus so knowledge about the bus — its adapter route and
    /// pins — is related. A family with no stated bus, or with more than one,
    /// is resolved without a bus entity, so the missing route or the conflict
    /// surfaces from the resolver instead of being decided here.
    pub fn resolve_ecu_family(
        store: &KnowledgeStore,
        vehicle_context: &VehicleContext,
        ecu_family: &str,
        capability_id: &str,
    ) -> Result<DiagnosticEnvironmentResolution, QueryError> {
        let mut context = vehicle_context.clone();
        context.ecu_family = Some(ecu_family.to_string());
        let buses = stated_buses(store, &context, ecu_family);
        let target = match buses.as_slice() {
            [bus] => DiagnosticTarget::family_on_bus(ecu_family, capability_id, bus.as_str())?,
            _ => DiagnosticTarget::family(ecu_family, capability_id)?,
        };
        let query = DiagnosticEnvironmentQuery::new(context, target)?;
        Ok(Self::resolve(store, &query))
    }

    /// Identifiers the DID catalogue lists as readable from this family on this
    /// vehicle. Catalogue entries not qualified to the family are formatting
    /// knowledge, not evidence that the module exposes the identifier, so they
    /// are excluded rather than assumed.
    pub fn readable_identifiers(
        store: &KnowledgeStore,
        vehicle_context: &VehicleContext,
        ecu_family: &str,
    ) -> Vec<ReadableIdentifier> {
        let mut context = vehicle_context.clone();
        context.ecu_family = Some(ecu_family.to_string());
        let result = store.query(&KnowledgeQuery::for_vehicle(context).include_indeterminate(true));

        let mut found: BTreeMap<u16, ReadableIdentifier> = BTreeMap::new();
        for entry in &result.records {
            if entry.applicability_resolution != ApplicabilityResolution::Applicable
                || entry.record.entity.kind != knowledge::EntityKind::IdentifierParameter
                || !record_mentions_ecu(entry, ecu_family)
            {
                continue;
            }
            let (
                KnowledgeValue::IdentifierDefinition {
                    identifier,
                    encoding,
                    unit,
                },
                ClaimKey::ParameterDefinition { parameter },
            ) = (&entry.record.value, &entry.record.key)
            else {
                continue;
            };
            // An identifier that cannot be expressed as a 16-bit DID cannot be
            // requested, so it is not readable by this application.
            let Some(number) = parse_identifier(identifier) else {
                continue;
            };
            let item = found.entry(number).or_insert_with(|| ReadableIdentifier {
                identifier: number,
                parameters: Vec::new(),
                evidence: Vec::new(),
                validation_state: entry.record.validation_state,
            });
            item.parameters.push(ReadableParameter {
                name: parameter.clone(),
                encoding: encoding.clone(),
                unit: unit.clone(),
            });
            item.evidence.extend(traces(std::slice::from_ref(entry)));
            item.validation_state =
                weakest_validation(vec![item.validation_state, entry.record.validation_state]);
        }
        found.into_values().collect()
    }
}

/// Buses the family's own applicable claims place it on, deduplicated.
fn stated_buses(store: &KnowledgeStore, context: &VehicleContext, ecu_family: &str) -> Vec<String> {
    let result =
        store.query(&KnowledgeQuery::for_vehicle(context.clone()).include_indeterminate(true));
    let mut buses = BTreeSet::new();
    for entry in &result.records {
        if entry.applicability_resolution != ApplicabilityResolution::Applicable
            || entry.record.entity.kind != knowledge::EntityKind::EcuFamily
            || entry.record.entity.id != ecu_family
        {
            continue;
        }
        if let (ClaimKey::Custom { name }, KnowledgeValue::Text { value }) =
            (&entry.record.key, &entry.record.value)
        {
            if name == SDD_NETWORK_CLAIM {
                buses.insert(value.clone());
            }
        }
    }
    buses.into_iter().collect()
}

fn parse_identifier(text: &str) -> Option<u16> {
    let digits = text
        .trim()
        .strip_prefix("0x")
        .or_else(|| text.trim().strip_prefix("0X"))?;
    u16::from_str_radix(digits, 16).ok()
}
