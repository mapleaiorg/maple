//! Mapleverse Service - The unified execution service
//!
//! The Mapleverse service coordinates execution of approved commitments.
//! It has NO decision authority - it only executes what AAS has approved.

#![deny(unsafe_code)]

use mapleverse_connectors::{CommunicationConnector, ComputationConnector, DataConnector};
use mapleverse_evidence::{EvidenceError, EvidenceStore};
use mapleverse_executor::{
    Evidence, ExecutionParameters, ExecutionRequest, ExecutionRequestId, ExecutionResult, Executor,
    ExecutorError,
};
use rcf_commitment::{CommitmentId, RcfCommitment};
use rcf_types::EffectDomain;
use std::sync::Arc;
use thiserror::Error;

/// The Mapleverse service
pub struct MapleverseService {
    executor: Arc<Executor>,
    evidence_store: Arc<EvidenceStore>,
}

impl MapleverseService {
    /// Create a new Mapleverse service with default connectors
    pub fn new() -> Self {
        let executor = Executor::new();

        // Register default connectors
        executor
            .register_handler(EffectDomain::Computation, ComputationConnector)
            .unwrap();
        executor
            .register_handler(EffectDomain::Data, DataConnector)
            .unwrap();
        executor
            .register_handler(EffectDomain::Communication, CommunicationConnector)
            .unwrap();

        Self {
            executor: Arc::new(executor),
            evidence_store: Arc::new(EvidenceStore::new()),
        }
    }

    /// Execute an approved commitment
    pub fn execute(
        &self,
        commitment: RcfCommitment,
        decision_id: String,
        params: ExecutionParameters,
    ) -> Result<ExecutionResult, MapleverseError> {
        let request = ExecutionRequest {
            request_id: ExecutionRequestId::generate(),
            commitment,
            decision_id,
            requested_at: chrono::Utc::now(),
            execution_parameters: params,
        };

        let result = self.executor.execute(request)?;

        // Store evidence from consequence
        if let Some(ref consequence) = result.consequence {
            self.evidence_store.store_consequence(consequence)?;
        }

        Ok(result)
    }

    /// Get execution status
    pub fn get_status(
        &self,
        request_id: &ExecutionRequestId,
    ) -> Result<Option<ExecutionResult>, MapleverseError> {
        self.executor.get_status(request_id).map_err(Into::into)
    }

    /// Abort an execution
    pub fn abort(
        &self,
        request_id: &ExecutionRequestId,
        reason: &str,
    ) -> Result<(), MapleverseError> {
        self.executor.abort(request_id, reason).map_err(Into::into)
    }

    /// Get evidence for a commitment
    pub fn get_evidence(
        &self,
        commitment_id: &CommitmentId,
    ) -> Result<Vec<Evidence>, MapleverseError> {
        self.evidence_store.get(commitment_id).map_err(Into::into)
    }

    /// Get the executor
    pub fn executor(&self) -> &Executor {
        &self.executor
    }

    /// Get the evidence store
    pub fn evidence_store(&self) -> &EvidenceStore {
        &self.evidence_store
    }
}

impl Default for MapleverseService {
    fn default() -> Self {
        Self::new()
    }
}

/// Mapleverse service errors
#[derive(Debug, Error)]
pub enum MapleverseError {
    #[error("Executor error: {0}")]
    Executor(#[from] ExecutorError),

    #[error("Evidence error: {0}")]
    Evidence(#[from] EvidenceError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcf_commitment::CommitmentBuilder;
    use rcf_types::{IdentityRef, ScopeConstraint};

    #[test]
    fn test_mapleverse_execution() {
        let service = MapleverseService::new();

        let commitment =
            CommitmentBuilder::new(IdentityRef::new("test-agent"), EffectDomain::Computation)
                .with_scope(ScopeConstraint::default())
                .build()
                .unwrap();

        let result = service
            .execute(
                commitment.clone(),
                "test-decision".to_string(),
                ExecutionParameters::default(),
            )
            .unwrap();

        assert!(result.status.is_success());

        // Check evidence was stored
        let evidence = service.get_evidence(&commitment.commitment_id).unwrap();
        assert!(!evidence.is_empty());
    }
}
