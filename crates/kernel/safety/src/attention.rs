use std::collections::HashMap;

use maple_mwl_types::WorldlineId;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::error::SafetyError;

/// Attention Budget — finite resource management.
///
/// Per I.S-BOUND: Coupling MUST always be bounded by available attention.
/// Attention is a finite resource. No worldline may consume unbounded
/// attention from another.
///
/// The attention budget ensures that:
/// - Total allocated attention never exceeds capacity
/// - Each worldline's allocation is bounded
/// - Reserved attention is protected from allocation
/// - Budget exhaustion is a safety signal
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttentionBudget {
    /// Total attention capacity (arbitrary units)
    pub total_capacity: u64,
    /// Allocated attention per worldline
    pub allocated: HashMap<WorldlineId, u64>,
    /// Reserved attention (cannot be allocated)
    pub reserved: u64,
}

impl AttentionBudget {
    /// Create a new attention budget with the given capacity.
    pub fn new(total_capacity: u64) -> Self {
        Self {
            total_capacity,
            allocated: HashMap::new(),
            reserved: 0,
        }
    }

    /// Create a budget with reserved capacity.
    pub fn with_reserve(total_capacity: u64, reserved: u64) -> Self {
        assert!(
            reserved <= total_capacity,
            "Reserved attention cannot exceed total capacity"
        );
        Self {
            total_capacity,
            allocated: HashMap::new(),
            reserved,
        }
    }

    /// Available attention (total - allocated - reserved).
    pub fn available(&self) -> u64 {
        let total_allocated: u64 = self.allocated.values().sum();
        self.total_capacity
            .saturating_sub(total_allocated)
            .saturating_sub(self.reserved)
    }

    /// Total currently allocated.
    pub fn total_allocated(&self) -> u64 {
        self.allocated.values().sum()
    }

    /// Allocate attention to a worldline.
    ///
    /// Returns error if insufficient attention is available.
    pub fn allocate(&mut self, wid: &WorldlineId, amount: u64) -> Result<(), SafetyError> {
        let available = self.available();
        if amount > available {
            warn!(
                worldline = %wid,
                requested = amount,
                available = available,
                "Attention allocation rejected — insufficient budget"
            );
            return Err(SafetyError::InsufficientAttention {
                requested: amount,
                available,
            });
        }

        let allocated_val = {
            let current = self.allocated.entry(wid.clone()).or_insert(0);
            *current += amount;
            *current
        };

        debug!(
            worldline = %wid,
            allocated = allocated_val,
            remaining = self.available(),
            "Attention allocated"
        );

        Ok(())
    }

    /// Release attention from a worldline.
    ///
    /// If the release amount exceeds the current allocation,
    /// the allocation is set to zero (never goes negative).
    pub fn release(&mut self, wid: &WorldlineId, amount: u64) {
        if let Some(current) = self.allocated.get_mut(wid) {
            *current = current.saturating_sub(amount);
            if *current == 0 {
                self.allocated.remove(wid);
            }
            debug!(
                worldline = %wid,
                released = amount,
                remaining = self.available(),
                "Attention released"
            );
        }
    }

    /// Release all attention from a worldline.
    pub fn release_all(&mut self, wid: &WorldlineId) {
        self.allocated.remove(wid);
    }

    /// Is the budget exhausted (no more available)?
    pub fn is_exhausted(&self) -> bool {
        self.available() == 0
    }

    /// Get the allocation for a specific worldline.
    pub fn allocation(&self, wid: &WorldlineId) -> u64 {
        self.allocated.get(wid).copied().unwrap_or(0)
    }

    /// Get the attention fraction consumed by a worldline (0.0–1.0).
    pub fn attention_fraction(&self, wid: &WorldlineId) -> f64 {
        let alloc = self.allocation(wid) as f64;
        let total = self.total_capacity as f64;
        if total == 0.0 {
            return 0.0;
        }
        (alloc / total).min(1.0)
    }

    /// Number of worldlines with active allocations.
    pub fn active_worldlines(&self) -> usize {
        self.allocated.len()
    }

    /// Check if a new allocation would be safe.
    ///
    /// Returns Ok if the allocation is within bounds.
    pub fn check_allocation(&self, _wid: &WorldlineId, amount: u64) -> Result<(), SafetyError> {
        let available = self.available();
        if amount > available {
            return Err(SafetyError::InsufficientAttention {
                requested: amount,
                available,
            });
        }
        Ok(())
    }
}

impl Default for AttentionBudget {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_mwl_types::IdentityMaterial;

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn other_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32]))
    }

    #[test]
    fn new_budget_has_full_capacity() {
        let budget = AttentionBudget::new(100);
        assert_eq!(budget.available(), 100);
        assert!(!budget.is_exhausted());
    }

    #[test]
    fn allocate_reduces_available() {
        let mut budget = AttentionBudget::new(100);
        let wid = test_worldline();

        budget.allocate(&wid, 30).unwrap();
        assert_eq!(budget.available(), 70);
        assert_eq!(budget.allocation(&wid), 30);
    }

    #[test]
    fn allocate_exceeding_budget_fails() {
        let mut budget = AttentionBudget::new(100);
        let wid = test_worldline();

        let result = budget.allocate(&wid, 150);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SafetyError::InsufficientAttention { .. }
        ));
    }

    #[test]
    fn release_increases_available() {
        let mut budget = AttentionBudget::new(100);
        let wid = test_worldline();

        budget.allocate(&wid, 50).unwrap();
        assert_eq!(budget.available(), 50);

        budget.release(&wid, 30);
        assert_eq!(budget.available(), 80);
        assert_eq!(budget.allocation(&wid), 20);
    }

    #[test]
    fn release_all() {
        let mut budget = AttentionBudget::new(100);
        let wid = test_worldline();

        budget.allocate(&wid, 50).unwrap();
        budget.release_all(&wid);
        assert_eq!(budget.available(), 100);
        assert_eq!(budget.allocation(&wid), 0);
    }

    #[test]
    fn reserved_reduces_available() {
        let budget = AttentionBudget::with_reserve(100, 20);
        assert_eq!(budget.available(), 80);
    }

    #[test]
    fn cannot_allocate_into_reserved() {
        let mut budget = AttentionBudget::with_reserve(100, 80);
        let wid = test_worldline();

        // Only 20 available (100 - 80 reserved)
        assert!(budget.allocate(&wid, 20).is_ok());
        assert!(budget.allocate(&wid, 1).is_err());
    }

    #[test]
    fn budget_exhaustion() {
        let mut budget = AttentionBudget::new(50);
        let wid = test_worldline();

        budget.allocate(&wid, 50).unwrap();
        assert!(budget.is_exhausted());
    }

    #[test]
    fn multiple_worldlines() {
        let mut budget = AttentionBudget::new(100);
        let wid1 = test_worldline();
        let wid2 = other_worldline();

        budget.allocate(&wid1, 40).unwrap();
        budget.allocate(&wid2, 30).unwrap();

        assert_eq!(budget.available(), 30);
        assert_eq!(budget.active_worldlines(), 2);
        assert_eq!(budget.total_allocated(), 70);
    }

    #[test]
    fn attention_fraction() {
        let mut budget = AttentionBudget::new(100);
        let wid = test_worldline();

        budget.allocate(&wid, 50).unwrap();
        assert!((budget.attention_fraction(&wid) - 0.5).abs() < 0.001);
    }

    #[test]
    fn release_over_allocation_saturates_to_zero() {
        let mut budget = AttentionBudget::new(100);
        let wid = test_worldline();

        budget.allocate(&wid, 30).unwrap();
        budget.release(&wid, 100); // Release more than allocated

        assert_eq!(budget.allocation(&wid), 0);
        assert_eq!(budget.available(), 100);
    }

    #[test]
    fn check_allocation_without_modifying() {
        let budget = AttentionBudget::new(100);
        let wid = test_worldline();

        assert!(budget.check_allocation(&wid, 50).is_ok());
        assert!(budget.check_allocation(&wid, 150).is_err());
        // Budget unchanged
        assert_eq!(budget.available(), 100);
    }

    #[test]
    fn coupling_bounded_by_attention() {
        // Per I.S-BOUND: Coupling MUST always be bounded by available attention
        let mut budget = AttentionBudget::new(100);
        let wid = test_worldline();

        // Allocate all attention
        budget.allocate(&wid, 100).unwrap();

        // Cannot establish more couplings — budget exhausted
        let other = other_worldline();
        let result = budget.allocate(&other, 1);
        assert!(result.is_err());
        assert!(budget.is_exhausted());
    }
}
