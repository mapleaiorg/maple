//! Core type definitions for the WorldLine Framework.
//!
//! This crate provides shared WorldLine type definitions and canonical IDs.

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

#[cfg(test)]
mod tests {
    use super::WorldlineId;

    #[test]
    fn worldline_id_is_available() {
        let _ = WorldlineId::ephemeral();
    }
}
