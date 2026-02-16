//! Commitment lifecycle manager — tracks the full lifecycle of self-commitments.
//!
//! Records state transitions from submission through gate adjudication
//! to final outcome (fulfilled/failed). Provides bounded storage with
//! FIFO eviction and summary statistics.

use std::collections::VecDeque;

use chrono::Utc;

use maple_worldline_intent::types::{IntentId, SubstrateTier};
use worldline_core::types::CommitmentId;

use crate::types::{
    CommitmentLifecycleStatus, CommitmentRecord, CommitmentSummary, SelfCommitmentId,
};

// ── Commitment Lifecycle Manager ────────────────────────────────────────

/// Manages the lifecycle of self-commitment records.
///
/// Provides bounded storage (FIFO eviction) and query methods.
pub struct CommitmentLifecycleManager {
    records: VecDeque<CommitmentRecord>,
    max_tracked: usize,
}

impl Default for CommitmentLifecycleManager {
    fn default() -> Self {
        Self::new(256)
    }
}

impl CommitmentLifecycleManager {
    /// Create a new lifecycle manager with the given capacity.
    pub fn new(max_tracked: usize) -> Self {
        Self {
            records: VecDeque::new(),
            max_tracked,
        }
    }

    /// Record a new commitment submission.
    pub fn record_submission(
        &mut self,
        intent_id: IntentId,
        commitment_id: CommitmentId,
        governance_tier: SubstrateTier,
    ) -> SelfCommitmentId {
        let id = SelfCommitmentId::new();
        let now = Utc::now();

        let record = CommitmentRecord {
            id: id.clone(),
            intent_id,
            commitment_id: Some(commitment_id),
            governance_tier,
            observation_start: now,
            observation_required_secs: 0, // already observed
            status: CommitmentLifecycleStatus::Submitted,
            created_at: now,
            resolved_at: None,
        };

        self.push_record(record);
        id
    }

    /// Record that a commitment was approved by the gate.
    pub fn record_approval(&mut self, self_commitment_id: &SelfCommitmentId) {
        if let Some(record) = self.find_mut(self_commitment_id) {
            record.status = CommitmentLifecycleStatus::Approved;
        }
    }

    /// Record that a commitment was denied by the gate.
    pub fn record_denial(&mut self, self_commitment_id: &SelfCommitmentId, reason: String) {
        if let Some(record) = self.find_mut(self_commitment_id) {
            record.status = CommitmentLifecycleStatus::Denied(reason);
            record.resolved_at = Some(Utc::now());
        }
    }

    /// Record that a commitment was fulfilled (self-modification succeeded).
    pub fn record_fulfilled(&mut self, self_commitment_id: &SelfCommitmentId) {
        if let Some(record) = self.find_mut(self_commitment_id) {
            record.status = CommitmentLifecycleStatus::Fulfilled;
            record.resolved_at = Some(Utc::now());
        }
    }

    /// Record that a commitment failed (self-modification failed).
    pub fn record_failed(&mut self, self_commitment_id: &SelfCommitmentId, reason: String) {
        if let Some(record) = self.find_mut(self_commitment_id) {
            record.status = CommitmentLifecycleStatus::Failed(reason);
            record.resolved_at = Some(Utc::now());
        }
    }

    /// Find a record by self-commitment ID.
    pub fn find(&self, self_commitment_id: &SelfCommitmentId) -> Option<&CommitmentRecord> {
        self.records.iter().find(|r| r.id == *self_commitment_id)
    }

    /// Find a record by intent ID.
    pub fn find_by_intent(&self, intent_id: &IntentId) -> Option<&CommitmentRecord> {
        self.records.iter().find(|r| r.intent_id == *intent_id)
    }

    /// Find a record by gate commitment ID.
    pub fn find_by_commitment(&self, commitment_id: &CommitmentId) -> Option<&CommitmentRecord> {
        self.records
            .iter()
            .find(|r| r.commitment_id.as_ref() == Some(commitment_id))
    }

    /// Get all active (non-terminal, non-deferred) commitment records.
    pub fn active_commitments(&self) -> Vec<&CommitmentRecord> {
        self.records
            .iter()
            .filter(|r| r.status.is_active())
            .collect()
    }

    /// Get all records.
    pub fn all_records(&self) -> &VecDeque<CommitmentRecord> {
        &self.records
    }

    /// Compute summary statistics.
    pub fn summary(&self) -> CommitmentSummary {
        let mut s = CommitmentSummary {
            total: self.records.len(),
            ..Default::default()
        };

        for r in &self.records {
            match &r.status {
                CommitmentLifecycleStatus::PendingObservation
                | CommitmentLifecycleStatus::ObservationComplete
                | CommitmentLifecycleStatus::Submitted => s.pending += 1,
                CommitmentLifecycleStatus::Approved => s.approved += 1,
                CommitmentLifecycleStatus::Denied(_) => s.denied += 1,
                CommitmentLifecycleStatus::Fulfilled => s.fulfilled += 1,
                CommitmentLifecycleStatus::Failed(_) => s.failed += 1,
                CommitmentLifecycleStatus::Deferred(_) => s.pending += 1,
            }
        }

        s
    }

    /// Number of tracked records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the manager has no records.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    // ── Internal ────────────────────────────────────────────────────────

    fn push_record(&mut self, record: CommitmentRecord) {
        if self.records.len() >= self.max_tracked {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }

    fn find_mut(&mut self, id: &SelfCommitmentId) -> Option<&mut CommitmentRecord> {
        self.records.iter_mut().find(|r| r.id == *id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_submission() -> (SelfCommitmentId, CommitmentLifecycleManager) {
        let mut mgr = CommitmentLifecycleManager::new(256);
        let id = mgr.record_submission(IntentId::new(), CommitmentId::new(), SubstrateTier::Tier0);
        (id, mgr)
    }

    use maple_worldline_intent::types::IntentId;

    #[test]
    fn record_submission_creates_record() {
        let (id, mgr) = make_submission();
        assert_eq!(mgr.len(), 1);
        let record = mgr.find(&id).unwrap();
        assert!(matches!(
            record.status,
            CommitmentLifecycleStatus::Submitted
        ));
    }

    #[test]
    fn approval_lifecycle() {
        let (id, mut mgr) = make_submission();
        mgr.record_approval(&id);

        let record = mgr.find(&id).unwrap();
        assert!(matches!(record.status, CommitmentLifecycleStatus::Approved));
        assert!(record.status.is_active());
    }

    #[test]
    fn denial_lifecycle() {
        let (id, mut mgr) = make_submission();
        mgr.record_denial(&id, "policy denied".into());

        let record = mgr.find(&id).unwrap();
        assert!(matches!(
            record.status,
            CommitmentLifecycleStatus::Denied(_)
        ));
        assert!(record.status.is_terminal());
        assert!(record.resolved_at.is_some());
    }

    #[test]
    fn fulfilled_lifecycle() {
        let (id, mut mgr) = make_submission();
        mgr.record_approval(&id);
        mgr.record_fulfilled(&id);

        let record = mgr.find(&id).unwrap();
        assert!(matches!(
            record.status,
            CommitmentLifecycleStatus::Fulfilled
        ));
    }

    #[test]
    fn failed_lifecycle() {
        let (id, mut mgr) = make_submission();
        mgr.record_approval(&id);
        mgr.record_failed(&id, "rollback triggered".into());

        let record = mgr.find(&id).unwrap();
        assert!(matches!(
            record.status,
            CommitmentLifecycleStatus::Failed(_)
        ));
    }

    #[test]
    fn find_by_intent() {
        let mut mgr = CommitmentLifecycleManager::new(256);
        let intent_id = IntentId::new();
        mgr.record_submission(intent_id.clone(), CommitmentId::new(), SubstrateTier::Tier1);

        assert!(mgr.find_by_intent(&intent_id).is_some());
        assert!(mgr.find_by_intent(&IntentId::new()).is_none());
    }

    #[test]
    fn summary_statistics() {
        let mut mgr = CommitmentLifecycleManager::new(256);

        let id1 = mgr.record_submission(IntentId::new(), CommitmentId::new(), SubstrateTier::Tier0);
        let id2 = mgr.record_submission(IntentId::new(), CommitmentId::new(), SubstrateTier::Tier0);
        let _id3 =
            mgr.record_submission(IntentId::new(), CommitmentId::new(), SubstrateTier::Tier0);

        mgr.record_approval(&id1);
        mgr.record_fulfilled(&id1);
        mgr.record_denial(&id2, "risk".into());
        // id3 stays submitted

        let s = mgr.summary();
        assert_eq!(s.total, 3);
        assert_eq!(s.fulfilled, 1);
        assert_eq!(s.denied, 1);
        assert_eq!(s.pending, 1);
    }

    #[test]
    fn bounded_storage_evicts_oldest() {
        let mut mgr = CommitmentLifecycleManager::new(3);

        for _ in 0..5 {
            mgr.record_submission(IntentId::new(), CommitmentId::new(), SubstrateTier::Tier0);
        }

        assert_eq!(mgr.len(), 3); // capped at max
    }
}
