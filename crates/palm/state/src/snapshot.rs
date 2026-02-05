//! Resonator state snapshot types.
//!
//! Complete snapshot of Resonator state for checkpoint/restore operations.
//! These snapshots capture the multi-dimensional state of a Resonator
//! including identity, presence, coupling, meaning, intent, commitments,
//! and attention.

use chrono::{DateTime, Utc};
use palm_types::{DeploymentId, InstanceId};
use serde::{Deserialize, Serialize};

use crate::error::StateSnapshotId;

/// Complete snapshot of Resonator state for checkpoint/restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonatorStateSnapshot {
    /// Unique identifier for this snapshot.
    pub id: StateSnapshotId,

    /// Metadata about the snapshot.
    pub metadata: SnapshotMetadata,

    /// Identity state including continuity chain.
    pub identity_state: IdentityStateSnapshot,

    /// Presence state in the resonance field.
    pub presence_state: PresenceStateSnapshot,

    /// Active couplings with other Resonators.
    pub coupling_state: Vec<CouplingSnapshot>,

    /// Meaning context and interpretations.
    pub meaning_context: MeaningContextSnapshot,

    /// Current intent state (if any).
    pub intent_state: Option<IntentSnapshot>,

    /// Pending commitments for reconciliation.
    pub pending_commitments: Vec<CommitmentSnapshot>,

    /// Attention budget state.
    pub attention_state: AttentionStateSnapshot,

    /// Application-specific state (opaque bytes).
    pub application_state: Option<bytes::Bytes>,

    /// Integrity hash for verification.
    pub integrity_hash: String,
}

/// Metadata about a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Instance this snapshot is for.
    pub instance_id: InstanceId,

    /// Deployment the instance belongs to.
    pub deployment_id: DeploymentId,

    /// Resonator identity.
    pub resonator_id: ResonatorId,

    /// When the snapshot was created.
    pub created_at: DateTime<Utc>,

    /// Incarnation number at snapshot time.
    pub incarnation: u64,

    /// Reason for the snapshot.
    pub reason: SnapshotReason,

    /// Whether the snapshot data is compressed.
    pub compressed: bool,

    /// Whether the snapshot data is encrypted.
    pub encrypted: bool,
}

/// Reason for creating a snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotReason {
    /// Scheduled periodic checkpoint.
    Scheduled,
    /// Before a restart operation.
    PreRestart,
    /// Before a migration operation.
    PreMigration,
    /// Manual checkpoint request.
    Manual,
    /// Part of health recovery.
    HealthRecovery,
}

impl std::fmt::Display for SnapshotReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotReason::Scheduled => write!(f, "scheduled"),
            SnapshotReason::PreRestart => write!(f, "pre-restart"),
            SnapshotReason::PreMigration => write!(f, "pre-migration"),
            SnapshotReason::Manual => write!(f, "manual"),
            SnapshotReason::HealthRecovery => write!(f, "health-recovery"),
        }
    }
}

/// Resonator identity (simplified for palm-state).
/// In full implementation, this would come from resonator-types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResonatorId(String);

impl ResonatorId {
    /// Create a new resonator ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new random resonator ID.
    pub fn generate() -> Self {
        Self(format!("resonator-{}", uuid::Uuid::new_v4()))
    }

    /// Get the ID as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ResonatorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Identity state snapshot.
/// NOTE: Actual chain verification is done by resonator-identity crate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityStateSnapshot {
    /// The Resonator's identity.
    pub resonator_id: ResonatorId,

    /// Continuity chain (for restoration).
    pub continuity_chain: ContinuityChainSnapshot,

    /// Current incarnation number.
    pub incarnation: u64,

    /// Reference to cryptographic key material (not the actual keys).
    pub key_reference: String,
}

/// Continuity chain snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityChainSnapshot {
    /// Links in the continuity chain.
    pub links: Vec<ContinuityLinkSnapshot>,
}

/// A single link in the continuity chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityLinkSnapshot {
    /// Previous incarnation number.
    pub previous_incarnation: u64,

    /// New incarnation number.
    pub new_incarnation: u64,

    /// When the transition occurred.
    pub timestamp: DateTime<Utc>,

    /// Cryptographic proof of continuity.
    pub proof: String,

    /// Reason for the incarnation change.
    pub reason: ContinuityReason,
}

/// Reason for a continuity transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContinuityReason {
    /// Normal restart.
    Restart,
    /// Migration to another node.
    Migration,
    /// Recovery from failure.
    Recovery,
    /// Version upgrade.
    Upgrade,
}

impl std::fmt::Display for ContinuityReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContinuityReason::Restart => write!(f, "restart"),
            ContinuityReason::Migration => write!(f, "migration"),
            ContinuityReason::Recovery => write!(f, "recovery"),
            ContinuityReason::Upgrade => write!(f, "upgrade"),
        }
    }
}

/// Presence state snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceStateSnapshot {
    /// How discoverable the Resonator is (0.0-1.0).
    pub discoverability: f64,

    /// How responsive the Resonator is (0.0-1.0).
    pub responsiveness: f64,

    /// How stable the Resonator's presence is (0.0-1.0).
    pub stability: f64,

    /// Readiness for new couplings (0.0-1.0).
    pub coupling_readiness: f64,

    /// Time of last presence signal.
    pub last_signal: DateTime<Utc>,
}

/// Coupling snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingSnapshot {
    /// The coupled peer's identity.
    pub peer_id: ResonatorId,

    /// Direction of the coupling.
    pub direction: CouplingDirection,

    /// Coupling intensity (0.0-1.0).
    pub intensity: f64,

    /// Scope of the coupling.
    pub scope: String,

    /// How long the coupling should persist.
    pub persistence: std::time::Duration,

    /// Accumulated meaning from this coupling.
    pub accumulated_meaning: Option<String>,

    /// When the coupling was established.
    pub created_at: DateTime<Utc>,
}

/// Direction of a coupling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CouplingDirection {
    /// Inbound coupling (other -> self).
    Inbound,
    /// Outbound coupling (self -> other).
    Outbound,
    /// Bidirectional coupling.
    Bidirectional,
}

impl std::fmt::Display for CouplingDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CouplingDirection::Inbound => write!(f, "inbound"),
            CouplingDirection::Outbound => write!(f, "outbound"),
            CouplingDirection::Bidirectional => write!(f, "bidirectional"),
        }
    }
}

/// Meaning context snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeaningContextSnapshot {
    /// Current interpretations.
    pub interpretations: Vec<InterpretationSnapshot>,

    /// Context factors influencing meaning.
    pub context_factors: Vec<String>,

    /// Confidence distribution across interpretations.
    pub confidence_distribution: Vec<f64>,
}

/// Interpretation snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpretationSnapshot {
    /// Interpretation identifier.
    pub id: String,

    /// The interpretation content.
    pub content: String,

    /// Confidence in this interpretation (0.0-1.0).
    pub confidence: f64,

    /// When the interpretation was formed.
    pub formed_at: DateTime<Utc>,
}

/// Intent snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentSnapshot {
    /// Intent identifier.
    pub id: String,

    /// Direction/goal of the intent.
    pub direction: String,

    /// Confidence in the intent (0.0-1.0).
    pub confidence: f64,

    /// Stability of the intent (0.0-1.0).
    pub stability: f64,

    /// Conditions required for the intent.
    pub conditions: Vec<String>,
}

/// Commitment snapshot (pending commitments for reconciliation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentSnapshot {
    /// Commitment identifier.
    pub commitment_id: String,

    /// Scope of the commitment.
    pub scope: String,

    /// Current status.
    pub status: String,

    /// When the commitment was declared.
    pub declared_at: DateTime<Utc>,

    /// Parties involved in the commitment.
    pub parties: Vec<String>,
}

/// Attention state snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionStateSnapshot {
    /// Total attention budget.
    pub total: u64,

    /// Available attention.
    pub available: u64,

    /// Currently allocated attention.
    pub allocated: u64,

    /// Reserved attention.
    pub reserved: u64,

    /// Current allocations.
    pub allocations: Vec<AttentionAllocationSnapshot>,
}

/// Attention allocation snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionAllocationSnapshot {
    /// Target of the allocation.
    pub target: String,

    /// Amount allocated.
    pub amount: u64,

    /// Purpose of the allocation.
    pub purpose: String,
}

impl ResonatorStateSnapshot {
    /// Calculate integrity hash for the snapshot.
    pub fn calculate_hash(&self) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();

        // Hash key fields
        hasher.update(self.id.as_uuid().as_bytes());
        hasher.update(self.metadata.resonator_id.as_str().as_bytes());
        hasher.update(&self.metadata.incarnation.to_le_bytes());

        // Hash identity state
        hasher.update(self.identity_state.resonator_id.as_str().as_bytes());
        hasher.update(&self.identity_state.incarnation.to_le_bytes());

        // Hash attention state
        hasher.update(&self.attention_state.total.to_le_bytes());
        hasher.update(&self.attention_state.available.to_le_bytes());

        // Hash commitment count
        hasher.update(&(self.pending_commitments.len() as u64).to_le_bytes());

        // Hash coupling count
        hasher.update(&(self.coupling_state.len() as u64).to_le_bytes());

        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(hasher.finalize())
    }

    /// Verify snapshot integrity.
    pub fn verify_integrity(&self) -> bool {
        self.integrity_hash == self.calculate_hash()
    }

    /// Create a new snapshot with calculated integrity hash.
    pub fn finalize(mut self) -> Self {
        self.integrity_hash = self.calculate_hash();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_snapshot() -> ResonatorStateSnapshot {
        let resonator_id = ResonatorId::generate();
        let instance_id = InstanceId::generate();
        let deployment_id = DeploymentId::generate();

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

    #[test]
    fn test_snapshot_integrity() {
        let snapshot = create_test_snapshot();

        assert!(snapshot.verify_integrity());
    }

    #[test]
    fn test_snapshot_integrity_tampered() {
        let mut snapshot = create_test_snapshot();

        // Tamper with the snapshot
        snapshot.attention_state.total = 999;

        // Integrity check should fail
        assert!(!snapshot.verify_integrity());
    }

    #[test]
    fn test_snapshot_reason_display() {
        assert_eq!(SnapshotReason::Scheduled.to_string(), "scheduled");
        assert_eq!(SnapshotReason::PreMigration.to_string(), "pre-migration");
    }
}
