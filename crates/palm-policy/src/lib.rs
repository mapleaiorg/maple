//! # PALM Policy System
//!
//! Policy evaluation system for PALM with platform-specific invariants.
//!
//! ## Overview
//!
//! The policy system enforces platform-specific invariants for PALM operations.
//! Each platform has unique requirements:
//!
//! - **Mapleverse**: Throughput-first, allows high-velocity operations
//! - **Finalverse**: Safety-first, requires human approval for critical operations
//! - **IBank**: Accountability-first, requires comprehensive audit trails
//! - **Development**: Minimal restrictions for development/testing
//!
//! ## Key Components
//!
//! - [`PolicyGate`]: Trait for policy evaluation
//! - [`PolicyEvaluator`]: High-level service for evaluating operations
//! - [`PolicyDecision`]: Result of policy evaluation
//! - [`PolicyDecisionCard`]: Audit card with full evaluation details
//! - [`PolicyEvaluationContext`]: Context for policy evaluation
//!
//! ## Platform Policies
//!
//! - [`BaseInvariantPolicy`]: Base invariants for all platforms
//! - [`MapleverseThroughputPolicy`]: Throughput-optimized policy
//! - [`FinalverseSafetyPolicy`]: Safety-first policy
//! - [`IBankAccountabilityPolicy`]: Accountability-first policy
//!
//! ## Example
//!
//! ```rust,no_run
//! use palm_policy::{
//!     PolicyEvaluator, PolicyEvaluationContext, PolicyDecision,
//! };
//! use palm_types::{PlatformProfile, policy::PalmOperation};
//!
//! # async fn example() {
//! // Create an evaluator for Finalverse
//! let evaluator = PolicyEvaluator::new(PlatformProfile::Finalverse);
//!
//! // Create evaluation context
//! let ctx = PolicyEvaluationContext::new("user-123", PlatformProfile::Finalverse)
//!     .with_environment("production");
//!
//! // Evaluate an operation
//! let operation = PalmOperation::CreateDeployment {
//!     spec_id: "my-agent-v1".into(),
//! };
//!
//! match evaluator.evaluate(&operation, &ctx).await {
//!     Ok(PolicyDecision::Allow) => {
//!         println!("Operation allowed");
//!     }
//!     Ok(PolicyDecision::Deny { reason, .. }) => {
//!         println!("Operation denied: {}", reason);
//!     }
//!     Ok(PolicyDecision::RequiresApproval { approvers, reason, .. }) => {
//!         println!("Approval required from {:?}: {}", approvers, reason);
//!     }
//!     Ok(PolicyDecision::Hold { reason, .. }) => {
//!         println!("Operation on hold: {}", reason);
//!     }
//!     Err(e) => {
//!         println!("Policy evaluation failed: {}", e);
//!     }
//! }
//! # }
//! ```
//!
//! ## Composing Policies
//!
//! Policies can be composed using [`ComposedPolicyGate`]:
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use palm_policy::{
//!     ComposedPolicyGate, EvaluationMode,
//!     BaseInvariantPolicy, FinalverseSafetyPolicy,
//! };
//!
//! let composed = ComposedPolicyGate::new("custom", "Custom Policy Stack")
//!     .add_gate(Arc::new(BaseInvariantPolicy::new()))
//!     .add_gate(Arc::new(FinalverseSafetyPolicy::new()))
//!     .with_evaluation_mode(EvaluationMode::MostRestrictive);
//! ```
//!
//! ## Audit Trail
//!
//! Every evaluation produces a [`PolicyDecisionCard`] with full audit details:
//!
//! ```rust,no_run
//! use palm_policy::{PolicyEvaluator, PolicyEvaluationContext};
//! use palm_types::{PlatformProfile, policy::PalmOperation};
//!
//! # async fn example() {
//! let evaluator = PolicyEvaluator::new(PlatformProfile::IBank);
//! let ctx = PolicyEvaluationContext::new("user-123", PlatformProfile::IBank);
//! let op = PalmOperation::CreateDeployment { spec_id: "agent".into() };
//!
//! let card = evaluator.evaluate_with_card(&op, &ctx).await.unwrap();
//!
//! println!("Decision ID: {}", card.id);
//! println!("Operation: {}", card.operation);
//! println!("Actor: {}", card.actor_id);
//! println!("Allowed: {}", card.was_allowed());
//! println!("Policies evaluated: {}", card.policies_evaluated.len());
//! # }
//! ```

#![deny(unsafe_code)]
#![cfg_attr(feature = "strict-docs", warn(missing_docs))]
#![cfg_attr(not(feature = "strict-docs"), allow(missing_docs))]

pub mod context;
pub mod decision;
pub mod error;
pub mod evaluator;
pub mod gate;
pub mod policies;

// Re-exports
pub use context::{
    ActorType, ApprovalScope, HumanApproval, PolicyEvaluationContext, QuotaUsage,
};
pub use decision::{PolicyDecision, PolicyDecisionCard, PolicyEvaluationRecord};
pub use error::{PolicyError, Result};
pub use evaluator::{PolicyEvaluator, PolicyEvaluatorBuilder};
pub use gate::{
    AllowAllPolicyGate, ComposedPolicyGate, DenyAllPolicyGate, EvaluationMode, PolicyGate,
};
pub use policies::{
    create_platform_policy, BaseInvariantPolicy, FinalverseSafetyPolicy,
    IBankAccountabilityPolicy, MapleverseThroughputPolicy,
};
