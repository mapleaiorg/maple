//! Storage layer for palm-daemon
//!
//! Provides persistent storage for specs, deployments, and instances.

mod memory;
mod traits;

pub use memory::InMemoryStorage;
pub use traits::{
    DeploymentStorage, EventStorage, InstanceStorage, SnapshotStorage, SpecStorage, Storage,
};
