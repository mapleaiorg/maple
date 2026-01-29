//! Commitment Type Definitions
//!
//! Supporting types for RCL-Commitment artifacts.

use rcl_types::{IdentityRef, TemporalAnchor, ResonanceType};
use serde::{Deserialize, Serialize};

/// Intended outcome of a commitment
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntendedOutcome {
    /// Description of the intended outcome
    pub description: String,

    /// Success criteria
    pub success_criteria: Vec<String>,

    /// Expected effects
    pub expected_effects: Vec<ExpectedEffect>,
}

impl IntendedOutcome {
    /// Create a new intended outcome
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            success_criteria: Vec::new(),
            expected_effects: Vec::new(),
        }
    }

    /// Add success criterion
    pub fn with_criterion(mut self, criterion: impl Into<String>) -> Self {
        self.success_criteria.push(criterion.into());
        self
    }

    /// Add expected effect
    pub fn with_effect(mut self, effect: ExpectedEffect) -> Self {
        self.expected_effects.push(effect);
        self
    }
}

/// Expected effect of a commitment
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExpectedEffect {
    /// Type of effect
    pub effect_type: String,

    /// Target of the effect
    pub target: String,

    /// Magnitude of the effect (optional description)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magnitude: Option<String>,

    /// Whether this effect is reversible
    pub reversible: bool,
}

impl ExpectedEffect {
    /// Create a new expected effect
    pub fn new(effect_type: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            effect_type: effect_type.into(),
            target: target.into(),
            magnitude: None,
            reversible: false,
        }
    }

    /// Set magnitude
    pub fn with_magnitude(mut self, magnitude: impl Into<String>) -> Self {
        self.magnitude = Some(magnitude.into());
        self
    }

    /// Mark as reversible
    pub fn reversible(mut self) -> Self {
        self.reversible = true;
        self
    }
}

/// Commitment scope - boundaries on the commitment
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommitmentScope {
    /// Description of the scope
    pub description: String,

    /// Boundaries
    pub boundaries: Vec<ScopeBoundary>,

    /// Exclusions (what is NOT included)
    pub exclusions: Vec<String>,
}

impl CommitmentScope {
    /// Create a new scope
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            boundaries: Vec::new(),
            exclusions: Vec::new(),
        }
    }

    /// Add boundary
    pub fn with_boundary(mut self, boundary: ScopeBoundary) -> Self {
        self.boundaries.push(boundary);
        self
    }

    /// Add exclusion
    pub fn with_exclusion(mut self, exclusion: impl Into<String>) -> Self {
        self.exclusions.push(exclusion.into());
        self
    }
}

/// Scope boundary
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScopeBoundary {
    /// Dimension of the boundary
    pub dimension: String,

    /// Constraint on that dimension
    pub constraint: String,
}

impl ScopeBoundary {
    /// Create a new boundary
    pub fn new(dimension: impl Into<String>, constraint: impl Into<String>) -> Self {
        Self {
            dimension: dimension.into(),
            constraint: constraint.into(),
        }
    }
}

/// Target of a commitment
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Target {
    /// Type of target
    pub target_type: TargetType,

    /// Identifier of the target
    pub identifier: String,

    /// Constraints on this target
    pub constraints: Vec<String>,
}

impl Target {
    /// Create a new target
    pub fn new(target_type: TargetType, identifier: impl Into<String>) -> Self {
        Self {
            target_type,
            identifier: identifier.into(),
            constraints: Vec::new(),
        }
    }

    /// Add constraint
    pub fn with_constraint(mut self, constraint: impl Into<String>) -> Self {
        self.constraints.push(constraint.into());
        self
    }
}

/// Type of target
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetType {
    /// An identity (person, agent)
    Identity,
    /// A resource (file, database, etc.)
    Resource,
    /// A system
    System,
    /// A location
    Location,
    /// Custom type
    Custom,
}

/// Evidence requirements for a commitment
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRequirements {
    /// Required evidence types
    pub required_evidence_types: Vec<EvidenceType>,

    /// Retention period in seconds
    pub retention_period_secs: u64,

    /// Audit level
    pub audit_level: AuditLevel,
}

impl EvidenceRequirements {
    /// Create standard evidence requirements
    pub fn standard() -> Self {
        Self {
            required_evidence_types: vec![EvidenceType::ExecutionLog],
            retention_period_secs: 86400 * 30, // 30 days
            audit_level: AuditLevel::Standard,
        }
    }

    /// Create minimal evidence requirements
    pub fn minimal() -> Self {
        Self {
            required_evidence_types: vec![EvidenceType::ExecutionLog],
            retention_period_secs: 86400, // 1 day
            audit_level: AuditLevel::Minimal,
        }
    }

    /// Create comprehensive evidence requirements
    pub fn comprehensive() -> Self {
        Self {
            required_evidence_types: vec![
                EvidenceType::ExecutionLog,
                EvidenceType::StateSnapshot,
                EvidenceType::ExternalAttestation,
            ],
            retention_period_secs: 86400 * 365, // 1 year
            audit_level: AuditLevel::Comprehensive,
        }
    }

    /// Create forensic evidence requirements
    pub fn forensic() -> Self {
        Self {
            required_evidence_types: vec![
                EvidenceType::ExecutionLog,
                EvidenceType::StateSnapshot,
                EvidenceType::ExternalAttestation,
                EvidenceType::HumanVerification,
            ],
            retention_period_secs: 86400 * 365 * 7, // 7 years
            audit_level: AuditLevel::Forensic,
        }
    }

    /// Add required evidence type
    pub fn with_evidence_type(mut self, evidence_type: EvidenceType) -> Self {
        if !self.required_evidence_types.contains(&evidence_type) {
            self.required_evidence_types.push(evidence_type);
        }
        self
    }

    /// Set retention period
    pub fn with_retention_period_secs(mut self, secs: u64) -> Self {
        self.retention_period_secs = secs;
        self
    }

    /// Set audit level
    pub fn with_audit_level(mut self, level: AuditLevel) -> Self {
        self.audit_level = level;
        self
    }
}

/// Type of evidence
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    /// Log of execution steps
    ExecutionLog,
    /// Snapshot of state
    StateSnapshot,
    /// Attestation from external system
    ExternalAttestation,
    /// Human verification
    HumanVerification,
}

/// Audit level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditLevel {
    /// Minimal auditing
    Minimal,
    /// Standard auditing
    Standard,
    /// Comprehensive auditing
    Comprehensive,
    /// Forensic-level auditing
    Forensic,
}

/// Audit metadata
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuditMetadata {
    /// When the commitment was created
    pub created_at: TemporalAnchor,

    /// Who created the commitment
    pub created_by: IdentityRef,

    /// Trace ID for distributed tracing
    pub trace_id: String,
}

impl AuditMetadata {
    /// Create new audit metadata
    pub fn new(created_by: IdentityRef) -> Self {
        Self {
            created_at: TemporalAnchor::now(),
            created_by,
            trace_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

/// Human co-signature requirement
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HumanCosignRequirement {
    /// Number of required human signers
    pub required_signers: u32,

    /// Constraints on who can sign
    pub signer_constraints: Vec<SignerConstraint>,

    /// Timeout for signatures in seconds
    pub timeout_secs: u64,
}

impl HumanCosignRequirement {
    /// Create a new requirement
    pub fn new(required_signers: u32) -> Self {
        Self {
            required_signers,
            signer_constraints: Vec::new(),
            timeout_secs: 3600, // 1 hour default
        }
    }

    /// Add signer constraint
    pub fn with_constraint(mut self, constraint: SignerConstraint) -> Self {
        self.signer_constraints.push(constraint);
        self
    }

    /// Set timeout
    pub fn with_timeout_secs(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

/// Constraint on who can co-sign
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SignerConstraint {
    /// Role requirement (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Identity pattern (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity_pattern: Option<String>,
}

impl SignerConstraint {
    /// Create a new constraint
    pub fn new() -> Self {
        Self {
            role: None,
            identity_pattern: None,
        }
    }

    /// Require a role
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    /// Require identity pattern
    pub fn with_identity_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.identity_pattern = Some(pattern.into());
        self
    }
}

impl Default for SignerConstraint {
    fn default() -> Self {
        Self::new()
    }
}

/// Risk classification
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskClassification {
    /// Low risk
    Low,
    /// Medium risk
    Medium,
    /// High risk
    High,
    /// Critical risk (requires human review)
    Critical,
}

impl Default for RiskClassification {
    fn default() -> Self {
        RiskClassification::Medium
    }
}

/// Validation error for Commitment artifacts
#[derive(Debug, thiserror::Error)]
pub enum CommitmentValidationError {
    /// Wrong resonance type
    #[error("Wrong resonance type: expected Commitment, got {0}")]
    WrongResonanceType(ResonanceType),

    /// No capabilities specified
    #[error("Commitment must specify at least one capability")]
    NoCapabilities,

    /// Invalid temporal bounds
    #[error("Invalid temporal bounds: effective_from ({effective_from}) must be before expires_at ({expires_at})")]
    InvalidTemporalBounds {
        effective_from: chrono::DateTime<chrono::Utc>,
        expires_at: chrono::DateTime<chrono::Utc>,
    },

    /// Hash mismatch
    #[error("Declaration hash does not match content")]
    HashMismatch,

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingRequiredField(String),
}

impl From<CommitmentValidationError> for rcl_types::RclError {
    fn from(err: CommitmentValidationError) -> Self {
        rcl_types::RclError::ValidationError(err.to_string())
    }
}
