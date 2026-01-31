//! Restore service - restores state from snapshots.
//!
//! The restore service loads snapshots and restores Resonator state,
//! delegating to the runtime for actual state application.

use std::sync::Arc;

use tracing::{debug, info, instrument};

use crate::error::{Result, StateError, StateSnapshotId};
use crate::snapshot::*;
use crate::storage::StateStorage;

/// Service for restoring state from snapshots.
pub struct RestoreService {
    /// State storage backend.
    storage: Arc<dyn StateStorage>,

    /// Runtime interface for applying state.
    runtime: Arc<dyn RuntimeStateRestorer>,

    /// Continuity service for chain verification.
    continuity_service: Arc<dyn ContinuityVerifier>,
}

/// Trait for restoring state to the runtime.
///
/// This abstracts the runtime interface so palm-state doesn't depend
/// on the full maple-runtime crate.
#[async_trait::async_trait]
pub trait RuntimeStateRestorer: Send + Sync {
    /// Restore identity state.
    async fn restore_identity(
        &self,
        resonator_id: &ResonatorId,
        incarnation: u64,
        key_reference: &str,
    ) -> Result<()>;

    /// Restore presence state.
    async fn restore_presence(
        &self,
        resonator_id: &ResonatorId,
        state: &PresenceStateSnapshot,
    ) -> Result<()>;

    /// Restore attention state.
    async fn restore_attention(
        &self,
        resonator_id: &ResonatorId,
        state: &AttentionStateSnapshot,
    ) -> Result<()>;

    /// Restore meaning context.
    async fn restore_meaning_context(
        &self,
        resonator_id: &ResonatorId,
        context: &MeaningContextSnapshot,
    ) -> Result<()>;

    /// Restore intent state.
    async fn restore_intent(
        &self,
        resonator_id: &ResonatorId,
        intent: &IntentSnapshot,
    ) -> Result<()>;

    /// Restore application state.
    async fn restore_application_state(
        &self,
        resonator_id: &ResonatorId,
        state: bytes::Bytes,
    ) -> Result<()>;
}

/// Trait for verifying continuity chains.
///
/// Delegates to resonator-identity for actual verification.
#[async_trait::async_trait]
pub trait ContinuityVerifier: Send + Sync {
    /// Verify a continuity chain is valid.
    async fn verify_chain(&self, chain: &ContinuityChainSnapshot) -> Result<()>;

    /// Create a new continuity link for restoration.
    async fn create_continuity_link(
        &self,
        resonator_id: &ResonatorId,
        previous_incarnation: u64,
        reason: ContinuityReason,
    ) -> Result<u64>;
}

/// Result of a restore operation.
#[derive(Debug, Clone)]
pub struct RestoreResult {
    /// The instance that was restored.
    pub instance_id: palm_types::InstanceId,

    /// The snapshot that was used.
    pub snapshot_id: StateSnapshotId,

    /// Previous incarnation number.
    pub previous_incarnation: u64,

    /// New incarnation number after restore.
    pub new_incarnation: u64,

    /// Number of couplings to restore (scheduled).
    pub couplings_to_restore: usize,

    /// Number of commitments to reconcile.
    pub commitments_to_reconcile: usize,
}

impl RestoreService {
    /// Create a new restore service.
    pub fn new(
        storage: Arc<dyn StateStorage>,
        runtime: Arc<dyn RuntimeStateRestorer>,
        continuity_service: Arc<dyn ContinuityVerifier>,
    ) -> Self {
        Self {
            storage,
            runtime,
            continuity_service,
        }
    }

    /// Restore Resonator state from a snapshot.
    #[instrument(skip(self), fields(instance_id = %instance_id, snapshot_id = %snapshot_id))]
    pub async fn restore(
        &self,
        instance_id: &palm_types::InstanceId,
        snapshot_id: &StateSnapshotId,
    ) -> Result<RestoreResult> {
        // Load snapshot
        let snapshot = self
            .storage
            .load(snapshot_id)
            .await?
            .ok_or_else(|| StateError::SnapshotNotFound(snapshot_id.clone()))?;

        self.restore_from_snapshot(instance_id, &snapshot).await
    }

    /// Restore from a snapshot object directly.
    #[instrument(skip(self, snapshot), fields(instance_id = %instance_id))]
    pub async fn restore_from_snapshot(
        &self,
        instance_id: &palm_types::InstanceId,
        snapshot: &ResonatorStateSnapshot,
    ) -> Result<RestoreResult> {
        debug!(instance_id = %instance_id, "Beginning state restoration");

        // 1. Verify snapshot integrity
        if !snapshot.verify_integrity() {
            return Err(StateError::IntegrityCheckFailed {
                expected: snapshot.integrity_hash.clone(),
                actual: snapshot.calculate_hash(),
            });
        }

        // 2. Verify continuity chain
        self.continuity_service
            .verify_chain(&snapshot.identity_state.continuity_chain)
            .await?;

        // 3. Create new continuity link
        let new_incarnation = self
            .continuity_service
            .create_continuity_link(
                &snapshot.identity_state.resonator_id,
                snapshot.identity_state.incarnation,
                ContinuityReason::Recovery,
            )
            .await?;

        info!(
            instance_id = %instance_id,
            previous_incarnation = snapshot.identity_state.incarnation,
            new_incarnation = new_incarnation,
            "Created continuity link"
        );

        // 4. Restore identity state
        self.runtime
            .restore_identity(
                &snapshot.identity_state.resonator_id,
                new_incarnation,
                &snapshot.identity_state.key_reference,
            )
            .await
            .map_err(|e| StateError::RestoreFailed {
                instance_id: instance_id.clone(),
                reason: format!("identity restore failed: {}", e),
            })?;

        // 5. Restore presence state (start low, will recover naturally)
        let reduced_presence = PresenceStateSnapshot {
            discoverability: snapshot.presence_state.discoverability * 0.5,
            responsiveness: snapshot.presence_state.responsiveness * 0.5,
            stability: 0.1, // Start with low stability
            coupling_readiness: snapshot.presence_state.coupling_readiness * 0.5,
            last_signal: chrono::Utc::now(),
        };

        self.runtime
            .restore_presence(&snapshot.metadata.resonator_id, &reduced_presence)
            .await
            .map_err(|e| StateError::RestoreFailed {
                instance_id: instance_id.clone(),
                reason: format!("presence restore failed: {}", e),
            })?;

        // 6. Restore attention state (reset allocations)
        let reset_attention = AttentionStateSnapshot {
            total: snapshot.attention_state.total,
            available: snapshot.attention_state.available,
            allocated: 0, // Reset allocations
            reserved: snapshot.attention_state.reserved,
            allocations: Vec::new(),
        };

        self.runtime
            .restore_attention(&snapshot.metadata.resonator_id, &reset_attention)
            .await
            .map_err(|e| StateError::RestoreFailed {
                instance_id: instance_id.clone(),
                reason: format!("attention restore failed: {}", e),
            })?;

        // 7. Restore meaning context (with reduced confidence)
        let reduced_meaning = MeaningContextSnapshot {
            interpretations: snapshot
                .meaning_context
                .interpretations
                .iter()
                .map(|i| InterpretationSnapshot {
                    id: i.id.clone(),
                    content: i.content.clone(),
                    confidence: i.confidence * 0.8, // Reduce confidence
                    formed_at: i.formed_at,
                })
                .collect(),
            context_factors: snapshot.meaning_context.context_factors.clone(),
            confidence_distribution: snapshot.meaning_context.confidence_distribution.clone(),
        };

        self.runtime
            .restore_meaning_context(&snapshot.metadata.resonator_id, &reduced_meaning)
            .await
            .map_err(|e| StateError::RestoreFailed {
                instance_id: instance_id.clone(),
                reason: format!("meaning context restore failed: {}", e),
            })?;

        // 8. Restore intent state (if any, with reduced stability)
        if let Some(intent) = &snapshot.intent_state {
            let reduced_intent = IntentSnapshot {
                id: intent.id.clone(),
                direction: intent.direction.clone(),
                confidence: intent.confidence * 0.8,
                stability: intent.stability * 0.5, // Significantly reduce stability
                conditions: intent.conditions.clone(),
            };

            self.runtime
                .restore_intent(&snapshot.metadata.resonator_id, &reduced_intent)
                .await
                .map_err(|e| StateError::RestoreFailed {
                    instance_id: instance_id.clone(),
                    reason: format!("intent restore failed: {}", e),
                })?;
        }

        // 9. Restore application state
        if let Some(app_state) = &snapshot.application_state {
            self.runtime
                .restore_application_state(&snapshot.metadata.resonator_id, app_state.clone())
                .await
                .map_err(|e| StateError::RestoreFailed {
                    instance_id: instance_id.clone(),
                    reason: format!("application state restore failed: {}", e),
                })?;
        }

        // NOTE: Couplings are restored gradually by CouplingRestorationManager
        // NOTE: Commitments are reconciled with AAS by CommitmentReconciler

        info!(instance_id = %instance_id, "State restoration completed");

        Ok(RestoreResult {
            instance_id: instance_id.clone(),
            snapshot_id: snapshot.id.clone(),
            previous_incarnation: snapshot.identity_state.incarnation,
            new_incarnation,
            couplings_to_restore: snapshot.coupling_state.len(),
            commitments_to_reconcile: snapshot.pending_commitments.len(),
        })
    }
}

/// Mock runtime state restorer for testing.
pub struct MockRuntimeStateRestorer {
    /// Whether to simulate errors.
    pub simulate_error: bool,
}

impl MockRuntimeStateRestorer {
    /// Create a new mock restorer.
    pub fn new() -> Self {
        Self {
            simulate_error: false,
        }
    }

    /// Create a mock restorer that returns errors.
    pub fn with_errors() -> Self {
        Self {
            simulate_error: true,
        }
    }
}

impl Default for MockRuntimeStateRestorer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl RuntimeStateRestorer for MockRuntimeStateRestorer {
    async fn restore_identity(
        &self,
        _resonator_id: &ResonatorId,
        _incarnation: u64,
        _key_reference: &str,
    ) -> Result<()> {
        if self.simulate_error {
            return Err(StateError::Runtime("simulated error".to_string()));
        }
        Ok(())
    }

    async fn restore_presence(
        &self,
        _resonator_id: &ResonatorId,
        _state: &PresenceStateSnapshot,
    ) -> Result<()> {
        if self.simulate_error {
            return Err(StateError::Runtime("simulated error".to_string()));
        }
        Ok(())
    }

    async fn restore_attention(
        &self,
        _resonator_id: &ResonatorId,
        _state: &AttentionStateSnapshot,
    ) -> Result<()> {
        if self.simulate_error {
            return Err(StateError::Runtime("simulated error".to_string()));
        }
        Ok(())
    }

    async fn restore_meaning_context(
        &self,
        _resonator_id: &ResonatorId,
        _context: &MeaningContextSnapshot,
    ) -> Result<()> {
        if self.simulate_error {
            return Err(StateError::Runtime("simulated error".to_string()));
        }
        Ok(())
    }

    async fn restore_intent(
        &self,
        _resonator_id: &ResonatorId,
        _intent: &IntentSnapshot,
    ) -> Result<()> {
        if self.simulate_error {
            return Err(StateError::Runtime("simulated error".to_string()));
        }
        Ok(())
    }

    async fn restore_application_state(
        &self,
        _resonator_id: &ResonatorId,
        _state: bytes::Bytes,
    ) -> Result<()> {
        if self.simulate_error {
            return Err(StateError::Runtime("simulated error".to_string()));
        }
        Ok(())
    }
}

/// Mock continuity verifier for testing.
pub struct MockContinuityVerifier {
    /// Whether to simulate errors.
    pub simulate_error: bool,

    /// Next incarnation number to return.
    pub next_incarnation: u64,
}

impl MockContinuityVerifier {
    /// Create a new mock verifier.
    pub fn new() -> Self {
        Self {
            simulate_error: false,
            next_incarnation: 2,
        }
    }
}

impl Default for MockContinuityVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ContinuityVerifier for MockContinuityVerifier {
    async fn verify_chain(&self, _chain: &ContinuityChainSnapshot) -> Result<()> {
        if self.simulate_error {
            return Err(StateError::ContinuityVerificationFailed(
                "simulated error".to_string(),
            ));
        }
        Ok(())
    }

    async fn create_continuity_link(
        &self,
        _resonator_id: &ResonatorId,
        _previous_incarnation: u64,
        _reason: ContinuityReason,
    ) -> Result<u64> {
        if self.simulate_error {
            return Err(StateError::ContinuityVerificationFailed(
                "simulated error".to_string(),
            ));
        }
        Ok(self.next_incarnation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryStateStorage;
    use chrono::Utc;

    fn create_test_snapshot() -> ResonatorStateSnapshot {
        let resonator_id = ResonatorId::generate();
        let instance_id = palm_types::InstanceId::generate();
        let deployment_id = palm_types::DeploymentId::generate();

        ResonatorStateSnapshot {
            id: StateSnapshotId::generate(),
            metadata: SnapshotMetadata {
                instance_id,
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
                key_reference: "test-key-ref".to_string(),
            },
            presence_state: PresenceStateSnapshot {
                discoverability: 0.9,
                responsiveness: 0.85,
                stability: 0.8,
                coupling_readiness: 0.75,
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
    async fn test_restore_from_snapshot() {
        let storage = Arc::new(InMemoryStateStorage::new());
        let runtime = Arc::new(MockRuntimeStateRestorer::new());
        let continuity = Arc::new(MockContinuityVerifier::new());

        let service = RestoreService::new(storage.clone(), runtime, continuity);

        let snapshot = create_test_snapshot();
        let instance_id = snapshot.metadata.instance_id.clone();

        // Store the snapshot first
        storage.store(&snapshot).await.unwrap();

        let result = service.restore(&instance_id, &snapshot.id).await.unwrap();

        assert_eq!(result.instance_id, instance_id);
        assert_eq!(result.previous_incarnation, 1);
        assert_eq!(result.new_incarnation, 2);
    }

    #[tokio::test]
    async fn test_restore_integrity_failure() {
        let storage = Arc::new(InMemoryStateStorage::new());
        let runtime = Arc::new(MockRuntimeStateRestorer::new());
        let continuity = Arc::new(MockContinuityVerifier::new());

        let service = RestoreService::new(storage.clone(), runtime, continuity);

        // Create a tampered snapshot
        let mut snapshot = create_test_snapshot();
        snapshot.attention_state.total = 999; // Tamper without updating hash

        let instance_id = snapshot.metadata.instance_id.clone();
        storage.store(&snapshot).await.unwrap();

        let result = service.restore(&instance_id, &snapshot.id).await;
        assert!(matches!(result, Err(StateError::IntegrityCheckFailed { .. })));
    }
}
