#![deny(unsafe_code)]
//! Governance policies and enforcement for WAF worldline operations.
//!
//! This crate provides:
//! - **Error types** for governance failures ([`GovernanceError`]).
//! - **Core types** for approval workflows ([`ApprovalRequest`], [`ApprovalDecision`], [`ApprovalStatus`]).
//! - **Tier classification** engine ([`GovernanceTierEngine`]).
//! - **Approval management** trait and simulated implementation ([`ApprovalManager`], [`SimulatedApprovalManager`]).

pub mod approval;
pub mod error;
pub mod tier_engine;
pub mod types;

// Re-exports for convenience.
pub use approval::{ApprovalManager, SimulatedApprovalManager};
pub use error::GovernanceError;
pub use tier_engine::GovernanceTierEngine;
pub use types::{ApprovalDecision, ApprovalRequest, ApprovalStatus};
