//! Workflow Orchestrator Runtime for MAPLE v2
//!
//! The workflow engine coordinates commitment programs. It advances
//! workflows through receipt-gated transitions, manages node activation,
//! handles escalation, and maintains complete provenance.
//!
//! # Key Principle
//!
//! **The orchestrator coordinates, it NEVER executes actions directly.**
//!
//! It declares commitments, waits for receipts, and advances state.
//! The actual work is done by resonators assigned to workflow roles.
//!
//! # Architecture
//!
//! The [`WorkflowOrchestrator`] composes specialized components:
//!
//! - [`DefinitionRegistry`] — Stores and retrieves workflow definitions
//! - [`StateMachine`] — Manages node activation and transition logic
//! - [`GateEvaluator`] — Evaluates transition gates against collected receipts
//! - [`EscalationHandler`] — Monitors timeouts and triggers escalation paths
//! - [`ProvenanceTracker`] — Records all state changes for auditability
//!
//! # Example
//!
//! ```rust
//! use workflow_engine::WorkflowOrchestrator;
//! use workflow_types::*;
//! use collective_types::{CollectiveId, RoleId};
//! use resonator_types::ResonatorId;
//!
//! let mut orchestrator = WorkflowOrchestrator::new(CollectiveId::new("acme"));
//!
//! // Register a workflow definition
//! let mut def = WorkflowDefinition::new(
//!     "Document Review",
//!     CollectiveId::new("acme"),
//!     ResonatorId::new("author"),
//! );
//! def.add_node(WorkflowNode::start("start")).unwrap();
//! def.add_node(WorkflowNode::action("review", "Review")).unwrap();
//! def.add_node(WorkflowNode::end("end")).unwrap();
//! def.add_edge(WorkflowEdge::new(NodeId::new("start"), NodeId::new("review"))).unwrap();
//! def.add_edge(WorkflowEdge::new(NodeId::new("review"), NodeId::new("end"))).unwrap();
//!
//! let def_id = orchestrator.register_definition(def).unwrap();
//!
//! // Launch a workflow instance
//! let inst_id = orchestrator.launch_instance(
//!     &def_id,
//!     ResonatorId::new("initiator"),
//! ).unwrap();
//!
//! assert!(orchestrator.get_instance(&inst_id).unwrap().is_active());
//! ```

#![deny(unsafe_code)]

pub mod definition_registry;
pub mod escalation_handler;
pub mod gate_evaluator;
pub mod orchestrator;
pub mod provenance_tracker;
pub mod state_machine;

// Re-export main types
pub use definition_registry::DefinitionRegistry;
pub use escalation_handler::EscalationHandler;
pub use gate_evaluator::GateEvaluator;
pub use orchestrator::WorkflowOrchestrator;
pub use provenance_tracker::ProvenanceTracker;
pub use state_machine::StateMachine;
