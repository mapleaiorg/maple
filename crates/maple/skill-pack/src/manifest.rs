//! Skill manifest types, parsed from `manifest.toml`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Complete skill manifest (represents `manifest.toml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    /// Core skill metadata.
    pub skill: SkillMetadata,
    /// Input schema — field name → field definition.
    pub inputs: BTreeMap<String, IoField>,
    /// Output schema — field name → field definition.
    pub outputs: BTreeMap<String, IoField>,
    /// Required capabilities.
    pub capabilities: CapabilityRequirements,
    /// Resource limits.
    pub resources: ResourceLimits,
    /// Sandbox configuration.
    pub sandbox: SandboxConfig,
    /// Optional metadata (tags, license, etc.).
    pub metadata: Option<SkillMetadataExtra>,
}

/// Core skill identification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// Skill name (unique within a registry).
    pub name: String,
    /// Semantic version.
    pub version: semver::Version,
    /// Human-readable description.
    pub description: String,
    /// Author or organization.
    pub author: Option<String>,
}

/// A single I/O field definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoField {
    /// Field type: "string", "integer", "number", "boolean", "array", "object", "any".
    #[serde(rename = "type")]
    pub field_type: String,
    /// Whether this field is required.
    #[serde(default)]
    pub required: bool,
    /// Default value (JSON-encoded).
    pub default: Option<serde_json::Value>,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
}

/// Required capabilities for the skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRequirements {
    /// List of capability IDs that must be held by the executing worldline.
    #[serde(default)]
    pub required: Vec<String>,
}

/// Resource limits bounding the skill's execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum compute time in milliseconds.
    #[serde(default = "default_compute_ms")]
    pub max_compute_ms: u64,
    /// Maximum memory in bytes.
    #[serde(default = "default_memory_bytes")]
    pub max_memory_bytes: u64,
    /// Maximum network transfer in bytes.
    #[serde(default = "default_network_bytes")]
    pub max_network_bytes: u64,
    /// Maximum storage in bytes.
    pub max_storage_bytes: Option<u64>,
    /// Maximum LLM tokens.
    pub max_llm_tokens: Option<u64>,
}

fn default_compute_ms() -> u64 {
    10_000
}
fn default_memory_bytes() -> u64 {
    52_428_800
}
fn default_network_bytes() -> u64 {
    10_485_760
}

/// Sandbox configuration controlling isolation level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Sandbox type.
    #[serde(rename = "type")]
    pub sandbox_type: SandboxType,
    /// Execution timeout in milliseconds.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    15_000
}

/// Sandbox isolation level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SandboxType {
    /// No sandboxing — only for fully trusted operators.
    Trusted,
    /// Process-level isolation.
    Process,
    /// WASM sandbox (maximum isolation).
    Wasm,
}

/// Optional metadata (tags, license, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadataExtra {
    /// Searchable tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// License identifier.
    pub license: Option<String>,
}

impl SkillManifest {
    /// Parse a manifest from TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, crate::SkillError> {
        toml::from_str(toml_str).map_err(|e| {
            crate::SkillError::InvalidManifest(format!("TOML parse error: {e}"))
        })
    }

    /// Serialize to TOML string.
    pub fn to_toml(&self) -> Result<String, crate::SkillError> {
        toml::to_string_pretty(self).map_err(|e| {
            crate::SkillError::Serialization(format!("TOML serialize error: {e}"))
        })
    }

    /// Get all required input field names.
    pub fn required_inputs(&self) -> Vec<&str> {
        self.inputs
            .iter()
            .filter(|(_, f)| f.required)
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Validate an input JSON object against the manifest's input schema.
    pub fn validate_input(&self, input: &serde_json::Value) -> Result<(), crate::SkillError> {
        let obj = input.as_object().ok_or_else(|| {
            crate::SkillError::ValidationFailed("input must be a JSON object".into())
        })?;

        // Check required fields
        for (name, field) in &self.inputs {
            if field.required && !obj.contains_key(name) {
                return Err(crate::SkillError::ValidationFailed(format!(
                    "required input field '{}' is missing",
                    name
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TOML: &str = r#"
[skill]
name = "web-search"
version = "1.0.0"
description = "Search the web and return structured results"
author = "mapleai"

[inputs.query]
type = "string"
required = true
description = "Search query"

[inputs.max_results]
type = "integer"
required = false
default = 5
description = "Max results"

[outputs.results]
type = "array"
description = "Search results"

[outputs.total]
type = "integer"
description = "Total results available"

[capabilities]
required = ["cap-network-access", "cap-web-search"]

[resources]
max_compute_ms = 5000
max_memory_bytes = 52428800
max_network_bytes = 10485760

[sandbox]
type = "process"
timeout_ms = 10000

[metadata]
tags = ["search", "web", "information-retrieval"]
license = "MIT"
"#;

    #[test]
    fn parse_manifest_toml() {
        let manifest = SkillManifest::from_toml(SAMPLE_TOML).unwrap();
        assert_eq!(manifest.skill.name, "web-search");
        assert_eq!(manifest.skill.version, semver::Version::new(1, 0, 0));
        assert_eq!(manifest.skill.author.as_deref(), Some("mapleai"));
        assert_eq!(manifest.inputs.len(), 2);
        assert_eq!(manifest.outputs.len(), 2);
        assert!(manifest.inputs["query"].required);
        assert!(!manifest.inputs["max_results"].required);
        assert_eq!(manifest.capabilities.required.len(), 2);
        assert_eq!(manifest.resources.max_compute_ms, 5000);
        assert_eq!(manifest.sandbox.sandbox_type, SandboxType::Process);
        assert_eq!(manifest.sandbox.timeout_ms, 10_000);
        let meta = manifest.metadata.as_ref().unwrap();
        assert_eq!(meta.tags.len(), 3);
        assert_eq!(meta.license.as_deref(), Some("MIT"));
    }

    #[test]
    fn roundtrip_toml() {
        let manifest = SkillManifest::from_toml(SAMPLE_TOML).unwrap();
        let toml_out = manifest.to_toml().unwrap();
        let manifest2 = SkillManifest::from_toml(&toml_out).unwrap();
        assert_eq!(manifest.skill.name, manifest2.skill.name);
        assert_eq!(manifest.skill.version, manifest2.skill.version);
        assert_eq!(manifest.inputs.len(), manifest2.inputs.len());
    }

    #[test]
    fn required_inputs() {
        let manifest = SkillManifest::from_toml(SAMPLE_TOML).unwrap();
        let required = manifest.required_inputs();
        assert_eq!(required, vec!["query"]);
    }

    #[test]
    fn validate_input_success() {
        let manifest = SkillManifest::from_toml(SAMPLE_TOML).unwrap();
        let input = serde_json::json!({"query": "rust programming"});
        assert!(manifest.validate_input(&input).is_ok());
    }

    #[test]
    fn validate_input_missing_required() {
        let manifest = SkillManifest::from_toml(SAMPLE_TOML).unwrap();
        let input = serde_json::json!({"max_results": 10});
        assert!(manifest.validate_input(&input).is_err());
    }

    #[test]
    fn validate_input_not_object() {
        let manifest = SkillManifest::from_toml(SAMPLE_TOML).unwrap();
        let input = serde_json::json!("not an object");
        assert!(manifest.validate_input(&input).is_err());
    }

    #[test]
    fn sandbox_types_serde() {
        for (s, expected) in [
            ("\"trusted\"", SandboxType::Trusted),
            ("\"process\"", SandboxType::Process),
            ("\"wasm\"", SandboxType::Wasm),
        ] {
            let parsed: SandboxType = serde_json::from_str(s).unwrap();
            assert_eq!(parsed, expected);
        }
    }
}
