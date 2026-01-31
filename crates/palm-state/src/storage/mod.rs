//! State storage backends.
//!
//! Provides storage abstractions for persisting Resonator state snapshots.

pub mod traits;
pub mod memory;

#[cfg(feature = "postgres")]
pub mod postgres;

pub use traits::StateStorage;
pub use memory::InMemoryStateStorage;

#[cfg(feature = "postgres")]
pub use postgres::PostgresStateStorage;
