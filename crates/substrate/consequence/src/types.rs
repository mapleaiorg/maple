//! Core type definitions for the self-consequence engine.
//!
//! Types for tracking consequence lifecycle from approved commitment
//! through execution to outcome recording.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use maple_worldline_commitment::types::SelfCommitmentId;
use maple_worldline_intent::types::{IntentId, SubstrateTier};

use crate::receipt::ExecutionReceipt;

// ── Identifier Types ────────────────────────────────────────────────────

/// Unique identifier for a self-consequence record.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SelfConsequenceId(pub String);

impl SelfConsequenceId {
    /// Generate a new unique self-consequence ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for SelfConsequenceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SelfConsequenceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "self-csq:{}", self.0)
    }
}

// ── Consequence Status ──────────────────────────────────────────────────

/// Lifecycle status of a self-consequence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ConsequenceStatus {
    /// Awaiting execution.
    Pending,
    /// Execution in progress.
    Executing,
    /// Modification applied successfully.
    Succeeded,
    /// Execution failed.
    Failed(String),
    /// Failed and rolled back to previous state.
    RolledBack(String),
}

impl ConsequenceStatus {
    /// Whether this status is terminal (no further transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed(_) | Self::RolledBack(_)
        )
    }

    /// Whether the consequence succeeded.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Succeeded)
    }
}

impl std::fmt::Display for ConsequenceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Executing => write!(f, "executing"),
            Self::Succeeded => write!(f, "succeeded"),
            Self::Failed(reason) => write!(f, "failed: {}", reason),
            Self::RolledBack(reason) => write!(f, "rolled-back: {}", reason),
        }
    }
}

// ── Consequence Record ──────────────────────────────────────────────────

/// A record tracking the lifecycle of a self-consequence execution.
#[derive(Clone, Debug)]
pub struct ConsequenceRecord {
    /// Unique consequence identifier.
    pub id: SelfConsequenceId,
    /// The self-commitment this consequence executes.
    pub self_commitment_id: SelfCommitmentId,
    /// The intent that originated this consequence chain.
    pub intent_id: IntentId,
    /// Governance tier of the original intent.
    pub governance_tier: SubstrateTier,
    /// Current lifecycle status.
    pub status: ConsequenceStatus,
    /// When execution started.
    pub execution_start: Option<DateTime<Utc>>,
    /// When execution completed.
    pub execution_end: Option<DateTime<Utc>>,
    /// Execution duration in milliseconds.
    pub duration_ms: Option<i64>,
    /// Cryptographic receipt proving execution (set on success).
    pub receipt: Option<ExecutionReceipt>,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
    /// Number of tests that passed during execution.
    pub tests_passed: usize,
    /// Number of tests that failed during execution.
    pub tests_failed: usize,
    /// Whether a rollback was attempted.
    pub rollback_attempted: bool,
}

impl ConsequenceRecord {
    /// Create a new consequence record in Pending state.
    pub fn new(
        self_commitment_id: SelfCommitmentId,
        intent_id: IntentId,
        governance_tier: SubstrateTier,
    ) -> Self {
        Self {
            id: SelfConsequenceId::new(),
            self_commitment_id,
            intent_id,
            governance_tier,
            status: ConsequenceStatus::Pending,
            execution_start: None,
            execution_end: None,
            duration_ms: None,
            receipt: None,
            created_at: Utc::now(),
            tests_passed: 0,
            tests_failed: 0,
            rollback_attempted: false,
        }
    }

    /// Mark execution as started.
    pub fn mark_executing(&mut self) {
        self.status = ConsequenceStatus::Executing;
        self.execution_start = Some(Utc::now());
    }

    /// Mark execution as succeeded with receipt.
    pub fn mark_succeeded(&mut self, receipt: ExecutionReceipt, tests_passed: usize) {
        let now = Utc::now();
        self.status = ConsequenceStatus::Succeeded;
        self.execution_end = Some(now);
        self.tests_passed = tests_passed;
        if let Some(start) = self.execution_start {
            self.duration_ms = Some((now - start).num_milliseconds());
        }
        self.receipt = Some(receipt);
    }

    /// Mark execution as failed.
    pub fn mark_failed(&mut self, reason: String, tests_passed: usize, tests_failed: usize) {
        let now = Utc::now();
        self.status = ConsequenceStatus::Failed(reason);
        self.execution_end = Some(now);
        self.tests_passed = tests_passed;
        self.tests_failed = tests_failed;
        if let Some(start) = self.execution_start {
            self.duration_ms = Some((now - start).num_milliseconds());
        }
    }

    /// Mark as rolled back after failure.
    pub fn mark_rolled_back(&mut self, reason: String) {
        self.status = ConsequenceStatus::RolledBack(reason);
        self.rollback_attempted = true;
    }
}

// ── Configuration ───────────────────────────────────────────────────────

/// Configuration for the self-consequence engine.
#[derive(Clone, Debug)]
pub struct ConsequenceConfig {
    /// Maximum execution duration (seconds) before timeout.
    pub max_execution_duration_secs: u64,
    /// Whether all proposal tests must pass.
    pub require_tests_pass: bool,
    /// Whether performance gates must be satisfied.
    pub require_performance_gates: bool,
    /// Maximum tracked consequence records (bounded storage).
    pub max_tracked_consequences: usize,
    /// Whether to automatically roll back on execution failure.
    pub auto_rollback_on_failure: bool,
}

impl Default for ConsequenceConfig {
    fn default() -> Self {
        Self {
            max_execution_duration_secs: 3600,
            require_tests_pass: true,
            require_performance_gates: true,
            max_tracked_consequences: 256,
            auto_rollback_on_failure: true,
        }
    }
}

// ── Summary ─────────────────────────────────────────────────────────────

/// Summary statistics for consequence tracking.
#[derive(Clone, Debug, Default)]
pub struct ConsequenceSummary {
    /// Total number of tracked consequences.
    pub total: usize,
    /// Consequences pending execution.
    pub pending: usize,
    /// Successfully executed consequences.
    pub succeeded: usize,
    /// Failed consequences.
    pub failed: usize,
    /// Rolled-back consequences.
    pub rolled_back: usize,
}

impl std::fmt::Display for ConsequenceSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "total={}, pending={}, succeeded={}, failed={}, rolled_back={}",
            self.total, self.pending, self.succeeded, self.failed, self.rolled_back
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_consequence_id_uniqueness() {
        let a = SelfConsequenceId::new();
        let b = SelfConsequenceId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn self_consequence_id_display() {
        let id = SelfConsequenceId::new();
        assert!(id.to_string().starts_with("self-csq:"));
    }

    #[test]
    fn consequence_status_display() {
        assert_eq!(ConsequenceStatus::Pending.to_string(), "pending");
        assert_eq!(ConsequenceStatus::Executing.to_string(), "executing");
        assert_eq!(ConsequenceStatus::Succeeded.to_string(), "succeeded");
        assert_eq!(
            ConsequenceStatus::Failed("timeout".into()).to_string(),
            "failed: timeout"
        );
        assert_eq!(
            ConsequenceStatus::RolledBack("reverted".into()).to_string(),
            "rolled-back: reverted"
        );
    }

    #[test]
    fn consequence_status_terminal() {
        assert!(!ConsequenceStatus::Pending.is_terminal());
        assert!(!ConsequenceStatus::Executing.is_terminal());
        assert!(ConsequenceStatus::Succeeded.is_terminal());
        assert!(ConsequenceStatus::Failed("x".into()).is_terminal());
        assert!(ConsequenceStatus::RolledBack("x".into()).is_terminal());
    }

    #[test]
    fn consequence_status_success() {
        assert!(!ConsequenceStatus::Pending.is_success());
        assert!(ConsequenceStatus::Succeeded.is_success());
        assert!(!ConsequenceStatus::Failed("x".into()).is_success());
    }

    #[test]
    fn consequence_record_lifecycle() {
        use crate::receipt::ExecutionReceipt;

        let mut record = ConsequenceRecord::new(
            SelfCommitmentId::new(),
            IntentId::new(),
            SubstrateTier::Tier0,
        );
        assert!(matches!(record.status, ConsequenceStatus::Pending));
        assert!(record.execution_start.is_none());

        record.mark_executing();
        assert!(matches!(record.status, ConsequenceStatus::Executing));
        assert!(record.execution_start.is_some());

        let receipt = ExecutionReceipt::new(
            record.id.clone(),
            record.self_commitment_id.clone(),
            record.intent_id.clone(),
            SubstrateTier::Tier0,
            5,
            "test execution",
        );
        record.mark_succeeded(receipt, 5);
        assert!(matches!(record.status, ConsequenceStatus::Succeeded));
        assert!(record.execution_end.is_some());
        assert_eq!(record.tests_passed, 5);
        assert!(record.receipt.is_some());
    }

    #[test]
    fn consequence_record_failure_and_rollback() {
        let mut record = ConsequenceRecord::new(
            SelfCommitmentId::new(),
            IntentId::new(),
            SubstrateTier::Tier1,
        );
        record.mark_executing();
        record.mark_failed("compilation error".into(), 2, 1);
        assert!(matches!(record.status, ConsequenceStatus::Failed(_)));
        assert_eq!(record.tests_passed, 2);
        assert_eq!(record.tests_failed, 1);

        record.mark_rolled_back("reverted via git".into());
        assert!(matches!(record.status, ConsequenceStatus::RolledBack(_)));
        assert!(record.rollback_attempted);
    }

    #[test]
    fn config_defaults() {
        let cfg = ConsequenceConfig::default();
        assert_eq!(cfg.max_execution_duration_secs, 3600);
        assert!(cfg.require_tests_pass);
        assert!(cfg.require_performance_gates);
        assert_eq!(cfg.max_tracked_consequences, 256);
        assert!(cfg.auto_rollback_on_failure);
    }

    #[test]
    fn summary_display() {
        let s = ConsequenceSummary {
            total: 10,
            pending: 1,
            succeeded: 6,
            failed: 2,
            rolled_back: 1,
        };
        let display = s.to_string();
        assert!(display.contains("total=10"));
        assert!(display.contains("succeeded=6"));
        assert!(display.contains("rolled_back=1"));
    }

    #[test]
    fn summary_default_all_zero() {
        let s = ConsequenceSummary::default();
        assert_eq!(s.total, 0);
        assert_eq!(s.pending, 0);
        assert_eq!(s.succeeded, 0);
        assert_eq!(s.failed, 0);
        assert_eq!(s.rolled_back, 0);
    }
}
