//! PALM Registry - Registry traits and implementations
//!
//! This crate provides the registry infrastructure for PALM:
//!
//! - **AgentRegistry**: Stores agent specifications (templates for deployment)
//! - **InstanceRegistry**: Tracks running agent instances
//! - **DiscoveryService**: Enables finding instances by capability
//!
//! ## In-Memory vs Persistent
//!
//! The crate provides in-memory implementations suitable for development and testing.
//! Production deployments should use persistent backends (etcd, PostgreSQL, etc.)
//! that implement the same traits.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod error;
pub mod agent;
pub mod instance;
pub mod discovery;
pub mod memory;

// Re-exports
pub use error::{RegistryError, Result};
pub use agent::AgentRegistry;
pub use instance::InstanceRegistry;
pub use discovery::{DiscoveryQuery, DiscoveryResult, DiscoveryService, RoutingStrategy};
pub use memory::{InMemoryAgentRegistry, InMemoryDiscoveryService, InMemoryInstanceRegistry};
