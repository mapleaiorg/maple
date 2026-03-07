//! Local catalog cache that stores entries in memory.
//!
//! The cache is the primary data structure backing offline search and discovery.
//! Entries are indexed by their fully qualified name.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::catalog::CatalogEntry;

/// In-memory catalog cache backed by a name-indexed `HashMap`.
///
/// The cache can be serialized to JSON for persistence across sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogCache {
    /// Name -> CatalogEntry index.
    pub(crate) entries: HashMap<String, CatalogEntry>,
    /// Timestamp of the last successful sync from a registry.
    pub(crate) last_synced: Option<chrono::DateTime<chrono::Utc>>,
}

impl CatalogCache {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            last_synced: None,
        }
    }

    /// Insert or update a catalog entry.
    pub fn upsert(&mut self, entry: CatalogEntry) {
        self.entries.insert(entry.name.clone(), entry);
    }

    /// Look up an entry by its fully qualified name.
    pub fn get(&self, name: &str) -> Option<&CatalogEntry> {
        self.entries.get(name)
    }

    /// Remove an entry by name. Returns the removed entry if it existed.
    pub fn remove(&mut self, name: &str) -> Option<CatalogEntry> {
        self.entries.remove(name)
    }

    /// Return an iterator over all cached entries.
    pub fn iter(&self) -> impl Iterator<Item = &CatalogEntry> {
        self.entries.values()
    }

    /// Number of entries currently in the cache.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.last_synced = None;
    }

    /// The timestamp of the last successful sync, if any.
    pub fn last_synced(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.last_synced
    }

    /// Mark the cache as having been synced at the given time.
    pub fn mark_synced(&mut self, at: chrono::DateTime<chrono::Utc>) {
        self.last_synced = Some(at);
    }
}

impl Default for CatalogCache {
    fn default() -> Self {
        Self::new()
    }
}
