//! Attention manager for MapleVerse
//!
//! Manages the attention economy:
//! - Attention is **scarce**: Limited per epoch
//! - Attention **regenerates**: Refills each epoch
//! - Attention is **tradeable**: Can be transferred between entities
//! - Attention **bounds action**: Operations require attention

use crate::errors::{WorldError, WorldResult};
use mapleverse_types::config::MapleVerseConfig;
use mapleverse_types::entity::EntityId;
use mapleverse_types::event::EpochId;
use std::collections::HashMap;

/// Manager for attention resources
pub struct AttentionManager {
    /// Configuration
    config: MapleVerseConfig,
    /// Current epoch
    current_epoch: EpochId,
    /// Attention state per entity
    attention_state: HashMap<EntityId, AttentionState>,
    /// Total attention consumed this epoch
    epoch_consumed: u64,
    /// Total attention transferred this epoch
    epoch_transferred: u64,
}

impl AttentionManager {
    /// Create a new attention manager
    pub fn new(config: MapleVerseConfig) -> Self {
        Self {
            config,
            current_epoch: EpochId::new(0),
            attention_state: HashMap::new(),
            epoch_consumed: 0,
            epoch_transferred: 0,
        }
    }

    /// Initialize attention for a new entity
    pub fn initialize_entity(&mut self, entity_id: EntityId) {
        let state = AttentionState {
            available: self.config.attention_config.base_attention_per_epoch,
            capacity: self.config.attention_config.base_attention_per_epoch,
            used_this_epoch: 0,
            received_this_epoch: 0,
            given_this_epoch: 0,
            last_regeneration_epoch: self.current_epoch,
        };
        self.attention_state.insert(entity_id, state);
    }

    /// Get available attention for an entity
    pub fn get_available(&self, entity_id: &EntityId) -> u64 {
        self.attention_state
            .get(entity_id)
            .map(|s| s.available)
            .unwrap_or(0)
    }

    /// Check if entity has enough attention
    pub fn has_sufficient(&self, entity_id: &EntityId, required: u64) -> bool {
        self.get_available(entity_id) >= required
    }

    /// Consume attention for an action
    pub fn consume(&mut self, entity_id: &EntityId, amount: u64) -> WorldResult<()> {
        let state = self
            .attention_state
            .get_mut(entity_id)
            .ok_or_else(|| WorldError::EntityNotFound(entity_id.clone()))?;

        if state.available < amount {
            return Err(WorldError::Types(
                mapleverse_types::errors::MapleVerseError::InsufficientAttention {
                    required: amount,
                    available: state.available,
                },
            ));
        }

        state.available -= amount;
        state.used_this_epoch += amount;
        self.epoch_consumed += amount;

        Ok(())
    }

    /// Transfer attention from one entity to another
    pub fn transfer(&mut self, from: &EntityId, to: &EntityId, amount: u64) -> WorldResult<()> {
        // Validate sender has enough
        {
            let from_state = self
                .attention_state
                .get(from)
                .ok_or_else(|| WorldError::EntityNotFound(from.clone()))?;

            if from_state.available < amount {
                return Err(WorldError::Types(
                    mapleverse_types::errors::MapleVerseError::InsufficientAttention {
                        required: amount,
                        available: from_state.available,
                    },
                ));
            }
        }

        // Check recipient exists
        if !self.attention_state.contains_key(to) {
            return Err(WorldError::EntityNotFound(to.clone()));
        }

        // Perform transfer
        {
            let from_state = self.attention_state.get_mut(from).unwrap();
            from_state.available -= amount;
            from_state.given_this_epoch += amount;
        }

        {
            let to_state = self.attention_state.get_mut(to).unwrap();
            to_state.available += amount; // Can exceed capacity when received
            to_state.received_this_epoch += amount;
        }

        self.epoch_transferred += amount;

        Ok(())
    }

    /// Regenerate attention for all entities at epoch boundary
    pub fn regenerate_all(&mut self, new_epoch: EpochId) -> RegenerationSummary {
        let regeneration_rate = self.config.attention_config.attention_regeneration_rate;
        let max_carryover = self.config.attention_config.max_attention_carryover;

        let mut total_regenerated = 0u64;
        let mut entities_regenerated = 0u64;

        for (_, state) in self.attention_state.iter_mut() {
            if new_epoch > state.last_regeneration_epoch {
                let carryover = state.available.min(max_carryover);
                let base_regen = (state.capacity as f64 * regeneration_rate) as u64;
                let new_available = (carryover + base_regen).min(state.capacity);

                let regenerated = new_available.saturating_sub(state.available);
                total_regenerated += regenerated;

                state.available = new_available;
                state.used_this_epoch = 0;
                state.received_this_epoch = 0;
                state.given_this_epoch = 0;
                state.last_regeneration_epoch = new_epoch;

                entities_regenerated += 1;
            }
        }

        self.current_epoch = new_epoch;

        // Reset epoch counters
        let summary = RegenerationSummary {
            epoch: new_epoch,
            entities_regenerated,
            total_regenerated,
            previous_epoch_consumed: self.epoch_consumed,
            previous_epoch_transferred: self.epoch_transferred,
        };

        self.epoch_consumed = 0;
        self.epoch_transferred = 0;

        summary
    }

    /// Get attention state for an entity
    pub fn get_state(&self, entity_id: &EntityId) -> Option<&AttentionState> {
        self.attention_state.get(entity_id)
    }

    /// Get current epoch
    pub fn current_epoch(&self) -> EpochId {
        self.current_epoch
    }

    /// Get statistics
    pub fn stats(&self) -> AttentionStats {
        let total_available: u64 = self.attention_state.values().map(|s| s.available).sum();
        let total_capacity: u64 = self.attention_state.values().map(|s| s.capacity).sum();

        AttentionStats {
            entity_count: self.attention_state.len() as u64,
            total_available,
            total_capacity,
            utilization: if total_capacity > 0 {
                1.0 - (total_available as f64 / total_capacity as f64)
            } else {
                0.0
            },
            epoch_consumed: self.epoch_consumed,
            epoch_transferred: self.epoch_transferred,
        }
    }

    /// Remove entity from tracking
    pub fn remove_entity(&mut self, entity_id: &EntityId) {
        self.attention_state.remove(entity_id);
    }
}

/// Attention state for a single entity
#[derive(Clone, Debug)]
pub struct AttentionState {
    /// Available attention units
    pub available: u64,
    /// Maximum capacity
    pub capacity: u64,
    /// Used this epoch
    pub used_this_epoch: u64,
    /// Received from others this epoch
    pub received_this_epoch: u64,
    /// Given to others this epoch
    pub given_this_epoch: u64,
    /// Last regeneration epoch
    pub last_regeneration_epoch: EpochId,
}

/// Summary of attention regeneration
#[derive(Clone, Debug)]
pub struct RegenerationSummary {
    /// New epoch
    pub epoch: EpochId,
    /// Number of entities that regenerated
    pub entities_regenerated: u64,
    /// Total attention regenerated
    pub total_regenerated: u64,
    /// Attention consumed in previous epoch
    pub previous_epoch_consumed: u64,
    /// Attention transferred in previous epoch
    pub previous_epoch_transferred: u64,
}

/// Attention statistics
#[derive(Clone, Debug, Default)]
pub struct AttentionStats {
    /// Number of tracked entities
    pub entity_count: u64,
    /// Total available attention
    pub total_available: u64,
    /// Total capacity
    pub total_capacity: u64,
    /// Current utilization (0.0 to 1.0)
    pub utilization: f64,
    /// Attention consumed this epoch
    pub epoch_consumed: u64,
    /// Attention transferred this epoch
    pub epoch_transferred: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> MapleVerseConfig {
        let mut config = MapleVerseConfig::test_world();
        config.attention_config.base_attention_per_epoch = 1000;
        config.attention_config.attention_regeneration_rate = 0.5;
        config.attention_config.max_attention_carryover = 200;
        config
    }

    #[test]
    fn test_initialize_entity() {
        let config = test_config();
        let base_attention = config.attention_config.base_attention_per_epoch;

        let mut manager = AttentionManager::new(config);
        manager.initialize_entity(EntityId::new("agent-1"));

        assert_eq!(
            manager.get_available(&EntityId::new("agent-1")),
            base_attention
        );
    }

    #[test]
    fn test_consume_attention() {
        let mut manager = AttentionManager::new(test_config());
        let entity = EntityId::new("agent-1");
        manager.initialize_entity(entity.clone());

        manager.consume(&entity, 300).unwrap();

        assert_eq!(manager.get_available(&entity), 700);
        assert_eq!(manager.stats().epoch_consumed, 300);
    }

    #[test]
    fn test_consume_insufficient() {
        let mut manager = AttentionManager::new(test_config());
        let entity = EntityId::new("agent-1");
        manager.initialize_entity(entity.clone());

        let result = manager.consume(&entity, 2000);
        assert!(result.is_err());
    }

    #[test]
    fn test_transfer_attention() {
        let mut manager = AttentionManager::new(test_config());

        let sender = EntityId::new("sender");
        let receiver = EntityId::new("receiver");

        manager.initialize_entity(sender.clone());
        manager.initialize_entity(receiver.clone());

        manager.transfer(&sender, &receiver, 200).unwrap();

        assert_eq!(manager.get_available(&sender), 800);
        assert_eq!(manager.get_available(&receiver), 1200); // Can exceed capacity

        let sender_state = manager.get_state(&sender).unwrap();
        assert_eq!(sender_state.given_this_epoch, 200);

        let receiver_state = manager.get_state(&receiver).unwrap();
        assert_eq!(receiver_state.received_this_epoch, 200);
    }

    #[test]
    fn test_regenerate_all() {
        let config = test_config();
        // base = 1000, regen rate = 0.5, max carryover = 200
        let mut manager = AttentionManager::new(config);

        let entity = EntityId::new("agent-1");
        manager.initialize_entity(entity.clone());

        // Use some attention
        manager.consume(&entity, 600).unwrap(); // 400 remaining
        assert_eq!(manager.get_available(&entity), 400);

        // Regenerate
        let summary = manager.regenerate_all(EpochId::new(1));

        // Carryover = min(400, 200) = 200
        // Regeneration = 1000 * 0.5 = 500
        // New available = min(200 + 500, 1000) = 700
        assert_eq!(manager.get_available(&entity), 700);
        assert_eq!(summary.entities_regenerated, 1);
        assert_eq!(summary.previous_epoch_consumed, 600);
    }

    #[test]
    fn test_stats() {
        let config = test_config();
        let base = config.attention_config.base_attention_per_epoch;

        let mut manager = AttentionManager::new(config);

        for i in 0..5 {
            manager.initialize_entity(EntityId::new(format!("agent-{}", i)));
        }

        let stats = manager.stats();
        assert_eq!(stats.entity_count, 5);
        assert_eq!(stats.total_available, base * 5);
        assert_eq!(stats.total_capacity, base * 5);
        assert_eq!(stats.utilization, 0.0);
    }

    #[test]
    fn test_has_sufficient() {
        let mut manager = AttentionManager::new(test_config());
        let entity = EntityId::new("agent-1");
        manager.initialize_entity(entity.clone());

        assert!(manager.has_sufficient(&entity, 500));
        assert!(manager.has_sufficient(&entity, 1000));
        assert!(!manager.has_sufficient(&entity, 1001));
    }

    #[test]
    fn test_remove_entity() {
        let mut manager = AttentionManager::new(test_config());
        let entity = EntityId::new("agent-1");
        manager.initialize_entity(entity.clone());

        assert!(manager.get_state(&entity).is_some());

        manager.remove_entity(&entity);
        assert!(manager.get_state(&entity).is_none());
    }

    #[test]
    fn test_transfer_to_nonexistent() {
        let mut manager = AttentionManager::new(test_config());
        let sender = EntityId::new("sender");
        manager.initialize_entity(sender.clone());

        let result = manager.transfer(&sender, &EntityId::new("nonexistent"), 100);
        assert!(result.is_err());
    }
}
