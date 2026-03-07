pub mod error;
pub mod manifest;
pub mod parser;
pub mod validate;

pub use error::PackageError;
pub use manifest::*;
pub use parser::{parse_maplefile, parse_maplefile_str, ManifestFormat, ParseError};
pub use validate::{validate_manifest, ValidationError, ValidationResult, ValidationWarning};

#[cfg(test)]
mod tests {
    use super::*;
    use semver::Version;

    fn sample_agent_manifest_yaml() -> &'static str {
        r#"
api_version: "maple.ai/v1"
kind: agent-package
name: "mapleai/agents/customer-support"
version: "1.0.0"
description: "Customer support agent with Zendesk integration"
metadata:
  authors:
    - "Maple AI Team"
  license: "MIT OR Apache-2.0"
  keywords:
    - "customer-support"
    - "zendesk"
  labels: {}
models:
  default:
    reference: "openai:gpt-4o"
    min_context: 128000
    capabilities:
      - "tool-calling"
      - "json-mode"
  alternatives:
    - reference: "anthropic:claude-sonnet"
      min_context: 200000
      capabilities:
        - "tool-calling"
  constraints:
    data_classification: "internal"
    jurisdictions:
      - "US"
      - "EU"
skills:
  - reference: "mapleai/skills/zendesk-connector"
    version: "^1.0"
    optional: false
    provides:
      - "zendesk.ticket.read"
      - "zendesk.ticket.update"
contracts:
  - reference: "mapleai/contracts/pci-dss"
    version: "^1.0"
    enforcement: mandatory
memory:
  worldline:
    mode: "event-ledger"
    backend: "postgres"
    retention:
      max_age_days: 365
      classification: "internal"
  vector:
    backend: "pgvector"
    dimensions: 1536
    distance_metric: "cosine"
policy:
  deny_by_default: true
  allow:
    - tool: "zendesk.ticket.read"
      requires_approval: false
    - tool: "zendesk.ticket.update"
      scope: "assigned-tickets-only"
      requires_approval: true
      rate_limit: 100
  rate_limits:
    rpm: 60
    tpm: 100000
    tool_calls_per_hour: 500
  budget:
    max_per_hour_usd: 5.0
    max_per_day_usd: 50.0
    alert_threshold_pct: 80.0
observability:
  traces: "otel"
  replay: "enabled"
  metrics: "prometheus"
runtime:
  min_maple_version: "0.1.0"
  features:
    - "worldline-v2"
eval:
  suites:
    - reference: "mapleai/eval/customer-support-baseline"
      threshold: 0.95
      blocking: true
  min_pass_rate: 0.9
  max_regression_pct: 5.0
"#
    }

    fn sample_model_manifest_yaml() -> &'static str {
        r#"
api_version: "maple.ai/v1"
kind: model-package
name: "mapleai/models/llama3-8b"
version: "1.0.0"
description: "LLaMA 3 8B quantized model"
metadata:
  authors:
    - "Maple AI Team"
  license: "MIT"
  keywords:
    - "llm"
    - "llama"
  labels:
    quantization: "q4_k_m"
    architecture: "llama"
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
      min_vram_gb: 6
      backends:
        - "cuda"
        - "metal"
"#
    }

    #[test]
    fn test_parse_agent_manifest_yaml() {
        let manifest =
            parse_maplefile_str(sample_agent_manifest_yaml(), ManifestFormat::Yaml).unwrap();
        assert_eq!(manifest.api_version, "maple.ai/v1");
        assert_eq!(manifest.kind, PackageKind::AgentPackage);
        assert_eq!(manifest.name.org, "mapleai");
        assert_eq!(manifest.name.category, "agents");
        assert_eq!(manifest.name.name, "customer-support");
        assert_eq!(manifest.version, Version::new(1, 0, 0));
        assert!(manifest.models.is_some());
        assert_eq!(manifest.skills.len(), 1);
        assert_eq!(manifest.contracts.len(), 1);
        assert!(manifest.memory.is_some());
        assert!(manifest.policy.is_some());
        assert!(manifest.observability.is_some());
    }

    #[test]
    fn test_parse_model_manifest_yaml() {
        let manifest =
            parse_maplefile_str(sample_model_manifest_yaml(), ManifestFormat::Yaml).unwrap();
        assert_eq!(manifest.kind, PackageKind::ModelPackage);
        assert_eq!(manifest.name.org, "mapleai");
        assert_eq!(manifest.name.category, "models");
        assert_eq!(manifest.name.name, "llama3-8b");
        assert!(manifest.runtime.is_some());
    }

    #[test]
    fn test_json_roundtrip() {
        let manifest =
            parse_maplefile_str(sample_agent_manifest_yaml(), ManifestFormat::Yaml).unwrap();
        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let roundtripped = parse_maplefile_str(&json, ManifestFormat::Json).unwrap();
        assert_eq!(roundtripped.api_version, manifest.api_version);
        assert_eq!(roundtripped.kind, manifest.kind);
        assert_eq!(roundtripped.name, manifest.name);
        assert_eq!(roundtripped.version, manifest.version);
    }

    #[test]
    fn test_validate_agent_without_models_warns() {
        let mut manifest =
            parse_maplefile_str(sample_agent_manifest_yaml(), ManifestFormat::Yaml).unwrap();
        manifest.models = None;
        let result = validate_manifest(&manifest);
        assert!(result.is_valid()); // warnings don't make it invalid
        assert!(result
            .warnings
            .iter()
            .any(|w| matches!(w, ValidationWarning::AgentWithoutModelRequirement)));
    }

    #[test]
    fn test_validate_skill_with_model_errors() {
        let mut manifest =
            parse_maplefile_str(sample_agent_manifest_yaml(), ManifestFormat::Yaml).unwrap();
        manifest.kind = PackageKind::SkillPackage;
        let result = validate_manifest(&manifest);
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e, ValidationError::SkillWithModelRequirement)));
    }

    #[test]
    fn test_validate_model_without_runtime_errors() {
        let mut manifest =
            parse_maplefile_str(sample_model_manifest_yaml(), ManifestFormat::Yaml).unwrap();
        manifest.runtime = None;
        let result = validate_manifest(&manifest);
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e, ValidationError::ModelWithoutRuntimeConstraints)));
    }

    #[test]
    fn test_validate_unsupported_api_version() {
        let mut manifest =
            parse_maplefile_str(sample_agent_manifest_yaml(), ManifestFormat::Yaml).unwrap();
        manifest.api_version = "maple.ai/v99".to_string();
        let result = validate_manifest(&manifest);
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e, ValidationError::UnsupportedApiVersion(_))));
    }

    #[test]
    fn test_package_name_parse_valid() {
        let name = PackageName::parse("mapleai/agents/customer-support").unwrap();
        assert_eq!(name.org, "mapleai");
        assert_eq!(name.category, "agents");
        assert_eq!(name.name, "customer-support");

        // 4-segment name
        let name = PackageName::parse("acme/skills/finance/sap-connector").unwrap();
        assert_eq!(name.org, "acme");
        assert_eq!(name.category, "skills/finance");
        assert_eq!(name.name, "sap-connector");

        // 2-segment name
        let name = PackageName::parse("mapleai/core").unwrap();
        assert_eq!(name.org, "mapleai");
        assert_eq!(name.category, "default");
        assert_eq!(name.name, "core");
    }

    #[test]
    fn test_package_name_parse_invalid() {
        // Too few segments
        assert!(PackageName::parse("single").is_err());

        // Too many segments
        assert!(PackageName::parse("a/b/c/d/e").is_err());

        // Uppercase
        assert!(PackageName::parse("Maple/agents/support").is_err());

        // Consecutive hyphens
        assert!(PackageName::parse("mapleai/agents/bad--name").is_err());

        // Empty segment
        assert!(PackageName::parse("mapleai//name").is_err());

        // Special chars
        assert!(PackageName::parse("maple_ai/agents/name").is_err());
    }

    #[test]
    fn test_package_name_qualified_roundtrip() {
        let name = PackageName::parse("mapleai/agents/support").unwrap();
        let qualified = name.to_qualified();
        let reparsed = PackageName::parse(&qualified).unwrap();
        assert_eq!(name, reparsed);
    }

    #[test]
    fn test_validate_invalid_version_constraint() {
        let mut manifest =
            parse_maplefile_str(sample_agent_manifest_yaml(), ManifestFormat::Yaml).unwrap();
        manifest.contracts[0].version = "not-a-version".to_string();
        let result = validate_manifest(&manifest);
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e, ValidationError::InvalidVersionConstraint { .. })));
    }

    #[test]
    fn test_validate_policy_pack_without_policy() {
        let mut manifest =
            parse_maplefile_str(sample_agent_manifest_yaml(), ManifestFormat::Yaml).unwrap();
        manifest.kind = PackageKind::PolicyPack;
        manifest.models = None;
        manifest.policy = None;
        let result = validate_manifest(&manifest);
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e, ValidationError::PolicyPackWithoutPolicyConfig)));
    }

    #[test]
    fn test_package_reference_parse() {
        let pr = PackageReference {
            reference: "mapleai/agents/support:1.0.0".to_string(),
            parsed: None,
        };
        let parsed = pr.parse().unwrap();
        assert!(parsed.registry.is_none());
        assert_eq!(parsed.name.org, "mapleai");
        assert_eq!(parsed.tag, Some("1.0.0".to_string()));

        // With registry
        let pr = PackageReference {
            reference: "registry.maple.ai/mapleai/agents/support:2.0.0".to_string(),
            parsed: None,
        };
        let parsed = pr.parse().unwrap();
        assert_eq!(parsed.registry, Some("registry.maple.ai".to_string()));
        assert_eq!(parsed.tag, Some("2.0.0".to_string()));

        // With OCI prefix
        let pr = PackageReference {
            reference: "oci://registry.maple.ai/mapleai/agents/support:3.0.0".to_string(),
            parsed: None,
        };
        let parsed = pr.parse().unwrap();
        assert_eq!(parsed.registry, Some("registry.maple.ai".to_string()));
        assert_eq!(parsed.tag, Some("3.0.0".to_string()));
    }
}
