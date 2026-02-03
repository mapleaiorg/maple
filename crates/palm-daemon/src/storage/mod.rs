//! Storage layer for palm-daemon
//!
//! Provides persistent storage for specs, deployments, and instances.

mod memory;
mod postgres;
mod traits;

pub use memory::InMemoryStorage;
pub use postgres::PostgresStorage;
#[allow(unused_imports)]
pub use traits::{
    ActivityStorage, DeploymentStorage, EventStorage, InstanceStorage, PlaygroundConfigStorage,
    ResonatorStorage, SnapshotStorage, SpecStorage, Storage,
};
