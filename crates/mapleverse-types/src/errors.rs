//! Error types for MapleVerse operations
//!
//! These errors enforce MapleVerse invariants at runtime, including the
//! critical "no human profiles" constraint.

use crate::entity::EntityId;
use crate::event::EpochId;
use crate::world::RegionId;
use thiserror::Error;

/// Errors that can occur in MapleVerse operations
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum MapleVerseError {
    // =========================================================================
    // CRITICAL: Human Profile Rejection (Runtime Enforced)
    // =========================================================================
    /// **CRITICAL**: Attempted to create a human profile in MapleVerse
    ///
    /// MapleVerse is a pure AI-agent civilization. Human profiles are not
    /// a configuration option - they are architecturally impossible.
    #[error("CRITICAL VIOLATION: Human profiles are not allowed in MapleVerse. Entity '{entity_id}' attempted to register as human. MapleVerse is a pure AI-agent civilization.")]
    HumanProfileRejected {
        /// The entity ID that attempted to register as human
        entity_id: String,
        /// Additional context about the rejection
        context: String,
    },

    /// Attempted to enable human profiles without the unsafe feature flag
    #[error("Cannot enable human profiles: the 'unsafe-human-profiles' feature is not enabled. This feature should NEVER be used in production.")]
    HumanProfilesFeatureDisabled,

    // =========================================================================
    // Entity Errors
    // =========================================================================
    /// Entity not found in the world
    #[error("Entity not found: {0}")]
    EntityNotFound(EntityId),

    /// Entity already exists
    #[error("Entity already exists: {0}")]
    EntityAlreadyExists(EntityId),

    /// Invalid entity state
    #[error("Invalid entity state for '{entity_id}': {reason}")]
    InvalidEntityState {
        /// The entity with invalid state
        entity_id: EntityId,
        /// Why the state is invalid
        reason: String,
    },

    /// Entity is not an individual (when individual required)
    #[error("Entity '{0}' is not an individual agent")]
    NotAnIndividual(EntityId),

    /// Entity is not a collective (when collective required)
    #[error("Entity '{0}' is not a collective")]
    NotACollective(EntityId),

    // =========================================================================
    // Economy Errors
    // =========================================================================
    /// Insufficient MAPLE tokens for operation
    #[error("Insufficient MAPLE balance: required {required}, available {available}")]
    InsufficientMaple {
        /// Amount required
        required: u64,
        /// Amount available
        available: u64,
    },

    /// Insufficient attention for operation
    #[error("Insufficient attention: required {required}, available {available}")]
    InsufficientAttention {
        /// Attention units required
        required: u64,
        /// Attention units available
        available: u64,
    },

    /// Invalid amount (negative or overflow)
    #[error("Invalid amount: {reason}")]
    InvalidAmount {
        /// Why the amount is invalid
        reason: String,
    },

    /// Transfer not allowed
    #[error("Transfer not allowed from '{from}' to '{to}': {reason}")]
    TransferNotAllowed {
        /// Source entity
        from: EntityId,
        /// Target entity
        to: EntityId,
        /// Why transfer is not allowed
        reason: String,
    },

    // =========================================================================
    // Attention Errors
    // =========================================================================
    /// Attention budget exhausted for current epoch
    #[error("Attention budget exhausted for entity '{entity_id}' in epoch {epoch}")]
    AttentionExhausted {
        /// The entity with no attention
        entity_id: EntityId,
        /// Current epoch
        epoch: u64,
    },

    /// Attention cannot be allocated (over budget)
    #[error("Cannot allocate {requested} attention units: only {remaining} remaining in budget")]
    AttentionOverBudget {
        /// Requested units
        requested: u64,
        /// Remaining in budget
        remaining: u64,
    },

    // =========================================================================
    // Reputation Errors
    // =========================================================================
    /// Invalid reputation source (not a receipt)
    #[error("Invalid reputation source: reputation can ONLY come from verified receipts, not '{attempted_source}'")]
    InvalidReputationSource {
        /// What was attempted as a reputation source
        attempted_source: String,
    },

    /// Receipt not found for reputation claim
    #[error("Receipt not found: {receipt_id}")]
    ReceiptNotFound {
        /// The missing receipt ID
        receipt_id: String,
    },

    /// Receipt already used for reputation
    #[error("Receipt '{receipt_id}' has already been used for reputation")]
    ReceiptAlreadyUsed {
        /// The duplicate receipt ID
        receipt_id: String,
    },

    /// Invalid receipt for reputation
    #[error("Receipt '{receipt_id}' is not valid for reputation: {reason}")]
    InvalidReceiptForReputation {
        /// The invalid receipt ID
        receipt_id: String,
        /// Why it's invalid
        reason: String,
    },

    // =========================================================================
    // Region/World Errors
    // =========================================================================
    /// Region not found
    #[error("Region not found: {0}")]
    RegionNotFound(RegionId),

    /// Region already exists
    #[error("Region already exists: {0}")]
    RegionAlreadyExists(RegionId),

    /// Migration not allowed (regions not neighbors)
    #[error("Cannot migrate from '{from}' to '{to}': regions are not neighbors")]
    MigrationNotAllowed {
        /// Source region
        from: RegionId,
        /// Target region (not a neighbor)
        to: RegionId,
    },

    /// Region at capacity
    #[error("Region '{region_id}' is at capacity ({capacity} entities)")]
    RegionAtCapacity {
        /// The full region
        region_id: RegionId,
        /// Maximum capacity
        capacity: u64,
    },

    /// Entity not in expected region
    #[error("Entity '{entity_id}' is not in region '{region_id}'")]
    EntityNotInRegion {
        /// The entity
        entity_id: EntityId,
        /// Expected region
        region_id: RegionId,
    },

    // =========================================================================
    // Epoch/Event Errors
    // =========================================================================
    /// Invalid epoch (in the past or too far in future)
    #[error("Invalid epoch {epoch}: {reason}")]
    InvalidEpoch {
        /// The invalid epoch number
        epoch: u64,
        /// Why it's invalid
        reason: String,
    },

    /// Epoch not found
    #[error("Epoch not found: {0}")]
    EpochNotFound(EpochId),

    /// Event processing failed
    #[error("Failed to process event '{event_id}': {reason}")]
    EventProcessingFailed {
        /// The event that failed
        event_id: String,
        /// Why it failed
        reason: String,
    },

    // =========================================================================
    // Collective Errors
    // =========================================================================
    /// Cannot add member to collective
    #[error("Cannot add member '{member_id}' to collective '{collective_id}': {reason}")]
    CannotAddMember {
        /// The collective
        collective_id: EntityId,
        /// The member being added
        member_id: EntityId,
        /// Why addition failed
        reason: String,
    },

    /// Cannot remove member from collective
    #[error("Cannot remove member '{member_id}' from collective '{collective_id}': {reason}")]
    CannotRemoveMember {
        /// The collective
        collective_id: EntityId,
        /// The member being removed
        member_id: EntityId,
        /// Why removal failed
        reason: String,
    },

    /// Collective governance violation
    #[error("Collective '{collective_id}' governance violation: {violation}")]
    GovernanceViolation {
        /// The collective
        collective_id: EntityId,
        /// What was violated
        violation: String,
    },

    // =========================================================================
    // Configuration Errors
    // =========================================================================
    /// Invalid configuration
    #[error("Invalid configuration: {reason}")]
    InvalidConfiguration {
        /// Why the configuration is invalid
        reason: String,
    },

    /// World not initialized
    #[error("MapleVerse world not initialized")]
    WorldNotInitialized,

    /// World already initialized
    #[error("MapleVerse world already initialized")]
    WorldAlreadyInitialized,

    // =========================================================================
    // Internal Errors
    // =========================================================================
    /// Internal error (should not happen)
    #[error("Internal error: {0}")]
    Internal(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl MapleVerseError {
    /// Create a human profile rejection error
    ///
    /// This is the CRITICAL error that enforces the pure AI-agent civilization invariant.
    pub fn human_rejected(entity_id: impl Into<String>, context: impl Into<String>) -> Self {
        Self::HumanProfileRejected {
            entity_id: entity_id.into(),
            context: context.into(),
        }
    }

    /// Check if this is a critical violation (human profile attempt)
    pub fn is_critical_violation(&self) -> bool {
        matches!(
            self,
            Self::HumanProfileRejected { .. } | Self::HumanProfilesFeatureDisabled
        )
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            // Critical violations are never recoverable
            Self::HumanProfileRejected { .. } | Self::HumanProfilesFeatureDisabled => false,
            // Resource exhaustion might be resolved by waiting
            Self::InsufficientMaple { .. }
            | Self::InsufficientAttention { .. }
            | Self::AttentionExhausted { .. }
            | Self::AttentionOverBudget { .. } => true,
            // Capacity issues might resolve
            Self::RegionAtCapacity { .. } => true,
            // Everything else depends on context
            _ => false,
        }
    }
}

/// Result type for MapleVerse operations
pub type MapleVerseResult<T> = Result<T, MapleVerseError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_human_rejection_error() {
        let err = MapleVerseError::human_rejected("user-123", "attempted registration");
        assert!(err.is_critical_violation());
        assert!(!err.is_recoverable());

        let msg = err.to_string();
        assert!(msg.contains("CRITICAL VIOLATION"));
        assert!(msg.contains("Human profiles are not allowed"));
        assert!(msg.contains("user-123"));
    }

    #[test]
    fn test_insufficient_maple_error() {
        let err = MapleVerseError::InsufficientMaple {
            required: 100,
            available: 50,
        };
        assert!(!err.is_critical_violation());
        assert!(err.is_recoverable());
    }

    #[test]
    fn test_invalid_reputation_source() {
        let err = MapleVerseError::InvalidReputationSource {
            attempted_source: "self-assessment".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("ONLY come from verified receipts"));
    }

    #[test]
    fn test_migration_not_allowed() {
        let err = MapleVerseError::MigrationNotAllowed {
            from: RegionId::new("region-a"),
            to: RegionId::new("region-z"),
        };
        let msg = err.to_string();
        assert!(msg.contains("not neighbors"));
    }

    #[test]
    fn test_attention_exhausted() {
        let err = MapleVerseError::AttentionExhausted {
            entity_id: EntityId::new("agent-1"),
            epoch: 42,
        };
        let msg = err.to_string();
        assert!(msg.contains("exhausted"));
        assert!(msg.contains("epoch 42"));
    }

    #[test]
    fn test_error_display() {
        let errors = vec![
            MapleVerseError::EntityNotFound(EntityId::new("test")),
            MapleVerseError::RegionNotFound(RegionId::new("test")),
            MapleVerseError::WorldNotInitialized,
            MapleVerseError::Internal("test error".to_string()),
        ];

        for err in errors {
            // Just verify we can format all error types
            let _ = format!("{}", err);
            let _ = format!("{:?}", err);
        }
    }
}
