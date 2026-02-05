use aas_types::{CommitmentOutcome, LifecycleStatus, PolicyDecisionCard};
use chrono::{DateTime, Utc};
use rcf_commitment::{CommitmentId, RcfCommitment};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Persistent commitment record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentRecord {
    pub commitment_id: CommitmentId,
    pub commitment: RcfCommitment,
    pub decision: PolicyDecisionCard,
    pub lifecycle_status: LifecycleStatus,
    pub outcome: Option<CommitmentOutcome>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Audit append payload. Hashes and sequencing are assigned by storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditAppend {
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub stage: String,
    pub success: bool,
    pub message: String,
    pub commitment_id: Option<CommitmentId>,
    #[serde(default)]
    pub payload: Value,
}

/// Persistent tamper-evident audit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub event_id: String,
    pub sequence: u64,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub stage: String,
    pub success: bool,
    pub message: String,
    pub commitment_id: Option<CommitmentId>,
    pub payload: Value,
    pub previous_hash: Option<String>,
    pub hash: String,
}

/// Persistent checkpoint for agent-kernel and runtime resume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCheckpoint {
    pub resonator_id: String,
    pub profile_name: String,
    pub state: String,
    pub active_commitments: Vec<String>,
    pub last_audit_event_id: Option<String>,
    pub metadata: Value,
    pub updated_at: DateTime<Utc>,
}

/// Dashboard/ops projection snapshots keyed by namespace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionSnapshot {
    pub namespace: String,
    pub key: String,
    pub schema_version: String,
    pub data: Value,
    pub snapshot_hash: String,
    pub created_at: DateTime<Utc>,
}

/// Semantic memory record for AI-assistive retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticRecord {
    pub namespace: String,
    pub record_id: String,
    pub embedding: Vec<f32>,
    pub content: String,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

/// Semantic query result with similarity score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticHit {
    pub record: SemanticRecord,
    pub score: f32,
}
