//! Mapleverse Evidence - Evidence collection and management
//!
//! Evidence provides accountability for consequences.

#![deny(unsafe_code)]

use mapleverse_types::{Consequence, Evidence, EvidenceType};
use rcf_commitment::CommitmentId;
use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;

/// Evidence store
pub struct EvidenceStore {
    evidence: RwLock<HashMap<CommitmentId, Vec<Evidence>>>,
}

impl EvidenceStore {
    pub fn new() -> Self {
        Self {
            evidence: RwLock::new(HashMap::new()),
        }
    }

    /// Store evidence for a commitment
    pub fn store(&self, commitment_id: CommitmentId, evidence: Evidence) -> Result<(), EvidenceError> {
        let mut store = self.evidence.write().map_err(|_| EvidenceError::LockError)?;
        store.entry(commitment_id).or_default().push(evidence);
        Ok(())
    }

    /// Store all evidence from a consequence
    pub fn store_consequence(&self, consequence: &Consequence) -> Result<(), EvidenceError> {
        let mut store = self.evidence.write().map_err(|_| EvidenceError::LockError)?;
        let entries = store
            .entry(consequence.commitment_id.clone())
            .or_default();
        entries.extend(consequence.evidence.iter().cloned());
        Ok(())
    }

    /// Get all evidence for a commitment
    pub fn get(&self, commitment_id: &CommitmentId) -> Result<Vec<Evidence>, EvidenceError> {
        let store = self.evidence.read().map_err(|_| EvidenceError::LockError)?;
        Ok(store.get(commitment_id).cloned().unwrap_or_default())
    }

    /// Query evidence by type
    pub fn query_by_type(
        &self,
        commitment_id: &CommitmentId,
        evidence_type: &EvidenceType,
    ) -> Result<Vec<Evidence>, EvidenceError> {
        let store = self.evidence.read().map_err(|_| EvidenceError::LockError)?;
        Ok(store
            .get(commitment_id)
            .map(|evidences| {
                evidences
                    .iter()
                    .filter(|e| &e.evidence_type == evidence_type)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default())
    }
}

impl Default for EvidenceStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Evidence errors
#[derive(Debug, Error)]
pub enum EvidenceError {
    #[error("Lock error")]
    LockError,

    #[error("Evidence not found")]
    NotFound,

    #[error("Invalid evidence: {0}")]
    Invalid(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_store() {
        let store = EvidenceStore::new();
        let commitment_id = CommitmentId::generate();

        let evidence = Evidence {
            evidence_type: EvidenceType::Log,
            description: "Test log".to_string(),
            data: vec![1, 2, 3],
            timestamp: chrono::Utc::now(),
        };

        store.store(commitment_id.clone(), evidence).unwrap();

        let retrieved = store.get(&commitment_id).unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].description, "Test log");
    }
}
