//! Commitment and consequence types

use super::ids::{CommitmentId, ResonatorId};
use super::temporal::TemporalAnchor;
use serde::{Deserialize, Serialize};

/// A commitment made by a Resonator
///
/// ARCHITECTURAL RULE: No consequence may occur without an explicit commitment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commitment {
    pub id: CommitmentId,
    pub resonator: ResonatorId,

    /// What is being committed to?
    pub content: CommitmentContent,

    /// When was this commitment made?
    pub created_at: TemporalAnchor,

    /// Current status
    pub status: CommitmentStatus,

    /// Audit trail (if required by profile)
    pub audit_trail: Option<AuditTrail>,

    /// Risk assessment (if required by profile)
    pub risk_assessment: Option<RiskAssessment>,
}

/// What is the Resonator committing to?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommitmentContent {
    /// Commitment to perform an action
    Action {
        description: String,
        reversible: bool,
    },

    /// Commitment to maintain a state
    State {
        description: String,
        duration: Option<u64>,
    },

    /// Commitment to respect a boundary
    Boundary { description: String },

    /// Commitment to deliver a result
    Result {
        description: String,
        deadline: Option<TemporalAnchor>,
    },
}

/// Status of a commitment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommitmentStatus {
    /// Commitment is pending execution
    Pending,

    /// Commitment is currently being fulfilled
    Active,

    /// Commitment has been fulfilled
    Fulfilled,

    /// Commitment was violated
    Violated,

    /// Commitment was revoked (with consent)
    Revoked,
}

/// Audit trail for accountability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTrail {
    /// Events in the audit trail
    pub events: Vec<AuditEvent>,

    /// Digital signature (for non-repudiation)
    pub signature: Option<Vec<u8>>,
}

/// An event in the audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: TemporalAnchor,
    pub event_type: AuditEventType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    Created,
    Activated,
    Progress,
    Fulfilled,
    Violated,
    Revoked,
}

/// Risk assessment for financial operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Risk level (0.0 = no risk, 1.0 = maximum risk)
    pub risk_level: f64,

    /// Maximum potential impact
    pub max_impact: Option<MonetaryValue>,

    /// Mitigation strategies
    pub mitigations: Vec<String>,

    /// Approval required?
    pub requires_approval: bool,
}

/// Monetary value (for iBank)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonetaryValue {
    /// Amount in smallest unit (e.g., cents)
    pub amount: i64,

    /// Currency code (ISO 4217)
    pub currency: Currency,
}

impl MonetaryValue {
    pub fn new(amount: i64) -> Self {
        Self {
            amount,
            currency: Currency::USD,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Currency {
    USD,
    EUR,
    GBP,
    JPY,
    CNY,
}

/// Configuration for commitment management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentConfig {
    /// Require audit trail?
    pub require_audit_trail: bool,

    /// Require risk assessment?
    pub require_risk_assessment: bool,

    /// Maximum consequence value (for iBank)
    pub max_consequence_value: Option<MonetaryValue>,

    /// Enable commitment revocation?
    pub allow_revocation: bool,

    /// Require consent for revocation?
    pub require_consent_for_revocation: bool,
}

impl Default for CommitmentConfig {
    fn default() -> Self {
        Self {
            require_audit_trail: false,
            require_risk_assessment: false,
            max_consequence_value: None,
            allow_revocation: true,
            require_consent_for_revocation: true,
        }
    }
}

/// Consequence of a commitment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Consequence {
    /// Associated commitment
    pub commitment_id: CommitmentId,

    /// What happened?
    pub outcome: ConsequenceOutcome,

    /// When did it happen?
    pub occurred_at: TemporalAnchor,

    /// Was it reversible?
    pub reversible: bool,

    /// Reversal record (if reversed)
    pub reversal: Option<ReversalRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsequenceOutcome {
    Success {
        description: String,
        impact: Option<String>,
    },
    Failure {
        description: String,
        reason: String,
    },
    Partial {
        description: String,
        completion_rate: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReversalRecord {
    pub reversed_at: TemporalAnchor,
    pub reversal_method: String,
    pub successful: bool,
}
