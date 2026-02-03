//! EVE Ingestion - Consequence ingestion for learning
//!
//! Ingests consequences from Mapleverse for analysis.

#![deny(unsafe_code)]

use eve_types::{AnalysisStatus, CommitmentCharacteristics, ConsequenceRecord};
use mapleverse_types::Consequence;
use rcf_commitment::CommitmentId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;

/// Consequence ingestion service
pub struct IngestionService {
    records: RwLock<HashMap<String, ConsequenceRecord>>,
    commitment_index: RwLock<HashMap<CommitmentId, String>>,
}

impl IngestionService {
    pub fn new() -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
            commitment_index: RwLock::new(HashMap::new()),
        }
    }

    /// Ingest a consequence for learning
    pub fn ingest(
        &self,
        consequence: Consequence,
        characteristics: CommitmentCharacteristics,
    ) -> Result<String, IngestionError> {
        let record_id = uuid::Uuid::new_v4().to_string();

        let record = ConsequenceRecord {
            record_id: record_id.clone(),
            consequence: consequence.clone(),
            commitment_characteristics: characteristics,
            recorded_at: chrono::Utc::now(),
            analysis_status: AnalysisStatus::Pending,
        };

        let mut records = self
            .records
            .write()
            .map_err(|_| IngestionError::LockError)?;
        records.insert(record_id.clone(), record);

        let mut index = self
            .commitment_index
            .write()
            .map_err(|_| IngestionError::LockError)?;
        index.insert(consequence.commitment_id, record_id.clone());

        Ok(record_id)
    }

    /// Get pending records for analysis
    pub fn get_pending(&self, limit: usize) -> Result<Vec<ConsequenceRecord>, IngestionError> {
        let records = self.records.read().map_err(|_| IngestionError::LockError)?;

        let pending: Vec<_> = records
            .values()
            .filter(|r| r.analysis_status == AnalysisStatus::Pending)
            .take(limit)
            .cloned()
            .collect();

        Ok(pending)
    }

    /// Mark a record as analyzed
    pub fn mark_analyzed(&self, record_id: &str) -> Result<(), IngestionError> {
        let mut records = self
            .records
            .write()
            .map_err(|_| IngestionError::LockError)?;

        if let Some(record) = records.get_mut(record_id) {
            record.analysis_status = AnalysisStatus::Analyzed;
            Ok(())
        } else {
            Err(IngestionError::NotFound(record_id.to_string()))
        }
    }

    /// Get a record by commitment ID
    pub fn get_by_commitment(
        &self,
        commitment_id: &CommitmentId,
    ) -> Result<Option<ConsequenceRecord>, IngestionError> {
        let index = self
            .commitment_index
            .read()
            .map_err(|_| IngestionError::LockError)?;

        if let Some(record_id) = index.get(commitment_id) {
            let records = self.records.read().map_err(|_| IngestionError::LockError)?;
            Ok(records.get(record_id).cloned())
        } else {
            Ok(None)
        }
    }

    /// Get statistics
    pub fn statistics(&self) -> Result<IngestionStats, IngestionError> {
        let records = self.records.read().map_err(|_| IngestionError::LockError)?;

        let total = records.len();
        let pending = records
            .values()
            .filter(|r| r.analysis_status == AnalysisStatus::Pending)
            .count();
        let analyzed = records
            .values()
            .filter(|r| r.analysis_status == AnalysisStatus::Analyzed)
            .count();

        Ok(IngestionStats {
            total,
            pending,
            analyzed,
        })
    }
}

impl Default for IngestionService {
    fn default() -> Self {
        Self::new()
    }
}

/// Ingestion statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IngestionStats {
    pub total: usize,
    pub pending: usize,
    pub analyzed: usize,
}

/// Ingestion errors
#[derive(Debug, Error)]
pub enum IngestionError {
    #[error("Record not found: {0}")]
    NotFound(String),

    #[error("Invalid consequence: {0}")]
    InvalidConsequence(String),

    #[error("Lock error")]
    LockError,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mapleverse_types::{ConsequenceId, ReversibilityStatus};
    use rcf_types::EffectDomain;

    #[test]
    fn test_ingestion() {
        let service = IngestionService::new();

        let consequence = Consequence {
            consequence_id: ConsequenceId::generate(),
            commitment_id: CommitmentId::generate(),
            effect_domain: EffectDomain::Computation,
            description: "Test consequence".to_string(),
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

        let record_id = service.ingest(consequence, characteristics).unwrap();

        let pending = service.get_pending(10).unwrap();
        assert_eq!(pending.len(), 1);

        service.mark_analyzed(&record_id).unwrap();

        let pending = service.get_pending(10).unwrap();
        assert_eq!(pending.len(), 0);
    }
}
