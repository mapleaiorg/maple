//! AAS Ledger - Immutable record of all commitments and outcomes
//!
//! The ledger provides full accountability through an immutable record
//! of all commitments, decisions, and consequences.

#![deny(unsafe_code)]

use aas_types::{
    AgentId, CommitmentLifecycle, CommitmentOutcome, LedgerEntry, LedgerEntryId, LifecycleStatus,
    PolicyDecisionCard,
};
use rcf_commitment::{CommitmentId, RcfCommitment};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;

/// The accountability ledger - immutable record of all commitments
pub struct AccountabilityLedger {
    entries: RwLock<HashMap<LedgerEntryId, LedgerEntry>>,
    commitment_index: RwLock<HashMap<CommitmentId, LedgerEntryId>>,
    agent_index: RwLock<HashMap<AgentId, Vec<LedgerEntryId>>>,
}

impl AccountabilityLedger {
    /// Create a new ledger
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            commitment_index: RwLock::new(HashMap::new()),
            agent_index: RwLock::new(HashMap::new()),
        }
    }

    /// Record a new commitment with its decision
    pub fn record_commitment(
        &self,
        commitment: RcfCommitment,
        decision: PolicyDecisionCard,
    ) -> Result<LedgerEntryId, LedgerError> {
        let entry_id = LedgerEntryId::generate();
        let now = chrono::Utc::now();

        let lifecycle = CommitmentLifecycle {
            status: match decision.decision {
                aas_types::Decision::Approved => LifecycleStatus::Approved,
                aas_types::Decision::Denied => LifecycleStatus::Denied,
                aas_types::Decision::PendingHumanReview => LifecycleStatus::Pending,
                aas_types::Decision::PendingAdditionalInfo => LifecycleStatus::Pending,
            },
            declared_at: now,
            adjudicated_at: Some(decision.decided_at),
            execution_started_at: None,
            execution_completed_at: None,
        };

        let agent_id = AgentId::new(&commitment.principal.id);

        let entry = LedgerEntry {
            entry_id: entry_id.clone(),
            commitment: commitment.clone(),
            decision,
            lifecycle,
            outcome: None,
            created_at: now,
            updated_at: now,
        };

        // Store entry
        let mut entries = self.entries.write().map_err(|_| LedgerError::LockError)?;
        entries.insert(entry_id.clone(), entry);

        // Update commitment index
        let mut commitment_index = self
            .commitment_index
            .write()
            .map_err(|_| LedgerError::LockError)?;
        commitment_index.insert(commitment.commitment_id, entry_id.clone());

        // Update agent index
        let mut agent_index = self
            .agent_index
            .write()
            .map_err(|_| LedgerError::LockError)?;
        agent_index
            .entry(agent_id)
            .or_default()
            .push(entry_id.clone());

        Ok(entry_id)
    }

    /// Record execution start
    pub fn record_execution_started(
        &self,
        commitment_id: &CommitmentId,
    ) -> Result<(), LedgerError> {
        let entry_id = self.get_entry_id(commitment_id)?;

        let mut entries = self.entries.write().map_err(|_| LedgerError::LockError)?;
        let entry = entries
            .get_mut(&entry_id)
            .ok_or_else(|| LedgerError::NotFound(entry_id.0.clone()))?;

        entry.lifecycle.status = LifecycleStatus::Executing;
        entry.lifecycle.execution_started_at = Some(chrono::Utc::now());
        entry.updated_at = chrono::Utc::now();

        Ok(())
    }

    /// Record outcome (consequence)
    pub fn record_outcome(
        &self,
        commitment_id: &CommitmentId,
        outcome: CommitmentOutcome,
    ) -> Result<(), LedgerError> {
        let entry_id = self.get_entry_id(commitment_id)?;

        let mut entries = self.entries.write().map_err(|_| LedgerError::LockError)?;
        let entry = entries
            .get_mut(&entry_id)
            .ok_or_else(|| LedgerError::NotFound(entry_id.0.clone()))?;

        entry.lifecycle.status = if outcome.success {
            LifecycleStatus::Completed
        } else {
            LifecycleStatus::Failed
        };
        entry.lifecycle.execution_completed_at = Some(outcome.completed_at);
        entry.outcome = Some(outcome);
        entry.updated_at = chrono::Utc::now();

        Ok(())
    }

    /// Get an entry by commitment ID
    pub fn get_by_commitment(
        &self,
        commitment_id: &CommitmentId,
    ) -> Result<Option<LedgerEntry>, LedgerError> {
        let commitment_index = self
            .commitment_index
            .read()
            .map_err(|_| LedgerError::LockError)?;

        if let Some(entry_id) = commitment_index.get(commitment_id) {
            let entries = self.entries.read().map_err(|_| LedgerError::LockError)?;
            Ok(entries.get(entry_id).cloned())
        } else {
            Ok(None)
        }
    }

    /// Get all entries for an agent
    pub fn get_by_agent(&self, agent_id: &AgentId) -> Result<Vec<LedgerEntry>, LedgerError> {
        let agent_index = self
            .agent_index
            .read()
            .map_err(|_| LedgerError::LockError)?;
        let entries = self.entries.read().map_err(|_| LedgerError::LockError)?;

        let entry_ids = match agent_index.get(agent_id) {
            Some(ids) => ids,
            None => return Ok(vec![]),
        };

        let results: Vec<_> = entry_ids
            .iter()
            .filter_map(|id| entries.get(id).cloned())
            .collect();

        Ok(results)
    }

    /// Query entries with filters
    pub fn query(&self, query: LedgerQuery) -> Result<Vec<LedgerEntry>, LedgerError> {
        let entries = self.entries.read().map_err(|_| LedgerError::LockError)?;

        let mut results: Vec<_> = entries
            .values()
            .filter(|entry| {
                // Filter by status
                if let Some(ref status) = query.status {
                    if entry.lifecycle.status != *status {
                        return false;
                    }
                }

                // Filter by time range
                if let Some(after) = query.after {
                    if entry.created_at < after {
                        return false;
                    }
                }
                if let Some(before) = query.before {
                    if entry.created_at > before {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Sort by creation time (newest first)
        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Apply limit
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// Get statistics about the ledger
    pub fn statistics(&self) -> Result<LedgerStatistics, LedgerError> {
        let entries = self.entries.read().map_err(|_| LedgerError::LockError)?;

        let total_commitments = entries.len();
        let mut by_status: HashMap<String, usize> = HashMap::new();
        let mut successful_executions = 0;
        let mut failed_executions = 0;

        for entry in entries.values() {
            let status_str = format!("{:?}", entry.lifecycle.status);
            *by_status.entry(status_str).or_insert(0) += 1;

            if let Some(ref outcome) = entry.outcome {
                if outcome.success {
                    successful_executions += 1;
                } else {
                    failed_executions += 1;
                }
            }
        }

        Ok(LedgerStatistics {
            total_commitments,
            by_status,
            successful_executions,
            failed_executions,
        })
    }

    /// Get entry ID for a commitment
    fn get_entry_id(&self, commitment_id: &CommitmentId) -> Result<LedgerEntryId, LedgerError> {
        let commitment_index = self
            .commitment_index
            .read()
            .map_err(|_| LedgerError::LockError)?;
        commitment_index
            .get(commitment_id)
            .cloned()
            .ok_or_else(|| LedgerError::NotFound(commitment_id.0.clone()))
    }
}

impl Default for AccountabilityLedger {
    fn default() -> Self {
        Self::new()
    }
}

/// Query parameters for ledger search
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LedgerQuery {
    pub status: Option<LifecycleStatus>,
    pub after: Option<chrono::DateTime<chrono::Utc>>,
    pub before: Option<chrono::DateTime<chrono::Utc>>,
    pub limit: Option<usize>,
}

/// Statistics about the ledger
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LedgerStatistics {
    pub total_commitments: usize,
    pub by_status: HashMap<String, usize>,
    pub successful_executions: usize,
    pub failed_executions: usize,
}

/// Ledger-related errors
#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("Entry not found: {0}")]
    NotFound(String),

    #[error("Immutability violation: {0}")]
    ImmutabilityViolation(String),

    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),

    #[error("Lock error")]
    LockError,
}

#[cfg(test)]
mod tests {
    use super::*;
    use aas_types::{
        AdjudicatorInfo, AdjudicatorType, Decision, DecisionId, Rationale, RiskAssessment,
        RiskLevel,
    };
    use rcf_commitment::CommitmentBuilder;
    use rcf_types::{EffectDomain, IdentityRef, ScopeConstraint};

    #[test]
    fn test_record_and_retrieve() {
        let ledger = AccountabilityLedger::new();

        let commitment =
            CommitmentBuilder::new(IdentityRef::new("test-agent"), EffectDomain::Computation)
                .with_scope(ScopeConstraint::default())
                .build()
                .unwrap();

        let commitment_id = commitment.commitment_id.clone();

        let decision = PolicyDecisionCard {
            decision_id: DecisionId::generate(),
            commitment_id: commitment_id.clone(),
            decision: Decision::Approved,
            rationale: Rationale {
                summary: "Test".to_string(),
                rule_references: vec![],
            },
            risk_assessment: RiskAssessment {
                overall_risk: RiskLevel::Low,
                risk_factors: vec![],
                mitigations: vec![],
            },
            conditions: vec![],
            approval_expiration: None,
            decided_at: chrono::Utc::now(),
            adjudicator: AdjudicatorInfo {
                adjudicator_type: AdjudicatorType::Automated,
                adjudicator_id: "test".to_string(),
            },
        };

        ledger.record_commitment(commitment, decision).unwrap();

        let entry = ledger.get_by_commitment(&commitment_id).unwrap();
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().lifecycle.status, LifecycleStatus::Approved);
    }
}
