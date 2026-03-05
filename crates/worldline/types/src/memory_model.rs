//! Four-plane memory model types.
//!
//! Defines the memory classification system used by the kernel memory engine.
//! Each worldline maintains memories across four planes: Working, Episodic,
//! Semantic, and Parametric.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::WorldlineId;

/// The four canonical memory planes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryClass {
    /// Short-term, per-task working memory.
    Working,
    /// Autobiographical event records.
    Episodic,
    /// Factual knowledge and learned associations.
    Semantic,
    /// Model weights and embedding vectors.
    Parametric,
}

impl std::fmt::Display for MemoryClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Working => write!(f, "Working"),
            Self::Episodic => write!(f, "Episodic"),
            Self::Semantic => write!(f, "Semantic"),
            Self::Parametric => write!(f, "Parametric"),
        }
    }
}

/// A record stored in one of the four memory planes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    /// Unique record identifier.
    pub id: uuid::Uuid,
    /// Owning worldline.
    pub worldline_id: WorldlineId,
    /// Which memory plane this record belongs to.
    pub class: MemoryClass,
    /// Record content.
    pub content: MemoryContent,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
    /// When this record was last accessed.
    pub accessed_at: DateTime<Utc>,
    /// Decay weight: 0.0 = fully decayed, 1.0 = fresh.
    pub decay_weight: f64,
}

/// Content variants for memory records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryContent {
    /// Plain text content.
    Text(String),
    /// Embedding vector.
    Embedding(Vec<f32>),
    /// Structured JSON data.
    Structured(serde_json::Value),
    /// Raw binary data.
    Binary(Vec<u8>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_class_display() {
        assert_eq!(MemoryClass::Working.to_string(), "Working");
        assert_eq!(MemoryClass::Episodic.to_string(), "Episodic");
        assert_eq!(MemoryClass::Semantic.to_string(), "Semantic");
        assert_eq!(MemoryClass::Parametric.to_string(), "Parametric");
    }

    #[test]
    fn memory_class_serde_roundtrip() {
        for class in [
            MemoryClass::Working,
            MemoryClass::Episodic,
            MemoryClass::Semantic,
            MemoryClass::Parametric,
        ] {
            let json = serde_json::to_string(&class).unwrap();
            let back: MemoryClass = serde_json::from_str(&json).unwrap();
            assert_eq!(class, back);
        }
    }

    #[test]
    fn memory_content_text_serde() {
        let content = MemoryContent::Text("hello world".into());
        let json = serde_json::to_string(&content).unwrap();
        let back: MemoryContent = serde_json::from_str(&json).unwrap();
        match back {
            MemoryContent::Text(s) => assert_eq!(s, "hello world"),
            _ => panic!("expected Text variant"),
        }
    }

    #[test]
    fn memory_content_embedding_serde() {
        let content = MemoryContent::Embedding(vec![0.1, 0.2, 0.3]);
        let json = serde_json::to_string(&content).unwrap();
        let back: MemoryContent = serde_json::from_str(&json).unwrap();
        match back {
            MemoryContent::Embedding(v) => assert_eq!(v.len(), 3),
            _ => panic!("expected Embedding variant"),
        }
    }

    #[test]
    fn memory_content_structured_serde() {
        let content = MemoryContent::Structured(serde_json::json!({"key": "value"}));
        let json = serde_json::to_string(&content).unwrap();
        let _: MemoryContent = serde_json::from_str(&json).unwrap();
    }
}
