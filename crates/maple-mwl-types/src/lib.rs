//! Core type definitions for the Maple WorldLine Framework (MWL).
//!
//! This crate provides all shared MWL type definitions. No business logic — just types.
//! Every MWL crate depends on this crate.

pub mod commitment;
pub mod confidence;
pub mod governance;
pub mod ids;
pub mod resonance;
pub mod temporal;
pub mod worldline_id;

// Re-export primary types at crate root for ergonomic use.
pub use commitment::{
    CommitmentScope, CommitmentStatus, DenialReason, EffectDomain, FailureReason, Reversibility,
    TemporalBounds,
};
pub use confidence::{ConfidenceProfile, RiskClass, RiskLevel};
pub use governance::{AdjudicationDecision, Capability, CapabilityScope, PolicyDecisionCard};
pub use ids::{CapabilityId, CommitmentId, EventId, NodeId, PolicyId, ProvenanceRef};
pub use resonance::ResonanceType;
pub use temporal::TemporalAnchor;
pub use worldline_id::{IdentityMaterial, WorldlineId};
