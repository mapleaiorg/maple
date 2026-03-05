//! # maple-crypto
//!
//! Cryptographic primitives for MAPLE: Ed25519 key pairs, signatures,
//! BLAKE3 content hashing, and genesis key material for worldline creation.
//!
//! This crate is intentionally standalone — no dependency on any type system
//! crate. Domain-specific types (WorldlineId, etc.) are handled by consumers
//! who convert to/from the raw byte types used here.

mod content_hash;
mod keys;
mod signature;

pub use content_hash::ContentHash;
pub use keys::{GenesisKeyMaterial, KeyPair};
pub use signature::{Signature, SignerId};

/// Errors from cryptographic operations.
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Invalid key material: {0}")]
    InvalidKeyMaterial(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}
