//! Mapleverse Types - The execution layer types
//!
//! The Mapleverse is where approved commitments become actual effects.
//! It has NO cognitive capabilities - it only executes what AAS approves.

#![deny(unsafe_code)]

use aas_types::{AgentId, CommitmentOutcome};
use rcl_commitment::{CommitmentId, RclCommitment};
use rcl_types::EffectDomain;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An execution request from AAS
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub request_id: ExecutionRequestId,
    pub commitment: RclCommitment,
    pub decision_id: String,
    pub requested_at: chrono::DateTime<chrono::Utc>,
    pub execution_parameters: ExecutionParameters,
}

/// Unique identifier for an execution request
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecutionRequestId(pub String);

impl ExecutionRequestId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

/// Parameters for execution
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExecutionParameters {
    pub timeout_secs: Option<u64>,
    pub retry_policy: Option<RetryPolicy>,
    pub monitoring_level: MonitoringLevel,
    pub rollback_on_failure: bool,
    pub custom_params: HashMap<String, String>,
}

/// Retry policy for failed executions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub backoff_secs: u64,
    pub exponential_backoff: bool,
}

/// Level of monitoring during execution
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MonitoringLevel {
    #[default]
    Standard,
    Enhanced,
    Full,
}

/// Result of an execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub request_id: ExecutionRequestId,
    pub commitment_id: CommitmentId,
    pub status: ExecutionStatus,
    pub consequence: Option<Consequence>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub execution_trace: Vec<ExecutionEvent>,
}

impl ExecutionResult {
    pub fn to_outcome(&self) -> CommitmentOutcome {
        CommitmentOutcome {
            success: matches!(self.status, ExecutionStatus::Completed),
            description: self.status.description().to_string(),
            completed_at: self.completed_at.unwrap_or_else(chrono::Utc::now),
        }
    }
}

/// Status of an execution
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    TimedOut,
    Aborted(String),
    RolledBack,
}

impl ExecutionStatus {
    pub fn description(&self) -> &str {
        match self {
            ExecutionStatus::Pending => "Execution pending",
            ExecutionStatus::Running => "Execution in progress",
            ExecutionStatus::Completed => "Execution completed successfully",
            ExecutionStatus::Failed(msg) => msg,
            ExecutionStatus::TimedOut => "Execution timed out",
            ExecutionStatus::Aborted(msg) => msg,
            ExecutionStatus::RolledBack => "Execution rolled back",
        }
    }

    pub fn is_terminal(&self) -> bool {
        !matches!(self, ExecutionStatus::Pending | ExecutionStatus::Running)
    }

    pub fn is_success(&self) -> bool {
        matches!(self, ExecutionStatus::Completed)
    }
}

/// A consequence - the observable effect of an execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Consequence {
    pub consequence_id: ConsequenceId,
    pub commitment_id: CommitmentId,
    pub effect_domain: EffectDomain,
    pub description: String,
    pub evidence: Vec<Evidence>,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    pub reversibility_status: ReversibilityStatus,
}

/// Unique identifier for a consequence
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConsequenceId(pub String);

impl ConsequenceId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

/// Evidence of a consequence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evidence {
    pub evidence_type: EvidenceType,
    pub description: String,
    pub data: Vec<u8>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Types of evidence
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceType {
    Log,
    StateSnapshot,
    ExternalReceipt,
    Signature,
    Hash,
    Custom(String),
}

/// Reversibility status of a consequence
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReversibilityStatus {
    Reversible,
    PartiallyReversible,
    Irreversible,
    Reversed,
}

/// An event during execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionEvent {
    pub event_id: String,
    pub event_type: ExecutionEventType,
    pub description: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: Option<HashMap<String, String>>,
}

/// Types of execution events
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionEventType {
    Started,
    StepCompleted,
    StepFailed,
    CheckpointReached,
    RollbackInitiated,
    Completed,
    Failed,
}

/// A connector configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectorConfig {
    pub connector_id: String,
    pub connector_type: ConnectorType,
    pub domain: EffectDomain,
    pub config: HashMap<String, String>,
    pub enabled: bool,
}

/// Types of connectors
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectorType {
    FileSystem,
    Network,
    Database,
    Api,
    MessageQueue,
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_status() {
        assert!(!ExecutionStatus::Running.is_terminal());
        assert!(ExecutionStatus::Completed.is_terminal());
        assert!(ExecutionStatus::Completed.is_success());
        assert!(!ExecutionStatus::Failed("error".to_string()).is_success());
    }
}
