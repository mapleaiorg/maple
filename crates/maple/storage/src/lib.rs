//! MAPLE unified storage abstractions.
//!
//! This crate defines a storage contract for MAPLE components:
//! - commitment/accountability records (system of record)
//! - append-only audit chains
//! - agent checkpoint state for restart/resume
//! - projection snapshots for dashboards/ops
//! - optional semantic memory for cognition assistance
//!
//! Design stance:
//! - Postgres remains the transactional source of truth.
//! - AI-friendly stores are projections/derivatives behind explicit traits.

#![deny(unsafe_code)]
#![cfg_attr(feature = "strict-docs", warn(missing_docs))]
#![cfg_attr(not(feature = "strict-docs"), allow(missing_docs))]
#![warn(rust_2018_idioms)]

mod error;
pub mod memory;
#[cfg(feature = "postgres")]
pub mod postgres;
mod model;
mod traits;

pub use error::{StorageError, StorageResult};
pub use model::{
    AgentCheckpoint, AuditAppend, AuditRecord, CommitmentRecord, ProjectionSnapshot, SemanticHit,
    SemanticRecord,
};
pub use traits::{
    AgentStateStore, AuditStore, CommitmentStore, MapleStorage, ProjectionStore, QueryWindow,
    SemanticMemoryStore,
};
