use std::collections::{BTreeMap, HashMap};

use maple_mwl_types::{ProvenanceRef, TemporalAnchor, WorldlineId};
use tracing::debug;

use crate::entry::{MemoryClass, MemoryEntry, MemoryId};
use crate::error::MemoryError;
use crate::filter::MemoryFilter;

/// Episodic Plane — persistent, provenance-bound memory.
/// This is the system of record. Everything important ends up here.
pub struct EpisodicPlane {
    episodic: EpisodicStore,
    semantic: SemanticStore,
}

impl EpisodicPlane {
    pub fn new() -> Self {
        Self {
            episodic: EpisodicStore {
                entries: BTreeMap::new(),
            },
            semantic: SemanticStore {
                entries: HashMap::new(),
            },
        }
    }

    /// Store an episodic entry (MUST have provenance).
    pub fn store_episodic(&mut self, entry: MemoryEntry) -> Result<MemoryId, MemoryError> {
        if entry.class != MemoryClass::Episodic {
            return Err(MemoryError::WrongClass {
                expected: "Episodic".into(),
                actual: entry.class.to_string(),
            });
        }

        if !entry.has_valid_provenance() {
            return Err(MemoryError::MissingProvenance);
        }

        let id = entry.id.clone();
        let anchor = entry.created;

        self.episodic.entries.entry(anchor).or_default().push(entry);

        debug!(id = %id, "Stored episodic entry");
        Ok(id)
    }

    /// Store a semantic entry (consolidated knowledge).
    pub fn store_semantic(
        &mut self,
        key: String,
        entry: MemoryEntry,
    ) -> Result<MemoryId, MemoryError> {
        if entry.class != MemoryClass::Semantic {
            return Err(MemoryError::WrongClass {
                expected: "Semantic".into(),
                actual: entry.class.to_string(),
            });
        }

        if !entry.has_valid_provenance() {
            return Err(MemoryError::MissingProvenance);
        }

        let id = entry.id.clone();
        self.semantic.entries.insert(key.clone(), entry);
        debug!(id = %id, key = %key, "Stored semantic entry");
        Ok(id)
    }

    /// Query episodic entries by time range (inclusive).
    pub fn query_temporal(&self, from: &TemporalAnchor, to: &TemporalAnchor) -> Vec<&MemoryEntry> {
        self.episodic
            .entries
            .range(*from..=*to)
            .flat_map(|(_, entries)| entries.iter())
            .collect()
    }

    /// Query episodic entries by worldline.
    pub fn query_by_worldline(&self, wid: &WorldlineId) -> Vec<&MemoryEntry> {
        self.episodic
            .entries
            .values()
            .flat_map(|entries| entries.iter())
            .filter(|e| e.worldline_id == *wid)
            .collect()
    }

    /// Look up semantic knowledge by key.
    pub fn lookup_semantic(&self, key: &str) -> Option<&MemoryEntry> {
        self.semantic.entries.get(key)
    }

    /// Get all entries for a specific provenance chain.
    pub fn query_by_provenance(&self, provenance: &ProvenanceRef) -> Vec<&MemoryEntry> {
        let mut results: Vec<&MemoryEntry> = self
            .episodic
            .entries
            .values()
            .flat_map(|entries| entries.iter())
            .filter(|e| e.provenance == *provenance)
            .collect();

        // Also check semantic store
        results.extend(
            self.semantic
                .entries
                .values()
                .filter(|e| e.provenance == *provenance),
        );

        results
    }

    /// Query all episodic plane entries matching a filter.
    pub fn query(&self, filter: &MemoryFilter) -> Vec<&MemoryEntry> {
        let mut results: Vec<&MemoryEntry> = self
            .episodic
            .entries
            .values()
            .flat_map(|entries| entries.iter())
            .filter(|e| filter.matches(e))
            .collect();

        results.extend(self.semantic.entries.values().filter(|e| filter.matches(e)));

        results
    }

    /// Get counts for statistics.
    pub fn episodic_count(&self) -> usize {
        self.episodic.entries.values().map(|v| v.len()).sum()
    }

    pub fn semantic_count(&self) -> usize {
        self.semantic.entries.len()
    }

    /// Get all episodic entries (for rebuild scenarios).
    pub fn all_episodic_entries(&self) -> Vec<&MemoryEntry> {
        self.episodic
            .entries
            .values()
            .flat_map(|entries| entries.iter())
            .collect()
    }
}

impl Default for EpisodicPlane {
    fn default() -> Self {
        Self::new()
    }
}

/// Episodic Store — temporal sequences with causal links.
struct EpisodicStore {
    entries: BTreeMap<TemporalAnchor, Vec<MemoryEntry>>,
}

/// Semantic Store — consolidated knowledge extracted from episodic patterns.
struct SemanticStore {
    entries: HashMap<String, MemoryEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{nil_provenance, provenance_from, MemoryContent};
    use maple_mwl_types::{EventId, IdentityMaterial};

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn other_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32]))
    }

    fn episodic_entry_at(text: &str, time_ms: u64) -> MemoryEntry {
        let mut entry = MemoryEntry::builder(
            MemoryClass::Episodic,
            MemoryContent::Text(text.into()),
            provenance_from(EventId::new()),
            test_worldline(),
        )
        .build();
        entry.created = TemporalAnchor::new(time_ms, 0, 0);
        entry
    }

    fn episodic_entry_with_wid(text: &str, wid: WorldlineId) -> MemoryEntry {
        MemoryEntry::builder(
            MemoryClass::Episodic,
            MemoryContent::Text(text.into()),
            provenance_from(EventId::new()),
            wid,
        )
        .build()
    }

    fn semantic_entry(text: &str) -> MemoryEntry {
        MemoryEntry::builder(
            MemoryClass::Semantic,
            MemoryContent::Text(text.into()),
            provenance_from(EventId::new()),
            test_worldline(),
        )
        .build()
    }

    #[test]
    fn store_and_query_episodic_by_time_range() {
        let mut ep = EpisodicPlane::new();

        ep.store_episodic(episodic_entry_at("t100", 100)).unwrap();
        ep.store_episodic(episodic_entry_at("t200", 200)).unwrap();
        ep.store_episodic(episodic_entry_at("t300", 300)).unwrap();
        ep.store_episodic(episodic_entry_at("t400", 400)).unwrap();

        let from = TemporalAnchor::new(150, 0, 0);
        let to = TemporalAnchor::new(350, 0, 0);
        let results = ep.query_temporal(&from, &to);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn store_and_query_semantic() {
        let mut ep = EpisodicPlane::new();

        ep.store_semantic("concept_a".into(), semantic_entry("A is fundamental"))
            .unwrap();
        ep.store_semantic("concept_b".into(), semantic_entry("B follows from A"))
            .unwrap();

        assert!(ep.lookup_semantic("concept_a").is_some());
        assert!(ep.lookup_semantic("concept_b").is_some());
        assert!(ep.lookup_semantic("concept_c").is_none());
    }

    #[test]
    fn query_by_worldline() {
        let mut ep = EpisodicPlane::new();

        let wid1 = test_worldline();
        let wid2 = other_worldline();

        ep.store_episodic(episodic_entry_with_wid("w1-a", wid1.clone()))
            .unwrap();
        ep.store_episodic(episodic_entry_with_wid("w1-b", wid1.clone()))
            .unwrap();
        ep.store_episodic(episodic_entry_with_wid("w2-a", wid2.clone()))
            .unwrap();

        let w1_results = ep.query_by_worldline(&wid1);
        assert_eq!(w1_results.len(), 2);

        let w2_results = ep.query_by_worldline(&wid2);
        assert_eq!(w2_results.len(), 1);
    }

    #[test]
    fn query_by_provenance() {
        let mut ep = EpisodicPlane::new();

        let event_id = EventId::new();
        let prov = provenance_from(event_id.clone());

        let mut entry1 = episodic_entry_at("linked-1", 100);
        entry1.provenance = prov.clone();
        ep.store_episodic(entry1).unwrap();

        let mut entry2 = episodic_entry_at("linked-2", 200);
        entry2.provenance = prov.clone();
        ep.store_episodic(entry2).unwrap();

        // Unrelated entry
        ep.store_episodic(episodic_entry_at("unrelated", 300))
            .unwrap();

        let results = ep.query_by_provenance(&prov);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn reject_episodic_without_provenance() {
        let mut ep = EpisodicPlane::new();

        let mut entry = episodic_entry_at("no-prov", 100);
        entry.provenance = nil_provenance();

        assert!(ep.store_episodic(entry).is_err());
    }

    #[test]
    fn reject_semantic_without_provenance() {
        let mut ep = EpisodicPlane::new();

        let mut entry = semantic_entry("no-prov");
        entry.provenance = nil_provenance();

        assert!(ep.store_semantic("key".into(), entry).is_err());
    }

    #[test]
    fn reject_wrong_class_episodic() {
        let mut ep = EpisodicPlane::new();

        let entry = MemoryEntry::builder(
            MemoryClass::Active,
            MemoryContent::Text("wrong".into()),
            provenance_from(EventId::new()),
            test_worldline(),
        )
        .build();

        assert!(ep.store_episodic(entry).is_err());
    }

    #[test]
    fn reject_wrong_class_semantic() {
        let mut ep = EpisodicPlane::new();

        let entry = MemoryEntry::builder(
            MemoryClass::Active,
            MemoryContent::Text("wrong".into()),
            provenance_from(EventId::new()),
            test_worldline(),
        )
        .build();

        assert!(ep.store_semantic("key".into(), entry).is_err());
    }

    #[test]
    fn episodic_count() {
        let mut ep = EpisodicPlane::new();
        ep.store_episodic(episodic_entry_at("a", 100)).unwrap();
        ep.store_episodic(episodic_entry_at("b", 200)).unwrap();
        assert_eq!(ep.episodic_count(), 2);
    }

    #[test]
    fn semantic_count() {
        let mut ep = EpisodicPlane::new();
        ep.store_semantic("k1".into(), semantic_entry("v1"))
            .unwrap();
        ep.store_semantic("k2".into(), semantic_entry("v2"))
            .unwrap();
        assert_eq!(ep.semantic_count(), 2);
    }

    #[test]
    fn query_with_filter() {
        let mut ep = EpisodicPlane::new();

        ep.store_episodic(episodic_entry_at("important data", 100))
            .unwrap();
        ep.store_episodic(episodic_entry_at("trivial", 200))
            .unwrap();

        let filter = MemoryFilter::new().with_content_contains("important");
        let results = ep.query(&filter);
        assert_eq!(results.len(), 1);
    }
}
