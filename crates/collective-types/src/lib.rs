//! Collective Resonator Domain Types
//!
//! This crate defines the domain types for Collective Resonators—
//! multi-agent organizations that coordinate commitments, enforce policy,
//! allocate resources, and maintain audit trails.
//!
//! # Key Concepts
//!
//! - **Collective Resonator**: NOT a super-agent. It IS a commitment
//!   coordination mechanism, policy enforcement boundary, resource allocation
//!   unit, and audit surface.
//! - **Role–Capability–Permit Graph (RCPG)**: Roles grant capabilities,
//!   capabilities require permits, permits are scoped and limited.
//! - **Threshold Commitments**: Collective decisions requiring M-of-N,
//!   role-based, weighted, or risk-tiered approval.
//! - **Treasury**: Financial state management with accounts, escrows,
//!   and allocation tracking.
//! - **Audit Journal**: Receipt-based accountability with disputes and sanctions.
//!
//! # Architecture
//!
//! This is a pure types crate with no runtime dependencies. All types
//! implement `Clone`, `Debug`, `Serialize`, `Deserialize`. IDs use the
//! newtype pattern and implement `Display`, `generate()`, and `new()`.

#![deny(unsafe_code)]

mod audit;
mod budget;
mod capability;
mod collective;
mod errors;
mod membership;
mod permit;
mod role;
mod threshold;
mod treasury;

pub use audit::*;
pub use budget::*;
pub use capability::*;
pub use collective::*;
pub use errors::*;
pub use membership::*;
pub use permit::*;
pub use role::*;
pub use threshold::*;
pub use treasury::*;
