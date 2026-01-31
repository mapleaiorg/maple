//! State Manager - Main facade for state operations.
//!
//! The StateManager orchestrates all state management operations including
//! checkpoints, restores, migrations, and scheduled maintenance.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;
use tracing::{info, instrument, warn};

use crate::checkpoint::{
    CheckpointConfig, CheckpointService, InstanceInfo, RuntimeStateGatherer,
};
use crate::commitment_reconcile::{AasClient, CommitmentReconciler, ReconciliationResult};
use crate::coupling_restore::{CouplingRestorationHandle, CouplingRestorationManager, CouplingRuntime};
use crate::error::{Result, StateError, StateSnapshotId};
use crate::migration::{MigrationResult, MigrationRuntime, MigrationService, NodeId};
use crate::restore::{ContinuityVerifier, RestoreResult, RestoreService, RuntimeStateRestorer};
use crate::snapshot::{ResonatorStateSnapshot, SnapshotMetadata, SnapshotReason};
use crate::storage::StateStorage;

/// Configuration for state management.
#[derive(Debug, Clone)]
pub struct StateManagerConfig {
    /// Interval between scheduled checkpoints.
    pub checkpoint_interval: Duration,

    /// Maximum snapshots to keep per instance.
    pub max_snapshots_per_instance: usize,

    /// Whether to enable compression.
    pub compression_enabled: bool,

    /// Whether to enable encryption.
    pub encryption_enabled: bool,

    /// Batch size for coupling restoration.
    pub coupling_restore_batch_size: usize,

    /// Delay between coupling restoration batches.
    pub coupling_restore_delay: Duration,
}

impl Default for StateManagerConfig {
    fn default() -> Self {
        Self {
            checkpoint_interval: Duration::from_secs(300), // 5 minutes
            max_snapshots_per_instance: 5,
            compression_enabled: true,
            encryption_enabled: false,
            coupling_restore_batch_size: 10,
            coupling_restore_delay: Duration::from_millis(100),
        }
    }
}

/// Events emitted by the state manager.
#[derive(Debug, Clone)]
pub enum StateEvent {
    /// Checkpoint was created.
    CheckpointCreated {
        instance_id: palm_types::InstanceId,
        snapshot_id: StateSnapshotId,
    },

    /// Checkpoint failed.
    CheckpointFailed {
        instance_id: palm_types::InstanceId,
        reason: String,
    },

    /// Restore started.
    RestoreStarted {
        instance_id: palm_types::InstanceId,
        snapshot_id: StateSnapshotId,
    },

    /// Restore completed.
    RestoreCompleted {
        instance_id: palm_types::InstanceId,
        snapshot_id: StateSnapshotId,
        new_incarnation: u64,
    },

    /// Restore failed.
    RestoreFailed {
        instance_id: palm_types::InstanceId,
        reason: String,
    },

    /// Migration started.
    MigrationStarted {
        instance_id: palm_types::InstanceId,
        to_node: NodeId,
    },

    /// Migration completed.
    MigrationCompleted {
        old_instance_id: palm_types::InstanceId,
        new_instance_id: palm_types::InstanceId,
        from_node: NodeId,
        to_node: NodeId,
    },

    /// Migration failed.
    MigrationFailed {
        instance_id: palm_types::InstanceId,
        reason: String,
    },

    /// Coupling restoration started.
    CouplingRestorationStarted {
        instance_id: palm_types::InstanceId,
        coupling_count: usize,
    },

    /// Coupling restoration completed.
    CouplingRestorationCompleted {
        instance_id: palm_types::InstanceId,
        restored: usize,
        failed: usize,
        skipped: usize,
    },

    /// Commitment reconciliation completed.
    CommitmentReconciliationCompleted {
        instance_id: palm_types::InstanceId,
        pending: usize,
        executed: usize,
        failed: usize,
    },
}

/// State Manager orchestrates all state operations.
pub struct StateManager {
    /// Configuration.
    config: StateManagerConfig,

    /// State storage.
    storage: Arc<dyn StateStorage>,

    /// Checkpoint service.
    checkpoint_service: CheckpointService,

    /// Restore service.
    restore_service: RestoreService,

    /// Migration service.
    migration_service: MigrationService,

    /// Coupling restoration manager.
    coupling_manager: CouplingRestorationManager,

    /// Commitment reconciler.
    commitment_reconciler: CommitmentReconciler,

    /// Event broadcaster.
    event_tx: broadcast::Sender<StateEvent>,
}

impl StateManager {
    /// Create a new state manager.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: StateManagerConfig,
        storage: Arc<dyn StateStorage>,
        state_gatherer: Arc<dyn RuntimeStateGatherer>,
        state_restorer: Arc<dyn RuntimeStateRestorer>,
        continuity_verifier: Arc<dyn ContinuityVerifier>,
        coupling_runtime: Arc<dyn CouplingRuntime>,
        migration_runtime: Arc<dyn MigrationRuntime>,
        aas_client: Arc<dyn AasClient>,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(1024);

        let checkpoint_config = CheckpointConfig {
            compression_enabled: config.compression_enabled,
            encryption_enabled: config.encryption_enabled,
            max_application_state_size: 10 * 1024 * 1024,
        };

        let checkpoint_service = CheckpointService::new(
            storage.clone(),
            state_gatherer.clone(),
            checkpoint_config,
        );

        let restore_service = RestoreService::new(
            storage.clone(),
            state_restorer,
            continuity_verifier,
        );

        let coupling_manager = CouplingRestorationManager::new(
            coupling_runtime,
            config.coupling_restore_batch_size,
            config.coupling_restore_delay,
        );

        let commitment_reconciler = CommitmentReconciler::new(aas_client);

        let migration_service = MigrationService::new(
            migration_runtime,
            state_gatherer,
            storage.clone(),
        );

        Self {
            config,
            storage,
            checkpoint_service,
            restore_service,
            migration_service,
            coupling_manager,
            commitment_reconciler,
            event_tx,
        }
    }

    /// Subscribe to state events.
    pub fn subscribe(&self) -> broadcast::Receiver<StateEvent> {
        self.event_tx.subscribe()
    }

    /// Create a checkpoint of instance state.
    #[instrument(skip(self, instance), fields(instance_id = %instance.instance_id))]
    pub async fn checkpoint(
        &self,
        instance: &InstanceInfo,
        reason: SnapshotReason,
    ) -> Result<StateSnapshotId> {
        let snapshot_id = match self
            .checkpoint_service
            .checkpoint_and_store(instance, reason)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                self.emit_event(StateEvent::CheckpointFailed {
                    instance_id: instance.instance_id.clone(),
                    reason: e.to_string(),
                });
                return Err(e);
            }
        };

        // Cleanup old snapshots
        if let Err(e) = self
            .storage
            .cleanup_old_snapshots(&instance.instance_id, self.config.max_snapshots_per_instance)
            .await
        {
            warn!(
                instance_id = %instance.instance_id,
                error = %e,
                "Failed to cleanup old snapshots"
            );
        }

        self.emit_event(StateEvent::CheckpointCreated {
            instance_id: instance.instance_id.clone(),
            snapshot_id: snapshot_id.clone(),
        });

        info!(
            instance_id = %instance.instance_id,
            snapshot_id = %snapshot_id,
            "Checkpoint created"
        );

        Ok(snapshot_id)
    }

    /// Restore instance state from a snapshot.
    #[instrument(skip(self), fields(instance_id = %instance_id, snapshot_id = %snapshot_id))]
    pub async fn restore(
        &self,
        instance_id: &palm_types::InstanceId,
        snapshot_id: &StateSnapshotId,
    ) -> Result<RestoreResult> {
        self.emit_event(StateEvent::RestoreStarted {
            instance_id: instance_id.clone(),
            snapshot_id: snapshot_id.clone(),
        });

        // Load snapshot for coupling and commitment info
        let snapshot = self
            .storage
            .load(snapshot_id)
            .await?
            .ok_or_else(|| StateError::SnapshotNotFound(snapshot_id.clone()))?;

        // Perform restore
        let result = match self.restore_service.restore(instance_id, snapshot_id).await {
            Ok(r) => r,
            Err(e) => {
                self.emit_event(StateEvent::RestoreFailed {
                    instance_id: instance_id.clone(),
                    reason: e.to_string(),
                });
                return Err(e);
            }
        };

        self.emit_event(StateEvent::RestoreCompleted {
            instance_id: instance_id.clone(),
            snapshot_id: snapshot_id.clone(),
            new_incarnation: result.new_incarnation,
        });

        // Reconcile commitments
        let reconciliation = self
            .commitment_reconciler
            .reconcile(&snapshot.pending_commitments)
            .await?;

        self.emit_event(StateEvent::CommitmentReconciliationCompleted {
            instance_id: instance_id.clone(),
            pending: reconciliation.pending.len(),
            executed: reconciliation.executed.len(),
            failed: reconciliation.failed.len(),
        });

        // Schedule coupling restoration
        if !snapshot.coupling_state.is_empty() {
            self.emit_event(StateEvent::CouplingRestorationStarted {
                instance_id: instance_id.clone(),
                coupling_count: snapshot.coupling_state.len(),
            });

            let handle = self.coupling_manager.schedule_restoration(
                instance_id.clone(),
                snapshot.coupling_state,
            );

            // Spawn a task to wait for completion and emit event
            let event_tx = self.event_tx.clone();
            let instance_id_clone = instance_id.clone();
            tokio::spawn(async move {
                let result = handle.wait().await;
                let _ = event_tx.send(StateEvent::CouplingRestorationCompleted {
                    instance_id: instance_id_clone,
                    restored: result.restored,
                    failed: result.failed,
                    skipped: result.skipped,
                });
            });
        }

        info!(
            instance_id = %instance_id,
            snapshot_id = %snapshot_id,
            new_incarnation = result.new_incarnation,
            "State restored"
        );

        Ok(result)
    }

    /// Migrate instance to another node.
    #[instrument(skip(self, instance), fields(instance_id = %instance.instance_id, to_node = %to_node))]
    pub async fn migrate(
        &self,
        instance: &InstanceInfo,
        to_node: &NodeId,
    ) -> Result<MigrationResult> {
        self.emit_event(StateEvent::MigrationStarted {
            instance_id: instance.instance_id.clone(),
            to_node: to_node.clone(),
        });

        let result = match self.migration_service.migrate(instance, to_node).await {
            Ok(r) => r,
            Err(e) => {
                self.emit_event(StateEvent::MigrationFailed {
                    instance_id: instance.instance_id.clone(),
                    reason: e.to_string(),
                });
                return Err(e);
            }
        };

        self.emit_event(StateEvent::MigrationCompleted {
            old_instance_id: result.old_instance_id.clone(),
            new_instance_id: result.new_instance_id.clone(),
            from_node: result.from_node.clone(),
            to_node: result.to_node.clone(),
        });

        Ok(result)
    }

    /// Get latest snapshot for an instance.
    pub async fn get_latest_snapshot(
        &self,
        instance_id: &palm_types::InstanceId,
    ) -> Result<Option<ResonatorStateSnapshot>> {
        self.storage.get_latest(instance_id).await
    }

    /// List snapshots for an instance.
    pub async fn list_snapshots(
        &self,
        instance_id: &palm_types::InstanceId,
    ) -> Result<Vec<SnapshotMetadata>> {
        self.storage.list(instance_id).await
    }

    /// Delete a snapshot.
    pub async fn delete_snapshot(&self, snapshot_id: &StateSnapshotId) -> Result<()> {
        self.storage.delete(snapshot_id).await
    }

    /// Run scheduled checkpoints for a list of instances.
    pub async fn run_scheduled_checkpoints(
        &self,
        instances: Vec<InstanceInfo>,
    ) -> Vec<(palm_types::InstanceId, Result<StateSnapshotId>)> {
        let mut results = Vec::with_capacity(instances.len());

        for instance in instances {
            let result = self
                .checkpoint(&instance, SnapshotReason::Scheduled)
                .await;
            results.push((instance.instance_id, result));
        }

        results
    }

    fn emit_event(&self, event: StateEvent) {
        let _ = self.event_tx.send(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkpoint::MockRuntimeStateGatherer;
    use crate::commitment_reconcile::MockAasClient;
    use crate::coupling_restore::MockCouplingRuntime;
    use crate::migration::MockMigrationRuntime;
    use crate::restore::{MockContinuityVerifier, MockRuntimeStateRestorer};
    use crate::snapshot::ResonatorId;
    use crate::storage::InMemoryStateStorage;

    fn create_test_manager() -> StateManager {
        let storage = Arc::new(InMemoryStateStorage::new());
        let state_gatherer = Arc::new(MockRuntimeStateGatherer::new());
        let state_restorer = Arc::new(MockRuntimeStateRestorer::new());
        let continuity_verifier = Arc::new(MockContinuityVerifier::new());
        let coupling_runtime = Arc::new(MockCouplingRuntime::all_present());
        let migration_runtime = Arc::new(MockMigrationRuntime::new("node-1"));
        let aas_client = Arc::new(MockAasClient::all_pending());

        StateManager::new(
            StateManagerConfig::default(),
            storage,
            state_gatherer,
            state_restorer,
            continuity_verifier,
            coupling_runtime,
            migration_runtime,
            aas_client,
        )
    }

    #[tokio::test]
    async fn test_checkpoint() {
        let manager = create_test_manager();

        let instance = InstanceInfo {
            instance_id: palm_types::InstanceId::generate(),
            deployment_id: palm_types::DeploymentId::generate(),
            resonator_id: ResonatorId::generate(),
        };

        let snapshot_id = manager
            .checkpoint(&instance, SnapshotReason::Manual)
            .await
            .unwrap();

        // Verify snapshot exists
        let loaded = manager.get_latest_snapshot(&instance.instance_id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().id, snapshot_id);
    }

    #[tokio::test]
    async fn test_restore() {
        let manager = create_test_manager();

        let instance = InstanceInfo {
            instance_id: palm_types::InstanceId::generate(),
            deployment_id: palm_types::DeploymentId::generate(),
            resonator_id: ResonatorId::generate(),
        };

        // Create a checkpoint first
        let snapshot_id = manager
            .checkpoint(&instance, SnapshotReason::Manual)
            .await
            .unwrap();

        // Restore from it
        let result = manager
            .restore(&instance.instance_id, &snapshot_id)
            .await
            .unwrap();

        assert_eq!(result.instance_id, instance.instance_id);
        assert_eq!(result.snapshot_id, snapshot_id);
    }

    #[tokio::test]
    async fn test_list_snapshots() {
        let manager = create_test_manager();

        let instance = InstanceInfo {
            instance_id: palm_types::InstanceId::generate(),
            deployment_id: palm_types::DeploymentId::generate(),
            resonator_id: ResonatorId::generate(),
        };

        // Create multiple checkpoints
        for _ in 0..3 {
            manager
                .checkpoint(&instance, SnapshotReason::Scheduled)
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let snapshots = manager.list_snapshots(&instance.instance_id).await.unwrap();
        assert_eq!(snapshots.len(), 3);
    }
}
