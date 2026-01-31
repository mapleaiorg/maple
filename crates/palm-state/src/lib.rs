//! # PALM State - State Management and Continuity for Resonator Fleets
//!
//! This crate provides state checkpointing, restoration, and migration
//! capabilities for PALM (Persistent Agent Lifecycle Manager).
//!
//! ## Overview
//!
//! PALM State orchestrates state operations at the fleet level:
//!
//! - **Checkpoints**: Capture complete Resonator state for persistence
//! - **Restoration**: Restore Resonator state from snapshots
//! - **Migration**: Move Resonators between nodes with continuity
//! - **Coupling Restoration**: Gradually re-establish couplings after restore
//! - **Commitment Reconciliation**: Reconcile pending commitments with AAS
//!
//! ## Architectural Boundaries
//!
//! PALM State respects the following boundaries:
//!
//! - **resonator-runtime** owns actual Resonator state, presence, coupling, attention
//! - **resonator-identity** owns identity creation, continuity proof creation/verification
//! - **palm-state** owns orchestrating checkpoints, transfers, restores at fleet level
//!
//! State operations CALL INTO runtime and identity crates, not duplicate them.
//!
//! ## Key Components
//!
//! - [`StateManager`]: Main facade for all state operations
//! - [`CheckpointService`]: Creates state snapshots
//! - [`RestoreService`]: Restores state from snapshots
//! - [`MigrationService`]: Handles cross-node migration
//! - [`CouplingRestorationManager`]: Gradual coupling re-establishment
//! - [`CommitmentReconciler`]: Reconciles commitments with AAS
//!
//! ## Example
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use palm_state::{
//!     StateManager, StateManagerConfig,
//!     checkpoint::{InstanceInfo, MockRuntimeStateGatherer},
//!     restore::{MockRuntimeStateRestorer, MockContinuityVerifier},
//!     coupling_restore::MockCouplingRuntime,
//!     migration::MockMigrationRuntime,
//!     commitment_reconcile::MockAasClient,
//!     storage::InMemoryStateStorage,
//!     snapshot::{SnapshotReason, ResonatorId},
//! };
//!
//! # async fn example() {
//! // Create dependencies
//! let storage = Arc::new(InMemoryStateStorage::new());
//! let state_gatherer = Arc::new(MockRuntimeStateGatherer::new());
//! let state_restorer = Arc::new(MockRuntimeStateRestorer::new());
//! let continuity_verifier = Arc::new(MockContinuityVerifier::new());
//! let coupling_runtime = Arc::new(MockCouplingRuntime::all_present());
//! let migration_runtime = Arc::new(MockMigrationRuntime::new("node-1"));
//! let aas_client = Arc::new(MockAasClient::all_pending());
//!
//! // Create state manager
//! let manager = StateManager::new(
//!     StateManagerConfig::default(),
//!     storage,
//!     state_gatherer,
//!     state_restorer,
//!     continuity_verifier,
//!     coupling_runtime,
//!     migration_runtime,
//!     aas_client,
//! );
//!
//! // Create a checkpoint
//! let instance = InstanceInfo {
//!     instance_id: palm_types::InstanceId::generate(),
//!     deployment_id: palm_types::DeploymentId::generate(),
//!     resonator_id: ResonatorId::generate(),
//! };
//!
//! let snapshot_id = manager.checkpoint(&instance, SnapshotReason::Manual).await.unwrap();
//! println!("Created checkpoint: {}", snapshot_id);
//! # }
//! ```
//!
//! ## State Snapshot Contents
//!
//! A complete snapshot includes:
//!
//! - **Identity State**: Resonator identity and continuity chain
//! - **Presence State**: Presence in resonance field
//! - **Coupling State**: Active couplings with other Resonators
//! - **Meaning Context**: Current interpretations and context
//! - **Intent State**: Current intent (if any)
//! - **Pending Commitments**: For reconciliation with AAS
//! - **Attention State**: Attention budget and allocations
//! - **Application State**: Application-specific opaque data
//!
//! ## Gradual Restoration
//!
//! After restore, state is restored gradually to avoid overwhelming the system:
//!
//! - Presence starts at 50% and recovers naturally
//! - Couplings are restored in batches with delays
//! - Meaning confidence is reduced to 80%
//! - Intent stability is reduced to 50%
//!
//! ## Continuity Guarantees
//!
//! Migration and restoration maintain cryptographic continuity:
//!
//! 1. Continuity chain is verified before restore
//! 2. New continuity link is created for the new incarnation
//! 3. Key references are preserved (not actual keys)

pub mod checkpoint;
pub mod commitment_reconcile;
pub mod coupling_restore;
pub mod error;
pub mod manager;
pub mod migration;
pub mod restore;
pub mod snapshot;
pub mod storage;

// Re-export main types
pub use checkpoint::{CheckpointConfig, CheckpointService, InstanceInfo, RuntimeStateGatherer};
pub use commitment_reconcile::{
    AasClient, CommitmentReconciler, CommitmentStatus, ReconciliationResult,
};
pub use coupling_restore::{
    CouplingRestorationHandle, CouplingRestorationManager, CouplingRestorationResult,
    CouplingRuntime,
};
pub use error::{Result, StateError, StateSnapshotId};
pub use manager::{StateEvent, StateManager, StateManagerConfig};
pub use migration::{MigrationResult, MigrationRuntime, MigrationService, NodeId};
pub use restore::{ContinuityVerifier, RestoreResult, RestoreService, RuntimeStateRestorer};
pub use snapshot::{
    AttentionStateSnapshot, CommitmentSnapshot, ContinuityChainSnapshot, ContinuityLinkSnapshot,
    ContinuityReason, CouplingDirection, CouplingSnapshot, IdentityStateSnapshot,
    IntentSnapshot, InterpretationSnapshot, MeaningContextSnapshot, PresenceStateSnapshot,
    ResonatorId, ResonatorStateSnapshot, SnapshotMetadata, SnapshotReason,
};
pub use storage::{InMemoryStateStorage, StateStorage};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_snapshot_integrity() {
        use snapshot::*;

        let resonator_id = ResonatorId::generate();
        let snapshot = ResonatorStateSnapshot {
            id: StateSnapshotId::generate(),
            metadata: SnapshotMetadata {
                instance_id: palm_types::InstanceId::generate(),
                deployment_id: palm_types::DeploymentId::generate(),
                resonator_id: resonator_id.clone(),
                created_at: chrono::Utc::now(),
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
                discoverability: 0.9,
                responsiveness: 0.85,
                stability: 0.8,
                coupling_readiness: 0.75,
                last_signal: chrono::Utc::now(),
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
        .finalize();

        assert!(snapshot.verify_integrity());
    }

    #[tokio::test]
    async fn test_state_manager_integration() {
        use checkpoint::MockRuntimeStateGatherer;
        use commitment_reconcile::MockAasClient;
        use coupling_restore::MockCouplingRuntime;
        use migration::MockMigrationRuntime;
        use restore::{MockContinuityVerifier, MockRuntimeStateRestorer};

        let storage = Arc::new(InMemoryStateStorage::new());
        let state_gatherer = Arc::new(MockRuntimeStateGatherer::new());
        let state_restorer = Arc::new(MockRuntimeStateRestorer::new());
        let continuity_verifier = Arc::new(MockContinuityVerifier::new());
        let coupling_runtime = Arc::new(MockCouplingRuntime::all_present());
        let migration_runtime = Arc::new(MockMigrationRuntime::new("node-1"));
        let aas_client = Arc::new(MockAasClient::all_pending());

        let manager = StateManager::new(
            StateManagerConfig::default(),
            storage.clone(),
            state_gatherer,
            state_restorer,
            continuity_verifier,
            coupling_runtime,
            migration_runtime,
            aas_client,
        );

        // Create instance info
        let instance = InstanceInfo {
            instance_id: palm_types::InstanceId::generate(),
            deployment_id: palm_types::DeploymentId::generate(),
            resonator_id: ResonatorId::generate(),
        };

        // Create checkpoint
        let snapshot_id = manager
            .checkpoint(&instance, SnapshotReason::Manual)
            .await
            .unwrap();

        // Verify it was stored
        let loaded = manager
            .get_latest_snapshot(&instance.instance_id)
            .await
            .unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().id, snapshot_id);

        // Restore from checkpoint
        let result = manager
            .restore(&instance.instance_id, &snapshot_id)
            .await
            .unwrap();

        assert_eq!(result.instance_id, instance.instance_id);
        assert_eq!(result.previous_incarnation, 1);
        assert_eq!(result.new_incarnation, 2);
    }
}
