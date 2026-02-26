//! Two-Plane Memory Engine for the MWL Kernel.
//!
//! Implements Constitutional Invariant I.2 (Intrinsic Typed Memory):
//! - **Working Plane**: Volatile, reasoning-time context (sensory + active).
//!   Rebuildable from Episodic + Fabric after crash.
//! - **Episodic Plane**: Persistent, provenance-bound history (episodic + semantic).
//!   Every entry MUST reference an EventId in the Event Fabric.
//!
//! Provenance binding ensures all memory has a causal trail back to the Event Fabric.

pub mod engine;
pub mod entry;
pub mod episodic;
pub mod error;
pub mod filter;
pub mod working;

pub use engine::{
    ConsolidationReport, MemoryEngine, MemoryEngineConfig, MemoryStats, RebuildReport,
};
pub use entry::{
    nil_provenance, provenance_from, MemoryClass, MemoryContent, MemoryEntry, MemoryEntryBuilder,
    MemoryId, MemoryPlane,
};
pub use episodic::EpisodicPlane;
pub use error::MemoryError;
pub use filter::MemoryFilter;
pub use working::{ContextSummary, WorkingPlane, WorkingPlaneConfig};

// ── WLL canonical store re-exports ──────────────────────────────────
/// WLL store primitives — content-addressed object storage.
pub mod wll {
    pub use wll_store::{
        ObjectStore as WllObjectStore,
        InMemoryObjectStore as WllInMemoryStore,
        StoredObject as WllStoredObject,
    };
}
