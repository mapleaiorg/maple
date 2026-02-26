//! Core type definitions for the WorldLine Framework.
//!
//! This crate provides shared WorldLine type definitions and canonical IDs.
//! It also re-exports canonical types from the WLL (WorldLine Ledger) crate
//! for downstream consumers that need ledger primitives.

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

// ── WLL canonical re-exports ──────────────────────────────────────────
//
// The WorldLine Ledger (WLL) provides production-grade implementations of:
//   - Content-addressable object storage
//   - Append-only receipt chains with commitment/outcome pairs
//   - Provenance DAG with causal ancestry tracking
//   - Cryptographic hashing (BLAKE3) and signing (Ed25519)
//
// Re-exporting here allows downstream maple crates to depend only on
// `worldline-types` while gaining full access to WLL primitives.

/// WLL canonical types — content addressing, receipts, and ledger primitives.
pub mod wll {
    // Core identity & types
    pub use wll_types::{
        ObjectId,
        WorldlineId as WllWorldlineId,
        IdentityMaterial as WllIdentityMaterial,
        CommitmentId as WllCommitmentId,
        CommitmentClass,
        EvidenceBundle as WllEvidenceBundle,
        Capability as WllCapability,
        CapabilityId as WllCapabilityId,
        CapabilityScope as WllCapabilityScope,
        Reversibility as WllReversibility,
        ReceiptId, ReceiptKind,
    };

    // Decision is in the commitment submodule
    pub use wll_types::commitment::Decision;

    // Temporal
    pub use wll_types::TemporalAnchor as WllTemporalAnchor;

    // Cryptography
    pub use wll_crypto::{
        ContentHasher,
        Signature, SigningKey, VerifyingKey,
        MerkleProof, MerkleTree, Side,
        HasReceiptHash, HashChainVerifier,
    };
}

#[cfg(test)]
mod tests {
    use super::WorldlineId;

    #[test]
    fn worldline_id_is_available() {
        let _ = WorldlineId::ephemeral();
    }
}
