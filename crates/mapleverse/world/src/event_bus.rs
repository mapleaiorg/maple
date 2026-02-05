//! Event bus for MapleVerse
//!
//! Provides event publication and subscription for world events.

use mapleverse_types::event::{EpochId, EventLog, WorldEvent, WorldEventId, WorldEventType};
use std::collections::HashMap;
use tokio::sync::broadcast;

/// Callback type for event handlers
pub type EventHandler = Box<dyn Fn(&WorldEvent) + Send + Sync>;

/// Event bus for publishing and subscribing to world events
pub struct EventBus {
    /// Event log for persistence
    event_log: EventLog,
    /// Broadcast channel for real-time event distribution
    sender: broadcast::Sender<WorldEvent>,
    /// Current epoch
    current_epoch: EpochId,
    /// Event counters by type
    event_counts: HashMap<String, u64>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            event_log: EventLog::new(),
            sender,
            current_epoch: EpochId::new(0),
            event_counts: HashMap::new(),
        }
    }

    /// Publish an event
    pub fn publish(&mut self, event: WorldEvent) {
        // Increment counter
        let type_name = format!("{:?}", event.event_type);
        *self.event_counts.entry(type_name).or_insert(0) += 1;

        // Log event
        self.event_log.add(event.clone());

        // Broadcast (ignore errors if no receivers)
        let _ = self.sender.send(event);
    }

    /// Create a new event and publish it
    pub fn emit(
        &mut self,
        event_type: WorldEventType,
        data: mapleverse_types::event::WorldEventData,
    ) -> WorldEventId {
        let event = WorldEvent::new(event_type, self.current_epoch, data);
        let event_id = event.id.clone();
        self.publish(event);
        event_id
    }

    /// Subscribe to events (returns a receiver)
    pub fn subscribe(&self) -> broadcast::Receiver<WorldEvent> {
        self.sender.subscribe()
    }

    /// Get event from log
    pub fn get_event(&self, event_id: &WorldEventId) -> Option<&WorldEvent> {
        self.event_log.get(event_id)
    }

    /// Get mutable event from log
    pub fn get_event_mut(&mut self, event_id: &WorldEventId) -> Option<&mut WorldEvent> {
        self.event_log.get_mut(event_id)
    }

    /// Get events for an epoch
    pub fn get_epoch_events(&self, epoch: EpochId) -> Vec<&WorldEvent> {
        self.event_log.get_by_epoch(epoch)
    }

    /// Get events for an entity
    pub fn get_entity_events(
        &self,
        entity_id: &mapleverse_types::entity::EntityId,
    ) -> Vec<&WorldEvent> {
        self.event_log.get_by_entity(entity_id)
    }

    /// Query events with filter
    pub fn query(&self, filter: &mapleverse_types::event::EventFilter) -> Vec<&WorldEvent> {
        self.event_log.query(filter)
    }

    /// Set current epoch
    pub fn set_epoch(&mut self, epoch: EpochId) {
        self.current_epoch = epoch;
    }

    /// Get current epoch
    pub fn current_epoch(&self) -> EpochId {
        self.current_epoch
    }

    /// Get total event count
    pub fn event_count(&self) -> usize {
        self.event_log.len()
    }

    /// Get event counts by type
    pub fn event_counts(&self) -> &HashMap<String, u64> {
        &self.event_counts
    }

    /// Get statistics
    pub fn stats(&self) -> EventBusStats {
        EventBusStats {
            total_events: self.event_log.len() as u64,
            current_epoch: self.current_epoch,
            subscriber_count: self.sender.receiver_count(),
            events_by_type: self.event_counts.clone(),
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Event bus statistics
#[derive(Clone, Debug)]
pub struct EventBusStats {
    /// Total events logged
    pub total_events: u64,
    /// Current epoch
    pub current_epoch: EpochId,
    /// Number of active subscribers
    pub subscriber_count: usize,
    /// Events by type
    pub events_by_type: HashMap<String, u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mapleverse_types::entity::EntityId;
    use mapleverse_types::event::WorldEventData;

    #[test]
    fn test_publish_event() {
        let mut bus = EventBus::new();

        let event = WorldEvent::new(
            WorldEventType::EntityCreated,
            EpochId::new(0),
            WorldEventData::None,
        );

        let event_id = event.id.clone();
        bus.publish(event);

        assert!(bus.get_event(&event_id).is_some());
        assert_eq!(bus.event_count(), 1);
    }

    #[test]
    fn test_emit_event() {
        let mut bus = EventBus::new();

        let event_id = bus.emit(WorldEventType::EntityCreated, WorldEventData::None);

        assert!(bus.get_event(&event_id).is_some());
    }

    #[test]
    fn test_get_epoch_events() {
        let mut bus = EventBus::new();
        bus.set_epoch(EpochId::new(1));

        bus.emit(WorldEventType::EntityCreated, WorldEventData::None);
        bus.emit(WorldEventType::MapleTransfer, WorldEventData::None);

        bus.set_epoch(EpochId::new(2));
        bus.emit(WorldEventType::Migration, WorldEventData::None);

        let epoch1_events = bus.get_epoch_events(EpochId::new(1));
        assert_eq!(epoch1_events.len(), 2);

        let epoch2_events = bus.get_epoch_events(EpochId::new(2));
        assert_eq!(epoch2_events.len(), 1);
    }

    #[test]
    fn test_event_counts() {
        let mut bus = EventBus::new();

        for _ in 0..3 {
            bus.emit(WorldEventType::EntityCreated, WorldEventData::None);
        }

        for _ in 0..2 {
            bus.emit(WorldEventType::MapleTransfer, WorldEventData::None);
        }

        let counts = bus.event_counts();
        assert_eq!(counts.get("EntityCreated"), Some(&3));
        assert_eq!(counts.get("MapleTransfer"), Some(&2));
    }

    #[tokio::test]
    async fn test_subscribe() {
        let mut bus = EventBus::new();
        let mut receiver = bus.subscribe();

        let event = WorldEvent::new(
            WorldEventType::EntityCreated,
            EpochId::new(0),
            WorldEventData::None,
        );
        let expected_id = event.id.clone();

        bus.publish(event);

        let received = receiver.recv().await.unwrap();
        assert_eq!(received.id, expected_id);
    }

    #[test]
    fn test_stats() {
        let mut bus = EventBus::new();
        bus.set_epoch(EpochId::new(5));

        for _ in 0..10 {
            bus.emit(WorldEventType::EntityCreated, WorldEventData::None);
        }

        let stats = bus.stats();
        assert_eq!(stats.total_events, 10);
        assert_eq!(stats.current_epoch, EpochId::new(5));
    }
}
