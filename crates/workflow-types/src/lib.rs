//! Workflow Domain Types for MAPLE v2
//!
//! Workflows in MAPLE are NOT BPMN process diagrams. They are
//! **commitment programs** — directed graphs of commitment declarations
//! with receipt-gated transitions.
//!
//! # Key Concepts
//!
//! - **WorkflowDefinition**: A blueprint (graph) of nodes and edges.
//!   Nodes are commitment templates. Edges are receipt-gated transitions.
//! - **WorkflowInstance**: A running execution of a definition, tracking
//!   which nodes are active and what receipts have been collected.
//! - **TransitionGate**: The condition that must be satisfied to traverse
//!   an edge (receipt emitted, condition met, timeout elapsed).
//! - **CommitmentTemplate**: A parameterized commitment declaration that
//!   gets instantiated when a workflow node activates.
//! - **EscalationPath**: What happens when a node times out or fails —
//!   escalation, retry, or abort.
//! - **CompletedWorkflow**: The final record of a workflow execution,
//!   including all receipts produced.
//!
//! # Design Principles
//!
//! 1. Workflows coordinate, never execute. The orchestrator declares
//!    commitments and waits for receipts.
//! 2. Every transition is receipt-gated. No implicit state changes.
//! 3. Workflows are auditable from start to finish via provenance.
//! 4. Escalation is explicit, never silent failure.

#![deny(unsafe_code)]

mod completion;
mod definition;
mod edge;
mod errors;
mod escalation;
mod instance;
mod template;

pub use completion::*;
pub use definition::*;
pub use edge::*;
pub use errors::*;
pub use escalation::*;
pub use instance::*;
pub use template::*;
