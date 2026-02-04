//! Workflow edges: receipt-gated transitions
//!
//! Edges connect nodes in the workflow graph. Every edge has a
//! TransitionGate that must be satisfied before the transition
//! fires. This is what makes workflows *receipt-gated* — you
//! can't just move to the next step, you must produce evidence.

use crate::NodeId;
use chrono::{DateTime, Utc};
use collective_types::ReceiptType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An edge in the workflow graph — a receipt-gated transition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowEdge {
    /// Source node
    pub source: NodeId,
    /// Target node
    pub target: NodeId,
    /// The gate that must be satisfied to traverse this edge
    pub gate: TransitionGate,
    /// Human-readable label for this transition
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
    /// Priority for edge ordering when multiple edges leave a decision node
    pub priority: u32,
    /// Metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl WorkflowEdge {
    /// Create a new edge with an automatic gate (always satisfied)
    pub fn new(source: NodeId, target: NodeId) -> Self {
        Self {
            source,
            target,
            gate: TransitionGate::Automatic,
            label: String::new(),
            priority: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create an edge with a receipt gate
    pub fn receipt_gated(source: NodeId, target: NodeId, receipt_type: ReceiptType) -> Self {
        Self {
            source,
            target,
            gate: TransitionGate::ReceiptEmitted { receipt_type },
            label: String::new(),
            priority: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create an edge with a condition gate
    pub fn conditional(source: NodeId, target: NodeId, condition: impl Into<String>) -> Self {
        Self {
            source,
            target,
            gate: TransitionGate::Condition {
                expression: condition.into(),
            },
            label: String::new(),
            priority: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create an edge with a timeout gate
    pub fn timeout(source: NodeId, target: NodeId, timeout_secs: u64) -> Self {
        Self {
            source,
            target,
            gate: TransitionGate::Timeout { timeout_secs },
            label: String::new(),
            priority: 0,
            metadata: HashMap::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_gate(mut self, gate: TransitionGate) -> Self {
        self.gate = gate;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// The condition that must be satisfied to traverse a workflow edge.
///
/// This is the heart of the receipt-gated workflow model:
/// transitions don't happen by fiat—they happen because
/// evidence (receipts) has been produced.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransitionGate {
    /// Always satisfied — used for start node outgoing edges
    /// and control flow nodes
    Automatic,

    /// A specific receipt type must have been emitted by the source node
    ReceiptEmitted {
        /// The type of receipt required
        receipt_type: ReceiptType,
    },

    /// All specified receipt types must have been emitted
    AllReceiptsEmitted {
        /// All of these receipt types must be present
        receipt_types: Vec<ReceiptType>,
    },

    /// At least one of the specified receipt types must have been emitted
    AnyReceiptEmitted {
        /// At least one of these receipt types must be present
        receipt_types: Vec<ReceiptType>,
    },

    /// A boolean condition expression (evaluated at runtime)
    Condition {
        /// The condition expression (simple expression language)
        expression: String,
    },

    /// Transition fires after a timeout elapses
    Timeout {
        /// Timeout duration in seconds
        timeout_secs: u64,
    },

    /// Threshold commitment must be satisfied
    ThresholdMet {
        /// Description of what threshold is being checked
        description: String,
        /// Minimum signatures required
        min_signatures: u32,
    },

    /// Composite: all sub-gates must be satisfied
    AllOf {
        /// All of these gates must be satisfied
        gates: Vec<TransitionGate>,
    },

    /// Composite: any sub-gate must be satisfied
    AnyOf {
        /// At least one of these gates must be satisfied
        gates: Vec<TransitionGate>,
    },
}

impl TransitionGate {
    /// Create a receipt-emitted gate
    pub fn receipt(receipt_type: ReceiptType) -> Self {
        Self::ReceiptEmitted { receipt_type }
    }

    /// Create an all-receipts gate
    pub fn all_receipts(types: Vec<ReceiptType>) -> Self {
        Self::AllReceiptsEmitted {
            receipt_types: types,
        }
    }

    /// Create an any-receipt gate
    pub fn any_receipt(types: Vec<ReceiptType>) -> Self {
        Self::AnyReceiptEmitted {
            receipt_types: types,
        }
    }

    /// Create a condition gate
    pub fn condition(expr: impl Into<String>) -> Self {
        Self::Condition {
            expression: expr.into(),
        }
    }

    /// Create a timeout gate
    pub fn timeout(secs: u64) -> Self {
        Self::Timeout { timeout_secs: secs }
    }

    /// Check if this is an automatic (always-satisfied) gate
    pub fn is_automatic(&self) -> bool {
        matches!(self, Self::Automatic)
    }

    /// Check if this gate requires receipts
    pub fn requires_receipts(&self) -> bool {
        matches!(
            self,
            Self::ReceiptEmitted { .. }
                | Self::AllReceiptsEmitted { .. }
                | Self::AnyReceiptEmitted { .. }
        )
    }
}

/// A receipt emitted during workflow execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowReceipt {
    /// Unique receipt ID
    pub receipt_id: String,
    /// The node that produced this receipt
    pub node_id: NodeId,
    /// The type of receipt
    pub receipt_type: ReceiptType,
    /// Who produced the receipt
    pub emitter: resonator_types::ResonatorId,
    /// When the receipt was emitted
    pub emitted_at: DateTime<Utc>,
    /// Additional data
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub data: HashMap<String, String>,
}

impl WorkflowReceipt {
    pub fn new(
        node_id: NodeId,
        receipt_type: ReceiptType,
        emitter: resonator_types::ResonatorId,
    ) -> Self {
        Self {
            receipt_id: uuid::Uuid::new_v4().to_string(),
            node_id,
            receipt_type,
            emitter,
            emitted_at: Utc::now(),
            data: HashMap::new(),
        }
    }

    pub fn with_data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_automatic_edge() {
        let edge = WorkflowEdge::new(NodeId::new("a"), NodeId::new("b"));
        assert!(edge.gate.is_automatic());
        assert!(!edge.gate.requires_receipts());
    }

    #[test]
    fn test_receipt_gated_edge() {
        let edge = WorkflowEdge::receipt_gated(
            NodeId::new("review"),
            NodeId::new("approve"),
            ReceiptType::CommitmentFulfilled,
        );
        assert!(!edge.gate.is_automatic());
        assert!(edge.gate.requires_receipts());
    }

    #[test]
    fn test_conditional_edge() {
        let edge =
            WorkflowEdge::conditional(NodeId::new("check"), NodeId::new("pass"), "score >= 80")
                .with_label("Pass")
                .with_priority(1);

        assert_eq!(edge.label, "Pass");
        assert_eq!(edge.priority, 1);
        assert!(!edge.gate.requires_receipts());
    }

    #[test]
    fn test_timeout_edge() {
        let edge = WorkflowEdge::timeout(NodeId::new("wait"), NodeId::new("escalate"), 3600);
        match &edge.gate {
            TransitionGate::Timeout { timeout_secs } => {
                assert_eq!(*timeout_secs, 3600);
            }
            _ => panic!("Expected timeout gate"),
        }
    }

    #[test]
    fn test_composite_gates() {
        let gate = TransitionGate::AllOf {
            gates: vec![
                TransitionGate::receipt(ReceiptType::CommitmentFulfilled),
                TransitionGate::receipt(ReceiptType::Audit),
            ],
        };
        assert!(!gate.is_automatic());

        let gate = TransitionGate::AnyOf {
            gates: vec![
                TransitionGate::receipt(ReceiptType::CommitmentFulfilled),
                TransitionGate::timeout(3600),
            ],
        };
        assert!(!gate.is_automatic());
    }

    #[test]
    fn test_gate_constructors() {
        let r = TransitionGate::receipt(ReceiptType::Financial);
        assert!(r.requires_receipts());

        let all = TransitionGate::all_receipts(vec![
            ReceiptType::CommitmentFulfilled,
            ReceiptType::Audit,
        ]);
        assert!(all.requires_receipts());

        let any = TransitionGate::any_receipt(vec![
            ReceiptType::CommitmentFulfilled,
            ReceiptType::CommitmentBroken,
        ]);
        assert!(any.requires_receipts());

        let c = TransitionGate::condition("value > 100");
        assert!(!c.requires_receipts());
    }

    #[test]
    fn test_workflow_receipt() {
        let receipt = WorkflowReceipt::new(
            NodeId::new("review"),
            ReceiptType::CommitmentFulfilled,
            resonator_types::ResonatorId::new("reviewer-1"),
        )
        .with_data("document_id", "doc-123");

        assert_eq!(receipt.node_id, NodeId::new("review"));
        assert_eq!(receipt.data.get("document_id").unwrap(), "doc-123");
    }

    #[test]
    fn test_edge_metadata() {
        let edge = WorkflowEdge::new(NodeId::new("a"), NodeId::new("b"))
            .with_metadata("condition_source", "manual")
            .with_gate(TransitionGate::condition("approved == true"));

        assert_eq!(edge.metadata.get("condition_source").unwrap(), "manual");
        assert!(!edge.gate.is_automatic());
    }

    #[test]
    fn test_threshold_met_gate() {
        let gate = TransitionGate::ThresholdMet {
            description: "Board approval".into(),
            min_signatures: 3,
        };
        assert!(!gate.is_automatic());
        assert!(!gate.requires_receipts());
    }
}
