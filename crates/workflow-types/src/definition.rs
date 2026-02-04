//! Workflow definitions: the blueprint for commitment programs
//!
//! A WorkflowDefinition is a directed graph where:
//! - Nodes are commitment templates (what to declare)
//! - Edges are receipt-gated transitions (when to proceed)
//!
//! Definitions are immutable once validated. To modify, create a new version.

use crate::{CommitmentTemplate, EscalationPath, WorkflowEdge, WorkflowError, WorkflowResult};
use chrono::{DateTime, Utc};
use collective_types::{CollectiveId, RoleId};
use resonator_types::ResonatorId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ── Identifiers ──────────────────────────────────────────────────────

/// Unique identifier for a workflow definition
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowDefinitionId(pub String);

impl WorkflowDefinitionId {
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

impl std::fmt::Display for WorkflowDefinitionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a workflow node
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Workflow Definition ──────────────────────────────────────────────

/// A workflow definition — the blueprint for a commitment program
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// Unique identifier
    pub id: WorkflowDefinitionId,
    /// Human-readable name
    pub name: String,
    /// Description of what this workflow accomplishes
    pub description: String,
    /// Version for tracking definition evolution
    pub version: u32,
    /// The collective this workflow belongs to
    pub collective_id: CollectiveId,
    /// Who authored this definition
    pub author: ResonatorId,
    /// Roles required by this workflow
    pub required_roles: Vec<WorkflowRole>,
    /// The nodes (commitment templates) in the graph
    pub nodes: Vec<WorkflowNode>,
    /// The edges (receipt-gated transitions) in the graph
    pub edges: Vec<WorkflowEdge>,
    /// Global escalation paths (can be overridden per-node)
    pub default_escalation: Option<EscalationPath>,
    /// Maximum duration for the entire workflow (seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_duration_secs: Option<u64>,
    /// When this definition was created
    pub created_at: DateTime<Utc>,
    /// Metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl WorkflowDefinition {
    /// Create a new workflow definition
    pub fn new(name: impl Into<String>, collective_id: CollectiveId, author: ResonatorId) -> Self {
        Self {
            id: WorkflowDefinitionId::generate(),
            name: name.into(),
            description: String::new(),
            version: 1,
            collective_id,
            author,
            required_roles: Vec::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            default_escalation: None,
            max_duration_secs: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_max_duration(mut self, secs: u64) -> Self {
        self.max_duration_secs = Some(secs);
        self
    }

    pub fn with_default_escalation(mut self, escalation: EscalationPath) -> Self {
        self.default_escalation = Some(escalation);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Add a role required by this workflow
    pub fn add_role(&mut self, role: WorkflowRole) {
        self.required_roles.push(role);
    }

    /// Add a node to the workflow graph
    pub fn add_node(&mut self, node: WorkflowNode) -> WorkflowResult<()> {
        if self.nodes.iter().any(|n| n.id == node.id) {
            return Err(WorkflowError::DuplicateNodeId(node.id));
        }
        self.nodes.push(node);
        Ok(())
    }

    /// Add an edge to the workflow graph
    pub fn add_edge(&mut self, edge: WorkflowEdge) -> WorkflowResult<()> {
        // Verify source and target nodes exist
        if !self.nodes.iter().any(|n| n.id == edge.source) {
            return Err(WorkflowError::NodeNotFound(edge.source));
        }
        if !self.nodes.iter().any(|n| n.id == edge.target) {
            return Err(WorkflowError::NodeNotFound(edge.target));
        }
        // Check for duplicate edges
        if self
            .edges
            .iter()
            .any(|e| e.source == edge.source && e.target == edge.target)
        {
            return Err(WorkflowError::DuplicateEdge {
                from: edge.source,
                to: edge.target,
            });
        }
        self.edges.push(edge);
        Ok(())
    }

    /// Get the start node (marked as NodeType::Start)
    pub fn start_node(&self) -> Option<&WorkflowNode> {
        self.nodes.iter().find(|n| n.node_type == NodeType::Start)
    }

    /// Get the end nodes (marked as NodeType::End)
    pub fn end_nodes(&self) -> Vec<&WorkflowNode> {
        self.nodes
            .iter()
            .filter(|n| n.node_type == NodeType::End)
            .collect()
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &NodeId) -> Option<&WorkflowNode> {
        self.nodes.iter().find(|n| &n.id == id)
    }

    /// Get outgoing edges from a node
    pub fn outgoing_edges(&self, node_id: &NodeId) -> Vec<&WorkflowEdge> {
        self.edges.iter().filter(|e| &e.source == node_id).collect()
    }

    /// Get incoming edges to a node
    pub fn incoming_edges(&self, node_id: &NodeId) -> Vec<&WorkflowEdge> {
        self.edges.iter().filter(|e| &e.target == node_id).collect()
    }

    /// Validate the workflow definition for structural correctness
    pub fn validate(&self) -> WorkflowResult<()> {
        // Must have at least one node
        if self.nodes.is_empty() {
            return Err(WorkflowError::ValidationError(
                "Workflow must have at least one node".into(),
            ));
        }

        // Must have exactly one start node
        let start_count = self
            .nodes
            .iter()
            .filter(|n| n.node_type == NodeType::Start)
            .count();
        if start_count == 0 {
            return Err(WorkflowError::NoStartNode);
        }
        if start_count > 1 {
            return Err(WorkflowError::ValidationError(
                "Workflow must have exactly one start node".into(),
            ));
        }

        // Must have at least one end node
        if self
            .nodes
            .iter()
            .filter(|n| n.node_type == NodeType::End)
            .count()
            == 0
        {
            return Err(WorkflowError::NoEndNode);
        }

        // Check node IDs are unique
        let mut seen_ids = HashSet::new();
        for node in &self.nodes {
            if !seen_ids.insert(&node.id) {
                return Err(WorkflowError::DuplicateNodeId(node.id.clone()));
            }
        }

        // Check all edge source/target nodes exist
        for edge in &self.edges {
            if !self.nodes.iter().any(|n| n.id == edge.source) {
                return Err(WorkflowError::NodeNotFound(edge.source.clone()));
            }
            if !self.nodes.iter().any(|n| n.id == edge.target) {
                return Err(WorkflowError::NodeNotFound(edge.target.clone()));
            }
        }

        // Check reachability from start node
        if let Some(start) = self.start_node() {
            let reachable = self.reachable_from(&start.id);
            for node in &self.nodes {
                if !reachable.contains(&node.id) {
                    return Err(WorkflowError::DisconnectedGraph);
                }
            }
        }

        Ok(())
    }

    /// Find all nodes reachable from a given node via BFS
    fn reachable_from(&self, start: &NodeId) -> HashSet<NodeId> {
        let mut visited = HashSet::new();
        let mut queue = vec![start.clone()];

        while let Some(current) = queue.pop() {
            if visited.insert(current.clone()) {
                for edge in self.outgoing_edges(&current) {
                    if !visited.contains(&edge.target) {
                        queue.push(edge.target.clone());
                    }
                }
            }
        }

        visited
    }

    /// Total number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Total number of edges
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

// ── Workflow Node ────────────────────────────────────────────────────

/// A node in the workflow graph — a commitment template to be instantiated
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowNode {
    /// Unique identifier within this workflow
    pub id: NodeId,
    /// Human-readable name
    pub name: String,
    /// Description of what this node represents
    pub description: String,
    /// Node type (start, end, action, decision, parallel join/fork)
    pub node_type: NodeType,
    /// The commitment template to instantiate at this node
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commitment_template: Option<CommitmentTemplate>,
    /// The role that must fulfill this node
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_role: Option<RoleId>,
    /// Maximum time (seconds) before escalation triggers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    /// Node-specific escalation override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escalation: Option<EscalationPath>,
    /// Metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl WorkflowNode {
    /// Create a new workflow node
    pub fn new(id: impl Into<String>, name: impl Into<String>, node_type: NodeType) -> Self {
        Self {
            id: NodeId::new(id),
            name: name.into(),
            description: String::new(),
            node_type,
            commitment_template: None,
            assigned_role: None,
            timeout_secs: None,
            escalation: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a start node
    pub fn start(id: impl Into<String>) -> Self {
        Self::new(id, "Start", NodeType::Start)
    }

    /// Create an end node
    pub fn end(id: impl Into<String>) -> Self {
        Self::new(id, "End", NodeType::End)
    }

    /// Create an action node
    pub fn action(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self::new(id, name, NodeType::Action)
    }

    /// Create a decision node
    pub fn decision(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self::new(id, name, NodeType::Decision)
    }

    /// Create a parallel fork node
    pub fn fork(id: impl Into<String>) -> Self {
        Self::new(id, "Fork", NodeType::ParallelFork)
    }

    /// Create a parallel join node
    pub fn join(id: impl Into<String>) -> Self {
        Self::new(id, "Join", NodeType::ParallelJoin)
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_commitment_template(mut self, template: CommitmentTemplate) -> Self {
        self.commitment_template = Some(template);
        self
    }

    pub fn with_assigned_role(mut self, role: RoleId) -> Self {
        self.assigned_role = Some(role);
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    pub fn with_escalation(mut self, escalation: EscalationPath) -> Self {
        self.escalation = Some(escalation);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Check if this node requires a commitment to be declared
    pub fn requires_commitment(&self) -> bool {
        self.commitment_template.is_some()
    }

    /// Check if this node is a control flow node (no commitment)
    pub fn is_control_flow(&self) -> bool {
        matches!(
            self.node_type,
            NodeType::Start
                | NodeType::End
                | NodeType::ParallelFork
                | NodeType::ParallelJoin
                | NodeType::Decision
        )
    }
}

// ── Node Type ────────────────────────────────────────────────────────

/// The type of a workflow node
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    /// The entry point of the workflow
    Start,
    /// A terminal node — workflow completes when any end node is reached
    End,
    /// An action node that requires a commitment to be declared and fulfilled
    Action,
    /// A decision point — outgoing edges have conditions
    Decision,
    /// Fork: activates all outgoing edges simultaneously
    ParallelFork,
    /// Join: waits for all incoming edges to complete
    ParallelJoin,
    /// A sub-workflow invocation
    SubWorkflow {
        /// The sub-workflow definition to invoke
        definition_id: WorkflowDefinitionId,
    },
}

// ── Workflow Role ────────────────────────────────────────────────────

/// A role declared in a workflow definition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowRole {
    /// The role ID (references collective's role registry)
    pub role_id: RoleId,
    /// Description of what this role does in this workflow
    pub description: String,
    /// Whether this role is required or optional
    pub required: bool,
    /// Minimum number of resonators that must hold this role
    pub min_assignees: u32,
}

impl WorkflowRole {
    pub fn new(role_id: RoleId, description: impl Into<String>) -> Self {
        Self {
            role_id,
            description: description.into(),
            required: true,
            min_assignees: 1,
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn with_min_assignees(mut self, min: u32) -> Self {
        self.min_assignees = min;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_simple_workflow() -> WorkflowDefinition {
        let collective_id = CollectiveId::new("test-collective");
        let author = ResonatorId::new("author-1");

        let mut wf = WorkflowDefinition::new("Test Workflow", collective_id, author)
            .with_description("A simple test workflow");

        let start = WorkflowNode::start("start");
        let action = WorkflowNode::action("review", "Review Document")
            .with_description("Review the submitted document")
            .with_assigned_role(RoleId::new("reviewer"))
            .with_timeout(3600);
        let end = WorkflowNode::end("end");

        wf.add_node(start).unwrap();
        wf.add_node(action).unwrap();
        wf.add_node(end).unwrap();

        wf.add_edge(WorkflowEdge::new(
            NodeId::new("start"),
            NodeId::new("review"),
        ))
        .unwrap();
        wf.add_edge(WorkflowEdge::new(NodeId::new("review"), NodeId::new("end")))
            .unwrap();

        wf
    }

    #[test]
    fn test_create_workflow_definition() {
        let wf = make_simple_workflow();

        assert_eq!(wf.name, "Test Workflow");
        assert_eq!(wf.node_count(), 3);
        assert_eq!(wf.edge_count(), 2);
        assert!(wf.start_node().is_some());
        assert_eq!(wf.end_nodes().len(), 1);
    }

    #[test]
    fn test_validate_valid_workflow() {
        let wf = make_simple_workflow();
        assert!(wf.validate().is_ok());
    }

    #[test]
    fn test_validate_no_start_node() {
        let mut wf = WorkflowDefinition::new("Bad", CollectiveId::new("c"), ResonatorId::new("a"));
        wf.add_node(WorkflowNode::action("action", "Do thing"))
            .unwrap();
        wf.add_node(WorkflowNode::end("end")).unwrap();

        let result = wf.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_no_end_node() {
        let mut wf = WorkflowDefinition::new("Bad", CollectiveId::new("c"), ResonatorId::new("a"));
        wf.add_node(WorkflowNode::start("start")).unwrap();
        wf.add_node(WorkflowNode::action("action", "Do thing"))
            .unwrap();

        let result = wf.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_disconnected_graph() {
        let mut wf = WorkflowDefinition::new(
            "Disconnected",
            CollectiveId::new("c"),
            ResonatorId::new("a"),
        );
        wf.add_node(WorkflowNode::start("start")).unwrap();
        wf.add_node(WorkflowNode::end("end")).unwrap();
        // island node not connected
        wf.add_node(WorkflowNode::action("island", "Island"))
            .unwrap();

        wf.add_edge(WorkflowEdge::new(NodeId::new("start"), NodeId::new("end")))
            .unwrap();

        let result = wf.validate();
        assert!(matches!(result, Err(WorkflowError::DisconnectedGraph)));
    }

    #[test]
    fn test_duplicate_node_id() {
        let mut wf = WorkflowDefinition::new("Dup", CollectiveId::new("c"), ResonatorId::new("a"));
        wf.add_node(WorkflowNode::start("start")).unwrap();
        let result = wf.add_node(WorkflowNode::action("start", "Duplicate"));
        assert!(matches!(result, Err(WorkflowError::DuplicateNodeId(_))));
    }

    #[test]
    fn test_edge_to_nonexistent_node() {
        let mut wf =
            WorkflowDefinition::new("Bad Edge", CollectiveId::new("c"), ResonatorId::new("a"));
        wf.add_node(WorkflowNode::start("start")).unwrap();

        let result = wf.add_edge(WorkflowEdge::new(
            NodeId::new("start"),
            NodeId::new("nonexistent"),
        ));
        assert!(matches!(result, Err(WorkflowError::NodeNotFound(_))));
    }

    #[test]
    fn test_outgoing_incoming_edges() {
        let wf = make_simple_workflow();

        let start_out = wf.outgoing_edges(&NodeId::new("start"));
        assert_eq!(start_out.len(), 1);
        assert_eq!(start_out[0].target, NodeId::new("review"));

        let end_in = wf.incoming_edges(&NodeId::new("end"));
        assert_eq!(end_in.len(), 1);
        assert_eq!(end_in[0].source, NodeId::new("review"));
    }

    #[test]
    fn test_node_constructors() {
        let start = WorkflowNode::start("s");
        assert_eq!(start.node_type, NodeType::Start);
        assert!(start.is_control_flow());
        assert!(!start.requires_commitment());

        let action = WorkflowNode::action("a", "Action");
        assert_eq!(action.node_type, NodeType::Action);
        assert!(!action.is_control_flow());

        let fork = WorkflowNode::fork("f");
        assert_eq!(fork.node_type, NodeType::ParallelFork);
        assert!(fork.is_control_flow());

        let join = WorkflowNode::join("j");
        assert_eq!(join.node_type, NodeType::ParallelJoin);

        let decision = WorkflowNode::decision("d", "Choice");
        assert_eq!(decision.node_type, NodeType::Decision);
    }

    #[test]
    fn test_workflow_role() {
        let role =
            WorkflowRole::new(RoleId::new("reviewer"), "Reviews documents").with_min_assignees(2);
        assert!(role.required);
        assert_eq!(role.min_assignees, 2);

        let optional = WorkflowRole::new(RoleId::new("observer"), "Observes").optional();
        assert!(!optional.required);
    }

    #[test]
    fn test_workflow_metadata() {
        let wf =
            WorkflowDefinition::new("Meta Test", CollectiveId::new("c"), ResonatorId::new("a"))
                .with_metadata("category", "finance")
                .with_max_duration(86400);

        assert_eq!(wf.metadata.get("category").unwrap(), "finance");
        assert_eq!(wf.max_duration_secs, Some(86400));
    }

    #[test]
    fn test_definition_id() {
        let id = WorkflowDefinitionId::generate();
        assert!(!id.0.is_empty());
        assert!(id.short().len() <= 8);

        let named = WorkflowDefinitionId::new("my-workflow");
        assert_eq!(format!("{}", named), "my-workflow");
    }
}
