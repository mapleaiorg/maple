//! Core Collective Identity Types
//!
//! A Collective Resonator is NOT a super-agent that reasons.
//! It IS a commitment coordination mechanism, policy enforcement boundary,
//! resource allocation unit, and audit surface.

use chrono::{DateTime, Utc};
use rcf_types::ContinuityRef;
use resonator_types::ResonatorId;
use serde::{Deserialize, Serialize};

/// Unique identifier for a Collective Resonator
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CollectiveId(pub String);

impl CollectiveId {
    /// Generate a new random CollectiveId
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Create a CollectiveId from a known string
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Short display form (first 8 chars)
    pub fn short(&self) -> String {
        self.0.chars().take(8).collect()
    }
}

impl std::fmt::Display for CollectiveId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Specification for creating a new Collective
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollectiveSpec {
    /// Human-readable name
    pub name: String,
    /// Purpose and description
    pub description: String,
    /// Optional institution profile name (e.g., "Firm", "DAO", "State")
    /// Not hardcoded—just a reference to a configurable profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub institution_profile: Option<String>,
    /// The resonator that created this collective
    pub created_by: ResonatorId,
}

impl CollectiveSpec {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        created_by: ResonatorId,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            institution_profile: None,
            created_by,
        }
    }

    pub fn with_institution_profile(mut self, profile: impl Into<String>) -> Self {
        self.institution_profile = Some(profile.into());
        self
    }
}

/// Status of a Collective
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CollectiveStatus {
    /// Collective is operational
    #[default]
    Active,
    /// Collective is temporarily suspended
    Suspended,
    /// Collective has been permanently dissolved
    Dissolved,
}

impl CollectiveStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, CollectiveStatus::Active)
    }

    pub fn is_operational(&self) -> bool {
        matches!(self, CollectiveStatus::Active)
    }
}

/// Full metadata for a Collective Resonator
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollectiveMetadata {
    /// Unique collective identity
    pub id: CollectiveId,
    /// Creation specification
    pub spec: CollectiveSpec,
    /// Current operational status
    pub status: CollectiveStatus,
    /// Identity continuity chain (same as individual resonators)
    pub continuity_chain: ContinuityRef,
    /// When the collective was formed
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
}

impl CollectiveMetadata {
    /// Create new collective metadata from a spec
    pub fn new(spec: CollectiveSpec) -> Self {
        let now = Utc::now();
        Self {
            id: CollectiveId::generate(),
            spec,
            status: CollectiveStatus::Active,
            continuity_chain: ContinuityRef::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create with a specific ID (for testing or migration)
    pub fn with_id(mut self, id: CollectiveId) -> Self {
        self.id = id;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collective_id_generate() {
        let id = CollectiveId::generate();
        assert!(!id.0.is_empty());
        assert_eq!(id.short().len(), 8);
    }

    #[test]
    fn test_collective_id_display() {
        let id = CollectiveId::new("test-collective-123");
        assert_eq!(format!("{}", id), "test-collective-123");
    }

    #[test]
    fn test_collective_spec() {
        let creator = ResonatorId::new("creator-1");
        let spec = CollectiveSpec::new("Acme Corp", "A test collective", creator.clone())
            .with_institution_profile("Firm");

        assert_eq!(spec.name, "Acme Corp");
        assert_eq!(spec.institution_profile.as_deref(), Some("Firm"));
        assert_eq!(spec.created_by, creator);
    }

    #[test]
    fn test_collective_metadata() {
        let creator = ResonatorId::new("creator-1");
        let spec = CollectiveSpec::new("Test", "Test collective", creator);
        let meta = CollectiveMetadata::new(spec);

        assert!(meta.status.is_active());
        assert!(meta.status.is_operational());
        assert!(meta.created_at <= Utc::now());
    }

    #[test]
    fn test_collective_status() {
        assert!(CollectiveStatus::Active.is_active());
        assert!(!CollectiveStatus::Suspended.is_active());
        assert!(!CollectiveStatus::Dissolved.is_active());
    }
}
