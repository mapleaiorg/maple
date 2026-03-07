//! MAPLE Registry Catalog — artifact search and discovery.
//!
//! Provides full-text and fuzzy search over package metadata,
//! with caching for offline use.

pub mod cache;
pub mod catalog;
pub mod error;
pub mod search;

pub use cache::CatalogCache;
pub use catalog::{CatalogEntry, SearchFilter, SearchResult, VersionInfo};
pub use error::CatalogError;
pub use search::SearchEngine;
