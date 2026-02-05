//! Event and epoch types for MapleVerse
//!
//! The MapleVerse operates in epochs. Each epoch is a discrete time period
//! during which:
//! - Attention regenerates
//! - Reputation decays
//! - World events are processed
//! - Economic summaries are calculated

use crate::economy::EconomicSummary;
use crate::entity::EntityId;
use crate::world::RegionId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for an epoch
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct EpochId(pub u64);

impl EpochId {
    /// Create a new epoch ID
    pub const fn new(epoch: u64) -> Self {
        Self(epoch)
    }

    /// Get the epoch number
    pub const fn number(&self) -> u64 {
        self.0
    }

    /// Get the next epoch
    pub const fn next(&self) -> Self {
        Self(self.0 + 1)
    }

    /// Get the previous epoch (saturating)
    pub const fn previous(&self) -> Self {
        Self(self.0.saturating_sub(1))
    }
}

impl std::fmt::Display for EpochId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "epoch-{}", self.0)
    }
}

impl From<u64> for EpochId {
    fn from(n: u64) -> Self {
        Self(n)
    }
}

/// An epoch in the MapleVerse simulation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Epoch {
    /// Epoch identifier
    pub id: EpochId,
    /// When this epoch started
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// When this epoch ended (None if current)
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Epoch status
    pub status: EpochStatus,
    /// Events that occurred during this epoch
    pub events: Vec<WorldEventId>,
    /// Summary statistics
    pub summary: Option<EpochSummary>,
}

impl Epoch {
    /// Create a new epoch
    pub fn new(id: EpochId) -> Self {
        Self {
            id,
            started_at: chrono::Utc::now(),
            ended_at: None,
            status: EpochStatus::Active,
            events: Vec::new(),
            summary: None,
        }
    }

    /// Create the genesis epoch (epoch 0)
    pub fn genesis() -> Self {
        Self::new(EpochId::new(0))
    }

    /// End this epoch
    pub fn end(&mut self, summary: EpochSummary) {
        self.ended_at = Some(chrono::Utc::now());
        self.status = EpochStatus::Completed;
        self.summary = Some(summary);
    }

    /// Add an event to this epoch
    pub fn add_event(&mut self, event_id: WorldEventId) {
        self.events.push(event_id);
    }

    /// Get epoch duration in seconds
    pub fn duration_secs(&self) -> Option<i64> {
        self.ended_at
            .map(|end| (end - self.started_at).num_seconds())
    }

    /// Check if this epoch is active
    pub fn is_active(&self) -> bool {
        self.status == EpochStatus::Active
    }

    /// Check if this epoch is completed
    pub fn is_completed(&self) -> bool {
        self.status == EpochStatus::Completed
    }
}

/// Status of an epoch
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpochStatus {
    /// Epoch is currently active
    Active,
    /// Epoch is being finalized
    Finalizing,
    /// Epoch is completed
    Completed,
    /// Epoch was skipped
    Skipped,
}

/// Summary of an epoch
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EpochSummary {
    /// Epoch ID
    pub epoch_id: EpochId,
    /// Total entities at epoch start
    pub starting_entities: u64,
    /// Entities created during epoch
    pub entities_created: u64,
    /// Entities terminated during epoch
    pub entities_terminated: u64,
    /// Total events processed
    pub events_processed: u64,
    /// Total MAPLE transferred
    pub maple_volume: u64,
    /// Total attention consumed
    pub attention_consumed: u64,
    /// Total reputation changes
    pub reputation_delta: i64,
    /// Number of migrations
    pub migrations: u64,
    /// Economic summary
    pub economic_summary: EconomicSummary,
    /// Per-region statistics
    pub region_stats: HashMap<RegionId, RegionEpochStats>,
}

impl EpochSummary {
    /// Create a new empty summary
    pub fn new(epoch_id: EpochId) -> Self {
        Self {
            epoch_id,
            ..Default::default()
        }
    }

    /// Get net entity change
    pub fn net_entity_change(&self) -> i64 {
        self.entities_created as i64 - self.entities_terminated as i64
    }

    /// Get ending entity count
    pub fn ending_entities(&self) -> u64 {
        (self.starting_entities as i64 + self.net_entity_change()) as u64
    }
}

/// Per-region statistics for an epoch
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RegionEpochStats {
    /// Entities at start of epoch
    pub starting_count: u64,
    /// Entities at end of epoch
    pub ending_count: u64,
    /// Migrations into this region
    pub migrations_in: u64,
    /// Migrations out of this region
    pub migrations_out: u64,
    /// MAPLE volume in this region
    pub maple_volume: u64,
    /// Attention consumed in this region
    pub attention_consumed: u64,
}

/// Unique identifier for a world event
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorldEventId(String);

impl WorldEventId {
    /// Create a new event ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new random event ID
    pub fn generate() -> Self {
        Self(format!("event-{}", Uuid::new_v4()))
    }

    /// Get the inner string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorldEventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A world event in MapleVerse
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldEvent {
    /// Unique event ID
    pub id: WorldEventId,
    /// Epoch when this event occurred
    pub epoch: EpochId,
    /// Event type
    pub event_type: WorldEventType,
    /// When the event occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Entity that initiated the event (if any)
    pub initiator: Option<EntityId>,
    /// Entities affected by this event
    pub affected_entities: Vec<EntityId>,
    /// Regions involved
    pub regions: Vec<RegionId>,
    /// Event data
    pub data: WorldEventData,
    /// Processing status
    pub status: EventStatus,
}

impl WorldEvent {
    /// Create a new world event
    pub fn new(event_type: WorldEventType, epoch: EpochId, data: WorldEventData) -> Self {
        Self {
            id: WorldEventId::generate(),
            epoch,
            event_type,
            timestamp: chrono::Utc::now(),
            initiator: None,
            affected_entities: Vec::new(),
            regions: Vec::new(),
            data,
            status: EventStatus::Pending,
        }
    }

    /// Set the initiator
    pub fn with_initiator(mut self, entity_id: EntityId) -> Self {
        self.initiator = Some(entity_id);
        self
    }

    /// Add an affected entity
    pub fn with_affected(mut self, entity_id: EntityId) -> Self {
        self.affected_entities.push(entity_id);
        self
    }

    /// Add a region
    pub fn with_region(mut self, region_id: RegionId) -> Self {
        self.regions.push(region_id);
        self
    }

    /// Mark as processing
    pub fn mark_processing(&mut self) {
        self.status = EventStatus::Processing;
    }

    /// Mark as completed
    pub fn mark_completed(&mut self) {
        self.status = EventStatus::Completed;
    }

    /// Mark as failed
    pub fn mark_failed(&mut self, reason: String) {
        self.status = EventStatus::Failed(reason);
    }

    /// Check if this event is actionable
    pub fn is_pending(&self) -> bool {
        self.status == EventStatus::Pending
    }
}

/// Type of world event
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldEventType {
    // Entity events
    /// New entity created
    EntityCreated,
    /// Entity terminated
    EntityTerminated,
    /// Entity status changed
    EntityStatusChanged,

    // Economy events
    /// MAPLE transfer
    MapleTransfer,
    /// Attention transfer
    AttentionTransfer,
    /// Reward distributed
    RewardDistributed,

    // Reputation events
    /// Reputation receipt processed
    ReputationReceipt,
    /// Reputation decay applied
    ReputationDecay,

    // World events
    /// Entity migrated to new region
    Migration,
    /// Region status changed
    RegionStatusChanged,
    /// New region created
    RegionCreated,

    // Collective events
    /// Collective formed
    CollectiveFormed,
    /// Member joined collective
    MemberJoined,
    /// Member left collective
    MemberLeft,

    // Epoch events
    /// Epoch started
    EpochStarted,
    /// Epoch ended
    EpochEnded,
    /// Attention regenerated
    AttentionRegenerated,

    // Commitment events
    /// Commitment created
    CommitmentCreated,
    /// Commitment fulfilled
    CommitmentFulfilled,
    /// Commitment failed
    CommitmentFailed,

    // System events
    /// World initialized
    WorldInitialized,
    /// Configuration changed
    ConfigurationChanged,
    /// Custom event
    Custom(String),
}

/// Event processing status
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventStatus {
    /// Event is pending processing
    Pending,
    /// Event is being processed
    Processing,
    /// Event completed successfully
    Completed,
    /// Event processing failed
    Failed(String),
    /// Event was cancelled
    Cancelled,
}

/// Data associated with a world event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorldEventData {
    /// No additional data
    None,

    /// Entity creation data
    EntityCreation {
        /// The created entity's ID
        entity_id: EntityId,
        /// The entity's display name
        entity_name: String,
        /// The region where entity was created
        region_id: RegionId,
    },

    /// Entity termination data
    EntityTermination {
        /// The terminated entity's ID
        entity_id: EntityId,
        /// Reason for termination
        reason: String,
    },

    /// MAPLE transfer data
    MapleTransfer {
        /// Source entity
        from: EntityId,
        /// Target entity
        to: EntityId,
        /// Amount transferred
        amount: u64,
        /// Fee charged
        fee: u64,
        /// Optional reference (commitment ID, etc.)
        reference: Option<String>,
    },

    /// Attention transfer data
    AttentionTransfer {
        /// Source entity
        from: EntityId,
        /// Target entity
        to: EntityId,
        /// Attention units transferred
        amount: u64,
    },

    /// Migration data
    Migration {
        /// The migrating entity
        entity_id: EntityId,
        /// Source region
        from_region: RegionId,
        /// Destination region
        to_region: RegionId,
    },

    /// Reputation receipt data
    ReputationReceiptData {
        /// Entity receiving reputation change
        entity_id: EntityId,
        /// The receipt ID that caused this change
        receipt_id: String,
        /// Reputation change amount
        change: i64,
        /// Optional category
        category: Option<String>,
    },

    /// Collective formation data
    CollectiveFormation {
        /// The new collective's ID
        collective_id: EntityId,
        /// Who founded the collective
        founder_id: EntityId,
        /// Collective name
        name: String,
    },

    /// Member join data
    MemberJoin {
        /// The collective being joined
        collective_id: EntityId,
        /// The new member
        member_id: EntityId,
    },

    /// Commitment data
    CommitmentEvent {
        /// The commitment ID
        commitment_id: String,
        /// Entity involved
        entity_id: EntityId,
        /// Description of the commitment event
        description: String,
    },

    /// Epoch transition data
    EpochTransition {
        /// Previous epoch
        from_epoch: EpochId,
        /// New epoch
        to_epoch: EpochId,
        /// Summary of the completed epoch
        summary: Box<EpochSummary>,
    },

    /// Custom data
    Custom(HashMap<String, String>),
}

/// Event filter for querying events
#[derive(Clone, Debug, Default)]
pub struct EventFilter {
    /// Filter by epoch
    pub epoch: Option<EpochId>,
    /// Filter by event types
    pub event_types: Option<Vec<WorldEventType>>,
    /// Filter by initiator
    pub initiator: Option<EntityId>,
    /// Filter by affected entity
    pub affected_entity: Option<EntityId>,
    /// Filter by region
    pub region: Option<RegionId>,
    /// Filter by status
    pub status: Option<EventStatus>,
    /// Limit results
    pub limit: Option<usize>,
}

impl EventFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by epoch
    pub fn epoch(mut self, epoch: EpochId) -> Self {
        self.epoch = Some(epoch);
        self
    }

    /// Filter by event type
    pub fn event_type(mut self, event_type: WorldEventType) -> Self {
        self.event_types
            .get_or_insert_with(Vec::new)
            .push(event_type);
        self
    }

    /// Filter by initiator
    pub fn initiator(mut self, entity_id: EntityId) -> Self {
        self.initiator = Some(entity_id);
        self
    }

    /// Filter by affected entity
    pub fn affected(mut self, entity_id: EntityId) -> Self {
        self.affected_entity = Some(entity_id);
        self
    }

    /// Filter by region
    pub fn region(mut self, region_id: RegionId) -> Self {
        self.region = Some(region_id);
        self
    }

    /// Set limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Check if an event matches this filter
    pub fn matches(&self, event: &WorldEvent) -> bool {
        if let Some(epoch) = self.epoch {
            if event.epoch != epoch {
                return false;
            }
        }

        if let Some(ref types) = self.event_types {
            if !types.contains(&event.event_type) {
                return false;
            }
        }

        if let Some(ref initiator) = self.initiator {
            if event.initiator.as_ref() != Some(initiator) {
                return false;
            }
        }

        if let Some(ref affected) = self.affected_entity {
            if !event.affected_entities.contains(affected) {
                return false;
            }
        }

        if let Some(ref region) = self.region {
            if !event.regions.contains(region) {
                return false;
            }
        }

        if let Some(ref status) = self.status {
            if &event.status != status {
                return false;
            }
        }

        true
    }
}

/// Event log for storing and querying events
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EventLog {
    /// All events indexed by ID
    events: HashMap<WorldEventId, WorldEvent>,
    /// Events indexed by epoch
    by_epoch: HashMap<EpochId, Vec<WorldEventId>>,
    /// Events indexed by entity
    by_entity: HashMap<EntityId, Vec<WorldEventId>>,
}

impl EventLog {
    /// Create a new event log
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an event to the log
    pub fn add(&mut self, event: WorldEvent) {
        let event_id = event.id.clone();
        let epoch = event.epoch;

        // Index by entity
        if let Some(ref initiator) = event.initiator {
            self.by_entity
                .entry(initiator.clone())
                .or_default()
                .push(event_id.clone());
        }

        for entity in &event.affected_entities {
            self.by_entity
                .entry(entity.clone())
                .or_default()
                .push(event_id.clone());
        }

        // Index by epoch
        self.by_epoch.entry(epoch).or_default().push(event_id.clone());

        // Store event
        self.events.insert(event_id, event);
    }

    /// Get an event by ID
    pub fn get(&self, event_id: &WorldEventId) -> Option<&WorldEvent> {
        self.events.get(event_id)
    }

    /// Get a mutable event by ID
    pub fn get_mut(&mut self, event_id: &WorldEventId) -> Option<&mut WorldEvent> {
        self.events.get_mut(event_id)
    }

    /// Get all events for an epoch
    pub fn get_by_epoch(&self, epoch: EpochId) -> Vec<&WorldEvent> {
        self.by_epoch
            .get(&epoch)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.events.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all events for an entity
    pub fn get_by_entity(&self, entity_id: &EntityId) -> Vec<&WorldEvent> {
        self.by_entity
            .get(entity_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.events.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Query events with a filter
    pub fn query(&self, filter: &EventFilter) -> Vec<&WorldEvent> {
        let mut results: Vec<&WorldEvent> = self
            .events
            .values()
            .filter(|e| filter.matches(e))
            .collect();

        // Sort by timestamp
        results.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        // Apply limit
        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        results
    }

    /// Get total event count
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epoch_id() {
        let epoch = EpochId::new(42);
        assert_eq!(epoch.number(), 42);
        assert_eq!(epoch.next().number(), 43);
        assert_eq!(epoch.previous().number(), 41);

        let genesis = EpochId::new(0);
        assert_eq!(genesis.previous().number(), 0); // Saturating
    }

    #[test]
    fn test_epoch_creation() {
        let epoch = Epoch::new(EpochId::new(1));
        assert!(epoch.is_active());
        assert!(!epoch.is_completed());
        assert!(epoch.ended_at.is_none());
    }

    #[test]
    fn test_epoch_end() {
        let mut epoch = Epoch::new(EpochId::new(1));
        let summary = EpochSummary::new(EpochId::new(1));

        epoch.end(summary);

        assert!(epoch.is_completed());
        assert!(epoch.ended_at.is_some());
        assert!(epoch.summary.is_some());
    }

    #[test]
    fn test_epoch_summary() {
        let mut summary = EpochSummary::new(EpochId::new(1));
        summary.starting_entities = 100;
        summary.entities_created = 20;
        summary.entities_terminated = 5;

        assert_eq!(summary.net_entity_change(), 15);
        assert_eq!(summary.ending_entities(), 115);
    }

    #[test]
    fn test_world_event_creation() {
        let event = WorldEvent::new(
            WorldEventType::EntityCreated,
            EpochId::new(1),
            WorldEventData::EntityCreation {
                entity_id: EntityId::new("entity-1"),
                entity_name: "Test Entity".to_string(),
                region_id: RegionId::new("region-1"),
            },
        )
        .with_initiator(EntityId::new("creator"))
        .with_affected(EntityId::new("entity-1"))
        .with_region(RegionId::new("region-1"));

        assert_eq!(event.event_type, WorldEventType::EntityCreated);
        assert!(event.is_pending());
        assert!(event.initiator.is_some());
        assert_eq!(event.affected_entities.len(), 1);
        assert_eq!(event.regions.len(), 1);
    }

    #[test]
    fn test_event_status_transitions() {
        let mut event = WorldEvent::new(
            WorldEventType::MapleTransfer,
            EpochId::new(1),
            WorldEventData::None,
        );

        assert_eq!(event.status, EventStatus::Pending);

        event.mark_processing();
        assert_eq!(event.status, EventStatus::Processing);

        event.mark_completed();
        assert_eq!(event.status, EventStatus::Completed);

        let mut failed_event = WorldEvent::new(
            WorldEventType::Migration,
            EpochId::new(1),
            WorldEventData::None,
        );
        failed_event.mark_failed("Test failure".to_string());
        assert!(matches!(failed_event.status, EventStatus::Failed(_)));
    }

    #[test]
    fn test_event_filter() {
        let event1 = WorldEvent::new(
            WorldEventType::EntityCreated,
            EpochId::new(1),
            WorldEventData::None,
        )
        .with_initiator(EntityId::new("creator-1"))
        .with_region(RegionId::new("region-a"));

        let event2 = WorldEvent::new(
            WorldEventType::MapleTransfer,
            EpochId::new(2),
            WorldEventData::None,
        )
        .with_initiator(EntityId::new("creator-2"));

        // Filter by epoch
        let filter = EventFilter::new().epoch(EpochId::new(1));
        assert!(filter.matches(&event1));
        assert!(!filter.matches(&event2));

        // Filter by event type
        let filter = EventFilter::new().event_type(WorldEventType::MapleTransfer);
        assert!(!filter.matches(&event1));
        assert!(filter.matches(&event2));

        // Filter by initiator
        let filter = EventFilter::new().initiator(EntityId::new("creator-1"));
        assert!(filter.matches(&event1));
        assert!(!filter.matches(&event2));

        // Filter by region
        let filter = EventFilter::new().region(RegionId::new("region-a"));
        assert!(filter.matches(&event1));
        assert!(!filter.matches(&event2));
    }

    #[test]
    fn test_event_log() {
        let mut log = EventLog::new();

        let event1 = WorldEvent::new(
            WorldEventType::EntityCreated,
            EpochId::new(1),
            WorldEventData::None,
        )
        .with_initiator(EntityId::new("creator"));

        let event2 = WorldEvent::new(
            WorldEventType::MapleTransfer,
            EpochId::new(1),
            WorldEventData::None,
        )
        .with_affected(EntityId::new("receiver"));

        let id1 = event1.id.clone();
        let id2 = event2.id.clone();

        log.add(event1);
        log.add(event2);

        assert_eq!(log.len(), 2);

        // Get by ID
        assert!(log.get(&id1).is_some());
        assert!(log.get(&id2).is_some());

        // Get by epoch
        let epoch_events = log.get_by_epoch(EpochId::new(1));
        assert_eq!(epoch_events.len(), 2);

        // Get by entity
        let creator_events = log.get_by_entity(&EntityId::new("creator"));
        assert_eq!(creator_events.len(), 1);
    }

    #[test]
    fn test_event_log_query() {
        let mut log = EventLog::new();

        for i in 0..10 {
            let event = WorldEvent::new(
                if i % 2 == 0 {
                    WorldEventType::EntityCreated
                } else {
                    WorldEventType::MapleTransfer
                },
                EpochId::new((i / 3) as u64),
                WorldEventData::None,
            );
            log.add(event);
        }

        // Query with filter
        let filter = EventFilter::new()
            .event_type(WorldEventType::EntityCreated)
            .limit(3);

        let results = log.query(&filter);
        assert!(results.len() <= 3);
        assert!(results
            .iter()
            .all(|e| e.event_type == WorldEventType::EntityCreated));
    }

    #[test]
    fn test_world_event_data_variants() {
        let data_variants = vec![
            WorldEventData::None,
            WorldEventData::EntityCreation {
                entity_id: EntityId::new("e1"),
                entity_name: "Test".to_string(),
                region_id: RegionId::new("r1"),
            },
            WorldEventData::MapleTransfer {
                from: EntityId::new("sender"),
                to: EntityId::new("receiver"),
                amount: 100,
                fee: 1,
                reference: Some("ref".to_string()),
            },
            WorldEventData::Migration {
                entity_id: EntityId::new("e1"),
                from_region: RegionId::new("r1"),
                to_region: RegionId::new("r2"),
            },
        ];

        for data in data_variants {
            let json = serde_json::to_string(&data).unwrap();
            let _: WorldEventData = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_event_types() {
        let types = vec![
            WorldEventType::EntityCreated,
            WorldEventType::EntityTerminated,
            WorldEventType::MapleTransfer,
            WorldEventType::Migration,
            WorldEventType::CollectiveFormed,
            WorldEventType::EpochStarted,
            WorldEventType::Custom("test".to_string()),
        ];

        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let _: WorldEventType = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_epoch_serialization() {
        let epoch = Epoch::new(EpochId::new(42));
        let json = serde_json::to_string(&epoch).unwrap();
        let deserialized: Epoch = serde_json::from_str(&json).unwrap();
        assert_eq!(epoch.id, deserialized.id);
    }
}
