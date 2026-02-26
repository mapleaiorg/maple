#![deny(unsafe_code)]
//! # maple-waf-context-graph
//!
//! WLL Context Graph — the content-addressed causal DAG that records
//! every evolution cycle in the WorldLine Autopoietic Factory.
//!
//! ## Architecture
//!
//! Each evolution step produces a chain of six node types:
//!
//! ```text
//! Intent → Inference → Delta → Evidence → Commitment → Consequence
//! ```
//!
//! Every node is content-addressed via BLAKE3:
//! `node_id = blake3(canonical_serialize(content + parent_ids + worldline_id + timestamp))`
//!
//! This enforces **I.WAF-1: Context Graph Integrity**.
//!
//! ## Key Types
//!
//! - [`WllNode`] — A node in the causal DAG
//! - [`NodeContent`] — Typed payload (Intent, Inference, Delta, Evidence, Commitment, Consequence)
//! - [`EvolutionChain`] — An ordered sequence of one evolution cycle
//! - [`ContentHash`] — BLAKE3 content-addressed identifier
//! - [`ContextGraphManager`] — High-level management trait
//! - [`GraphStorage`] — Pluggable storage backend trait
//! - [`GraphValidator`] — Integrity validation

pub mod error;
pub mod graph;
pub mod manager;
pub mod nodes;
pub mod storage;
pub mod types;
pub mod validation;

// Re-export primary types.
pub use error::{GraphError, StorageError, ValidationError};
pub use graph::{EvolutionChain, NodeContent, WllNode};
pub use manager::{ContextGraphManager, InMemoryContextGraphManager};
pub use nodes::{
    CommitmentNode, ConsequenceNode, DecisionPoint, DeltaNode, DeltaSizeMetrics, DeploymentMetrics,
    EvidenceBundleRef, HealthStatus, InferenceNode, IntentNode, RejectedAlternative,
    StructuredRationale, SubstrateType,
};
pub use storage::{GraphStorage, InMemoryGraphStorage};
pub use types::{ContentHash, GovernanceTier, NodeContentType, TemporalRange, ValidationResult};
pub use validation::GraphValidator;

// ── WLL canonical re-exports ────────────────────────────────────────
/// WLL primitives for content-addressing, causal DAG, and object storage.
pub mod wll {
    pub use wll_dag::ProvenanceDag as WllDag;
    pub use wll_store::{ObjectStore as WllObjectStore, InMemoryObjectStore as WllStore};
    pub use wll_crypto::ContentHasher;
}
