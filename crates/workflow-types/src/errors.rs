//! Error types for the Workflow layer

use crate::{NodeId, WorkflowDefinitionId, WorkflowInstanceId};

/// Errors that can occur in Workflow operations
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("Workflow definition not found: {0}")]
    DefinitionNotFound(WorkflowDefinitionId),

    #[error("Workflow instance not found: {0}")]
    InstanceNotFound(WorkflowInstanceId),

    #[error("Node not found: {0}")]
    NodeNotFound(NodeId),

    #[error("Edge not found: {from} -> {to}")]
    EdgeNotFound { from: NodeId, to: NodeId },

    #[error("Invalid transition: {0}")]
    InvalidTransition(String),

    #[error("Transition gate not satisfied: {0}")]
    GateNotSatisfied(String),

    #[error("Workflow already completed")]
    AlreadyCompleted,

    #[error("Workflow already failed")]
    AlreadyFailed,

    #[error("Workflow not active")]
    NotActive,

    #[error("Node not active: {0}")]
    NodeNotActive(NodeId),

    #[error("Node already completed: {0}")]
    NodeAlreadyCompleted(NodeId),

    #[error("Escalation triggered: {0}")]
    EscalationTriggered(String),

    #[error("Timeout exceeded for node: {0}")]
    NodeTimeout(NodeId),

    #[error("Missing required receipt: {0}")]
    MissingReceipt(String),

    #[error("Cycle detected in workflow graph")]
    CycleDetected,

    #[error("No start node defined")]
    NoStartNode,

    #[error("No end node defined")]
    NoEndNode,

    #[error("Disconnected graph: unreachable nodes")]
    DisconnectedGraph,

    #[error("Duplicate node ID: {0}")]
    DuplicateNodeId(NodeId),

    #[error("Duplicate edge: {from} -> {to}")]
    DuplicateEdge { from: NodeId, to: NodeId },

    #[error("Role not assigned in workflow: {0}")]
    RoleNotAssigned(String),

    #[error("Collective error: {0}")]
    CollectiveError(#[from] collective_types::CollectiveError),

    #[error("Workflow validation error: {0}")]
    ValidationError(String),
}

/// Result type alias for workflow operations
pub type WorkflowResult<T> = Result<T, WorkflowError>;
