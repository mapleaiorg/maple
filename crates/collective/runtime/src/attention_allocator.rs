//! Attention Allocator — manages collective attention economics
//!
//! Allocates attention from the collective's pool to individual members.
//! Enforces Invariant 5: Coupling bounded by attention.
//! Attention is the fundamental resource that limits what any member can do.

use collective_types::{
    AttentionPool, AttentionUnits, AuditJournal, CollectiveId, CollectiveReceipt, CollectiveResult,
    CouplingSlotPool, ReceiptType,
};
use resonator_types::ResonatorId;
use tracing::{debug, info, warn};

/// Configuration for attention allocation
#[derive(Clone, Debug)]
pub struct AttentionConfig {
    /// Total attention pool for the collective
    pub total_attention: u64,
    /// Total coupling slots
    pub total_coupling_slots: u32,
    /// Safety reserve percentage (0.0-1.0) — never fully exhausted
    pub safety_reserve_ratio: f64,
    /// Threshold for exhaustion warning (0.0-1.0)
    pub exhaustion_warning_threshold: f64,
    /// Default allocation per member
    pub default_member_allocation: u64,
}

impl Default for AttentionConfig {
    fn default() -> Self {
        Self {
            total_attention: 100_000,
            total_coupling_slots: 100,
            safety_reserve_ratio: 0.10,
            exhaustion_warning_threshold: 0.80,
            default_member_allocation: 1_000,
        }
    }
}

/// Manages attention and coupling slot allocation for a Collective
pub struct CollectiveAttentionAllocator {
    /// The attention pool
    attention_pool: AttentionPool,
    /// Coupling slot pool
    coupling_pool: CouplingSlotPool,
    /// Configuration
    config: AttentionConfig,
    /// Collective identity for receipts
    collective_id: CollectiveId,
    /// Safety reserve (locked, cannot be allocated)
    safety_reserve: AttentionUnits,
}

impl CollectiveAttentionAllocator {
    /// Create a new allocator with config
    pub fn new(collective_id: CollectiveId, config: AttentionConfig) -> Self {
        let safety_reserve = AttentionUnits::new(
            (config.total_attention as f64 * config.safety_reserve_ratio) as u64,
        );

        // Effective pool = total - safety reserve
        let effective_total = config.total_attention - safety_reserve.0;

        Self {
            attention_pool: AttentionPool::new(effective_total),
            coupling_pool: CouplingSlotPool::new(config.total_coupling_slots),
            config,
            collective_id,
            safety_reserve,
        }
    }

    /// Create with default config
    pub fn with_defaults(collective_id: CollectiveId) -> Self {
        Self::new(collective_id, AttentionConfig::default())
    }

    // --- Attention allocation ---

    /// Allocate attention to a member
    pub fn allocate_attention(
        &mut self,
        resonator: ResonatorId,
        amount: AttentionUnits,
        journal: &mut AuditJournal,
    ) -> CollectiveResult<()> {
        self.attention_pool.allocate(resonator.clone(), amount)?;

        debug!(
            resonator = %resonator,
            amount = amount.0,
            remaining = self.attention_pool.available().0,
            "Attention allocated"
        );

        // Check exhaustion warning
        if self.is_exhaustion_imminent() {
            warn!(
                available = self.attention_pool.available().0,
                total = self.attention_pool.total.0,
                "Attention exhaustion imminent"
            );

            journal.log_receipt(CollectiveReceipt::new(
                self.collective_id.clone(),
                ReceiptType::Custom("attention_warning".into()),
                resonator,
                "Attention pool nearing exhaustion".to_string(),
            ));
        }

        Ok(())
    }

    /// Allocate default amount to a new member
    pub fn allocate_default(
        &mut self,
        resonator: ResonatorId,
        journal: &mut AuditJournal,
    ) -> CollectiveResult<()> {
        let amount = AttentionUnits::new(self.config.default_member_allocation);
        self.allocate_attention(resonator, amount, journal)
    }

    /// Release attention from a member
    pub fn release_attention(&mut self, resonator: &ResonatorId, amount: AttentionUnits) {
        self.attention_pool.release(resonator, amount);

        debug!(
            resonator = %resonator,
            amount = amount.0,
            available = self.attention_pool.available().0,
            "Attention released"
        );
    }

    /// Release all attention for a member (on departure)
    pub fn release_all_attention(&mut self, resonator: &ResonatorId) {
        let current = self.attention_pool.allocation_for(resonator);
        if !current.is_zero() {
            self.attention_pool.release(resonator, current);
            info!(
                resonator = %resonator,
                released = current.0,
                "All attention released for member"
            );
        }
    }

    /// Rebalance attention across members (graceful degradation)
    pub fn rebalance(&mut self, active_member_ids: &[ResonatorId], journal: &mut AuditJournal) {
        if active_member_ids.is_empty() {
            return;
        }

        let total_available = self.attention_pool.total;
        let per_member = AttentionUnits::new(total_available.0 / active_member_ids.len() as u64);

        // Release all existing allocations
        for member_id in active_member_ids {
            self.release_all_attention(member_id);
        }

        // Reallocate evenly
        for member_id in active_member_ids {
            // Best-effort: if allocation fails, continue with others
            let _ = self.attention_pool.allocate(member_id.clone(), per_member);
        }

        info!(
            members = active_member_ids.len(),
            per_member = per_member.0,
            "Attention rebalanced"
        );

        journal.log_receipt(CollectiveReceipt::new(
            self.collective_id.clone(),
            ReceiptType::Custom("attention_rebalanced".into()),
            ResonatorId::new("system"),
            format!(
                "Attention rebalanced: {} per member across {} members",
                per_member,
                active_member_ids.len()
            ),
        ));
    }

    // --- Coupling slots ---

    /// Acquire a coupling slot
    pub fn acquire_coupling_slot(&mut self) -> CollectiveResult<()> {
        self.coupling_pool.acquire()
    }

    /// Release a coupling slot
    pub fn release_coupling_slot(&mut self) {
        self.coupling_pool.release();
    }

    // --- Query methods ---

    /// Available attention
    pub fn available_attention(&self) -> AttentionUnits {
        self.attention_pool.available()
    }

    /// Total attention (excluding safety reserve)
    pub fn total_attention(&self) -> AttentionUnits {
        self.attention_pool.total
    }

    /// Currently allocated attention
    pub fn allocated_attention(&self) -> AttentionUnits {
        self.attention_pool.allocated
    }

    /// Attention allocated to a specific member
    pub fn attention_for(&self, resonator: &ResonatorId) -> AttentionUnits {
        self.attention_pool.allocation_for(resonator)
    }

    /// Safety reserve (not allocatable)
    pub fn safety_reserve(&self) -> AttentionUnits {
        self.safety_reserve
    }

    /// Available coupling slots
    pub fn available_coupling_slots(&self) -> u32 {
        self.coupling_pool.available()
    }

    /// Used coupling slots
    pub fn used_coupling_slots(&self) -> u32 {
        self.coupling_pool.used
    }

    /// Check if attention exhaustion is imminent
    pub fn is_exhaustion_imminent(&self) -> bool {
        let total = self.attention_pool.total.0 as f64;
        let allocated = self.attention_pool.allocated.0 as f64;
        if total == 0.0 {
            return true;
        }
        (allocated / total) >= self.config.exhaustion_warning_threshold
    }

    /// Utilization ratio (0.0 - 1.0)
    pub fn utilization_ratio(&self) -> f64 {
        let total = self.attention_pool.total.0 as f64;
        if total == 0.0 {
            return 0.0;
        }
        self.attention_pool.allocated.0 as f64 / total
    }

    /// Get config
    pub fn config(&self) -> &AttentionConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (CollectiveAttentionAllocator, AuditJournal) {
        let id = CollectiveId::new("test");
        let config = AttentionConfig {
            total_attention: 10_000,
            total_coupling_slots: 10,
            safety_reserve_ratio: 0.10, // 1000 reserved
            exhaustion_warning_threshold: 0.80,
            default_member_allocation: 500,
        };
        (
            CollectiveAttentionAllocator::new(id.clone(), config),
            AuditJournal::new(id),
        )
    }

    #[test]
    fn test_safety_reserve() {
        let (alloc, _) = setup();
        assert_eq!(alloc.safety_reserve(), AttentionUnits::new(1_000));
        // Effective total = 10000 - 1000 = 9000
        assert_eq!(alloc.total_attention(), AttentionUnits::new(9_000));
    }

    #[test]
    fn test_allocate_and_release() {
        let (mut alloc, mut journal) = setup();
        let res = ResonatorId::new("res-1");

        alloc
            .allocate_attention(res.clone(), AttentionUnits::new(2_000), &mut journal)
            .unwrap();

        assert_eq!(alloc.attention_for(&res), AttentionUnits::new(2_000));
        assert_eq!(alloc.available_attention(), AttentionUnits::new(7_000));

        alloc.release_attention(&res, AttentionUnits::new(1_000));
        assert_eq!(alloc.attention_for(&res), AttentionUnits::new(1_000));
        assert_eq!(alloc.available_attention(), AttentionUnits::new(8_000));
    }

    #[test]
    fn test_over_allocation() {
        let (mut alloc, mut journal) = setup();
        let res = ResonatorId::new("res-1");

        // Try to allocate more than available
        let result = alloc.allocate_attention(res, AttentionUnits::new(20_000), &mut journal);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_allocation() {
        let (mut alloc, mut journal) = setup();
        let res = ResonatorId::new("res-1");

        alloc.allocate_default(res.clone(), &mut journal).unwrap();
        assert_eq!(alloc.attention_for(&res), AttentionUnits::new(500));
    }

    #[test]
    fn test_release_all() {
        let (mut alloc, mut journal) = setup();
        let res = ResonatorId::new("res-1");

        alloc
            .allocate_attention(res.clone(), AttentionUnits::new(3_000), &mut journal)
            .unwrap();

        alloc.release_all_attention(&res);
        assert_eq!(alloc.attention_for(&res), AttentionUnits::zero());
        assert_eq!(alloc.available_attention(), AttentionUnits::new(9_000));
    }

    #[test]
    fn test_rebalance() {
        let (mut alloc, mut journal) = setup();
        let members = vec![
            ResonatorId::new("res-1"),
            ResonatorId::new("res-2"),
            ResonatorId::new("res-3"),
        ];

        // Initially allocate unevenly
        alloc
            .allocate_attention(members[0].clone(), AttentionUnits::new(5_000), &mut journal)
            .unwrap();
        alloc
            .allocate_attention(members[1].clone(), AttentionUnits::new(1_000), &mut journal)
            .unwrap();

        // Rebalance
        alloc.rebalance(&members, &mut journal);

        // Each should get 9000/3 = 3000
        assert_eq!(alloc.attention_for(&members[0]), AttentionUnits::new(3_000));
        assert_eq!(alloc.attention_for(&members[1]), AttentionUnits::new(3_000));
        assert_eq!(alloc.attention_for(&members[2]), AttentionUnits::new(3_000));
    }

    #[test]
    fn test_exhaustion_warning() {
        let (mut alloc, mut journal) = setup();

        // Allocate 80%+ of 9000 = 7200+
        alloc
            .allocate_attention(
                ResonatorId::new("res-1"),
                AttentionUnits::new(7_500),
                &mut journal,
            )
            .unwrap();

        assert!(alloc.is_exhaustion_imminent());
        assert!(alloc.utilization_ratio() > 0.80);
    }

    #[test]
    fn test_coupling_slots() {
        let (mut alloc, _) = setup();

        assert_eq!(alloc.available_coupling_slots(), 10);

        alloc.acquire_coupling_slot().unwrap();
        alloc.acquire_coupling_slot().unwrap();
        assert_eq!(alloc.available_coupling_slots(), 8);
        assert_eq!(alloc.used_coupling_slots(), 2);

        alloc.release_coupling_slot();
        assert_eq!(alloc.available_coupling_slots(), 9);
    }

    #[test]
    fn test_coupling_slot_exhaustion() {
        let id = CollectiveId::new("test");
        let config = AttentionConfig {
            total_coupling_slots: 2,
            ..AttentionConfig::default()
        };
        let mut alloc = CollectiveAttentionAllocator::new(id, config);

        alloc.acquire_coupling_slot().unwrap();
        alloc.acquire_coupling_slot().unwrap();

        let result = alloc.acquire_coupling_slot();
        assert!(result.is_err());
    }
}
