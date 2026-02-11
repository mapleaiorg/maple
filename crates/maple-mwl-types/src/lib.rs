//! Core type definitions for the Maple WorldLine Framework (MWL).
//!
//! This crate provides all shared MWL type definitions. No business logic — just types.
//! Every MWL crate depends on this crate.

pub mod worldline_id;
pub mod temporal;
pub mod resonance;
pub mod commitment;
pub mod confidence;
pub mod governance;
pub mod ids;

// Re-export primary types at crate root for ergonomic use.
pub use worldline_id::{WorldlineId, IdentityMaterial};
pub use temporal::TemporalAnchor;
pub use resonance::ResonanceType;
pub use commitment::{
    CommitmentScope, CommitmentStatus, DenialReason, EffectDomain, FailureReason, Reversibility,
    TemporalBounds,
};
pub use confidence::{ConfidenceProfile, RiskClass, RiskLevel};
pub use governance::{
    AdjudicationDecision, Capability, CapabilityScope, PolicyDecisionCard,
};
pub use ids::{CapabilityId, CommitmentId, EventId, NodeId, PolicyId, ProvenanceRef};
