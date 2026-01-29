//! Identity types for MAPLE Resonance Runtime

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a Resonator
///
/// NOT a UUID in the traditional sense - represents persistent identity
/// that survives restarts, migrations, and network partitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResonatorId(Uuid);

impl ResonatorId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl Default for ResonatorId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ResonatorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "resonator:{}", self.0)
    }
}

/// Unique identifier for a coupling relationship
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CouplingId(Uuid);

impl CouplingId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for CouplingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "coupling:{}", self.0)
    }
}

/// Unique identifier for a commitment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitmentId(Uuid);

impl CommitmentId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for CommitmentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "commitment:{}", self.0)
    }
}

/// Unique identifier for a temporal anchor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnchorId(Uuid);

impl AnchorId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for AnchorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "anchor:{}", self.0)
    }
}

/// Unique identifier for an allocation token
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AllocationToken(Uuid);

impl AllocationToken {
    pub fn new(_resonator: ResonatorId, _amount: u64) -> Self {
        Self(Uuid::new_v4())
    }
}
