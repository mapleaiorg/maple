use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use tracing::debug;

use crate::entry::{MemoryClass, MemoryEntry, MemoryId};
use crate::error::MemoryError;
use crate::filter::MemoryFilter;

/// Configuration for the Working Plane.
#[derive(Clone, Debug)]
pub struct WorkingPlaneConfig {
    /// Maximum entries in sensory buffer (default: 1000)
    pub sensory_max_entries: usize,
    /// Maximum age for sensory entries (default: 5 minutes)
    pub sensory_max_age: Duration,
    /// Maximum entries in active context (default: 100)
    pub active_max_entries: usize,
}

impl Default for WorkingPlaneConfig {
    fn default() -> Self {
        Self {
            sensory_max_entries: 1000,
            sensory_max_age: Duration::from_secs(300),
            active_max_entries: 100,
        }
    }
}

/// Working Plane — volatile, reasoning-time memory.
///
/// Contains current context and recent sensory input.
/// CAN be lost on crash — rebuildable from Episodic + Fabric.
pub struct WorkingPlane {
    sensory: SensoryBuffer,
    active: ActiveContext,
}

/// Summary of current working plane state.
#[derive(Clone, Debug)]
pub struct ContextSummary {
    pub sensory_count: usize,
    pub active_count: usize,
    pub total_entries: usize,
}

impl WorkingPlane {
    pub fn new(config: WorkingPlaneConfig) -> Self {
        Self {
            sensory: SensoryBuffer {
                entries: VecDeque::new(),
                max_entries: config.sensory_max_entries,
                max_age: config.sensory_max_age,
            },
            active: ActiveContext {
                entries: HashMap::new(),
                max_entries: config.active_max_entries,
            },
        }
    }

    /// Store a sensory entry (auto-expires old entries via FIFO).
    pub fn store_sensory(&mut self, entry: MemoryEntry) -> Result<MemoryId, MemoryError> {
        if entry.class != MemoryClass::Sensory {
            return Err(MemoryError::WrongClass {
                expected: "Sensory".into(),
                actual: entry.class.to_string(),
            });
        }

        let id = entry.id.clone();

        // FIFO eviction: remove oldest if at capacity
        while self.sensory.entries.len() >= self.sensory.max_entries {
            self.sensory.entries.pop_front();
        }

        self.sensory.entries.push_back(entry);
        debug!(id = %id, "Stored sensory entry");
        Ok(id)
    }

    /// Promote a sensory entry to active context.
    pub fn promote_to_active(&mut self, id: &MemoryId) -> Result<(), MemoryError> {
        let pos = self
            .sensory
            .entries
            .iter()
            .position(|e| e.id == *id)
            .ok_or_else(|| MemoryError::NotFound(id.to_string()))?;

        let mut entry = self.sensory.entries.remove(pos).unwrap();
        entry.class = MemoryClass::Active;

        // Evict oldest active if at capacity
        if self.active.entries.len() >= self.active.max_entries {
            // Remove the entry with the oldest created timestamp
            if let Some(oldest_id) = self
                .active
                .entries
                .values()
                .min_by_key(|e| e.created)
                .map(|e| e.id.clone())
            {
                self.active.entries.remove(&oldest_id);
            }
        }

        let id = entry.id.clone();
        self.active.entries.insert(id, entry);
        Ok(())
    }

    /// Store directly in active context.
    pub fn store_active(&mut self, entry: MemoryEntry) -> Result<MemoryId, MemoryError> {
        if entry.class != MemoryClass::Active {
            return Err(MemoryError::WrongClass {
                expected: "Active".into(),
                actual: entry.class.to_string(),
            });
        }

        // Evict oldest if at capacity
        if self.active.entries.len() >= self.active.max_entries {
            if let Some(oldest_id) = self
                .active
                .entries
                .values()
                .min_by_key(|e| e.created)
                .map(|e| e.id.clone())
            {
                self.active.entries.remove(&oldest_id);
            }
        }

        let id = entry.id.clone();
        self.active.entries.insert(id.clone(), entry);
        debug!(id = %id, "Stored active entry");
        Ok(id)
    }

    /// Query active context entries matching a filter.
    pub fn query_active(&self, filter: &MemoryFilter) -> Vec<&MemoryEntry> {
        self.active
            .entries
            .values()
            .filter(|e| filter.matches(e))
            .collect()
    }

    /// Query sensory buffer entries matching a filter.
    pub fn query_sensory(&self, filter: &MemoryFilter) -> Vec<&MemoryEntry> {
        self.sensory
            .entries
            .iter()
            .filter(|e| filter.matches(e))
            .collect()
    }

    /// Query all working plane entries matching a filter.
    pub fn query(&self, filter: &MemoryFilter) -> Vec<&MemoryEntry> {
        let mut results = self.query_sensory(filter);
        results.extend(self.query_active(filter));
        results
    }

    /// Get current context summary.
    pub fn context_summary(&self) -> ContextSummary {
        ContextSummary {
            sensory_count: self.sensory.entries.len(),
            active_count: self.active.entries.len(),
            total_entries: self.sensory.entries.len() + self.active.entries.len(),
        }
    }

    /// Clear all working memory (e.g., on crash recovery).
    pub fn clear(&mut self) {
        self.sensory.entries.clear();
        self.active.entries.clear();
        debug!("Working plane cleared");
    }

    /// Expire old sensory entries based on max_age.
    /// Returns the number of entries removed.
    pub fn gc(&mut self) -> usize {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let max_age_ms = self.sensory.max_age.as_millis() as u64;
        let cutoff = now_ms.saturating_sub(max_age_ms);

        let before = self.sensory.entries.len();
        self.sensory
            .entries
            .retain(|e| e.created.physical_ms >= cutoff);
        let removed = before - self.sensory.entries.len();

        if removed > 0 {
            debug!(removed = removed, "Sensory GC removed expired entries");
        }
        removed
    }

    /// Drain all active context entries (used during consolidation).
    pub(crate) fn drain_active(&mut self) -> Vec<MemoryEntry> {
        self.active.entries.drain().map(|(_, v)| v).collect()
    }

    /// Get a reference to a specific active entry.
    pub fn get_active(&self, id: &MemoryId) -> Option<&MemoryEntry> {
        self.active.entries.get(id)
    }

    /// Get sensory buffer capacity info.
    pub fn sensory_len(&self) -> usize {
        self.sensory.entries.len()
    }

    /// Get active context size.
    pub fn active_len(&self) -> usize {
        self.active.entries.len()
    }
}

/// Sensory Buffer — high-bandwidth, short-retention raw input.
struct SensoryBuffer {
    entries: VecDeque<MemoryEntry>,
    max_entries: usize,
    max_age: Duration,
}

/// Active Context — current reasoning state.
struct ActiveContext {
    entries: HashMap<MemoryId, MemoryEntry>,
    max_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{MemoryContent, provenance_from};
    use maple_mwl_types::{EventId, IdentityMaterial, WorldlineId};

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn sensory_entry(text: &str) -> MemoryEntry {
        MemoryEntry::builder(
            MemoryClass::Sensory,
            MemoryContent::Text(text.into()),
            provenance_from(EventId::new()),
            test_worldline(),
        )
        .build()
    }

    fn active_entry(text: &str) -> MemoryEntry {
        MemoryEntry::builder(
            MemoryClass::Active,
            MemoryContent::Text(text.into()),
            provenance_from(EventId::new()),
            test_worldline(),
        )
        .build()
    }

    #[test]
    fn sensory_buffer_fifo_eviction() {
        let config = WorkingPlaneConfig {
            sensory_max_entries: 3,
            ..Default::default()
        };
        let mut wp = WorkingPlane::new(config);

        for i in 0..5 {
            wp.store_sensory(sensory_entry(&format!("s{}", i))).unwrap();
        }

        assert_eq!(wp.sensory_len(), 3);
        // Should have s2, s3, s4 (oldest evicted)
        let entries: Vec<_> = wp.query_sensory(&MemoryFilter::new());
        let texts: Vec<_> = entries.iter().filter_map(|e| match &e.content {
            MemoryContent::Text(t) => Some(t.as_str()),
            _ => None,
        }).collect();
        assert_eq!(texts, vec!["s2", "s3", "s4"]);
    }

    #[test]
    fn sensory_gc_removes_old_entries() {
        let config = WorkingPlaneConfig {
            sensory_max_age: Duration::from_millis(0), // expire immediately
            ..Default::default()
        };
        let mut wp = WorkingPlane::new(config);

        wp.store_sensory(sensory_entry("old")).unwrap();

        // GC should remove everything since max_age is 0
        std::thread::sleep(Duration::from_millis(1));
        let removed = wp.gc();
        assert_eq!(removed, 1);
        assert_eq!(wp.sensory_len(), 0);
    }

    #[test]
    fn promote_sensory_to_active() {
        let mut wp = WorkingPlane::new(WorkingPlaneConfig::default());

        let id = wp.store_sensory(sensory_entry("promote me")).unwrap();
        assert_eq!(wp.sensory_len(), 1);
        assert_eq!(wp.active_len(), 0);

        wp.promote_to_active(&id).unwrap();
        assert_eq!(wp.sensory_len(), 0);
        assert_eq!(wp.active_len(), 1);

        let entry = wp.get_active(&id).unwrap();
        assert_eq!(entry.class, MemoryClass::Active);
    }

    #[test]
    fn promote_nonexistent_fails() {
        let mut wp = WorkingPlane::new(WorkingPlaneConfig::default());
        let fake_id = MemoryId::new();
        assert!(wp.promote_to_active(&fake_id).is_err());
    }

    #[test]
    fn active_context_max_entries() {
        let config = WorkingPlaneConfig {
            active_max_entries: 3,
            ..Default::default()
        };
        let mut wp = WorkingPlane::new(config);

        for i in 0..5 {
            wp.store_active(active_entry(&format!("a{}", i))).unwrap();
        }

        assert_eq!(wp.active_len(), 3);
    }

    #[test]
    fn store_wrong_class_rejected() {
        let mut wp = WorkingPlane::new(WorkingPlaneConfig::default());

        let entry = MemoryEntry::builder(
            MemoryClass::Episodic,
            MemoryContent::Text("wrong".into()),
            provenance_from(EventId::new()),
            test_worldline(),
        )
        .build();

        assert!(wp.store_sensory(entry.clone()).is_err());
        assert!(wp.store_active(entry).is_err());
    }

    #[test]
    fn clear_empties_both_buffers() {
        let mut wp = WorkingPlane::new(WorkingPlaneConfig::default());

        wp.store_sensory(sensory_entry("s1")).unwrap();
        wp.store_active(active_entry("a1")).unwrap();
        assert_eq!(wp.context_summary().total_entries, 2);

        wp.clear();
        assert_eq!(wp.sensory_len(), 0);
        assert_eq!(wp.active_len(), 0);
    }

    #[test]
    fn query_active_with_filter() {
        let mut wp = WorkingPlane::new(WorkingPlaneConfig::default());

        wp.store_active(active_entry("foo bar")).unwrap();
        wp.store_active(active_entry("baz qux")).unwrap();

        let filter = MemoryFilter::new().with_content_contains("foo");
        let results = wp.query_active(&filter);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn drain_active_returns_all() {
        let mut wp = WorkingPlane::new(WorkingPlaneConfig::default());

        wp.store_active(active_entry("a1")).unwrap();
        wp.store_active(active_entry("a2")).unwrap();
        wp.store_active(active_entry("a3")).unwrap();

        let drained = wp.drain_active();
        assert_eq!(drained.len(), 3);
        assert_eq!(wp.active_len(), 0);
    }

    #[test]
    fn context_summary() {
        let mut wp = WorkingPlane::new(WorkingPlaneConfig::default());

        wp.store_sensory(sensory_entry("s1")).unwrap();
        wp.store_sensory(sensory_entry("s2")).unwrap();
        wp.store_active(active_entry("a1")).unwrap();

        let summary = wp.context_summary();
        assert_eq!(summary.sensory_count, 2);
        assert_eq!(summary.active_count, 1);
        assert_eq!(summary.total_entries, 3);
    }
}
