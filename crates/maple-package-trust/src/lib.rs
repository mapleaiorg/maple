//! MAPLE Package Trust — signing, verification, SBOM generation, and build attestation.
//!
//! This crate provides three pillars of supply-chain trust for MAPLE packages:
//!
//! - **sign**: Ed25519 cryptographic signing and verification of package digests.
//!   Supports both self-contained verification and trust-anchor (pinned-key)
//!   verification.
//! - **sbom**: SBOM generation from a resolved build lockfile. The MAPLE SBOM
//!   format (`maple-sbom/v1`) extends standard SBOMs with AI-specific metadata
//!   including model provenance, evaluation baselines, and data classification.
//! - **attest**: Build attestation that bundles provenance, SBOM, eval results,
//!   and a cryptographic signature into a single verifiable document.

pub mod attest;
pub mod sbom;
pub mod sign;

// Re-export primary types for convenience.
pub use attest::{
    AttestationSubject, BuildAttestation, EvalResult, create_attestation, verify_attestation,
};
pub use sbom::{
    AiEvalReference, AiSbomMetadata, MapleSbom, ModelSbomInfo, SbomComponent, SbomPackageInfo,
    SbomRelationship, SbomRelationshipType, generate_sbom,
};
pub use sign::{PackageSignature, SignError, sign_package, verify_against_trust_anchor, verify_signature};
