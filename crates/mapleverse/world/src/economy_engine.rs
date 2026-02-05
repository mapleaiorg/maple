//! Economy engine for MapleVerse
//!
//! Manages the dual-currency system:
//! - **MAPLE**: Primary value token
//! - **Attention**: Scarce resource that bounds action capacity

use crate::errors::{WorldError, WorldResult};
use mapleverse_types::config::MapleVerseConfig;
use mapleverse_types::economy::{Amount, TransferRecord, TransferType};
use mapleverse_types::entity::EntityId;
use std::collections::HashMap;
use uuid::Uuid;

/// Engine for managing MAPLE economy
pub struct EconomyEngine {
    /// Configuration
    config: MapleVerseConfig,
    /// Total MAPLE in circulation
    total_supply: u64,
    /// Transfer history
    transfers: Vec<TransferRecord>,
    /// Total volume transferred
    total_volume: u64,
    /// Transfer count
    transfer_count: u64,
}

impl EconomyEngine {
    /// Create a new economy engine
    pub fn new(config: MapleVerseConfig) -> Self {
        Self {
            config,
            total_supply: 0,
            transfers: Vec::new(),
            total_volume: 0,
            transfer_count: 0,
        }
    }

    /// Record initial MAPLE allocation for a new entity
    pub fn allocate_initial(&mut self, entity_id: EntityId) -> u64 {
        let amount = self.config.economy_config.initial_maple_balance;
        self.total_supply += amount;

        let record = TransferRecord {
            transfer_id: format!("alloc-{}", Uuid::new_v4()),
            from: EntityId::new("system"),
            to: entity_id,
            amount: Amount::new(amount),
            transfer_type: TransferType::Reward,
            fee: Amount::new(0),
            timestamp: chrono::Utc::now(),
            reference: Some("initial_allocation".to_string()),
        };

        self.transfers.push(record);
        amount
    }

    /// Record a MAPLE transfer between entities
    ///
    /// Note: The actual balance updates happen in the entities themselves.
    /// This engine records the transfer for auditing.
    pub fn record_transfer(
        &mut self,
        from: EntityId,
        to: EntityId,
        amount: u64,
        transfer_type: TransferType,
        reference: Option<String>,
    ) -> TransferId {
        let fee = self.calculate_fee(amount);

        let record = TransferRecord {
            transfer_id: format!("tx-{}", Uuid::new_v4()),
            from,
            to,
            amount: Amount::new(amount),
            transfer_type,
            fee: Amount::new(fee),
            timestamp: chrono::Utc::now(),
            reference,
        };

        let transfer_id = TransferId(record.transfer_id.clone());
        self.transfers.push(record);
        self.total_volume += amount;
        self.transfer_count += 1;

        transfer_id
    }

    /// Calculate transfer fee
    pub fn calculate_fee(&self, amount: u64) -> u64 {
        let fee_bps = self.config.economy_config.maple_transfer_fee_bps as u64;
        (amount * fee_bps) / 10000
    }

    /// Get transfer by ID
    pub fn get_transfer(&self, transfer_id: &str) -> Option<&TransferRecord> {
        self.transfers.iter().find(|t| t.transfer_id == transfer_id)
    }

    /// Get transfers for an entity
    pub fn get_entity_transfers(&self, entity_id: &EntityId) -> Vec<&TransferRecord> {
        self.transfers
            .iter()
            .filter(|t| &t.from == entity_id || &t.to == entity_id)
            .collect()
    }

    /// Get recent transfers
    pub fn recent_transfers(&self, limit: usize) -> Vec<&TransferRecord> {
        self.transfers.iter().rev().take(limit).collect()
    }

    /// Get total supply
    pub fn total_supply(&self) -> u64 {
        self.total_supply
    }

    /// Get total volume
    pub fn total_volume(&self) -> u64 {
        self.total_volume
    }

    /// Get transfer count
    pub fn transfer_count(&self) -> u64 {
        self.transfer_count
    }

    /// Get statistics
    pub fn stats(&self) -> EconomyStats {
        EconomyStats {
            total_supply: self.total_supply,
            total_volume: self.total_volume,
            transfer_count: self.transfer_count,
            avg_transfer_size: if self.transfer_count > 0 {
                self.total_volume as f64 / self.transfer_count as f64
            } else {
                0.0
            },
        }
    }

    /// Reset epoch statistics (called at epoch boundary)
    pub fn reset_epoch_stats(&mut self) -> EpochEconomyStats {
        let stats = EpochEconomyStats {
            volume: self.total_volume,
            transfer_count: self.transfer_count,
        };

        // Note: We don't reset total_volume and transfer_count as they're lifetime stats
        // For epoch-specific stats, we'd need separate counters

        stats
    }
}

/// Unique identifier for a transfer
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TransferId(pub String);

/// Economy statistics
#[derive(Clone, Debug, Default)]
pub struct EconomyStats {
    /// Total MAPLE in circulation
    pub total_supply: u64,
    /// Total volume transferred (lifetime)
    pub total_volume: u64,
    /// Total transfer count (lifetime)
    pub transfer_count: u64,
    /// Average transfer size
    pub avg_transfer_size: f64,
}

/// Epoch-specific economy statistics
#[derive(Clone, Debug, Default)]
pub struct EpochEconomyStats {
    /// Volume this epoch
    pub volume: u64,
    /// Transfers this epoch
    pub transfer_count: u64,
}

/// Manager for tracking balances (used with entity registry)
pub struct BalanceTracker {
    /// Entity balances (for quick lookup)
    balances: HashMap<EntityId, u64>,
}

impl BalanceTracker {
    /// Create a new balance tracker
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
        }
    }

    /// Set balance for an entity
    pub fn set_balance(&mut self, entity_id: EntityId, amount: u64) {
        self.balances.insert(entity_id, amount);
    }

    /// Get balance for an entity
    pub fn get_balance(&self, entity_id: &EntityId) -> u64 {
        self.balances.get(entity_id).copied().unwrap_or(0)
    }

    /// Update balance (add/subtract)
    pub fn update_balance(&mut self, entity_id: &EntityId, delta: i64) -> WorldResult<u64> {
        let current = self.get_balance(entity_id);
        let new_balance = if delta >= 0 {
            current.saturating_add(delta as u64)
        } else {
            let abs_delta = (-delta) as u64;
            if current < abs_delta {
                return Err(WorldError::Types(
                    mapleverse_types::errors::MapleVerseError::InsufficientMaple {
                        required: abs_delta,
                        available: current,
                    },
                ));
            }
            current - abs_delta
        };

        self.balances.insert(entity_id.clone(), new_balance);
        Ok(new_balance)
    }

    /// Get total across all tracked balances
    pub fn total(&self) -> u64 {
        self.balances.values().sum()
    }

    /// Get entity count
    pub fn entity_count(&self) -> usize {
        self.balances.len()
    }
}

impl Default for BalanceTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> MapleVerseConfig {
        let mut config = MapleVerseConfig::test_world();
        config.economy_config.maple_transfer_fee_bps = 10; // 0.1%
        config
    }

    #[test]
    fn test_allocate_initial() {
        let config = test_config();
        let initial_balance = config.economy_config.initial_maple_balance;

        let mut engine = EconomyEngine::new(config);

        let amount = engine.allocate_initial(EntityId::new("agent-1"));

        assert_eq!(amount, initial_balance);
        assert_eq!(engine.total_supply(), initial_balance);
    }

    #[test]
    fn test_record_transfer() {
        let mut engine = EconomyEngine::new(test_config());

        let transfer_id = engine.record_transfer(
            EntityId::new("sender"),
            EntityId::new("receiver"),
            1000,
            TransferType::Direct,
            Some("test transfer".to_string()),
        );

        assert!(engine.get_transfer(&transfer_id.0).is_some());
        assert_eq!(engine.total_volume(), 1000);
        assert_eq!(engine.transfer_count(), 1);
    }

    #[test]
    fn test_calculate_fee() {
        let engine = EconomyEngine::new(test_config());

        // 10 bps = 0.1%
        let fee = engine.calculate_fee(10000);
        assert_eq!(fee, 10); // 10000 * 10 / 10000 = 10
    }

    #[test]
    fn test_get_entity_transfers() {
        let mut engine = EconomyEngine::new(test_config());

        let sender = EntityId::new("sender");
        let receiver = EntityId::new("receiver");
        let other = EntityId::new("other");

        engine.record_transfer(
            sender.clone(),
            receiver.clone(),
            100,
            TransferType::Direct,
            None,
        );

        engine.record_transfer(
            other.clone(),
            receiver.clone(),
            50,
            TransferType::Direct,
            None,
        );

        let sender_transfers = engine.get_entity_transfers(&sender);
        assert_eq!(sender_transfers.len(), 1);

        let receiver_transfers = engine.get_entity_transfers(&receiver);
        assert_eq!(receiver_transfers.len(), 2);
    }

    #[test]
    fn test_stats() {
        let config = test_config();
        let initial = config.economy_config.initial_maple_balance;

        let mut engine = EconomyEngine::new(config);

        engine.allocate_initial(EntityId::new("agent-1"));
        engine.allocate_initial(EntityId::new("agent-2"));

        engine.record_transfer(
            EntityId::new("agent-1"),
            EntityId::new("agent-2"),
            500,
            TransferType::Direct,
            None,
        );

        let stats = engine.stats();
        assert_eq!(stats.total_supply, initial * 2);
        assert_eq!(stats.total_volume, 500);
        assert_eq!(stats.transfer_count, 1);
    }

    #[test]
    fn test_balance_tracker() {
        let mut tracker = BalanceTracker::new();

        tracker.set_balance(EntityId::new("agent-1"), 1000);
        assert_eq!(tracker.get_balance(&EntityId::new("agent-1")), 1000);

        tracker
            .update_balance(&EntityId::new("agent-1"), 500)
            .unwrap();
        assert_eq!(tracker.get_balance(&EntityId::new("agent-1")), 1500);

        tracker
            .update_balance(&EntityId::new("agent-1"), -300)
            .unwrap();
        assert_eq!(tracker.get_balance(&EntityId::new("agent-1")), 1200);
    }

    #[test]
    fn test_balance_tracker_insufficient() {
        let mut tracker = BalanceTracker::new();
        tracker.set_balance(EntityId::new("agent-1"), 100);

        let result = tracker.update_balance(&EntityId::new("agent-1"), -200);
        assert!(result.is_err());
    }

    #[test]
    fn test_recent_transfers() {
        let mut engine = EconomyEngine::new(test_config());

        for i in 0..10 {
            engine.record_transfer(
                EntityId::new("sender"),
                EntityId::new(format!("receiver-{}", i)),
                100,
                TransferType::Direct,
                None,
            );
        }

        let recent = engine.recent_transfers(5);
        assert_eq!(recent.len(), 5);
    }
}
