//! Entity registry for MapleVerse
//!
//! **CRITICAL**: This module enforces the NO HUMAN PROFILES invariant.
//! Every entity registration goes through validation that rejects human profiles.

use crate::errors::{WorldError, WorldResult};
use mapleverse_types::config::MapleVerseConfig;
use mapleverse_types::economy::{AttentionBudget, MapleBalance};
use mapleverse_types::entity::{EntityId, EntityKind, MapleVerseEntity};
use mapleverse_types::errors::MapleVerseError;
use mapleverse_types::world::RegionId;
use std::collections::HashMap;

/// Registry of all entities in the MapleVerse
///
/// # NO HUMAN PROFILES
///
/// This registry **REJECTS** any attempt to register human profiles.
/// This is a runtime enforced invariant, not a configuration option.
pub struct EntityRegistry {
    /// All registered entities
    entities: HashMap<EntityId, MapleVerseEntity>,
    /// Configuration reference
    config: MapleVerseConfig,
    /// Total entities created (including terminated)
    total_created: u64,
    /// Total entities terminated
    total_terminated: u64,
}

impl EntityRegistry {
    /// Create a new entity registry
    pub fn new(config: MapleVerseConfig) -> Self {
        Self {
            entities: HashMap::new(),
            config,
            total_created: 0,
            total_terminated: 0,
        }
    }

    /// Register a new entity
    ///
    /// # NO HUMAN PROFILES
    ///
    /// This method will **REJECT** any entity with `EntityKind::Human`.
    pub fn register(&mut self, mut entity: MapleVerseEntity) -> WorldResult<EntityId> {
        // CRITICAL: Validate entity is not human
        self.validate_not_human(&entity)?;

        // Check for duplicates
        if self.entities.contains_key(&entity.id) {
            return Err(WorldError::RegistrationFailed {
                entity_id: entity.id.clone(),
                reason: "Entity ID already exists".to_string(),
            });
        }

        // Set initial balances from config
        entity.maple_balance = MapleBalance::new(self.config.economy_config.initial_maple_balance);
        entity.attention_budget = AttentionBudget::new(
            self.config.attention_config.base_attention_per_epoch,
            self.config.attention_config.base_attention_per_epoch,
        );

        let entity_id = entity.id.clone();
        self.entities.insert(entity_id.clone(), entity);
        self.total_created += 1;

        Ok(entity_id)
    }

    /// Register a new individual agent
    pub fn register_individual(
        &mut self,
        name: impl Into<String>,
        region_id: RegionId,
    ) -> WorldResult<EntityId> {
        let entity = MapleVerseEntity::new_individual(name, region_id, None);
        self.register(entity)
    }

    /// Register a new collective
    pub fn register_collective(
        &mut self,
        name: impl Into<String>,
        region_id: RegionId,
        founder_id: EntityId,
    ) -> WorldResult<EntityId> {
        // Verify founder exists and is an individual
        let founder = self.get(&founder_id)?;
        if !founder.is_individual() {
            return Err(WorldError::RegistrationFailed {
                entity_id: EntityId::generate(),
                reason: "Collective founder must be an individual agent".to_string(),
            });
        }

        let entity = MapleVerseEntity::new_collective(name, region_id, None, founder_id);
        self.register(entity)
    }

    /// **REJECTED**: Attempt to register a human
    ///
    /// This method exists to provide a clear error when human registration is attempted.
    /// It will ALWAYS fail.
    pub fn register_human(&self, name: impl Into<String>) -> WorldResult<EntityId> {
        Err(WorldError::Types(MapleVerseError::human_rejected(
            name.into(),
            "EntityRegistry::register_human() is not allowed",
        )))
    }

    /// Validate that an entity is not human
    fn validate_not_human(&self, entity: &MapleVerseEntity) -> WorldResult<()> {
        if matches!(entity.kind, EntityKind::Human) {
            return Err(WorldError::Types(MapleVerseError::human_rejected(
                entity.id.to_string(),
                "Cannot register human profiles in MapleVerse",
            )));
        }

        // Also check via the entity's validation
        entity.validate().map_err(WorldError::Types)?;

        Ok(())
    }

    /// Get an entity by ID
    pub fn get(&self, entity_id: &EntityId) -> WorldResult<&MapleVerseEntity> {
        self.entities
            .get(entity_id)
            .ok_or_else(|| WorldError::EntityNotFound(entity_id.clone()))
    }

    /// Get a mutable entity by ID
    pub fn get_mut(&mut self, entity_id: &EntityId) -> WorldResult<&mut MapleVerseEntity> {
        self.entities
            .get_mut(entity_id)
            .ok_or_else(|| WorldError::EntityNotFound(entity_id.clone()))
    }

    /// Check if an entity exists
    pub fn exists(&self, entity_id: &EntityId) -> bool {
        self.entities.contains_key(entity_id)
    }

    /// Terminate an entity
    pub fn terminate(
        &mut self,
        entity_id: &EntityId,
        reason: impl Into<String>,
    ) -> WorldResult<()> {
        let entity = self.get_mut(entity_id)?;
        entity.status = mapleverse_types::entity::EntityStatus::Terminated;
        self.total_terminated += 1;
        let _ = reason.into(); // Log reason in production
        Ok(())
    }

    /// Get all entity IDs
    pub fn entity_ids(&self) -> impl Iterator<Item = &EntityId> {
        self.entities.keys()
    }

    /// Get all entities
    pub fn entities(&self) -> impl Iterator<Item = &MapleVerseEntity> {
        self.entities.values()
    }

    /// Get mutable iterator over entities
    pub fn entities_mut(&mut self) -> impl Iterator<Item = &mut MapleVerseEntity> {
        self.entities.values_mut()
    }

    /// Get count of active entities
    pub fn active_count(&self) -> usize {
        self.entities
            .values()
            .filter(|e| e.status == mapleverse_types::entity::EntityStatus::Active)
            .count()
    }

    /// Get total entity count
    pub fn total_count(&self) -> usize {
        self.entities.len()
    }

    /// Get entities in a specific region
    pub fn entities_in_region(&self, region_id: &RegionId) -> Vec<&MapleVerseEntity> {
        self.entities
            .values()
            .filter(|e| &e.region_id == region_id)
            .collect()
    }

    /// Get count of entities by kind
    pub fn count_by_kind(&self, kind: &EntityKind) -> usize {
        self.entities.values().filter(|e| &e.kind == kind).count()
    }

    /// Get statistics
    pub fn stats(&self) -> RegistryStats {
        RegistryStats {
            total_entities: self.entities.len() as u64,
            active_entities: self.active_count() as u64,
            individuals: self.count_by_kind(&EntityKind::Individual) as u64,
            collectives: self.count_by_kind(&EntityKind::Collective) as u64,
            total_created: self.total_created,
            total_terminated: self.total_terminated,
        }
    }
}

/// Statistics about the entity registry
#[derive(Clone, Debug, Default)]
pub struct RegistryStats {
    /// Total entities in registry
    pub total_entities: u64,
    /// Active entities
    pub active_entities: u64,
    /// Individual agents
    pub individuals: u64,
    /// Collectives
    pub collectives: u64,
    /// Total entities ever created
    pub total_created: u64,
    /// Total entities terminated
    pub total_terminated: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> MapleVerseConfig {
        MapleVerseConfig::test_world()
    }

    #[test]
    fn test_register_individual() {
        let mut registry = EntityRegistry::new(test_config());

        let entity_id = registry
            .register_individual("TestAgent", RegionId::new("region-1"))
            .unwrap();

        assert!(registry.exists(&entity_id));
        let entity = registry.get(&entity_id).unwrap();
        assert!(entity.is_individual());
        assert_eq!(entity.name, "TestAgent");
    }

    #[test]
    fn test_register_collective() {
        let mut registry = EntityRegistry::new(test_config());

        let founder_id = registry
            .register_individual("Founder", RegionId::new("region-1"))
            .unwrap();

        let collective_id = registry
            .register_collective(
                "TestCollective",
                RegionId::new("region-1"),
                founder_id.clone(),
            )
            .unwrap();

        let collective = registry.get(&collective_id).unwrap();
        assert!(collective.is_collective());
    }

    #[test]
    fn test_human_registration_rejected() {
        let registry = EntityRegistry::new(test_config());

        let result = registry.register_human("HumanUser");
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.is_critical_violation());
    }

    #[test]
    fn test_human_entity_rejected() {
        // Try to sneak a human entity through direct registration
        let result = MapleVerseEntity::new_human("SneakyHuman", "human-1");
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_registration() {
        let mut registry = EntityRegistry::new(test_config());

        let entity = MapleVerseEntity::new_individual("Agent", RegionId::new("region-1"), None);

        registry.register(entity.clone()).unwrap();

        // Try to register same entity again
        let result = registry.register(entity);
        assert!(result.is_err());
    }

    #[test]
    fn test_terminate_entity() {
        let mut registry = EntityRegistry::new(test_config());

        let entity_id = registry
            .register_individual("ToTerminate", RegionId::new("region-1"))
            .unwrap();

        registry.terminate(&entity_id, "Test termination").unwrap();

        let entity = registry.get(&entity_id).unwrap();
        assert_eq!(
            entity.status,
            mapleverse_types::entity::EntityStatus::Terminated
        );
    }

    #[test]
    fn test_registry_stats() {
        let mut registry = EntityRegistry::new(test_config());

        for i in 0..5 {
            registry
                .register_individual(format!("Agent{}", i), RegionId::new("region-1"))
                .unwrap();
        }

        let founder = registry.entity_ids().next().cloned().unwrap();
        registry
            .register_collective("Collective1", RegionId::new("region-1"), founder)
            .unwrap();

        let stats = registry.stats();
        assert_eq!(stats.total_entities, 6);
        assert_eq!(stats.individuals, 5);
        assert_eq!(stats.collectives, 1);
        assert_eq!(stats.total_created, 6);
        assert_eq!(stats.total_terminated, 0);
    }

    #[test]
    fn test_entities_in_region() {
        let mut registry = EntityRegistry::new(test_config());

        for i in 0..3 {
            registry
                .register_individual(format!("AgentA{}", i), RegionId::new("region-a"))
                .unwrap();
        }

        for i in 0..2 {
            registry
                .register_individual(format!("AgentB{}", i), RegionId::new("region-b"))
                .unwrap();
        }

        let region_a_entities = registry.entities_in_region(&RegionId::new("region-a"));
        assert_eq!(region_a_entities.len(), 3);

        let region_b_entities = registry.entities_in_region(&RegionId::new("region-b"));
        assert_eq!(region_b_entities.len(), 2);
    }

    #[test]
    fn test_initial_balances() {
        let config = MapleVerseConfig::test_world();
        let initial_maple = config.economy_config.initial_maple_balance;
        let initial_attention = config.attention_config.base_attention_per_epoch;

        let mut registry = EntityRegistry::new(config);

        let entity_id = registry
            .register_individual("Agent", RegionId::new("region-1"))
            .unwrap();

        let entity = registry.get(&entity_id).unwrap();
        assert_eq!(entity.maple_balance.amount(), initial_maple);
        assert_eq!(entity.attention_budget.available, initial_attention);
    }
}
