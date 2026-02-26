//! # maple-kernel-provenance
//!
//! Provenance Index (PVP) — causal DAG for full audit trails.
//!
//! Per I.4 (Causal Provenance): No event without lineage. Every event
//! references its causal parents. The Provenance Index maintains a DAG
//! that enables 8 query types:
//!
//! 1. **Ancestors** — all events that causally precede a given event
//! 2. **Descendants** — all events causally downstream of a given event
//! 3. **Causal path** — find a path from one event to another
//! 4. **Audit trail** — full provenance chain for a commitment
//! 5. **WorldLine history** — all events from a specific worldline
//! 6. **Regulatory slice** — all events related to a specific policy
//! 7. **Impact analysis** — how an event's consequences rippled
//! 8. **Risk contagion** — causal connections of a worldline
//!
//! The index also supports checkpoint compression (I.PVP-1) which
//! compresses old events while preserving boundary nodes for causal
//! path integrity.

pub mod checkpoint;
pub mod error;
pub mod index;
pub mod node;
pub mod reports;

pub use checkpoint::{Checkpoint, CheckpointRef};
pub use error::ProvenanceError;
pub use index::ProvenanceIndex;
pub use node::ProvenanceNode;
pub use reports::{ContagionReport, ImpactReport};

// ── WLL canonical DAG re-exports ────────────────────────────────────
/// WLL DAG primitives — causal ancestry tracking and provenance.
pub mod wll {
    pub use wll_dag::{
        ProvenanceDag as WllDag,
        DagNode as WllDagNode,
    };
}
