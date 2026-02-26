#![deny(unsafe_code)]
//! # maple-waf-swap-gate
//!
//! Atomic Swap Gate Protocol for the WorldLine Autopoietic Factory.
//!
//! Enforces:
//! - **I.WAF-3: Swap Atomicity** — Logic swap is atomic; no partial upgrades
//! - **I.WAF-4: Rollback Guarantee** — System can always revert to last stable state
//! - **I.WAF-5: Evidence Completeness** — No swap without satisfying EvidenceBundle

pub mod error;
pub mod gate;
pub mod rollback;
pub mod shadow;
pub mod types;

pub use error::SwapError;
pub use gate::WafSwapGate;
pub use rollback::RollbackManager;
pub use shadow::{ShadowResult, ShadowRunner, SimulatedShadowRunner};
pub use types::{Snapshot, SwapResult, UpgradeProposal};

// ── WLL canonical gate re-exports ───────────────────────────────────
/// WLL gate primitives — policy pipeline for atomic swap evaluation.
pub mod wll {
    pub use wll_gate::{
        CommitmentGate as WllGate,
        GateStage as WllGateStage,
        PolicyRule as WllPolicyRule,
    };
}
