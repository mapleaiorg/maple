//! Policy types for the MAPLE Guard policy engine.
//!
//! A policy is a named, versioned set of rules with an enforcement level.
//! Policies use DENY-FIRST semantics: if any mandatory rule denies, the action is blocked.

use serde::{Deserialize, Serialize};

/// A policy is a named, versioned set of rules with an enforcement level.
/// Policies are DENY-FIRST: if any rule denies, the action is blocked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Unique identifier
    pub id: PolicyId,
    /// Human-readable name
    pub name: String,
    /// Semantic version
    pub version: semver::Version,
    /// Description of what this policy enforces
    pub description: String,
    /// Enforcement domain
    pub domain: PolicyDomain,
    /// Enforcement level
    pub enforcement: EnforcementLevel,
    /// The rules that make up this policy
    pub rules: Vec<PolicyRule>,
    /// Metadata
    pub metadata: PolicyMetadata,
}

/// Unique policy identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PolicyId(pub String);

/// Where this policy is enforced.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDomain {
    /// Input/prompt ingress
    Ingress,
    /// Model inference calls
    Inference,
    /// Tool/action execution
    ToolExecution,
    /// Output/response egress
    Egress,
    /// Memory read/write operations
    Memory,
    /// Data handling and classification
    DataClassification,
    /// Financial operations
    Financial,
    /// Cross-domain (applies everywhere)
    Global,
}

/// Enforcement level determines how rule matches are handled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementLevel {
    /// Action is blocked if rule denies
    Mandatory,
    /// Logged but not blocked
    Advisory,
    /// Evaluated during audit only
    AuditOnly,
    /// Dry-run: evaluate but don't block, log what would happen
    DryRun,
}

/// A single policy rule -- the atomic unit of enforcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Rule identifier (unique within policy)
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// The condition that triggers this rule
    pub condition: RuleCondition,
    /// The action to take when condition matches
    pub action: RuleAction,
    /// Priority (lower = evaluated first)
    pub priority: u32,
    /// Is this rule enabled?
    pub enabled: bool,
}

/// Condition that triggers a policy rule.
///
/// Conditions can be composed using `All`, `Any`, and `Not` combinators.
/// There are 15+ condition types covering data classification, tool matching,
/// model selection, content patterns, risk scores, financial thresholds,
/// rate limiting, tenant routing, and jurisdiction matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleCondition {
    /// Always matches
    Always,
    /// Never matches (disabled rule)
    Never,
    /// Match on data classification level
    DataClassification {
        /// Classification levels that trigger this rule
        /// e.g. "public", "internal", "confidential", "regulated"
        levels: Vec<String>,
    },
    /// Match on tool/capability being invoked
    ToolMatch {
        /// Tool name patterns (glob: "banking.*", "*.delete")
        patterns: Vec<String>,
    },
    /// Match on model being used
    ModelMatch {
        /// Model name patterns
        patterns: Vec<String>,
    },
    /// Match on content patterns (regex)
    ContentMatch {
        /// Regex patterns to match against content
        patterns: Vec<String>,
        /// Where to look: "input", "output", "both"
        scope: String,
    },
    /// Match on risk score threshold
    RiskThreshold {
        /// Minimum risk score to trigger (0.0 - 1.0)
        min_score: f64,
    },
    /// Match on amount/value threshold (financial)
    AmountThreshold {
        /// Minimum amount to trigger
        min_amount: f64,
        /// Currency
        currency: String,
    },
    /// Match on rate/frequency
    RateExceeded {
        /// Maximum allowed per window
        max_count: u32,
        /// Time window in seconds
        window_seconds: u64,
    },
    /// Match on tenant/org
    TenantMatch {
        /// Tenant identifiers
        tenants: Vec<String>,
    },
    /// Match on jurisdiction
    JurisdictionMatch {
        /// ISO country codes
        jurisdictions: Vec<String>,
    },
    /// Match on the worldline/agent identity
    IdentityMatch {
        /// WorldLine identity patterns (glob)
        patterns: Vec<String>,
    },
    /// Match on specific metadata key-value pairs
    MetadataMatch {
        /// Key to match
        key: String,
        /// Value pattern (glob)
        pattern: String,
    },
    /// Match on time window (e.g., outside business hours)
    TimeWindow {
        /// Allowed days of week (0=Monday, 6=Sunday)
        days: Vec<u8>,
        /// Start hour (0-23) in UTC
        start_hour: u8,
        /// End hour (0-23) in UTC
        end_hour: u8,
    },
    /// Match on request/operation type
    OperationType {
        /// Operation types that trigger this rule
        operations: Vec<String>,
    },
    /// Logical AND of multiple conditions
    All {
        conditions: Vec<RuleCondition>,
    },
    /// Logical OR of multiple conditions
    Any {
        conditions: Vec<RuleCondition>,
    },
    /// Logical NOT of a condition
    Not {
        condition: Box<RuleCondition>,
    },
}

/// Action to take when a rule condition matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleAction {
    /// Allow the action to proceed
    Allow,
    /// Block the action with a reason
    Deny {
        reason: String,
        /// Error code for programmatic handling
        code: Option<String>,
    },
    /// Require human approval before proceeding
    RequireApproval {
        /// Who can approve
        approvers: Vec<String>,
        /// Timeout before auto-deny
        timeout_seconds: Option<u64>,
        /// Approval message template
        message: String,
    },
    /// Redact sensitive content and allow
    Redact {
        /// What to redact (regex patterns)
        patterns: Vec<String>,
        /// Replacement text
        replacement: String,
    },
    /// Route to a different model/backend
    Reroute {
        /// Target backend
        backend: String,
        /// Target model
        model: Option<String>,
        /// Reason for reroute
        reason: String,
    },
    /// Log and continue (for advisory/audit policies)
    Log {
        /// Log level
        level: String,
        /// Log message template
        message: String,
    },
    /// Throttle (reduce rate)
    Throttle {
        /// Maximum rate after throttling
        max_per_minute: u32,
    },
    /// Alert (notify operators)
    Alert {
        /// Alert channel
        channel: String,
        /// Alert severity
        severity: String,
        /// Alert message template
        message: String,
    },
}

/// Metadata for a policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyMetadata {
    /// Authors of this policy
    pub authors: Vec<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Compliance frameworks this policy supports
    /// e.g. "SOC2", "HIPAA", "PCI-DSS", "GDPR"
    pub compliance_frameworks: Vec<String>,
    /// When this policy becomes effective
    pub effective_date: Option<chrono::DateTime<chrono::Utc>>,
    /// When this policy expires
    pub expiry_date: Option<chrono::DateTime<chrono::Utc>>,
    /// When this policy should be reviewed
    pub review_date: Option<chrono::DateTime<chrono::Utc>>,
}

impl PolicyMetadata {
    /// Create empty metadata.
    pub fn empty() -> Self {
        Self {
            authors: Vec::new(),
            tags: Vec::new(),
            compliance_frameworks: Vec::new(),
            effective_date: None,
            expiry_date: None,
            review_date: None,
        }
    }
}
