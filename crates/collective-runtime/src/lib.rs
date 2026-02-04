//! Collective Resonator Runtime
//!
//! This crate provides the runtime implementation for Collective Resonators—
//! multi-agent organizations that coordinate commitments, enforce policy,
//! allocate resources, and maintain audit trails.
//!
//! # Architecture
//!
//! The [`CollectiveResonator`] is the main entry point. It composes
//! specialized managers, each handling one aspect of collective coordination:
//!
//! - [`MembershipManager`] — Member lifecycle (add, suspend, expel, reinstate)
//! - [`RoleRouter`] — Routes actions to eligible resonators via Role-Capability-Permit graph
//! - [`PolicyEnforcer`] — Validates actions against collective policies and budgets
//! - [`ThresholdEngine`] — Manages multi-party approval for collective decisions
//! - [`TreasuryManager`] — Financial operations with audit trails
//! - [`CollectiveAttentionAllocator`] — Attention economics and coupling slot management
//! - [`ContinuityManager`] — State checkpointing and recovery
//!
//! # Key Invariants
//!
//! 1. A Collective does NOT "think" — it coordinates commitments
//! 2. Safety overrides optimization (Invariant 6)
//! 3. Coupling is bounded by attention (Invariant 5)
//! 4. Failure must be explicit (Invariant 8)
//! 5. Every significant action produces a receipt (accountability)
//!
//! # Example
//!
//! ```rust
//! use collective_runtime::CollectiveResonator;
//! use collective_types::{CollectiveSpec, RoleId, Role, CapabilityId, Capability, ActionType};
//! use resonator_types::ResonatorId;
//!
//! // Create a collective
//! let spec = CollectiveSpec::new("Acme Corp", "Trading firm", ResonatorId::new("founder"));
//! let mut collective = CollectiveResonator::new(spec);
//!
//! // Add members
//! collective.add_member(ResonatorId::new("trader-1"), vec![RoleId::new("trader")]).unwrap();
//!
//! // The collective coordinates — it never acts directly
//! assert_eq!(collective.active_member_count(), 1);
//! ```

#![deny(unsafe_code)]

pub mod attention_allocator;
pub mod collective_resonator;
pub mod continuity;
pub mod membership_manager;
pub mod policy_enforcer;
pub mod role_router;
pub mod threshold_engine;
pub mod treasury_manager;

// Re-export main types for convenience
pub use attention_allocator::{AttentionConfig, CollectiveAttentionAllocator};
pub use collective_resonator::CollectiveResonator;
pub use continuity::{CollectiveCheckpoint, ContinuityManager};
pub use membership_manager::MembershipManager;
pub use policy_enforcer::{PolicyCheckRequest, PolicyConfig, PolicyDecision, PolicyEnforcer};
pub use role_router::{ActionRequest, RoleRouter, RouteResult};
pub use threshold_engine::{SignatureResult, ThresholdEngine};
pub use treasury_manager::TreasuryManager;
