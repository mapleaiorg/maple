//! World and region types for MapleVerse
//!
//! The MapleVerse world is structured into regions. Entities exist in regions
//! and can only migrate to **neighboring regions**.

use crate::entity::EntityId;
use crate::errors::{MapleVerseError, MapleVerseResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Unique identifier for a region
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegionId(String);

impl RegionId {
    /// Create a new region ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new random region ID
    pub fn generate() -> Self {
        Self(format!("region-{}", Uuid::new_v4()))
    }

    /// Get the inner string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RegionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for RegionId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for RegionId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// A region in the MapleVerse world
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Region {
    /// Unique identifier
    pub id: RegionId,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Maximum capacity (number of entities)
    pub capacity: u64,
    /// Current entity count
    pub entity_count: u64,
    /// Neighboring regions (can migrate to these)
    pub neighbors: HashSet<RegionId>,
    /// Region type
    pub region_type: RegionType,
    /// Region status
    pub status: RegionStatus,
    /// When this region was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Region-specific properties
    pub properties: HashMap<String, String>,
    /// Economic modifiers for this region
    pub economic_modifiers: EconomicModifiers,
}

impl Region {
    /// Create a new region
    pub fn new(name: impl Into<String>, capacity: u64) -> Self {
        Self {
            id: RegionId::generate(),
            name: name.into(),
            description: String::new(),
            capacity,
            entity_count: 0,
            neighbors: HashSet::new(),
            region_type: RegionType::Standard,
            status: RegionStatus::Active,
            created_at: chrono::Utc::now(),
            properties: HashMap::new(),
            economic_modifiers: EconomicModifiers::default(),
        }
    }

    /// Create a region with specific ID
    pub fn with_id(id: impl Into<RegionId>, name: impl Into<String>, capacity: u64) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            capacity,
            entity_count: 0,
            neighbors: HashSet::new(),
            region_type: RegionType::Standard,
            status: RegionStatus::Active,
            created_at: chrono::Utc::now(),
            properties: HashMap::new(),
            economic_modifiers: EconomicModifiers::default(),
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set region type
    pub fn with_type(mut self, region_type: RegionType) -> Self {
        self.region_type = region_type;
        self
    }

    /// Add a neighbor
    pub fn add_neighbor(&mut self, neighbor_id: RegionId) {
        self.neighbors.insert(neighbor_id);
    }

    /// Remove a neighbor
    pub fn remove_neighbor(&mut self, neighbor_id: &RegionId) {
        self.neighbors.remove(neighbor_id);
    }

    /// Check if another region is a neighbor
    pub fn is_neighbor(&self, other_id: &RegionId) -> bool {
        self.neighbors.contains(other_id)
    }

    /// Check if region has capacity for more entities
    pub fn has_capacity(&self) -> bool {
        self.entity_count < self.capacity
    }

    /// Get remaining capacity
    pub fn remaining_capacity(&self) -> u64 {
        self.capacity.saturating_sub(self.entity_count)
    }

    /// Get occupancy percentage
    pub fn occupancy(&self) -> f64 {
        if self.capacity == 0 {
            return 1.0;
        }
        self.entity_count as f64 / self.capacity as f64
    }

    /// Increment entity count
    pub fn add_entity(&mut self) -> MapleVerseResult<()> {
        if !self.has_capacity() {
            return Err(MapleVerseError::RegionAtCapacity {
                region_id: self.id.clone(),
                capacity: self.capacity,
            });
        }
        self.entity_count += 1;
        Ok(())
    }

    /// Decrement entity count
    pub fn remove_entity(&mut self) {
        self.entity_count = self.entity_count.saturating_sub(1);
    }

    /// Check if migration to another region is allowed
    pub fn can_migrate_to(&self, target: &Region) -> MapleVerseResult<()> {
        // Must be a neighbor
        if !self.is_neighbor(&target.id) {
            return Err(MapleVerseError::MigrationNotAllowed {
                from: self.id.clone(),
                to: target.id.clone(),
            });
        }

        // Target must have capacity
        if !target.has_capacity() {
            return Err(MapleVerseError::RegionAtCapacity {
                region_id: target.id.clone(),
                capacity: target.capacity,
            });
        }

        // Target must be active
        if target.status != RegionStatus::Active {
            return Err(MapleVerseError::MigrationNotAllowed {
                from: self.id.clone(),
                to: target.id.clone(),
            });
        }

        Ok(())
    }
}

/// Type of region
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionType {
    /// Standard region with no special properties
    Standard,
    /// Hub region - central, well-connected
    Hub,
    /// Frontier region - edge of the world, fewer neighbors
    Frontier,
    /// Sanctuary region - protected, limited access
    Sanctuary,
    /// Market region - focused on economic activity
    Market,
    /// Academy region - focused on learning/skills
    Academy,
    /// Custom region type
    Custom(String),
}

impl RegionType {
    /// Get default capacity multiplier for this type
    pub fn capacity_multiplier(&self) -> f64 {
        match self {
            Self::Standard => 1.0,
            Self::Hub => 2.0,
            Self::Frontier => 0.5,
            Self::Sanctuary => 0.25,
            Self::Market => 1.5,
            Self::Academy => 0.75,
            Self::Custom(_) => 1.0,
        }
    }
}

/// Status of a region
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionStatus {
    /// Region is active and accepting entities
    Active,
    /// Region is at capacity, no new entities
    Full,
    /// Region is closed for maintenance
    Maintenance,
    /// Region is archived (read-only)
    Archived,
    /// Region is being initialized
    Initializing,
}

/// Economic modifiers for a region
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EconomicModifiers {
    /// MAPLE earning multiplier (1.0 = normal)
    pub maple_earning_multiplier: f64,
    /// Attention regeneration multiplier
    pub attention_regen_multiplier: f64,
    /// Reputation gain multiplier
    pub reputation_multiplier: f64,
    /// Transfer fee modifier (added to base fee)
    pub transfer_fee_modifier: i16,
}

impl EconomicModifiers {
    /// Standard modifiers (no changes)
    pub fn standard() -> Self {
        Self {
            maple_earning_multiplier: 1.0,
            attention_regen_multiplier: 1.0,
            reputation_multiplier: 1.0,
            transfer_fee_modifier: 0,
        }
    }

    /// Hub region modifiers (better connectivity, higher fees)
    pub fn hub() -> Self {
        Self {
            maple_earning_multiplier: 1.2,
            attention_regen_multiplier: 0.9, // Busy hub, less rest
            reputation_multiplier: 1.1,
            transfer_fee_modifier: 5, // Higher fees
        }
    }

    /// Frontier modifiers (higher risk/reward)
    pub fn frontier() -> Self {
        Self {
            maple_earning_multiplier: 1.5,
            attention_regen_multiplier: 1.2,
            reputation_multiplier: 0.8, // Less visible
            transfer_fee_modifier: -5, // Lower fees
        }
    }

    /// Market modifiers (focused on trade)
    pub fn market() -> Self {
        Self {
            maple_earning_multiplier: 1.3,
            attention_regen_multiplier: 0.8, // Busy trading
            reputation_multiplier: 1.2, // High visibility
            transfer_fee_modifier: -10, // Lower fees to encourage trade
        }
    }
}

/// The topology of the MapleVerse world
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorldTopology {
    /// All regions in the world
    regions: HashMap<RegionId, Region>,
    /// Bidirectional neighbor graph
    neighbor_graph: HashMap<RegionId, HashSet<RegionId>>,
    /// Entity to region mapping
    entity_locations: HashMap<EntityId, RegionId>,
}

impl WorldTopology {
    /// Create a new empty world topology
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a region to the world
    pub fn add_region(&mut self, region: Region) -> MapleVerseResult<()> {
        if self.regions.contains_key(&region.id) {
            return Err(MapleVerseError::RegionAlreadyExists(region.id.clone()));
        }

        let region_id = region.id.clone();
        self.regions.insert(region_id.clone(), region);
        self.neighbor_graph.insert(region_id, HashSet::new());

        Ok(())
    }

    /// Remove a region from the world
    pub fn remove_region(&mut self, region_id: &RegionId) -> MapleVerseResult<Region> {
        let region = self
            .regions
            .remove(region_id)
            .ok_or_else(|| MapleVerseError::RegionNotFound(region_id.clone()))?;

        // Remove from neighbor graph
        if let Some(neighbors) = self.neighbor_graph.remove(region_id) {
            // Remove this region from all its neighbors' lists
            for neighbor_id in neighbors {
                if let Some(neighbor_neighbors) = self.neighbor_graph.get_mut(&neighbor_id) {
                    neighbor_neighbors.remove(region_id);
                }
            }
        }

        Ok(region)
    }

    /// Get a region by ID
    pub fn get_region(&self, region_id: &RegionId) -> Option<&Region> {
        self.regions.get(region_id)
    }

    /// Get a mutable region by ID
    pub fn get_region_mut(&mut self, region_id: &RegionId) -> Option<&mut Region> {
        self.regions.get_mut(region_id)
    }

    /// Connect two regions as neighbors (bidirectional)
    pub fn connect_regions(
        &mut self,
        region_a: &RegionId,
        region_b: &RegionId,
    ) -> MapleVerseResult<()> {
        if !self.regions.contains_key(region_a) {
            return Err(MapleVerseError::RegionNotFound(region_a.clone()));
        }
        if !self.regions.contains_key(region_b) {
            return Err(MapleVerseError::RegionNotFound(region_b.clone()));
        }

        // Update neighbor graph (bidirectional)
        self.neighbor_graph
            .entry(region_a.clone())
            .or_default()
            .insert(region_b.clone());
        self.neighbor_graph
            .entry(region_b.clone())
            .or_default()
            .insert(region_a.clone());

        // Update region structs
        if let Some(region) = self.regions.get_mut(region_a) {
            region.add_neighbor(region_b.clone());
        }
        if let Some(region) = self.regions.get_mut(region_b) {
            region.add_neighbor(region_a.clone());
        }

        Ok(())
    }

    /// Disconnect two regions
    pub fn disconnect_regions(
        &mut self,
        region_a: &RegionId,
        region_b: &RegionId,
    ) -> MapleVerseResult<()> {
        // Update neighbor graph
        if let Some(neighbors) = self.neighbor_graph.get_mut(region_a) {
            neighbors.remove(region_b);
        }
        if let Some(neighbors) = self.neighbor_graph.get_mut(region_b) {
            neighbors.remove(region_a);
        }

        // Update region structs
        if let Some(region) = self.regions.get_mut(region_a) {
            region.remove_neighbor(region_b);
        }
        if let Some(region) = self.regions.get_mut(region_b) {
            region.remove_neighbor(region_a);
        }

        Ok(())
    }

    /// Check if two regions are neighbors
    pub fn are_neighbors(&self, region_a: &RegionId, region_b: &RegionId) -> bool {
        self.neighbor_graph
            .get(region_a)
            .map(|n| n.contains(region_b))
            .unwrap_or(false)
    }

    /// Get all neighbors of a region
    pub fn get_neighbors(&self, region_id: &RegionId) -> Option<&HashSet<RegionId>> {
        self.neighbor_graph.get(region_id)
    }

    /// Place an entity in a region
    pub fn place_entity(
        &mut self,
        entity_id: EntityId,
        region_id: RegionId,
    ) -> MapleVerseResult<()> {
        let region = self
            .regions
            .get_mut(&region_id)
            .ok_or_else(|| MapleVerseError::RegionNotFound(region_id.clone()))?;

        region.add_entity()?;
        self.entity_locations.insert(entity_id, region_id);

        Ok(())
    }

    /// Remove an entity from its current region
    pub fn remove_entity(&mut self, entity_id: &EntityId) -> MapleVerseResult<RegionId> {
        let region_id = self
            .entity_locations
            .remove(entity_id)
            .ok_or_else(|| MapleVerseError::EntityNotFound(entity_id.clone()))?;

        if let Some(region) = self.regions.get_mut(&region_id) {
            region.remove_entity();
        }

        Ok(region_id)
    }

    /// Get the region where an entity is located
    pub fn get_entity_location(&self, entity_id: &EntityId) -> Option<&RegionId> {
        self.entity_locations.get(entity_id)
    }

    /// Migrate an entity to a new region (must be neighbor)
    pub fn migrate_entity(
        &mut self,
        entity_id: &EntityId,
        to_region_id: &RegionId,
    ) -> MapleVerseResult<()> {
        let from_region_id = self
            .entity_locations
            .get(entity_id)
            .ok_or_else(|| MapleVerseError::EntityNotFound(entity_id.clone()))?
            .clone();

        // Check if migration is allowed
        let from_region = self
            .regions
            .get(&from_region_id)
            .ok_or_else(|| MapleVerseError::RegionNotFound(from_region_id.clone()))?;

        let to_region = self
            .regions
            .get(to_region_id)
            .ok_or_else(|| MapleVerseError::RegionNotFound(to_region_id.clone()))?;

        from_region.can_migrate_to(to_region)?;

        // Perform migration
        self.regions.get_mut(&from_region_id).unwrap().remove_entity();
        self.regions.get_mut(to_region_id).unwrap().add_entity()?;
        self.entity_locations.insert(entity_id.clone(), to_region_id.clone());

        Ok(())
    }

    /// Get all region IDs
    pub fn region_ids(&self) -> impl Iterator<Item = &RegionId> {
        self.regions.keys()
    }

    /// Get all regions
    pub fn regions(&self) -> impl Iterator<Item = &Region> {
        self.regions.values()
    }

    /// Get total entity count across all regions
    pub fn total_entities(&self) -> u64 {
        self.regions.values().map(|r| r.entity_count).sum()
    }

    /// Get region count
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Find path between two regions (BFS)
    pub fn find_path(&self, from: &RegionId, to: &RegionId) -> Option<Vec<RegionId>> {
        if from == to {
            return Some(vec![from.clone()]);
        }

        if !self.regions.contains_key(from) || !self.regions.contains_key(to) {
            return None;
        }

        use std::collections::VecDeque;

        let mut visited: HashSet<RegionId> = HashSet::new();
        let mut queue: VecDeque<(RegionId, Vec<RegionId>)> = VecDeque::new();

        visited.insert(from.clone());
        queue.push_back((from.clone(), vec![from.clone()]));

        while let Some((current, path)) = queue.pop_front() {
            if let Some(neighbors) = self.neighbor_graph.get(&current) {
                for neighbor in neighbors {
                    if neighbor == to {
                        let mut final_path = path.clone();
                        final_path.push(neighbor.clone());
                        return Some(final_path);
                    }

                    if !visited.contains(neighbor) {
                        visited.insert(neighbor.clone());
                        let mut new_path = path.clone();
                        new_path.push(neighbor.clone());
                        queue.push_back((neighbor.clone(), new_path));
                    }
                }
            }
        }

        None
    }
}

/// Builder for creating world topology
pub struct WorldTopologyBuilder {
    regions: Vec<Region>,
    connections: Vec<(RegionId, RegionId)>,
}

impl Default for WorldTopologyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldTopologyBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
            connections: Vec::new(),
        }
    }

    /// Add a region
    pub fn region(mut self, region: Region) -> Self {
        self.regions.push(region);
        self
    }

    /// Connect two regions
    pub fn connect(mut self, a: impl Into<RegionId>, b: impl Into<RegionId>) -> Self {
        self.connections.push((a.into(), b.into()));
        self
    }

    /// Build the topology
    pub fn build(self) -> MapleVerseResult<WorldTopology> {
        let mut topology = WorldTopology::new();

        for region in self.regions {
            topology.add_region(region)?;
        }

        for (a, b) in self.connections {
            topology.connect_regions(&a, &b)?;
        }

        Ok(topology)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_id() {
        let id1 = RegionId::generate();
        let id2 = RegionId::generate();
        assert_ne!(id1, id2);

        let id3 = RegionId::new("test-region");
        assert_eq!(id3.as_str(), "test-region");
    }

    #[test]
    fn test_region_creation() {
        let region = Region::new("Test Region", 1000);
        assert_eq!(region.name, "Test Region");
        assert_eq!(region.capacity, 1000);
        assert_eq!(region.entity_count, 0);
        assert!(region.has_capacity());
    }

    #[test]
    fn test_region_neighbors() {
        let mut region = Region::new("Region A", 100);
        let neighbor_id = RegionId::new("region-b");

        region.add_neighbor(neighbor_id.clone());
        assert!(region.is_neighbor(&neighbor_id));

        region.remove_neighbor(&neighbor_id);
        assert!(!region.is_neighbor(&neighbor_id));
    }

    #[test]
    fn test_region_capacity() {
        let mut region = Region::new("Small Region", 2);

        assert!(region.add_entity().is_ok());
        assert!(region.add_entity().is_ok());
        assert!(region.add_entity().is_err()); // At capacity

        assert_eq!(region.occupancy(), 1.0);

        region.remove_entity();
        assert!(region.has_capacity());
        assert_eq!(region.remaining_capacity(), 1);
    }

    #[test]
    fn test_region_migration() {
        let mut from_region = Region::new("From", 100);
        let mut to_region = Region::new("To", 100);

        // Must be neighbors
        assert!(from_region.can_migrate_to(&to_region).is_err());

        // Add as neighbor
        from_region.add_neighbor(to_region.id.clone());

        // Now should work
        assert!(from_region.can_migrate_to(&to_region).is_ok());

        // Fill target region
        for _ in 0..100 {
            to_region.add_entity().unwrap();
        }

        // Should fail - at capacity
        assert!(from_region.can_migrate_to(&to_region).is_err());
    }

    #[test]
    fn test_world_topology_basic() {
        let mut topology = WorldTopology::new();

        let region_a = Region::with_id("region-a", "Region A", 100);
        let region_b = Region::with_id("region-b", "Region B", 100);

        topology.add_region(region_a).unwrap();
        topology.add_region(region_b).unwrap();

        assert_eq!(topology.region_count(), 2);

        // Not neighbors yet
        assert!(!topology.are_neighbors(&RegionId::new("region-a"), &RegionId::new("region-b")));

        // Connect them
        topology
            .connect_regions(&RegionId::new("region-a"), &RegionId::new("region-b"))
            .unwrap();

        assert!(topology.are_neighbors(&RegionId::new("region-a"), &RegionId::new("region-b")));
        assert!(topology.are_neighbors(&RegionId::new("region-b"), &RegionId::new("region-a")));
    }

    #[test]
    fn test_entity_placement() {
        let mut topology = WorldTopology::new();
        topology.add_region(Region::with_id("region-a", "A", 10)).unwrap();

        let entity_id = EntityId::new("entity-1");
        topology
            .place_entity(entity_id.clone(), RegionId::new("region-a"))
            .unwrap();

        assert_eq!(
            topology.get_entity_location(&entity_id),
            Some(&RegionId::new("region-a"))
        );

        let region = topology.get_region(&RegionId::new("region-a")).unwrap();
        assert_eq!(region.entity_count, 1);
    }

    #[test]
    fn test_entity_migration() {
        let mut topology = WorldTopology::new();
        topology.add_region(Region::with_id("region-a", "A", 10)).unwrap();
        topology.add_region(Region::with_id("region-b", "B", 10)).unwrap();
        topology
            .connect_regions(&RegionId::new("region-a"), &RegionId::new("region-b"))
            .unwrap();

        let entity_id = EntityId::new("entity-1");
        topology
            .place_entity(entity_id.clone(), RegionId::new("region-a"))
            .unwrap();

        // Migrate to neighbor
        topology
            .migrate_entity(&entity_id, &RegionId::new("region-b"))
            .unwrap();

        assert_eq!(
            topology.get_entity_location(&entity_id),
            Some(&RegionId::new("region-b"))
        );

        // Check entity counts
        assert_eq!(
            topology.get_region(&RegionId::new("region-a")).unwrap().entity_count,
            0
        );
        assert_eq!(
            topology.get_region(&RegionId::new("region-b")).unwrap().entity_count,
            1
        );
    }

    #[test]
    fn test_migration_non_neighbor_fails() {
        let mut topology = WorldTopology::new();
        topology.add_region(Region::with_id("region-a", "A", 10)).unwrap();
        topology.add_region(Region::with_id("region-b", "B", 10)).unwrap();
        // NOT connected

        let entity_id = EntityId::new("entity-1");
        topology
            .place_entity(entity_id.clone(), RegionId::new("region-a"))
            .unwrap();

        // Should fail - not neighbors
        let result = topology.migrate_entity(&entity_id, &RegionId::new("region-b"));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MapleVerseError::MigrationNotAllowed { .. }
        ));
    }

    #[test]
    fn test_find_path() {
        let mut topology = WorldTopology::new();

        // Create a chain: A - B - C - D
        for name in ["a", "b", "c", "d"] {
            topology
                .add_region(Region::with_id(format!("region-{}", name), name.to_uppercase(), 10))
                .unwrap();
        }

        topology.connect_regions(&RegionId::new("region-a"), &RegionId::new("region-b")).unwrap();
        topology.connect_regions(&RegionId::new("region-b"), &RegionId::new("region-c")).unwrap();
        topology.connect_regions(&RegionId::new("region-c"), &RegionId::new("region-d")).unwrap();

        let path = topology
            .find_path(&RegionId::new("region-a"), &RegionId::new("region-d"))
            .unwrap();

        assert_eq!(path.len(), 4);
        assert_eq!(path[0], RegionId::new("region-a"));
        assert_eq!(path[3], RegionId::new("region-d"));
    }

    #[test]
    fn test_find_path_no_route() {
        let mut topology = WorldTopology::new();
        topology.add_region(Region::with_id("region-a", "A", 10)).unwrap();
        topology.add_region(Region::with_id("region-b", "B", 10)).unwrap();
        // NOT connected

        let path = topology.find_path(&RegionId::new("region-a"), &RegionId::new("region-b"));
        assert!(path.is_none());
    }

    #[test]
    fn test_topology_builder() {
        let topology = WorldTopologyBuilder::new()
            .region(Region::with_id("region-a", "A", 100))
            .region(Region::with_id("region-b", "B", 100))
            .region(Region::with_id("region-c", "C", 100))
            .connect("region-a", "region-b")
            .connect("region-b", "region-c")
            .build()
            .unwrap();

        assert_eq!(topology.region_count(), 3);
        assert!(topology.are_neighbors(&RegionId::new("region-a"), &RegionId::new("region-b")));
        assert!(topology.are_neighbors(&RegionId::new("region-b"), &RegionId::new("region-c")));
        assert!(!topology.are_neighbors(&RegionId::new("region-a"), &RegionId::new("region-c")));
    }

    #[test]
    fn test_economic_modifiers() {
        let standard = EconomicModifiers::standard();
        assert_eq!(standard.maple_earning_multiplier, 1.0);

        let hub = EconomicModifiers::hub();
        assert!(hub.maple_earning_multiplier > 1.0);
        assert!(hub.transfer_fee_modifier > 0);

        let frontier = EconomicModifiers::frontier();
        assert!(frontier.maple_earning_multiplier > 1.0);
        assert!(frontier.transfer_fee_modifier < 0);
    }

    #[test]
    fn test_region_types() {
        assert_eq!(RegionType::Standard.capacity_multiplier(), 1.0);
        assert_eq!(RegionType::Hub.capacity_multiplier(), 2.0);
        assert!(RegionType::Frontier.capacity_multiplier() < 1.0);
    }

    #[test]
    fn test_remove_region() {
        let mut topology = WorldTopology::new();
        topology.add_region(Region::with_id("region-a", "A", 100)).unwrap();
        topology.add_region(Region::with_id("region-b", "B", 100)).unwrap();
        topology.connect_regions(&RegionId::new("region-a"), &RegionId::new("region-b")).unwrap();

        let removed = topology.remove_region(&RegionId::new("region-a")).unwrap();
        assert_eq!(removed.name, "A");

        // Should no longer be connected
        assert!(!topology.are_neighbors(&RegionId::new("region-a"), &RegionId::new("region-b")));
        assert_eq!(topology.region_count(), 1);
    }

    #[test]
    fn test_serialization() {
        let region = Region::new("Test", 100)
            .with_description("A test region")
            .with_type(RegionType::Hub);

        let json = serde_json::to_string(&region).unwrap();
        let deserialized: Region = serde_json::from_str(&json).unwrap();

        assert_eq!(region.name, deserialized.name);
        assert_eq!(region.region_type, deserialized.region_type);
    }
}
