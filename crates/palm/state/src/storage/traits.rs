//! Storage trait definition.
//!
//! Defines the interface for state snapshot storage backends.

use async_trait::async_trait;
use palm_types::InstanceId;

use crate::error::{Result, StateSnapshotId};
use crate::snapshot::{ResonatorStateSnapshot, SnapshotMetadata};

/// Trait for state snapshot storage backends.
#[async_trait]
pub trait StateStorage: Send + Sync {
    /// Store a snapshot.
    async fn store(&self, snapshot: &ResonatorStateSnapshot) -> Result<()>;

    /// Load a snapshot by ID.
    async fn load(&self, id: &StateSnapshotId) -> Result<Option<ResonatorStateSnapshot>>;

    /// Get the latest snapshot for an instance.
    async fn get_latest(&self, instance_id: &InstanceId) -> Result<Option<ResonatorStateSnapshot>>;

    /// List snapshot metadata for an instance.
    async fn list(&self, instance_id: &InstanceId) -> Result<Vec<SnapshotMetadata>>;

    /// Delete a snapshot.
    async fn delete(&self, id: &StateSnapshotId) -> Result<()>;

    /// Cleanup old snapshots, keeping only the most recent N.
    ///
    /// Returns the number of snapshots deleted.
    async fn cleanup_old_snapshots(
        &self,
        instance_id: &InstanceId,
        keep_count: usize,
    ) -> Result<usize>;

    /// Check if a snapshot exists.
    async fn exists(&self, id: &StateSnapshotId) -> Result<bool> {
        Ok(self.load(id).await?.is_some())
    }

    /// Get the count of snapshots for an instance.
    async fn count(&self, instance_id: &InstanceId) -> Result<usize> {
        Ok(self.list(instance_id).await?.len())
    }
}
