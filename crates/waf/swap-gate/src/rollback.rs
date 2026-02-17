use crate::error::SwapError;
use crate::types::Snapshot;
use maple_waf_context_graph::ContentHash;
use std::sync::{Arc, RwLock};

/// Manages snapshots for rollback capability.
///
/// Invariant I.WAF-4: System can always revert to last stable state.
pub struct RollbackManager {
    /// Stack of snapshots (most recent first).
    snapshots: Arc<RwLock<Vec<Snapshot>>>,
    /// Maximum number of snapshots to retain.
    max_snapshots: usize,
}

impl RollbackManager {
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(Vec::new())),
            max_snapshots,
        }
    }

    /// Take a snapshot of the current state.
    pub fn take_snapshot(&self, state: Vec<u8>, description: impl Into<String>) -> ContentHash {
        let snapshot = Snapshot::new(state, description);
        let hash = snapshot.hash.clone();
        let mut snapshots = self.snapshots.write().expect("lock not poisoned");
        snapshots.push(snapshot);
        if snapshots.len() > self.max_snapshots {
            snapshots.remove(0);
        }
        hash
    }

    /// Get the latest snapshot.
    pub fn latest(&self) -> Option<Snapshot> {
        let snapshots = self.snapshots.read().expect("lock not poisoned");
        snapshots.last().cloned()
    }

    /// Get a snapshot by hash.
    pub fn get(&self, hash: &ContentHash) -> Result<Snapshot, SwapError> {
        let snapshots = self.snapshots.read().expect("lock not poisoned");
        snapshots
            .iter()
            .find(|s| s.hash == *hash)
            .cloned()
            .ok_or_else(|| SwapError::SnapshotNotFound(hash.clone()))
    }

    /// Rollback to the latest snapshot.
    pub fn rollback_to_latest(&self) -> Result<Snapshot, SwapError> {
        self.latest()
            .ok_or_else(|| SwapError::RollbackTriggered("no snapshots available".into()))
    }

    /// Rollback to a specific snapshot.
    pub fn rollback_to(&self, hash: &ContentHash) -> Result<Snapshot, SwapError> {
        self.get(hash)
    }

    /// Number of stored snapshots.
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.read().expect("lock not poisoned").len()
    }

    /// Clear all snapshots.
    pub fn clear(&self) {
        self.snapshots.write().expect("lock not poisoned").clear();
    }
}

impl Default for RollbackManager {
    fn default() -> Self {
        Self::new(10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_and_retrieve_snapshot() {
        let mgr = RollbackManager::new(10);
        let hash = mgr.take_snapshot(vec![1, 2, 3], "first");
        let snap = mgr.get(&hash).unwrap();
        assert_eq!(snap.state, vec![1, 2, 3]);
        assert_eq!(snap.description, "first");
    }

    #[test]
    fn latest_snapshot() {
        let mgr = RollbackManager::new(10);
        mgr.take_snapshot(vec![1], "first");
        mgr.take_snapshot(vec![2], "second");
        let latest = mgr.latest().unwrap();
        assert_eq!(latest.state, vec![2]);
    }

    #[test]
    fn max_snapshots_enforced() {
        let mgr = RollbackManager::new(3);
        mgr.take_snapshot(vec![1], "a");
        mgr.take_snapshot(vec![2], "b");
        mgr.take_snapshot(vec![3], "c");
        mgr.take_snapshot(vec![4], "d");
        assert_eq!(mgr.snapshot_count(), 3);
    }

    #[test]
    fn rollback_to_latest() {
        let mgr = RollbackManager::new(10);
        mgr.take_snapshot(vec![1], "stable");
        let snap = mgr.rollback_to_latest().unwrap();
        assert_eq!(snap.state, vec![1]);
    }

    #[test]
    fn rollback_no_snapshots() {
        let mgr = RollbackManager::new(10);
        assert!(mgr.rollback_to_latest().is_err());
    }

    #[test]
    fn rollback_to_specific() {
        let mgr = RollbackManager::new(10);
        let h1 = mgr.take_snapshot(vec![1], "first");
        mgr.take_snapshot(vec![2], "second");
        let snap = mgr.rollback_to(&h1).unwrap();
        assert_eq!(snap.state, vec![1]);
    }

    #[test]
    fn get_nonexistent() {
        let mgr = RollbackManager::new(10);
        assert!(mgr.get(&ContentHash::hash(b"nope")).is_err());
    }

    #[test]
    fn clear_snapshots() {
        let mgr = RollbackManager::new(10);
        mgr.take_snapshot(vec![1], "a");
        mgr.clear();
        assert_eq!(mgr.snapshot_count(), 0);
    }
}
