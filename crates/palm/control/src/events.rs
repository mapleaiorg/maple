//! Event aggregation from all PALM subsystems
//!
//! The EventAggregator collects events from all PALM subsystems (deployment,
//! health, state) and provides a unified stream for monitoring and audit.

use palm_types::{EventSeverity, EventSource, PalmEvent, PalmEventEnvelope, PlatformProfile};
use tokio::sync::broadcast;
use tracing::debug;

/// Channel capacity for the unified event stream
const EVENT_CHANNEL_CAPACITY: usize = 16384;

/// Aggregates events from all PALM subsystems into a single stream
pub struct EventAggregator {
    /// Sender for the unified event stream
    unified_tx: broadcast::Sender<PalmEventEnvelope>,
}

impl EventAggregator {
    /// Create a new event aggregator
    pub fn new() -> Self {
        let (unified_tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Self { unified_tx }
    }

    /// Subscribe to the unified event stream
    pub fn subscribe(&self) -> broadcast::Receiver<PalmEventEnvelope> {
        self.unified_tx.subscribe()
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.unified_tx.receiver_count()
    }

    /// Forward events from a subsystem receiver
    ///
    /// This method runs until the source channel is closed.
    pub async fn forward_from(
        &self,
        mut source_rx: broadcast::Receiver<PalmEventEnvelope>,
        source_name: &'static str,
    ) {
        loop {
            match source_rx.recv().await {
                Ok(event) => {
                    debug!(
                        source = source_name,
                        event_id = %event.id,
                        "Forwarding event from subsystem"
                    );
                    // Send to unified stream, ignore errors (no subscribers is fine)
                    let _ = self.unified_tx.send(event);
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(
                        source = source_name,
                        lagged = n,
                        "Event aggregator lagged behind source"
                    );
                    // Continue processing, we just missed some events
                }
                Err(broadcast::error::RecvError::Closed) => {
                    debug!(source = source_name, "Source channel closed");
                    break;
                }
            }
        }
    }

    /// Emit a control plane event
    pub fn emit(&self, event: PalmEvent, platform: PlatformProfile, severity: EventSeverity) {
        let envelope = PalmEventEnvelope {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source: EventSource::ControlPlane,
            severity,
            platform,
            correlation_id: None,
            actor: None,
            event,
        };
        let _ = self.unified_tx.send(envelope);
    }

    /// Emit an event with full context
    pub fn emit_with_context(
        &self,
        event: PalmEvent,
        platform: PlatformProfile,
        severity: EventSeverity,
        correlation_id: Option<String>,
        actor: Option<String>,
    ) {
        let envelope = PalmEventEnvelope {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source: EventSource::ControlPlane,
            severity,
            platform,
            correlation_id,
            actor,
            event,
        };
        let _ = self.unified_tx.send(envelope);
    }

    /// Emit an info-level event
    pub fn emit_info(&self, event: PalmEvent, platform: PlatformProfile) {
        self.emit(event, platform, EventSeverity::Info);
    }

    /// Emit a warning-level event
    pub fn emit_warning(&self, event: PalmEvent, platform: PlatformProfile) {
        self.emit(event, platform, EventSeverity::Warning);
    }

    /// Emit an error-level event
    pub fn emit_error(&self, event: PalmEvent, platform: PlatformProfile) {
        self.emit(event, platform, EventSeverity::Error);
    }

    /// Emit a critical-level event
    pub fn emit_critical(&self, event: PalmEvent, platform: PlatformProfile) {
        self.emit(event, platform, EventSeverity::Critical);
    }
}

impl Default for EventAggregator {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for EventAggregator {
    fn clone(&self) -> Self {
        Self {
            unified_tx: self.unified_tx.clone(),
        }
    }
}

/// Builder for constructing event forwarding pipelines
pub struct EventForwarderBuilder {
    aggregator: EventAggregator,
    handles: Vec<tokio::task::JoinHandle<()>>,
}

impl EventForwarderBuilder {
    /// Create a new forwarder builder
    pub fn new(aggregator: EventAggregator) -> Self {
        Self {
            aggregator,
            handles: Vec::new(),
        }
    }

    /// Add a source to forward events from
    pub fn add_source(
        mut self,
        source_rx: broadcast::Receiver<PalmEventEnvelope>,
        source_name: &'static str,
    ) -> Self {
        let aggregator = self.aggregator.clone();
        let handle = tokio::spawn(async move {
            aggregator.forward_from(source_rx, source_name).await;
        });
        self.handles.push(handle);
        self
    }

    /// Build and return the handles for the forwarding tasks
    pub fn build(self) -> Vec<tokio::task::JoinHandle<()>> {
        self.handles
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use palm_types::{AgentSpecId, DeploymentId};
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_event_emission() {
        let aggregator = EventAggregator::new();
        let mut rx = aggregator.subscribe();

        aggregator.emit_info(
            PalmEvent::DeploymentCreated {
                deployment_id: DeploymentId::generate(),
                spec_id: AgentSpecId::generate(),
            },
            PlatformProfile::Development,
        );

        let received = timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("timeout")
            .expect("receive error");

        assert_eq!(received.source, EventSource::ControlPlane);
        assert_eq!(received.severity, EventSeverity::Info);
        assert_eq!(received.platform, PlatformProfile::Development);
    }

    #[tokio::test]
    async fn test_event_forwarding() {
        let aggregator = EventAggregator::new();
        let mut unified_rx = aggregator.subscribe();

        // Create a source channel
        let (source_tx, source_rx) = broadcast::channel::<PalmEventEnvelope>(16);

        // Start forwarding
        let agg_clone = aggregator.clone();
        let forward_handle = tokio::spawn(async move {
            agg_clone.forward_from(source_rx, "test-source").await;
        });

        // Send an event through the source
        let envelope = PalmEventEnvelope {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source: EventSource::Deployment,
            severity: EventSeverity::Info,
            platform: PlatformProfile::Mapleverse,
            correlation_id: None,
            actor: None,
            event: PalmEvent::DeploymentCompleted {
                deployment_id: DeploymentId::generate(),
                duration_seconds: 60,
            },
        };
        source_tx.send(envelope.clone()).unwrap();

        // Should receive the forwarded event
        let received = timeout(Duration::from_millis(100), unified_rx.recv())
            .await
            .expect("timeout")
            .expect("receive error");

        assert_eq!(received.id, envelope.id);
        assert_eq!(received.source, EventSource::Deployment);

        // Clean up
        drop(source_tx);
        let _ = forward_handle.await;
    }

    #[tokio::test]
    async fn test_subscriber_count() {
        let aggregator = EventAggregator::new();

        assert_eq!(aggregator.subscriber_count(), 0);

        let _rx1 = aggregator.subscribe();
        assert_eq!(aggregator.subscriber_count(), 1);

        let _rx2 = aggregator.subscribe();
        assert_eq!(aggregator.subscriber_count(), 2);
    }

    #[test]
    fn test_severity_helpers() {
        let aggregator = EventAggregator::new();
        let mut rx = aggregator.subscribe();

        aggregator.emit_warning(
            PalmEvent::DeploymentPaused {
                deployment_id: DeploymentId::generate(),
            },
            PlatformProfile::Development,
        );

        // Non-blocking check since we're in a sync test
        match rx.try_recv() {
            Ok(envelope) => {
                assert_eq!(envelope.severity, EventSeverity::Warning);
            }
            Err(_) => panic!("Expected to receive event"),
        }
    }
}
