use serde::{Deserialize, Serialize};
use semver::Version;

use crate::error::PackageError;

/// The kind of artifact this package represents.
/// Each kind has different required fields and validation rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageKind {
    /// A complete agent: resonator + profile + capabilities + contracts + worldline config
    AgentPackage,
    /// A reusable skill/tool that agents can invoke
    SkillPackage,
    /// A policy/contract bundle for governance
    ContractBundle,
    /// An LLM model artifact (GGUF, safetensors, adapter)
    ModelPackage,
    /// An evaluation suite (test vectors, red-team cases, regression benchmarks)
    EvalSuite,
    /// A UI module for agent interfaces (optional, future)
    UiModule,
    /// A knowledge pack (domain ontology, regulatory corpus, industry data)
    KnowledgePack,
    /// A policy pack (executable constraints: data classification, model allowlists, tool policies)
    PolicyPack,
    /// An evidence pack (audit templates, export formats, GRC integrations)
    EvidencePack,
}

impl std::fmt::Display for PackageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentPackage => write!(f, "agent-package"),
            Self::SkillPackage => write!(f, "skill-package"),
            Self::ContractBundle => write!(f, "contract-bundle"),
            Self::ModelPackage => write!(f, "model-package"),
            Self::EvalSuite => write!(f, "eval-suite"),
            Self::UiModule => write!(f, "ui-module"),
            Self::KnowledgePack => write!(f, "knowledge-pack"),
            Self::PolicyPack => write!(f, "policy-pack"),
            Self::EvidencePack => write!(f, "evidence-pack"),
        }
    }
}

/// The top-level Maplefile manifest — analogous to Dockerfile but for cognitive services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapleManifest {
    /// Schema version for forward compatibility
    pub api_version: String, // "maple.ai/v1"
    /// What kind of artifact
    pub kind: PackageKind,
    /// Fully qualified name: <org>/<category>/<name>
    /// e.g., "mapleai/agents/customer-support"
    pub name: PackageName,
    /// Semantic version
    pub version: Version,
    /// Human-readable description
    pub description: Option<String>,
    /// Authorship and licensing
    pub metadata: PackageMetadata,
    /// Base image/package to extend (optional)
    pub base: Option<PackageReference>,
    /// Model requirements (for AgentPackage)
    pub models: Option<ModelRequirements>,
    /// Skill dependencies
    #[serde(default)]
    pub skills: Vec<SkillDependency>,
    /// Contract/policy dependencies
    #[serde(default)]
    pub contracts: Vec<ContractDependency>,
    /// Memory/storage configuration
    pub memory: Option<MemoryConfig>,
    /// Policy declarations (deny-by-default capabilities)
    pub policy: Option<PolicyConfig>,
    /// Observability configuration
    pub observability: Option<ObservabilityConfig>,
    /// Runtime compatibility constraints
    pub runtime: Option<RuntimeConstraints>,
    /// Eval baselines that must pass before deployment
    pub eval: Option<EvalConfig>,
    /// Build provenance (populated during `maple build`)
    pub provenance: Option<BuildProvenance>,
}

/// Fully qualified package name with validation.
///
/// Format: "org/category/name" where each segment is lowercase alphanumeric + hyphens,
/// 1-128 chars per segment, 2-4 segments total.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PackageName {
    /// Organization namespace, e.g., "mapleai"
    pub org: String,
    /// Category path, e.g., "agents" or "skills/finance"
    pub category: String,
    /// Package name, e.g., "customer-support"
    pub name: String,
}

impl PackageName {
    /// Parse from string format: "org/category/name"
    ///
    /// Validates: lowercase alphanumeric + hyphens, no consecutive hyphens,
    /// 1-128 chars per segment, 2-4 segments total.
    pub fn parse(s: &str) -> Result<Self, PackageError> {
        let segment_re = regex::Regex::new(r"^[a-z0-9]([a-z0-9-]*[a-z0-9])?$").unwrap();

        let segments: Vec<&str> = s.split('/').collect();
        if segments.len() < 2 || segments.len() > 4 {
            return Err(PackageError::InvalidPackageName {
                name: s.to_string(),
                reason: format!(
                    "expected 2-4 segments separated by '/', got {}",
                    segments.len()
                ),
            });
        }

        for (i, seg) in segments.iter().enumerate() {
            if seg.is_empty() || seg.len() > 128 {
                return Err(PackageError::InvalidPackageName {
                    name: s.to_string(),
                    reason: format!("segment {} must be 1-128 characters, got {}", i, seg.len()),
                });
            }
            if !segment_re.is_match(seg) {
                return Err(PackageError::InvalidPackageName {
                    name: s.to_string(),
                    reason: format!(
                        "segment '{}' must be lowercase alphanumeric with hyphens, no consecutive hyphens",
                        seg
                    ),
                });
            }
            if seg.contains("--") {
                return Err(PackageError::InvalidPackageName {
                    name: s.to_string(),
                    reason: format!("segment '{}' contains consecutive hyphens", seg),
                });
            }
        }

        let org = segments[0].to_string();
        let name = segments[segments.len() - 1].to_string();
        let category = segments[1..segments.len() - 1].join("/");

        // For 2-segment names like "org/name", category defaults to "default"
        let category = if category.is_empty() {
            "default".to_string()
        } else {
            category
        };

        Ok(Self {
            org,
            category,
            name,
        })
    }

    /// Full qualified string representation
    pub fn to_qualified(&self) -> String {
        if self.category == "default" {
            format!("{}/{}", self.org, self.name)
        } else {
            format!("{}/{}/{}", self.org, self.category, self.name)
        }
    }
}

impl std::fmt::Display for PackageName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_qualified())
    }
}

impl TryFrom<String> for PackageName {
    type Error = PackageError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s)
    }
}

impl From<PackageName> for String {
    fn from(name: PackageName) -> Self {
        name.to_qualified()
    }
}

/// Reference to another package (dependency or base)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageReference {
    /// OCI reference: "oci://registry.maple.ai/agents/customer-support:1.2.0"
    /// or short form: "mapleai/agents/customer-support:1.2.0"
    pub reference: String,
    /// Parsed registry, name, tag/digest
    #[serde(skip)]
    pub parsed: Option<ParsedReference>,
}

#[derive(Debug, Clone)]
pub struct ParsedReference {
    pub registry: Option<String>,
    pub name: PackageName,
    pub tag: Option<String>,
    pub digest: Option<String>,
}

impl PackageReference {
    /// Parse the reference string into components
    pub fn parse(&self) -> Result<ParsedReference, PackageError> {
        let s = self.reference.as_str();

        // Strip oci:// prefix if present
        let s = s.strip_prefix("oci://").unwrap_or(s);

        // Split off digest (@sha256:...) or tag (:version)
        let (name_part, tag, digest) = if let Some(at_pos) = s.rfind('@') {
            let digest = s[at_pos + 1..].to_string();
            let name_part = &s[..at_pos];
            (name_part, None, Some(digest))
        } else if let Some(colon_pos) = s.rfind(':') {
            // Ensure the colon is not in a registry URL (e.g., "registry:5000/org/name")
            let after_colon = &s[colon_pos + 1..];
            if after_colon.contains('/') {
                // This is a port number, not a tag
                (s, None, None)
            } else {
                let tag = after_colon.to_string();
                let name_part = &s[..colon_pos];
                (name_part, Some(tag), None)
            }
        } else {
            (s, None, None)
        };

        // Check if the first segment contains a dot (registry hostname)
        let segments: Vec<&str> = name_part.split('/').collect();
        let (registry, pkg_name_str) = if segments.len() >= 3
            && (segments[0].contains('.') || segments[0].contains(':'))
        {
            (
                Some(segments[0].to_string()),
                segments[1..].join("/"),
            )
        } else {
            (None, name_part.to_string())
        };

        let name = PackageName::parse(&pkg_name_str)?;

        Ok(ParsedReference {
            registry,
            name,
            tag,
            digest,
        })
    }
}

/// Model requirements for an agent package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRequirements {
    /// Default model to use
    pub default: ModelReference,
    /// Alternative models (policy chooses at runtime)
    #[serde(default)]
    pub alternatives: Vec<ModelReference>,
    /// Constraints on model selection
    pub constraints: Option<ModelConstraints>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelReference {
    /// Reference: "maple/models/llama3.2-8b-q4" or "openai:gpt-4o" or "anthropic:claude-sonnet"
    pub reference: String,
    /// Minimum context window required
    pub min_context: Option<u32>,
    /// Required capabilities
    #[serde(default)]
    pub capabilities: Vec<String>, // "tool-calling", "vision", "json-mode"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConstraints {
    /// Data classification level that restricts model choice
    /// e.g., "confidential" → only on-prem models allowed
    pub data_classification: Option<String>,
    /// Jurisdiction restrictions
    #[serde(default)]
    pub jurisdictions: Vec<String>,
    /// Maximum cost per 1K tokens (for budget routing)
    pub max_cost_per_1k_tokens: Option<f64>,
}

/// Skill dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDependency {
    /// OCI reference to skill package
    pub reference: String,
    /// Version constraint (semver range)
    pub version: String,
    /// Whether this skill is optional
    #[serde(default)]
    pub optional: bool,
    /// Capabilities this skill provides
    #[serde(default)]
    pub provides: Vec<String>,
}

/// Contract/policy dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractDependency {
    /// OCI reference to contract bundle
    pub reference: String,
    /// Version constraint
    pub version: String,
    /// Enforcement level
    pub enforcement: EnforcementLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EnforcementLevel {
    /// Must pass or action is blocked
    Mandatory,
    /// Logged but not blocked
    Advisory,
    /// Evaluated during audit only
    AuditOnly,
}

/// Memory and storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// WorldLine event ledger config
    pub worldline: WorldlineMemoryConfig,
    /// Vector store for embeddings/RAG
    pub vector: Option<VectorStoreConfig>,
    /// Structured storage (SQL)
    pub structured: Option<StructuredStoreConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldlineMemoryConfig {
    /// Event ledger mode
    pub mode: String, // "event-ledger" | "snapshot" | "hybrid"
    /// Storage backend preference
    pub backend: String, // "postgres" | "sqlite" | "rocksdb"
    /// Retention policy
    pub retention: Option<RetentionPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Maximum age of events before compaction
    pub max_age_days: Option<u32>,
    /// Maximum number of events before compaction
    pub max_events: Option<u64>,
    /// Data classification for retention compliance
    pub classification: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreConfig {
    pub backend: String, // "qdrant" | "pgvector" | "sqlite-vec"
    pub dimensions: u32,
    pub distance_metric: String, // "cosine" | "euclidean" | "dot"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredStoreConfig {
    pub backend: String,          // "postgres" | "sqlite"
    pub migrations: Vec<String>,  // paths to migration files within package
}

/// Policy configuration (deny-by-default)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// Default deny all capabilities
    #[serde(default = "default_true")]
    pub deny_by_default: bool,
    /// Explicit allow list
    #[serde(default)]
    pub allow: Vec<CapabilityGrant>,
    /// Rate limits
    pub rate_limits: Option<RateLimitConfig>,
    /// Budget caps
    pub budget: Option<BudgetConfig>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityGrant {
    /// Tool or resource being granted
    pub tool: String, // "zendesk.ticket.read" or "banking.payment.prepare"
    /// Scope restrictions
    pub scope: Option<String>,
    /// Requires human approval
    #[serde(default)]
    pub requires_approval: bool,
    /// Maximum invocations per time window
    pub rate_limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Max requests per minute
    pub rpm: Option<u32>,
    /// Max tokens per minute (for model calls)
    pub tpm: Option<u64>,
    /// Max tool calls per hour
    pub tool_calls_per_hour: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// Maximum spend per hour (USD)
    pub max_per_hour_usd: Option<f64>,
    /// Maximum spend per day (USD)
    pub max_per_day_usd: Option<f64>,
    /// Alert threshold (percentage of daily budget)
    pub alert_threshold_pct: Option<f64>,
}

/// Observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Enable OpenTelemetry tracing
    pub traces: Option<String>, // "otel" | "none"
    /// Enable replay capability
    pub replay: Option<String>, // "enabled" | "disabled"
    /// Metrics export
    pub metrics: Option<String>, // "prometheus" | "otel" | "none"
}

/// Runtime compatibility constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConstraints {
    /// Minimum MAPLE runtime version
    pub min_maple_version: Option<Version>,
    /// Required runtime features
    #[serde(default)]
    pub features: Vec<String>,
    /// Platform requirements
    pub platform: Option<PlatformConstraints>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConstraints {
    /// Required OS
    #[serde(default)]
    pub os: Vec<String>, // "linux", "macos", "windows"
    /// Required architecture
    #[serde(default)]
    pub arch: Vec<String>, // "x86_64", "aarch64"
    /// GPU requirements (for model packages)
    pub gpu: Option<GpuRequirements>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRequirements {
    /// Minimum VRAM in GB
    pub min_vram_gb: Option<u32>,
    /// Required compute capability
    pub compute_capability: Option<String>,
    /// Required backends
    #[serde(default)]
    pub backends: Vec<String>, // "cuda", "metal", "vulkan", "rocm"
}

/// Eval configuration — gates that must pass before deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalConfig {
    /// Eval suites that must pass
    #[serde(default)]
    pub suites: Vec<EvalSuiteReference>,
    /// Minimum pass rate
    pub min_pass_rate: Option<f64>,
    /// Maximum regression tolerance
    pub max_regression_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSuiteReference {
    /// OCI reference to eval suite package
    pub reference: String,
    /// Minimum score threshold
    pub threshold: Option<f64>,
    /// Is this eval blocking or advisory?
    #[serde(default = "default_true")]
    pub blocking: bool,
}

/// Build provenance — populated by `maple build`, immutable after signing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildProvenance {
    /// Content hash of the complete package
    pub digest: String,
    /// Build timestamp
    pub built_at: chrono::DateTime<chrono::Utc>,
    /// Builder identity
    pub builder: Option<String>,
    /// Source repository and commit
    pub source: Option<SourceReference>,
    /// Resolved dependency graph (all transitive deps with exact versions)
    #[serde(default)]
    pub resolved_deps: Vec<ResolvedDependency>,
    /// WorldLine event ID for build provenance
    pub worldline_event: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceReference {
    pub repository: String,
    pub commit: String,
    pub branch: Option<String>,
    #[serde(default)]
    pub dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedDependency {
    pub name: String,
    pub version: Version,
    pub digest: String,
    pub kind: PackageKind,
}

/// Package-level metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    #[serde(default)]
    pub authors: Vec<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub labels: std::collections::HashMap<String, String>,
}
