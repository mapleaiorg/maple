//! Budget and resource allocation types
//!
//! Collectives allocate resources (attention, financial, coupling slots)
//! to their members. These budgets bound what members can do.

use chrono::{DateTime, Utc};
use resonator_types::ResonatorId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Attention units — the fundamental resource in resonance economics
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct AttentionUnits(pub u64);

impl AttentionUnits {
    pub fn new(units: u64) -> Self {
        Self(units)
    }

    pub fn zero() -> Self {
        Self(0)
    }

    pub fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl std::fmt::Display for AttentionUnits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}au", self.0)
    }
}

impl std::ops::Add for AttentionUnits {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub for AttentionUnits {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

/// Financial amount (generic currency units)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct Amount(pub u64);

impl Amount {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn zero() -> Self {
        Self(0)
    }

    pub fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Add for Amount {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Amount {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

/// A composite budget covering all resource types
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Budget {
    /// Attention units budget
    pub attention: AttentionUnits,
    /// Financial budget
    pub financial: Amount,
    /// Coupling slots budget
    pub coupling_slots: u32,
    /// Workflow initiation quota
    pub workflow_quota: u32,
}

impl Budget {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_attention(mut self, attention: u64) -> Self {
        self.attention = AttentionUnits(attention);
        self
    }

    pub fn with_financial(mut self, financial: u64) -> Self {
        self.financial = Amount(financial);
        self
    }

    pub fn with_coupling_slots(mut self, slots: u32) -> Self {
        self.coupling_slots = slots;
        self
    }

    pub fn with_workflow_quota(mut self, quota: u32) -> Self {
        self.workflow_quota = quota;
        self
    }

    /// Check if all budgets are zero
    pub fn is_empty(&self) -> bool {
        self.attention.is_zero()
            && self.financial.is_zero()
            && self.coupling_slots == 0
            && self.workflow_quota == 0
    }
}

/// An allocation of budget to a specific resonator
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BudgetAllocation {
    /// Who receives the allocation
    pub recipient: ResonatorId,
    /// The allocated budget
    pub budget: Budget,
    /// When the allocation was made
    pub allocated_at: DateTime<Utc>,
    /// Optional budget period (when it resets)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<BudgetPeriod>,
}

impl BudgetAllocation {
    pub fn new(recipient: ResonatorId, budget: Budget) -> Self {
        Self {
            recipient,
            budget,
            allocated_at: Utc::now(),
            period: None,
        }
    }

    pub fn with_period(mut self, period: BudgetPeriod) -> Self {
        self.period = Some(period);
        self
    }
}

/// How often a budget resets
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BudgetPeriod {
    /// Resets each epoch (Mapleverse-specific)
    Epoch,
    /// Resets daily
    Daily,
    /// Resets weekly
    Weekly,
    /// Resets monthly
    Monthly,
    /// Custom period in seconds
    Custom(u64),
}

/// Pool of attention units for a Collective
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttentionPool {
    /// Total attention available to the collective
    pub total: AttentionUnits,
    /// Currently allocated attention
    pub allocated: AttentionUnits,
    /// Per-member allocations
    pub allocations: HashMap<ResonatorId, AttentionUnits>,
}

impl AttentionPool {
    pub fn new(total: u64) -> Self {
        Self {
            total: AttentionUnits(total),
            allocated: AttentionUnits::zero(),
            allocations: HashMap::new(),
        }
    }

    /// Available (unallocated) attention
    pub fn available(&self) -> AttentionUnits {
        self.total.saturating_sub(self.allocated)
    }

    /// Allocate attention to a resonator
    pub fn allocate(
        &mut self,
        resonator: ResonatorId,
        amount: AttentionUnits,
    ) -> Result<(), crate::CollectiveError> {
        if amount > self.available() {
            return Err(crate::CollectiveError::InsufficientAttention {
                required: amount.0,
                available: self.available().0,
            });
        }

        let current = self.allocations.entry(resonator).or_insert(AttentionUnits::zero());
        *current = current.saturating_add(amount);
        self.allocated = self.allocated.saturating_add(amount);
        Ok(())
    }

    /// Release attention from a resonator
    pub fn release(
        &mut self,
        resonator: &ResonatorId,
        amount: AttentionUnits,
    ) {
        if let Some(current) = self.allocations.get_mut(resonator) {
            let release_amount = if amount > *current { *current } else { amount };
            *current = current.saturating_sub(release_amount);
            self.allocated = self.allocated.saturating_sub(release_amount);

            if current.is_zero() {
                self.allocations.remove(resonator);
            }
        }
    }

    /// Get a resonator's current allocation
    pub fn allocation_for(&self, resonator: &ResonatorId) -> AttentionUnits {
        self.allocations
            .get(resonator)
            .copied()
            .unwrap_or(AttentionUnits::zero())
    }
}

/// Pool of coupling slots for a Collective
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CouplingSlotPool {
    /// Total coupling slots
    pub total: u32,
    /// Currently used slots
    pub used: u32,
}

impl CouplingSlotPool {
    pub fn new(total: u32) -> Self {
        Self { total, used: 0 }
    }

    /// Available coupling slots
    pub fn available(&self) -> u32 {
        self.total.saturating_sub(self.used)
    }

    /// Acquire a coupling slot
    pub fn acquire(&mut self) -> Result<(), crate::CollectiveError> {
        if self.used >= self.total {
            return Err(crate::CollectiveError::NoCouplingSlots);
        }
        self.used += 1;
        Ok(())
    }

    /// Release a coupling slot
    pub fn release(&mut self) {
        self.used = self.used.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attention_units() {
        let a = AttentionUnits::new(100);
        let b = AttentionUnits::new(50);
        assert_eq!(a + b, AttentionUnits::new(150));
        assert_eq!(a - b, AttentionUnits::new(50));
        assert_eq!(a.saturating_sub(AttentionUnits::new(200)), AttentionUnits::zero());
        assert_eq!(format!("{}", a), "100au");
    }

    #[test]
    fn test_amount() {
        let a = Amount::new(1000);
        let b = Amount::new(300);
        assert_eq!(a + b, Amount::new(1300));
        assert_eq!(a - b, Amount::new(700));
        assert!(!a.is_zero());
        assert!(Amount::zero().is_zero());
    }

    #[test]
    fn test_budget() {
        let budget = Budget::new()
            .with_attention(1000)
            .with_financial(50_000)
            .with_coupling_slots(10)
            .with_workflow_quota(5);

        assert!(!budget.is_empty());
        assert_eq!(budget.attention, AttentionUnits::new(1000));
        assert_eq!(budget.financial, Amount::new(50_000));

        assert!(Budget::new().is_empty());
    }

    #[test]
    fn test_attention_pool() {
        let mut pool = AttentionPool::new(1000);
        assert_eq!(pool.available(), AttentionUnits::new(1000));

        let res1 = ResonatorId::new("res-1");
        let res2 = ResonatorId::new("res-2");

        pool.allocate(res1.clone(), AttentionUnits::new(300)).unwrap();
        assert_eq!(pool.available(), AttentionUnits::new(700));
        assert_eq!(pool.allocation_for(&res1), AttentionUnits::new(300));

        pool.allocate(res2.clone(), AttentionUnits::new(500)).unwrap();
        assert_eq!(pool.available(), AttentionUnits::new(200));

        // Try to over-allocate
        let result = pool.allocate(ResonatorId::new("res-3"), AttentionUnits::new(300));
        assert!(result.is_err());

        // Release
        pool.release(&res1, AttentionUnits::new(100));
        assert_eq!(pool.allocation_for(&res1), AttentionUnits::new(200));
        assert_eq!(pool.available(), AttentionUnits::new(300));
    }

    #[test]
    fn test_coupling_slot_pool() {
        let mut pool = CouplingSlotPool::new(3);
        assert_eq!(pool.available(), 3);

        pool.acquire().unwrap();
        pool.acquire().unwrap();
        pool.acquire().unwrap();
        assert_eq!(pool.available(), 0);

        let result = pool.acquire();
        assert!(result.is_err());

        pool.release();
        assert_eq!(pool.available(), 1);
        pool.acquire().unwrap();
    }
}
