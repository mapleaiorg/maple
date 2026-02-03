//! EVE Service - The unified Epistemic Validation Engine
//!
//! EVE learns from consequences but has NO authority.
//! It produces insights that must be reviewed by humans before being acted upon.

#![deny(unsafe_code)]

use eve_artifacts::{ArtifactStore, ArtifactStoreError};
use eve_evaluation::{EvaluationEngine, EvaluationError};
use eve_ingestion::{IngestionError, IngestionService};
use eve_types::{
    ArtifactQuery, CommitmentCharacteristics, EveInsight, LearningArtifact,
};
use mapleverse_types::Consequence;
use std::sync::Arc;
use thiserror::Error;

/// The EVE service
pub struct EveService {
    ingestion: Arc<IngestionService>,
    evaluation: Arc<EvaluationEngine>,
    artifacts: Arc<ArtifactStore>,
}

impl EveService {
    pub fn new() -> Self {
        Self {
            ingestion: Arc::new(IngestionService::new()),
            evaluation: Arc::new(EvaluationEngine::new()),
            artifacts: Arc::new(ArtifactStore::new()),
        }
    }

    /// Ingest a consequence for learning
    pub fn ingest(
        &self,
        consequence: Consequence,
        characteristics: CommitmentCharacteristics,
    ) -> Result<String, EveError> {
        self.ingestion
            .ingest(consequence, characteristics)
            .map_err(EveError::Ingestion)
    }

    /// Process pending consequences and generate artifacts
    pub fn process_pending(&self, batch_size: usize) -> Result<Vec<LearningArtifact>, EveError> {
        // Get pending records
        let pending = self.ingestion.get_pending(batch_size)?;

        if pending.is_empty() {
            return Ok(vec![]);
        }

        // Analyze the batch
        let artifacts = self.evaluation.analyze_batch(&pending)?;

        // Store artifacts
        for artifact in &artifacts {
            self.artifacts.store(artifact.clone())?;
        }

        // Mark records as analyzed
        for record in &pending {
            self.ingestion.mark_analyzed(&record.record_id)?;
        }

        Ok(artifacts)
    }

    /// Generate insights from current artifacts
    pub fn generate_insights(&self) -> Result<Vec<EveInsight>, EveError> {
        self.evaluation
            .generate_insights()
            .map_err(EveError::Evaluation)
    }

    /// Query artifacts
    pub fn query_artifacts(&self, query: ArtifactQuery) -> Result<Vec<LearningArtifact>, EveError> {
        self.artifacts.query(query).map_err(EveError::Artifacts)
    }

    /// Get all artifacts
    pub fn get_all_artifacts(&self) -> Result<Vec<LearningArtifact>, EveError> {
        self.evaluation
            .get_artifacts()
            .map_err(EveError::Evaluation)
    }

    /// Get ingestion service
    pub fn ingestion(&self) -> &IngestionService {
        &self.ingestion
    }

    /// Get evaluation engine
    pub fn evaluation(&self) -> &EvaluationEngine {
        &self.evaluation
    }

    /// Get artifact store
    pub fn artifacts(&self) -> &ArtifactStore {
        &self.artifacts
    }
}

impl Default for EveService {
    fn default() -> Self {
        Self::new()
    }
}

/// EVE service errors
#[derive(Debug, Error)]
pub enum EveError {
    #[error("Ingestion error: {0}")]
    Ingestion(#[from] IngestionError),

    #[error("Evaluation error: {0}")]
    Evaluation(#[from] EvaluationError),

    #[error("Artifact store error: {0}")]
    Artifacts(#[from] ArtifactStoreError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use mapleverse_types::{ConsequenceId, ReversibilityStatus};
    use rcf_commitment::CommitmentId;
    use rcf_types::EffectDomain;

    #[test]
    fn test_eve_service() {
        let eve = EveService::new();

        // Ingest some consequences
        for _ in 0..10 {
            let consequence = Consequence {
                consequence_id: ConsequenceId::generate(),
                commitment_id: CommitmentId::generate(),
                effect_domain: EffectDomain::Computation,
                description: "Test".to_string(),
                evidence: vec![],
                occurred_at: chrono::Utc::now(),
                reversibility_status: ReversibilityStatus::Irreversible,
            };

            let characteristics = CommitmentCharacteristics {
                domain: EffectDomain::Computation,
                risk_level: "low".to_string(),
                scope_size: "small".to_string(),
                reversibility: "irreversible".to_string(),
                agent_history_length: 10,
            };

            eve.ingest(consequence, characteristics).unwrap();
        }

        // Process pending
        let artifacts = eve.process_pending(100).unwrap();

        // Should have detected pattern
        assert!(!artifacts.is_empty());
    }
}
