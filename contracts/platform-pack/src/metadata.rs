//! Platform pack metadata

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata for a platform pack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformMetadata {
    /// Platform name (unique identifier)
    pub name: String,

    /// Display name for UI
    pub display_name: String,

    /// Platform version (semver)
    pub version: String,

    /// Description
    pub description: String,

    /// Author/owner
    pub author: String,

    /// License
    pub license: String,

    /// Homepage URL
    pub homepage: Option<String>,

    /// Repository URL
    pub repository: Option<String>,

    /// Documentation URL
    pub documentation: Option<String>,

    /// Keywords for discovery
    #[serde(default)]
    pub keywords: Vec<String>,

    /// Compatibility information
    pub compatibility: CompatibilityInfo,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,

    /// Additional metadata
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Compatibility information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityInfo {
    /// Minimum PALM version required
    pub min_palm_version: String,

    /// Maximum PALM version supported (None = no limit)
    pub max_palm_version: Option<String>,

    /// Minimum resonance architecture version
    pub min_resonance_version: String,

    /// Required features
    #[serde(default)]
    pub required_features: Vec<String>,

    /// Optional features (enhanced functionality)
    #[serde(default)]
    pub optional_features: Vec<String>,

    /// Breaking changes from previous versions
    #[serde(default)]
    pub breaking_changes: Vec<BreakingChange>,
}

/// Breaking change record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakingChange {
    /// Version that introduced the breaking change
    pub version: String,

    /// Description of the breaking change
    pub description: String,

    /// Migration instructions
    pub migration: Option<String>,
}

impl Default for PlatformMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            name: "unknown".to_string(),
            display_name: "Unknown Platform".to_string(),
            version: "0.1.0".to_string(),
            description: "No description".to_string(),
            author: "Unknown".to_string(),
            license: "UNLICENSED".to_string(),
            homepage: None,
            repository: None,
            documentation: None,
            keywords: vec![],
            compatibility: CompatibilityInfo::default(),
            created_at: now,
            updated_at: now,
            extra: HashMap::new(),
        }
    }
}

impl Default for CompatibilityInfo {
    fn default() -> Self {
        Self {
            min_palm_version: "0.1.0".to_string(),
            max_palm_version: None,
            min_resonance_version: "0.1.0".to_string(),
            required_features: vec![],
            optional_features: vec![],
            breaking_changes: vec![],
        }
    }
}

impl PlatformMetadata {
    /// Create metadata for Mapleverse platform
    pub fn mapleverse() -> Self {
        let now = Utc::now();
        Self {
            name: "mapleverse".to_string(),
            display_name: "Mapleverse".to_string(),
            version: "0.1.0".to_string(),
            description: "High-throughput AI agent platform for MapleAI Intelligence".to_string(),
            author: "MapleAI".to_string(),
            license: "Proprietary".to_string(),
            homepage: Some("https://mapleai.io".to_string()),
            repository: Some("https://github.com/mapleai/mapleverse".to_string()),
            documentation: Some("https://docs.mapleai.io/mapleverse".to_string()),
            keywords: vec![
                "ai".to_string(),
                "agents".to_string(),
                "high-throughput".to_string(),
                "scalable".to_string(),
            ],
            compatibility: CompatibilityInfo {
                min_palm_version: "0.1.0".to_string(),
                max_palm_version: None,
                min_resonance_version: "0.1.0".to_string(),
                required_features: vec!["live_migration".to_string(), "hot_reload".to_string()],
                optional_features: vec!["gpu_acceleration".to_string()],
                breaking_changes: vec![],
            },
            created_at: now,
            updated_at: now,
            extra: HashMap::new(),
        }
    }

    /// Create metadata for Finalverse platform
    pub fn finalverse() -> Self {
        let now = Utc::now();
        Self {
            name: "finalverse".to_string(),
            display_name: "Finalverse".to_string(),
            version: "0.1.0".to_string(),
            description: "Safety-first AI agent platform with human oversight".to_string(),
            author: "Finalverse Team".to_string(),
            license: "Proprietary".to_string(),
            homepage: Some("https://finalverse.ai".to_string()),
            repository: Some("https://github.com/finalverse/finalverse".to_string()),
            documentation: Some("https://docs.finalverse.ai".to_string()),
            keywords: vec![
                "ai".to_string(),
                "agents".to_string(),
                "safety".to_string(),
                "oversight".to_string(),
            ],
            compatibility: CompatibilityInfo {
                min_palm_version: "0.1.0".to_string(),
                max_palm_version: None,
                min_resonance_version: "0.1.0".to_string(),
                required_features: vec![
                    "human_approval".to_string(),
                    "checkpoints".to_string(),
                    "audit_logging".to_string(),
                ],
                optional_features: vec!["canary_deployments".to_string()],
                breaking_changes: vec![],
            },
            created_at: now,
            updated_at: now,
            extra: HashMap::new(),
        }
    }

    /// Create metadata for iBank platform
    pub fn ibank() -> Self {
        let now = Utc::now();
        Self {
            name: "ibank".to_string(),
            display_name: "iBank".to_string(),
            version: "0.1.0".to_string(),
            description: "Accountability-focused AI agent platform for financial services"
                .to_string(),
            author: "iBank Team".to_string(),
            license: "Proprietary".to_string(),
            homepage: Some("https://ibank.ai".to_string()),
            repository: Some("https://github.com/ibank/ibank".to_string()),
            documentation: Some("https://docs.ibank.ai".to_string()),
            keywords: vec![
                "ai".to_string(),
                "agents".to_string(),
                "financial".to_string(),
                "compliance".to_string(),
                "accountability".to_string(),
            ],
            compatibility: CompatibilityInfo {
                min_palm_version: "0.1.0".to_string(),
                max_palm_version: None,
                min_resonance_version: "0.1.0".to_string(),
                required_features: vec![
                    "human_approval".to_string(),
                    "checkpoints".to_string(),
                    "audit_logging".to_string(),
                    "commitment_accounting".to_string(),
                    "integrity_chain".to_string(),
                ],
                optional_features: vec![],
                breaking_changes: vec![],
            },
            created_at: now,
            updated_at: now,
            extra: [("compliance_level".to_string(), serde_json::json!("PCI-DSS"))]
                .into_iter()
                .collect(),
        }
    }

    /// Check if this pack is compatible with a PALM version
    pub fn is_compatible_with_palm(&self, palm_version: &str) -> bool {
        use semver::Version;

        let palm_ver = match Version::parse(palm_version) {
            Ok(v) => v,
            Err(_) => return false,
        };

        let min_ver = match Version::parse(&self.compatibility.min_palm_version) {
            Ok(v) => v,
            Err(_) => return false,
        };

        if palm_ver < min_ver {
            return false;
        }

        if let Some(ref max_version) = self.compatibility.max_palm_version {
            let max_ver = match Version::parse(max_version) {
                Ok(v) => v,
                Err(_) => return true, // Ignore invalid max version
            };
            if palm_ver > max_ver {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_metadata() {
        let meta = PlatformMetadata::default();
        assert_eq!(meta.name, "unknown");
    }

    #[test]
    fn test_mapleverse_metadata() {
        let meta = PlatformMetadata::mapleverse();
        assert_eq!(meta.name, "mapleverse");
        assert!(meta
            .compatibility
            .required_features
            .contains(&"live_migration".to_string()));
    }

    #[test]
    fn test_finalverse_metadata() {
        let meta = PlatformMetadata::finalverse();
        assert_eq!(meta.name, "finalverse");
        assert!(meta
            .compatibility
            .required_features
            .contains(&"human_approval".to_string()));
    }

    #[test]
    fn test_ibank_metadata() {
        let meta = PlatformMetadata::ibank();
        assert_eq!(meta.name, "ibank");
        assert!(meta
            .compatibility
            .required_features
            .contains(&"commitment_accounting".to_string()));
        assert!(meta.extra.contains_key("compliance_level"));
    }

    #[test]
    fn test_compatibility_check() {
        let meta = PlatformMetadata {
            compatibility: CompatibilityInfo {
                min_palm_version: "0.2.0".to_string(),
                max_palm_version: Some("1.0.0".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        assert!(!meta.is_compatible_with_palm("0.1.0")); // Too old
        assert!(meta.is_compatible_with_palm("0.2.0")); // Min version
        assert!(meta.is_compatible_with_palm("0.5.0")); // In range
        assert!(meta.is_compatible_with_palm("1.0.0")); // Max version
        assert!(!meta.is_compatible_with_palm("1.1.0")); // Too new
    }
}
