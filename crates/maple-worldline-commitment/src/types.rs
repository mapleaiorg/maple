//! Core type definitions for the self-commitment engine.
//!
//! Types for tracking self-commitment lifecycle from intent through
//! gate adjudication to outcome.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use maple_worldline_intent::types::{IntentId, SubstrateTier};

// Re-export for convenience
pub use worldline_core::types::CommitmentId;

// ── Identifier Types ────────────────────────────────────────────────────

/// Unique identifier for a self-commitment record.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SelfCommitmentId(pub String);

impl SelfCommitmentId {
    /// Generate a new unique self-commitment ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for SelfCommitmentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SelfCommitmentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "self-cmt:{}", self.0)
    }
}

// ── Lifecycle Status ────────────────────────────────────────────────────

/// Lifecycle status of a self-commitment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CommitmentLifecycleStatus {
    /// Observation period is in progress.
    PendingObservation,
    /// Observation period completed; ready for gate submission.
    ObservationComplete,
    /// Submitted to the commitment gate.
    Submitted,
    /// Approved by the commitment gate.
    Approved,
    /// Denied by the commitment gate.
    Denied(String),
    /// Self-modification completed successfully.
    Fulfilled,
    /// Self-modification failed.
    Failed(String),
    /// Deferred for later evaluation.
    Deferred(String),
}

impl CommitmentLifecycleStatus {
    /// Whether this status is terminal (no further transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Denied(_) | Self::Fulfilled | Self::Failed(_))
    }

    /// Whether this commitment is active (not terminal, not deferred).
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::PendingObservation | Self::ObservationComplete | Self::Submitted | Self::Approved
        )
    }
}

impl std::fmt::Display for CommitmentLifecycleStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PendingObservation => write!(f, "pending-observation"),
            Self::ObservationComplete => write!(f, "observation-complete"),
            Self::Submitted => write!(f, "submitted"),
            Self::Approved => write!(f, "approved"),
            Self::Denied(reason) => write!(f, "denied: {}", reason),
            Self::Fulfilled => write!(f, "fulfilled"),
            Self::Failed(reason) => write!(f, "failed: {}", reason),
            Self::Deferred(reason) => write!(f, "deferred: {}", reason),
        }
    }
}

// ── Commitment Record ───────────────────────────────────────────────────

/// A record tracking the lifecycle of a self-commitment.
#[derive(Clone, Debug)]
pub struct CommitmentRecord {
    /// Unique self-commitment identifier.
    pub id: SelfCommitmentId,
    /// The intent this commitment was derived from.
    pub intent_id: IntentId,
    /// The commitment ID from the gate (set after submission).
    pub commitment_id: Option<CommitmentId>,
    /// Governance tier of the original intent.
    pub governance_tier: SubstrateTier,
    /// When the observation period started.
    pub observation_start: DateTime<Utc>,
    /// Required observation period (seconds).
    pub observation_required_secs: u64,
    /// Current lifecycle status.
    pub status: CommitmentLifecycleStatus,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
    /// When the commitment was resolved (terminal state reached).
    pub resolved_at: Option<DateTime<Utc>>,
}

impl CommitmentRecord {
    /// Whether observation has been completed based on elapsed time.
    pub fn observation_elapsed(&self) -> bool {
        let elapsed = (Utc::now() - self.observation_start).num_seconds().max(0) as u64;
        elapsed >= self.observation_required_secs
    }
}

// ── Configuration ───────────────────────────────────────────────────────

/// Configuration for the self-commitment engine.
#[derive(Clone, Debug)]
pub struct CommitmentConfig {
    /// Minimum confidence for an intent to be committed.
    pub min_confidence: f64,
    /// Whether to enforce observation periods.
    pub require_observation_period: bool,
    /// Whether rollback plans are mandatory.
    pub require_rollback: bool,
    /// Maximum concurrent active commitments.
    pub max_concurrent_commitments: usize,
    /// Maximum tracked commitment records (bounded storage).
    pub max_tracked_commitments: usize,
}

impl Default for CommitmentConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.8,
            require_observation_period: true,
            require_rollback: true,
            max_concurrent_commitments: 3,
            max_tracked_commitments: 256,
        }
    }
}

// ── Summary ─────────────────────────────────────────────────────────────

/// Summary statistics for commitment tracking.
#[derive(Clone, Debug, Default)]
pub struct CommitmentSummary {
    /// Total number of tracked commitments.
    pub total: usize,
    /// Commitments pending observation or submission.
    pub pending: usize,
    /// Approved commitments.
    pub approved: usize,
    /// Denied commitments.
    pub denied: usize,
    /// Successfully fulfilled commitments.
    pub fulfilled: usize,
    /// Failed commitments.
    pub failed: usize,
}

impl std::fmt::Display for CommitmentSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "total={}, pending={}, approved={}, denied={}, fulfilled={}, failed={}",
            self.total, self.pending, self.approved, self.denied, self.fulfilled, self.failed
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_commitment_id_uniqueness() {
        let a = SelfCommitmentId::new();
        let b = SelfCommitmentId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn self_commitment_id_display() {
        let id = SelfCommitmentId::new();
        assert!(id.to_string().starts_with("self-cmt:"));
    }

    #[test]
    fn lifecycle_status_display() {
        assert_eq!(
            CommitmentLifecycleStatus::PendingObservation.to_string(),
            "pending-observation"
        );
        assert_eq!(
            CommitmentLifecycleStatus::Denied("risk too high".into()).to_string(),
            "denied: risk too high"
        );
    }

    #[test]
    fn lifecycle_terminal_states() {
        assert!(!CommitmentLifecycleStatus::PendingObservation.is_terminal());
        assert!(!CommitmentLifecycleStatus::Submitted.is_terminal());
        assert!(!CommitmentLifecycleStatus::Approved.is_terminal());
        assert!(CommitmentLifecycleStatus::Denied("x".into()).is_terminal());
        assert!(CommitmentLifecycleStatus::Fulfilled.is_terminal());
        assert!(CommitmentLifecycleStatus::Failed("x".into()).is_terminal());
    }

    #[test]
    fn lifecycle_active_states() {
        assert!(CommitmentLifecycleStatus::PendingObservation.is_active());
        assert!(CommitmentLifecycleStatus::ObservationComplete.is_active());
        assert!(CommitmentLifecycleStatus::Submitted.is_active());
        assert!(CommitmentLifecycleStatus::Approved.is_active());
        assert!(!CommitmentLifecycleStatus::Deferred("x".into()).is_active());
        assert!(!CommitmentLifecycleStatus::Fulfilled.is_active());
    }

    #[test]
    fn config_defaults() {
        let cfg = CommitmentConfig::default();
        assert!((cfg.min_confidence - 0.8).abs() < f64::EPSILON);
        assert!(cfg.require_observation_period);
        assert!(cfg.require_rollback);
        assert_eq!(cfg.max_concurrent_commitments, 3);
        assert_eq!(cfg.max_tracked_commitments, 256);
    }

    #[test]
    fn summary_display() {
        let s = CommitmentSummary {
            total: 10,
            pending: 2,
            approved: 3,
            denied: 1,
            fulfilled: 3,
            failed: 1,
        };
        let display = s.to_string();
        assert!(display.contains("total=10"));
        assert!(display.contains("fulfilled=3"));
    }

    #[test]
    fn summary_default_all_zero() {
        let s = CommitmentSummary::default();
        assert_eq!(s.total, 0);
        assert_eq!(s.pending, 0);
        assert_eq!(s.approved, 0);
    }
}
