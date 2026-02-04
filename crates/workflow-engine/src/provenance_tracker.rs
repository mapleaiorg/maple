//! Provenance tracker: records all workflow state changes
//!
//! Every state transition in a workflow is recorded for auditability.
//! The provenance tracker maintains an ordered chain of events that
//! can be replayed to understand exactly what happened during execution.

use chrono::{DateTime, Utc};
use resonator_types::ResonatorId;
use serde::{Deserialize, Serialize};
use workflow_types::{NodeId, WorkflowInstanceId};

/// Tracks all provenance events for workflow instances
#[derive(Clone, Debug)]
pub struct ProvenanceTracker {
    /// Events indexed by instance ID
    events: std::collections::HashMap<WorkflowInstanceId, Vec<ProvenanceRecord>>,
}

impl ProvenanceTracker {
    pub fn new() -> Self {
        Self {
            events: std::collections::HashMap::new(),
        }
    }

    /// Record a provenance event
    pub fn record(&mut self, instance_id: &WorkflowInstanceId, record: ProvenanceRecord) {
        let events = self.events.entry(instance_id.clone()).or_default();
        tracing::trace!(
            instance = %instance_id,
            event = %record.event_type,
            "Provenance recorded"
        );
        events.push(record);
    }

    /// Record a workflow started event
    pub fn record_started(&mut self, instance_id: &WorkflowInstanceId, initiator: &ResonatorId) {
        self.record(
            instance_id,
            ProvenanceRecord::new(
                ProvenanceEventType::WorkflowStarted,
                format!("Workflow started by {}", initiator),
            )
            .with_actor(initiator.clone()),
        );
    }

    /// Record a node activation
    pub fn record_node_activated(&mut self, instance_id: &WorkflowInstanceId, node_id: &NodeId) {
        self.record(
            instance_id,
            ProvenanceRecord::new(
                ProvenanceEventType::NodeActivated {
                    node_id: node_id.clone(),
                },
                format!("Node '{}' activated", node_id),
            ),
        );
    }

    /// Record a node completion
    pub fn record_node_completed(
        &mut self,
        instance_id: &WorkflowInstanceId,
        node_id: &NodeId,
        actor: Option<&ResonatorId>,
    ) {
        let mut record = ProvenanceRecord::new(
            ProvenanceEventType::NodeCompleted {
                node_id: node_id.clone(),
            },
            format!("Node '{}' completed", node_id),
        );
        if let Some(actor) = actor {
            record = record.with_actor(actor.clone());
        }
        self.record(instance_id, record);
    }

    /// Record a receipt emission
    pub fn record_receipt_emitted(
        &mut self,
        instance_id: &WorkflowInstanceId,
        node_id: &NodeId,
        receipt_id: &str,
        emitter: &ResonatorId,
    ) {
        self.record(
            instance_id,
            ProvenanceRecord::new(
                ProvenanceEventType::ReceiptEmitted {
                    node_id: node_id.clone(),
                    receipt_id: receipt_id.to_string(),
                },
                format!(
                    "Receipt '{}' emitted at node '{}' by {}",
                    receipt_id, node_id, emitter
                ),
            )
            .with_actor(emitter.clone()),
        );
    }

    /// Record a transition
    pub fn record_transition(
        &mut self,
        instance_id: &WorkflowInstanceId,
        from: &NodeId,
        to: &NodeId,
    ) {
        self.record(
            instance_id,
            ProvenanceRecord::new(
                ProvenanceEventType::TransitionFired {
                    from: from.clone(),
                    to: to.clone(),
                },
                format!("Transition fired: {} -> {}", from, to),
            ),
        );
    }

    /// Record an escalation
    pub fn record_escalation(
        &mut self,
        instance_id: &WorkflowInstanceId,
        node_id: &NodeId,
        reason: &str,
    ) {
        self.record(
            instance_id,
            ProvenanceRecord::new(
                ProvenanceEventType::EscalationTriggered {
                    node_id: node_id.clone(),
                    reason: reason.to_string(),
                },
                format!("Escalation at node '{}': {}", node_id, reason),
            ),
        );
    }

    /// Record workflow completion
    pub fn record_completed(&mut self, instance_id: &WorkflowInstanceId) {
        self.record(
            instance_id,
            ProvenanceRecord::new(
                ProvenanceEventType::WorkflowCompleted,
                "Workflow completed successfully",
            ),
        );
    }

    /// Record workflow failure
    pub fn record_failed(&mut self, instance_id: &WorkflowInstanceId, reason: &str) {
        self.record(
            instance_id,
            ProvenanceRecord::new(
                ProvenanceEventType::WorkflowFailed {
                    reason: reason.to_string(),
                },
                format!("Workflow failed: {}", reason),
            ),
        );
    }

    // ── Query methods ────────────────────────────────────────────────

    /// Get all events for an instance
    pub fn events_for(&self, instance_id: &WorkflowInstanceId) -> Vec<&ProvenanceRecord> {
        self.events
            .get(instance_id)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Get event count for an instance
    pub fn event_count(&self, instance_id: &WorkflowInstanceId) -> usize {
        self.events.get(instance_id).map(|v| v.len()).unwrap_or(0)
    }

    /// Get events for a specific node
    pub fn events_for_node(
        &self,
        instance_id: &WorkflowInstanceId,
        node_id: &NodeId,
    ) -> Vec<&ProvenanceRecord> {
        self.events_for(instance_id)
            .into_iter()
            .filter(|r| r.involves_node(node_id))
            .collect()
    }

    /// Clear events for an instance (for cleanup)
    pub fn clear(&mut self, instance_id: &WorkflowInstanceId) {
        self.events.remove(instance_id);
    }

    /// Total events across all instances
    pub fn total_events(&self) -> usize {
        self.events.values().map(|v| v.len()).sum()
    }
}

impl Default for ProvenanceTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// A provenance record — one event in the provenance chain
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    /// The type of event
    pub event_type: ProvenanceEventType,
    /// Human-readable description
    pub description: String,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// Who caused this event (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<ResonatorId>,
}

impl ProvenanceRecord {
    pub fn new(event_type: ProvenanceEventType, description: impl Into<String>) -> Self {
        Self {
            event_type,
            description: description.into(),
            timestamp: Utc::now(),
            actor: None,
        }
    }

    pub fn with_actor(mut self, actor: ResonatorId) -> Self {
        self.actor = Some(actor);
        self
    }

    /// Check if this record involves a specific node
    pub fn involves_node(&self, node_id: &NodeId) -> bool {
        match &self.event_type {
            ProvenanceEventType::NodeActivated { node_id: n } => n == node_id,
            ProvenanceEventType::NodeCompleted { node_id: n } => n == node_id,
            ProvenanceEventType::ReceiptEmitted { node_id: n, .. } => n == node_id,
            ProvenanceEventType::TransitionFired { from, to } => from == node_id || to == node_id,
            ProvenanceEventType::EscalationTriggered { node_id: n, .. } => n == node_id,
            _ => false,
        }
    }
}

/// Types of provenance events
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProvenanceEventType {
    /// Workflow was started
    WorkflowStarted,
    /// A node was activated
    NodeActivated { node_id: NodeId },
    /// A node was completed
    NodeCompleted { node_id: NodeId },
    /// A receipt was emitted
    ReceiptEmitted { node_id: NodeId, receipt_id: String },
    /// A transition was fired
    TransitionFired { from: NodeId, to: NodeId },
    /// An escalation was triggered
    EscalationTriggered { node_id: NodeId, reason: String },
    /// Workflow completed
    WorkflowCompleted,
    /// Workflow failed
    WorkflowFailed { reason: String },
    /// Workflow paused
    WorkflowPaused { reason: String },
    /// Workflow resumed
    WorkflowResumed,
    /// Workflow cancelled
    WorkflowCancelled { reason: String },
}

impl std::fmt::Display for ProvenanceEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkflowStarted => write!(f, "workflow_started"),
            Self::NodeActivated { node_id } => write!(f, "node_activated:{}", node_id),
            Self::NodeCompleted { node_id } => write!(f, "node_completed:{}", node_id),
            Self::ReceiptEmitted { node_id, .. } => {
                write!(f, "receipt_emitted:{}", node_id)
            }
            Self::TransitionFired { from, to } => {
                write!(f, "transition:{}→{}", from, to)
            }
            Self::EscalationTriggered { node_id, .. } => {
                write!(f, "escalation:{}", node_id)
            }
            Self::WorkflowCompleted => write!(f, "workflow_completed"),
            Self::WorkflowFailed { .. } => write!(f, "workflow_failed"),
            Self::WorkflowPaused { .. } => write!(f, "workflow_paused"),
            Self::WorkflowResumed => write!(f, "workflow_resumed"),
            Self::WorkflowCancelled { .. } => write!(f, "workflow_cancelled"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_provenance() {
        let mut tracker = ProvenanceTracker::new();
        let inst_id = WorkflowInstanceId::new("inst-1");

        tracker.record_started(&inst_id, &ResonatorId::new("init"));
        tracker.record_node_activated(&inst_id, &NodeId::new("review"));
        tracker.record_node_completed(
            &inst_id,
            &NodeId::new("review"),
            Some(&ResonatorId::new("reviewer")),
        );
        tracker.record_completed(&inst_id);

        assert_eq!(tracker.event_count(&inst_id), 4);
        assert_eq!(tracker.total_events(), 4);
    }

    #[test]
    fn test_events_for_node() {
        let mut tracker = ProvenanceTracker::new();
        let inst_id = WorkflowInstanceId::new("inst-1");

        tracker.record_node_activated(&inst_id, &NodeId::new("a"));
        tracker.record_node_activated(&inst_id, &NodeId::new("b"));
        tracker.record_node_completed(&inst_id, &NodeId::new("a"), None);

        let events_a = tracker.events_for_node(&inst_id, &NodeId::new("a"));
        assert_eq!(events_a.len(), 2); // activated + completed

        let events_b = tracker.events_for_node(&inst_id, &NodeId::new("b"));
        assert_eq!(events_b.len(), 1); // only activated
    }

    #[test]
    fn test_record_receipt() {
        let mut tracker = ProvenanceTracker::new();
        let inst_id = WorkflowInstanceId::new("inst-1");

        tracker.record_receipt_emitted(
            &inst_id,
            &NodeId::new("review"),
            "receipt-123",
            &ResonatorId::new("reviewer"),
        );

        let events = tracker.events_for(&inst_id);
        assert_eq!(events.len(), 1);
        assert!(events[0].actor.is_some());
    }

    #[test]
    fn test_record_transition() {
        let mut tracker = ProvenanceTracker::new();
        let inst_id = WorkflowInstanceId::new("inst-1");

        tracker.record_transition(&inst_id, &NodeId::new("review"), &NodeId::new("approve"));

        let events = tracker.events_for_node(&inst_id, &NodeId::new("review"));
        assert_eq!(events.len(), 1);

        // Also shows up for target node
        let events = tracker.events_for_node(&inst_id, &NodeId::new("approve"));
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_record_escalation() {
        let mut tracker = ProvenanceTracker::new();
        let inst_id = WorkflowInstanceId::new("inst-1");

        tracker.record_escalation(&inst_id, &NodeId::new("task"), "Timeout exceeded");

        let events = tracker.events_for_node(&inst_id, &NodeId::new("task"));
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_clear() {
        let mut tracker = ProvenanceTracker::new();
        let inst_id = WorkflowInstanceId::new("inst-1");

        tracker.record_started(&inst_id, &ResonatorId::new("init"));
        assert_eq!(tracker.event_count(&inst_id), 1);

        tracker.clear(&inst_id);
        assert_eq!(tracker.event_count(&inst_id), 0);
    }

    #[test]
    fn test_record_failed() {
        let mut tracker = ProvenanceTracker::new();
        let inst_id = WorkflowInstanceId::new("inst-1");

        tracker.record_failed(&inst_id, "Budget exhausted");

        let events = tracker.events_for(&inst_id);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0].event_type,
            ProvenanceEventType::WorkflowFailed { .. }
        ));
    }

    #[test]
    fn test_involves_node() {
        let record = ProvenanceRecord::new(
            ProvenanceEventType::TransitionFired {
                from: NodeId::new("a"),
                to: NodeId::new("b"),
            },
            "Transition",
        );
        assert!(record.involves_node(&NodeId::new("a")));
        assert!(record.involves_node(&NodeId::new("b")));
        assert!(!record.involves_node(&NodeId::new("c")));

        let started = ProvenanceRecord::new(ProvenanceEventType::WorkflowStarted, "Started");
        assert!(!started.involves_node(&NodeId::new("a")));
    }

    #[test]
    fn test_event_type_display() {
        let evt = ProvenanceEventType::TransitionFired {
            from: NodeId::new("a"),
            to: NodeId::new("b"),
        };
        assert_eq!(format!("{}", evt), "transition:a→b");

        let evt = ProvenanceEventType::WorkflowStarted;
        assert_eq!(format!("{}", evt), "workflow_started");
    }

    #[test]
    fn test_empty_instance() {
        let tracker = ProvenanceTracker::new();
        let inst_id = WorkflowInstanceId::new("nonexistent");

        assert_eq!(tracker.event_count(&inst_id), 0);
        assert!(tracker.events_for(&inst_id).is_empty());
    }
}
