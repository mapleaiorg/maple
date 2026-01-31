//! Checkpoint service - creates state snapshots.
//!
//! The checkpoint service gathers state from the runtime and creates
//! complete snapshots for persistence.

use std::sync::Arc;

use chrono::Utc;
use tracing::{debug, instrument};

use crate::error::{Result, StateError, StateSnapshotId};
use crate::snapshot::*;
use crate::storage::StateStorage;

/// Configuration for checkpoint operations.
#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    /// Whether to compress snapshot data.
    pub compression_enabled: bool,

    /// Whether to encrypt snapshot data.
    pub encryption_enabled: bool,

    /// Maximum size for application state (bytes).
    pub max_application_state_size: usize,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            compression_enabled: true,
            encryption_enabled: false,
            max_application_state_size: 10 * 1024 * 1024, // 10 MB
        }
    }
}

/// Service for creating state checkpoints.
pub struct CheckpointService {
    /// State storage backend.
    storage: Arc<dyn StateStorage>,

    /// Runtime interface for gathering state.
    runtime: Arc<dyn RuntimeStateGatherer>,

    /// Configuration.
    config: CheckpointConfig,
}

/// Trait for gathering state from the runtime.
///
/// This abstracts the runtime interface so palm-state doesn't depend
/// on the full maple-runtime crate.
#[async_trait::async_trait]
pub trait RuntimeStateGatherer: Send + Sync {
    /// Get identity state for a Resonator.
    async fn get_identity_state(
        &self,
        resonator_id: &ResonatorId,
    ) -> Result<IdentityStateSnapshot>;

    /// Get presence state for a Resonator.
    async fn get_presence_state(
        &self,
        resonator_id: &ResonatorId,
    ) -> Result<PresenceStateSnapshot>;

    /// Get coupling state for a Resonator.
    async fn get_coupling_state(&self, resonator_id: &ResonatorId) -> Result<Vec<CouplingSnapshot>>;

    /// Get meaning context for a Resonator.
    async fn get_meaning_context(
        &self,
        resonator_id: &ResonatorId,
    ) -> Result<MeaningContextSnapshot>;

    /// Get intent state for a Resonator.
    async fn get_intent_state(&self, resonator_id: &ResonatorId) -> Result<Option<IntentSnapshot>>;

    /// Get pending commitments for a Resonator.
    async fn get_pending_commitments(
        &self,
        resonator_id: &ResonatorId,
    ) -> Result<Vec<CommitmentSnapshot>>;

    /// Get attention state for a Resonator.
    async fn get_attention_state(
        &self,
        resonator_id: &ResonatorId,
    ) -> Result<AttentionStateSnapshot>;

    /// Get application-specific state for a Resonator.
    async fn get_application_state(
        &self,
        resonator_id: &ResonatorId,
    ) -> Result<Option<bytes::Bytes>>;
}

/// Instance information needed for checkpointing.
#[derive(Debug, Clone)]
pub struct InstanceInfo {
    /// Instance ID.
    pub instance_id: palm_types::InstanceId,

    /// Deployment ID.
    pub deployment_id: palm_types::DeploymentId,

    /// Resonator ID.
    pub resonator_id: ResonatorId,
}

impl CheckpointService {
    /// Create a new checkpoint service.
    pub fn new(
        storage: Arc<dyn StateStorage>,
        runtime: Arc<dyn RuntimeStateGatherer>,
        config: CheckpointConfig,
    ) -> Self {
        Self {
            storage,
            runtime,
            config,
        }
    }

    /// Create a checkpoint snapshot from current instance state.
    #[instrument(skip(self, instance), fields(instance_id = %instance.instance_id))]
    pub async fn create_checkpoint(
        &self,
        instance: &InstanceInfo,
        reason: SnapshotReason,
    ) -> Result<ResonatorStateSnapshot> {
        debug!(instance_id = %instance.instance_id, "Creating checkpoint");

        // Gather all state from runtime
        let identity_state = self
            .runtime
            .get_identity_state(&instance.resonator_id)
            .await?;
        let presence_state = self
            .runtime
            .get_presence_state(&instance.resonator_id)
            .await?;
        let coupling_state = self
            .runtime
            .get_coupling_state(&instance.resonator_id)
            .await?;
        let meaning_context = self
            .runtime
            .get_meaning_context(&instance.resonator_id)
            .await?;
        let intent_state = self
            .runtime
            .get_intent_state(&instance.resonator_id)
            .await?;
        let pending_commitments = self
            .runtime
            .get_pending_commitments(&instance.resonator_id)
            .await?;
        let attention_state = self
            .runtime
            .get_attention_state(&instance.resonator_id)
            .await?;
        let application_state = self
            .runtime
            .get_application_state(&instance.resonator_id)
            .await?;

        // Validate application state size
        if let Some(ref app_state) = application_state {
            if app_state.len() > self.config.max_application_state_size {
                return Err(StateError::CheckpointFailed {
                    instance_id: instance.instance_id.clone(),
                    reason: format!(
                        "Application state too large: {} bytes (max: {} bytes)",
                        app_state.len(),
                        self.config.max_application_state_size
                    ),
                });
            }
        }

        let metadata = SnapshotMetadata {
            instance_id: instance.instance_id.clone(),
            deployment_id: instance.deployment_id.clone(),
            resonator_id: instance.resonator_id.clone(),
            created_at: Utc::now(),
            incarnation: identity_state.incarnation,
            reason,
            compressed: self.config.compression_enabled,
            encrypted: self.config.encryption_enabled,
        };

        let snapshot = ResonatorStateSnapshot {
            id: StateSnapshotId::generate(),
            metadata,
            identity_state,
            presence_state,
            coupling_state,
            meaning_context,
            intent_state,
            pending_commitments,
            attention_state,
            application_state,
            integrity_hash: String::new(),
        }
        .finalize();

        debug!(
            instance_id = %instance.instance_id,
            snapshot_id = %snapshot.id,
            "Checkpoint created"
        );

        Ok(snapshot)
    }

    /// Create and store a checkpoint.
    #[instrument(skip(self, instance), fields(instance_id = %instance.instance_id))]
    pub async fn checkpoint_and_store(
        &self,
        instance: &InstanceInfo,
        reason: SnapshotReason,
    ) -> Result<StateSnapshotId> {
        let snapshot = self.create_checkpoint(instance, reason).await?;
        let snapshot_id = snapshot.id.clone();

        self.storage.store(&snapshot).await?;

        Ok(snapshot_id)
    }
}

/// Mock runtime state gatherer for testing.
pub struct MockRuntimeStateGatherer {
    /// Identity state to return.
    pub identity_state: Option<IdentityStateSnapshot>,

    /// Presence state to return.
    pub presence_state: Option<PresenceStateSnapshot>,

    /// Whether to simulate errors.
    pub simulate_error: bool,
}

impl MockRuntimeStateGatherer {
    /// Create a new mock gatherer with default healthy state.
    pub fn new() -> Self {
        let resonator_id = ResonatorId::generate();

        Self {
            identity_state: Some(IdentityStateSnapshot {
                resonator_id: resonator_id.clone(),
                continuity_chain: ContinuityChainSnapshot { links: vec![] },
                incarnation: 1,
                key_reference: "mock-key-ref".to_string(),
            }),
            presence_state: Some(PresenceStateSnapshot {
                discoverability: 0.9,
                responsiveness: 0.85,
                stability: 0.8,
                coupling_readiness: 0.75,
                last_signal: Utc::now(),
            }),
            simulate_error: false,
        }
    }

    /// Create a mock gatherer that returns errors.
    pub fn with_errors() -> Self {
        Self {
            identity_state: None,
            presence_state: None,
            simulate_error: true,
        }
    }
}

impl Default for MockRuntimeStateGatherer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl RuntimeStateGatherer for MockRuntimeStateGatherer {
    async fn get_identity_state(
        &self,
        _resonator_id: &ResonatorId,
    ) -> Result<IdentityStateSnapshot> {
        if self.simulate_error {
            return Err(StateError::Runtime("simulated error".to_string()));
        }

        self.identity_state
            .clone()
            .ok_or_else(|| StateError::Runtime("no identity state".to_string()))
    }

    async fn get_presence_state(
        &self,
        _resonator_id: &ResonatorId,
    ) -> Result<PresenceStateSnapshot> {
        if self.simulate_error {
            return Err(StateError::Runtime("simulated error".to_string()));
        }

        self.presence_state
            .clone()
            .ok_or_else(|| StateError::Runtime("no presence state".to_string()))
    }

    async fn get_coupling_state(
        &self,
        _resonator_id: &ResonatorId,
    ) -> Result<Vec<CouplingSnapshot>> {
        if self.simulate_error {
            return Err(StateError::Runtime("simulated error".to_string()));
        }

        Ok(vec![])
    }

    async fn get_meaning_context(
        &self,
        _resonator_id: &ResonatorId,
    ) -> Result<MeaningContextSnapshot> {
        if self.simulate_error {
            return Err(StateError::Runtime("simulated error".to_string()));
        }

        Ok(MeaningContextSnapshot {
            interpretations: vec![],
            context_factors: vec![],
            confidence_distribution: vec![],
        })
    }

    async fn get_intent_state(
        &self,
        _resonator_id: &ResonatorId,
    ) -> Result<Option<IntentSnapshot>> {
        if self.simulate_error {
            return Err(StateError::Runtime("simulated error".to_string()));
        }

        Ok(None)
    }

    async fn get_pending_commitments(
        &self,
        _resonator_id: &ResonatorId,
    ) -> Result<Vec<CommitmentSnapshot>> {
        if self.simulate_error {
            return Err(StateError::Runtime("simulated error".to_string()));
        }

        Ok(vec![])
    }

    async fn get_attention_state(
        &self,
        _resonator_id: &ResonatorId,
    ) -> Result<AttentionStateSnapshot> {
        if self.simulate_error {
            return Err(StateError::Runtime("simulated error".to_string()));
        }

        Ok(AttentionStateSnapshot {
            total: 100,
            available: 80,
            allocated: 20,
            reserved: 0,
            allocations: vec![],
        })
    }

    async fn get_application_state(
        &self,
        _resonator_id: &ResonatorId,
    ) -> Result<Option<bytes::Bytes>> {
        if self.simulate_error {
            return Err(StateError::Runtime("simulated error".to_string()));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryStateStorage;

    #[tokio::test]
    async fn test_create_checkpoint() {
        let storage = Arc::new(InMemoryStateStorage::new());
        let runtime = Arc::new(MockRuntimeStateGatherer::new());
        let service = CheckpointService::new(storage, runtime, CheckpointConfig::default());

        let instance = InstanceInfo {
            instance_id: palm_types::InstanceId::generate(),
            deployment_id: palm_types::DeploymentId::generate(),
            resonator_id: ResonatorId::generate(),
        };

        let snapshot = service
            .create_checkpoint(&instance, SnapshotReason::Manual)
            .await
            .unwrap();

        assert!(snapshot.verify_integrity());
        assert_eq!(snapshot.metadata.instance_id, instance.instance_id);
        assert_eq!(snapshot.metadata.reason, SnapshotReason::Manual);
    }

    #[tokio::test]
    async fn test_checkpoint_and_store() {
        let storage = Arc::new(InMemoryStateStorage::new());
        let runtime = Arc::new(MockRuntimeStateGatherer::new());
        let service =
            CheckpointService::new(storage.clone(), runtime, CheckpointConfig::default());

        let instance = InstanceInfo {
            instance_id: palm_types::InstanceId::generate(),
            deployment_id: palm_types::DeploymentId::generate(),
            resonator_id: ResonatorId::generate(),
        };

        let snapshot_id = service
            .checkpoint_and_store(&instance, SnapshotReason::Scheduled)
            .await
            .unwrap();

        // Verify it was stored
        let loaded = storage.load(&snapshot_id).await.unwrap();
        assert!(loaded.is_some());
    }

    #[tokio::test]
    async fn test_checkpoint_error() {
        let storage = Arc::new(InMemoryStateStorage::new());
        let runtime = Arc::new(MockRuntimeStateGatherer::with_errors());
        let service = CheckpointService::new(storage, runtime, CheckpointConfig::default());

        let instance = InstanceInfo {
            instance_id: palm_types::InstanceId::generate(),
            deployment_id: palm_types::DeploymentId::generate(),
            resonator_id: ResonatorId::generate(),
        };

        let result = service
            .create_checkpoint(&instance, SnapshotReason::Manual)
            .await;
        assert!(result.is_err());
    }
}
