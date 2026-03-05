//! Inter-worldline coupling and attention budget management.
//!
//! Couplings represent active relationships between worldlines, consuming
//! attention budget proportional to their strength. Every worldline has a
//! finite attention budget that bounds the number and intensity of its
//! simultaneous couplings.

use serde::{Deserialize, Serialize};
use worldline_types::{TemporalAnchor, WorldlineId};

/// Unique identifier for a coupling between two worldlines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CouplingId(pub uuid::Uuid);

impl CouplingId {
    /// Generate a new random coupling ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for CouplingId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CouplingId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cpl:{}", self.0)
    }
}

/// Coupling strength in the range \[0.0, 1.0\].
///
/// Higher strength means more attention cost and tighter coupling.
/// Attention cost scales linearly with strength.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CouplingStrength(f64);

impl CouplingStrength {
    /// Create a new coupling strength, validated to \[0.0, 1.0\].
    pub fn new(value: f64) -> Result<Self, CouplingError> {
        if !(0.0..=1.0).contains(&value) {
            return Err(CouplingError::InvalidStrength(value));
        }
        Ok(Self(value))
    }

    /// Get the strength value.
    pub fn value(&self) -> f64 {
        self.0
    }

    /// Attention cost equals the strength value (linear scaling).
    pub fn attention_cost(&self) -> f64 {
        self.0
    }
}

/// State machine for coupling lifecycle.
///
/// Valid transitions: Proposed → Active → Suspended ↔ Active, Active → Dissolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CouplingState {
    /// Proposed but not yet accepted by remote worldline.
    Proposed,
    /// Active — mutual awareness established, attention allocated.
    Active,
    /// Temporarily suspended (attention released).
    Suspended,
    /// Permanently dissolved.
    Dissolved,
}

/// A coupling between two worldlines.
///
/// Represents a directed relationship from a local worldline to a remote one,
/// consuming attention budget proportional to coupling strength.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coupling {
    /// Unique coupling identifier.
    pub id: CouplingId,
    /// Local worldline in this coupling.
    pub local: WorldlineId,
    /// Remote worldline in this coupling.
    pub remote: WorldlineId,
    /// Coupling strength.
    pub strength: CouplingStrength,
    /// When this coupling was established.
    pub established_at: TemporalAnchor,
    /// Last interaction timestamp.
    pub last_interaction: TemporalAnchor,
    /// Attention cost consumed by this coupling.
    pub attention_cost: f64,
    /// Current state.
    pub state: CouplingState,
}

/// Attention budget constraining the total coupling capacity of a worldline.
///
/// Each worldline has a finite attention budget (determined by its profile)
/// that limits both the number and total strength of simultaneous couplings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionBudget {
    /// Total attention capacity.
    pub total: f64,
    /// Currently allocated attention.
    pub allocated: f64,
    /// Maximum simultaneous couplings.
    pub max_couplings: usize,
}

impl AttentionBudget {
    /// Create a new budget with the given capacity.
    pub fn new(total: f64, max_couplings: usize) -> Self {
        Self {
            total,
            allocated: 0.0,
            max_couplings,
        }
    }

    /// Remaining attention capacity.
    pub fn remaining(&self) -> f64 {
        self.total - self.allocated
    }

    /// Check if the given cost can be allocated.
    pub fn can_allocate(&self, cost: f64) -> bool {
        self.allocated + cost <= self.total
    }

    /// Allocate attention for a coupling.
    pub fn allocate(&mut self, cost: f64) -> Result<(), CouplingError> {
        if !self.can_allocate(cost) {
            return Err(CouplingError::BudgetExceeded {
                needed: cost,
                available: self.remaining(),
            });
        }
        self.allocated += cost;
        Ok(())
    }

    /// Release attention when a coupling is dissolved or suspended.
    pub fn release(&mut self, cost: f64) {
        self.allocated = (self.allocated - cost).max(0.0);
    }
}

/// Errors from coupling operations.
#[derive(Debug, thiserror::Error)]
pub enum CouplingError {
    /// Coupling strength outside valid range.
    #[error("invalid coupling strength: {0} (must be 0.0..=1.0)")]
    InvalidStrength(f64),

    /// Attention budget exceeded.
    #[error("attention budget exceeded: needed {needed}, available {available}")]
    BudgetExceeded {
        /// Attention needed.
        needed: f64,
        /// Attention available.
        available: f64,
    },

    /// Maximum coupling count reached.
    #[error("maximum couplings reached: {max}")]
    MaxCouplingsReached {
        /// Maximum allowed couplings.
        max: usize,
    },

    /// Coupling not found.
    #[error("coupling not found: {0}")]
    NotFound(String),

    /// Invalid state transition.
    #[error("invalid coupling state transition")]
    InvalidTransition,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coupling_strength_valid() {
        assert!(CouplingStrength::new(0.0).is_ok());
        assert!(CouplingStrength::new(0.5).is_ok());
        assert!(CouplingStrength::new(1.0).is_ok());
    }

    #[test]
    fn coupling_strength_invalid() {
        assert!(CouplingStrength::new(-0.1).is_err());
        assert!(CouplingStrength::new(1.1).is_err());
        assert!(CouplingStrength::new(f64::NAN).is_err());
    }

    #[test]
    fn coupling_strength_attention_cost() {
        let s = CouplingStrength::new(0.7).unwrap();
        assert!((s.attention_cost() - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn attention_budget_lifecycle() {
        let mut budget = AttentionBudget::new(10.0, 5);
        assert_eq!(budget.remaining(), 10.0);
        assert!(budget.can_allocate(5.0));

        budget.allocate(3.0).unwrap();
        assert!((budget.remaining() - 7.0).abs() < f64::EPSILON);

        budget.allocate(7.0).unwrap();
        assert!((budget.remaining()).abs() < f64::EPSILON);

        assert!(budget.allocate(0.1).is_err());

        budget.release(5.0);
        assert!((budget.remaining() - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn attention_budget_release_clamps() {
        let mut budget = AttentionBudget::new(10.0, 5);
        budget.allocate(3.0).unwrap();
        // Releasing more than allocated clamps to 0
        budget.release(100.0);
        assert_eq!(budget.allocated, 0.0);
        assert_eq!(budget.remaining(), 10.0);
    }

    #[test]
    fn coupling_id_uniqueness() {
        let a = CouplingId::new();
        let b = CouplingId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn coupling_id_display() {
        let id = CouplingId::new();
        let s = id.to_string();
        assert!(s.starts_with("cpl:"));
    }

    #[test]
    fn coupling_state_serde_roundtrip() {
        for state in [
            CouplingState::Proposed,
            CouplingState::Active,
            CouplingState::Suspended,
            CouplingState::Dissolved,
        ] {
            let json = serde_json::to_string(&state).unwrap();
            let back: CouplingState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, back);
        }
    }
}
