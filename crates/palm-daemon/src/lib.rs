//! PALM Daemon library
//!
//! This module provides the core components for the PALM daemon:
//! - REST API handlers
//! - Storage backends
//! - Scheduler and reconciliation
//! - Server lifecycle management

pub mod api;
pub mod config;
pub mod error;
pub mod scheduler;
pub mod server;
pub mod storage;

pub use config::DaemonConfig;
pub use error::{ApiError, DaemonError, StorageError};
pub use scheduler::Scheduler;
pub use server::Server;
pub use storage::{InMemoryStorage, Storage};
