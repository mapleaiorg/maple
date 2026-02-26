//! WorldLine identity model with continuity chains.
//!
//! Implements persistent identity without accounts or sessions (I.1: WorldLine Primacy).
//! Identity persists across restarts, migrations, and key rotations through continuity chains.

pub mod bridge;
pub mod continuity;
pub mod error;
pub mod manager;

pub use bridge::{resonator_id_to_worldline, ResonatorIdentity};
pub use continuity::{ContinuityChain, ContinuityContext, ContinuitySegment, KeyRef};
pub use error::{ContinuityError, IdentityError};
pub use manager::{IdentityManager, IdentityRecord};

// ── WLL canonical crypto re-exports ─────────────────────────────────
//
// Re-exporting WLL crypto primitives allows downstream identity consumers
// to use the canonical signing/hashing without adding wll-crypto directly.

/// WLL crypto primitives for identity operations.
pub mod wll {
    pub use wll_crypto::{ContentHasher, Signature, SigningKey, VerifyingKey};
}

#[cfg(test)]
mod tests {
    use super::IdentityManager;

    #[test]
    fn identity_manager_is_available() {
        let _ = IdentityManager::new();
    }
}
