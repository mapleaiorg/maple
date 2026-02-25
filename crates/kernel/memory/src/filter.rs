use maple_mwl_types::{TemporalAnchor, WorldlineId};

use crate::entry::{MemoryClass, MemoryEntry};

/// Memory query filter — used across both planes.
#[derive(Clone, Debug, Default)]
pub struct MemoryFilter {
    pub class: Option<MemoryClass>,
    pub worldline_id: Option<WorldlineId>,
    pub time_range: Option<(TemporalAnchor, TemporalAnchor)>,
    pub min_confidence: Option<f64>,
    pub content_contains: Option<String>,
}

impl MemoryFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_class(mut self, class: MemoryClass) -> Self {
        self.class = Some(class);
        self
    }

    pub fn with_worldline(mut self, wid: WorldlineId) -> Self {
        self.worldline_id = Some(wid);
        self
    }

    pub fn with_time_range(mut self, from: TemporalAnchor, to: TemporalAnchor) -> Self {
        self.time_range = Some((from, to));
        self
    }

    pub fn with_min_confidence(mut self, min: f64) -> Self {
        self.min_confidence = Some(min);
        self
    }

    pub fn with_content_contains(mut self, text: impl Into<String>) -> Self {
        self.content_contains = Some(text.into());
        self
    }

    /// Check if a memory entry matches this filter.
    pub fn matches(&self, entry: &MemoryEntry) -> bool {
        if let Some(ref class) = self.class {
            if entry.class != *class {
                return false;
            }
        }

        if let Some(ref wid) = self.worldline_id {
            if entry.worldline_id != *wid {
                return false;
            }
        }

        if let Some((ref from, ref to)) = self.time_range {
            if entry.created < *from || entry.created > *to {
                return false;
            }
        }

        if let Some(min) = self.min_confidence {
            if entry.confidence < min {
                return false;
            }
        }

        if let Some(ref text) = self.content_contains {
            match &entry.content {
                crate::entry::MemoryContent::Text(s) => {
                    if !s.contains(text.as_str()) {
                        return false;
                    }
                }
                _ => return false,
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::*;
    use maple_mwl_types::IdentityMaterial;

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn test_entry(class: MemoryClass, text: &str, confidence: f64) -> MemoryEntry {
        MemoryEntry::builder(
            class,
            MemoryContent::Text(text.into()),
            provenance_from(maple_mwl_types::EventId::new()),
            test_worldline(),
        )
        .confidence(confidence)
        .build()
    }

    #[test]
    fn filter_by_class() {
        let entry = test_entry(MemoryClass::Sensory, "hello", 1.0);
        let filter = MemoryFilter::new().with_class(MemoryClass::Sensory);
        assert!(filter.matches(&entry));

        let filter = MemoryFilter::new().with_class(MemoryClass::Active);
        assert!(!filter.matches(&entry));
    }

    #[test]
    fn filter_by_confidence() {
        let entry = test_entry(MemoryClass::Active, "data", 0.7);
        let filter = MemoryFilter::new().with_min_confidence(0.5);
        assert!(filter.matches(&entry));

        let filter = MemoryFilter::new().with_min_confidence(0.9);
        assert!(!filter.matches(&entry));
    }

    #[test]
    fn filter_by_content() {
        let entry = test_entry(MemoryClass::Active, "hello world", 1.0);
        let filter = MemoryFilter::new().with_content_contains("world");
        assert!(filter.matches(&entry));

        let filter = MemoryFilter::new().with_content_contains("xyz");
        assert!(!filter.matches(&entry));
    }

    #[test]
    fn filter_by_worldline() {
        let wid = test_worldline();
        let entry = test_entry(MemoryClass::Active, "data", 1.0);
        let filter = MemoryFilter::new().with_worldline(wid);
        assert!(filter.matches(&entry));

        let other_wid = WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32]));
        let filter = MemoryFilter::new().with_worldline(other_wid);
        assert!(!filter.matches(&entry));
    }

    #[test]
    fn empty_filter_matches_all() {
        let entry = test_entry(MemoryClass::Episodic, "anything", 0.3);
        let filter = MemoryFilter::new();
        assert!(filter.matches(&entry));
    }

    #[test]
    fn combined_filters() {
        let entry = test_entry(MemoryClass::Active, "important data", 0.8);
        let filter = MemoryFilter::new()
            .with_class(MemoryClass::Active)
            .with_min_confidence(0.5)
            .with_content_contains("important");
        assert!(filter.matches(&entry));
    }
}
