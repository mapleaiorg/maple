//! MAPLE Guard Firewall — deny-by-default capability firewall.
//!
//! Provides capability declarations, grants, and runtime enforcement
//! with comprehensive audit logging.

pub mod audit;
pub mod engine;
pub mod firewall;
pub mod grants;

pub use audit::{AuditEntry, AuditLog, AuditOutcome};
pub use firewall::{CapabilityFirewall, FirewallDecision, ToolCallRequest};
pub use grants::{CapabilityGrant, GrantCondition, GrantScope, RateLimit};
