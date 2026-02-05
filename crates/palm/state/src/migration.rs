//! Migration service - moves instances between nodes.
//!
//! The migration service handles the process of moving a Resonator
//! from one node to another while maintaining continuity.

use std::sync::Arc;

use tracing::{info, instrument};

use crate::checkpoint::{CheckpointConfig, CheckpointService, InstanceInfo, RuntimeStateGatherer};
use crate::error::{Result, StateError};
use crate::snapshot::{ResonatorId, ResonatorStateSnapshot, SnapshotReason};
use crate::storage::StateStorage;

/// Node identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NodeId(String);

impl NodeId {
    /// Create a new node ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Result of a migration operation.
#[derive(Debug, Clone)]
pub struct MigrationResult {
    /// The old instance ID.
    pub old_instance_id: palm_types::InstanceId,

    /// The new instance ID on the target node.
    pub new_instance_id: palm_types::InstanceId,

    /// The source node.
    pub from_node: NodeId,

    /// The target node.
    pub to_node: NodeId,

    /// The snapshot used for migration.
    pub snapshot_id: crate::error::StateSnapshotId,

    /// New incarnation number after migration.
    pub new_incarnation: u64,
}

/// Trait for migration runtime operations.
#[async_trait::async_trait]
pub trait MigrationRuntime: Send + Sync {
    /// Get the current node ID.
    fn node_id(&self) -> NodeId;

    /// Drain couplings from a Resonator.
    async fn drain_couplings(&self, resonator_id: &ResonatorId, timeout_secs: u64) -> Result<()>;

    /// Wait for pending commitments to complete or timeout.
    async fn await_commitments(&self, resonator_id: &ResonatorId, timeout_secs: u64) -> Result<()>;

    /// Terminate a Resonator on this node.
    async fn terminate_resonator(&self, resonator_id: &ResonatorId) -> Result<()>;

    /// Request a remote node to create an instance from a snapshot.
    async fn request_remote_create(
        &self,
        to_node: &NodeId,
        snapshot: &ResonatorStateSnapshot,
    ) -> Result<palm_types::InstanceId>;
}

/// Migration service for moving instances between nodes.
pub struct MigrationService {
    /// Runtime for migration operations.
    runtime: Arc<dyn MigrationRuntime>,

    /// Runtime for gathering state.
    state_gatherer: Arc<dyn RuntimeStateGatherer>,

    /// State storage.
    storage: Arc<dyn StateStorage>,
}

impl MigrationService {
    /// Create a new migration service.
    pub fn new(
        runtime: Arc<dyn MigrationRuntime>,
        state_gatherer: Arc<dyn RuntimeStateGatherer>,
        storage: Arc<dyn StateStorage>,
    ) -> Self {
        Self {
            runtime,
            state_gatherer,
            storage,
        }
    }

    /// Migrate an instance to a different node.
    #[instrument(skip(self), fields(instance_id = %instance.instance_id, to_node = %to_node))]
    pub async fn migrate(
        &self,
        instance: &InstanceInfo,
        to_node: &NodeId,
    ) -> Result<MigrationResult> {
        let from_node = self.runtime.node_id();

        info!(
            instance_id = %instance.instance_id,
            from_node = %from_node,
            to_node = %to_node,
            "Starting migration"
        );

        // 1. Create pre-migration checkpoint
        let checkpoint_service = CheckpointService::new(
            self.storage.clone(),
            self.state_gatherer.clone(),
            CheckpointConfig::default(),
        );

        let snapshot = checkpoint_service
            .create_checkpoint(instance, SnapshotReason::PreMigration)
            .await?;

        let snapshot_id = snapshot.id.clone();
        self.storage.store(&snapshot).await?;

        info!(
            instance_id = %instance.instance_id,
            snapshot_id = %snapshot_id,
            "Pre-migration checkpoint created"
        );

        // 2. Drain couplings on source
        self.runtime
            .drain_couplings(&instance.resonator_id, 30)
            .await
            .map_err(|e| StateError::MigrationFailed(format!("coupling drain failed: {}", e)))?;

        // 3. Wait for pending commitments
        self.runtime
            .await_commitments(&instance.resonator_id, 60)
            .await
            .map_err(|e| StateError::MigrationFailed(format!("commitment wait failed: {}", e)))?;

        // 4. Request target node to create instance
        let new_instance_id = self
            .runtime
            .request_remote_create(to_node, &snapshot)
            .await
            .map_err(|e| StateError::MigrationFailed(format!("remote create failed: {}", e)))?;

        info!(
            old_instance = %instance.instance_id,
            new_instance = %new_instance_id,
            to_node = %to_node,
            "Remote instance created"
        );

        // 5. Terminate on source
        self.runtime
            .terminate_resonator(&instance.resonator_id)
            .await
            .map_err(|e| StateError::MigrationFailed(format!("termination failed: {}", e)))?;

        info!(
            old_instance = %instance.instance_id,
            new_instance = %new_instance_id,
            to_node = %to_node,
            "Migration completed"
        );

        Ok(MigrationResult {
            old_instance_id: instance.instance_id.clone(),
            new_instance_id,
            from_node,
            to_node: to_node.clone(),
            snapshot_id,
            new_incarnation: snapshot.identity_state.incarnation + 1,
        })
    }
}

/// Mock migration runtime for testing.
pub struct MockMigrationRuntime {
    /// Current node ID.
    node_id: NodeId,

    /// Whether to simulate errors.
    simulate_error: bool,
}

impl MockMigrationRuntime {
    /// Create a new mock runtime.
    pub fn new(node_id: impl Into<String>) -> Self {
        Self {
            node_id: NodeId::new(node_id),
            simulate_error: false,
        }
    }

    /// Create a mock runtime that simulates errors.
    pub fn with_errors(node_id: impl Into<String>) -> Self {
        Self {
            node_id: NodeId::new(node_id),
            simulate_error: true,
        }
    }
}

#[async_trait::async_trait]
impl MigrationRuntime for MockMigrationRuntime {
    fn node_id(&self) -> NodeId {
        self.node_id.clone()
    }

    async fn drain_couplings(&self, _resonator_id: &ResonatorId, _timeout_secs: u64) -> Result<()> {
        if self.simulate_error {
            return Err(StateError::MigrationFailed("simulated error".to_string()));
        }
        Ok(())
    }

    async fn await_commitments(
        &self,
        _resonator_id: &ResonatorId,
        _timeout_secs: u64,
    ) -> Result<()> {
        if self.simulate_error {
            return Err(StateError::MigrationFailed("simulated error".to_string()));
        }
        Ok(())
    }

    async fn terminate_resonator(&self, _resonator_id: &ResonatorId) -> Result<()> {
        if self.simulate_error {
            return Err(StateError::MigrationFailed("simulated error".to_string()));
        }
        Ok(())
    }

    async fn request_remote_create(
        &self,
        _to_node: &NodeId,
        _snapshot: &ResonatorStateSnapshot,
    ) -> Result<palm_types::InstanceId> {
        if self.simulate_error {
            return Err(StateError::MigrationFailed("simulated error".to_string()));
        }
        Ok(palm_types::InstanceId::generate())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkpoint::MockRuntimeStateGatherer;
    use crate::storage::InMemoryStateStorage;

    #[tokio::test]
    async fn test_migration() {
        let storage = Arc::new(InMemoryStateStorage::new());
        let runtime = Arc::new(MockMigrationRuntime::new("node-1"));
        let state_gatherer = Arc::new(MockRuntimeStateGatherer::new());

        let service = MigrationService::new(runtime, state_gatherer, storage.clone());

        let instance = InstanceInfo {
            instance_id: palm_types::InstanceId::generate(),
            deployment_id: palm_types::DeploymentId::generate(),
            resonator_id: ResonatorId::generate(),
        };

        let result = service
            .migrate(&instance, &NodeId::new("node-2"))
            .await
            .unwrap();

        assert_eq!(result.old_instance_id, instance.instance_id);
        assert_ne!(result.new_instance_id, instance.instance_id);
        assert_eq!(result.from_node.as_str(), "node-1");
        assert_eq!(result.to_node.as_str(), "node-2");

        // Verify snapshot was stored
        assert!(storage.exists(&result.snapshot_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_migration_error() {
        let storage = Arc::new(InMemoryStateStorage::new());
        let runtime = Arc::new(MockMigrationRuntime::with_errors("node-1"));
        let state_gatherer = Arc::new(MockRuntimeStateGatherer::new());

        let service = MigrationService::new(runtime, state_gatherer, storage);

        let instance = InstanceInfo {
            instance_id: palm_types::InstanceId::generate(),
            deployment_id: palm_types::DeploymentId::generate(),
            resonator_id: ResonatorId::generate(),
        };

        let result = service.migrate(&instance, &NodeId::new("node-2")).await;
        assert!(result.is_err());
    }
}
