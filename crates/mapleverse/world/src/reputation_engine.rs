//! Reputation engine for MapleVerse
//!
//! **CRITICAL INVARIANT**: Reputation comes ONLY from verified receipts.
//!
//! This engine enforces that ALL reputation changes must be backed by receipts.
//! There is no mechanism to modify reputation without a receipt.

use crate::errors::{WorldError, WorldResult};
use mapleverse_types::config::MapleVerseConfig;
use mapleverse_types::entity::EntityId;
use mapleverse_types::errors::MapleVerseError;
use mapleverse_types::event::EpochId;
use mapleverse_types::reputation::{ReputationReceipt, ReputationReceiptId, ReputationScore};
use std::collections::HashMap;

/// Engine for managing receipt-based reputation
///
/// # CRITICAL: Receipts Only
///
/// This engine does NOT allow reputation modifications without receipts.
/// Every reputation change is backed by a verified receipt.
pub struct ReputationEngine {
    /// Configuration
    config: MapleVerseConfig,
    /// Reputation scores per entity
    scores: HashMap<EntityId, ReputationScore>,
    /// All reputation receipts (for audit)
    receipts: HashMap<ReputationReceiptId, ReputationReceipt>,
    /// Receipts by entity
    receipts_by_entity: HashMap<EntityId, Vec<ReputationReceiptId>>,
    /// Used receipt IDs (prevent double-counting)
    used_receipts: std::collections::HashSet<String>,
    /// Current epoch
    current_epoch: EpochId,
}

impl ReputationEngine {
    /// Create a new reputation engine
    pub fn new(config: MapleVerseConfig) -> Self {
        Self {
            config,
            scores: HashMap::new(),
            receipts: HashMap::new(),
            receipts_by_entity: HashMap::new(),
            used_receipts: std::collections::HashSet::new(),
            current_epoch: EpochId::new(0),
        }
    }

    /// Initialize reputation for a new entity
    pub fn initialize_entity(&mut self, entity_id: EntityId) {
        let score = ReputationScore::new(
            self.config.reputation_config.initial_reputation,
            self.config.reputation_config.min_reputation,
            self.config.reputation_config.max_reputation,
        );
        self.scores.insert(entity_id.clone(), score);
        self.receipts_by_entity.insert(entity_id, Vec::new());
    }

    /// Process a reputation receipt
    ///
    /// This is the ONLY way to modify reputation.
    pub fn process_receipt(&mut self, receipt: ReputationReceipt) -> WorldResult<()> {
        // Validate receipt
        receipt.validate().map_err(WorldError::Types)?;

        // Check for duplicate receipt
        let source_receipt_id = receipt.source.receipt_id().to_string();
        if self.used_receipts.contains(&source_receipt_id) {
            return Err(WorldError::Types(MapleVerseError::ReceiptAlreadyUsed {
                receipt_id: source_receipt_id,
            }));
        }

        // Get or create entity score
        let entity_id = receipt.entity_id.clone();
        if !self.scores.contains_key(&entity_id) {
            self.initialize_entity(entity_id.clone());
        }

        // Apply reputation change
        let score = self.scores.get_mut(&entity_id).unwrap();
        let weighted_change = (receipt.reputation_change as f64
            * self.config.reputation_config.receipt_reputation_weight)
            as i64;

        let adjusted_receipt = ReputationReceipt {
            reputation_change: weighted_change,
            ..receipt.clone()
        };

        score.apply_receipt(&adjusted_receipt, self.current_epoch.number());

        // Store receipt
        self.used_receipts.insert(source_receipt_id);
        let receipt_id = receipt.id.clone();
        self.receipts_by_entity
            .entry(entity_id)
            .or_default()
            .push(receipt_id.clone());
        self.receipts.insert(receipt_id, receipt);

        Ok(())
    }

    /// **REJECTED**: Attempt to modify reputation without receipt
    ///
    /// This method exists to provide a clear error when non-receipt reputation
    /// modification is attempted. It will ALWAYS fail.
    pub fn modify_without_receipt(
        &self,
        _entity_id: &EntityId,
        _amount: i64,
        source: &str,
    ) -> WorldResult<()> {
        Err(WorldError::Types(
            MapleVerseError::InvalidReputationSource {
                attempted_source: source.to_string(),
            },
        ))
    }

    /// Get reputation score for an entity
    pub fn get_score(&self, entity_id: &EntityId) -> Option<&ReputationScore> {
        self.scores.get(entity_id)
    }

    /// Get reputation value for an entity
    pub fn get_reputation(&self, entity_id: &EntityId) -> i64 {
        self.scores.get(entity_id).map(|s| s.score()).unwrap_or(0)
    }

    /// Get all receipts for an entity
    pub fn get_entity_receipts(&self, entity_id: &EntityId) -> Vec<&ReputationReceipt> {
        self.receipts_by_entity
            .get(entity_id)
            .map(|ids| ids.iter().filter_map(|id| self.receipts.get(id)).collect())
            .unwrap_or_default()
    }

    /// Apply reputation decay at epoch boundary
    pub fn apply_decay(&mut self, new_epoch: EpochId) -> DecaySummary {
        let decay_rate = self.config.reputation_config.reputation_decay_per_epoch;
        let mut total_decayed = 0i64;
        let mut entities_decayed = 0u64;

        for (_, score) in self.scores.iter_mut() {
            let before = score.score();
            score.apply_decay(decay_rate, new_epoch.number());
            let after = score.score();

            let decayed = before - after;
            if decayed != 0 {
                total_decayed += decayed.abs();
                entities_decayed += 1;
            }
        }

        self.current_epoch = new_epoch;

        DecaySummary {
            epoch: new_epoch,
            entities_decayed,
            total_decayed,
        }
    }

    /// Check if entity meets reputation threshold
    pub fn meets_threshold(&self, entity_id: &EntityId, threshold: i64) -> bool {
        self.scores
            .get(entity_id)
            .map(|s| s.meets_threshold(threshold))
            .unwrap_or(false)
    }

    /// Get statistics
    pub fn stats(&self) -> ReputationStats {
        let scores: Vec<i64> = self.scores.values().map(|s| s.score()).collect();

        let avg_reputation = if !scores.is_empty() {
            scores.iter().sum::<i64>() as f64 / scores.len() as f64
        } else {
            0.0
        };

        let positive_count = scores.iter().filter(|&&s| s > 0).count() as u64;
        let negative_count = scores.iter().filter(|&&s| s < 0).count() as u64;

        ReputationStats {
            entity_count: self.scores.len() as u64,
            total_receipts: self.receipts.len() as u64,
            avg_reputation,
            positive_count,
            negative_count,
            neutral_count: self.scores.len() as u64 - positive_count - negative_count,
        }
    }

    /// Remove entity from tracking
    pub fn remove_entity(&mut self, entity_id: &EntityId) {
        self.scores.remove(entity_id);
        self.receipts_by_entity.remove(entity_id);
    }

    /// Get receipt by ID
    pub fn get_receipt(&self, receipt_id: &ReputationReceiptId) -> Option<&ReputationReceipt> {
        self.receipts.get(receipt_id)
    }

    /// Check if a source receipt has been used
    pub fn is_receipt_used(&self, source_receipt_id: &str) -> bool {
        self.used_receipts.contains(source_receipt_id)
    }
}

/// Summary of reputation decay
#[derive(Clone, Debug)]
pub struct DecaySummary {
    /// New epoch
    pub epoch: EpochId,
    /// Number of entities that had decay applied
    pub entities_decayed: u64,
    /// Total reputation decayed
    pub total_decayed: i64,
}

/// Reputation statistics
#[derive(Clone, Debug, Default)]
pub struct ReputationStats {
    /// Number of tracked entities
    pub entity_count: u64,
    /// Total receipts processed
    pub total_receipts: u64,
    /// Average reputation
    pub avg_reputation: f64,
    /// Entities with positive reputation
    pub positive_count: u64,
    /// Entities with negative reputation
    pub negative_count: u64,
    /// Entities with neutral (0) reputation
    pub neutral_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mapleverse_types::reputation::ReputationSource;

    fn test_config() -> MapleVerseConfig {
        let mut config = MapleVerseConfig::test_world();
        config.reputation_config.initial_reputation = 0;
        config.reputation_config.receipt_reputation_weight = 1.0;
        config.reputation_config.reputation_decay_per_epoch = 0.1;
        config
    }

    fn make_receipt(entity_id: EntityId, change: i64) -> ReputationReceipt {
        ReputationReceipt::new(
            entity_id,
            ReputationSource::CommitmentReceipt {
                receipt_id: format!("receipt-{}", uuid::Uuid::new_v4()),
                commitment_id: "commit-1".to_string(),
            },
            change,
            "Test receipt",
            1,
        )
    }

    #[test]
    fn test_initialize_entity() {
        let mut engine = ReputationEngine::new(test_config());

        engine.initialize_entity(EntityId::new("agent-1"));

        assert_eq!(engine.get_reputation(&EntityId::new("agent-1")), 0);
    }

    #[test]
    fn test_process_receipt() {
        let mut engine = ReputationEngine::new(test_config());

        let entity = EntityId::new("agent-1");
        engine.initialize_entity(entity.clone());

        let receipt = make_receipt(entity.clone(), 100);
        engine.process_receipt(receipt).unwrap();

        assert_eq!(engine.get_reputation(&entity), 100);
    }

    #[test]
    fn test_duplicate_receipt_rejected() {
        let mut engine = ReputationEngine::new(test_config());

        let entity = EntityId::new("agent-1");
        engine.initialize_entity(entity.clone());

        let receipt = ReputationReceipt::new(
            entity.clone(),
            ReputationSource::CommitmentReceipt {
                receipt_id: "same-receipt".to_string(),
                commitment_id: "commit-1".to_string(),
            },
            100,
            "Test",
            1,
        );

        engine.process_receipt(receipt.clone()).unwrap();

        // Second receipt with same source should fail
        let result = engine.process_receipt(receipt);
        assert!(result.is_err());
    }

    #[test]
    fn test_modify_without_receipt_rejected() {
        let engine = ReputationEngine::new(test_config());

        let result =
            engine.modify_without_receipt(&EntityId::new("agent-1"), 100, "self-assessment");

        assert!(result.is_err());
    }

    #[test]
    fn test_apply_decay() {
        let mut engine = ReputationEngine::new(test_config());

        let entity = EntityId::new("agent-1");
        engine.initialize_entity(entity.clone());

        // Add positive reputation
        let receipt = make_receipt(entity.clone(), 100);
        engine.process_receipt(receipt).unwrap();

        // Apply 10% decay
        let summary = engine.apply_decay(EpochId::new(1));

        assert_eq!(engine.get_reputation(&entity), 90);
        assert_eq!(summary.entities_decayed, 1);
    }

    #[test]
    fn test_meets_threshold() {
        let mut engine = ReputationEngine::new(test_config());

        let entity = EntityId::new("agent-1");
        engine.initialize_entity(entity.clone());

        let receipt = make_receipt(entity.clone(), 50);
        engine.process_receipt(receipt).unwrap();

        assert!(engine.meets_threshold(&entity, 30));
        assert!(engine.meets_threshold(&entity, 50));
        assert!(!engine.meets_threshold(&entity, 100));
    }

    #[test]
    fn test_get_entity_receipts() {
        let mut engine = ReputationEngine::new(test_config());

        let entity = EntityId::new("agent-1");
        engine.initialize_entity(entity.clone());

        for i in 0..3 {
            let receipt = ReputationReceipt::new(
                entity.clone(),
                ReputationSource::CommitmentReceipt {
                    receipt_id: format!("receipt-{}", i),
                    commitment_id: format!("commit-{}", i),
                },
                10,
                "Test",
                1,
            );
            engine.process_receipt(receipt).unwrap();
        }

        let receipts = engine.get_entity_receipts(&entity);
        assert_eq!(receipts.len(), 3);
    }

    #[test]
    fn test_stats() {
        let mut engine = ReputationEngine::new(test_config());

        for i in 0..5 {
            let entity = EntityId::new(format!("agent-{}", i));
            engine.initialize_entity(entity.clone());

            // Give some positive, some negative reputation
            let change = if i < 3 { 50 } else { -50 };
            let receipt = make_receipt(entity, change);
            engine.process_receipt(receipt).unwrap();
        }

        let stats = engine.stats();
        assert_eq!(stats.entity_count, 5);
        assert_eq!(stats.positive_count, 3);
        assert_eq!(stats.negative_count, 2);
    }

    #[test]
    fn test_reputation_weight() {
        let mut config = test_config();
        config.reputation_config.receipt_reputation_weight = 0.5; // 50% weight

        let mut engine = ReputationEngine::new(config);

        let entity = EntityId::new("agent-1");
        engine.initialize_entity(entity.clone());

        let receipt = make_receipt(entity.clone(), 100);
        engine.process_receipt(receipt).unwrap();

        // Should be 100 * 0.5 = 50
        assert_eq!(engine.get_reputation(&entity), 50);
    }

    #[test]
    fn test_is_receipt_used() {
        let mut engine = ReputationEngine::new(test_config());

        let entity = EntityId::new("agent-1");
        engine.initialize_entity(entity.clone());

        let receipt = ReputationReceipt::new(
            entity,
            ReputationSource::CommitmentReceipt {
                receipt_id: "test-receipt-123".to_string(),
                commitment_id: "commit-1".to_string(),
            },
            100,
            "Test",
            1,
        );

        assert!(!engine.is_receipt_used("test-receipt-123"));

        engine.process_receipt(receipt).unwrap();

        assert!(engine.is_receipt_used("test-receipt-123"));
    }
}
