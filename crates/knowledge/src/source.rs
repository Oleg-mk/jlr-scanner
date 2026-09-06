use crate::KnowledgeError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct SourceId(pub String);

impl SourceId {
    pub fn new(value: impl Into<String>) -> Result<Self, KnowledgeError> {
        let value = value.into();
        validate_stable_id(&value)?;
        Ok(Self(value))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Documented,
    Captured,
    Synthetic,
    UnverifiedResearch,
}

impl SourceType {
    pub fn is_real_evidence(self) -> bool {
        matches!(self, Self::Documented | Self::Captured)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RedistributionStatus {
    Permitted,
    RestrictedMetadataOnly,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContentFingerprint {
    pub algorithm: String,
    pub value: String,
}

impl ContentFingerprint {
    pub fn sha256(value: impl Into<String>) -> Result<Self, KnowledgeError> {
        let fingerprint = Self {
            algorithm: "sha256".into(),
            value: value.into().to_ascii_lowercase(),
        };
        fingerprint.validate()?;
        Ok(fingerprint)
    }

    pub fn validate(&self) -> Result<(), KnowledgeError> {
        if self.algorithm != "sha256" {
            return Err(KnowledgeError::InvalidFingerprint(
                "only sha256 is supported in schema v1".into(),
            ));
        }
        if self.value.len() != 64 || !self.value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(KnowledgeError::InvalidFingerprint(
                "SHA-256 must contain exactly 64 hexadecimal characters".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceRecord {
    pub id: SourceId,
    pub title: String,
    pub source_type: SourceType,
    pub origin: String,
    pub source_locator: String,
    pub content_fingerprint: Option<ContentFingerprint>,
    pub acquired_on: Option<String>,
    #[serde(default)]
    pub declared_vehicle_programs: Vec<String>,
    pub provenance: String,
    pub redistribution_status: RedistributionStatus,
    pub notes: Option<String>,
}

impl SourceRecord {
    pub fn validate(&self) -> Result<(), KnowledgeError> {
        validate_stable_id(&self.id.0)?;
        for (name, value) in [
            ("title", &self.title),
            ("origin", &self.origin),
            ("source_locator", &self.source_locator),
            ("provenance", &self.provenance),
        ] {
            if value.trim().is_empty() {
                return Err(KnowledgeError::InvalidSource(format!(
                    "{name} must not be empty"
                )));
            }
        }
        if let Some(fingerprint) = &self.content_fingerprint {
            fingerprint.validate()?;
        }
        if self.source_type.is_real_evidence() && self.content_fingerprint.is_none() {
            return Err(KnowledgeError::InvalidSource(
                "documented/captured sources require a content fingerprint".into(),
            ));
        }
        if self
            .declared_vehicle_programs
            .iter()
            .any(|program| program.trim().is_empty())
        {
            return Err(KnowledgeError::InvalidSource(
                "declared vehicle programs must not be empty strings".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationOutcome {
    Inserted,
    ExistingEquivalent,
}

#[derive(Clone, Debug, Default)]
pub struct SourceRegistry {
    records: BTreeMap<SourceId, SourceRecord>,
}

impl SourceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        record: SourceRecord,
    ) -> Result<RegistrationOutcome, KnowledgeError> {
        record.validate()?;
        match self.records.get(&record.id) {
            Some(existing) if existing == &record => Ok(RegistrationOutcome::ExistingEquivalent),
            Some(_) => Err(KnowledgeError::DuplicateConflict {
                kind: "source",
                id: record.id.0,
            }),
            None => {
                self.records.insert(record.id.clone(), record);
                Ok(RegistrationOutcome::Inserted)
            }
        }
    }

    pub fn register_local_file(
        &mut self,
        mut record: SourceRecord,
        path: impl AsRef<Path>,
    ) -> Result<RegistrationOutcome, KnowledgeError> {
        record.content_fingerprint = Some(ContentFingerprint::sha256(sha256_file(path)?)?);
        self.register(record)
    }

    pub fn get(&self, id: &SourceId) -> Option<&SourceRecord> {
        self.records.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &SourceRecord> {
        self.records.values()
    }
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn sha256_file(path: impl AsRef<Path>) -> Result<String, KnowledgeError> {
    let mut file = File::open(path).map_err(|error| KnowledgeError::Io(error.to_string()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 16 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| KnowledgeError::Io(error.to_string()))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub(crate) fn validate_stable_id(value: &str) -> Result<(), KnowledgeError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(KnowledgeError::InvalidId(value.into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn source(id: &str) -> SourceRecord {
        SourceRecord {
            id: SourceId::new(id).unwrap(),
            title: "Synthetic source".into(),
            source_type: SourceType::Synthetic,
            origin: "F5 unit test".into(),
            source_locator: "inline:test".into(),
            content_fingerprint: None,
            acquired_on: None,
            declared_vehicle_programs: vec![],
            provenance: "Generated only for a focused unit test".into(),
            redistribution_status: RedistributionStatus::Permitted,
            notes: None,
        }
    }

    #[test]
    fn local_file_hash_is_stable_and_not_filename_based() {
        let path = std::env::temp_dir().join(format!(
            "jlr-knowledge-source-{}-{}.txt",
            std::process::id(),
            1
        ));
        fs::write(&path, b"stable source content").unwrap();
        let first = sha256_file(&path).unwrap();
        let second = sha256_file(&path).unwrap();
        fs::remove_file(&path).unwrap();
        assert_eq!(first, second);
        assert_eq!(first, sha256_bytes(b"stable source content"));
    }

    #[test]
    fn duplicate_equivalent_source_is_idempotent_but_conflict_is_rejected() {
        let mut registry = SourceRegistry::new();
        let record = source("source-a");
        assert_eq!(
            registry.register(record.clone()).unwrap(),
            RegistrationOutcome::Inserted
        );
        assert_eq!(
            registry.register(record).unwrap(),
            RegistrationOutcome::ExistingEquivalent
        );
        let mut conflicting = source("source-a");
        conflicting.title = "Changed".into();
        assert!(matches!(
            registry.register(conflicting),
            Err(KnowledgeError::DuplicateConflict { kind: "source", .. })
        ));
    }

    #[test]
    fn missing_provenance_and_real_source_hash_are_rejected() {
        let mut invalid = source("invalid");
        invalid.provenance.clear();
        assert!(invalid.validate().is_err());
        invalid.provenance = "traceable".into();
        invalid.source_type = SourceType::Documented;
        assert!(invalid.validate().is_err());
    }
}
