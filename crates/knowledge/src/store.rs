use crate::{
    ApplicabilityResolution, EntityKind, EvidenceId, EvidenceRecord, IngestionBatch,
    KnowledgeError, KnowledgeRecord, KnowledgeValue, RegistrationOutcome, SourceRecord,
    SourceRegistry, SourceType, ValidationState, VehicleContext,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default)]
pub struct KnowledgeStore {
    sources: SourceRegistry,
    evidence: BTreeMap<EvidenceId, EvidenceRecord>,
    records: BTreeMap<String, KnowledgeRecord>,
}

impl KnowledgeStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sources(&self) -> &SourceRegistry {
        &self.sources
    }

    pub fn register_source(
        &mut self,
        source: SourceRecord,
    ) -> Result<RegistrationOutcome, KnowledgeError> {
        self.sources.register(source)
    }

    pub fn add_evidence(
        &mut self,
        evidence: EvidenceRecord,
    ) -> Result<RegistrationOutcome, KnowledgeError> {
        evidence.validate(&self.sources)?;
        match self.evidence.get(&evidence.id) {
            Some(existing) if existing == &evidence => Ok(RegistrationOutcome::ExistingEquivalent),
            Some(_) => Err(KnowledgeError::DuplicateConflict {
                kind: "evidence",
                id: evidence.id.0,
            }),
            None => {
                self.evidence.insert(evidence.id.clone(), evidence);
                Ok(RegistrationOutcome::Inserted)
            }
        }
    }

    pub fn add_record(
        &mut self,
        record: KnowledgeRecord,
    ) -> Result<RegistrationOutcome, KnowledgeError> {
        record.validate_shape()?;
        self.validate_record_evidence(&record)?;
        match self.records.get(&record.id) {
            Some(existing) if existing == &record => Ok(RegistrationOutcome::ExistingEquivalent),
            Some(_) => Err(KnowledgeError::DuplicateConflict {
                kind: "knowledge record",
                id: record.id,
            }),
            None => {
                self.records.insert(record.id.clone(), record);
                Ok(RegistrationOutcome::Inserted)
            }
        }
    }

    pub fn get_record(&self, id: &str) -> Option<&KnowledgeRecord> {
        self.records.get(id)
    }

    pub fn get_evidence(&self, id: &EvidenceId) -> Option<&EvidenceRecord> {
        self.evidence.get(id)
    }

    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    pub fn evidence_count(&self) -> usize {
        self.evidence.len()
    }

    /// Apply a parsed batch atomically without cloning the store.
    ///
    /// Phase one validates the source, every evidence record and every
    /// knowledge record against the current store plus the batch itself and
    /// mutates nothing, so a rejected batch leaves no partial write. Phase two
    /// inserts. Only the source registry, which is small, is copied; a store
    /// holding the whole SDD corpus is never cloned per manifest.
    pub(crate) fn apply_batch(&mut self, batch: IngestionBatch) -> Result<(), KnowledgeError> {
        let mut sources = self.sources.clone();
        sources.register(batch.source.clone())?;
        {
            let mut pending_evidence: BTreeMap<&EvidenceId, &EvidenceRecord> = BTreeMap::new();
            for evidence in &batch.evidence {
                evidence.validate(&sources)?;
                let conflict = || KnowledgeError::DuplicateConflict {
                    kind: "evidence",
                    id: evidence.id.0.clone(),
                };
                if self
                    .evidence
                    .get(&evidence.id)
                    .is_some_and(|existing| existing != evidence)
                {
                    return Err(conflict());
                }
                if pending_evidence
                    .insert(&evidence.id, evidence)
                    .is_some_and(|previous| previous != evidence)
                {
                    return Err(conflict());
                }
            }
            let mut pending_records: BTreeMap<&str, &KnowledgeRecord> = BTreeMap::new();
            for record in &batch.records {
                record.validate_shape()?;
                validate_record_evidence_with(record, &sources, |id| {
                    pending_evidence
                        .get(id)
                        .copied()
                        .or_else(|| self.evidence.get(id))
                })?;
                let conflict = || KnowledgeError::DuplicateConflict {
                    kind: "knowledge record",
                    id: record.id.clone(),
                };
                if self
                    .records
                    .get(&record.id)
                    .is_some_and(|existing| existing != record)
                {
                    return Err(conflict());
                }
                if pending_records
                    .insert(&record.id, record)
                    .is_some_and(|previous| previous != record)
                {
                    return Err(conflict());
                }
            }
        }
        self.sources = sources;
        for evidence in batch.evidence {
            self.evidence.insert(evidence.id.clone(), evidence);
        }
        for record in batch.records {
            self.records.insert(record.id.clone(), record);
        }
        Ok(())
    }

    pub fn query(&self, query: &KnowledgeQuery) -> KnowledgeQueryResult {
        let mut resolved = Vec::new();
        for record in self.records.values() {
            if !matches_entity_filters(record, query) {
                continue;
            }
            let resolution = query
                .vehicle_context
                .as_ref()
                .map_or(ApplicabilityResolution::InsufficientContext, |context| {
                    record.applicability.resolve(context)
                });
            match resolution {
                ApplicabilityResolution::NotApplicable => continue,
                ApplicabilityResolution::InsufficientContext
                | ApplicabilityResolution::InsufficientEvidence
                    if !query.include_indeterminate =>
                {
                    continue
                }
                _ => {}
            }
            let evidence = record
                .evidence_ids
                .iter()
                .filter_map(|id| self.trace_evidence(id))
                .collect();
            resolved.push(ResolvedKnowledge {
                record: record.clone(),
                applicability_resolution: resolution,
                evidence,
            });
        }
        let conflicts = detect_conflicts(&resolved);
        KnowledgeQueryResult {
            records: resolved,
            conflicts,
        }
    }

    pub fn trace_back(&self, record_id: &str) -> Result<Vec<EvidenceTrace>, KnowledgeError> {
        let record = self
            .records
            .get(record_id)
            .ok_or_else(|| KnowledgeError::InvalidRecord(format!("unknown record: {record_id}")))?;
        record
            .evidence_ids
            .iter()
            .map(|id| {
                self.trace_evidence(id)
                    .ok_or_else(|| KnowledgeError::MissingEvidence(id.0.clone()))
            })
            .collect()
    }

    fn trace_evidence(&self, id: &EvidenceId) -> Option<EvidenceTrace> {
        let evidence = self.evidence.get(id)?.clone();
        let source = self.sources.get(&evidence.source_id)?.clone();
        Some(EvidenceTrace { evidence, source })
    }

    fn validate_record_evidence(&self, record: &KnowledgeRecord) -> Result<(), KnowledgeError> {
        validate_record_evidence_with(record, &self.sources, |id| self.evidence.get(id))
    }
}

/// The record-level evidence rules, over any evidence lookup, so that a batch
/// can be validated against the store plus itself before anything is written.
fn validate_record_evidence_with<'a>(
    record: &KnowledgeRecord,
    sources: &SourceRegistry,
    lookup: impl Fn(&EvidenceId) -> Option<&'a EvidenceRecord>,
) -> Result<(), KnowledgeError> {
    {
        let mut source_ids = BTreeSet::new();
        let mut source_types = BTreeSet::new();
        for id in &record.evidence_ids {
            let evidence =
                lookup(id).ok_or_else(|| KnowledgeError::MissingEvidence(id.0.clone()))?;
            let source = sources
                .get(&evidence.source_id)
                .ok_or_else(|| KnowledgeError::MissingSource(evidence.source_id.0.clone()))?;
            source_ids.insert(source.id.clone());
            source_types.insert(source.source_type);
        }

        let contains_synthetic = source_types.contains(&SourceType::Synthetic);
        if contains_synthetic && source_types.len() > 1 {
            return Err(KnowledgeError::ValidationViolation(
                "synthetic evidence cannot be mixed with real/research evidence".into(),
            ));
        }
        let all_real = source_types.iter().all(|kind| kind.is_real_evidence());
        let has_capture = source_types.contains(&SourceType::Captured);
        let valid = match record.validation_state {
            ValidationState::Unverified => true,
            ValidationState::SourceBacked => all_real,
            ValidationState::Corroborated => all_real && source_ids.len() >= 2,
            ValidationState::CaptureValidated => all_real && has_capture,
            ValidationState::Contradicted | ValidationState::Deprecated => all_real,
        };
        if !valid {
            return Err(KnowledgeError::ValidationViolation(format!(
                "{:?} is not supported by the referenced source classes",
                record.validation_state
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct KnowledgeQuery {
    pub vehicle_context: Option<VehicleContext>,
    pub ecu_family: Option<String>,
    pub diagnostic_implementation: Option<String>,
    pub include_indeterminate: bool,
}

impl KnowledgeQuery {
    pub fn for_vehicle(context: VehicleContext) -> Self {
        Self {
            vehicle_context: Some(context),
            ..Self::default()
        }
    }

    pub fn include_indeterminate(mut self, include: bool) -> Self {
        self.include_indeterminate = include;
        self
    }

    pub fn with_ecu_family(mut self, ecu_family: impl Into<String>) -> Self {
        self.ecu_family = Some(ecu_family.into());
        self
    }

    pub fn with_diagnostic_implementation(mut self, implementation: impl Into<String>) -> Self {
        self.diagnostic_implementation = Some(implementation.into());
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceTrace {
    pub evidence: EvidenceRecord,
    pub source: SourceRecord,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedKnowledge {
    pub record: KnowledgeRecord,
    pub applicability_resolution: ApplicabilityResolution,
    pub evidence: Vec<EvidenceTrace>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictReport {
    pub entity: crate::KnowledgeEntity,
    pub key: crate::ClaimKey,
    pub record_ids: Vec<String>,
    pub values: Vec<KnowledgeValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeQueryResult {
    pub records: Vec<ResolvedKnowledge>,
    pub conflicts: Vec<ConflictReport>,
}

fn matches_entity_filters(record: &KnowledgeRecord, query: &KnowledgeQuery) -> bool {
    if let Some(expected) = &query.ecu_family {
        let entity_match =
            record.entity.kind == EntityKind::EcuFamily && record.entity.id == *expected;
        if !entity_match && !record.applicability.ecu_family.mentions(expected) {
            return false;
        }
    }
    if let Some(expected) = &query.diagnostic_implementation {
        let entity_match = record.entity.kind == EntityKind::DiagnosticImplementation
            && record.entity.id == *expected;
        if !entity_match
            && !record
                .applicability
                .diagnostic_implementation
                .mentions(expected)
        {
            return false;
        }
    }
    true
}

fn detect_conflicts(records: &[ResolvedKnowledge]) -> Vec<ConflictReport> {
    let mut grouped: BTreeMap<_, Vec<&KnowledgeRecord>> = BTreeMap::new();
    for resolved in records {
        grouped
            .entry((resolved.record.entity.clone(), resolved.record.key.clone()))
            .or_default()
            .push(&resolved.record);
    }
    let mut conflicts = Vec::new();
    for ((entity, key), group) in grouped {
        let values: BTreeSet<_> = group.iter().map(|record| record.value.clone()).collect();
        if values.len() < 2 {
            continue;
        }
        conflicts.push(ConflictReport {
            entity,
            key,
            record_ids: group.iter().map(|record| record.id.clone()).collect(),
            values: values.into_iter().collect(),
        });
    }
    conflicts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Applicability, ClaimKey, ContentFingerprint, DimensionConstraint, EvidenceRecord,
        KnowledgeEntity, RedistributionStatus, SourceId, SourceLocator, YearConstraint,
    };

    fn documented_source(id: &str) -> SourceRecord {
        SourceRecord {
            id: SourceId::new(id).unwrap(),
            title: format!("Documented source {id}"),
            source_type: SourceType::Documented,
            origin: "Traceable documentation".into(),
            source_locator: format!("document:{id}"),
            content_fingerprint: Some(
                ContentFingerprint::sha256(crate::sha256_bytes(id.as_bytes())).unwrap(),
            ),
            acquired_on: None,
            declared_vehicle_programs: vec![],
            provenance: "Exact document identity and locator".into(),
            redistribution_status: crate::RedistributionStatus::RestrictedMetadataOnly,
            notes: None,
        }
    }

    fn evidence(id: &str, source_id: &str) -> EvidenceRecord {
        EvidenceRecord {
            id: EvidenceId::new(id).unwrap(),
            source_id: SourceId::new(source_id).unwrap(),
            evidence_class: None,
            locator: SourceLocator {
                description: "section 1".into(),
                document_page: Some(1),
                document_section: Some("1".into()),
                record_key: None,
                capture_timestamp_us: None,
            },
            excerpt: Some("minimal trace".into()),
            notes: None,
        }
    }

    fn applicability() -> Applicability {
        Applicability {
            vehicle_program: DimensionConstraint::one_of(vec!["PROGRAM-A".into()]).unwrap(),
            model_year: YearConstraint::Any,
            architecture_generation: DimensionConstraint::Any,
            ecu_family: DimensionConstraint::one_of(vec!["ECU-A".into()]).unwrap(),
            powertrain: DimensionConstraint::Any,
            variant: DimensionConstraint::Any,
            market: DimensionConstraint::Any,
            diagnostic_implementation: DimensionConstraint::one_of(vec!["IMPL-A".into()]).unwrap(),
            other: BTreeMap::new(),
        }
    }

    fn record(id: &str, evidence_id: &str, value: &str) -> KnowledgeRecord {
        KnowledgeRecord {
            id: id.into(),
            entity: KnowledgeEntity {
                kind: EntityKind::DiagnosticImplementation,
                id: "IMPL-A".into(),
            },
            key: ClaimKey::UsesProtocolFamily,
            value: KnowledgeValue::ProtocolFamily { name: value.into() },
            applicability: applicability(),
            evidence_ids: vec![EvidenceId::new(evidence_id).unwrap()],
            validation_state: ValidationState::SourceBacked,
        }
    }

    #[test]
    fn missing_source_and_missing_evidence_are_rejected_and_trace_back_works() {
        let mut store = KnowledgeStore::new();
        assert!(matches!(
            store.add_evidence(evidence("e-a", "s-a")),
            Err(KnowledgeError::MissingSource(_))
        ));
        store.register_source(documented_source("s-a")).unwrap();
        store.add_evidence(evidence("e-a", "s-a")).unwrap();
        assert!(matches!(
            store.add_record(record("r-missing", "missing", "UDS")),
            Err(KnowledgeError::MissingEvidence(_))
        ));
        store.add_record(record("r-a", "e-a", "UDS")).unwrap();
        let trace = store.trace_back("r-a").unwrap();
        assert_eq!(trace[0].source.id.0, "s-a");
        assert_eq!(trace[0].evidence.locator.document_page, Some(1));
    }

    #[test]
    fn contradictory_claims_coexist_and_query_reports_conflict() {
        let mut store = KnowledgeStore::new();
        for (source_id, evidence_id) in [("s-a", "e-a"), ("s-b", "e-b")] {
            store.register_source(documented_source(source_id)).unwrap();
            store
                .add_evidence(evidence(evidence_id, source_id))
                .unwrap();
        }
        store.add_record(record("r-a", "e-a", "UDS")).unwrap();
        store.add_record(record("r-b", "e-b", "KWP2000")).unwrap();

        let context = VehicleContext {
            vehicle_program: Some("PROGRAM-A".into()),
            model_year: Some(2012),
            architecture_generation: None,
            ecu_family: Some("ECU-A".into()),
            powertrain: None,
            variant: None,
            market: None,
            diagnostic_implementation: Some("IMPL-A".into()),
            other: BTreeMap::new(),
        };
        let result = store.query(&KnowledgeQuery::for_vehicle(context));
        assert_eq!(result.records.len(), 2);
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(
            result.conflicts[0].record_ids,
            vec!["r-a".to_string(), "r-b".to_string()]
        );
    }

    #[test]
    fn duplicate_source_evidence_does_not_corroborate_and_synthetic_cannot_promote() {
        let mut store = KnowledgeStore::new();
        store.register_source(documented_source("s-a")).unwrap();
        store.add_evidence(evidence("e-a", "s-a")).unwrap();
        store.add_evidence(evidence("e-b", "s-a")).unwrap();
        let mut claimed = record("r-a", "e-a", "UDS");
        claimed.evidence_ids.push(EvidenceId::new("e-b").unwrap());
        claimed.validation_state = ValidationState::Corroborated;
        assert!(matches!(
            store.add_record(claimed),
            Err(KnowledgeError::ValidationViolation(_))
        ));

        let mut synthetic = documented_source("s-synthetic");
        synthetic.source_type = SourceType::Synthetic;
        synthetic.content_fingerprint = None;
        synthetic.redistribution_status = RedistributionStatus::Permitted;
        store.register_source(synthetic).unwrap();
        let mut synthetic_evidence = evidence("e-synthetic", "s-synthetic");
        synthetic_evidence.evidence_class = Some(crate::EvidenceClass::SyntheticTest);
        store.add_evidence(synthetic_evidence).unwrap();
        let claimed = record("r-synthetic", "e-synthetic", "UDS");
        assert!(matches!(
            store.add_record(claimed),
            Err(KnowledgeError::ValidationViolation(_))
        ));
    }
}
