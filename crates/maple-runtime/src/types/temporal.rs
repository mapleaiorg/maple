//! Temporal coordination types
//!
//! The Resonance Architecture does NOT assume synchronized clocks.
//! Time is defined relationally through temporal anchors.

use super::ids::{AnchorId, CommitmentId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A temporal anchor - any event that allows ordering interactions
///
/// Temporal anchors enable causal ordering without global clocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalAnchor {
    /// Unique identifier
    pub id: AnchorId,

    /// Local timestamp (for this Resonator only)
    ///
    /// This is NOT a global timestamp - it's only meaningful
    /// within the context of a single Resonator's timeline.
    pub local_time: LocalTimestamp,

    /// Causal dependencies (what happened before this)
    ///
    /// This is what enables causal ordering across Resonators.
    pub causal_deps: Vec<AnchorId>,

    /// Associated commitment (if any)
    pub commitment: Option<CommitmentId>,
}

impl TemporalAnchor {
    /// Create a new temporal anchor with current local time
    pub fn now() -> Self {
        Self {
            id: AnchorId::generate(),
            local_time: LocalTimestamp::now(),
            causal_deps: Vec::new(),
            commitment: None,
        }
    }

    /// Create a temporal anchor with explicit dependencies
    pub fn with_deps(causal_deps: Vec<AnchorId>) -> Self {
        Self {
            id: AnchorId::generate(),
            local_time: LocalTimestamp::now(),
            causal_deps,
            commitment: None,
        }
    }

    /// Add a causal dependency
    pub fn add_dep(&mut self, dep: AnchorId) {
        if !self.causal_deps.contains(&dep) {
            self.causal_deps.push(dep);
        }
    }
}

/// Local timestamp - only meaningful within a single Resonator's timeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalTimestamp {
    /// Local monotonic counter
    pub sequence: u64,

    /// Wall clock time (for human-readable debugging)
    ///
    /// NOT used for causal ordering - only for debugging and logging.
    pub wall_clock: DateTime<Utc>,
}

impl LocalTimestamp {
    pub fn now() -> Self {
        Self {
            sequence: 0, // Will be assigned by timeline
            wall_clock: Utc::now(),
        }
    }

    pub fn with_sequence(sequence: u64) -> Self {
        Self {
            sequence,
            wall_clock: Utc::now(),
        }
    }
}

impl Default for LocalTimestamp {
    fn default() -> Self {
        Self::now()
    }
}

/// Configuration for temporal coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalConfig {
    /// Enable wall clock timestamps for debugging?
    pub enable_wall_clock: bool,

    /// Maximum causal dependency chain length
    pub max_causal_chain_length: usize,

    /// Enable causal verification?
    pub enable_causal_verification: bool,
}

impl Default for TemporalConfig {
    fn default() -> Self {
        Self {
            enable_wall_clock: true,
            max_causal_chain_length: 1000,
            enable_causal_verification: true,
        }
    }
}
