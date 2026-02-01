//! Correlation engine for linking related events

use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

/// Unique correlation identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorrelationId(pub String);

impl CorrelationId {
    /// Generate a new correlation ID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create from string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl Default for CorrelationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A correlated event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelatedEvent {
    /// Event ID
    pub event_id: String,

    /// Event type/name
    pub event_type: String,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Source component
    pub source: String,

    /// Platform
    pub platform: String,

    /// Correlation ID
    pub correlation_id: CorrelationId,

    /// Causation ID (event that caused this one)
    pub causation_id: Option<String>,

    /// Trace ID (for distributed tracing integration)
    pub trace_id: Option<String>,

    /// Related resource IDs
    pub resources: HashMap<String, String>,

    /// Event data
    pub data: HashMap<String, serde_json::Value>,
}

impl CorrelatedEvent {
    /// Create a new correlated event builder
    pub fn builder(event_type: impl Into<String>) -> CorrelatedEventBuilder {
        CorrelatedEventBuilder::new(event_type)
    }
}

/// Builder for correlated events
#[derive(Debug)]
pub struct CorrelatedEventBuilder {
    event_type: String,
    source: Option<String>,
    platform: Option<String>,
    correlation_id: Option<CorrelationId>,
    causation_id: Option<String>,
    trace_id: Option<String>,
    resources: HashMap<String, String>,
    data: HashMap<String, serde_json::Value>,
}

impl CorrelatedEventBuilder {
    /// Create a new builder
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            event_type: event_type.into(),
            source: None,
            platform: None,
            correlation_id: None,
            causation_id: None,
            trace_id: None,
            resources: HashMap::new(),
            data: HashMap::new(),
        }
    }

    /// Set source component
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set platform
    pub fn platform(mut self, platform: impl Into<String>) -> Self {
        self.platform = Some(platform.into());
        self
    }

    /// Set correlation ID
    pub fn correlation_id(mut self, id: CorrelationId) -> Self {
        self.correlation_id = Some(id);
        self
    }

    /// Set causation ID
    pub fn caused_by(mut self, event_id: impl Into<String>) -> Self {
        self.causation_id = Some(event_id.into());
        self
    }

    /// Set trace ID
    pub fn trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Add a resource reference
    pub fn resource(mut self, key: impl Into<String>, id: impl Into<String>) -> Self {
        self.resources.insert(key.into(), id.into());
        self
    }

    /// Add deployment reference
    pub fn deployment(self, id: impl Into<String>) -> Self {
        self.resource("deployment_id", id)
    }

    /// Add instance reference
    pub fn instance(self, id: impl Into<String>) -> Self {
        self.resource("instance_id", id)
    }

    /// Add data
    pub fn data(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.data.insert(key.into(), v);
        }
        self
    }

    /// Build the event
    pub fn build(self) -> CorrelatedEvent {
        CorrelatedEvent {
            event_id: Uuid::new_v4().to_string(),
            event_type: self.event_type,
            timestamp: Utc::now(),
            source: self.source.unwrap_or_else(|| "unknown".to_string()),
            platform: self.platform.unwrap_or_else(|| "unknown".to_string()),
            correlation_id: self.correlation_id.unwrap_or_else(CorrelationId::new),
            causation_id: self.causation_id,
            trace_id: self.trace_id,
            resources: self.resources,
            data: self.data,
        }
    }
}

/// Event correlation grouping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCorrelation {
    /// Correlation ID
    pub id: CorrelationId,

    /// Correlated events
    pub events: Vec<CorrelatedEvent>,

    /// Start time (first event)
    pub start_time: DateTime<Utc>,

    /// End time (last event)
    pub end_time: DateTime<Utc>,

    /// Root cause event (if identified)
    pub root_event_id: Option<String>,

    /// Affected resources
    pub affected_resources: HashMap<String, Vec<String>>,
}

impl EventCorrelation {
    /// Create from a list of events with the same correlation ID
    pub fn from_events(events: Vec<CorrelatedEvent>) -> Option<Self> {
        if events.is_empty() {
            return None;
        }

        let id = events[0].correlation_id.clone();
        let start_time = events.iter().map(|e| e.timestamp).min().unwrap();
        let end_time = events.iter().map(|e| e.timestamp).max().unwrap();

        // Find root event (one with no causation_id, or the earliest)
        let root_event_id = events
            .iter()
            .find(|e| e.causation_id.is_none())
            .or_else(|| events.iter().min_by_key(|e| e.timestamp))
            .map(|e| e.event_id.clone());

        // Collect affected resources
        let mut affected_resources: HashMap<String, Vec<String>> = HashMap::new();
        for event in &events {
            for (key, value) in &event.resources {
                affected_resources
                    .entry(key.clone())
                    .or_default()
                    .push(value.clone());
            }
        }

        // Deduplicate resource lists
        for values in affected_resources.values_mut() {
            values.sort();
            values.dedup();
        }

        Some(Self {
            id,
            events,
            start_time,
            end_time,
            root_event_id,
            affected_resources,
        })
    }

    /// Get duration of the correlation
    pub fn duration(&self) -> Duration {
        self.end_time - self.start_time
    }

    /// Get event count
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

/// Correlation engine for managing event correlations
pub struct CorrelationEngine {
    /// Active correlations by ID
    correlations: DashMap<CorrelationId, Vec<CorrelatedEvent>>,

    /// Event index by event ID
    event_index: DashMap<String, CorrelationId>,

    /// Resource index (resource_type:resource_id -> correlation IDs)
    resource_index: DashMap<String, Vec<CorrelationId>>,

    /// Total events processed
    total_events: AtomicU64,

    /// TTL for correlations (after last event)
    ttl: Duration,
}

impl CorrelationEngine {
    /// Create a new correlation engine
    pub fn new() -> Self {
        Self::with_ttl(Duration::hours(24))
    }

    /// Create with custom TTL
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            correlations: DashMap::new(),
            event_index: DashMap::new(),
            resource_index: DashMap::new(),
            total_events: AtomicU64::new(0),
            ttl,
        }
    }

    /// Add an event to the correlation engine
    pub fn add_event(&self, event: CorrelatedEvent) {
        let correlation_id = event.correlation_id.clone();
        let event_id = event.event_id.clone();

        // Add to correlation group
        self.correlations
            .entry(correlation_id.clone())
            .or_default()
            .push(event.clone());

        // Index by event ID
        self.event_index.insert(event_id, correlation_id.clone());

        // Index by resources
        for (key, value) in &event.resources {
            let resource_key = format!("{}:{}", key, value);
            self.resource_index
                .entry(resource_key)
                .or_default()
                .push(correlation_id.clone());
        }

        self.total_events.fetch_add(1, Ordering::Relaxed);
    }

    /// Get correlation by ID
    pub fn get_correlation(&self, id: &CorrelationId) -> Option<EventCorrelation> {
        self.correlations
            .get(id)
            .map(|events| EventCorrelation::from_events(events.clone()))
            .flatten()
    }

    /// Get correlation for an event
    pub fn get_correlation_for_event(&self, event_id: &str) -> Option<EventCorrelation> {
        self.event_index
            .get(event_id)
            .and_then(|id| self.get_correlation(&id))
    }

    /// Find correlations affecting a resource
    pub fn find_by_resource(
        &self,
        resource_type: &str,
        resource_id: &str,
    ) -> Vec<EventCorrelation> {
        let key = format!("{}:{}", resource_type, resource_id);
        self.resource_index
            .get(&key)
            .map(|ids| {
                // Deduplicate correlation IDs
                let mut unique_ids: Vec<_> = ids.iter().cloned().collect();
                unique_ids.sort_by(|a, b| a.0.cmp(&b.0));
                unique_ids.dedup_by(|a, b| a.0 == b.0);

                unique_ids
                    .iter()
                    .filter_map(|id| self.get_correlation(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find correlations in a time range
    pub fn find_in_time_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Vec<EventCorrelation> {
        self.correlations
            .iter()
            .filter_map(|entry| {
                let events = entry.value();
                if events.is_empty() {
                    return None;
                }

                let has_events_in_range = events
                    .iter()
                    .any(|e| e.timestamp >= from && e.timestamp < to);

                if has_events_in_range {
                    EventCorrelation::from_events(events.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get total events processed
    pub fn total_events(&self) -> u64 {
        self.total_events.load(Ordering::Relaxed)
    }

    /// Get active correlation count
    pub fn correlation_count(&self) -> usize {
        self.correlations.len()
    }

    /// Clean up expired correlations
    pub fn cleanup_expired(&self) {
        let cutoff = Utc::now() - self.ttl;
        let mut expired_ids = Vec::new();

        for entry in self.correlations.iter() {
            let events = entry.value();
            if let Some(last) = events.iter().map(|e| e.timestamp).max() {
                if last < cutoff {
                    expired_ids.push(entry.key().clone());
                }
            }
        }

        for id in expired_ids {
            if let Some((_, events)) = self.correlations.remove(&id) {
                // Clean up event index
                for event in &events {
                    self.event_index.remove(&event.event_id);
                }

                // Clean up resource index
                for event in &events {
                    for (key, value) in &event.resources {
                        let resource_key = format!("{}:{}", key, value);
                        if let Some(mut ids) = self.resource_index.get_mut(&resource_key) {
                            ids.retain(|i| i != &id);
                        }
                    }
                }
            }
        }
    }
}

impl Default for CorrelationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correlation_id() {
        let id1 = CorrelationId::new();
        let id2 = CorrelationId::new();
        assert_ne!(id1, id2);

        let id3 = CorrelationId::from_string("test-123");
        assert_eq!(id3.0, "test-123");
    }

    #[test]
    fn test_correlated_event_builder() {
        let event = CorrelatedEvent::builder("instance.started")
            .source("scheduler")
            .platform("development")
            .deployment("deploy-1")
            .instance("instance-1")
            .data("duration_ms", 1500)
            .build();

        assert_eq!(event.event_type, "instance.started");
        assert_eq!(event.source, "scheduler");
        assert!(event.resources.contains_key("deployment_id"));
        assert!(event.resources.contains_key("instance_id"));
    }

    #[test]
    fn test_event_correlation() {
        let correlation_id = CorrelationId::new();

        let events = vec![
            CorrelatedEvent::builder("deployment.created")
                .source("api")
                .platform("dev")
                .correlation_id(correlation_id.clone())
                .deployment("deploy-1")
                .build(),
            CorrelatedEvent::builder("instance.starting")
                .source("scheduler")
                .platform("dev")
                .correlation_id(correlation_id.clone())
                .deployment("deploy-1")
                .instance("instance-1")
                .build(),
            CorrelatedEvent::builder("instance.started")
                .source("scheduler")
                .platform("dev")
                .correlation_id(correlation_id.clone())
                .deployment("deploy-1")
                .instance("instance-1")
                .build(),
        ];

        let correlation = EventCorrelation::from_events(events).unwrap();
        assert_eq!(correlation.event_count(), 3);
        assert!(correlation.affected_resources.contains_key("deployment_id"));
        assert!(correlation.affected_resources.contains_key("instance_id"));
    }

    #[test]
    fn test_correlation_engine() {
        let engine = CorrelationEngine::new();
        let correlation_id = CorrelationId::new();

        let event1 = CorrelatedEvent::builder("deployment.created")
            .source("api")
            .platform("dev")
            .correlation_id(correlation_id.clone())
            .deployment("deploy-1")
            .build();

        let event1_id = event1.event_id.clone();
        engine.add_event(event1);

        let event2 = CorrelatedEvent::builder("instance.started")
            .source("scheduler")
            .platform("dev")
            .correlation_id(correlation_id.clone())
            .deployment("deploy-1")
            .instance("instance-1")
            .caused_by(&event1_id)
            .build();

        engine.add_event(event2);

        assert_eq!(engine.total_events(), 2);
        assert_eq!(engine.correlation_count(), 1);

        let correlation = engine.get_correlation(&correlation_id).unwrap();
        assert_eq!(correlation.event_count(), 2);

        // Find by resource
        let by_deployment = engine.find_by_resource("deployment_id", "deploy-1");
        assert_eq!(by_deployment.len(), 1);

        let by_instance = engine.find_by_resource("instance_id", "instance-1");
        assert_eq!(by_instance.len(), 1);
    }

    #[test]
    fn test_find_by_event() {
        let engine = CorrelationEngine::new();
        let correlation_id = CorrelationId::new();

        let event = CorrelatedEvent::builder("test.event")
            .source("test")
            .platform("dev")
            .correlation_id(correlation_id.clone())
            .build();

        let event_id = event.event_id.clone();
        engine.add_event(event);

        let correlation = engine.get_correlation_for_event(&event_id).unwrap();
        assert_eq!(correlation.id, correlation_id);
    }
}
