//! Economy types for MapleVerse
//!
//! Dual-currency system:
//! - **MAPLE**: The primary token for value transfer
//! - **Attention**: Scarce resource that regenerates per epoch, bounds action capacity

use crate::errors::{MapleVerseError, MapleVerseResult};
use serde::{Deserialize, Serialize};

/// A monetary amount (used for MAPLE tokens)
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Amount(u64);

impl Amount {
    /// Create a new amount
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Zero amount
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Get the value
    pub const fn value(&self) -> u64 {
        self.0
    }

    /// Check if zero
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Checked addition
    pub fn checked_add(&self, other: Amount) -> Option<Amount> {
        self.0.checked_add(other.0).map(Amount)
    }

    /// Checked subtraction
    pub fn checked_sub(&self, other: Amount) -> Option<Amount> {
        self.0.checked_sub(other.0).map(Amount)
    }

    /// Saturating addition
    pub fn saturating_add(&self, other: Amount) -> Amount {
        Amount(self.0.saturating_add(other.0))
    }

    /// Saturating subtraction
    pub fn saturating_sub(&self, other: Amount) -> Amount {
        Amount(self.0.saturating_sub(other.0))
    }
}

impl std::ops::Add for Amount {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl std::ops::Sub for Amount {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for Amount {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

/// Attention units - the scarce resource that bounds action capacity
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct AttentionUnits(u64);

impl AttentionUnits {
    /// Create new attention units
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Zero attention
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Get the value
    pub const fn value(&self) -> u64 {
        self.0
    }

    /// Check if zero
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Checked addition
    pub fn checked_add(&self, other: AttentionUnits) -> Option<AttentionUnits> {
        self.0.checked_add(other.0).map(AttentionUnits)
    }

    /// Checked subtraction
    pub fn checked_sub(&self, other: AttentionUnits) -> Option<AttentionUnits> {
        self.0.checked_sub(other.0).map(AttentionUnits)
    }
}

impl std::fmt::Display for AttentionUnits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}attn", self.0)
    }
}

impl From<u64> for AttentionUnits {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

/// MAPLE token balance
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MapleBalance {
    /// Current balance
    amount: Amount,
    /// Total earned (lifetime)
    total_earned: Amount,
    /// Total spent (lifetime)
    total_spent: Amount,
    /// Total transferred out
    total_transferred_out: Amount,
    /// Total received
    total_received: Amount,
}

impl MapleBalance {
    /// Create a new balance with initial amount
    pub fn new(initial: u64) -> Self {
        Self {
            amount: Amount::new(initial),
            total_earned: Amount::new(initial),
            total_spent: Amount::zero(),
            total_transferred_out: Amount::zero(),
            total_received: Amount::zero(),
        }
    }

    /// Get current balance amount
    pub fn amount(&self) -> u64 {
        self.amount.value()
    }

    /// Get the Amount type
    pub fn as_amount(&self) -> Amount {
        self.amount
    }

    /// Check if balance is sufficient
    pub fn has_sufficient(&self, required: u64) -> bool {
        self.amount.value() >= required
    }

    /// Earn MAPLE (from rewards, etc.)
    pub fn earn(&mut self, amount: u64) {
        let earned = Amount::new(amount);
        self.amount = self.amount.saturating_add(earned);
        self.total_earned = self.total_earned.saturating_add(earned);
    }

    /// Spend MAPLE
    pub fn spend(&mut self, amount: u64) -> MapleVerseResult<()> {
        if !self.has_sufficient(amount) {
            return Err(MapleVerseError::InsufficientMaple {
                required: amount,
                available: self.amount.value(),
            });
        }
        let spent = Amount::new(amount);
        self.amount = self.amount.saturating_sub(spent);
        self.total_spent = self.total_spent.saturating_add(spent);
        Ok(())
    }

    /// Transfer MAPLE to another entity
    pub fn transfer_out(&mut self, amount: u64) -> MapleVerseResult<()> {
        if !self.has_sufficient(amount) {
            return Err(MapleVerseError::InsufficientMaple {
                required: amount,
                available: self.amount.value(),
            });
        }
        let transferred = Amount::new(amount);
        self.amount = self.amount.saturating_sub(transferred);
        self.total_transferred_out = self.total_transferred_out.saturating_add(transferred);
        Ok(())
    }

    /// Receive MAPLE from another entity
    pub fn receive(&mut self, amount: u64) {
        let received = Amount::new(amount);
        self.amount = self.amount.saturating_add(received);
        self.total_received = self.total_received.saturating_add(received);
    }

    /// Get total earned (lifetime)
    pub fn total_earned(&self) -> u64 {
        self.total_earned.value()
    }

    /// Get total spent (lifetime)
    pub fn total_spent(&self) -> u64 {
        self.total_spent.value()
    }

    /// Get net position (earned - spent + received - transferred)
    pub fn net_position(&self) -> i64 {
        let inflows = self.total_earned.value() + self.total_received.value();
        let outflows = self.total_spent.value() + self.total_transferred_out.value();
        inflows as i64 - outflows as i64
    }
}

/// Attention budget for an entity
///
/// Attention is:
/// - Scarce: Limited per epoch
/// - Regenerating: Refills each epoch
/// - Tradeable: Can be transferred between entities
/// - Bounding: Actions require attention
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttentionBudget {
    /// Available attention units
    pub available: u64,
    /// Maximum capacity
    pub capacity: u64,
    /// Used this epoch
    pub used_this_epoch: u64,
    /// Epoch when last regenerated
    pub last_regeneration_epoch: u64,
    /// Attention received from others
    pub received: u64,
    /// Attention given to others
    pub given: u64,
}

impl Default for AttentionBudget {
    fn default() -> Self {
        Self {
            available: 1000,
            capacity: 1000,
            used_this_epoch: 0,
            last_regeneration_epoch: 0,
            received: 0,
            given: 0,
        }
    }
}

impl AttentionBudget {
    /// Create a new attention budget
    pub fn new(available: u64, capacity: u64) -> Self {
        Self {
            available: available.min(capacity),
            capacity,
            used_this_epoch: 0,
            last_regeneration_epoch: 0,
            received: 0,
            given: 0,
        }
    }

    /// Check if there's enough attention
    pub fn has_sufficient(&self, required: u64) -> bool {
        self.available >= required
    }

    /// Consume attention for an action
    pub fn consume(&mut self, amount: u64) -> MapleVerseResult<()> {
        if !self.has_sufficient(amount) {
            return Err(MapleVerseError::InsufficientAttention {
                required: amount,
                available: self.available,
            });
        }
        self.available -= amount;
        self.used_this_epoch += amount;
        Ok(())
    }

    /// Give attention to another entity
    pub fn give(&mut self, amount: u64) -> MapleVerseResult<u64> {
        if !self.has_sufficient(amount) {
            return Err(MapleVerseError::InsufficientAttention {
                required: amount,
                available: self.available,
            });
        }
        self.available -= amount;
        self.given += amount;
        Ok(amount)
    }

    /// Receive attention from another entity
    pub fn receive_attention(&mut self, amount: u64) {
        // Received attention can exceed normal capacity (it's a gift!)
        self.available += amount;
        self.received += amount;
    }

    /// Regenerate attention for new epoch
    pub fn regenerate(&mut self, epoch: u64, regeneration_rate: f64, max_carryover: u64) {
        if epoch <= self.last_regeneration_epoch {
            return; // Already regenerated for this epoch
        }

        // Carryover from previous epoch (capped)
        let carryover = self.available.min(max_carryover);

        // Calculate regeneration
        let base_regen = (self.capacity as f64 * regeneration_rate) as u64;

        // New available = carryover + regeneration, capped at capacity
        self.available = (carryover + base_regen).min(self.capacity);
        self.used_this_epoch = 0;
        self.last_regeneration_epoch = epoch;
    }

    /// Get utilization ratio for this epoch
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            return 0.0;
        }
        self.used_this_epoch as f64 / self.capacity as f64
    }

    /// Get efficiency score (how well attention was used)
    pub fn efficiency_score(&self) -> f64 {
        let total_available = self.capacity + self.received;
        if total_available == 0 {
            return 0.0;
        }
        (self.used_this_epoch + self.given) as f64 / total_available as f64
    }
}

/// Transfer record for auditing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferRecord {
    /// Unique transfer ID
    pub transfer_id: String,
    /// Source entity
    pub from: crate::entity::EntityId,
    /// Target entity
    pub to: crate::entity::EntityId,
    /// Amount transferred
    pub amount: Amount,
    /// Transfer type
    pub transfer_type: TransferType,
    /// Fee charged (if any)
    pub fee: Amount,
    /// When the transfer occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Reference (receipt ID, etc.)
    pub reference: Option<String>,
}

/// Type of transfer
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferType {
    /// Direct transfer between entities
    Direct,
    /// Payment for a service/good
    Payment,
    /// Reward for completing a commitment
    Reward,
    /// Fee collection
    Fee,
    /// Attention transfer
    Attention,
    /// Collective treasury operation
    Treasury,
}

/// Economic summary for reporting
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EconomicSummary {
    /// Total MAPLE in circulation
    pub total_maple_supply: u64,
    /// Total attention available
    pub total_attention_available: u64,
    /// Number of transfers this epoch
    pub transfers_this_epoch: u64,
    /// Total volume this epoch
    pub volume_this_epoch: u64,
    /// Average transaction size
    pub avg_transaction_size: f64,
    /// Velocity (transactions per active entity)
    pub velocity: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_operations() {
        let a = Amount::new(100);
        let b = Amount::new(50);

        assert_eq!((a + b).value(), 150);
        assert_eq!((a - b).value(), 50);
        assert_eq!(a.checked_add(b).unwrap().value(), 150);
        assert_eq!(a.checked_sub(b).unwrap().value(), 50);

        let max = Amount::new(u64::MAX);
        assert!(max.checked_add(Amount::new(1)).is_none());
    }

    #[test]
    fn test_amount_saturating() {
        let a = Amount::new(100);
        let b = Amount::new(150);

        assert_eq!(a.saturating_sub(b).value(), 0);
        assert_eq!(
            Amount::new(u64::MAX)
                .saturating_add(Amount::new(100))
                .value(),
            u64::MAX
        );
    }

    #[test]
    fn test_attention_units() {
        let a = AttentionUnits::new(100);
        assert_eq!(a.value(), 100);
        assert!(!a.is_zero());
        assert!(AttentionUnits::zero().is_zero());

        let display = format!("{}", a);
        assert_eq!(display, "100attn");
    }

    #[test]
    fn test_maple_balance_earn_spend() {
        let mut balance = MapleBalance::new(1000);

        assert_eq!(balance.amount(), 1000);
        assert!(balance.has_sufficient(1000));
        assert!(!balance.has_sufficient(1001));

        balance.earn(500);
        assert_eq!(balance.amount(), 1500);
        assert_eq!(balance.total_earned(), 1500);

        balance.spend(200).unwrap();
        assert_eq!(balance.amount(), 1300);
        assert_eq!(balance.total_spent(), 200);

        // Cannot overspend
        assert!(balance.spend(2000).is_err());
    }

    #[test]
    fn test_maple_balance_transfer() {
        let mut sender = MapleBalance::new(1000);
        let mut receiver = MapleBalance::new(500);

        sender.transfer_out(300).unwrap();
        receiver.receive(300);

        assert_eq!(sender.amount(), 700);
        assert_eq!(receiver.amount(), 800);

        // Cannot transfer more than available
        assert!(sender.transfer_out(1000).is_err());
    }

    #[test]
    fn test_maple_balance_net_position() {
        let mut balance = MapleBalance::new(1000);

        balance.earn(500); // +500
        balance.spend(200).unwrap(); // -200
        balance.receive(100); // +100
        balance.transfer_out(300).unwrap(); // -300

        // Initial 1000, earned 500, spent 200, received 100, transferred 300
        // Net should be: (1000 + 500 + 100) - (200 + 300) = 1100
        assert_eq!(balance.net_position(), 1100);
    }

    #[test]
    fn test_attention_budget_consume() {
        let mut budget = AttentionBudget::new(1000, 1000);

        assert!(budget.has_sufficient(500));
        budget.consume(500).unwrap();
        assert_eq!(budget.available, 500);
        assert_eq!(budget.used_this_epoch, 500);

        // Cannot consume more than available
        assert!(budget.consume(600).is_err());
    }

    #[test]
    fn test_attention_budget_give_receive() {
        let mut giver = AttentionBudget::new(1000, 1000);
        let mut receiver = AttentionBudget::new(500, 1000);

        let amount = giver.give(200).unwrap();
        receiver.receive_attention(amount);

        assert_eq!(giver.available, 800);
        assert_eq!(giver.given, 200);
        assert_eq!(receiver.available, 700);
        assert_eq!(receiver.received, 200);

        // Cannot give more than available
        assert!(giver.give(1000).is_err());
    }

    #[test]
    fn test_attention_regeneration() {
        let mut budget = AttentionBudget::new(200, 1000);
        budget.used_this_epoch = 800;
        budget.last_regeneration_epoch = 1;

        // Regenerate for epoch 2
        budget.regenerate(2, 0.5, 100);

        // Carryover: min(200, 100) = 100
        // Regeneration: 1000 * 0.5 = 500
        // New available: min(100 + 500, 1000) = 600
        assert_eq!(budget.available, 600);
        assert_eq!(budget.used_this_epoch, 0);
        assert_eq!(budget.last_regeneration_epoch, 2);

        // Same epoch regeneration does nothing
        budget.available = 300;
        budget.regenerate(2, 0.5, 100);
        assert_eq!(budget.available, 300);
    }

    #[test]
    fn test_attention_utilization() {
        let mut budget = AttentionBudget::new(1000, 1000);
        assert_eq!(budget.utilization(), 0.0);

        budget.consume(500).unwrap();
        assert_eq!(budget.utilization(), 0.5);

        budget.consume(500).unwrap();
        assert_eq!(budget.utilization(), 1.0);
    }

    #[test]
    fn test_attention_efficiency() {
        let mut budget = AttentionBudget::new(1000, 1000);

        budget.consume(400).unwrap();
        budget.give(100).unwrap();

        // Used: 400, Given: 100, Total: 500
        // Total available: 1000 + 0 (received) = 1000
        // Efficiency: 500/1000 = 0.5
        assert_eq!(budget.efficiency_score(), 0.5);
    }

    #[test]
    fn test_transfer_record() {
        use crate::entity::EntityId;

        let record = TransferRecord {
            transfer_id: "tx-123".to_string(),
            from: EntityId::new("sender"),
            to: EntityId::new("receiver"),
            amount: Amount::new(100),
            transfer_type: TransferType::Direct,
            fee: Amount::new(1),
            timestamp: chrono::Utc::now(),
            reference: Some("commitment-456".to_string()),
        };

        let json = serde_json::to_string(&record).unwrap();
        let deserialized: TransferRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record.transfer_id, deserialized.transfer_id);
    }

    #[test]
    fn test_transfer_types() {
        let types = vec![
            TransferType::Direct,
            TransferType::Payment,
            TransferType::Reward,
            TransferType::Fee,
            TransferType::Attention,
            TransferType::Treasury,
        ];

        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let _: TransferType = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_economic_summary() {
        let summary = EconomicSummary {
            total_maple_supply: 1_000_000,
            total_attention_available: 500_000,
            transfers_this_epoch: 10_000,
            volume_this_epoch: 500_000,
            avg_transaction_size: 50.0,
            velocity: 2.5,
        };

        let json = serde_json::to_string(&summary).unwrap();
        let _: EconomicSummary = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_zero_capacity_edge_cases() {
        let budget = AttentionBudget::new(0, 0);
        assert_eq!(budget.utilization(), 0.0);
        assert_eq!(budget.efficiency_score(), 0.0);
    }
}
