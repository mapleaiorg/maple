//! Mapleverse Connectors - Domain-specific execution adapters
//!
//! Connectors provide the actual implementation of effects for each domain.
//! They translate commitments into real-world actions.

#![deny(unsafe_code)]

use mapleverse_executor::{ExecutionHandler, ExecutorError};
use mapleverse_types::{
    Consequence, ConsequenceId, Evidence, EvidenceType, ExecutionParameters, ReversibilityStatus,
};
use rcf_commitment::{RcfCommitment, Reversibility};
use rcf_types::EffectDomain;
use std::collections::HashMap;
use thiserror::Error;

/// Connector registry
pub struct ConnectorRegistry {
    connectors: HashMap<EffectDomain, Box<dyn Connector + Send + Sync>>,
}

impl ConnectorRegistry {
    pub fn new() -> Self {
        Self {
            connectors: HashMap::new(),
        }
    }

    pub fn register<C: Connector + Send + Sync + 'static>(&mut self, connector: C) {
        self.connectors
            .insert(connector.supported_domain(), Box::new(connector));
    }

    pub fn get(&self, domain: &EffectDomain) -> Option<&(dyn Connector + Send + Sync)> {
        self.connectors.get(domain).map(|c| c.as_ref())
    }
}

impl Default for ConnectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for domain connectors
pub trait Connector {
    fn supported_domain(&self) -> EffectDomain;
    fn execute(&self, commitment: &RcfCommitment) -> Result<Consequence, ConnectorError>;
    fn rollback(&self, commitment: &RcfCommitment) -> Result<(), ConnectorError>;
    fn validate(&self, commitment: &RcfCommitment) -> Result<(), ConnectorError>;
}

/// Computation domain connector (no-op / sandboxed)
pub struct ComputationConnector;

impl Connector for ComputationConnector {
    fn supported_domain(&self) -> EffectDomain {
        EffectDomain::Computation
    }

    fn execute(&self, commitment: &RcfCommitment) -> Result<Consequence, ConnectorError> {
        // Simulated computation execution
        Ok(Consequence {
            consequence_id: ConsequenceId::generate(),
            commitment_id: commitment.commitment_id.clone(),
            effect_domain: EffectDomain::Computation,
            description: format!(
                "Computation executed: {}",
                commitment.intended_outcome.description
            ),
            evidence: vec![Evidence {
                evidence_type: EvidenceType::Log,
                description: "Computation log".to_string(),
                data: Vec::new(),
                timestamp: chrono::Utc::now(),
            }],
            occurred_at: chrono::Utc::now(),
            reversibility_status: ReversibilityStatus::Irreversible,
        })
    }

    fn rollback(&self, _commitment: &RcfCommitment) -> Result<(), ConnectorError> {
        // Computation cannot be rolled back
        Err(ConnectorError::RollbackNotSupported)
    }

    fn validate(&self, _commitment: &RcfCommitment) -> Result<(), ConnectorError> {
        Ok(())
    }
}

impl ExecutionHandler for ComputationConnector {
    fn execute(
        &self,
        commitment: &RcfCommitment,
        _params: &ExecutionParameters,
    ) -> Result<Consequence, ExecutorError> {
        Connector::execute(self, commitment).map_err(|e| ExecutorError::ExecutionFailed(e.to_string()))
    }

    fn rollback(&self, commitment: &RcfCommitment) -> Result<(), ExecutorError> {
        Connector::rollback(self, commitment).map_err(|e| ExecutorError::RollbackFailed(e.to_string()))
    }

    fn can_handle(&self, domain: &EffectDomain) -> bool {
        domain == &EffectDomain::Computation
    }
}

/// Data domain connector
pub struct DataConnector;

impl Connector for DataConnector {
    fn supported_domain(&self) -> EffectDomain {
        EffectDomain::Data
    }

    fn execute(&self, commitment: &RcfCommitment) -> Result<Consequence, ConnectorError> {
        Ok(Consequence {
            consequence_id: ConsequenceId::generate(),
            commitment_id: commitment.commitment_id.clone(),
            effect_domain: EffectDomain::Data,
            description: format!("Data operation: {}", commitment.intended_outcome.description),
            evidence: vec![Evidence {
                evidence_type: EvidenceType::StateSnapshot,
                description: "Data state snapshot".to_string(),
                data: Vec::new(),
                timestamp: chrono::Utc::now(),
            }],
            occurred_at: chrono::Utc::now(),
            reversibility_status: match &commitment.reversibility {
                Reversibility::Reversible => ReversibilityStatus::Reversible,
                Reversibility::PartiallyReversible(_) => ReversibilityStatus::PartiallyReversible,
                Reversibility::Irreversible => ReversibilityStatus::Irreversible,
            },
        })
    }

    fn rollback(&self, _commitment: &RcfCommitment) -> Result<(), ConnectorError> {
        // Data operations can potentially be rolled back
        Ok(())
    }

    fn validate(&self, _commitment: &RcfCommitment) -> Result<(), ConnectorError> {
        Ok(())
    }
}

impl ExecutionHandler for DataConnector {
    fn execute(
        &self,
        commitment: &RcfCommitment,
        _params: &ExecutionParameters,
    ) -> Result<Consequence, ExecutorError> {
        Connector::execute(self, commitment).map_err(|e| ExecutorError::ExecutionFailed(e.to_string()))
    }

    fn rollback(&self, commitment: &RcfCommitment) -> Result<(), ExecutorError> {
        Connector::rollback(self, commitment).map_err(|e| ExecutorError::RollbackFailed(e.to_string()))
    }

    fn can_handle(&self, domain: &EffectDomain) -> bool {
        domain == &EffectDomain::Data
    }
}

/// Communication domain connector
pub struct CommunicationConnector;

impl Connector for CommunicationConnector {
    fn supported_domain(&self) -> EffectDomain {
        EffectDomain::Communication
    }

    fn execute(&self, commitment: &RcfCommitment) -> Result<Consequence, ConnectorError> {
        Ok(Consequence {
            consequence_id: ConsequenceId::generate(),
            commitment_id: commitment.commitment_id.clone(),
            effect_domain: EffectDomain::Communication,
            description: format!("Communication sent: {}", commitment.intended_outcome.description),
            evidence: vec![Evidence {
                evidence_type: EvidenceType::ExternalReceipt,
                description: "Message delivery receipt".to_string(),
                data: Vec::new(),
                timestamp: chrono::Utc::now(),
            }],
            occurred_at: chrono::Utc::now(),
            reversibility_status: ReversibilityStatus::Irreversible,
        })
    }

    fn rollback(&self, _commitment: &RcfCommitment) -> Result<(), ConnectorError> {
        Err(ConnectorError::RollbackNotSupported)
    }

    fn validate(&self, _commitment: &RcfCommitment) -> Result<(), ConnectorError> {
        Ok(())
    }
}

impl ExecutionHandler for CommunicationConnector {
    fn execute(
        &self,
        commitment: &RcfCommitment,
        _params: &ExecutionParameters,
    ) -> Result<Consequence, ExecutorError> {
        Connector::execute(self, commitment).map_err(|e| ExecutorError::ExecutionFailed(e.to_string()))
    }

    fn rollback(&self, commitment: &RcfCommitment) -> Result<(), ExecutorError> {
        Connector::rollback(self, commitment).map_err(|e| ExecutorError::RollbackFailed(e.to_string()))
    }

    fn can_handle(&self, domain: &EffectDomain) -> bool {
        domain == &EffectDomain::Communication
    }
}

/// Connector errors
#[derive(Debug, Error)]
pub enum ConnectorError {
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Rollback not supported")]
    RollbackNotSupported,

    #[error("Rollback failed: {0}")]
    RollbackFailed(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Timeout")]
    Timeout,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcf_commitment::CommitmentBuilder;
    use rcf_types::{IdentityRef, ScopeConstraint};

    #[test]
    fn test_computation_connector() {
        let connector = ComputationConnector;
        let commitment = CommitmentBuilder::new(
            IdentityRef::new("test-agent"),
            EffectDomain::Computation,
        )
        .with_scope(ScopeConstraint::default())
        .build()
        .unwrap();

        let consequence = Connector::execute(&connector, &commitment).unwrap();
        assert_eq!(consequence.effect_domain, EffectDomain::Computation);
    }
}
