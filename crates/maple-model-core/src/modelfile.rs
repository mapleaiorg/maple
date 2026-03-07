//! MapleModelfile parsing.
//!
//! Provides types and parsers for the MAPLE model definition file format,
//! which describes model configuration, governance constraints, and
//! benchmark requirements in YAML.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::metadata::InferenceDefaults;

/// Errors that can occur when parsing a Modelfile.
#[derive(Debug, thiserror::Error)]
pub enum ModelfileError {
    #[error("failed to read modelfile: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse modelfile YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("invalid modelfile: {0}")]
    Validation(String),
}

/// A parsed MapleModelfile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapleModelfile {
    /// Resource kind, always "MapleModelfile".
    pub kind: String,

    /// Model name.
    pub name: String,

    /// Base model reference (e.g. "llama-3.1-8b-instruct:Q4_K_M").
    pub base: String,

    /// Default inference parameters override.
    pub defaults: Option<InferenceDefaults>,

    /// Prompt templates by name.
    pub templates: Option<HashMap<String, String>>,

    /// Contract references (RCF contract identifiers).
    pub contracts: Vec<String>,

    /// Governance constraints.
    pub governance: Option<ModelGovernance>,

    /// Benchmark requirements.
    pub benchmarks: Option<ModelBenchmarkConfig>,
}

/// Governance constraints for model deployment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelGovernance {
    /// Data classification allowlists.
    pub allowlists: Vec<DataClassificationRule>,

    /// Allowed deployment jurisdictions (e.g. "US", "EU").
    pub jurisdictions: Vec<String>,

    /// Maximum cost per 1000 tokens (in USD).
    pub max_cost_per_1k: Option<f64>,

    /// Audit level (e.g. "full", "minimal", "none").
    pub audit_level: Option<String>,
}

/// A data classification rule for governance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataClassificationRule {
    /// Data classification label (e.g. "public", "internal", "confidential").
    pub data_classification: String,
}

/// Benchmark requirements for model qualification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelBenchmarkConfig {
    /// Minimum tokens per second throughput.
    pub min_tokens_per_second: Option<f64>,

    /// Maximum time-to-first-token in milliseconds.
    pub max_ttft_ms: Option<u64>,

    /// Minimum evaluation score (0.0 - 1.0).
    pub min_eval_score: Option<f64>,
}

/// Parse a MapleModelfile from YAML content.
pub fn parse_modelfile(content: &str) -> Result<MapleModelfile, ModelfileError> {
    let modelfile: MapleModelfile = serde_yaml::from_str(content)?;

    // Validate the kind field
    if modelfile.kind != "MapleModelfile" {
        return Err(ModelfileError::Validation(format!(
            "expected kind 'MapleModelfile', got '{}'",
            modelfile.kind
        )));
    }

    // Validate name is non-empty
    if modelfile.name.is_empty() {
        return Err(ModelfileError::Validation(
            "model name must not be empty".to_string(),
        ));
    }

    // Validate base is non-empty
    if modelfile.base.is_empty() {
        return Err(ModelfileError::Validation(
            "base model reference must not be empty".to_string(),
        ));
    }

    Ok(modelfile)
}

/// Parse a MapleModelfile from a file path.
pub fn parse_modelfile_path(path: &Path) -> Result<MapleModelfile, ModelfileError> {
    let content = std::fs::read_to_string(path)?;
    parse_modelfile(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_modelfile_yaml_with_governance() {
        let yaml = r#"
kind: MapleModelfile
name: my-custom-model
base: "llama-3.1-8b-instruct:Q4_K_M"
defaults:
  temperature: 0.5
  top_p: 0.85
  top_k: 50
  repeat_penalty: 1.05
  max_tokens: 4096
  stop_sequences:
    - "<|eot_id|>"
    - "<|end|>"
templates:
  system: "You are a helpful coding assistant."
  greeting: "Hello! How can I help you today?"
contracts:
  - "rcf://safety/v1"
  - "rcf://privacy/v1"
governance:
  allowlists:
    - data_classification: public
    - data_classification: internal
  jurisdictions:
    - US
    - EU
    - UK
  max_cost_per_1k: 0.005
  audit_level: full
benchmarks:
  min_tokens_per_second: 30.0
  max_ttft_ms: 500
  min_eval_score: 0.85
"#;

        let modelfile = parse_modelfile(yaml).expect("parse should succeed");

        assert_eq!(modelfile.kind, "MapleModelfile");
        assert_eq!(modelfile.name, "my-custom-model");
        assert_eq!(modelfile.base, "llama-3.1-8b-instruct:Q4_K_M");

        // Defaults
        let defaults = modelfile.defaults.as_ref().expect("defaults present");
        assert!((defaults.temperature - 0.5).abs() < f32::EPSILON);
        assert!((defaults.top_p - 0.85).abs() < f32::EPSILON);
        assert_eq!(defaults.top_k, Some(50));
        assert_eq!(defaults.max_tokens, Some(4096));
        assert_eq!(defaults.stop_sequences.len(), 2);

        // Templates
        let templates = modelfile.templates.as_ref().expect("templates present");
        assert_eq!(templates.len(), 2);
        assert!(templates.contains_key("system"));

        // Contracts
        assert_eq!(modelfile.contracts.len(), 2);
        assert_eq!(modelfile.contracts[0], "rcf://safety/v1");

        // Governance
        let gov = modelfile.governance.as_ref().expect("governance present");
        assert_eq!(gov.allowlists.len(), 2);
        assert_eq!(gov.allowlists[0].data_classification, "public");
        assert_eq!(gov.jurisdictions, vec!["US", "EU", "UK"]);
        assert!((gov.max_cost_per_1k.unwrap() - 0.005).abs() < f64::EPSILON);
        assert_eq!(gov.audit_level.as_deref(), Some("full"));

        // Benchmarks
        let bench = modelfile.benchmarks.as_ref().expect("benchmarks present");
        assert!((bench.min_tokens_per_second.unwrap() - 30.0).abs() < f64::EPSILON);
        assert_eq!(bench.max_ttft_ms, Some(500));
        assert!((bench.min_eval_score.unwrap() - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_modelfile_invalid_kind() {
        let yaml = r#"
kind: WrongKind
name: test
base: some-model
contracts: []
"#;
        let err = parse_modelfile(yaml).unwrap_err();
        assert!(
            err.to_string().contains("expected kind 'MapleModelfile'"),
            "error: {err}"
        );
    }

    #[test]
    fn test_parse_modelfile_empty_name() {
        let yaml = r#"
kind: MapleModelfile
name: ""
base: some-model
contracts: []
"#;
        let err = parse_modelfile(yaml).unwrap_err();
        assert!(
            err.to_string().contains("model name must not be empty"),
            "error: {err}"
        );
    }

    #[test]
    fn test_parse_modelfile_minimal() {
        let yaml = r#"
kind: MapleModelfile
name: minimal-model
base: llama-3.1-8b
contracts: []
"#;
        let modelfile = parse_modelfile(yaml).expect("parse should succeed");
        assert_eq!(modelfile.name, "minimal-model");
        assert!(modelfile.defaults.is_none());
        assert!(modelfile.templates.is_none());
        assert!(modelfile.governance.is_none());
        assert!(modelfile.benchmarks.is_none());
        assert!(modelfile.contracts.is_empty());
    }
}
