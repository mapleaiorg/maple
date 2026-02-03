//! Attention allocation and management
//!
//! CRITICAL INVARIANT: Coupling MUST always be bounded by available attention.

use dashmap::DashMap;
use crate::types::*;
use crate::types::AttentionConfig;

/// Attention Allocator manages attention budgets for all Resonators
///
/// Attention is the finite capacity of a Resonator to process resonance.
pub struct AttentionAllocator {
    /// Per-Resonator attention budgets
    budgets: DashMap<ResonatorId, AttentionBudget>,

    /// Configuration
    #[allow(dead_code)]
    config: AttentionConfig,
}

impl AttentionAllocator {
    pub fn new(config: &AttentionConfig) -> Self {
        Self {
            budgets: DashMap::new(),
            config: config.clone(),
        }
    }

    /// Allocate attention budget for a new Resonator
    pub async fn allocate_budget(
        &self,
        resonator: &ResonatorId,
        spec: &AttentionBudgetSpec,
    ) -> Result<(), String> {
        let mut budget = AttentionBudget::new(spec.total_capacity);

        if let Some(reserve) = spec.safety_reserve {
            budget.safety_reserve = reserve;
        }

        if let Some(threshold) = spec.exhaustion_threshold {
            budget.exhaustion_threshold = threshold;
        }

        self.budgets.insert(*resonator, budget);

        tracing::debug!(
            "Allocated attention budget for {}: {} total",
            resonator,
            spec.total_capacity
        );

        Ok(())
    }

    /// Allocate attention for a coupling
    ///
    /// Returns error if insufficient attention (prevents over-coupling)
    pub async fn allocate(
        &self,
        resonator: &ResonatorId,
        amount: u64,
    ) -> Result<AllocationToken, AttentionError> {
        let budget = self
            .budgets
            .get_mut(resonator)
            .ok_or(AttentionError::ResonatorNotFound)?;

        let available = budget.available();

        if amount > available {
            return Err(AttentionError::InsufficientAttention {
                requested: amount,
                available,
            });
        }

        let token = AllocationToken::new(*resonator, amount);

        // Record allocation (would be recorded in ledger in real implementation)

        tracing::trace!(
            "Allocated {} attention for {} (available: {})",
            amount,
            resonator,
            available - amount
        );

        Ok(token)
    }

    /// Allocate more attention to an existing coupling
    pub async fn allocate_more(
        &self,
        resonator: &ResonatorId,
        amount: u64,
    ) -> Result<(), AttentionError> {
        let budget = self
            .budgets
            .get_mut(resonator)
            .ok_or(AttentionError::ResonatorNotFound)?;

        let available = budget.available();

        if amount > available {
            return Err(AttentionError::InsufficientAttention {
                requested: amount,
                available,
            });
        }

        // Would record in ledger in real implementation

        Ok(())
    }

    /// Release attention (partial)
    pub async fn release_partial(&self, resonator: &ResonatorId, amount: u64) {
        if let Some(_budget) = self.budgets.get_mut(resonator) {
            // Would release from specific coupling in real implementation
            tracing::trace!("Released {} attention for {}", amount, resonator);
        }
    }

    /// Release all attention from a coupling
    pub async fn release_all(&self, resonator: &ResonatorId, amount: u64) {
        if let Some(_budget) = self.budgets.get_mut(resonator) {
            // Would release from specific coupling in real implementation
            tracing::trace!("Released all {} attention for {}", amount, resonator);
        }
    }

    /// Check if attention exhaustion is imminent
    pub fn is_exhaustion_imminent(&self, resonator: &ResonatorId) -> bool {
        if let Some(budget) = self.budgets.get(resonator) {
            return budget.is_exhaustion_imminent();
        }
        false
    }

    /// Get available attention for coupling
    pub async fn available_for_coupling(
        &self,
        resonator: &ResonatorId,
    ) -> Result<u64, AttentionError> {
        let budget = self
            .budgets
            .get(resonator)
            .ok_or(AttentionError::ResonatorNotFound)?;

        Ok(budget.available())
    }

    /// Get budget for a Resonator
    pub async fn get_budget(&self, resonator: &ResonatorId) -> Option<AttentionBudget> {
        self.budgets.get(resonator).map(|r| r.clone())
    }

    /// Restore attention budget from continuity record
    pub async fn restore_budget(
        &self,
        resonator: &ResonatorId,
        budget: &AttentionBudget,
    ) -> Result<(), String> {
        self.budgets.insert(*resonator, budget.clone());
        tracing::debug!("Restored attention budget for {}", resonator);
        Ok(())
    }

    /// Rebalance attention across couplings
    ///
    /// Called when attention pressure is high to optimize allocation
    pub async fn rebalance(&self, _resonator: &ResonatorId) {
        // Placeholder: In real implementation, would:
        // 1. Sort couplings by importance (meaning convergence, interaction count)
        // 2. Reduce attention to low-value couplings
        // 3. This is a graceful degradation mechanism
        tracing::debug!("Rebalancing attention (placeholder)");
    }

    /// Remove budget (for cleanup)
    pub fn remove_budget(&self, resonator: &ResonatorId) {
        self.budgets.remove(resonator);
    }
}
