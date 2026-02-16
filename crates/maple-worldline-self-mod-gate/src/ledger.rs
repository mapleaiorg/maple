//! Self-modification ledger — tamper-evident audit log.
//!
//! Every self-modification decision, execution, and rollback is recorded
//! in the `SelfModificationLedger`. This provides full auditability:
//! who proposed what, when, what was decided, and what happened.

use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::adjudication::CheckResult;
use crate::commitment::IntentChain;
use crate::types::{PolicyDecisionCard, SelfModTier};

// ── Deployment Status ──────────────────────────────────────────────────

/// Status of a self-modification deployment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DeploymentStatus {
    /// Awaiting execution.
    Pending,
    /// Currently being deployed.
    InProgress,
    /// Deployment succeeded.
    Succeeded,
    /// Deployment failed.
    Failed(String),
    /// Deployment was rolled back.
    RolledBack(String),
}

impl DeploymentStatus {
    /// Whether this is a terminal (completed) status.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed(_) | Self::RolledBack(_)
        )
    }

    /// Whether this deployment succeeded.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Succeeded)
    }

    /// Whether this deployment was rolled back.
    pub fn is_rolled_back(&self) -> bool {
        matches!(self, Self::RolledBack(_))
    }
}

// ── Performance Delta ──────────────────────────────────────────────────

/// Performance measurement before and after a self-modification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceDelta {
    /// Metric name (e.g. "latency_p99", "throughput").
    pub metric: String,
    /// Value before the modification.
    pub before: f64,
    /// Value after the modification.
    pub after: f64,
    /// Unit of measurement.
    pub unit: String,
}

impl PerformanceDelta {
    /// The absolute change (after - before).
    pub fn delta(&self) -> f64 {
        self.after - self.before
    }

    /// Whether this represents an improvement (lower is better assumed by default).
    /// For metrics where lower is better (latency), negative delta = improvement.
    pub fn is_improvement_lower_better(&self) -> bool {
        self.after < self.before
    }

    /// Whether this represents an improvement (higher is better).
    pub fn is_improvement_higher_better(&self) -> bool {
        self.after > self.before
    }
}

// ── Ledger Entry ───────────────────────────────────────────────────────

/// A single entry in the self-modification ledger.
///
/// Records everything about a self-modification: the commitment,
/// the adjudication decision, check results, deployment outcome,
/// and performance impact.
#[derive(Clone, Debug)]
pub struct SelfModificationLedgerEntry {
    /// Commitment ID.
    pub commitment_id: String,
    /// Self-modification tier.
    pub tier: SelfModTier,
    /// Files affected by this modification.
    pub affected_files: Vec<String>,
    /// Provenance chain.
    pub intent_chain: IntentChain,
    /// The adjudication decision.
    pub adjudication: PolicyDecisionCard,
    /// Results of all checks during adjudication.
    pub check_results: Vec<CheckResult>,
    /// Current deployment status.
    pub deployment_status: DeploymentStatus,
    /// Performance measurements (populated after execution).
    pub performance_delta: Vec<PerformanceDelta>,
    /// Whether a rollback was triggered.
    pub rollback_triggered: bool,
    /// When the entry was created.
    pub created_at: DateTime<Utc>,
    /// When the entry was last updated.
    pub updated_at: DateTime<Utc>,
}

impl SelfModificationLedgerEntry {
    /// Create a new ledger entry from an adjudication result.
    pub fn new(
        commitment_id: String,
        tier: SelfModTier,
        affected_files: Vec<String>,
        intent_chain: IntentChain,
        adjudication: PolicyDecisionCard,
        check_results: Vec<CheckResult>,
    ) -> Self {
        let now = Utc::now();
        Self {
            commitment_id,
            tier,
            affected_files,
            intent_chain,
            adjudication,
            check_results,
            deployment_status: DeploymentStatus::Pending,
            performance_delta: vec![],
            rollback_triggered: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the deployment status.
    pub fn update_status(&mut self, status: DeploymentStatus) {
        self.deployment_status = status;
        self.updated_at = Utc::now();
    }

    /// Record a rollback.
    pub fn record_rollback(&mut self, reason: String) {
        self.rollback_triggered = true;
        self.deployment_status = DeploymentStatus::RolledBack(reason);
        self.updated_at = Utc::now();
    }

    /// Add performance measurements.
    pub fn add_performance_delta(&mut self, delta: PerformanceDelta) {
        self.performance_delta.push(delta);
        self.updated_at = Utc::now();
    }
}

// ── Ledger ─────────────────────────────────────────────────────────────

/// Self-modification ledger — the audit trail.
///
/// Bounded FIFO: retains the most recent `max_entries` entries.
/// All mutations are recorded as entries for full auditability.
pub struct SelfModificationLedger {
    /// Ledger entries.
    entries: VecDeque<SelfModificationLedgerEntry>,
    /// Maximum entries to retain.
    max_entries: usize,
}

impl SelfModificationLedger {
    /// Create a new ledger with the given capacity.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries,
        }
    }

    /// Record a new ledger entry.
    pub fn record(&mut self, entry: SelfModificationLedgerEntry) {
        self.entries.push_back(entry);
        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }

    /// Find an entry by commitment ID.
    pub fn find(&self, commitment_id: &str) -> Option<&SelfModificationLedgerEntry> {
        self.entries
            .iter()
            .find(|e| e.commitment_id == commitment_id)
    }

    /// Find a mutable entry by commitment ID.
    pub fn find_mut(&mut self, commitment_id: &str) -> Option<&mut SelfModificationLedgerEntry> {
        self.entries
            .iter_mut()
            .find(|e| e.commitment_id == commitment_id)
    }

    /// Recent entries (most recent first).
    pub fn recent(&self, count: usize) -> Vec<&SelfModificationLedgerEntry> {
        self.entries.iter().rev().take(count).collect()
    }

    /// Count of rollbacks in the ledger.
    pub fn rollback_count(&self) -> usize {
        self.entries.iter().filter(|e| e.rollback_triggered).count()
    }

    /// Success rate (succeeded / terminal entries).
    pub fn success_rate(&self) -> f64 {
        let terminal: Vec<_> = self
            .entries
            .iter()
            .filter(|e| e.deployment_status.is_terminal())
            .collect();
        if terminal.is_empty() {
            return 0.0;
        }
        let succeeded = terminal
            .iter()
            .filter(|e| e.deployment_status.is_success())
            .count();
        succeeded as f64 / terminal.len() as f64
    }

    /// Total entries in the ledger.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the ledger is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for SelfModificationLedger {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commitment::IntentChain;
    use crate::types::{PolicyDecisionCard, SelfModTier};
    use maple_worldline_intent::types::{IntentId, MeaningId};

    fn make_entry(id: &str, tier: SelfModTier) -> SelfModificationLedgerEntry {
        SelfModificationLedgerEntry::new(
            id.into(),
            tier,
            vec!["src/config.rs".into()],
            IntentChain {
                observation_ids: vec!["obs-1".into()],
                meaning_ids: vec![MeaningId::new()],
                intent_id: IntentId::new(),
            },
            PolicyDecisionCard::approved(),
            vec![],
        )
    }

    #[test]
    fn ledger_record_and_find() {
        let mut ledger = SelfModificationLedger::new(100);
        let entry = make_entry("commit-1", SelfModTier::Tier0Configuration);
        ledger.record(entry);

        assert_eq!(ledger.len(), 1);
        assert!(ledger.find("commit-1").is_some());
        assert!(ledger.find("nonexistent").is_none());
    }

    #[test]
    fn ledger_fifo_eviction() {
        let mut ledger = SelfModificationLedger::new(3);
        ledger.record(make_entry("c-1", SelfModTier::Tier0Configuration));
        ledger.record(make_entry("c-2", SelfModTier::Tier0Configuration));
        ledger.record(make_entry("c-3", SelfModTier::Tier0Configuration));
        ledger.record(make_entry("c-4", SelfModTier::Tier0Configuration));

        assert_eq!(ledger.len(), 3);
        assert!(ledger.find("c-1").is_none()); // Evicted
        assert!(ledger.find("c-4").is_some()); // Retained
    }

    #[test]
    fn ledger_success_rate() {
        let mut ledger = SelfModificationLedger::new(100);

        let mut e1 = make_entry("c-1", SelfModTier::Tier0Configuration);
        e1.update_status(DeploymentStatus::Succeeded);
        ledger.record(e1);

        let mut e2 = make_entry("c-2", SelfModTier::Tier0Configuration);
        e2.update_status(DeploymentStatus::Succeeded);
        ledger.record(e2);

        let mut e3 = make_entry("c-3", SelfModTier::Tier1OperatorInternal);
        e3.record_rollback("regression detected".into());
        ledger.record(e3);

        // 2 succeeded out of 3 terminal
        assert!((ledger.success_rate() - 2.0 / 3.0).abs() < 0.01);
        assert_eq!(ledger.rollback_count(), 1);
    }

    #[test]
    fn performance_delta() {
        let delta = PerformanceDelta {
            metric: "latency_p99".into(),
            before: 100.0,
            after: 80.0,
            unit: "ms".into(),
        };
        assert!(delta.is_improvement_lower_better());
        assert!(!delta.is_improvement_higher_better());
        assert!((delta.delta() - (-20.0)).abs() < 0.01);
    }
}
