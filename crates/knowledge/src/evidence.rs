use crate::source::validate_stable_id;
use crate::{KnowledgeError, SourceId, SourceRegistry};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    DirectObservation,
    OemDocumentation,
    StandardDocumentation,
    SourceCode,
    SecondaryCorroboration,
    SyntheticTest,
    UnverifiedResearch,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct EvidenceId(pub String);

impl EvidenceId {
    pub fn new(value: impl Into<String>) -> Result<Self, KnowledgeError> {
        let value = value.into();
        validate_stable_id(&value)?;
        Ok(Self(value))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceLocator {
    pub description: String,
    pub document_page: Option<u32>,
    pub document_section: Option<String>,
    pub record_key: Option<String>,
    pub capture_timestamp_us: Option<u64>,
}

impl SourceLocator {
    pub fn validate(&self) -> Result<(), KnowledgeError> {
        if self.description.trim().is_empty() {
            return Err(KnowledgeError::InvalidEvidence(
                "source locator description must not be empty".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRecord {
    pub id: EvidenceId,
    pub source_id: SourceId,
    #[serde(default)]
    pub evidence_class: Option<EvidenceClass>,
    pub locator: SourceLocator,
    pub excerpt: Option<String>,
    pub notes: Option<String>,
}

impl EvidenceRecord {
    pub fn validate(&self, sources: &SourceRegistry) -> Result<(), KnowledgeError> {
        validate_stable_id(&self.id.0)?;
        self.locator.validate()?;
        let source = sources
            .get(&self.source_id)
            .ok_or_else(|| KnowledgeError::MissingSource(self.source_id.0.clone()))?;
        let invalid_class = match (source.source_type, self.evidence_class) {
            (crate::SourceType::Synthetic, Some(EvidenceClass::SyntheticTest)) => false,
            (crate::SourceType::Synthetic, _) => true,
            (crate::SourceType::Captured, Some(EvidenceClass::DirectObservation)) => false,
            (crate::SourceType::Captured, _) => true,
            (
                crate::SourceType::Documented,
                Some(EvidenceClass::SyntheticTest | EvidenceClass::DirectObservation),
            ) => true,
            (
                crate::SourceType::UnverifiedResearch,
                Some(EvidenceClass::UnverifiedResearch | EvidenceClass::SecondaryCorroboration),
            ) => false,
            (crate::SourceType::UnverifiedResearch, _) => true,
            _ => false,
        };
        if invalid_class {
            return Err(KnowledgeError::InvalidEvidence(
                "evidence class is incompatible with the registered source type".into(),
            ));
        }
        if self
            .excerpt
            .as_ref()
            .is_some_and(|excerpt| excerpt.chars().count() > 500)
        {
            return Err(KnowledgeError::InvalidEvidence(
                "excerpt exceeds the 500-character traceability limit".into(),
            ));
        }
        Ok(())
    }
}
