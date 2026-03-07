//! # Software Bill of Materials (SBOM) Generation
//!
//! Generates a MAPLE-native SBOM that captures the full dependency graph,
//! AI-specific metadata (model provenance, training data lineage), and
//! component relationships for audit and compliance.
//!
//! The SBOM format (`maple-sbom/v1`) extends traditional SBOMs with fields
//! specific to AI agent packages: model references, capability declarations,
//! and evaluation baselines.

use chrono::{DateTime, Utc};
use maple_build::BuildLockfile;
use maple_package::{MapleManifest, PackageKind};
use serde::{Deserialize, Serialize};

/// A complete MAPLE Software Bill of Materials.
///
/// Captures the package identity, all transitive dependencies (components),
/// their relationships, and AI-specific metadata such as model provenance
/// and evaluation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapleSbom {
    /// SBOM format version identifier.
    pub sbom_version: String,

    /// Timestamp when this SBOM was generated.
    pub created_at: DateTime<Utc>,

    /// Top-level package information.
    pub package: SbomPackageInfo,

    /// All components (direct and transitive dependencies).
    pub components: Vec<SbomComponent>,

    /// Dependency relationships between components.
    pub relationships: Vec<SbomRelationship>,

    /// AI-specific metadata for the package.
    pub ai_metadata: AiSbomMetadata,
}

/// Top-level package identity in the SBOM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbomPackageInfo {
    /// Fully qualified package name (e.g., "mapleai/agents/customer-support").
    pub name: String,

    /// Semantic version of the package.
    pub version: String,

    /// Package kind (agent, skill, model, etc.).
    pub kind: PackageKind,

    /// Human-readable description.
    pub description: Option<String>,

    /// License expression (SPDX).
    pub license: Option<String>,
}

/// A single component (dependency) in the SBOM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbomComponent {
    /// Fully qualified component name.
    pub name: String,

    /// Resolved version.
    pub version: String,

    /// Component kind.
    pub kind: PackageKind,

    /// Content-addressed digest of the resolved artifact.
    pub digest: String,

    /// Model-specific information (present only for model components).
    pub model_info: Option<ModelSbomInfo>,
}

/// Model-specific SBOM information.
///
/// Captures provenance details for model artifacts that are critical
/// for AI governance and audit trails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSbomInfo {
    /// Model architecture (e.g., "transformer", "llama", "mistral").
    pub architecture: Option<String>,

    /// Parameter count (e.g., "8B", "70B").
    pub parameter_count: Option<String>,

    /// Quantization format (e.g., "q4_k_m", "fp16").
    pub quantization: Option<String>,

    /// Training data lineage description.
    pub training_data: Option<String>,

    /// Known capabilities (e.g., "tool-calling", "vision").
    pub capabilities: Vec<String>,
}

/// A relationship between two components in the dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbomRelationship {
    /// The component that depends on another.
    pub from: String,

    /// The component being depended upon.
    pub to: String,

    /// The nature of the relationship.
    pub relationship_type: SbomRelationshipType,
}

/// Types of relationships between SBOM components.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SbomRelationshipType {
    /// Runtime dependency (required for execution).
    DependsOn,
    /// Build-time dependency (required for packaging).
    BuildDependency,
    /// Evaluation dependency (required for testing).
    EvalDependency,
    /// Model dependency (required model backend).
    ModelDependency,
}

/// AI-specific metadata that augments the standard SBOM.
///
/// Captures information about model usage, evaluation baselines, and
/// policy constraints that are unique to AI agent packages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSbomMetadata {
    /// Whether this package uses any AI models.
    pub uses_models: bool,

    /// References to models used by this package (from manifest).
    pub model_references: Vec<String>,

    /// Evaluation suite references and their thresholds.
    pub eval_suites: Vec<AiEvalReference>,

    /// Data classification level (e.g., "public", "internal", "confidential").
    pub data_classification: Option<String>,

    /// Jurisdiction restrictions for model usage.
    pub jurisdictions: Vec<String>,
}

/// Reference to an evaluation suite in the AI metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiEvalReference {
    /// OCI reference to the eval suite package.
    pub reference: String,

    /// Required pass threshold (0.0 - 1.0).
    pub threshold: Option<f64>,

    /// Whether this eval blocks deployment.
    pub blocking: bool,
}

/// Generate a MAPLE SBOM from a package manifest and its resolved lockfile.
///
/// The SBOM includes:
/// - Top-level package identity from the manifest
/// - All resolved components from the lockfile
/// - Dependency relationships extracted from lockfile entries
/// - AI-specific metadata (model references, eval config, data classification)
///
/// # Arguments
/// * `manifest` - The parsed MAPLE package manifest.
/// * `lockfile` - The resolved dependency lockfile from `maple-build`.
///
/// # Example
/// ```no_run
/// use maple_package_trust::sbom::generate_sbom;
/// // let manifest = parse_maplefile(...);
/// // let lockfile = build_graph.to_lockfile();
/// // let sbom = generate_sbom(&manifest, &lockfile);
/// ```
pub fn generate_sbom(manifest: &MapleManifest, lockfile: &BuildLockfile) -> MapleSbom {
    let package = SbomPackageInfo {
        name: manifest.name.to_qualified(),
        version: manifest.version.to_string(),
        kind: manifest.kind.clone(),
        description: manifest.description.clone(),
        license: manifest.metadata.license.clone(),
    };

    // Build components from lockfile entries
    let components: Vec<SbomComponent> = lockfile
        .entries
        .iter()
        .map(|entry| {
            let model_info = if entry.kind == PackageKind::ModelPackage {
                Some(ModelSbomInfo {
                    architecture: None,
                    parameter_count: None,
                    quantization: None,
                    training_data: None,
                    capabilities: Vec::new(),
                })
            } else {
                None
            };

            SbomComponent {
                name: entry.name.clone(),
                version: entry.version.to_string(),
                kind: entry.kind.clone(),
                digest: entry.digest.clone(),
                model_info,
            }
        })
        .collect();

    // Build relationships from lockfile dependency edges
    let relationships: Vec<SbomRelationship> = lockfile
        .entries
        .iter()
        .flat_map(|entry| {
            entry.dependencies.iter().map(move |dep| SbomRelationship {
                from: entry.name.clone(),
                to: dep.clone(),
                relationship_type: SbomRelationshipType::DependsOn,
            })
        })
        .collect();

    // Extract AI metadata from the manifest
    let ai_metadata = extract_ai_metadata(manifest);

    MapleSbom {
        sbom_version: "maple-sbom/v1".to_string(),
        created_at: Utc::now(),
        package,
        components,
        relationships,
        ai_metadata,
    }
}

/// Extract AI-specific metadata from the manifest for the SBOM.
fn extract_ai_metadata(manifest: &MapleManifest) -> AiSbomMetadata {
    let mut model_references = Vec::new();
    let mut data_classification = None;
    let mut jurisdictions = Vec::new();

    if let Some(ref models) = manifest.models {
        model_references.push(models.default.reference.clone());
        for alt in &models.alternatives {
            model_references.push(alt.reference.clone());
        }
        if let Some(ref constraints) = models.constraints {
            data_classification = constraints.data_classification.clone();
            jurisdictions = constraints.jurisdictions.clone();
        }
    }

    let eval_suites = manifest
        .eval
        .as_ref()
        .map(|eval| {
            eval.suites
                .iter()
                .map(|s| AiEvalReference {
                    reference: s.reference.clone(),
                    threshold: s.threshold,
                    blocking: s.blocking,
                })
                .collect()
        })
        .unwrap_or_default();

    AiSbomMetadata {
        uses_models: !model_references.is_empty(),
        model_references,
        eval_suites,
        data_classification,
        jurisdictions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_build::{BuildLockfile, LockfileEntry};
    use maple_package::{PackageKind, PackageName};
    use semver::Version;

    /// Build a minimal test manifest for SBOM generation.
    fn test_manifest() -> MapleManifest {
        MapleManifest {
            api_version: "maple.ai/v1".to_string(),
            kind: PackageKind::AgentPackage,
            name: PackageName::parse("testorg/agents/test-agent").unwrap(),
            version: Version::new(1, 0, 0),
            description: Some("A test agent for SBOM generation".to_string()),
            metadata: maple_package::PackageMetadata {
                authors: vec!["Test Author".to_string()],
                license: Some("MIT".to_string()),
                homepage: None,
                repository: None,
                keywords: vec![],
                labels: Default::default(),
            },
            base: None,
            models: Some(maple_package::ModelRequirements {
                default: maple_package::ModelReference {
                    reference: "openai:gpt-4o".to_string(),
                    min_context: Some(128000),
                    capabilities: vec!["tool-calling".to_string()],
                },
                alternatives: vec![maple_package::ModelReference {
                    reference: "anthropic:claude-sonnet".to_string(),
                    min_context: Some(200000),
                    capabilities: vec!["tool-calling".to_string()],
                }],
                constraints: Some(maple_package::ModelConstraints {
                    data_classification: Some("internal".to_string()),
                    jurisdictions: vec!["US".to_string(), "EU".to_string()],
                    max_cost_per_1k_tokens: None,
                }),
            }),
            skills: vec![maple_package::SkillDependency {
                reference: "testorg/skills/zendesk-connector".to_string(),
                version: "^1.0".to_string(),
                optional: false,
                provides: vec!["zendesk.ticket.read".to_string()],
            }],
            contracts: vec![],
            memory: None,
            policy: None,
            observability: None,
            runtime: None,
            eval: Some(maple_package::EvalConfig {
                suites: vec![maple_package::EvalSuiteReference {
                    reference: "testorg/eval/baseline".to_string(),
                    threshold: Some(0.95),
                    blocking: true,
                }],
                min_pass_rate: Some(0.9),
                max_regression_pct: Some(5.0),
            }),
            provenance: None,
        }
    }

    /// Build a test lockfile with multiple dependency entries.
    fn test_lockfile() -> BuildLockfile {
        BuildLockfile {
            schema_version: 1,
            entries: vec![
                LockfileEntry {
                    name: "testorg/agents/test-agent".to_string(),
                    kind: PackageKind::AgentPackage,
                    version: Version::new(1, 0, 0),
                    digest: "blake3:aabbccdd".to_string(),
                    dependencies: vec!["testorg/skills/zendesk-connector".to_string()],
                },
                LockfileEntry {
                    name: "testorg/skills/zendesk-connector".to_string(),
                    kind: PackageKind::SkillPackage,
                    version: Version::new(1, 2, 0),
                    digest: "blake3:11223344".to_string(),
                    dependencies: vec![],
                },
                LockfileEntry {
                    name: "testorg/models/gpt4o-adapter".to_string(),
                    kind: PackageKind::ModelPackage,
                    version: Version::new(0, 1, 0),
                    digest: "blake3:55667788".to_string(),
                    dependencies: vec![],
                },
            ],
        }
    }

    #[test]
    fn test_generate_sbom_basic() {
        let manifest = test_manifest();
        let lockfile = test_lockfile();

        let sbom = generate_sbom(&manifest, &lockfile);

        assert_eq!(sbom.sbom_version, "maple-sbom/v1");
        assert_eq!(sbom.package.name, "testorg/agents/test-agent");
        assert_eq!(sbom.package.version, "1.0.0");
        assert_eq!(sbom.package.kind, PackageKind::AgentPackage);
        assert_eq!(sbom.package.license, Some("MIT".to_string()));
    }

    #[test]
    fn test_sbom_includes_all_deps() {
        let manifest = test_manifest();
        let lockfile = test_lockfile();

        let sbom = generate_sbom(&manifest, &lockfile);

        assert_eq!(sbom.components.len(), 3);

        let names: Vec<&str> = sbom.components.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"testorg/agents/test-agent"));
        assert!(names.contains(&"testorg/skills/zendesk-connector"));
        assert!(names.contains(&"testorg/models/gpt4o-adapter"));
    }

    #[test]
    fn test_sbom_model_component_has_model_info() {
        let manifest = test_manifest();
        let lockfile = test_lockfile();

        let sbom = generate_sbom(&manifest, &lockfile);

        let model_component = sbom
            .components
            .iter()
            .find(|c| c.kind == PackageKind::ModelPackage)
            .expect("should have a model component");
        assert!(model_component.model_info.is_some());

        // Non-model components should not have model_info
        let skill_component = sbom
            .components
            .iter()
            .find(|c| c.kind == PackageKind::SkillPackage)
            .expect("should have a skill component");
        assert!(skill_component.model_info.is_none());
    }

    #[test]
    fn test_sbom_relationships() {
        let manifest = test_manifest();
        let lockfile = test_lockfile();

        let sbom = generate_sbom(&manifest, &lockfile);

        // The root agent depends on the zendesk skill
        assert_eq!(sbom.relationships.len(), 1);
        assert_eq!(sbom.relationships[0].from, "testorg/agents/test-agent");
        assert_eq!(
            sbom.relationships[0].to,
            "testorg/skills/zendesk-connector"
        );
        assert_eq!(
            sbom.relationships[0].relationship_type,
            SbomRelationshipType::DependsOn
        );
    }

    #[test]
    fn test_sbom_ai_metadata() {
        let manifest = test_manifest();
        let lockfile = test_lockfile();

        let sbom = generate_sbom(&manifest, &lockfile);

        assert!(sbom.ai_metadata.uses_models);
        assert_eq!(sbom.ai_metadata.model_references.len(), 2);
        assert!(sbom.ai_metadata.model_references.contains(&"openai:gpt-4o".to_string()));
        assert!(sbom
            .ai_metadata
            .model_references
            .contains(&"anthropic:claude-sonnet".to_string()));

        assert_eq!(
            sbom.ai_metadata.data_classification,
            Some("internal".to_string())
        );
        assert_eq!(sbom.ai_metadata.jurisdictions, vec!["US", "EU"]);

        assert_eq!(sbom.ai_metadata.eval_suites.len(), 1);
        assert_eq!(
            sbom.ai_metadata.eval_suites[0].reference,
            "testorg/eval/baseline"
        );
        assert_eq!(sbom.ai_metadata.eval_suites[0].threshold, Some(0.95));
        assert!(sbom.ai_metadata.eval_suites[0].blocking);
    }

    #[test]
    fn test_sbom_without_models() {
        let mut manifest = test_manifest();
        manifest.models = None;
        manifest.eval = None;
        let lockfile = test_lockfile();

        let sbom = generate_sbom(&manifest, &lockfile);

        assert!(!sbom.ai_metadata.uses_models);
        assert!(sbom.ai_metadata.model_references.is_empty());
        assert!(sbom.ai_metadata.eval_suites.is_empty());
        assert!(sbom.ai_metadata.data_classification.is_none());
        assert!(sbom.ai_metadata.jurisdictions.is_empty());
    }

    #[test]
    fn test_sbom_json_roundtrip() {
        let manifest = test_manifest();
        let lockfile = test_lockfile();

        let sbom = generate_sbom(&manifest, &lockfile);

        let json = serde_json::to_string_pretty(&sbom).unwrap();
        let deserialized: MapleSbom = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.sbom_version, sbom.sbom_version);
        assert_eq!(deserialized.package.name, sbom.package.name);
        assert_eq!(deserialized.components.len(), sbom.components.len());
        assert_eq!(deserialized.relationships.len(), sbom.relationships.len());
        assert_eq!(
            deserialized.ai_metadata.uses_models,
            sbom.ai_metadata.uses_models
        );
    }

    #[test]
    fn test_sbom_empty_lockfile() {
        let manifest = test_manifest();
        let lockfile = BuildLockfile {
            schema_version: 1,
            entries: vec![],
        };

        let sbom = generate_sbom(&manifest, &lockfile);

        assert!(sbom.components.is_empty());
        assert!(sbom.relationships.is_empty());
    }
}
