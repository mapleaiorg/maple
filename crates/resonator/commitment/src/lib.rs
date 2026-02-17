//! Contract Lifecycle Manager for MAPLE Resonators
//!
//! This module implements commitment/contract lifecycle management for the
//! Resonance Architecture. Commitments are explicit promises with audit trails
//! that must exist before consequences can occur (Invariant #4: Commitment
//! precedes Consequence).
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                  CONTRACT LIFECYCLE MANAGER                     │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │   Draft ──> Proposed ──> Accepted ──> Active ──> Executing     │
//! │              │            │           │            │           │
//! │              v            v           v            v           │
//! │          Rejected     Expired    Suspended      Failed         │
//! │                                      │                         │
//! │                                      v                         │
//! │                                  Disputed ──> Resolved         │
//! │                                                   │            │
//! │                                                   v            │
//! │                                              Completed         │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Components
//!
//! - [`ContractLifecycleManager`]: Main manager orchestrating lifecycle
//! - [`ContractStateMachine`]: State machine for contract transitions
//! - [`DisputeHandler`]: Handles disputes on contracts
//! - [`ExpiryTracker`]: Monitors contract expiration
//!
//! # Invariant Enforcement
//!
//! - Invariant #3: Intent must be stabilized before commitment
//! - Invariant #4: Commitment must exist before consequence

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Duration, Utc};
use rcf_commitment::{CommitmentId, RcfCommitment};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Re-export core types for backward compatibility
pub use rcf_commitment::CommitmentId as RcfCommitmentId;

/// Contract lifecycle state with full state machine support.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractStatus {
    /// Initial draft state.
    Draft,
    /// Proposed and awaiting acceptance.
    Proposed,
    /// Accepted but not yet active.
    Accepted,
    /// Active and ready for execution.
    Active,
    /// Currently executing.
    Executing,
    /// Successfully completed.
    Completed,
    /// Failed during execution.
    Failed { reason: String },
    /// Rejected during proposal.
    Rejected { reason: String },
    /// Expired due to temporal bounds.
    Expired,
    /// Suspended (can be resumed).
    Suspended { reason: String },
    /// Under dispute.
    Disputed { dispute_id: String },
    /// Dispute resolved.
    Resolved { resolution: String },
    /// Revoked (cannot be resumed).
    Revoked { reason: String },

    // Legacy variants for backward compatibility
    /// Legacy: equivalent to Active
    Inactive,
}

impl ContractStatus {
    /// Check if contract is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ContractStatus::Completed
                | ContractStatus::Failed { .. }
                | ContractStatus::Rejected { .. }
                | ContractStatus::Expired
                | ContractStatus::Revoked { .. }
        )
    }

    /// Check if contract can be executed.
    pub fn is_executable(&self) -> bool {
        matches!(self, ContractStatus::Active)
    }

    /// Check if contract is currently executing.
    pub fn is_executing(&self) -> bool {
        matches!(self, ContractStatus::Executing)
    }
}

/// Stored contract record with lifecycle metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredContract {
    /// The underlying RCF commitment.
    pub contract: RcfCommitment,
    /// Current lifecycle status.
    pub status: ContractStatus,
    /// When the contract was created.
    pub created_at: DateTime<Utc>,
    /// When the status was last changed.
    pub status_changed_at: DateTime<Utc>,
    /// History of status changes.
    pub status_history: Vec<StatusChange>,
    /// Digital signature if signed.
    pub signature: Option<ContractSignature>,
    /// Associated dispute if any.
    pub dispute: Option<Dispute>,
}

impl StoredContract {
    /// Create a new stored contract in Draft state.
    pub fn new(contract: RcfCommitment) -> Self {
        let now = Utc::now();
        Self {
            contract,
            status: ContractStatus::Draft,
            created_at: now,
            status_changed_at: now,
            status_history: vec![StatusChange {
                from: None,
                to: ContractStatus::Draft,
                timestamp: now,
                reason: "Contract created".to_string(),
                actor: None,
            }],
            signature: None,
            dispute: None,
        }
    }

    /// Create a new stored contract in Active state (for backward compatibility).
    pub fn new_active(contract: RcfCommitment) -> Self {
        let now = Utc::now();
        Self {
            contract,
            status: ContractStatus::Active,
            created_at: now,
            status_changed_at: now,
            status_history: vec![StatusChange {
                from: None,
                to: ContractStatus::Active,
                timestamp: now,
                reason: "Contract created active".to_string(),
                actor: None,
            }],
            signature: None,
            dispute: None,
        }
    }
}

/// Record of a status change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatusChange {
    /// Previous status (None if initial).
    pub from: Option<ContractStatus>,
    /// New status.
    pub to: ContractStatus,
    /// When the change occurred.
    pub timestamp: DateTime<Utc>,
    /// Reason for the change.
    pub reason: String,
    /// Actor who caused the change.
    pub actor: Option<String>,
}

/// Digital signature for a contract.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContractSignature {
    /// Signer identity.
    pub signer: String,
    /// Signature bytes (hex encoded).
    pub signature: String,
    /// Algorithm used.
    pub algorithm: String,
    /// When signed.
    pub signed_at: DateTime<Utc>,
}

/// Dispute record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dispute {
    /// Unique dispute identifier.
    pub dispute_id: String,
    /// Who raised the dispute.
    pub raised_by: String,
    /// Reason for dispute.
    pub reason: String,
    /// Evidence supporting the dispute.
    pub evidence: Vec<DisputeEvidence>,
    /// When the dispute was raised.
    pub raised_at: DateTime<Utc>,
    /// Resolution if resolved.
    pub resolution: Option<DisputeResolution>,
}

/// Evidence for a dispute.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisputeEvidence {
    /// Evidence type.
    pub evidence_type: String,
    /// Content or reference.
    pub content: String,
    /// When submitted.
    pub submitted_at: DateTime<Utc>,
}

/// Resolution of a dispute.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisputeResolution {
    /// Resolution outcome.
    pub outcome: DisputeOutcome,
    /// Explanation of resolution.
    pub explanation: String,
    /// Who resolved it.
    pub resolved_by: String,
    /// When resolved.
    pub resolved_at: DateTime<Utc>,
}

/// Outcome of a dispute.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisputeOutcome {
    /// Dispute upheld, contract terminated.
    Upheld,
    /// Dispute rejected, contract continues.
    Rejected,
    /// Compromise reached.
    Compromise,
    /// Referred to higher authority.
    Referred,
}

/// Contract state machine for valid transitions.
#[derive(Debug, Clone)]
pub struct ContractStateMachine;

impl ContractStateMachine {
    /// Check if a transition is valid.
    pub fn is_valid_transition(from: &ContractStatus, to: &ContractStatus) -> bool {
        use ContractStatus::*;

        match (from, to) {
            // Draft transitions
            (Draft, Proposed) => true,
            (Draft, Rejected { .. }) => true,

            // Proposed transitions
            (Proposed, Accepted) => true,
            (Proposed, Rejected { .. }) => true,
            (Proposed, Expired) => true,

            // Accepted transitions
            (Accepted, Active) => true,
            (Accepted, Expired) => true,
            (Accepted, Revoked { .. }) => true,

            // Active transitions
            (Active, Executing) => true,
            (Active, Suspended { .. }) => true,
            (Active, Expired) => true,
            (Active, Revoked { .. }) => true,
            (Active, Disputed { .. }) => true,

            // Executing transitions
            (Executing, Completed) => true,
            (Executing, Failed { .. }) => true,
            (Executing, Suspended { .. }) => true,
            (Executing, Disputed { .. }) => true,

            // Suspended transitions
            (Suspended { .. }, Active) => true,
            (Suspended { .. }, Disputed { .. }) => true,
            (Suspended { .. }, Revoked { .. }) => true,
            (Suspended { .. }, Expired) => true,

            // Disputed transitions
            (Disputed { .. }, Resolved { .. }) => true,
            (Disputed { .. }, Revoked { .. }) => true,

            // Resolved can go back to Active or to terminal
            (Resolved { .. }, Active) => true,
            (Resolved { .. }, Completed) => true,
            (Resolved { .. }, Revoked { .. }) => true,

            // Terminal states cannot transition
            (Completed, _) => false,
            (Failed { .. }, _) => false,
            (Rejected { .. }, _) => false,
            (Expired, _) => false,
            (Revoked { .. }, _) => false,

            // Legacy: Inactive treated as Active
            (Inactive, to) => Self::is_valid_transition(&Active, to),
            (from, Inactive) => Self::is_valid_transition(from, &Active),

            _ => false,
        }
    }

    /// Get valid transitions from a state.
    pub fn valid_transitions(from: &ContractStatus) -> Vec<ContractStatus> {
        use ContractStatus::*;

        match from {
            Draft => vec![
                Proposed,
                Rejected {
                    reason: String::new(),
                },
            ],
            Proposed => vec![
                Accepted,
                Rejected {
                    reason: String::new(),
                },
                Expired,
            ],
            Accepted => vec![
                Active,
                Expired,
                Revoked {
                    reason: String::new(),
                },
            ],
            Active => vec![
                Executing,
                Suspended {
                    reason: String::new(),
                },
                Expired,
                Revoked {
                    reason: String::new(),
                },
                Disputed {
                    dispute_id: String::new(),
                },
            ],
            Executing => vec![
                Completed,
                Failed {
                    reason: String::new(),
                },
                Suspended {
                    reason: String::new(),
                },
                Disputed {
                    dispute_id: String::new(),
                },
            ],
            Suspended { .. } => vec![
                Active,
                Disputed {
                    dispute_id: String::new(),
                },
                Revoked {
                    reason: String::new(),
                },
                Expired,
            ],
            Disputed { .. } => vec![
                Resolved {
                    resolution: String::new(),
                },
                Revoked {
                    reason: String::new(),
                },
            ],
            Resolved { .. } => vec![
                Active,
                Completed,
                Revoked {
                    reason: String::new(),
                },
            ],
            // Terminal states
            Completed | Failed { .. } | Rejected { .. } | Expired | Revoked { .. } => vec![],
            Inactive => Self::valid_transitions(&Active),
        }
    }
}

/// Handles disputes on contracts.
#[derive(Debug, Clone, Default)]
pub struct DisputeHandler {
    /// Active disputes by ID.
    disputes: HashMap<String, Dispute>,
}

impl DisputeHandler {
    /// Create a new dispute handler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Raise a dispute on a contract.
    pub fn raise_dispute(
        &mut self,
        contract_id: &CommitmentId,
        raised_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Dispute {
        let dispute_id = format!("dispute-{}-{}", contract_id.0, uuid::Uuid::new_v4());
        let dispute = Dispute {
            dispute_id: dispute_id.clone(),
            raised_by: raised_by.into(),
            reason: reason.into(),
            evidence: Vec::new(),
            raised_at: Utc::now(),
            resolution: None,
        };
        self.disputes.insert(dispute_id, dispute.clone());
        dispute
    }

    /// Add evidence to a dispute.
    pub fn add_evidence(
        &mut self,
        dispute_id: &str,
        evidence_type: impl Into<String>,
        content: impl Into<String>,
    ) -> Result<(), ContractEngineError> {
        let dispute = self
            .disputes
            .get_mut(dispute_id)
            .ok_or_else(|| ContractEngineError::DisputeNotFound(dispute_id.to_string()))?;

        dispute.evidence.push(DisputeEvidence {
            evidence_type: evidence_type.into(),
            content: content.into(),
            submitted_at: Utc::now(),
        });

        Ok(())
    }

    /// Resolve a dispute.
    pub fn resolve_dispute(
        &mut self,
        dispute_id: &str,
        outcome: DisputeOutcome,
        explanation: impl Into<String>,
        resolved_by: impl Into<String>,
    ) -> Result<DisputeResolution, ContractEngineError> {
        let dispute = self
            .disputes
            .get_mut(dispute_id)
            .ok_or_else(|| ContractEngineError::DisputeNotFound(dispute_id.to_string()))?;

        let resolution = DisputeResolution {
            outcome,
            explanation: explanation.into(),
            resolved_by: resolved_by.into(),
            resolved_at: Utc::now(),
        };

        dispute.resolution = Some(resolution.clone());

        Ok(resolution)
    }

    /// Get a dispute by ID.
    pub fn get_dispute(&self, dispute_id: &str) -> Option<&Dispute> {
        self.disputes.get(dispute_id)
    }
}

/// Tracks contract expiration.
#[derive(Debug, Clone)]
pub struct ExpiryTracker {
    /// Warning threshold before expiry (milliseconds).
    warning_threshold_ms: i64,
}

impl ExpiryTracker {
    /// Create a new expiry tracker.
    pub fn new(warning_threshold_ms: i64) -> Self {
        Self {
            warning_threshold_ms,
        }
    }

    /// Check if a contract is expired.
    pub fn is_expired(&self, contract: &RcfCommitment) -> bool {
        !contract.is_valid_at(Utc::now())
    }

    /// Check if a contract is about to expire.
    pub fn is_expiring_soon(&self, contract: &RcfCommitment) -> bool {
        let future = Utc::now() + Duration::milliseconds(self.warning_threshold_ms);
        contract.is_valid_at(Utc::now()) && !contract.is_valid_at(future)
    }

    /// Get time until expiry (if valid).
    pub fn time_until_expiry(&self, contract: &RcfCommitment) -> Option<Duration> {
        let end = contract.temporal_validity.valid_until?;
        let now = Utc::now();
        if end > now {
            Some(end - now)
        } else {
            None
        }
    }
}

impl Default for ExpiryTracker {
    fn default() -> Self {
        Self::new(300_000) // 5 minutes
    }
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

    /// Transition contract status with validation.
    fn transition(
        &self,
        contract_id: &CommitmentId,
        new_status: ContractStatus,
        reason: &str,
        actor: Option<String>,
    ) -> Result<(), ContractEngineError>;

    /// List all contracts.
    fn list_contracts(&self) -> Result<Vec<StoredContract>, ContractEngineError>;

    /// Get contracts by status.
    fn get_by_status(
        &self,
        status: &ContractStatus,
    ) -> Result<Vec<StoredContract>, ContractEngineError>;
}

/// The main contract lifecycle manager.
///
/// Wraps a ContractEngine and adds lifecycle management features.
#[derive(Clone)]
pub struct ContractLifecycleManager {
    /// Underlying engine.
    engine: Arc<dyn ContractEngine>,
    /// Dispute handler.
    dispute_handler: Arc<RwLock<DisputeHandler>>,
    /// Expiry tracker.
    expiry_tracker: ExpiryTracker,
}

impl ContractLifecycleManager {
    /// Create a new lifecycle manager.
    pub fn new(engine: Arc<dyn ContractEngine>) -> Self {
        Self {
            engine,
            dispute_handler: Arc::new(RwLock::new(DisputeHandler::new())),
            expiry_tracker: ExpiryTracker::default(),
        }
    }

    /// Create a new contract in Draft state.
    pub fn create_draft(
        &self,
        contract: RcfCommitment,
    ) -> Result<CommitmentId, ContractEngineError> {
        let id = contract.commitment_id.clone();
        self.engine.register_contract(contract)?;
        Ok(id)
    }

    /// Propose a contract (Draft -> Proposed).
    pub fn propose(
        &self,
        contract_id: &CommitmentId,
        actor: Option<String>,
    ) -> Result<(), ContractEngineError> {
        self.engine.transition(
            contract_id,
            ContractStatus::Proposed,
            "Contract proposed",
            actor,
        )
    }

    /// Accept a contract (Proposed -> Accepted).
    pub fn accept(
        &self,
        contract_id: &CommitmentId,
        actor: Option<String>,
    ) -> Result<(), ContractEngineError> {
        self.engine.transition(
            contract_id,
            ContractStatus::Accepted,
            "Contract accepted",
            actor,
        )
    }

    /// Activate a contract (Accepted -> Active).
    pub fn activate(
        &self,
        contract_id: &CommitmentId,
        actor: Option<String>,
    ) -> Result<(), ContractEngineError> {
        self.engine.transition(
            contract_id,
            ContractStatus::Active,
            "Contract activated",
            actor,
        )
    }

    /// Start execution (Active -> Executing).
    pub fn start_execution(
        &self,
        contract_id: &CommitmentId,
        actor: Option<String>,
    ) -> Result<(), ContractEngineError> {
        self.engine.transition(
            contract_id,
            ContractStatus::Executing,
            "Execution started",
            actor,
        )
    }

    /// Complete execution (Executing -> Completed).
    pub fn complete(
        &self,
        contract_id: &CommitmentId,
        actor: Option<String>,
    ) -> Result<(), ContractEngineError> {
        self.engine.transition(
            contract_id,
            ContractStatus::Completed,
            "Execution completed successfully",
            actor,
        )
    }

    /// Mark execution as failed (Executing -> Failed).
    pub fn fail(
        &self,
        contract_id: &CommitmentId,
        reason: impl Into<String>,
        actor: Option<String>,
    ) -> Result<(), ContractEngineError> {
        let reason = reason.into();
        self.engine.transition(
            contract_id,
            ContractStatus::Failed {
                reason: reason.clone(),
            },
            &reason,
            actor,
        )
    }

    /// Suspend a contract.
    pub fn suspend(
        &self,
        contract_id: &CommitmentId,
        reason: impl Into<String>,
        actor: Option<String>,
    ) -> Result<(), ContractEngineError> {
        let reason = reason.into();
        self.engine.transition(
            contract_id,
            ContractStatus::Suspended {
                reason: reason.clone(),
            },
            &reason,
            actor,
        )
    }

    /// Resume a suspended contract.
    pub fn resume(
        &self,
        contract_id: &CommitmentId,
        actor: Option<String>,
    ) -> Result<(), ContractEngineError> {
        self.engine.transition(
            contract_id,
            ContractStatus::Active,
            "Contract resumed",
            actor,
        )
    }

    /// Raise a dispute on a contract.
    pub fn raise_dispute(
        &self,
        contract_id: &CommitmentId,
        raised_by: impl Into<String>,
        reason: impl Into<String>,
        actor: Option<String>,
    ) -> Result<Dispute, ContractEngineError> {
        let raised_by = raised_by.into();
        let reason = reason.into();

        let dispute = self
            .dispute_handler
            .write()
            .map_err(|_| ContractEngineError::LockError)?
            .raise_dispute(contract_id, &raised_by, &reason);

        self.engine.transition(
            contract_id,
            ContractStatus::Disputed {
                dispute_id: dispute.dispute_id.clone(),
            },
            &format!("Dispute raised by {}: {}", raised_by, reason),
            actor,
        )?;

        Ok(dispute)
    }

    /// Resolve a dispute.
    pub fn resolve_dispute(
        &self,
        contract_id: &CommitmentId,
        dispute_id: &str,
        outcome: DisputeOutcome,
        explanation: impl Into<String>,
        resolved_by: impl Into<String>,
        actor: Option<String>,
    ) -> Result<DisputeResolution, ContractEngineError> {
        let explanation = explanation.into();
        let resolved_by_str = resolved_by.into();

        let resolution = self
            .dispute_handler
            .write()
            .map_err(|_| ContractEngineError::LockError)?
            .resolve_dispute(dispute_id, outcome.clone(), &explanation, &resolved_by_str)?;

        self.engine.transition(
            contract_id,
            ContractStatus::Resolved {
                resolution: explanation.clone(),
            },
            &format!("Dispute resolved: {:?}", outcome),
            actor,
        )?;

        Ok(resolution)
    }

    /// Check and expire contracts that have exceeded their temporal bounds.
    pub fn check_expirations(&self) -> Result<Vec<CommitmentId>, ContractEngineError> {
        let contracts = self.engine.list_contracts()?;
        let mut expired = Vec::new();

        for stored in contracts {
            if !stored.status.is_terminal() && self.expiry_tracker.is_expired(&stored.contract) {
                self.engine.transition(
                    &stored.contract.commitment_id,
                    ContractStatus::Expired,
                    "Contract expired",
                    None,
                )?;
                expired.push(stored.contract.commitment_id);
            }
        }

        Ok(expired)
    }

    /// Get contracts that are about to expire.
    pub fn get_expiring_soon(&self) -> Result<Vec<StoredContract>, ContractEngineError> {
        let contracts = self.engine.list_contracts()?;
        Ok(contracts
            .into_iter()
            .filter(|c| {
                !c.status.is_terminal() && self.expiry_tracker.is_expiring_soon(&c.contract)
            })
            .collect())
    }

    /// Get the underlying engine.
    pub fn engine(&self) -> &Arc<dyn ContractEngine> {
        &self.engine
    }

    /// Get a contract.
    pub fn get_contract(
        &self,
        contract_id: &CommitmentId,
    ) -> Result<Option<StoredContract>, ContractEngineError> {
        self.engine.get_contract(contract_id)
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
            StoredContract::new_active(contract),
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
        entry.status_changed_at = Utc::now();
        Ok(())
    }

    fn transition(
        &self,
        contract_id: &CommitmentId,
        new_status: ContractStatus,
        reason: &str,
        actor: Option<String>,
    ) -> Result<(), ContractEngineError> {
        let mut guard = self
            .contracts
            .write()
            .map_err(|_| ContractEngineError::LockError)?;

        let entry = guard
            .get_mut(contract_id)
            .ok_or_else(|| ContractEngineError::NotFound(contract_id.0.clone()))?;

        // Validate transition
        if !ContractStateMachine::is_valid_transition(&entry.status, &new_status) {
            return Err(ContractEngineError::InvalidTransition {
                from: format!("{:?}", entry.status),
                to: format!("{:?}", new_status),
            });
        }

        let now = Utc::now();
        let change = StatusChange {
            from: Some(entry.status.clone()),
            to: new_status.clone(),
            timestamp: now,
            reason: reason.to_string(),
            actor,
        };

        entry.status_history.push(change);
        entry.status = new_status;
        entry.status_changed_at = now;

        Ok(())
    }

    fn list_contracts(&self) -> Result<Vec<StoredContract>, ContractEngineError> {
        let guard = self
            .contracts
            .read()
            .map_err(|_| ContractEngineError::LockError)?;
        Ok(guard.values().cloned().collect())
    }

    fn get_by_status(
        &self,
        status: &ContractStatus,
    ) -> Result<Vec<StoredContract>, ContractEngineError> {
        let guard = self
            .contracts
            .read()
            .map_err(|_| ContractEngineError::LockError)?;
        Ok(guard
            .values()
            .filter(|c| &c.status == status)
            .cloned()
            .collect())
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

    #[error("Invalid state transition from {from} to {to}")]
    InvalidTransition { from: String, to: String },

    #[error("Contract is in terminal state: {0}")]
    TerminalState(String),

    #[error("Dispute not found: {0}")]
    DisputeNotFound(String),

    #[error("Contract expired")]
    Expired,

    #[error("Contract not active")]
    NotActive,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcf_commitment::CommitmentBuilder;
    use rcf_types::{EffectDomain, IdentityRef, ScopeConstraint};

    fn make_contract() -> RcfCommitment {
        CommitmentBuilder::new(IdentityRef::new("agent-a"), EffectDomain::Computation)
            .with_scope(ScopeConstraint::default())
            .build()
            .unwrap()
    }

    #[test]
    fn register_and_activate_contract() {
        let engine = InMemoryContractEngine::new();
        let contract = make_contract();

        engine.register_contract(contract.clone()).unwrap();
        assert!(engine.is_active(&contract.commitment_id).unwrap());
    }

    #[test]
    fn test_full_lifecycle_happy_path() {
        let engine = Arc::new(InMemoryContractEngine::new());

        let contract = make_contract();
        let id = contract.commitment_id.clone();

        // Register as draft (using engine directly since manager creates in Draft)
        engine.register_contract(contract).unwrap();

        // Manual transitions for test
        engine
            .transition(&id, ContractStatus::Executing, "Start execution", None)
            .unwrap();
        engine
            .transition(&id, ContractStatus::Completed, "Done", None)
            .unwrap();

        let stored = engine.get_contract(&id).unwrap().unwrap();
        assert!(matches!(stored.status, ContractStatus::Completed));
        assert!(stored.status.is_terminal());
    }

    #[test]
    fn test_invalid_transition_rejected() {
        let engine = Arc::new(InMemoryContractEngine::new());
        let contract = make_contract();
        let id = contract.commitment_id.clone();

        engine.register_contract(contract).unwrap();

        // Try invalid transition: Active -> Completed (should go through Executing)
        let result = engine.transition(&id, ContractStatus::Completed, "Invalid", None);
        assert!(matches!(
            result,
            Err(ContractEngineError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn test_dispute_lifecycle() {
        let engine = Arc::new(InMemoryContractEngine::new());
        let manager = ContractLifecycleManager::new(engine);

        let contract = make_contract();
        let id = contract.commitment_id.clone();

        manager.engine.register_contract(contract).unwrap();

        // Raise dispute
        let dispute = manager
            .raise_dispute(&id, "user-1", "Terms violated", None)
            .unwrap();

        let stored = manager.get_contract(&id).unwrap().unwrap();
        assert!(matches!(stored.status, ContractStatus::Disputed { .. }));

        // Resolve dispute
        let resolution = manager
            .resolve_dispute(
                &id,
                &dispute.dispute_id,
                DisputeOutcome::Rejected,
                "No violation found",
                "adjudicator",
                None,
            )
            .unwrap();

        assert_eq!(resolution.outcome, DisputeOutcome::Rejected);

        let stored = manager.get_contract(&id).unwrap().unwrap();
        assert!(matches!(stored.status, ContractStatus::Resolved { .. }));
    }

    #[test]
    fn test_state_machine_valid_transitions() {
        // Check some valid transitions
        assert!(ContractStateMachine::is_valid_transition(
            &ContractStatus::Draft,
            &ContractStatus::Proposed
        ));
        assert!(ContractStateMachine::is_valid_transition(
            &ContractStatus::Active,
            &ContractStatus::Executing
        ));
        assert!(ContractStateMachine::is_valid_transition(
            &ContractStatus::Executing,
            &ContractStatus::Completed
        ));

        // Check some invalid transitions
        assert!(!ContractStateMachine::is_valid_transition(
            &ContractStatus::Completed,
            &ContractStatus::Active
        ));
        assert!(!ContractStateMachine::is_valid_transition(
            &ContractStatus::Draft,
            &ContractStatus::Completed
        ));
    }
}
