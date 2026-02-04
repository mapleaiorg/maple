use chrono::{DateTime, Utc};
use rcf_commitment::RcfCommitment;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Canonical transfer intent entering the iBank pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferIntent {
    pub trace_id: String,
    pub origin_actor: String,
    pub counterparty_actor: String,
    pub transaction_type: String,
    pub amount_minor: u64,
    pub currency: String,
    pub rail: String,
    pub destination: String,
    pub purpose: String,
    pub jurisdiction: String,
    /// 0..100 deterministic counterparty risk input from KYC/vendor feeds.
    pub counterparty_risk: u8,
    /// 0..100 anomaly/fraud score from deterministic detectors.
    pub anomaly_score: u8,
    /// 0.0..1.0 model uncertainty from cognition backend.
    pub model_uncertainty: f32,
    /// True for dispute/chargeback-like flows that require human review.
    pub dispute_flag: bool,
    /// 0.0..1.0 ambiguity score from upstream meaning formation.
    pub ambiguity: f32,
    pub compliance_flags: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

impl TransferIntent {
    pub fn new(
        origin_actor: impl Into<String>,
        counterparty_actor: impl Into<String>,
        amount_minor: u64,
        currency: impl Into<String>,
        rail: impl Into<String>,
        destination: impl Into<String>,
        purpose: impl Into<String>,
    ) -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            origin_actor: origin_actor.into(),
            counterparty_actor: counterparty_actor.into(),
            transaction_type: "transfer".to_string(),
            amount_minor,
            currency: currency.into(),
            rail: rail.into(),
            destination: destination.into(),
            purpose: purpose.into(),
            jurisdiction: "unknown".to_string(),
            counterparty_risk: 0,
            anomaly_score: 0,
            model_uncertainty: 0.0,
            dispute_flag: false,
            ambiguity: 0.0,
            compliance_flags: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    pub fn with_transaction_type(
        mut self,
        transaction_type: impl Into<String>,
        dispute_flag: bool,
    ) -> Self {
        self.transaction_type = transaction_type.into();
        self.dispute_flag = dispute_flag;
        self
    }

    pub fn with_risk_inputs(
        mut self,
        jurisdiction: impl Into<String>,
        counterparty_risk: u8,
        anomaly_score: u8,
        model_uncertainty: f32,
    ) -> Self {
        self.jurisdiction = jurisdiction.into();
        self.counterparty_risk = counterparty_risk.min(100);
        self.anomaly_score = anomaly_score.min(100);
        self.model_uncertainty = model_uncertainty.clamp(0.0, 1.0);
        self
    }
}

/// Stage output: parsed meaning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeaningField {
    pub summary: String,
    pub inferred_action: String,
    pub ambiguity_notes: Vec<String>,
    pub ambiguity_score: f32,
    pub confidence: f32,
    pub formed_at: DateTime<Utc>,
}

pub type MeaningRecord = MeaningField;

/// Confidence profile for intent stabilization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceProfile {
    pub meaning_confidence: f32,
    pub model_confidence: f32,
    pub overall_confidence: f32,
    pub blocking_ambiguity: bool,
    pub notes: Vec<String>,
}

/// Stage output: stabilized intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentRecord {
    pub objective: String,
    pub rationale: String,
    pub confidence: ConfidenceProfile,
    pub stabilized_at: DateTime<Utc>,
}

/// Canonical transfer payload used inside accountable wire messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferPayload {
    pub from: String,
    pub to: String,
    pub amount_minor: u64,
    pub currency: String,
    pub destination: String,
    pub purpose: String,
}

/// Explicit reference to a commitment declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentReference {
    pub commitment_id: String,
    /// Hash anchor allows the wire payload to be audited against ledger state.
    pub commitment_hash: String,
}

/// Origin proof attached to every accountable wire message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginProof {
    pub key_id: String,
    pub nonce: String,
    pub signed_at: DateTime<Utc>,
    pub signature: String,
}

/// Audit witness attached to every accountable wire message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditWitness {
    pub entry_id: String,
    pub entry_hash: String,
    pub observed_at: DateTime<Utc>,
}

/// iBank accountable wire format.
///
/// Non-negotiable invariant: consequential messages must always include origin proof,
/// audit witness, and may include commitment reference (required by router for side effects).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountableWireMessage {
    pub message_id: String,
    pub trace_id: String,
    pub origin_actor: String,
    pub payload: TransferPayload,
    pub origin_proof: OriginProof,
    pub audit_witness: AuditWitness,
    pub commitment_ref: Option<CommitmentReference>,
}

impl AccountableWireMessage {
    pub fn message_id() -> String {
        Uuid::new_v4().to_string()
    }
}

/// Connector execution receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorReceipt {
    pub settlement_id: String,
    pub rail: String,
    pub settled_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Route result returned from successful side effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResult {
    pub connector: String,
    pub external_reference: String,
    pub settled_at: DateTime<Utc>,
}

/// Persisted consequence record for both success and failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsequenceRecord {
    pub success: bool,
    pub detail: String,
    pub route: Option<RouteResult>,
    pub occurred_at: DateTime<Utc>,
}

/// Risk factor breakdown used for explainability and deterministic replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskBreakdown {
    pub amount: u8,
    pub counterparty: u8,
    pub jurisdiction: u8,
    pub anomaly: u8,
    pub model_uncertainty: u8,
}

/// Explainable risk report from deterministic policy evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskReport {
    pub score: u8,
    pub reasons: Vec<String>,
    pub factors: RiskBreakdown,
    pub fraud_score: u8,
    pub blocking_ambiguity: bool,
    pub requires_hybrid: bool,
    pub denied: bool,
}

/// Human approval payload used by hybrid routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanApproval {
    pub approved: bool,
    pub approver_id: String,
    pub note: Option<String>,
    pub approved_at: DateTime<Utc>,
}

impl HumanApproval {
    pub fn approved_by(approver_id: impl Into<String>) -> Self {
        Self {
            approved: true,
            approver_id: approver_id.into(),
            note: None,
            approved_at: Utc::now(),
        }
    }
}

/// API/App request entering the single iBank orchestration entrypoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleRequest {
    pub trace_id: Option<String>,
    pub origin_actor: String,
    pub counterparty_actor: String,
    pub transaction_type: String,
    pub amount_minor: u64,
    pub currency: String,
    pub rail: String,
    pub destination: String,
    pub jurisdiction: String,
    pub user_intent: String,
    pub ambiguity_hint: Option<f32>,
    pub counterparty_risk: u8,
    pub anomaly_score: u8,
    pub model_uncertainty: f32,
    pub compliance_flags: Vec<String>,
    pub metadata: BTreeMap<String, String>,
    pub approval: Option<HumanApproval>,
}

impl HandleRequest {
    pub fn new(
        origin_actor: impl Into<String>,
        counterparty_actor: impl Into<String>,
        amount_minor: u64,
        currency: impl Into<String>,
        rail: impl Into<String>,
        destination: impl Into<String>,
        user_intent: impl Into<String>,
    ) -> Self {
        Self {
            trace_id: None,
            origin_actor: origin_actor.into(),
            counterparty_actor: counterparty_actor.into(),
            transaction_type: "transfer".to_string(),
            amount_minor,
            currency: currency.into(),
            rail: rail.into(),
            destination: destination.into(),
            jurisdiction: "unknown".to_string(),
            user_intent: user_intent.into(),
            ambiguity_hint: None,
            counterparty_risk: 0,
            anomaly_score: 0,
            model_uncertainty: 0.0,
            compliance_flags: Vec::new(),
            metadata: BTreeMap::new(),
            approval: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    PureAi,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HandleStatus {
    ExecutedAutonomous,
    ExecutedHybrid,
    PendingHumanApproval,
    Denied,
    Failed,
}

/// Unified orchestration response from `IBankEngine::handle`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleResponse {
    pub trace_id: String,
    pub commitment_id: Option<String>,
    pub status: HandleStatus,
    pub mode: Option<ExecutionMode>,
    pub decision_reason: String,
    pub meaning: Option<MeaningField>,
    pub intent: Option<IntentRecord>,
    pub risk_report: Option<RiskReport>,
    pub route: Option<RouteResult>,
}

/// Platform-specific regulatory/compliance commitment data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegulatoryComplianceData {
    pub status: String,
    pub required_checks: Vec<String>,
    pub proof_placeholders: Vec<String>,
}

/// Platform-specific risk snapshot embedded in commitment records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessmentData {
    pub score: u8,
    pub fraud_score: u8,
    pub reasons: Vec<String>,
}

/// iBank platform extension data persisted with every commitment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IBankPlatformCommitmentData {
    pub transaction_type: String,
    pub value: String,
    pub risk_assessment: RiskAssessmentData,
    pub regulatory_compliance: RegulatoryComplianceData,
    pub state_snapshot_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentScopeContext {
    pub action: String,
    pub resources: Vec<String>,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentParties {
    pub principal: String,
    pub counterparty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentTemporalBounds {
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
}

/// Full commitment record persisted to the append-only ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentRecord {
    pub commitment: RcfCommitment,
    pub scope: CommitmentScopeContext,
    pub parties: CommitmentParties,
    pub temporal_bounds: CommitmentTemporalBounds,
    pub reversibility: String,
    pub confidence_context: ConfidenceProfile,
    pub platform: IBankPlatformCommitmentData,
}
