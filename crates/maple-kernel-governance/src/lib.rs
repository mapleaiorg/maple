//! # maple-kernel-governance
//!
//! Governance Engine — AAS (Agent Accountability Service) as the normative
//! authority that governs identity, capability issuance, commitment
//! adjudication, and constitutional invariant enforcement.
//!
//! Per Whitepaper §6.1: "AAS is the normative authority of Maple AI. It
//! decides—deterministically and audibly—whether an agent's declared Commitment
//! may be allowed to bind the world."
//!
//! ## Core Components
//!
//! - **AgentAccountabilityService** — The central governance authority
//! - **CapabilityManager** — Bounded authority grants and revocations
//! - **PolicyEngine** — Policy-as-code evaluation with constitutional protection
//! - **InvariantEnforcer** — Continuous verification of all 9 constitutional invariants
//!
//! ## Constitutional Invariants
//!
//! Per I.GCP-2 (Constitutional Immutability), invariants I.1-I.9 cannot be
//! weakened by any policy or operator:
//!
//! - **I.1** WorldLine Primacy
//! - **I.2** Intrinsic Typed Memory
//! - **I.3** Commitment Boundary
//! - **I.4** Causal Provenance
//! - **I.5** Resonance-Bounded Coupling
//! - **I.6** Pluggable Evolution Laws
//! - **I.7** Safety Overrides Optimization
//! - **I.8** Substrate Independence
//! - **I.9** Implementation Provenance & Constitutional Evolution
//!
//! ## Gate Integration
//!
//! The `CapabilityManager` implements `maple_kernel_gate::CapabilityProvider`
//! and `PolicyEngine` implements `maple_kernel_gate::PolicyProvider`, enabling
//! direct integration with the Commitment Gate pipeline (replacing mocks).

pub mod aas;
pub mod capability;
pub mod error;
pub mod invariants;
pub mod policy;

pub use aas::AgentAccountabilityService;
pub use capability::{CapabilityGrant, CapabilityManager, RevocationRecord};
pub use error::{AasError, InvariantViolation, PolicyError, ViolationSeverity};
pub use invariants::{
    BoundedAuthorityInvariant, CausalProvenanceInvariant, CommitmentBoundaryInvariant,
    ImmutabilityInvariant, ImplementationProvenanceInvariant, IntrinsicMemoryInvariant, Invariant,
    InvariantEnforcer, NonCollapseInvariant, PluggableEvolutionLawsInvariant,
    PreExecutionInvariant, ResonanceBoundedCouplingInvariant, SafetyOverridesOptimizationInvariant,
    SubstrateIndependenceInvariant, SystemState, WorldlinePrimacyInvariant,
};
pub use policy::{Policy, PolicyAction, PolicyCondition, PolicyEngine};
