//! MAPLE Resonance Runtime Core
//!
//! The foundational runtime for Mapleverse, Finalverse, and iBank.
//! This runtime implements the Resonance Architecture, treating all entities
//! as Resonators participating in continuous, meaningful interaction.

pub mod runtime;
mod registry;
mod profile_manager;
pub mod handle;
mod continuity;

pub use runtime::{MapleRuntime, ResonatorSpec, ResonatorIdentitySpec, CapabilitySpec, MemorySnapshot};
pub use registry::ResonatorRegistry;
pub use profile_manager::ProfileManager;
pub use handle::{
    ResonatorHandle, CouplingHandle, ScheduleHandle, DecouplingResult,
    TaskId, RejectionReason, DeferralReason, ScheduleStatus
};
pub use continuity::{ContinuityProof, ContinuityRecord};
