use std::collections::HashMap;

use maple_mwl_types::{EventId, ProvenanceRef, TemporalAnchor, WorldlineId};
use serde::{Deserialize, Serialize};

/// Unique identifier for a memory entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub uuid::Uuid);

impl MemoryId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for MemoryId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MemoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mem:{}", self.0)
    }
}

/// Every memory entry is provenance-bound.
/// Memory without provenance is architecturally invalid.
///
/// Per I.2 (Intrinsic Typed Memory): Two-plane with provenance binding.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: MemoryId,
    pub class: MemoryClass,
    pub content: MemoryContent,
    /// Links to EventId in Event Fabric — REQUIRED.
    /// Memory without provenance is invalid.
    pub provenance: ProvenanceRef,
    pub worldline_id: WorldlineId,
    pub created: TemporalAnchor,
    /// Confidence in this memory (0.0 - 1.0)
    pub confidence: f64,
    pub metadata: HashMap<String, String>,
}

impl MemoryEntry {
    /// Check if this entry has valid provenance (non-nil EventId).
    pub fn has_valid_provenance(&self) -> bool {
        !self.provenance.0 .0.is_nil()
    }

    /// Create a builder for ergonomic entry construction.
    pub fn builder(
        class: MemoryClass,
        content: MemoryContent,
        provenance: ProvenanceRef,
        worldline_id: WorldlineId,
    ) -> MemoryEntryBuilder {
        MemoryEntryBuilder {
            class,
            content,
            provenance,
            worldline_id,
            confidence: 1.0,
            metadata: HashMap::new(),
        }
    }
}

/// Builder for constructing MemoryEntry instances.
pub struct MemoryEntryBuilder {
    class: MemoryClass,
    content: MemoryContent,
    provenance: ProvenanceRef,
    worldline_id: WorldlineId,
    confidence: f64,
    metadata: HashMap<String, String>,
}

impl MemoryEntryBuilder {
    pub fn confidence(mut self, c: f64) -> Self {
        self.confidence = c.clamp(0.0, 1.0);
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> MemoryEntry {
        MemoryEntry {
            id: MemoryId::new(),
            class: self.class,
            content: self.content,
            provenance: self.provenance,
            worldline_id: self.worldline_id,
            created: TemporalAnchor::now(0),
            confidence: self.confidence,
            metadata: self.metadata,
        }
    }
}

/// Classification of a memory entry, determining which plane it lives in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryClass {
    /// Raw input, high bandwidth, short retention (Working Plane)
    Sensory,
    /// Current reasoning context (Working Plane)
    Active,
    /// Temporal sequences with causal links (Episodic Plane)
    Episodic,
    /// Consolidated knowledge (Episodic Plane)
    Semantic,
}

impl MemoryClass {
    /// Which memory plane this class belongs to.
    pub fn plane(&self) -> MemoryPlane {
        match self {
            Self::Sensory | Self::Active => MemoryPlane::Working,
            Self::Episodic | Self::Semantic => MemoryPlane::Episodic,
        }
    }
}

impl std::fmt::Display for MemoryClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sensory => write!(f, "Sensory"),
            Self::Active => write!(f, "Active"),
            Self::Episodic => write!(f, "Episodic"),
            Self::Semantic => write!(f, "Semantic"),
        }
    }
}

/// The two memory planes in the architecture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryPlane {
    /// Volatile — current reasoning context, rebuildable
    Working,
    /// Persistent — committed history with provenance
    Episodic,
}

/// Content stored in a memory entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MemoryContent {
    Text(String),
    Structured(serde_json::Value),
    Binary(Vec<u8>),
    Reference(MemoryId),
}

/// Create a provenance ref that is explicitly nil (for testing rejection).
pub fn nil_provenance() -> ProvenanceRef {
    ProvenanceRef(EventId(uuid::Uuid::nil()))
}

/// Create a valid provenance ref from an EventId.
pub fn provenance_from(event_id: EventId) -> ProvenanceRef {
    ProvenanceRef(event_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_mwl_types::IdentityMaterial;

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn valid_provenance() -> ProvenanceRef {
        provenance_from(EventId::new())
    }

    #[test]
    fn memory_class_plane_routing() {
        assert_eq!(MemoryClass::Sensory.plane(), MemoryPlane::Working);
        assert_eq!(MemoryClass::Active.plane(), MemoryPlane::Working);
        assert_eq!(MemoryClass::Episodic.plane(), MemoryPlane::Episodic);
        assert_eq!(MemoryClass::Semantic.plane(), MemoryPlane::Episodic);
    }

    #[test]
    fn valid_provenance_detected() {
        let entry = MemoryEntry::builder(
            MemoryClass::Active,
            MemoryContent::Text("test".into()),
            valid_provenance(),
            test_worldline(),
        )
        .build();
        assert!(entry.has_valid_provenance());
    }

    #[test]
    fn nil_provenance_detected() {
        let entry = MemoryEntry::builder(
            MemoryClass::Active,
            MemoryContent::Text("test".into()),
            nil_provenance(),
            test_worldline(),
        )
        .build();
        assert!(!entry.has_valid_provenance());
    }

    #[test]
    fn builder_sets_confidence() {
        let entry = MemoryEntry::builder(
            MemoryClass::Sensory,
            MemoryContent::Text("test".into()),
            valid_provenance(),
            test_worldline(),
        )
        .confidence(0.75)
        .build();
        assert!((entry.confidence - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn builder_clamps_confidence() {
        let entry = MemoryEntry::builder(
            MemoryClass::Sensory,
            MemoryContent::Text("test".into()),
            valid_provenance(),
            test_worldline(),
        )
        .confidence(1.5)
        .build();
        assert!((entry.confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn builder_sets_metadata() {
        let entry = MemoryEntry::builder(
            MemoryClass::Active,
            MemoryContent::Text("test".into()),
            valid_provenance(),
            test_worldline(),
        )
        .metadata("source", "test")
        .build();
        assert_eq!(entry.metadata.get("source").unwrap(), "test");
    }

    #[test]
    fn memory_content_variants_serialize() {
        let variants = vec![
            MemoryContent::Text("hello".into()),
            MemoryContent::Structured(serde_json::json!({"key": "value"})),
            MemoryContent::Binary(vec![1, 2, 3]),
            MemoryContent::Reference(MemoryId::new()),
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let _: MemoryContent = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn entry_serialization_roundtrip() {
        let entry = MemoryEntry::builder(
            MemoryClass::Episodic,
            MemoryContent::Text("important".into()),
            valid_provenance(),
            test_worldline(),
        )
        .confidence(0.9)
        .metadata("type", "observation")
        .build();

        let json = serde_json::to_string(&entry).unwrap();
        let restored: MemoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.id, restored.id);
        assert_eq!(entry.class, restored.class);
        assert!(restored.has_valid_provenance());
    }
}
