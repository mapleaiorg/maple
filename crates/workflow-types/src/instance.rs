//! Workflow instances: running executions of workflow definitions
//!
//! A WorkflowInstance tracks the runtime state of a workflow:
//! which nodes are active, what receipts have been collected,
//! and the provenance chain of all state transitions.

use crate::{EscalationState, NodeId, WorkflowDefinitionId, WorkflowReceipt};
use chrono::{DateTime, Utc};
use collective_types::{CollectiveId, RoleId};
use resonator_types::ResonatorId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Instance Identifier ──────────────────────────────────────────────

/// Unique identifier for a workflow instance
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowInstanceId(pub String);

impl WorkflowInstanceId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn short(&self) -> &str {
        &self.0[..8.min(self.0.len())]
    }
}

impl std::fmt::Display for WorkflowInstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Workflow Instance ────────────────────────────────────────────────

/// A running instance of a workflow definition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowInstance {
    /// Unique instance identifier
    pub id: WorkflowInstanceId,
    /// The definition this instance was created from
    pub definition_id: WorkflowDefinitionId,
    /// The collective running this workflow
    pub collective_id: CollectiveId,
    /// Who initiated this workflow
    pub initiator: ResonatorId,
    /// Current state
    pub state: WorkflowState,
    /// Node states: tracking each node's execution status
    pub node_states: HashMap<NodeId, NodeState>,
    /// Role assignments: which resonators are assigned to workflow roles
    pub role_assignments: HashMap<RoleId, Vec<ResonatorId>>,
    /// All receipts collected during execution
    pub receipts: Vec<WorkflowReceipt>,
    /// Provenance chain: ordered record of all state transitions
    pub provenance: Vec<ProvenanceEntry>,
    /// Runtime parameters (instantiation values)
    pub parameters: HashMap<String, String>,
    /// When the instance was created
    pub created_at: DateTime<Utc>,
    /// When the instance was last updated
    pub updated_at: DateTime<Utc>,
    /// When the instance completed (if completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Deadline for the entire workflow
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<DateTime<Utc>>,
    /// Metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl WorkflowInstance {
    /// Create a new workflow instance
    pub fn new(
        definition_id: WorkflowDefinitionId,
        collective_id: CollectiveId,
        initiator: ResonatorId,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: WorkflowInstanceId::generate(),
            definition_id,
            collective_id,
            initiator,
            state: WorkflowState::Created,
            node_states: HashMap::new(),
            role_assignments: HashMap::new(),
            receipts: Vec::new(),
            provenance: Vec::new(),
            parameters: HashMap::new(),
            created_at: now,
            updated_at: now,
            completed_at: None,
            deadline: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_parameter(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.parameters.insert(key.into(), value.into());
        self
    }

    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Start the workflow (transition from Created to Active)
    pub fn start(&mut self) {
        self.state = WorkflowState::Active;
        self.updated_at = Utc::now();
        self.record_provenance("workflow_started", "Workflow instance started");
    }

    /// Activate a node
    pub fn activate_node(&mut self, node_id: NodeId) {
        let now = Utc::now();
        self.node_states.insert(
            node_id.clone(),
            NodeState {
                status: NodeStatus::Active,
                activated_at: Some(now),
                completed_at: None,
                assigned_resonator: None,
                escalation: EscalationState::new(),
                retry_count: 0,
            },
        );
        self.updated_at = now;
        self.record_provenance("node_activated", format!("Node '{}' activated", node_id));
    }

    /// Complete a node
    pub fn complete_node(&mut self, node_id: &NodeId) {
        if let Some(state) = self.node_states.get_mut(node_id) {
            state.status = NodeStatus::Completed;
            state.completed_at = Some(Utc::now());
        }
        self.updated_at = Utc::now();
        self.record_provenance("node_completed", format!("Node '{}' completed", node_id));
    }

    /// Fail a node
    pub fn fail_node(&mut self, node_id: &NodeId, reason: impl Into<String>) {
        let reason = reason.into();
        if let Some(state) = self.node_states.get_mut(node_id) {
            state.status = NodeStatus::Failed;
            state.completed_at = Some(Utc::now());
        }
        self.updated_at = Utc::now();
        self.record_provenance(
            "node_failed",
            format!("Node '{}' failed: {}", node_id, reason),
        );
    }

    /// Skip a node
    pub fn skip_node(&mut self, node_id: &NodeId) {
        if let Some(state) = self.node_states.get_mut(node_id) {
            state.status = NodeStatus::Skipped;
            state.completed_at = Some(Utc::now());
        }
        self.updated_at = Utc::now();
        self.record_provenance("node_skipped", format!("Node '{}' skipped", node_id));
    }

    /// Assign a resonator to a role
    pub fn assign_role(&mut self, role: RoleId, resonator: ResonatorId) {
        self.role_assignments
            .entry(role.clone())
            .or_default()
            .push(resonator.clone());
        self.record_provenance(
            "role_assigned",
            format!("Resonator '{}' assigned to role '{}'", resonator, role),
        );
    }

    /// Assign a resonator to a specific node
    pub fn assign_resonator_to_node(&mut self, node_id: &NodeId, resonator: ResonatorId) {
        if let Some(state) = self.node_states.get_mut(node_id) {
            state.assigned_resonator = Some(resonator.clone());
        }
        self.record_provenance(
            "resonator_assigned",
            format!("Resonator '{}' assigned to node '{}'", resonator, node_id),
        );
    }

    /// Add a receipt
    pub fn add_receipt(&mut self, receipt: WorkflowReceipt) {
        self.record_provenance(
            "receipt_emitted",
            format!(
                "Receipt '{}' emitted by node '{}'",
                receipt.receipt_id, receipt.node_id
            ),
        );
        self.receipts.push(receipt);
        self.updated_at = Utc::now();
    }

    /// Complete the workflow
    pub fn complete(&mut self) {
        self.state = WorkflowState::Completed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        self.record_provenance("workflow_completed", "Workflow instance completed");
    }

    /// Fail the workflow
    pub fn fail(&mut self, reason: impl Into<String>) {
        let reason = reason.into();
        self.state = WorkflowState::Failed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        self.record_provenance("workflow_failed", format!("Workflow failed: {}", reason));
    }

    /// Pause the workflow
    pub fn pause(&mut self, reason: impl Into<String>) {
        let reason = reason.into();
        self.state = WorkflowState::Paused;
        self.updated_at = Utc::now();
        self.record_provenance("workflow_paused", format!("Workflow paused: {}", reason));
    }

    /// Resume a paused workflow
    pub fn resume(&mut self) {
        self.state = WorkflowState::Active;
        self.updated_at = Utc::now();
        self.record_provenance("workflow_resumed", "Workflow instance resumed");
    }

    /// Cancel the workflow
    pub fn cancel(&mut self, reason: impl Into<String>) {
        let reason = reason.into();
        self.state = WorkflowState::Cancelled;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        self.record_provenance(
            "workflow_cancelled",
            format!("Workflow cancelled: {}", reason),
        );
    }

    // ── Query methods ────────────────────────────────────────────────

    /// Check if the workflow is active
    pub fn is_active(&self) -> bool {
        self.state == WorkflowState::Active
    }

    /// Check if the workflow is terminal (completed, failed, or cancelled)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            WorkflowState::Completed | WorkflowState::Failed | WorkflowState::Cancelled
        )
    }

    /// Get all currently active nodes
    pub fn active_nodes(&self) -> Vec<&NodeId> {
        self.node_states
            .iter()
            .filter(|(_, s)| s.status == NodeStatus::Active)
            .map(|(id, _)| id)
            .collect()
    }

    /// Get all completed nodes
    pub fn completed_nodes(&self) -> Vec<&NodeId> {
        self.node_states
            .iter()
            .filter(|(_, s)| s.status == NodeStatus::Completed)
            .map(|(id, _)| id)
            .collect()
    }

    /// Get node state
    pub fn get_node_state(&self, node_id: &NodeId) -> Option<&NodeState> {
        self.node_states.get(node_id)
    }

    /// Get receipts for a specific node
    pub fn receipts_for_node(&self, node_id: &NodeId) -> Vec<&WorkflowReceipt> {
        self.receipts
            .iter()
            .filter(|r| &r.node_id == node_id)
            .collect()
    }

    /// Get resonators assigned to a role
    pub fn resonators_for_role(&self, role: &RoleId) -> Vec<&ResonatorId> {
        self.role_assignments
            .get(role)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Total receipts collected
    pub fn receipt_count(&self) -> usize {
        self.receipts.len()
    }

    /// Total provenance entries
    pub fn provenance_count(&self) -> usize {
        self.provenance.len()
    }

    /// Check if the workflow has expired
    pub fn is_expired(&self) -> bool {
        match self.deadline {
            Some(deadline) => Utc::now() >= deadline,
            None => false,
        }
    }

    /// Duration since creation
    pub fn elapsed_secs(&self) -> i64 {
        Utc::now()
            .signed_duration_since(self.created_at)
            .num_seconds()
    }

    // ── Internal ─────────────────────────────────────────────────────

    /// Record a provenance entry
    fn record_provenance(&mut self, event_type: impl Into<String>, description: impl Into<String>) {
        self.provenance.push(ProvenanceEntry {
            sequence: self.provenance.len() as u64,
            event_type: event_type.into(),
            description: description.into(),
            timestamp: Utc::now(),
            actor: None,
        });
    }
}

// ── Workflow State ───────────────────────────────────────────────────

/// The lifecycle state of a workflow instance
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WorkflowState {
    /// Instance created but not yet started
    #[default]
    Created,
    /// Actively executing
    Active,
    /// Paused (waiting for external input or escalation resolution)
    Paused,
    /// Successfully completed
    Completed,
    /// Failed (unrecoverable error or escalation abort)
    Failed,
    /// Cancelled by an authorized actor
    Cancelled,
}

impl WorkflowState {
    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

// ── Node State ───────────────────────────────────────────────────────

/// Runtime state of a workflow node
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeState {
    /// Current status
    pub status: NodeStatus,
    /// When the node was activated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activated_at: Option<DateTime<Utc>>,
    /// When the node was completed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// The resonator assigned to execute this node
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_resonator: Option<ResonatorId>,
    /// Escalation state
    pub escalation: EscalationState,
    /// Number of times this node has been retried
    pub retry_count: u32,
}

impl NodeState {
    /// Check if the node is waiting for action
    pub fn is_active(&self) -> bool {
        self.status == NodeStatus::Active
    }

    /// Duration since activation (if active)
    pub fn active_duration_secs(&self) -> Option<i64> {
        self.activated_at
            .map(|at| Utc::now().signed_duration_since(at).num_seconds())
    }

    /// Check if the node has timed out
    pub fn is_timed_out(&self, timeout_secs: u64) -> bool {
        self.active_duration_secs()
            .map(|d| d >= timeout_secs as i64)
            .unwrap_or(false)
    }
}

/// Status of a workflow node
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NodeStatus {
    /// Not yet activated
    #[default]
    Pending,
    /// Currently active (waiting for commitment fulfillment)
    Active,
    /// Successfully completed
    Completed,
    /// Failed
    Failed,
    /// Skipped (via escalation or conditional logic)
    Skipped,
    /// Waiting (for parallel join — some but not all inputs ready)
    Waiting,
}

// ── Provenance ───────────────────────────────────────────────────────

/// An entry in the workflow provenance chain
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProvenanceEntry {
    /// Monotonically increasing sequence number
    pub sequence: u64,
    /// Type of event
    pub event_type: String,
    /// Human-readable description
    pub description: String,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// Who caused this event (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<ResonatorId>,
}

impl ProvenanceEntry {
    pub fn new(
        sequence: u64,
        event_type: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            sequence,
            event_type: event_type.into(),
            description: description.into(),
            timestamp: Utc::now(),
            actor: None,
        }
    }

    pub fn with_actor(mut self, actor: ResonatorId) -> Self {
        self.actor = Some(actor);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use collective_types::ReceiptType;

    fn make_instance() -> WorkflowInstance {
        WorkflowInstance::new(
            WorkflowDefinitionId::new("wf-def-1"),
            CollectiveId::new("collective-1"),
            ResonatorId::new("initiator-1"),
        )
    }

    #[test]
    fn test_create_instance() {
        let inst = make_instance();
        assert_eq!(inst.state, WorkflowState::Created);
        assert!(!inst.is_active());
        assert!(!inst.is_terminal());
        assert_eq!(inst.active_nodes().len(), 0);
    }

    #[test]
    fn test_workflow_lifecycle() {
        let mut inst = make_instance();

        // Start
        inst.start();
        assert!(inst.is_active());

        // Activate a node
        inst.activate_node(NodeId::new("review"));
        assert_eq!(inst.active_nodes().len(), 1);

        // Complete the node
        inst.complete_node(&NodeId::new("review"));
        assert_eq!(inst.active_nodes().len(), 0);
        assert_eq!(inst.completed_nodes().len(), 1);

        // Complete the workflow
        inst.complete();
        assert!(inst.is_terminal());
        assert!(inst.completed_at.is_some());
        assert!(inst.provenance_count() > 0);
    }

    #[test]
    fn test_workflow_failure() {
        let mut inst = make_instance();
        inst.start();
        inst.activate_node(NodeId::new("task"));
        inst.fail_node(&NodeId::new("task"), "Something went wrong");
        inst.fail("Node task failed");

        assert_eq!(inst.state, WorkflowState::Failed);
        assert!(inst.is_terminal());
    }

    #[test]
    fn test_workflow_pause_resume() {
        let mut inst = make_instance();
        inst.start();
        assert!(inst.is_active());

        inst.pause("Waiting for human input");
        assert_eq!(inst.state, WorkflowState::Paused);
        assert!(!inst.is_active());

        inst.resume();
        assert!(inst.is_active());
    }

    #[test]
    fn test_workflow_cancel() {
        let mut inst = make_instance();
        inst.start();
        inst.cancel("No longer needed");

        assert_eq!(inst.state, WorkflowState::Cancelled);
        assert!(inst.is_terminal());
    }

    #[test]
    fn test_role_assignments() {
        let mut inst = make_instance();
        inst.assign_role(RoleId::new("reviewer"), ResonatorId::new("res-1"));
        inst.assign_role(RoleId::new("reviewer"), ResonatorId::new("res-2"));
        inst.assign_role(RoleId::new("approver"), ResonatorId::new("res-3"));

        let reviewers = inst.resonators_for_role(&RoleId::new("reviewer"));
        assert_eq!(reviewers.len(), 2);

        let approvers = inst.resonators_for_role(&RoleId::new("approver"));
        assert_eq!(approvers.len(), 1);

        let unknown = inst.resonators_for_role(&RoleId::new("unknown"));
        assert_eq!(unknown.len(), 0);
    }

    #[test]
    fn test_receipts() {
        let mut inst = make_instance();
        inst.start();
        inst.activate_node(NodeId::new("review"));

        let receipt = WorkflowReceipt::new(
            NodeId::new("review"),
            ReceiptType::CommitmentFulfilled,
            ResonatorId::new("reviewer-1"),
        );
        inst.add_receipt(receipt);

        assert_eq!(inst.receipt_count(), 1);
        assert_eq!(inst.receipts_for_node(&NodeId::new("review")).len(), 1);
        assert_eq!(inst.receipts_for_node(&NodeId::new("other")).len(), 0);
    }

    #[test]
    fn test_node_state() {
        let mut inst = make_instance();
        inst.start();
        inst.activate_node(NodeId::new("task"));

        let state = inst.get_node_state(&NodeId::new("task")).unwrap();
        assert!(state.is_active());
        assert!(state.activated_at.is_some());
        assert!(!state.is_timed_out(3600));

        inst.assign_resonator_to_node(&NodeId::new("task"), ResonatorId::new("worker"));
        let state = inst.get_node_state(&NodeId::new("task")).unwrap();
        assert!(state.assigned_resonator.is_some());
    }

    #[test]
    fn test_skip_node() {
        let mut inst = make_instance();
        inst.start();
        inst.activate_node(NodeId::new("optional"));
        inst.skip_node(&NodeId::new("optional"));

        let state = inst.get_node_state(&NodeId::new("optional")).unwrap();
        assert_eq!(state.status, NodeStatus::Skipped);
    }

    #[test]
    fn test_provenance() {
        let mut inst = make_instance();
        inst.start();
        inst.activate_node(NodeId::new("a"));
        inst.complete_node(&NodeId::new("a"));
        inst.complete();

        // Should have: started, activated, completed_node, completed_workflow
        assert!(inst.provenance_count() >= 4);

        // Verify sequence numbers
        for (i, entry) in inst.provenance.iter().enumerate() {
            assert_eq!(entry.sequence, i as u64);
        }
    }

    #[test]
    fn test_workflow_state_terminal() {
        assert!(!WorkflowState::Created.is_terminal());
        assert!(!WorkflowState::Active.is_terminal());
        assert!(!WorkflowState::Paused.is_terminal());
        assert!(WorkflowState::Completed.is_terminal());
        assert!(WorkflowState::Failed.is_terminal());
        assert!(WorkflowState::Cancelled.is_terminal());
    }

    #[test]
    fn test_instance_parameters() {
        let inst = make_instance()
            .with_parameter("document_id", "doc-123")
            .with_parameter("priority", "high")
            .with_metadata("source", "api");

        assert_eq!(inst.parameters.get("document_id").unwrap(), "doc-123");
        assert_eq!(inst.metadata.get("source").unwrap(), "api");
    }

    #[test]
    fn test_instance_id() {
        let id = WorkflowInstanceId::generate();
        assert!(!id.0.is_empty());
        assert!(id.short().len() <= 8);

        let named = WorkflowInstanceId::new("inst-1");
        assert_eq!(format!("{}", named), "inst-1");
    }

    #[test]
    fn test_node_status_variants() {
        let statuses = vec![
            NodeStatus::Pending,
            NodeStatus::Active,
            NodeStatus::Completed,
            NodeStatus::Failed,
            NodeStatus::Skipped,
            NodeStatus::Waiting,
        ];
        assert_eq!(statuses.len(), 6);
        assert_eq!(NodeStatus::default(), NodeStatus::Pending);
    }
}
