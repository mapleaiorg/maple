//! World state for MapleVerse
//!
//! The complete world state, orchestrating all subsystems.
//!
//! # NO HUMAN PROFILES
//!
//! Every registration path enforces the "no humans" invariant.

use crate::attention_manager::AttentionManager;
use crate::economy_engine::EconomyEngine;
use crate::entity_registry::EntityRegistry;
use crate::errors::{WorldError, WorldResult};
use crate::event_bus::EventBus;
use crate::region_manager::RegionManager;
use crate::reputation_engine::ReputationEngine;
use mapleverse_types::config::MapleVerseConfig;
use mapleverse_types::economy::TransferType;
use mapleverse_types::entity::{EntityId, MapleVerseEntity};
use mapleverse_types::event::{Epoch, EpochId, EpochSummary, WorldEventData, WorldEventType};
use mapleverse_types::reputation::ReputationReceipt;
use mapleverse_types::world::{Region, RegionId};

/// The complete MapleVerse world state
///
/// This is the main entry point for world operations. It coordinates
/// all subsystems (entities, regions, economy, reputation, attention, events).
///
/// # NO HUMAN PROFILES
///
/// All entity registration goes through validation that rejects human profiles.
pub struct WorldState {
    /// Configuration
    config: MapleVerseConfig,
    /// Entity registry
    entities: EntityRegistry,
    /// Region manager
    regions: RegionManager,
    /// Economy engine
    economy: EconomyEngine,
    /// Reputation engine
    reputation: ReputationEngine,
    /// Attention manager
    attention: AttentionManager,
    /// Event bus
    events: EventBus,
    /// Current epoch
    current_epoch: Option<Epoch>,
    /// Whether world is initialized
    initialized: bool,
}

impl WorldState {
    /// Create a new world state
    pub fn new(config: MapleVerseConfig) -> WorldResult<Self> {
        // Validate configuration
        config.validate().map_err(WorldError::Types)?;

        Ok(Self {
            entities: EntityRegistry::new(config.clone()),
            regions: RegionManager::new(config.clone()),
            economy: EconomyEngine::new(config.clone()),
            reputation: ReputationEngine::new(config.clone()),
            attention: AttentionManager::new(config.clone()),
            events: EventBus::new(),
            config,
            current_epoch: None,
            initialized: false,
        })
    }

    /// Initialize the world (must be called before operations)
    pub fn initialize(&mut self) -> WorldResult<()> {
        if self.initialized {
            return Err(WorldError::AlreadyInitialized);
        }

        // Start epoch 0
        let epoch = Epoch::genesis();
        self.current_epoch = Some(epoch);
        self.events.set_epoch(EpochId::new(0));

        // Emit world initialized event
        self.events
            .emit(WorldEventType::WorldInitialized, WorldEventData::None);

        self.initialized = true;
        Ok(())
    }

    /// Check if world is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get current epoch
    pub fn current_epoch(&self) -> WorldResult<&Epoch> {
        self.current_epoch.as_ref().ok_or(WorldError::NoActiveEpoch)
    }

    /// Get current epoch ID
    pub fn current_epoch_id(&self) -> WorldResult<EpochId> {
        Ok(self.current_epoch()?.id)
    }

    // =========================================================================
    // Entity Operations (NO HUMAN PROFILES)
    // =========================================================================

    /// Register a new individual agent
    ///
    /// # NO HUMAN PROFILES
    ///
    /// This method only creates AI agents. Attempting to create human profiles
    /// through any mechanism will result in a critical error.
    pub fn register_individual(
        &mut self,
        name: impl Into<String>,
        region_id: RegionId,
    ) -> WorldResult<EntityId> {
        self.ensure_initialized()?;

        // Register entity (validates not human)
        let entity_id = self.entities.register_individual(name, region_id.clone())?;

        // Place in region
        self.regions
            .place_entity(entity_id.clone(), region_id.clone())?;

        // Initialize economy
        self.economy.allocate_initial(entity_id.clone());

        // Initialize attention
        self.attention.initialize_entity(entity_id.clone());

        // Initialize reputation
        self.reputation.initialize_entity(entity_id.clone());

        // Emit event
        let entity = self.entities.get(&entity_id)?;
        self.events.emit(
            WorldEventType::EntityCreated,
            WorldEventData::EntityCreation {
                entity_id: entity_id.clone(),
                entity_name: entity.name.clone(),
                region_id,
            },
        );

        Ok(entity_id)
    }

    /// Register a new collective
    pub fn register_collective(
        &mut self,
        name: impl Into<String>,
        region_id: RegionId,
        founder_id: EntityId,
    ) -> WorldResult<EntityId> {
        self.ensure_initialized()?;

        let name = name.into();

        // Register collective
        let collective_id = self.entities.register_collective(
            name.clone(),
            region_id.clone(),
            founder_id.clone(),
        )?;

        // Place in region
        self.regions
            .place_entity(collective_id.clone(), region_id)?;

        // Initialize subsystems
        self.economy.allocate_initial(collective_id.clone());
        self.attention.initialize_entity(collective_id.clone());
        self.reputation.initialize_entity(collective_id.clone());

        // Emit event
        self.events.emit(
            WorldEventType::CollectiveFormed,
            WorldEventData::CollectiveFormation {
                collective_id: collective_id.clone(),
                founder_id,
                name,
            },
        );

        Ok(collective_id)
    }

    /// Get an entity
    pub fn get_entity(&self, entity_id: &EntityId) -> WorldResult<&MapleVerseEntity> {
        self.entities.get(entity_id)
    }

    /// Get a mutable entity
    pub fn get_entity_mut(&mut self, entity_id: &EntityId) -> WorldResult<&mut MapleVerseEntity> {
        self.entities.get_mut(entity_id)
    }

    // =========================================================================
    // Region Operations
    // =========================================================================

    /// Create a new region
    pub fn create_region(&mut self, name: impl Into<String>) -> WorldResult<RegionId> {
        self.ensure_initialized()?;
        let region_id = self.regions.create_region(name)?;

        self.events
            .emit(WorldEventType::RegionCreated, WorldEventData::None);

        Ok(region_id)
    }

    /// Add an existing region
    pub fn add_region(&mut self, region: Region) -> WorldResult<RegionId> {
        self.ensure_initialized()?;
        self.regions.add_region(region)
    }

    /// Connect two regions
    pub fn connect_regions(&mut self, region_a: &RegionId, region_b: &RegionId) -> WorldResult<()> {
        self.regions.connect_regions(region_a, region_b)
    }

    /// Get a region
    pub fn get_region(&self, region_id: &RegionId) -> WorldResult<&Region> {
        self.regions.get_region(region_id)
    }

    /// Migrate entity to a new region (must be neighbor)
    pub fn migrate_entity(
        &mut self,
        entity_id: &EntityId,
        to_region_id: &RegionId,
    ) -> WorldResult<()> {
        self.ensure_initialized()?;

        let from_region_id = self
            .regions
            .get_entity_region(entity_id)
            .cloned()
            .ok_or_else(|| WorldError::EntityNotFound(entity_id.clone()))?;

        // Perform migration (validates neighbor)
        self.regions.migrate_entity(entity_id, to_region_id)?;

        // Update entity's region
        let entity = self.entities.get_mut(entity_id)?;
        entity.region_id = to_region_id.clone();

        // Emit event
        self.events.emit(
            WorldEventType::Migration,
            WorldEventData::Migration {
                entity_id: entity_id.clone(),
                from_region: from_region_id,
                to_region: to_region_id.clone(),
            },
        );

        Ok(())
    }

    // =========================================================================
    // Economy Operations
    // =========================================================================

    /// Transfer MAPLE between entities
    pub fn transfer_maple(
        &mut self,
        from: &EntityId,
        to: &EntityId,
        amount: u64,
    ) -> WorldResult<()> {
        self.ensure_initialized()?;

        // Check sender balance
        let from_entity = self.entities.get(from)?;
        if !from_entity.maple_balance.has_sufficient(amount) {
            return Err(WorldError::Types(
                mapleverse_types::errors::MapleVerseError::InsufficientMaple {
                    required: amount,
                    available: from_entity.maple_balance.amount(),
                },
            ));
        }

        // Calculate fee
        let fee = self.economy.calculate_fee(amount);

        // Update balances
        {
            let sender = self.entities.get_mut(from)?;
            sender
                .maple_balance
                .transfer_out(amount + fee)
                .map_err(WorldError::Types)?;
        }

        {
            let receiver = self.entities.get_mut(to)?;
            receiver.maple_balance.receive(amount);
        }

        // Record transfer
        self.economy
            .record_transfer(from.clone(), to.clone(), amount, TransferType::Direct, None);

        // Emit event
        self.events.emit(
            WorldEventType::MapleTransfer,
            WorldEventData::MapleTransfer {
                from: from.clone(),
                to: to.clone(),
                amount,
                fee,
                reference: None,
            },
        );

        Ok(())
    }

    // =========================================================================
    // Attention Operations
    // =========================================================================

    /// Consume attention for an action
    pub fn consume_attention(&mut self, entity_id: &EntityId, amount: u64) -> WorldResult<()> {
        self.ensure_initialized()?;

        self.attention.consume(entity_id, amount)?;

        // Update entity
        let entity = self.entities.get_mut(entity_id)?;
        entity
            .attention_budget
            .consume(amount)
            .map_err(WorldError::Types)?;

        Ok(())
    }

    /// Transfer attention between entities
    pub fn transfer_attention(
        &mut self,
        from: &EntityId,
        to: &EntityId,
        amount: u64,
    ) -> WorldResult<()> {
        self.ensure_initialized()?;

        if !self.config.attention_config.attention_tradeable {
            return Err(WorldError::Types(
                mapleverse_types::errors::MapleVerseError::TransferNotAllowed {
                    from: from.clone(),
                    to: to.clone(),
                    reason: "Attention transfers are disabled".to_string(),
                },
            ));
        }

        // Perform transfer in manager
        self.attention.transfer(from, to, amount)?;

        // Update entities
        {
            let sender = self.entities.get_mut(from)?;
            sender
                .attention_budget
                .give(amount)
                .map_err(WorldError::Types)?;
        }

        {
            let receiver = self.entities.get_mut(to)?;
            receiver.attention_budget.receive_attention(amount);
        }

        // Emit event
        self.events.emit(
            WorldEventType::AttentionTransfer,
            WorldEventData::AttentionTransfer {
                from: from.clone(),
                to: to.clone(),
                amount,
            },
        );

        Ok(())
    }

    // =========================================================================
    // Reputation Operations
    // =========================================================================

    /// Process a reputation receipt
    pub fn process_reputation_receipt(&mut self, receipt: ReputationReceipt) -> WorldResult<()> {
        self.ensure_initialized()?;

        let entity_id = receipt.entity_id.clone();
        let change = receipt.reputation_change;
        let receipt_id = receipt.id.as_str().to_string();
        let category = receipt.category.clone();

        self.reputation.process_receipt(receipt)?;

        // Emit event
        self.events.emit(
            WorldEventType::ReputationReceipt,
            WorldEventData::ReputationReceiptData {
                entity_id,
                receipt_id,
                change,
                category,
            },
        );

        Ok(())
    }

    /// Get entity's reputation
    pub fn get_reputation(&self, entity_id: &EntityId) -> i64 {
        self.reputation.get_reputation(entity_id)
    }

    // =========================================================================
    // Epoch Operations
    // =========================================================================

    /// Advance to the next epoch
    pub fn advance_epoch(&mut self) -> WorldResult<EpochSummary> {
        self.ensure_initialized()?;

        let current = self.current_epoch.take().ok_or(WorldError::NoActiveEpoch)?;
        let current_epoch_id = current.id;
        let next_epoch_id = current_epoch_id.next();

        // Collect epoch statistics
        let entity_stats = self.entities.stats();
        let _region_stats = self.regions.stats();
        let economy_stats = self.economy.stats();
        let attention_stats = self.attention.stats();
        let _reputation_stats = self.reputation.stats();

        // Create epoch summary
        let summary = EpochSummary {
            epoch_id: current_epoch_id,
            starting_entities: entity_stats.total_entities,
            entities_created: entity_stats.total_created - entity_stats.total_terminated,
            entities_terminated: entity_stats.total_terminated,
            events_processed: self.events.event_count() as u64,
            maple_volume: economy_stats.total_volume,
            attention_consumed: attention_stats.epoch_consumed,
            reputation_delta: 0, // Would need tracking
            migrations: 0,       // Would need tracking
            economic_summary: mapleverse_types::economy::EconomicSummary {
                total_maple_supply: economy_stats.total_supply,
                total_attention_available: attention_stats.total_available,
                transfers_this_epoch: economy_stats.transfer_count,
                volume_this_epoch: economy_stats.total_volume,
                avg_transaction_size: economy_stats.avg_transfer_size,
                velocity: 0.0,
            },
            region_stats: std::collections::HashMap::new(),
        };

        // Regenerate attention for all entities
        let _attention_summary = self.attention.regenerate_all(next_epoch_id);

        // Apply reputation decay
        let _decay_summary = self.reputation.apply_decay(next_epoch_id);

        // Create new epoch
        let new_epoch = Epoch::new(next_epoch_id);
        self.events.set_epoch(next_epoch_id);

        // Emit epoch events
        self.events.emit(
            WorldEventType::EpochEnded,
            WorldEventData::EpochTransition {
                from_epoch: current_epoch_id,
                to_epoch: next_epoch_id,
                summary: Box::new(summary.clone()),
            },
        );

        self.events
            .emit(WorldEventType::EpochStarted, WorldEventData::None);
        self.events
            .emit(WorldEventType::AttentionRegenerated, WorldEventData::None);
        self.events
            .emit(WorldEventType::ReputationDecay, WorldEventData::None);

        self.current_epoch = Some(new_epoch);

        Ok(summary)
    }

    // =========================================================================
    // Utility Methods
    // =========================================================================

    /// Ensure world is initialized
    fn ensure_initialized(&self) -> WorldResult<()> {
        if !self.initialized {
            return Err(WorldError::NotInitialized);
        }
        Ok(())
    }

    /// Get configuration
    pub fn config(&self) -> &MapleVerseConfig {
        &self.config
    }

    /// Get entity registry
    pub fn entities(&self) -> &EntityRegistry {
        &self.entities
    }

    /// Get region manager
    pub fn regions(&self) -> &RegionManager {
        &self.regions
    }

    /// Get economy engine
    pub fn economy(&self) -> &EconomyEngine {
        &self.economy
    }

    /// Get reputation engine
    pub fn reputation(&self) -> &ReputationEngine {
        &self.reputation
    }

    /// Get attention manager
    pub fn attention(&self) -> &AttentionManager {
        &self.attention
    }

    /// Get event bus
    pub fn events(&self) -> &EventBus {
        &self.events
    }

    /// Get mutable event bus
    pub fn events_mut(&mut self) -> &mut EventBus {
        &mut self.events
    }

    /// Subscribe to events
    pub fn subscribe(
        &self,
    ) -> tokio::sync::broadcast::Receiver<mapleverse_types::event::WorldEvent> {
        self.events.subscribe()
    }

    /// Get world statistics
    pub fn stats(&self) -> WorldStats {
        WorldStats {
            initialized: self.initialized,
            current_epoch: self.current_epoch.as_ref().map(|e| e.id),
            entity_count: self.entities.total_count() as u64,
            region_count: self.regions.region_count() as u64,
            total_maple: self.economy.total_supply(),
            event_count: self.events.event_count() as u64,
        }
    }
}

/// World statistics
#[derive(Clone, Debug)]
pub struct WorldStats {
    /// Whether world is initialized
    pub initialized: bool,
    /// Current epoch ID
    pub current_epoch: Option<EpochId>,
    /// Total entity count
    pub entity_count: u64,
    /// Total region count
    pub region_count: u64,
    /// Total MAPLE supply
    pub total_maple: u64,
    /// Total events logged
    pub event_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mapleverse_types::reputation::ReputationSource;

    fn test_config() -> MapleVerseConfig {
        MapleVerseConfig::test_world()
    }

    fn initialized_world() -> WorldState {
        let mut world = WorldState::new(test_config()).unwrap();
        world.initialize().unwrap();

        // Create a default region
        world.create_region("Default Region").unwrap();

        world
    }

    #[test]
    fn test_create_world() {
        let world = WorldState::new(test_config()).unwrap();
        assert!(!world.is_initialized());
    }

    #[test]
    fn test_initialize_world() {
        let mut world = WorldState::new(test_config()).unwrap();
        world.initialize().unwrap();

        assert!(world.is_initialized());
        assert!(world.current_epoch().is_ok());
        assert_eq!(world.current_epoch_id().unwrap().number(), 0);
    }

    #[test]
    fn test_double_initialize_fails() {
        let mut world = WorldState::new(test_config()).unwrap();
        world.initialize().unwrap();

        let result = world.initialize();
        assert!(result.is_err());
    }

    #[test]
    fn test_register_individual() {
        let mut world = initialized_world();

        let region_id = world.regions().region_ids().next().cloned().unwrap();
        let entity_id = world.register_individual("TestAgent", region_id).unwrap();

        let entity = world.get_entity(&entity_id).unwrap();
        assert!(entity.is_individual());
        assert_eq!(entity.name, "TestAgent");
    }

    #[test]
    fn test_register_collective() {
        let mut world = initialized_world();

        let region_id = world.regions().region_ids().next().cloned().unwrap();
        let founder = world
            .register_individual("Founder", region_id.clone())
            .unwrap();

        let collective_id = world
            .register_collective("TestCollective", region_id, founder)
            .unwrap();

        let collective = world.get_entity(&collective_id).unwrap();
        assert!(collective.is_collective());
    }

    #[test]
    fn test_transfer_maple() {
        let mut world = initialized_world();
        let initial_balance = world.config().economy_config.initial_maple_balance;

        let region_id = world.regions().region_ids().next().cloned().unwrap();
        let sender = world
            .register_individual("Sender", region_id.clone())
            .unwrap();
        let receiver = world.register_individual("Receiver", region_id).unwrap();

        world.transfer_maple(&sender, &receiver, 100).unwrap();

        // Sender should have less (100 + fee)
        let sender_balance = world.get_entity(&sender).unwrap().maple_balance.amount();
        assert!(sender_balance < initial_balance);

        // Receiver should have more
        let receiver_balance = world.get_entity(&receiver).unwrap().maple_balance.amount();
        assert_eq!(receiver_balance, initial_balance + 100);
    }

    #[test]
    fn test_consume_attention() {
        let mut world = initialized_world();
        let base_attention = world.config().attention_config.base_attention_per_epoch;

        let region_id = world.regions().region_ids().next().cloned().unwrap();
        let entity_id = world.register_individual("Agent", region_id).unwrap();

        // Consume half of base attention
        let consume_amount = base_attention / 2;
        world.consume_attention(&entity_id, consume_amount).unwrap();

        let remaining = world
            .get_entity(&entity_id)
            .unwrap()
            .attention_budget
            .available;
        assert_eq!(remaining, base_attention - consume_amount);
    }

    #[test]
    fn test_migrate_entity() {
        let mut world = initialized_world();

        let region_a = world.create_region("Region A").unwrap();
        let region_b = world.create_region("Region B").unwrap();
        world.connect_regions(&region_a, &region_b).unwrap();

        let entity_id = world
            .register_individual("Migrant", region_a.clone())
            .unwrap();

        world.migrate_entity(&entity_id, &region_b).unwrap();

        let entity = world.get_entity(&entity_id).unwrap();
        assert_eq!(entity.region_id, region_b);
    }

    #[test]
    fn test_migrate_to_non_neighbor_fails() {
        let mut world = initialized_world();

        let region_a = world.create_region("Region A").unwrap();
        let region_b = world.create_region("Region B").unwrap();
        // NOT connected!

        let entity_id = world
            .register_individual("Migrant", region_a.clone())
            .unwrap();

        let result = world.migrate_entity(&entity_id, &region_b);
        assert!(result.is_err());
    }

    #[test]
    fn test_process_reputation_receipt() {
        let mut world = initialized_world();

        let region_id = world.regions().region_ids().next().cloned().unwrap();
        let entity_id = world.register_individual("Agent", region_id).unwrap();

        let receipt = ReputationReceipt::new(
            entity_id.clone(),
            ReputationSource::CommitmentReceipt {
                receipt_id: "test-receipt".to_string(),
                commitment_id: "commit-1".to_string(),
            },
            50,
            "Good work",
            0,
        );

        world.process_reputation_receipt(receipt).unwrap();

        assert_eq!(world.get_reputation(&entity_id), 50);
    }

    #[test]
    fn test_advance_epoch() {
        let mut world = initialized_world();

        let region_id = world.regions().region_ids().next().cloned().unwrap();
        world.register_individual("Agent", region_id).unwrap();

        assert_eq!(world.current_epoch_id().unwrap().number(), 0);

        let summary = world.advance_epoch().unwrap();

        assert_eq!(summary.epoch_id.number(), 0);
        assert_eq!(world.current_epoch_id().unwrap().number(), 1);
    }

    #[test]
    fn test_world_stats() {
        let mut world = initialized_world();

        let region_id = world.regions().region_ids().next().cloned().unwrap();
        world
            .register_individual("Agent1", region_id.clone())
            .unwrap();
        world.register_individual("Agent2", region_id).unwrap();

        let stats = world.stats();
        assert!(stats.initialized);
        assert_eq!(stats.entity_count, 2);
        assert!(stats.total_maple > 0);
    }

    #[test]
    fn test_operations_before_init_fail() {
        let mut world = WorldState::new(test_config()).unwrap();

        let result = world.create_region("Test");
        assert!(result.is_err());
    }
}
