//! MAPLE Resonance Runtime Core
//!
//! The foundational runtime for Mapleverse, Finalverse, and iBank.
//! This runtime implements the Resonance Architecture, treating all entities
//! as Resonators participating in continuous, meaningful interaction.

mod continuity;
pub mod handle;
mod profile_manager;
mod registry;
pub mod runtime;

pub use continuity::{ContinuityProof, ContinuityRecord};
pub use handle::{
    CouplingHandle, DecouplingResult, DeferralReason, RejectionReason, ResonatorHandle,
    ScheduleHandle, ScheduleStatus, TaskId,
};
pub use profile_manager::ProfileManager;
pub use registry::ResonatorRegistry;
pub use runtime::{
    CapabilitySpec, MapleRuntime, MemorySnapshot, ResonatorIdentitySpec, ResonatorSpec,
};
