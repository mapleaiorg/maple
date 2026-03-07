//! MAPLE Local Package Store — content-addressed storage for packages, blobs, and manifests.
//!
//! This crate provides:
//!
//! - **layout**: On-disk directory structure for the `~/.maple/` store.
//! - **store**: Content-addressed blob storage, manifest management, package
//!   index tracking, garbage collection, and disk usage reporting.
//! - **error**: Store error types.

pub mod error;
pub mod layout;
pub mod store;

// Re-export primary types for convenience.
pub use error::StoreError;
pub use layout::StoreLayout;
pub use store::{DiskUsage, GcResult, IndexEntry, PackageIndex, PackageStore};
