//! Completed workflows: the final record of a workflow execution
//!
//! When a workflow reaches a terminal state (completed, failed, cancelled),
//! a CompletedWorkflow record is produced. This is the permanent audit
//! artifact that captures everything that happened.

use crate::{
    NodeId, ProvenanceEntry, WorkflowDefinitionId, WorkflowInstanceId, WorkflowReceipt,
    WorkflowState,
};
use chrono::{DateTime, Utc};
use collective_types::{CollectiveId, RoleId};
use resonator_types::ResonatorId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The completed record of a workflow execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompletedWorkflow {
    /// The instance ID
    pub instance_id: WorkflowInstanceId,
    /// The definition this was created from
    pub definition_id: WorkflowDefinitionId,
    /// The collective that ran this workflow
    pub collective_id: CollectiveId,
    /// Who initiated the workflow
    pub initiator: ResonatorId,
    /// Final state (Completed, Failed, or Cancelled)
    pub final_state: WorkflowState,
    /// Summary of what happened
    pub summary: String,
    /// All receipts produced during execution
    pub receipts: Vec<WorkflowReceipt>,
    /// Final node statuses
    pub node_outcomes: Vec<NodeOutcome>,
    /// Complete provenance chain
    pub provenance: Vec<ProvenanceEntry>,
    /// Role assignments used during execution
    pub role_assignments: HashMap<RoleId, Vec<ResonatorId>>,
    /// When the workflow started
    pub started_at: DateTime<Utc>,
    /// When the workflow ended
    pub ended_at: DateTime<Utc>,
    /// Total duration in seconds
    pub duration_secs: i64,
    /// Runtime parameters
    pub parameters: HashMap<String, String>,
    /// Metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl CompletedWorkflow {
    /// Create a completed workflow record from an instance
    pub fn from_instance(
        instance_id: WorkflowInstanceId,
        definition_id: WorkflowDefinitionId,
        collective_id: CollectiveId,
        initiator: ResonatorId,
        final_state: WorkflowState,
    ) -> Self {
        let now = Utc::now();
        Self {
            instance_id,
            definition_id,
            collective_id,
            initiator,
            final_state,
            summary: String::new(),
            receipts: Vec::new(),
            node_outcomes: Vec::new(),
            provenance: Vec::new(),
            role_assignments: HashMap::new(),
            started_at: now,
            ended_at: now,
            duration_secs: 0,
            parameters: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }

    pub fn with_receipts(mut self, receipts: Vec<WorkflowReceipt>) -> Self {
        self.receipts = receipts;
        self
    }

    pub fn with_node_outcomes(mut self, outcomes: Vec<NodeOutcome>) -> Self {
        self.node_outcomes = outcomes;
        self
    }

    pub fn with_provenance(mut self, provenance: Vec<ProvenanceEntry>) -> Self {
        self.provenance = provenance;
        self
    }

    pub fn with_role_assignments(mut self, assignments: HashMap<RoleId, Vec<ResonatorId>>) -> Self {
        self.role_assignments = assignments;
        self
    }

    pub fn with_timestamps(mut self, started_at: DateTime<Utc>, ended_at: DateTime<Utc>) -> Self {
        self.duration_secs = ended_at.signed_duration_since(started_at).num_seconds();
        self.started_at = started_at;
        self.ended_at = ended_at;
        self
    }

    pub fn with_parameters(mut self, parameters: HashMap<String, String>) -> Self {
        self.parameters = parameters;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    // ── Query methods ────────────────────────────────────────────────

    /// Whether the workflow completed successfully
    pub fn is_success(&self) -> bool {
        self.final_state == WorkflowState::Completed
    }

    /// Whether the workflow failed
    pub fn is_failure(&self) -> bool {
        self.final_state == WorkflowState::Failed
    }

    /// Total receipts produced
    pub fn receipt_count(&self) -> usize {
        self.receipts.len()
    }

    /// Total nodes that were executed
    pub fn nodes_executed(&self) -> usize {
        self.node_outcomes.iter().filter(|o| o.executed).count()
    }

    /// Nodes that completed successfully
    pub fn nodes_succeeded(&self) -> usize {
        self.node_outcomes.iter().filter(|o| o.success).count()
    }

    /// Nodes that failed
    pub fn nodes_failed(&self) -> usize {
        self.node_outcomes
            .iter()
            .filter(|o| o.executed && !o.success)
            .count()
    }

    /// Get all unique actors who participated
    pub fn participants(&self) -> Vec<&ResonatorId> {
        let mut participants: Vec<&ResonatorId> = self
            .role_assignments
            .values()
            .flat_map(|v| v.iter())
            .collect();
        participants.sort_by(|a, b| a.0.cmp(&b.0));
        participants.dedup_by(|a, b| a.0 == b.0);
        participants
    }
}

/// The outcome of a single workflow node
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeOutcome {
    /// The node ID
    pub node_id: NodeId,
    /// The node name
    pub node_name: String,
    /// Whether this node was executed (vs skipped)
    pub executed: bool,
    /// Whether the node completed successfully
    pub success: bool,
    /// Receipts produced by this node
    pub receipt_ids: Vec<String>,
    /// Who executed this node
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executor: Option<ResonatorId>,
    /// Duration of node execution in seconds
    pub duration_secs: i64,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Number of retries
    pub retry_count: u32,
}

impl NodeOutcome {
    /// Create a successful node outcome
    pub fn success(node_id: NodeId, node_name: impl Into<String>, duration_secs: i64) -> Self {
        Self {
            node_id,
            node_name: node_name.into(),
            executed: true,
            success: true,
            receipt_ids: Vec::new(),
            executor: None,
            duration_secs,
            error: None,
            retry_count: 0,
        }
    }

    /// Create a failed node outcome
    pub fn failure(
        node_id: NodeId,
        node_name: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            node_id,
            node_name: node_name.into(),
            executed: true,
            success: false,
            receipt_ids: Vec::new(),
            executor: None,
            duration_secs: 0,
            error: Some(error.into()),
            retry_count: 0,
        }
    }

    /// Create a skipped node outcome
    pub fn skipped(node_id: NodeId, node_name: impl Into<String>) -> Self {
        Self {
            node_id,
            node_name: node_name.into(),
            executed: false,
            success: false,
            receipt_ids: Vec::new(),
            executor: None,
            duration_secs: 0,
            error: None,
            retry_count: 0,
        }
    }

    pub fn with_executor(mut self, executor: ResonatorId) -> Self {
        self.executor = Some(executor);
        self
    }

    pub fn with_receipts(mut self, receipt_ids: Vec<String>) -> Self {
        self.receipt_ids = receipt_ids;
        self
    }

    pub fn with_retries(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completed_workflow() {
        let completed = CompletedWorkflow::from_instance(
            WorkflowInstanceId::new("inst-1"),
            WorkflowDefinitionId::new("def-1"),
            CollectiveId::new("collective-1"),
            ResonatorId::new("initiator-1"),
            WorkflowState::Completed,
        )
        .with_summary("Successfully processed document review")
        .with_node_outcomes(vec![
            NodeOutcome::success(NodeId::new("review"), "Review", 120)
                .with_executor(ResonatorId::new("reviewer-1")),
            NodeOutcome::success(NodeId::new("approve"), "Approve", 30)
                .with_executor(ResonatorId::new("approver-1")),
        ]);

        assert!(completed.is_success());
        assert!(!completed.is_failure());
        assert_eq!(completed.nodes_executed(), 2);
        assert_eq!(completed.nodes_succeeded(), 2);
        assert_eq!(completed.nodes_failed(), 0);
    }

    #[test]
    fn test_failed_workflow() {
        let completed = CompletedWorkflow::from_instance(
            WorkflowInstanceId::new("inst-2"),
            WorkflowDefinitionId::new("def-1"),
            CollectiveId::new("collective-1"),
            ResonatorId::new("initiator-1"),
            WorkflowState::Failed,
        )
        .with_summary("Review failed due to timeout")
        .with_node_outcomes(vec![
            NodeOutcome::success(NodeId::new("submit"), "Submit", 5),
            NodeOutcome::failure(
                NodeId::new("review"),
                "Review",
                "Timed out after 3600 seconds",
            )
            .with_retries(2),
        ]);

        assert!(!completed.is_success());
        assert!(completed.is_failure());
        assert_eq!(completed.nodes_executed(), 2);
        assert_eq!(completed.nodes_succeeded(), 1);
        assert_eq!(completed.nodes_failed(), 1);
    }

    #[test]
    fn test_node_outcomes() {
        let success = NodeOutcome::success(NodeId::new("a"), "Action A", 60);
        assert!(success.executed);
        assert!(success.success);

        let failure = NodeOutcome::failure(NodeId::new("b"), "Action B", "Error");
        assert!(failure.executed);
        assert!(!failure.success);
        assert!(failure.error.is_some());

        let skipped = NodeOutcome::skipped(NodeId::new("c"), "Action C");
        assert!(!skipped.executed);
        assert!(!skipped.success);
    }

    #[test]
    fn test_participants() {
        let mut assignments = HashMap::new();
        assignments.insert(
            RoleId::new("reviewer"),
            vec![ResonatorId::new("res-1"), ResonatorId::new("res-2")],
        );
        assignments.insert(
            RoleId::new("approver"),
            vec![ResonatorId::new("res-1"), ResonatorId::new("res-3")],
        );

        let completed = CompletedWorkflow::from_instance(
            WorkflowInstanceId::new("inst-3"),
            WorkflowDefinitionId::new("def-1"),
            CollectiveId::new("c"),
            ResonatorId::new("init"),
            WorkflowState::Completed,
        )
        .with_role_assignments(assignments);

        let participants = completed.participants();
        // res-1 appears in both roles but should be deduped
        assert_eq!(participants.len(), 3);
    }

    #[test]
    fn test_timestamps() {
        let start = Utc::now() - chrono::Duration::hours(1);
        let end = Utc::now();

        let completed = CompletedWorkflow::from_instance(
            WorkflowInstanceId::new("inst-4"),
            WorkflowDefinitionId::new("def-1"),
            CollectiveId::new("c"),
            ResonatorId::new("init"),
            WorkflowState::Completed,
        )
        .with_timestamps(start, end);

        assert!(completed.duration_secs >= 3599); // ~1 hour
    }

    #[test]
    fn test_receipt_count() {
        use collective_types::ReceiptType;

        let completed = CompletedWorkflow::from_instance(
            WorkflowInstanceId::new("inst-5"),
            WorkflowDefinitionId::new("def-1"),
            CollectiveId::new("c"),
            ResonatorId::new("init"),
            WorkflowState::Completed,
        )
        .with_receipts(vec![
            WorkflowReceipt::new(
                NodeId::new("a"),
                ReceiptType::CommitmentFulfilled,
                ResonatorId::new("r1"),
            ),
            WorkflowReceipt::new(NodeId::new("b"), ReceiptType::Audit, ResonatorId::new("r2")),
        ]);

        assert_eq!(completed.receipt_count(), 2);
    }

    #[test]
    fn test_with_parameters() {
        let mut params = HashMap::new();
        params.insert("doc_id".to_string(), "doc-123".to_string());

        let completed = CompletedWorkflow::from_instance(
            WorkflowInstanceId::new("inst-6"),
            WorkflowDefinitionId::new("def-1"),
            CollectiveId::new("c"),
            ResonatorId::new("init"),
            WorkflowState::Completed,
        )
        .with_parameters(params)
        .with_metadata("source", "test");

        assert_eq!(completed.parameters.get("doc_id").unwrap(), "doc-123");
        assert_eq!(completed.metadata.get("source").unwrap(), "test");
    }
}
