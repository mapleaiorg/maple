//! Attention budget and allocation types
//!
//! CRITICAL INVARIANT: Coupling MUST always be bounded by available attention.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::ids::CouplingId;

/// Attention budget for a Resonator
///
/// Attention is the finite capacity of a Resonator to process resonance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionBudget {
    /// Total attention capacity
    pub total_capacity: u64,

    /// Currently allocated attention (by coupling)
    pub allocated: HashMap<CouplingId, u64>,

    /// Reserved for safety (never fully deplete)
    pub safety_reserve: u64,

    /// Threshold that triggers degradation (0.0 to 1.0)
    pub exhaustion_threshold: f64,
}

impl AttentionBudget {
    pub fn new(total_capacity: u64) -> Self {
        let safety_reserve = total_capacity / 10; // Reserve 10%
        Self {
            total_capacity,
            allocated: HashMap::new(),
            safety_reserve,
            exhaustion_threshold: 0.8,
        }
    }

    /// Calculate currently used attention
    pub fn used(&self) -> u64 {
        self.allocated.values().sum()
    }

    /// Calculate available attention
    pub fn available(&self) -> u64 {
        self.total_capacity
            .saturating_sub(self.used())
            .saturating_sub(self.safety_reserve)
    }

    /// Calculate utilization (0.0 to 1.0)
    pub fn utilization(&self) -> f64 {
        if self.total_capacity == 0 {
            return 0.0;
        }
        self.used() as f64 / self.total_capacity as f64
    }

    /// Is exhaustion imminent?
    pub fn is_exhaustion_imminent(&self) -> bool {
        self.utilization() > self.exhaustion_threshold
    }

    /// Can allocate this amount?
    pub fn can_allocate(&self, amount: u64) -> bool {
        self.available() >= amount
    }
}

/// Attention allocation specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionBudgetSpec {
    /// Total capacity to allocate
    pub total_capacity: u64,

    /// Custom safety reserve (optional)
    pub safety_reserve: Option<u64>,

    /// Custom exhaustion threshold (optional)
    pub exhaustion_threshold: Option<f64>,
}

impl Default for AttentionBudgetSpec {
    fn default() -> Self {
        Self {
            total_capacity: 10000,
            safety_reserve: None,
            exhaustion_threshold: None,
        }
    }
}

/// What happens when attention is exhausted?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExhaustionBehavior {
    /// Reject new coupling attempts
    Reject,

    /// Gracefully degrade by weakening existing couplings
    GracefulDegrade,

    /// Queue new couplings for later
    Queue,
}

/// Attention class for prioritization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttentionClass {
    /// Critical operations (safety-related)
    Critical,

    /// High priority
    High,

    /// Normal priority
    Normal,

    /// Low priority (background tasks)
    Low,
}

impl AttentionClass {
    /// Get the attention cost multiplier for this class
    pub fn cost_multiplier(&self) -> f64 {
        match self {
            AttentionClass::Critical => 2.0,
            AttentionClass::High => 1.5,
            AttentionClass::Normal => 1.0,
            AttentionClass::Low => 0.5,
        }
    }
}

/// Configuration for attention allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionConfig {
    /// Default capacity for new Resonators
    pub default_capacity: u64,

    /// Allow unlimited attention (disable attention economics)
    pub allow_unlimited: bool,

    /// Behavior on exhaustion
    pub exhaustion_behavior: ExhaustionBehavior,

    /// Enable attention rebalancing?
    pub enable_rebalancing: bool,
}

impl Default for AttentionConfig {
    fn default() -> Self {
        Self {
            default_capacity: 10000,
            allow_unlimited: false,
            exhaustion_behavior: ExhaustionBehavior::GracefulDegrade,
            enable_rebalancing: true,
        }
    }
}
