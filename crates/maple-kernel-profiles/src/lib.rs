//! # maple-kernel-profiles
//!
//! WorldLine Profiles — canonical behavioral specifications for worldline types,
//! cross-profile constraint merging, and enforcement.
//!
//! ## Canonical Profiles
//!
//! Five canonical profiles define the fundamental worldline archetypes:
//!
//! - **Human** — highest agency, strictest safety protections, informed consent,
//!   coercion detection, full human oversight
//! - **Agent** — bounded autonomy, audit trails required, human approval for
//!   high-risk, no irreversible actions without oversight
//! - **Financial** — conservative risk tolerance, strict auditability, highest
//!   confidence thresholds, maps to iBank archetype
//! - **World** — environmental/contextual, read-heavy, many concurrent couplings,
//!   implicit consent, low consequence scope
//! - **Coordination** — orchestration, high autonomy within bounded scope, many
//!   concurrent couplings, strict cascade limits
//!
//! ## Constitutional Invariant
//!
//! - **I.PROF-1 (Maximum Restriction Principle)**: In any cross-profile
//!   interaction, the most restrictive constraint from either profile applies
//!   to every dimension. No profile can weaken another's safety guarantees.
//!
//! ## Profile Dimensions
//!
//! Each profile specifies constraints across six dimensions:
//!
//! - **CouplingLimits** — max strength, concurrent couplings, asymmetry, consent
//! - **AttentionBudgetConfig** — capacity, reserve, per-coupling fraction
//! - **IntentResolutionRules** — confidence threshold, multi-signal, stabilization
//! - **CommitmentAuthority** — allowed domains, risk class, irreversibility, audit
//! - **ConsequenceScopeLimit** — affected parties, cascade depth, cross-domain
//! - **HumanInvolvementConfig** — oversight level, coercion detection, agency protection

pub mod canonical;
pub mod dimensions;
pub mod enforcer;
pub mod error;
pub mod merge;
pub mod platform;

pub use canonical::{
    agent_profile, canonical_profile, coordination_profile, financial_profile, human_profile,
    world_profile,
};
pub use dimensions::{
    AttentionBudgetConfig, CommitmentAuthority, ConsentLevel, ConsequenceScopeLimit, CouplingLimits,
    ExhaustionBehavior, HumanInvolvementConfig, IntentResolutionRules, OversightLevel, ProfileType,
    ReversibilityPreference, WorldlineProfile,
};
pub use enforcer::{CommitmentProposal, CouplingProposal, ProfileEnforcer};
pub use error::{
    EnforcementResult, ProfileError, ProfileViolation, ViolationDimension, ViolationSeverity,
};
pub use merge::merged_constraints;
pub use platform::{
    active_profile_types, finalverse_platform, ibank_platform, is_profile_active,
    mapleverse_platform, platform_profile, PlatformProfileConfig,
};
