//! Region manager for MapleVerse
//!
//! Manages world topology and entity placement.
//! **CRITICAL**: Migration is only allowed between **neighboring regions**.

use crate::errors::{WorldError, WorldResult};
use mapleverse_types::config::MapleVerseConfig;
use mapleverse_types::entity::EntityId;
use mapleverse_types::errors::MapleVerseError;
use mapleverse_types::world::{Region, RegionId, WorldTopology};

/// Manager for regions and entity placement
pub struct RegionManager {
    /// World topology
    topology: WorldTopology,
    /// Configuration
    config: MapleVerseConfig,
}

impl RegionManager {
    /// Create a new region manager
    pub fn new(config: MapleVerseConfig) -> Self {
        Self {
            topology: WorldTopology::new(),
            config,
        }
    }

    /// Create a region manager with initial topology
    pub fn with_topology(topology: WorldTopology, config: MapleVerseConfig) -> Self {
        Self { topology, config }
    }

    /// Add a new region
    pub fn add_region(&mut self, region: Region) -> WorldResult<RegionId> {
        let region_id = region.id.clone();
        self.topology
            .add_region(region)
            .map_err(WorldError::Types)?;
        Ok(region_id)
    }

    /// Create and add a standard region
    pub fn create_region(&mut self, name: impl Into<String>) -> WorldResult<RegionId> {
        let region = Region::new(name, self.config.region_config.default_region_capacity);
        self.add_region(region)
    }

    /// Connect two regions (bidirectional)
    pub fn connect_regions(&mut self, region_a: &RegionId, region_b: &RegionId) -> WorldResult<()> {
        self.topology
            .connect_regions(region_a, region_b)
            .map_err(WorldError::Types)
    }

    /// Disconnect two regions
    pub fn disconnect_regions(
        &mut self,
        region_a: &RegionId,
        region_b: &RegionId,
    ) -> WorldResult<()> {
        self.topology
            .disconnect_regions(region_a, region_b)
            .map_err(WorldError::Types)
    }

    /// Get a region
    pub fn get_region(&self, region_id: &RegionId) -> WorldResult<&Region> {
        self.topology
            .get_region(region_id)
            .ok_or_else(|| WorldError::RegionNotFound(region_id.clone()))
    }

    /// Get a mutable region
    pub fn get_region_mut(&mut self, region_id: &RegionId) -> WorldResult<&mut Region> {
        self.topology
            .get_region_mut(region_id)
            .ok_or_else(|| WorldError::RegionNotFound(region_id.clone()))
    }

    /// Place an entity in a region
    pub fn place_entity(&mut self, entity_id: EntityId, region_id: RegionId) -> WorldResult<()> {
        self.topology
            .place_entity(entity_id, region_id)
            .map_err(WorldError::Types)
    }

    /// Remove an entity from its current region
    pub fn remove_entity(&mut self, entity_id: &EntityId) -> WorldResult<RegionId> {
        self.topology
            .remove_entity(entity_id)
            .map_err(WorldError::Types)
    }

    /// Get entity's current region
    pub fn get_entity_region(&self, entity_id: &EntityId) -> Option<&RegionId> {
        self.topology.get_entity_location(entity_id)
    }

    /// Migrate an entity to a new region
    ///
    /// # Migration Rules
    ///
    /// - Target region MUST be a neighbor of source region
    /// - Target region MUST have capacity
    /// - Target region MUST be active
    pub fn migrate_entity(
        &mut self,
        entity_id: &EntityId,
        to_region_id: &RegionId,
    ) -> WorldResult<MigrationResult> {
        // Get current location
        let from_region_id = self
            .topology
            .get_entity_location(entity_id)
            .cloned()
            .ok_or_else(|| WorldError::Types(MapleVerseError::EntityNotFound(entity_id.clone())))?;

        // Check if regions are neighbors
        if !self.topology.are_neighbors(&from_region_id, to_region_id) {
            return Err(WorldError::Types(MapleVerseError::MigrationNotAllowed {
                from: from_region_id.clone(),
                to: to_region_id.clone(),
            }));
        }

        // Perform migration
        self.topology
            .migrate_entity(entity_id, to_region_id)
            .map_err(WorldError::Types)?;

        Ok(MigrationResult {
            entity_id: entity_id.clone(),
            from_region: from_region_id,
            to_region: to_region_id.clone(),
        })
    }

    /// Check if migration is possible
    pub fn can_migrate(
        &self,
        entity_id: &EntityId,
        to_region_id: &RegionId,
    ) -> Result<(), MapleVerseError> {
        let from_region_id = self
            .topology
            .get_entity_location(entity_id)
            .ok_or_else(|| MapleVerseError::EntityNotFound(entity_id.clone()))?;

        if !self.topology.are_neighbors(from_region_id, to_region_id) {
            return Err(MapleVerseError::MigrationNotAllowed {
                from: from_region_id.clone(),
                to: to_region_id.clone(),
            });
        }

        let to_region = self
            .topology
            .get_region(to_region_id)
            .ok_or_else(|| MapleVerseError::RegionNotFound(to_region_id.clone()))?;

        if !to_region.has_capacity() {
            return Err(MapleVerseError::RegionAtCapacity {
                region_id: to_region_id.clone(),
                capacity: to_region.capacity,
            });
        }

        Ok(())
    }

    /// Find path between two regions
    pub fn find_path(&self, from: &RegionId, to: &RegionId) -> Option<Vec<RegionId>> {
        self.topology.find_path(from, to)
    }

    /// Get neighbors of a region
    pub fn get_neighbors(&self, region_id: &RegionId) -> WorldResult<Vec<RegionId>> {
        self.topology
            .get_neighbors(region_id)
            .map(|n| n.iter().cloned().collect())
            .ok_or_else(|| WorldError::RegionNotFound(region_id.clone()))
    }

    /// Check if two regions are neighbors
    pub fn are_neighbors(&self, region_a: &RegionId, region_b: &RegionId) -> bool {
        self.topology.are_neighbors(region_a, region_b)
    }

    /// Get all region IDs
    pub fn region_ids(&self) -> impl Iterator<Item = &RegionId> {
        self.topology.region_ids()
    }

    /// Get all regions
    pub fn regions(&self) -> impl Iterator<Item = &Region> {
        self.topology.regions()
    }

    /// Get region count
    pub fn region_count(&self) -> usize {
        self.topology.region_count()
    }

    /// Get total entity count across all regions
    pub fn total_entities(&self) -> u64 {
        self.topology.total_entities()
    }

    /// Get statistics
    pub fn stats(&self) -> RegionManagerStats {
        let regions: Vec<_> = self.topology.regions().collect();
        let total_capacity: u64 = regions.iter().map(|r| r.capacity).sum();
        let total_entities: u64 = regions.iter().map(|r| r.entity_count).sum();

        RegionManagerStats {
            region_count: regions.len() as u64,
            total_capacity,
            total_entities,
            occupancy: if total_capacity > 0 {
                total_entities as f64 / total_capacity as f64
            } else {
                0.0
            },
        }
    }
}

/// Result of a migration operation
#[derive(Clone, Debug)]
pub struct MigrationResult {
    /// Entity that migrated
    pub entity_id: EntityId,
    /// Source region
    pub from_region: RegionId,
    /// Destination region
    pub to_region: RegionId,
}

/// Statistics about region manager
#[derive(Clone, Debug, Default)]
pub struct RegionManagerStats {
    /// Number of regions
    pub region_count: u64,
    /// Total capacity across all regions
    pub total_capacity: u64,
    /// Total entities across all regions
    pub total_entities: u64,
    /// Overall occupancy (entities/capacity)
    pub occupancy: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> MapleVerseConfig {
        MapleVerseConfig::test_world()
    }

    #[test]
    fn test_create_region() {
        let mut manager = RegionManager::new(test_config());

        let region_id = manager.create_region("TestRegion").unwrap();
        assert!(manager.get_region(&region_id).is_ok());
    }

    #[test]
    fn test_connect_regions() {
        let mut manager = RegionManager::new(test_config());

        let region_a = manager.create_region("Region A").unwrap();
        let region_b = manager.create_region("Region B").unwrap();

        assert!(!manager.are_neighbors(&region_a, &region_b));

        manager.connect_regions(&region_a, &region_b).unwrap();

        assert!(manager.are_neighbors(&region_a, &region_b));
        assert!(manager.are_neighbors(&region_b, &region_a)); // Bidirectional
    }

    #[test]
    fn test_place_entity() {
        let mut manager = RegionManager::new(test_config());

        let region_id = manager.create_region("Region").unwrap();
        let entity_id = EntityId::generate();

        manager
            .place_entity(entity_id.clone(), region_id.clone())
            .unwrap();

        assert_eq!(manager.get_entity_region(&entity_id), Some(&region_id));

        let region = manager.get_region(&region_id).unwrap();
        assert_eq!(region.entity_count, 1);
    }

    #[test]
    fn test_migrate_entity_to_neighbor() {
        let mut manager = RegionManager::new(test_config());

        let region_a = manager.create_region("Region A").unwrap();
        let region_b = manager.create_region("Region B").unwrap();
        manager.connect_regions(&region_a, &region_b).unwrap();

        let entity_id = EntityId::generate();
        manager
            .place_entity(entity_id.clone(), region_a.clone())
            .unwrap();

        let result = manager.migrate_entity(&entity_id, &region_b).unwrap();

        assert_eq!(result.from_region, region_a);
        assert_eq!(result.to_region, region_b);
        assert_eq!(manager.get_entity_region(&entity_id), Some(&region_b));
    }

    #[test]
    fn test_migrate_to_non_neighbor_fails() {
        let mut manager = RegionManager::new(test_config());

        let region_a = manager.create_region("Region A").unwrap();
        let region_b = manager.create_region("Region B").unwrap();
        // NOT connected!

        let entity_id = EntityId::generate();
        manager
            .place_entity(entity_id.clone(), region_a.clone())
            .unwrap();

        let result = manager.migrate_entity(&entity_id, &region_b);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_path() {
        let mut manager = RegionManager::new(test_config());

        // Create chain: A - B - C
        let region_a = manager.create_region("A").unwrap();
        let region_b = manager.create_region("B").unwrap();
        let region_c = manager.create_region("C").unwrap();

        manager.connect_regions(&region_a, &region_b).unwrap();
        manager.connect_regions(&region_b, &region_c).unwrap();

        let path = manager.find_path(&region_a, &region_c).unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], region_a);
        assert_eq!(path[2], region_c);
    }

    #[test]
    fn test_get_neighbors() {
        let mut manager = RegionManager::new(test_config());

        let region_a = manager.create_region("A").unwrap();
        let region_b = manager.create_region("B").unwrap();
        let region_c = manager.create_region("C").unwrap();

        manager.connect_regions(&region_a, &region_b).unwrap();
        manager.connect_regions(&region_a, &region_c).unwrap();

        let neighbors = manager.get_neighbors(&region_a).unwrap();
        assert_eq!(neighbors.len(), 2);
        assert!(neighbors.contains(&region_b));
        assert!(neighbors.contains(&region_c));
    }

    #[test]
    fn test_stats() {
        let mut manager = RegionManager::new(test_config());

        manager.create_region("Region 1").unwrap();
        manager.create_region("Region 2").unwrap();

        let stats = manager.stats();
        assert_eq!(stats.region_count, 2);
        assert!(stats.total_capacity > 0);
    }

    #[test]
    fn test_can_migrate() {
        let mut manager = RegionManager::new(test_config());

        let region_a = manager.create_region("A").unwrap();
        let region_b = manager.create_region("B").unwrap();
        manager.connect_regions(&region_a, &region_b).unwrap();

        let entity_id = EntityId::generate();
        manager
            .place_entity(entity_id.clone(), region_a.clone())
            .unwrap();

        // Should be able to migrate to neighbor
        assert!(manager.can_migrate(&entity_id, &region_b).is_ok());
    }
}
