//! Template generation for each `PackageKind`.
//!
//! Each template produces a complete Maplefile YAML string with sensible
//! defaults, YAML comments explaining every section, and placeholder values
//! for the user to customise.

use maple_package::PackageKind;

/// Generate a complete Maplefile YAML template for the given package kind.
///
/// The template is immediately parseable by `maple_package::parse_maplefile_str`
/// and will pass validation (no hard errors).
pub fn generate_template(kind: PackageKind, name: &str, org: &str) -> String {
    match kind {
        PackageKind::AgentPackage => agent_package_template(name, org),
        PackageKind::SkillPackage => skill_package_template(name, org),
        PackageKind::ModelPackage => model_package_template(name, org),
        PackageKind::ContractBundle => contract_bundle_template(name, org),
        PackageKind::EvalSuite => eval_suite_template(name, org),
        PackageKind::KnowledgePack => knowledge_pack_template(name, org),
        PackageKind::PolicyPack => policy_pack_template(name, org),
        PackageKind::EvidencePack => evidence_pack_template(name, org),
        PackageKind::UiModule => ui_module_template(name, org),
    }
}

fn agent_package_template(name: &str, org: &str) -> String {
    format!(
        r#"# Maplefile — Agent Package
# Documentation: https://docs.maple.ai/packages/agent-package

# Schema version for forward compatibility
api_version: "maple.ai/v1"

# Package kind — determines validation rules and required fields
kind: agent-package

# Fully qualified name: <org>/<category>/<name>
name: "{org}/agents/{name}"

# Semantic version (semver)
version: "0.1.0"

# Human-readable description of what this agent does
description: "TODO: Describe your agent"

# Authorship and licensing metadata
metadata:
  authors:
    - "{org}"
  license: "MIT OR Apache-2.0"
  keywords: []
  labels: {{}}

# Model requirements — which LLM this agent needs
# reference format: "provider:model-id" or "org/models/name"
models:
  default:
    reference: "openai:gpt-4o"
    capabilities:
      - "tool-calling"
  alternatives: []
  # constraints:
  #   data_classification: "internal"
  #   jurisdictions: ["US", "EU"]

# Skills this agent can invoke
# Each skill is a reusable tool package
skills: []
# - reference: "mapleai/skills/web-search"
#   version: "^1.0"
#   optional: false
#   provides:
#     - "web.search"

# Contracts/policies this agent must comply with
contracts: []
# - reference: "mapleai/contracts/pci-dss"
#   version: "^1.0"
#   enforcement: mandatory

# Memory and storage configuration
memory:
  worldline:
    mode: "event-ledger"
    backend: "sqlite"
  # vector:
  #   backend: "pgvector"
  #   dimensions: 1536
  #   distance_metric: "cosine"

# Deny-by-default capability policy
policy:
  deny_by_default: true
  allow: []
  # - tool: "web.search"
  #   requires_approval: false
  # rate_limits:
  #   rpm: 60
  #   tpm: 100000
  # budget:
  #   max_per_hour_usd: 5.0
  #   max_per_day_usd: 50.0

# Observability — tracing, replay, and metrics
observability:
  traces: "otel"
  replay: "enabled"
  metrics: "prometheus"

# Runtime compatibility constraints
# runtime:
#   min_maple_version: "0.1.0"
#   features:
#     - "worldline-v2"

# Eval baselines that must pass before deployment
# eval:
#   suites:
#     - reference: "mapleai/eval/baseline"
#       threshold: 0.95
#       blocking: true
#   min_pass_rate: 0.9
"#
    )
}

fn skill_package_template(name: &str, org: &str) -> String {
    format!(
        r#"# Maplefile — Skill Package
# A reusable tool/capability that agents can invoke.
# Documentation: https://docs.maple.ai/packages/skill-package

# Schema version for forward compatibility
api_version: "maple.ai/v1"

# Package kind
kind: skill-package

# Fully qualified name: <org>/skills/<name>
name: "{org}/skills/{name}"

# Semantic version (semver)
version: "0.1.0"

# Human-readable description of what this skill does
description: "TODO: Describe your skill"

# Authorship and licensing metadata
metadata:
  authors:
    - "{org}"
  license: "MIT OR Apache-2.0"
  keywords: []
  labels: {{}}

# NOTE: Skill packages must NOT declare model requirements.
# Models are provided by the agent that invokes this skill.

# Skills can depend on other skills
skills: []
# - reference: "mapleai/skills/http-client"
#   version: "^1.0"
#   optional: false
#   provides:
#     - "http.get"
#     - "http.post"

# Contracts this skill must comply with
contracts: []
# - reference: "mapleai/contracts/data-handling"
#   version: "^1.0"
#   enforcement: mandatory

# Policy declarations for this skill's capabilities
policy:
  deny_by_default: true
  allow: []
  # - tool: "my-skill.action"
  #   requires_approval: false

# Observability settings
observability:
  traces: "otel"
  metrics: "prometheus"
"#
    )
}

fn model_package_template(name: &str, org: &str) -> String {
    format!(
        r#"# Maplefile — Model Package
# An LLM model artifact (GGUF, safetensors, adapter).
# Documentation: https://docs.maple.ai/packages/model-package

# Schema version for forward compatibility
api_version: "maple.ai/v1"

# Package kind
kind: model-package

# Fully qualified name: <org>/models/<name>
name: "{org}/models/{name}"

# Semantic version (semver)
version: "0.1.0"

# Human-readable description of this model
description: "TODO: Describe your model"

# Authorship and licensing metadata
metadata:
  authors:
    - "{org}"
  license: "MIT OR Apache-2.0"
  keywords:
    - "llm"
  labels: {{}}

# Runtime constraints are REQUIRED for model packages.
# Specify platform, hardware, and compatibility requirements.
runtime:
  features: []
  platform:
    os:
      - "linux"
      - "macos"
    arch:
      - "x86_64"
      - "aarch64"
    gpu:
      min_vram_gb: 8
      backends:
        - "cuda"
        - "metal"
"#
    )
}

fn contract_bundle_template(name: &str, org: &str) -> String {
    format!(
        r#"# Maplefile — Contract Bundle
# A policy/contract bundle for governance and compliance.
# Documentation: https://docs.maple.ai/packages/contract-bundle

# Schema version for forward compatibility
api_version: "maple.ai/v1"

# Package kind
kind: contract-bundle

# Fully qualified name: <org>/contracts/<name>
name: "{org}/contracts/{name}"

# Semantic version (semver)
version: "0.1.0"

# Human-readable description of this contract bundle
description: "TODO: Describe your contract bundle"

# Authorship and licensing metadata
metadata:
  authors:
    - "{org}"
  license: "MIT OR Apache-2.0"
  keywords:
    - "governance"
  labels: {{}}

# Contracts included in this bundle
contracts: []
# - reference: "mapleai/contracts/data-handling"
#   version: "^1.0"
#   enforcement: mandatory

# Policy configuration for enforcing contracts
policy:
  deny_by_default: true
  allow: []
"#
    )
}

fn eval_suite_template(name: &str, org: &str) -> String {
    format!(
        r#"# Maplefile — Eval Suite
# An evaluation suite containing test vectors, red-team cases, and benchmarks.
# Documentation: https://docs.maple.ai/packages/eval-suite

# Schema version for forward compatibility
api_version: "maple.ai/v1"

# Package kind
kind: eval-suite

# Fully qualified name: <org>/eval/<name>
name: "{org}/eval/{name}"

# Semantic version (semver)
version: "0.1.0"

# Human-readable description of this eval suite
description: "TODO: Describe your evaluation suite"

# Authorship and licensing metadata
metadata:
  authors:
    - "{org}"
  license: "MIT OR Apache-2.0"
  keywords:
    - "evaluation"
    - "testing"
  labels: {{}}

# Eval configuration — defines pass criteria
eval:
  suites: []
  # - reference: "mapleai/eval/baseline"
  #   threshold: 0.95
  #   blocking: true
  min_pass_rate: 0.9
  max_regression_pct: 5.0
"#
    )
}

fn knowledge_pack_template(name: &str, org: &str) -> String {
    format!(
        r#"# Maplefile — Knowledge Pack
# A domain ontology, regulatory corpus, or industry dataset.
# Documentation: https://docs.maple.ai/packages/knowledge-pack

# Schema version for forward compatibility
api_version: "maple.ai/v1"

# Package kind
kind: knowledge-pack

# Fully qualified name: <org>/knowledge/<name>
name: "{org}/knowledge/{name}"

# Semantic version (semver)
version: "0.1.0"

# Human-readable description of this knowledge pack
description: "TODO: Describe your knowledge pack"

# Authorship and licensing metadata
# NOTE: keywords are important for knowledge pack discoverability
metadata:
  authors:
    - "{org}"
  license: "MIT OR Apache-2.0"
  keywords:
    - "knowledge"
    - "domain"
  labels: {{}}
"#
    )
}

fn policy_pack_template(name: &str, org: &str) -> String {
    format!(
        r#"# Maplefile — Policy Pack
# Executable constraints: data classification, model allowlists, tool policies.
# Documentation: https://docs.maple.ai/packages/policy-pack

# Schema version for forward compatibility
api_version: "maple.ai/v1"

# Package kind
kind: policy-pack

# Fully qualified name: <org>/policies/<name>
name: "{org}/policies/{name}"

# Semantic version (semver)
version: "0.1.0"

# Human-readable description of this policy pack
description: "TODO: Describe your policy pack"

# Authorship and licensing metadata
metadata:
  authors:
    - "{org}"
  license: "MIT OR Apache-2.0"
  keywords:
    - "policy"
    - "governance"
  labels: {{}}

# Policy configuration is REQUIRED for policy packs.
# Define deny-by-default rules and explicit allow grants.
policy:
  deny_by_default: true
  allow: []
  # - tool: "example.action"
  #   requires_approval: true
  #   scope: "read-only"
  # rate_limits:
  #   rpm: 60
  #   tpm: 100000
  # budget:
  #   max_per_hour_usd: 10.0
  #   max_per_day_usd: 100.0
"#
    )
}

fn evidence_pack_template(name: &str, org: &str) -> String {
    format!(
        r#"# Maplefile — Evidence Pack
# Audit templates, export formats, and GRC integrations.
# Documentation: https://docs.maple.ai/packages/evidence-pack

# Schema version for forward compatibility
api_version: "maple.ai/v1"

# Package kind
kind: evidence-pack

# Fully qualified name: <org>/evidence/<name>
name: "{org}/evidence/{name}"

# Semantic version (semver)
version: "0.1.0"

# Human-readable description of this evidence pack
description: "TODO: Describe your evidence pack"

# Authorship and licensing metadata
metadata:
  authors:
    - "{org}"
  license: "MIT OR Apache-2.0"
  keywords:
    - "audit"
    - "evidence"
    - "compliance"
  labels: {{}}

# Observability — tracing and metrics for evidence collection
observability:
  traces: "otel"
  metrics: "prometheus"
"#
    )
}

fn ui_module_template(name: &str, org: &str) -> String {
    format!(
        r#"# Maplefile — UI Module
# A user interface module for agent interactions.
# Documentation: https://docs.maple.ai/packages/ui-module

# Schema version for forward compatibility
api_version: "maple.ai/v1"

# Package kind
kind: ui-module

# Fully qualified name: <org>/ui/<name>
name: "{org}/ui/{name}"

# Semantic version (semver)
version: "0.1.0"

# Human-readable description of this UI module
description: "TODO: Describe your UI module"

# Authorship and licensing metadata
metadata:
  authors:
    - "{org}"
  license: "MIT OR Apache-2.0"
  keywords:
    - "ui"
    - "frontend"
  labels: {{}}

# Runtime constraints for the UI module
runtime:
  features: []
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_package::{parse_maplefile_str, validate_manifest, ManifestFormat};

    /// All PackageKind variants for iteration in tests.
    fn all_kinds() -> Vec<(PackageKind, &'static str)> {
        vec![
            (PackageKind::AgentPackage, "agent-package"),
            (PackageKind::SkillPackage, "skill-package"),
            (PackageKind::ModelPackage, "model-package"),
            (PackageKind::ContractBundle, "contract-bundle"),
            (PackageKind::EvalSuite, "eval-suite"),
            (PackageKind::KnowledgePack, "knowledge-pack"),
            (PackageKind::PolicyPack, "policy-pack"),
            (PackageKind::EvidencePack, "evidence-pack"),
            (PackageKind::UiModule, "ui-module"),
        ]
    }

    #[test]
    fn templates_generate_valid_yaml() {
        for (kind, kind_name) in all_kinds() {
            let template = generate_template(kind, "test-pkg", "myorg");
            let result = parse_maplefile_str(&template, ManifestFormat::Yaml);
            assert!(
                result.is_ok(),
                "Template for {kind_name} failed to parse: {:?}",
                result.err()
            );
        }
    }

    #[test]
    fn templates_pass_validation() {
        for (kind, kind_name) in all_kinds() {
            let template = generate_template(kind, "test-pkg", "myorg");
            let manifest = parse_maplefile_str(&template, ManifestFormat::Yaml)
                .unwrap_or_else(|e| panic!("Parse failed for {kind_name}: {e}"));
            let result = validate_manifest(&manifest);
            assert!(
                result.is_valid(),
                "Validation failed for {kind_name}: {:?}",
                result.errors
            );
        }
    }

    #[test]
    fn agent_template_has_correct_fields() {
        let template = generate_template(PackageKind::AgentPackage, "my-agent", "acme");
        let manifest = parse_maplefile_str(&template, ManifestFormat::Yaml).unwrap();
        assert_eq!(manifest.api_version, "maple.ai/v1");
        assert_eq!(manifest.kind, PackageKind::AgentPackage);
        assert_eq!(manifest.name.org, "acme");
        assert_eq!(manifest.name.category, "agents");
        assert_eq!(manifest.name.name, "my-agent");
        assert_eq!(manifest.version, semver::Version::new(0, 1, 0));
        assert!(manifest.models.is_some());
        assert!(manifest.memory.is_some());
        assert!(manifest.policy.is_some());
        assert!(manifest.observability.is_some());
        let policy = manifest.policy.unwrap();
        assert!(policy.deny_by_default);
    }

    #[test]
    fn skill_template_has_no_models() {
        let template = generate_template(PackageKind::SkillPackage, "my-skill", "acme");
        let manifest = parse_maplefile_str(&template, ManifestFormat::Yaml).unwrap();
        assert_eq!(manifest.kind, PackageKind::SkillPackage);
        assert!(manifest.models.is_none());
    }

    #[test]
    fn model_template_has_runtime_constraints() {
        let template = generate_template(PackageKind::ModelPackage, "my-model", "acme");
        let manifest = parse_maplefile_str(&template, ManifestFormat::Yaml).unwrap();
        assert_eq!(manifest.kind, PackageKind::ModelPackage);
        assert!(manifest.runtime.is_some());
    }

    #[test]
    fn policy_pack_template_has_policy() {
        let template = generate_template(PackageKind::PolicyPack, "my-policies", "acme");
        let manifest = parse_maplefile_str(&template, ManifestFormat::Yaml).unwrap();
        assert_eq!(manifest.kind, PackageKind::PolicyPack);
        assert!(manifest.policy.is_some());
    }
}
