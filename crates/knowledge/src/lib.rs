//! Evidence-first JLR knowledge foundation.
//!
//! This crate is read-only domain infrastructure. It has no protocol,
//! transport, operating-system, application-shell, or UI dependency.

mod applicability;
/// Which identifiers are the car's battery, and what each one is for
/// (ADR-0030).
pub mod battery;
mod error;
mod evidence;
mod ingestion;
/// Manifests on a disk: what counts as one, and how to read and write it
/// whether it is packed or not (ADR-0023).
pub mod manifest_file;
mod model;
mod source;
mod store;

pub use applicability::{
    Applicability, ApplicabilityResolution, DimensionConstraint, VehicleContext, YearConstraint,
};
pub use battery::{battery_role, is_headline as is_headline_battery_parameter, BatteryRole};
pub use error::KnowledgeError;
pub use evidence::{EvidenceClass, EvidenceId, EvidenceRecord, SourceLocator};
pub use ingestion::{
    IngestionAdapter, IngestionBatch, IngestionReceipt, JsonManifestAdapter,
    KNOWLEDGE_SCHEMA_VERSION,
};
pub use model::{
    CanIdFormat, ClaimKey, DiagnosticSafetyClass, EntityKind, ImplementationMarkerKind,
    KnowledgeEntity, KnowledgeRecord, KnowledgeValue, ValidationState,
    ISO9141_NODE_ADDRESSING_MODE,
};
pub use source::{
    sha256_bytes, sha256_file, ContentFingerprint, RedistributionStatus, RegistrationOutcome,
    SourceId, SourceRecord, SourceRegistry, SourceType,
};
pub use store::{
    ConflictReport, EvidenceTrace, KnowledgeQuery, KnowledgeQueryResult, KnowledgeStore,
    ResolvedKnowledge,
};
