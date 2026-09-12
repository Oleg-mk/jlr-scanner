use crate::source::validate_stable_id;
use crate::{Applicability, EvidenceId, KnowledgeError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    VehicleProgram,
    EcuFamily,
    DiagnosticImplementation,
    ProtocolFamily,
    NetworkRoute,
    DiagnosticAddressing,
    DiagnosticCapability,
    IdentifierParameter,
    /// A fault code. See ADR-0010.
    DiagnosticTroubleCode,
    /// One of SDD's VIN decode models: the rules that place a VIN in it and
    /// the attributes read off such a VIN (F11, VIN decoding).
    VinDecodeModel,
    /// One parameter of a car's configuration file: where it sits in which
    /// block, its type, its options and their texts (ADR-0028). A layout,
    /// not a value: the value comes from the car.
    ConfigurationParameter,
}

/// The addressing mode of a module on a single serial line — a K-line — where
/// the one-byte node address is the whole of the addressing and no CAN
/// identifier format exists (ADR-0029). The ingest records it, the resolver
/// asks for no CAN identifier format under it, and a CAN path refuses it.
pub const ISO9141_NODE_ADDRESSING_MODE: &str = "iso9141_node";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CanIdFormat {
    Standard11Bit,
    Extended29Bit,
}

impl CanIdFormat {
    pub fn maximum_id(self) -> u32 {
        match self {
            Self::Standard11Bit => 0x7ff,
            Self::Extended29Bit => 0x1fff_ffff,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSafetyClass {
    ReadOnly,
    VolatileControl,
    ServiceRoutine,
    PersistentChange,
    ForbiddenProgramming,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ImplementationMarkerKind {
    ModuleAcronym,
    HardwareFamily,
    HardwarePartNumber,
    SoftwarePartNumber,
    CalibrationId,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeEntity {
    pub kind: EntityKind,
    pub id: String,
}

impl KnowledgeEntity {
    pub fn validate(&self) -> Result<(), KnowledgeError> {
        validate_stable_id(&self.id)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClaimKey {
    Alias,
    UsesProtocolFamily,
    NetworkRoute,
    DiagnosticAddressing,
    SupportsCapability { capability: String },
    IdentifierDefinition { namespace: String },
    ParameterDefinition { parameter: String },
    Custom { name: String },
}

impl ClaimKey {
    pub fn validate(&self) -> Result<(), KnowledgeError> {
        let optional_name = match self {
            Self::SupportsCapability { capability } => Some(capability),
            Self::IdentifierDefinition { namespace } => Some(namespace),
            Self::ParameterDefinition { parameter } => Some(parameter),
            Self::Custom { name } => Some(name),
            _ => None,
        };
        if optional_name.is_some_and(|value| value.trim().is_empty()) {
            return Err(KnowledgeError::InvalidRecord(
                "claim key name must not be empty".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum KnowledgeValue {
    Text {
        value: String,
    },
    TextSet {
        values: Vec<String>,
    },
    ProtocolFamily {
        name: String,
    },
    NetworkRoute {
        logical_name: String,
        connector: Option<String>,
        pins: Vec<u8>,
        bitrate_bps: Option<u32>,
    },
    BackendRoute {
        backend: String,
        route_id: String,
    },
    DiagnosticAddressing {
        request_id: Option<u32>,
        response_id: Option<u32>,
        #[serde(default)]
        functional_request_id: Option<u32>,
        #[serde(default)]
        can_id_format: Option<CanIdFormat>,
        addressing_mode: Option<String>,
    },
    Capability {
        name: String,
        supported: bool,
        #[serde(default)]
        safety_class: Option<DiagnosticSafetyClass>,
    },
    ImplementationMarker {
        marker_kind: ImplementationMarkerKind,
        value: String,
    },
    IdentifierDefinition {
        identifier: String,
        encoding: Option<String>,
        unit: Option<String>,
    },
    Unknown {
        reason: String,
    },
}

impl KnowledgeValue {
    pub fn validate(&self) -> Result<(), KnowledgeError> {
        let invalid = match self {
            Self::Text { value } => value.trim().is_empty(),
            Self::TextSet { values } => {
                values.is_empty()
                    || values.iter().any(|value| value.trim().is_empty())
                    || values.windows(2).any(|pair| pair[0] >= pair[1])
            }
            Self::ProtocolFamily { name } => name.trim().is_empty(),
            Self::NetworkRoute {
                logical_name,
                connector,
                pins,
                ..
            } => {
                logical_name.trim().is_empty()
                    || connector
                        .as_ref()
                        .is_some_and(|value| value.trim().is_empty())
                    || pins.windows(2).any(|pair| pair[0] >= pair[1])
                    || pins.iter().any(|pin| !(1..=16).contains(pin))
            }
            Self::BackendRoute { backend, route_id } => {
                backend.trim().is_empty() || route_id.trim().is_empty()
            }
            Self::DiagnosticAddressing {
                request_id,
                response_id,
                functional_request_id,
                can_id_format,
                addressing_mode,
            } => {
                let empty = request_id.is_none()
                    && response_id.is_none()
                    && functional_request_id.is_none()
                    && addressing_mode
                        .as_ref()
                        .map_or(true, |value| value.trim().is_empty());
                let invalid_id = [request_id, response_id, functional_request_id]
                    .into_iter()
                    .flatten()
                    .any(|id| *id > 0x1fff_ffff);
                let invalid_for_format = can_id_format.is_some_and(|format| {
                    [request_id, response_id, functional_request_id]
                        .into_iter()
                        .flatten()
                        .any(|id| *id > format.maximum_id())
                });
                empty || invalid_id || invalid_for_format
            }
            Self::Capability { name, .. } => name.trim().is_empty(),
            Self::ImplementationMarker { value, .. } => value.trim().is_empty(),
            Self::IdentifierDefinition { identifier, .. } => identifier.trim().is_empty(),
            Self::Unknown { reason } => reason.trim().is_empty(),
        };
        if invalid {
            return Err(KnowledgeError::InvalidRecord(
                "knowledge value is empty, unsorted, or structurally invalid".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ValidationState {
    Unverified,
    SourceBacked,
    Corroborated,
    CaptureValidated,
    Contradicted,
    Deprecated,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeRecord {
    pub id: String,
    pub entity: KnowledgeEntity,
    pub key: ClaimKey,
    pub value: KnowledgeValue,
    pub applicability: Applicability,
    pub evidence_ids: Vec<EvidenceId>,
    pub validation_state: ValidationState,
}

impl KnowledgeRecord {
    pub fn validate_shape(&self) -> Result<(), KnowledgeError> {
        validate_stable_id(&self.id)?;
        self.entity.validate()?;
        self.key.validate()?;
        self.value.validate()?;
        self.applicability.validate()?;
        if self.evidence_ids.is_empty() {
            return Err(KnowledgeError::InvalidRecord(
                "knowledge record requires evidence".into(),
            ));
        }
        if self.evidence_ids.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(KnowledgeError::InvalidRecord(
                "evidence IDs must be sorted and unique".into(),
            ));
        }
        Ok(())
    }
}
