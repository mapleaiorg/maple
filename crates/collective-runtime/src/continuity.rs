//! Continuity Manager — checkpointing and recovery for Collectives
//!
//! Manages the collective's continuity chain and state checkpointing.
//! Enables a collective to survive restarts, migrations, and failures
//! while maintaining identity continuity and accountability.

use chrono::{DateTime, Utc};
use collective_types::{
    AuditJournal, CollectiveId, CollectiveMetadata, MembershipGraph, RoleRegistry, Treasury,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// A checkpoint of the collective's complete state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollectiveCheckpoint {
    /// Checkpoint identifier
    pub checkpoint_id: String,
    /// The collective's metadata
    pub metadata: CollectiveMetadata,
    /// Membership state at checkpoint time
    pub membership: MembershipGraph,
    /// Role registry at checkpoint time
    pub role_registry: RoleRegistry,
    /// Treasury state at checkpoint time
    pub treasury: Treasury,
    /// Audit journal at checkpoint time
    pub audit_journal: AuditJournal,
    /// When the checkpoint was taken
    pub created_at: DateTime<Utc>,
    /// Sequence number (monotonically increasing)
    pub sequence: u64,
    /// Hash of the previous checkpoint (for chain integrity)
    pub previous_checkpoint_hash: Option<String>,
}

impl CollectiveCheckpoint {
    /// Create a new checkpoint
    pub fn new(
        metadata: CollectiveMetadata,
        membership: MembershipGraph,
        role_registry: RoleRegistry,
        treasury: Treasury,
        audit_journal: AuditJournal,
        sequence: u64,
        previous_hash: Option<String>,
    ) -> Self {
        Self {
            checkpoint_id: uuid::Uuid::new_v4().to_string(),
            metadata,
            membership,
            role_registry,
            treasury,
            audit_journal,
            created_at: Utc::now(),
            sequence,
            previous_checkpoint_hash: previous_hash,
        }
    }

    /// Compute a hash of this checkpoint (for chain integrity)
    pub fn compute_hash(&self) -> String {
        // Simplified hash — in production, use blake3 over serialized data
        format!(
            "checkpoint:{}:seq:{}:{}",
            self.checkpoint_id,
            self.sequence,
            self.created_at.timestamp()
        )
    }
}

/// Manages continuity and state persistence for a Collective
pub struct ContinuityManager {
    /// The collective ID
    collective_id: CollectiveId,
    /// Current checkpoint sequence
    current_sequence: u64,
    /// Hash of the last checkpoint
    last_checkpoint_hash: Option<String>,
    /// Stored checkpoints (in production, persisted to storage)
    checkpoints: Vec<CollectiveCheckpoint>,
    /// Maximum checkpoints to retain in memory
    max_retained_checkpoints: usize,
}

impl ContinuityManager {
    pub fn new(collective_id: CollectiveId) -> Self {
        Self {
            collective_id,
            current_sequence: 0,
            last_checkpoint_hash: None,
            checkpoints: Vec::new(),
            max_retained_checkpoints: 10,
        }
    }

    /// Set maximum retained checkpoints
    pub fn set_max_retained(&mut self, max: usize) {
        self.max_retained_checkpoints = max;
    }

    /// Take a checkpoint of the current state
    pub fn checkpoint(
        &mut self,
        metadata: CollectiveMetadata,
        membership: MembershipGraph,
        role_registry: RoleRegistry,
        treasury: Treasury,
        audit_journal: AuditJournal,
    ) -> CollectiveCheckpoint {
        self.current_sequence += 1;

        let checkpoint = CollectiveCheckpoint::new(
            metadata,
            membership,
            role_registry,
            treasury,
            audit_journal,
            self.current_sequence,
            self.last_checkpoint_hash.clone(),
        );

        self.last_checkpoint_hash = Some(checkpoint.compute_hash());

        info!(
            collective = %self.collective_id,
            sequence = self.current_sequence,
            checkpoint_id = %checkpoint.checkpoint_id,
            "Collective checkpoint created"
        );

        // Store and trim
        self.checkpoints.push(checkpoint.clone());
        self.trim_checkpoints();

        checkpoint
    }

    /// Restore from the latest checkpoint
    pub fn restore_latest(&self) -> Option<&CollectiveCheckpoint> {
        self.checkpoints.last()
    }

    /// Restore from a specific checkpoint by sequence number
    pub fn restore_by_sequence(&self, sequence: u64) -> Option<&CollectiveCheckpoint> {
        self.checkpoints.iter().find(|c| c.sequence == sequence)
    }

    /// Restore from a specific checkpoint by ID
    pub fn restore_by_id(&self, checkpoint_id: &str) -> Option<&CollectiveCheckpoint> {
        self.checkpoints
            .iter()
            .find(|c| c.checkpoint_id == checkpoint_id)
    }

    /// Verify chain integrity (each checkpoint links to the previous)
    pub fn verify_chain_integrity(&self) -> bool {
        if self.checkpoints.len() <= 1 {
            return true;
        }

        for i in 1..self.checkpoints.len() {
            let current = &self.checkpoints[i];
            let previous = &self.checkpoints[i - 1];

            if let Some(ref prev_hash) = current.previous_checkpoint_hash {
                if *prev_hash != previous.compute_hash() {
                    warn!(
                        sequence = current.sequence,
                        "Checkpoint chain integrity violation"
                    );
                    return false;
                }
            }
        }

        true
    }

    /// Current sequence number
    pub fn current_sequence(&self) -> u64 {
        self.current_sequence
    }

    /// Number of stored checkpoints
    pub fn checkpoint_count(&self) -> usize {
        self.checkpoints.len()
    }

    /// Get all checkpoint IDs with their sequence numbers
    pub fn checkpoint_index(&self) -> Vec<(u64, String)> {
        self.checkpoints
            .iter()
            .map(|c| (c.sequence, c.checkpoint_id.clone()))
            .collect()
    }

    /// Trim checkpoints to max retained count
    fn trim_checkpoints(&mut self) {
        while self.checkpoints.len() > self.max_retained_checkpoints {
            self.checkpoints.remove(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use collective_types::CollectiveSpec;
    use resonator_types::ResonatorId;

    fn make_state(
        coll_id: &CollectiveId,
    ) -> (
        CollectiveMetadata,
        MembershipGraph,
        RoleRegistry,
        Treasury,
        AuditJournal,
    ) {
        let spec = CollectiveSpec::new("Test", "Test collective", ResonatorId::new("creator"));
        let metadata = CollectiveMetadata::new(spec).with_id(coll_id.clone());
        let membership = MembershipGraph::new(coll_id.clone());
        let roles = RoleRegistry::new();
        let treasury = Treasury::new(coll_id.clone());
        let journal = AuditJournal::new(coll_id.clone());
        (metadata, membership, roles, treasury, journal)
    }

    #[test]
    fn test_checkpoint_creation() {
        let coll_id = CollectiveId::new("test");
        let mut mgr = ContinuityManager::new(coll_id.clone());

        let (meta, membership, roles, treasury, journal) = make_state(&coll_id);
        let cp = mgr.checkpoint(meta, membership, roles, treasury, journal);

        assert_eq!(cp.sequence, 1);
        assert_eq!(mgr.current_sequence(), 1);
        assert_eq!(mgr.checkpoint_count(), 1);
    }

    #[test]
    fn test_restore_latest() {
        let coll_id = CollectiveId::new("test");
        let mut mgr = ContinuityManager::new(coll_id.clone());

        // Create two checkpoints
        let (meta, membership, roles, treasury, journal) = make_state(&coll_id);
        mgr.checkpoint(meta, membership, roles, treasury, journal);

        let (meta2, membership2, roles2, treasury2, journal2) = make_state(&coll_id);
        mgr.checkpoint(meta2, membership2, roles2, treasury2, journal2);

        let latest = mgr.restore_latest().unwrap();
        assert_eq!(latest.sequence, 2);
    }

    #[test]
    fn test_restore_by_sequence() {
        let coll_id = CollectiveId::new("test");
        let mut mgr = ContinuityManager::new(coll_id.clone());

        let (meta, membership, roles, treasury, journal) = make_state(&coll_id);
        mgr.checkpoint(meta, membership, roles, treasury, journal);

        let (meta2, membership2, roles2, treasury2, journal2) = make_state(&coll_id);
        mgr.checkpoint(meta2, membership2, roles2, treasury2, journal2);

        let first = mgr.restore_by_sequence(1).unwrap();
        assert_eq!(first.sequence, 1);

        let second = mgr.restore_by_sequence(2).unwrap();
        assert_eq!(second.sequence, 2);

        assert!(mgr.restore_by_sequence(99).is_none());
    }

    #[test]
    fn test_chain_integrity() {
        let coll_id = CollectiveId::new("test");
        let mut mgr = ContinuityManager::new(coll_id.clone());

        for _ in 0..5 {
            let (meta, membership, roles, treasury, journal) = make_state(&coll_id);
            mgr.checkpoint(meta, membership, roles, treasury, journal);
        }

        assert!(mgr.verify_chain_integrity());
    }

    #[test]
    fn test_checkpoint_trimming() {
        let coll_id = CollectiveId::new("test");
        let mut mgr = ContinuityManager::new(coll_id.clone());
        mgr.set_max_retained(3);

        for _ in 0..10 {
            let (meta, membership, roles, treasury, journal) = make_state(&coll_id);
            mgr.checkpoint(meta, membership, roles, treasury, journal);
        }

        assert_eq!(mgr.checkpoint_count(), 3);
        assert_eq!(mgr.current_sequence(), 10);

        // Earliest retained should be sequence 8
        let index = mgr.checkpoint_index();
        assert_eq!(index[0].0, 8);
    }
}
