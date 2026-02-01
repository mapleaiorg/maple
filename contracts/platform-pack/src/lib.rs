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
pub mod policy;
pub mod health;
pub mod state;
pub mod resources;
pub mod recovery;
pub mod metadata;
pub mod validation;

pub use contract::{PlatformPack, PlatformPackConfig, PlatformCapabilities, PackError};
pub use policy::PlatformPolicyConfig;
pub use health::PlatformHealthConfig;
pub use state::PlatformStateConfig;
pub use resources::PlatformResourceConfig;
pub use recovery::PlatformRecoveryConfig;
pub use metadata::PlatformMetadata;
pub use validation::{validate_pack, ValidationResult, ValidationError};
