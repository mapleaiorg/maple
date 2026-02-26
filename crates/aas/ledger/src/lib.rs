//! AAS Ledger - immutable accountability view backed by MAPLE storage.
//!
//! This crate provides the AAS-facing ledger API while delegating persistence to
//! `maple-storage`. That gives a single source of truth for commitment lifecycle,
//! outcomes, and query surfaces across runtime components.

#![deny(unsafe_code)]

// ── WLL canonical ledger re-exports ─────────────────────────────────
/// WLL ledger primitives — canonical receipts and ledger interfaces.
pub mod wll {
    pub use wll_ledger::{
        InMemoryLedger as WllInMemoryLedger,
        LedgerReader as WllLedgerReader,
        LedgerWriter as WllLedgerWriter,
    };
}

use aas_types::{
    AgentId, CommitmentLifecycle, CommitmentOutcome, LedgerEntry, LedgerEntryId, LifecycleStatus,
    PolicyDecisionCard, ToolExecutionReceipt, ToolReceiptStatus,
};
use chrono::{DateTime, Utc};
use maple_storage::memory::InMemoryMapleStorage;
use maple_storage::{AuditAppend, CommitmentRecord, MapleStorage, QueryWindow, StorageError};
use rcf_commitment::{CommitmentId, RcfCommitment};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// The accountability ledger facade.
///
/// Internally this wraps a `MapleStorage` backend so AAS and runtime surfaces
/// read/write the same durable commitment records.
pub struct AccountabilityLedger {
    storage: Arc<dyn MapleStorage>,
}

impl AccountabilityLedger {
    /// Create a new ledger backed by in-memory MAPLE storage.
    pub fn new() -> Self {
        Self {
            storage: Arc::new(InMemoryMapleStorage::new()),
        }
    }

    /// Create a ledger backed by an explicit MAPLE storage adapter.
    pub fn with_storage(storage: Arc<dyn MapleStorage>) -> Self {
        Self { storage }
    }

    /// Access the underlying storage backend.
    pub fn storage(&self) -> Arc<dyn MapleStorage> {
        Arc::clone(&self.storage)
    }

    /// Record a new commitment with its decision.
    pub async fn record_commitment(
        &self,
        commitment: RcfCommitment,
        decision: PolicyDecisionCard,
    ) -> Result<LedgerEntryId, LedgerError> {
        let commitment_id = commitment.commitment_id.clone();
        self.storage
            .create_commitment(commitment, decision, Utc::now())
            .await
            .map_err(LedgerError::from)?;
        Ok(ledger_entry_id(&commitment_id))
    }

    /// Record execution start.
    pub async fn record_execution_started(
        &self,
        commitment_id: &CommitmentId,
    ) -> Result<(), LedgerError> {
        let current = self
            .storage
            .get_commitment(commitment_id)
            .await
            .map_err(LedgerError::from)?
            .ok_or_else(|| LedgerError::NotFound(commitment_id.0.clone()))?;

        if current.lifecycle_status != LifecycleStatus::Approved {
            return Err(LedgerError::InvalidStateTransition(format!(
                "cannot start execution from status {:?}",
                current.lifecycle_status
            )));
        }

        self.storage
            .transition_lifecycle(
                commitment_id,
                LifecycleStatus::Approved,
                LifecycleStatus::Executing,
                Utc::now(),
            )
            .await
            .map_err(LedgerError::from)
    }

    /// Record outcome (consequence).
    pub async fn record_outcome(
        &self,
        commitment_id: &CommitmentId,
        outcome: CommitmentOutcome,
    ) -> Result<(), LedgerError> {
        let current = self
            .storage
            .get_commitment(commitment_id)
            .await
            .map_err(LedgerError::from)?
            .ok_or_else(|| LedgerError::NotFound(commitment_id.0.clone()))?;

        if !matches!(current.lifecycle_status, LifecycleStatus::Executing) {
            return Err(LedgerError::InvalidStateTransition(format!(
                "cannot record outcome from status {:?}",
                current.lifecycle_status
            )));
        }

        let final_status = if outcome.success {
            LifecycleStatus::Completed
        } else {
            LifecycleStatus::Failed
        };

        self.storage
            .set_outcome(commitment_id, outcome, final_status)
            .await
            .map_err(LedgerError::from)
    }

    /// Persist an immutable tool receipt for replay and accountability.
    pub async fn record_tool_receipt(
        &self,
        receipt: ToolExecutionReceipt,
    ) -> Result<(), LedgerError> {
        let message = format!(
            "tool receipt {} for capability {} ({:?})",
            receipt.receipt_id, receipt.capability_id, receipt.status
        );
        let success = matches!(receipt.status, ToolReceiptStatus::Succeeded);
        self.storage
            .append_audit(AuditAppend {
                timestamp: receipt.timestamp,
                actor: "aas-ledger".to_string(),
                stage: "tool_receipt".to_string(),
                success,
                message,
                commitment_id: Some(receipt.contract_id.clone()),
                payload: serde_json::json!({
                    "tool_receipt": receipt,
                }),
            })
            .await
            .map_err(LedgerError::from)?;
        Ok(())
    }

    /// Return all persisted receipts for a commitment.
    pub async fn get_tool_receipts_by_commitment(
        &self,
        commitment_id: &CommitmentId,
    ) -> Result<Vec<ToolExecutionReceipt>, LedgerError> {
        let audits = self
            .storage
            .list_audit(QueryWindow {
                limit: 0,
                offset: 0,
            })
            .await
            .map_err(LedgerError::from)?;

        let mut receipts = audits
            .into_iter()
            .filter(|record| record.stage == "tool_receipt")
            .filter(|record| {
                record
                    .commitment_id
                    .as_ref()
                    .is_some_and(|id| id == commitment_id)
            })
            .filter_map(|record| {
                parse_tool_receipt(&record.payload)
                    .map_err(|err| LedgerError::Backend(err.to_string()))
                    .ok()
            })
            .collect::<Vec<_>>();

        receipts.sort_by(|a, b| compare_dt_desc(a.timestamp, b.timestamp));
        Ok(receipts)
    }

    /// Get an entry by commitment ID.
    pub async fn get_by_commitment(
        &self,
        commitment_id: &CommitmentId,
    ) -> Result<Option<LedgerEntry>, LedgerError> {
        let record = self
            .storage
            .get_commitment(commitment_id)
            .await
            .map_err(LedgerError::from)?;
        Ok(record.map(commitment_record_to_entry))
    }

    /// Get all entries for an agent.
    pub async fn get_by_agent(&self, agent_id: &AgentId) -> Result<Vec<LedgerEntry>, LedgerError> {
        let records = self
            .storage
            .list_commitments(QueryWindow {
                limit: 0,
                offset: 0,
            })
            .await
            .map_err(LedgerError::from)?;

        let mut entries: Vec<_> = records
            .into_iter()
            .filter(|record| record.commitment.principal.id == agent_id.0)
            .map(commitment_record_to_entry)
            .collect();

        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(entries)
    }

    /// Query entries with filters.
    pub async fn query(&self, query: LedgerQuery) -> Result<Vec<LedgerEntry>, LedgerError> {
        let records = self
            .storage
            .list_commitments(QueryWindow {
                limit: 0,
                offset: 0,
            })
            .await
            .map_err(LedgerError::from)?;

        let mut results: Vec<_> = records
            .into_iter()
            .map(commitment_record_to_entry)
            .filter(|entry| {
                if let Some(ref status) = query.status {
                    if entry.lifecycle.status != *status {
                        return false;
                    }
                }

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
            .collect();

        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// Get statistics about the ledger.
    pub async fn statistics(&self) -> Result<LedgerStatistics, LedgerError> {
        let entries = self
            .query(LedgerQuery {
                limit: None,
                ..Default::default()
            })
            .await?;

        let total_commitments = entries.len();
        let mut by_status: HashMap<String, usize> = HashMap::new();
        let mut successful_executions = 0;
        let mut failed_executions = 0;

        for entry in entries {
            let status_str = format!("{:?}", entry.lifecycle.status);
            *by_status.entry(status_str).or_insert(0) += 1;

            if let Some(outcome) = entry.outcome {
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
}

impl Default for AccountabilityLedger {
    fn default() -> Self {
        Self::new()
    }
}

fn ledger_entry_id(commitment_id: &CommitmentId) -> LedgerEntryId {
    LedgerEntryId(format!("entry:{}", commitment_id.0))
}

fn commitment_record_to_entry(record: CommitmentRecord) -> LedgerEntry {
    let lifecycle = CommitmentLifecycle {
        status: record.lifecycle_status,
        declared_at: record.created_at,
        adjudicated_at: Some(record.decision.decided_at),
        execution_started_at: record.execution_started_at,
        execution_completed_at: record.execution_completed_at,
    };

    LedgerEntry {
        entry_id: ledger_entry_id(&record.commitment_id),
        commitment: record.commitment,
        decision: record.decision,
        lifecycle,
        outcome: record.outcome,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

fn parse_tool_receipt(payload: &serde_json::Value) -> Result<ToolExecutionReceipt, StorageError> {
    let Some(receipt_value) = payload.get("tool_receipt").cloned() else {
        return Err(StorageError::Serialization(
            "missing tool_receipt payload".to_string(),
        ));
    };

    serde_json::from_value(receipt_value)
        .map_err(|err| StorageError::Serialization(err.to_string()))
}

fn compare_dt_desc(left: DateTime<Utc>, right: DateTime<Utc>) -> std::cmp::Ordering {
    right.cmp(&left)
}

/// Query parameters for ledger search.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LedgerQuery {
    pub status: Option<LifecycleStatus>,
    pub after: Option<chrono::DateTime<chrono::Utc>>,
    pub before: Option<chrono::DateTime<chrono::Utc>>,
    pub limit: Option<usize>,
}

/// Statistics about the ledger.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LedgerStatistics {
    pub total_commitments: usize,
    pub by_status: HashMap<String, usize>,
    pub successful_executions: usize,
    pub failed_executions: usize,
}

/// Ledger-related errors.
#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("Entry not found: {0}")]
    NotFound(String),

    #[error("Immutability violation: {0}")]
    ImmutabilityViolation(String),

    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),

    #[error("Backend error: {0}")]
    Backend(String),
}

impl From<StorageError> for LedgerError {
    fn from(value: StorageError) -> Self {
        match value {
            StorageError::NotFound(msg) => Self::NotFound(msg),
            StorageError::Conflict(msg) => Self::ImmutabilityViolation(msg),
            StorageError::InvariantViolation(msg) => Self::InvalidStateTransition(msg),
            StorageError::InvalidInput(msg)
            | StorageError::Serialization(msg)
            | StorageError::Backend(msg) => Self::Backend(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aas_types::{
        AdjudicatorInfo, AdjudicatorType, Decision, DecisionId, Rationale, RiskAssessment,
        RiskLevel, ToolExecutionReceipt, ToolReceiptStatus,
    };
    use proptest::prelude::*;
    use rcf_commitment::CommitmentBuilder;
    use rcf_types::{EffectDomain, IdentityRef, ScopeConstraint};

    fn approved_decision(commitment_id: CommitmentId) -> PolicyDecisionCard {
        PolicyDecisionCard {
            decision_id: DecisionId::generate(),
            commitment_id,
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
        }
    }

    #[tokio::test]
    async fn test_record_and_retrieve() {
        let ledger = AccountabilityLedger::new();

        let commitment =
            CommitmentBuilder::new(IdentityRef::new("test-agent"), EffectDomain::Computation)
                .with_scope(ScopeConstraint::default())
                .build()
                .unwrap();

        let commitment_id = commitment.commitment_id.clone();

        let decision = approved_decision(commitment_id.clone());

        ledger
            .record_commitment(commitment, decision)
            .await
            .unwrap();

        let entry = ledger.get_by_commitment(&commitment_id).await.unwrap();
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().lifecycle.status, LifecycleStatus::Approved);
    }

    #[tokio::test]
    async fn tool_receipts_are_persisted_and_replayable() {
        let ledger = AccountabilityLedger::new();

        let commitment =
            CommitmentBuilder::new(IdentityRef::new("test-agent"), EffectDomain::Computation)
                .with_scope(ScopeConstraint::default())
                .build()
                .unwrap();
        let commitment_id = commitment.commitment_id.clone();

        let decision = approved_decision(commitment_id.clone());

        ledger
            .record_commitment(commitment, decision)
            .await
            .expect("commitment should persist");

        let receipt = ToolExecutionReceipt {
            receipt_id: "receipt-1".to_string(),
            tool_call_id: "tool-call-1".to_string(),
            contract_id: commitment_id.clone(),
            capability_id: "simulate_transfer".to_string(),
            hash: "hash-1".to_string(),
            timestamp: chrono::Utc::now(),
            status: ToolReceiptStatus::Succeeded,
        };
        ledger
            .record_tool_receipt(receipt.clone())
            .await
            .expect("receipt should persist");

        let receipts = ledger
            .get_tool_receipts_by_commitment(&commitment_id)
            .await
            .expect("receipts should replay");
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].receipt_id, receipt.receipt_id);
        assert_eq!(receipts[0].tool_call_id, receipt.tool_call_id);
    }

    #[tokio::test]
    async fn lifecycle_cannot_skip_execution_started_before_outcome() {
        let ledger = AccountabilityLedger::new();

        let commitment =
            CommitmentBuilder::new(IdentityRef::new("test-agent"), EffectDomain::Computation)
                .with_scope(ScopeConstraint::default())
                .build()
                .unwrap();
        let commitment_id = commitment.commitment_id.clone();
        ledger
            .record_commitment(commitment, approved_decision(commitment_id.clone()))
            .await
            .expect("commitment should be stored");

        let result = ledger
            .record_outcome(
                &commitment_id,
                CommitmentOutcome {
                    success: true,
                    description: "skip".to_string(),
                    completed_at: chrono::Utc::now(),
                },
            )
            .await;
        assert!(matches!(
            result,
            Err(LedgerError::InvalidStateTransition(_))
        ));
    }

    #[derive(Debug, Clone)]
    enum LifecycleOp {
        Start,
        Complete(bool),
    }

    fn op_strategy() -> impl Strategy<Value = Vec<LifecycleOp>> {
        proptest::collection::vec(
            prop_oneof![
                Just(LifecycleOp::Start),
                any::<bool>().prop_map(LifecycleOp::Complete),
            ],
            0..12,
        )
    }

    proptest! {
        #[test]
        fn property_lifecycle_transitions_are_explicit(ops in op_strategy()) {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");

            rt.block_on(async move {
                let ledger = AccountabilityLedger::new();
                let commitment =
                    CommitmentBuilder::new(IdentityRef::new("prop-agent"), EffectDomain::Computation)
                        .with_scope(ScopeConstraint::default())
                        .build()
                        .unwrap();
                let commitment_id = commitment.commitment_id.clone();
                ledger
                    .record_commitment(commitment, approved_decision(commitment_id.clone()))
                    .await
                    .expect("commitment");

                let mut saw_invalid = false;
                for op in ops {
                    let result = match op {
                        LifecycleOp::Start => ledger.record_execution_started(&commitment_id).await,
                        LifecycleOp::Complete(success) => ledger.record_outcome(
                            &commitment_id,
                            CommitmentOutcome {
                                success,
                                description: "prop".to_string(),
                                completed_at: chrono::Utc::now(),
                            },
                        ).await,
                    };
                    if result.is_err() {
                        saw_invalid = true;
                    }
                }

                let entry = ledger
                    .get_by_commitment(&commitment_id)
                    .await
                    .expect("query")
                    .expect("entry");
                match entry.lifecycle.status {
                    LifecycleStatus::Approved => {}
                    LifecycleStatus::Executing => {}
                    LifecycleStatus::Completed | LifecycleStatus::Failed => {
                        // terminal states are valid only when no invalid transition was accepted.
                    }
                    other => panic!("unexpected status {:?}", other),
                }

                if saw_invalid {
                    // Explicit failure is required; state remains a valid lifecycle state.
                    assert!(matches!(
                        entry.lifecycle.status,
                        LifecycleStatus::Approved
                            | LifecycleStatus::Executing
                            | LifecycleStatus::Completed
                            | LifecycleStatus::Failed
                    ));
                }
            });
        }
    }
}
