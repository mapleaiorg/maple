//! PALM Platform Pack Contract
//!
//! This crate defines the formal interface that platform-specific implementations
//! must satisfy to integrate with PALM. A Platform Pack is a cohesive bundle of:
//!
//! - Policy configurations
//! - Health probe definitions
//! - State serialization strategies
//! - Resource constraints
//! - Recovery behaviors
//!
//! Platform Packs enable the same PALM core to orchestrate radically different
//! deployment environments (Mapleverse high-throughput, Finalverse safety-first,
//! iBank accountability-focused) without architectural fragmentation.

pub mod contract;
pub mod health;
pub mod metadata;
pub mod policy;
pub mod recovery;
pub mod resources;
pub mod state;
pub mod validation;

pub use contract::{PackError, PlatformCapabilities, PlatformPack, PlatformPackConfig};
pub use health::PlatformHealthConfig;
pub use metadata::PlatformMetadata;
pub use policy::PlatformPolicyConfig;
pub use recovery::PlatformRecoveryConfig;
pub use resources::PlatformResourceConfig;
pub use state::PlatformStateConfig;
pub use validation::{validate_pack, ValidationError, ValidationResult};
