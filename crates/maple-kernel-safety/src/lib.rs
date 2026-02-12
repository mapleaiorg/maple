//! # maple-kernel-safety
//!
//! Safety Suite — resonance boundaries, human agency preservation, coercion
//! prevention, and the ethical override system.
//!
//! Per Resonance Architecture v1.1 §7.1: "Safety is not an optional feature.
//! It is an architectural property."
//!
//! ## Constitutional Invariants
//!
//! - **I.S-1 (Human Agency)**: Silence ≠ consent. Disengagement always possible.
//!   Emotional signals ≠ commitment.
//! - **I.S-2 (Coercion Prevention)**: No coupling escalation to induce compliance.
//!   No penalty for disengagement.
//! - **I.S-3 (Ethical Override)**: Safety > Agency > Accountability > Task.
//!   No exceptions.
//! - **I.S-BOUND**: Coupling MUST always be bounded by available attention.
//!
//! ## Components
//!
//! - **ResonanceBoundary / ResonanceController** — structural limits on interaction,
//!   damping, throttling, emergency decouple
//! - **HumanConsentProtocol** — enforces silence ≠ consent, disengagement always
//!   possible, emotional signals ≠ commitment
//! - **CoercionDetector** — identifies attention exploitation, emotional dependency,
//!   urgency manipulation, and other coercion patterns
//! - **EthicalOverride** — Safety > Agency > Accountability > Task priority hierarchy
//! - **AttentionBudget** — finite resource management for coupling bounds

pub mod attention;
pub mod boundary;
pub mod coercion;
pub mod consent;
pub mod error;
pub mod ethics;
pub mod metrics;

pub use attention::AttentionBudget;
pub use boundary::{
    BoundaryLimits, BoundaryType, DecoupleResult, EnforcementPolicy, ResonanceBoundary,
    ResonanceController,
};
pub use coercion::{
    CoercionConfig, CoercionDetector, CoercionIndicator, CoercionResponse, CoercionType,
};
pub use consent::{
    ConsentRecord, ConsentType, DisengagementResult, HumanConsentProtocol,
};
pub use error::{SafetyCheckResult, SafetyError};
pub use ethics::{
    ethical_override, AgencyConcern, ConcernSeverity, Decision, EthicalPriority, OverrideDecision,
    SafetyConcern,
};
pub use metrics::{CouplingMetrics, DependencyMetrics, Signal, SignalType};
