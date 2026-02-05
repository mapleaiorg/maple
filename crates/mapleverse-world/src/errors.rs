//! Error types for MapleVerse World operations

use mapleverse_types::entity::EntityId;
use mapleverse_types::errors::MapleVerseError;
use mapleverse_types::event::EpochId;
use mapleverse_types::world::RegionId;
use thiserror::Error;

/// Errors that can occur in world operations
#[derive(Error, Debug)]
pub enum WorldError {
    /// Error from the types layer
    #[error(transparent)]
    Types(#[from] MapleVerseError),

    /// World is not initialized
    #[error("World not initialized")]
    NotInitialized,

    /// World already initialized
    #[error("World already initialized")]
    AlreadyInitialized,

    /// Entity registration failed
    #[error("Failed to register entity '{entity_id}': {reason}")]
    RegistrationFailed {
        /// The entity that failed to register
        entity_id: EntityId,
        /// Why registration failed
        reason: String,
    },

    /// Entity not found
    #[error("Entity not found: {0}")]
    EntityNotFound(EntityId),

    /// Region not found
    #[error("Region not found: {0}")]
    RegionNotFound(RegionId),

    /// Invalid epoch
    #[error("Invalid epoch: current is {current}, requested {requested}")]
    InvalidEpoch {
        /// Current epoch
        current: EpochId,
        /// Requested epoch
        requested: EpochId,
    },

    /// Epoch transition failed
    #[error("Epoch transition failed: {0}")]
    EpochTransitionFailed(String),

    /// Operation requires active epoch
    #[error("No active epoch")]
    NoActiveEpoch,

    /// Concurrent modification error
    #[error("Concurrent modification detected: {0}")]
    ConcurrentModification(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl WorldError {
    /// Check if this is a critical violation (human profile attempt)
    pub fn is_critical_violation(&self) -> bool {
        match self {
            Self::Types(e) => e.is_critical_violation(),
            _ => false,
        }
    }
}

/// Result type for world operations
pub type WorldResult<T> = Result<T, WorldError>;
