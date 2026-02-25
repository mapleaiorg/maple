//! AAS Types - The ONLY authority layer
#![deny(unsafe_code)]

use rcf_commitment::{CommitmentId, RcfCommitment};
use rcf_types::{EffectDomain, TemporalValidity};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);
impl AgentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}
impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capability {
    pub capability_id: String,
    pub domain: EffectDomain,
    pub scope: rcf_types::ScopeConstraint,
    pub validity: TemporalValidity,
    pub status: CapabilityStatus,
    pub issuer: AgentId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityStatus {
    Active,
    Suspended,
    Revoked,
    Expired,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyDecisionCard {
    pub decision_id: DecisionId,
    pub commitment_id: CommitmentId,
    pub decision: Decision,
    pub rationale: Rationale,
    pub risk_assessment: RiskAssessment,
    pub conditions: Vec<Condition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_expiration: Option<chrono::DateTime<chrono::Utc>>,
    pub decided_at: chrono::DateTime<chrono::Utc>,
    pub adjudicator: AdjudicatorInfo,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DecisionId(pub String);
impl DecisionId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    Approved,
    Denied,
    PendingHumanReview,
    PendingAdditionalInfo,
}
impl Decision {
    pub fn allows_execution(&self) -> bool {
        matches!(self, Decision::Approved)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rationale {
    pub summary: String,
    pub rule_references: Vec<RuleReference>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleReference {
    pub rule_id: String,
    pub rule_description: String,
    pub evaluation_result: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk: RiskLevel,
    pub risk_factors: Vec<RiskFactor>,
    pub mitigations: Vec<Mitigation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskFactor {
    pub name: String,
    pub description: String,
    pub severity: RiskLevel,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mitigation {
    pub description: String,
    pub applied: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Condition {
    pub condition_type: ConditionType,
    pub description: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionType {
    HumanApproval,
    AdditionalVerification,
    RateLimiting,
    Monitoring,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdjudicatorInfo {
    pub adjudicator_type: AdjudicatorType,
    pub adjudicator_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdjudicatorType {
    Automated,
    Human,
    Hybrid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub entry_id: LedgerEntryId,
    pub commitment: RcfCommitment,
    pub decision: PolicyDecisionCard,
    pub lifecycle: CommitmentLifecycle,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<CommitmentOutcome>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LedgerEntryId(pub String);
impl LedgerEntryId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitmentLifecycle {
    pub status: LifecycleStatus,
    pub declared_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjudicated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_started_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleStatus {
    Pending,
    Approved,
    Denied,
    Executing,
    Completed,
    Failed,
    Expired,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitmentOutcome {
    pub success: bool,
    pub description: String,
    pub completed_at: chrono::DateTime<chrono::Utc>,
}

/// Execution receipt persisted for replay and accountability.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolExecutionReceipt {
    pub receipt_id: String,
    pub tool_call_id: String,
    pub contract_id: CommitmentId,
    pub capability_id: String,
    pub hash: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub status: ToolReceiptStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolReceiptStatus {
    Succeeded,
    Failed,
}
