//! Resonator commitment engine surfaces.
//!
//! This crate exposes the minimal contract-engine abstraction used by
//! `maple-runtime::AgentKernel` composition. It stores explicit executable
//! contracts (RCF commitments) and their activation state.

#![deny(unsafe_code)]

use rcf_commitment::{CommitmentId, RcfCommitment};
use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;

/// Contract activation status tracked by the resonator-side engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContractStatus {
    Active,
    Inactive,
    Revoked,
}

/// Stored contract record.
#[derive(Clone, Debug)]
pub struct StoredContract {
    pub contract: RcfCommitment,
    pub status: ContractStatus,
}

/// Contract engine abstraction used by AgentState.
pub trait ContractEngine: Send + Sync {
    fn register_contract(&self, contract: RcfCommitment) -> Result<(), ContractEngineError>;

    fn get_contract(
        &self,
        contract_id: &CommitmentId,
    ) -> Result<Option<StoredContract>, ContractEngineError>;

    fn set_status(
        &self,
        contract_id: &CommitmentId,
        status: ContractStatus,
    ) -> Result<(), ContractEngineError>;

    fn is_active(&self, contract_id: &CommitmentId) -> Result<bool, ContractEngineError> {
        let record = self.get_contract(contract_id)?;
        Ok(matches!(
            record.map(|r| r.status),
            Some(ContractStatus::Active)
        ))
    }
}

/// Deterministic in-memory engine for tests/dev.
#[derive(Default)]
pub struct InMemoryContractEngine {
    contracts: RwLock<HashMap<CommitmentId, StoredContract>>,
}

impl InMemoryContractEngine {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ContractEngine for InMemoryContractEngine {
    fn register_contract(&self, contract: RcfCommitment) -> Result<(), ContractEngineError> {
        let mut guard = self
            .contracts
            .write()
            .map_err(|_| ContractEngineError::LockError)?;

        if guard.contains_key(&contract.commitment_id) {
            return Err(ContractEngineError::AlreadyExists(
                contract.commitment_id.0.clone(),
            ));
        }

        guard.insert(
            contract.commitment_id.clone(),
            StoredContract {
                contract,
                status: ContractStatus::Active,
            },
        );
        Ok(())
    }

    fn get_contract(
        &self,
        contract_id: &CommitmentId,
    ) -> Result<Option<StoredContract>, ContractEngineError> {
        let guard = self
            .contracts
            .read()
            .map_err(|_| ContractEngineError::LockError)?;
        Ok(guard.get(contract_id).cloned())
    }

    fn set_status(
        &self,
        contract_id: &CommitmentId,
        status: ContractStatus,
    ) -> Result<(), ContractEngineError> {
        let mut guard = self
            .contracts
            .write()
            .map_err(|_| ContractEngineError::LockError)?;
        let entry = guard
            .get_mut(contract_id)
            .ok_or_else(|| ContractEngineError::NotFound(contract_id.0.clone()))?;
        entry.status = status;
        Ok(())
    }
}

/// Contract-engine errors.
#[derive(Debug, Error)]
pub enum ContractEngineError {
    #[error("Contract not found: {0}")]
    NotFound(String),

    #[error("Contract already exists: {0}")]
    AlreadyExists(String),

    #[error("Contract engine lock error")]
    LockError,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcf_commitment::CommitmentBuilder;
    use rcf_types::{EffectDomain, IdentityRef, ScopeConstraint};

    #[test]
    fn register_and_activate_contract() {
        let engine = InMemoryContractEngine::new();
        let contract =
            CommitmentBuilder::new(IdentityRef::new("agent-a"), EffectDomain::Computation)
                .with_scope(ScopeConstraint::default())
                .build()
                .unwrap();

        engine.register_contract(contract.clone()).unwrap();
        assert!(engine.is_active(&contract.commitment_id).unwrap());
    }
}
