use crate::{
    EvidenceRecord, KnowledgeError, KnowledgeRecord, KnowledgeStore, SourceId, SourceRecord,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const KNOWLEDGE_SCHEMA_VERSION: u32 = 1;
const JSON_MANIFEST_PARSER_ID: &str = "jlr-knowledge-json-manifest";
const JSON_MANIFEST_PARSER_VERSION: &str = "1.0.0";

pub trait IngestionAdapter {
    fn parser_id(&self) -> &'static str;
    fn parser_version(&self) -> &'static str;
    fn parse(&self, input: &str) -> Result<IngestionBatch, KnowledgeError>;
}

/// Serialises to exactly the JSON manifest shape `JsonManifestAdapter` reads,
/// so a batch any adapter produced can be written out and loaded back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IngestionBatch {
    pub schema_version: u32,
    pub source: SourceRecord,
    pub evidence: Vec<EvidenceRecord>,
    pub records: Vec<KnowledgeRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IngestionReceipt {
    pub schema_version: u32,
    pub parser_id: String,
    pub parser_version: String,
    pub source_id: SourceId,
    pub evidence_ids: Vec<String>,
    pub record_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct JsonManifestAdapter;

impl IngestionAdapter for JsonManifestAdapter {
    fn parser_id(&self) -> &'static str {
        JSON_MANIFEST_PARSER_ID
    }

    fn parser_version(&self) -> &'static str {
        JSON_MANIFEST_PARSER_VERSION
    }

    fn parse(&self, input: &str) -> Result<IngestionBatch, KnowledgeError> {
        let manifest: IngestionManifest = serde_json::from_str(input)
            .map_err(|error| KnowledgeError::Parse(error.to_string()))?;
        if manifest.schema_version != KNOWLEDGE_SCHEMA_VERSION {
            return Err(KnowledgeError::UnsupportedSchema(manifest.schema_version));
        }
        manifest.source.validate()?;
        ensure_unique(
            manifest
                .evidence
                .iter()
                .map(|evidence| evidence.id.0.as_str()),
            "evidence",
        )?;
        ensure_unique(
            manifest.records.iter().map(|record| record.id.as_str()),
            "knowledge record",
        )?;
        for record in &manifest.records {
            record.validate_shape()?;
        }

        let mut evidence = manifest.evidence;
        evidence.sort_by(|left, right| left.id.cmp(&right.id));
        let mut records = manifest.records;
        records.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(IngestionBatch {
            schema_version: manifest.schema_version,
            source: manifest.source,
            evidence,
            records,
        })
    }
}

impl KnowledgeStore {
    pub fn ingest(
        &mut self,
        adapter: &impl IngestionAdapter,
        input: &str,
    ) -> Result<IngestionReceipt, KnowledgeError> {
        let batch = adapter.parse(input)?;
        self.ingest_batch(adapter.parser_id(), adapter.parser_version(), batch)
    }

    /// Ingest an already-parsed batch. Atomic: a rejected batch leaves no
    /// partial write, and the store is not cloned to achieve that.
    pub fn ingest_batch(
        &mut self,
        parser_id: &str,
        parser_version: &str,
        batch: IngestionBatch,
    ) -> Result<IngestionReceipt, KnowledgeError> {
        let receipt = IngestionReceipt {
            schema_version: batch.schema_version,
            parser_id: parser_id.into(),
            parser_version: parser_version.into(),
            source_id: batch.source.id.clone(),
            evidence_ids: batch
                .evidence
                .iter()
                .map(|evidence| evidence.id.0.clone())
                .collect(),
            record_ids: batch
                .records
                .iter()
                .map(|record| record.id.clone())
                .collect(),
        };
        self.apply_batch(batch)?;
        Ok(receipt)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IngestionManifest {
    schema_version: u32,
    source: SourceRecord,
    evidence: Vec<EvidenceRecord>,
    records: Vec<KnowledgeRecord>,
}

fn ensure_unique<'a>(
    values: impl IntoIterator<Item = &'a str>,
    kind: &'static str,
) -> Result<(), KnowledgeError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(KnowledgeError::DuplicateConflict {
                kind,
                id: value.into(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SYNTHETIC: &str =
        include_str!("../../../fixtures/knowledge/synthetic/f5_architecture.json");

    #[test]
    fn valid_manifest_is_deterministic_and_idempotent() {
        let adapter = JsonManifestAdapter;
        let mut first = KnowledgeStore::new();
        let first_receipt = first.ingest(&adapter, SYNTHETIC).unwrap();
        let second_receipt = first.ingest(&adapter, SYNTHETIC).unwrap();
        assert_eq!(first_receipt, second_receipt);

        let mut second = KnowledgeStore::new();
        assert_eq!(second.ingest(&adapter, SYNTHETIC).unwrap(), first_receipt);
    }

    #[test]
    fn malformed_and_unsupported_fields_are_rejected_without_partial_write() {
        let adapter = JsonManifestAdapter;
        let mut store = KnowledgeStore::new();
        assert!(store.ingest(&adapter, "{").is_err());
        assert!(store
            .ingest(
                &adapter,
                &SYNTHETIC.replace(
                    "\"schema_version\": 1",
                    "\"schema_version\": 1, \"unsupported_semantics\": true"
                )
            )
            .is_err());
        assert_eq!(store.sources().iter().count(), 0);
    }
}
