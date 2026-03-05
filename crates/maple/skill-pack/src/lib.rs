//! Maple Skill Pack — the universal packaging format for tools, skills, and operators.
//!
//! A Skill Pack is a directory containing:
//! - `manifest.toml` — name, version, I/O schemas, required capabilities, resource limits
//! - `policy.toml` — deny-first policies specific to this skill
//! - Optional `tests/golden/` — golden test traces for conformance testing
//!
//! Skill Packs provide the compatibility bridge between MAPLE and the wider AI ecosystem.
//! Native MAPLE skills, converted OpenAI tools, and converted Anthropic skills all
//! produce the same canonical Skill Pack format.

pub mod conformance;
pub mod converter_anthropic;
pub mod converter_openai;
pub mod error;
pub mod golden;
pub mod loader;
pub mod manifest;
pub mod policy;
pub mod registry;

pub use error::SkillError;
pub use golden::GoldenTrace;
pub use loader::SkillPackLoader;
pub use manifest::{
    IoField, ResourceLimits, SandboxConfig, SandboxType, SkillManifest, SkillMetadata,
};
pub use policy::{PolicyCondition, PolicyEffect, SkillPolicy};
pub use registry::{RegisteredSkill, SkillRegistry, SkillSource};

/// A fully loaded Skill Pack ready for registration and execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillPack {
    /// Skill manifest (from manifest.toml).
    pub manifest: SkillManifest,
    /// Skill-specific policies (from policy.toml).
    pub policies: Vec<SkillPolicy>,
    /// Golden test traces (from tests/golden/).
    pub golden_traces: Vec<GoldenTrace>,
    /// Source directory this pack was loaded from.
    pub source_path: Option<std::path::PathBuf>,
}

impl SkillPack {
    /// The skill's canonical name.
    pub fn name(&self) -> &str {
        &self.manifest.skill.name
    }

    /// The skill's version.
    pub fn version(&self) -> &semver::Version {
        &self.manifest.skill.version
    }

    /// Required capability IDs.
    pub fn required_capabilities(&self) -> &[String] {
        &self.manifest.capabilities.required
    }

    /// Resource limits for this skill.
    pub fn resource_limits(&self) -> &ResourceLimits {
        &self.manifest.resources
    }

    /// Validate the pack's internal consistency.
    pub fn validate(&self) -> Result<(), SkillError> {
        // Name must be non-empty
        if self.manifest.skill.name.is_empty() {
            return Err(SkillError::InvalidManifest(
                "skill name cannot be empty".into(),
            ));
        }

        // Must have at least one input
        if self.manifest.inputs.is_empty() {
            return Err(SkillError::InvalidManifest(
                "skill must define at least one input".into(),
            ));
        }

        // Must have at least one output
        if self.manifest.outputs.is_empty() {
            return Err(SkillError::InvalidManifest(
                "skill must define at least one output".into(),
            ));
        }

        // Resource limits must be positive
        if self.manifest.resources.max_compute_ms == 0 {
            return Err(SkillError::InvalidManifest(
                "max_compute_ms must be > 0".into(),
            ));
        }

        // Sandbox timeout must be positive
        if self.manifest.sandbox.timeout_ms == 0 {
            return Err(SkillError::InvalidManifest(
                "sandbox timeout_ms must be > 0".into(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> SkillManifest {
        SkillManifest {
            skill: SkillMetadata {
                name: "test-skill".into(),
                version: semver::Version::new(1, 0, 0),
                description: "A test skill".into(),
                author: Some("test".into()),
            },
            inputs: vec![("query".into(), IoField {
                field_type: "string".into(),
                required: true,
                default: None,
                description: "Search query".into(),
            })]
            .into_iter()
            .collect(),
            outputs: vec![("result".into(), IoField {
                field_type: "string".into(),
                required: true,
                default: None,
                description: "Result".into(),
            })]
            .into_iter()
            .collect(),
            capabilities: manifest::CapabilityRequirements {
                required: vec!["cap-test".into()],
            },
            resources: ResourceLimits {
                max_compute_ms: 5000,
                max_memory_bytes: 52_428_800,
                max_network_bytes: 10_485_760,
                max_storage_bytes: None,
                max_llm_tokens: None,
            },
            sandbox: SandboxConfig {
                sandbox_type: SandboxType::Process,
                timeout_ms: 10_000,
            },
            metadata: None,
        }
    }

    #[test]
    fn skill_pack_validate_success() {
        let pack = SkillPack {
            manifest: sample_manifest(),
            policies: vec![],
            golden_traces: vec![],
            source_path: None,
        };
        assert!(pack.validate().is_ok());
    }

    #[test]
    fn skill_pack_validate_empty_name() {
        let mut manifest = sample_manifest();
        manifest.skill.name = String::new();
        let pack = SkillPack {
            manifest,
            policies: vec![],
            golden_traces: vec![],
            source_path: None,
        };
        assert!(pack.validate().is_err());
    }

    #[test]
    fn skill_pack_validate_no_inputs() {
        let mut manifest = sample_manifest();
        manifest.inputs.clear();
        let pack = SkillPack {
            manifest,
            policies: vec![],
            golden_traces: vec![],
            source_path: None,
        };
        assert!(pack.validate().is_err());
    }

    #[test]
    fn skill_pack_accessors() {
        let pack = SkillPack {
            manifest: sample_manifest(),
            policies: vec![],
            golden_traces: vec![],
            source_path: None,
        };
        assert_eq!(pack.name(), "test-skill");
        assert_eq!(pack.version(), &semver::Version::new(1, 0, 0));
        assert_eq!(pack.required_capabilities(), &["cap-test"]);
        assert_eq!(pack.resource_limits().max_compute_ms, 5000);
    }
}
