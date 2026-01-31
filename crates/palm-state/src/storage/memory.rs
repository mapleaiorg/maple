//! In-memory state storage for development and testing.
//!
//! Provides a simple in-memory implementation of the StateStorage trait.
//! Not suitable for production use.

use async_trait::async_trait;
use dashmap::DashMap;
use palm_types::InstanceId;
use std::sync::Arc;

use super::traits::StateStorage;
use crate::error::{Result, StateSnapshotId};
use crate::snapshot::{ResonatorStateSnapshot, SnapshotMetadata};

/// In-memory state storage implementation.
pub struct InMemoryStateStorage {
    /// All snapshots indexed by ID.
    snapshots: Arc<DashMap<StateSnapshotId, ResonatorStateSnapshot>>,

    /// Snapshot IDs indexed by instance ID.
    by_instance: Arc<DashMap<InstanceId, Vec<StateSnapshotId>>>,
}

impl InMemoryStateStorage {
    /// Create a new in-memory storage.
    pub fn new() -> Self {
        Self {
            snapshots: Arc::new(DashMap::new()),
            by_instance: Arc::new(DashMap::new()),
        }
    }

    /// Get the total number of snapshots stored.
    pub fn total_count(&self) -> usize {
        self.snapshots.len()
    }

    /// Clear all snapshots.
    pub fn clear(&self) {
        self.snapshots.clear();
        self.by_instance.clear();
    }
}

impl Default for InMemoryStateStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StateStorage for InMemoryStateStorage {
    async fn store(&self, snapshot: &ResonatorStateSnapshot) -> Result<()> {
        // Store the snapshot
        self.snapshots.insert(snapshot.id.clone(), snapshot.clone());

        // Add to instance index
        self.by_instance
            .entry(snapshot.metadata.instance_id.clone())
            .or_default()
            .push(snapshot.id.clone());

        Ok(())
    }

    async fn load(&self, id: &StateSnapshotId) -> Result<Option<ResonatorStateSnapshot>> {
        Ok(self.snapshots.get(id).map(|s| s.clone()))
    }

    async fn get_latest(&self, instance_id: &InstanceId) -> Result<Option<ResonatorStateSnapshot>> {
        let snapshot_ids = self.by_instance.get(instance_id);

        if let Some(ids) = snapshot_ids {
            // Find the most recent by created_at
            let mut latest: Option<ResonatorStateSnapshot> = None;
            let mut latest_time = chrono::DateTime::<chrono::Utc>::MIN_UTC;

            for id in ids.iter() {
                if let Some(snapshot) = self.snapshots.get(id) {
                    if snapshot.metadata.created_at > latest_time {
                        latest_time = snapshot.metadata.created_at;
                        latest = Some(snapshot.clone());
                    }
                }
            }

            Ok(latest)
        } else {
            Ok(None)
        }
    }

    async fn list(&self, instance_id: &InstanceId) -> Result<Vec<SnapshotMetadata>> {
        let snapshot_ids = self.by_instance.get(instance_id);

        if let Some(ids) = snapshot_ids {
            let mut metadata: Vec<SnapshotMetadata> = Vec::new();

            for id in ids.iter() {
                if let Some(snapshot) = self.snapshots.get(id) {
                    metadata.push(snapshot.metadata.clone());
                }
            }

            // Sort by created_at descending (newest first)
            metadata.sort_by(|a, b| b.created_at.cmp(&a.created_at));

            Ok(metadata)
        } else {
            Ok(Vec::new())
        }
    }

    async fn delete(&self, id: &StateSnapshotId) -> Result<()> {
        if let Some((_, snapshot)) = self.snapshots.remove(id) {
            if let Some(mut ids) = self.by_instance.get_mut(&snapshot.metadata.instance_id) {
                ids.retain(|i| i != id);
            }
        }
        Ok(())
    }

    async fn cleanup_old_snapshots(
        &self,
        instance_id: &InstanceId,
        keep_count: usize,
    ) -> Result<usize> {
        let metadata = self.list(instance_id).await?;

        if metadata.len() <= keep_count {
            return Ok(0);
        }

        // Collect snapshot IDs to delete (already sorted by created_at descending)
        let to_delete: Vec<StateSnapshotId> = metadata
            .iter()
            .skip(keep_count)
            .filter_map(|m| {
                // Find snapshot ID from metadata
                self.snapshots
                    .iter()
                    .find(|s| {
                        s.metadata.instance_id == *instance_id
                            && s.metadata.created_at == m.created_at
                    })
                    .map(|s| s.id.clone())
            })
            .collect();

        let count = to_delete.len();

        for id in to_delete {
            self.delete(&id).await?;
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::*;
    use chrono::Utc;

    fn create_test_snapshot(instance_id: &InstanceId) -> ResonatorStateSnapshot {
        let resonator_id = ResonatorId::generate();
        let deployment_id = palm_types::DeploymentId::generate();

        ResonatorStateSnapshot {
            id: StateSnapshotId::generate(),
            metadata: SnapshotMetadata {
                instance_id: instance_id.clone(),
                deployment_id,
                resonator_id: resonator_id.clone(),
                created_at: Utc::now(),
                incarnation: 1,
                reason: SnapshotReason::Manual,
                compressed: false,
                encrypted: false,
            },
            identity_state: IdentityStateSnapshot {
                resonator_id,
                continuity_chain: ContinuityChainSnapshot { links: vec![] },
                incarnation: 1,
                key_reference: "test".to_string(),
            },
            presence_state: PresenceStateSnapshot {
                discoverability: 1.0,
                responsiveness: 1.0,
                stability: 1.0,
                coupling_readiness: 1.0,
                last_signal: Utc::now(),
            },
            coupling_state: vec![],
            meaning_context: MeaningContextSnapshot {
                interpretations: vec![],
                context_factors: vec![],
                confidence_distribution: vec![],
            },
            intent_state: None,
            pending_commitments: vec![],
            attention_state: AttentionStateSnapshot {
                total: 100,
                available: 80,
                allocated: 20,
                reserved: 0,
                allocations: vec![],
            },
            application_state: None,
            integrity_hash: String::new(),
        }
        .finalize()
    }

    #[tokio::test]
    async fn test_store_and_load() {
        let storage = InMemoryStateStorage::new();
        let instance_id = InstanceId::generate();
        let snapshot = create_test_snapshot(&instance_id);
        let snapshot_id = snapshot.id.clone();

        storage.store(&snapshot).await.unwrap();

        let loaded = storage.load(&snapshot_id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().id, snapshot_id);
    }

    #[tokio::test]
    async fn test_get_latest() {
        let storage = InMemoryStateStorage::new();
        let instance_id = InstanceId::generate();

        // Store multiple snapshots
        for _ in 0..3 {
            let snapshot = create_test_snapshot(&instance_id);
            storage.store(&snapshot).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        let latest = storage.get_latest(&instance_id).await.unwrap();
        assert!(latest.is_some());

        // Verify it's the most recent
        let all = storage.list(&instance_id).await.unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(latest.unwrap().metadata.created_at, all[0].created_at);
    }

    #[tokio::test]
    async fn test_delete() {
        let storage = InMemoryStateStorage::new();
        let instance_id = InstanceId::generate();
        let snapshot = create_test_snapshot(&instance_id);
        let snapshot_id = snapshot.id.clone();

        storage.store(&snapshot).await.unwrap();
        assert!(storage.exists(&snapshot_id).await.unwrap());

        storage.delete(&snapshot_id).await.unwrap();
        assert!(!storage.exists(&snapshot_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_cleanup() {
        let storage = InMemoryStateStorage::new();
        let instance_id = InstanceId::generate();

        // Store 5 snapshots
        for _ in 0..5 {
            let snapshot = create_test_snapshot(&instance_id);
            storage.store(&snapshot).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        assert_eq!(storage.count(&instance_id).await.unwrap(), 5);

        // Keep only 2
        let deleted = storage
            .cleanup_old_snapshots(&instance_id, 2)
            .await
            .unwrap();
        assert_eq!(deleted, 3);

        let remaining = storage.list(&instance_id).await.unwrap();
        assert_eq!(remaining.len(), 2);
    }

    #[tokio::test]
    async fn test_multiple_instances() {
        let storage = InMemoryStateStorage::new();

        let instance1 = InstanceId::generate();
        let instance2 = InstanceId::generate();

        // Store snapshots for both instances
        storage
            .store(&create_test_snapshot(&instance1))
            .await
            .unwrap();
        storage
            .store(&create_test_snapshot(&instance1))
            .await
            .unwrap();
        storage
            .store(&create_test_snapshot(&instance2))
            .await
            .unwrap();

        assert_eq!(storage.count(&instance1).await.unwrap(), 2);
        assert_eq!(storage.count(&instance2).await.unwrap(), 1);
        assert_eq!(storage.total_count(), 3);
    }
}
