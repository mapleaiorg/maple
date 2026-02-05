//! Mapleverse Executor - The execution engine
//!
//! The executor runs approved commitments through connectors.
//! It has NO decision-making authority - only AAS can approve executions.

#![deny(unsafe_code)]

use rcf_commitment::{CommitmentId, RcfCommitment, Reversibility};
use rcf_types::EffectDomain;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;

// ============================================================================
// Execution Types (local to this crate)
// ============================================================================

/// An execution request from AAS
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub request_id: ExecutionRequestId,
    pub commitment: RcfCommitment,
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

// ============================================================================
// Executor Implementation
// ============================================================================

/// The execution engine
pub struct Executor {
    executions: RwLock<HashMap<ExecutionRequestId, ExecutionResult>>,
    handlers: RwLock<HashMap<EffectDomain, Box<dyn ExecutionHandler + Send + Sync>>>,
}

impl Executor {
    /// Create a new executor
    pub fn new() -> Self {
        Self {
            executions: RwLock::new(HashMap::new()),
            handlers: RwLock::new(HashMap::new()),
        }
    }

    /// Register a handler for a domain
    pub fn register_handler<H: ExecutionHandler + Send + Sync + 'static>(
        &self,
        domain: EffectDomain,
        handler: H,
    ) -> Result<(), ExecutorError> {
        let mut handlers = self
            .handlers
            .write()
            .map_err(|_| ExecutorError::LockError)?;
        handlers.insert(domain, Box::new(handler));
        Ok(())
    }

    /// Execute an approved commitment
    pub fn execute(&self, request: ExecutionRequest) -> Result<ExecutionResult, ExecutorError> {
        let request_id = request.request_id.clone();
        let commitment_id = request.commitment.commitment_id.clone();
        let started_at = chrono::Utc::now();

        // Initialize result
        let mut result = ExecutionResult {
            request_id: request_id.clone(),
            commitment_id: commitment_id.clone(),
            status: ExecutionStatus::Running,
            consequence: None,
            started_at,
            completed_at: None,
            execution_trace: vec![ExecutionEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                event_type: ExecutionEventType::Started,
                description: "Execution started".to_string(),
                timestamp: started_at,
                data: None,
            }],
        };

        // Store initial state
        {
            let mut executions = self
                .executions
                .write()
                .map_err(|_| ExecutorError::LockError)?;
            executions.insert(request_id.clone(), result.clone());
        }

        // Get handler for domain
        let handlers = self.handlers.read().map_err(|_| ExecutorError::LockError)?;
        let handler = handlers
            .get(&request.commitment.effect_domain)
            .ok_or_else(|| ExecutorError::NoHandler(request.commitment.effect_domain.clone()))?;

        // Execute
        match handler.execute(&request.commitment, &request.execution_parameters) {
            Ok(consequence) => {
                result.status = ExecutionStatus::Completed;
                result.consequence = Some(consequence);
                result.completed_at = Some(chrono::Utc::now());
                result.execution_trace.push(ExecutionEvent {
                    event_id: uuid::Uuid::new_v4().to_string(),
                    event_type: ExecutionEventType::Completed,
                    description: "Execution completed successfully".to_string(),
                    timestamp: chrono::Utc::now(),
                    data: None,
                });
            }
            Err(e) => {
                result.status = ExecutionStatus::Failed(e.to_string());
                result.completed_at = Some(chrono::Utc::now());
                result.execution_trace.push(ExecutionEvent {
                    event_id: uuid::Uuid::new_v4().to_string(),
                    event_type: ExecutionEventType::Failed,
                    description: format!("Execution failed: {}", e),
                    timestamp: chrono::Utc::now(),
                    data: None,
                });

                // Rollback if configured
                if request.execution_parameters.rollback_on_failure {
                    self.rollback(&request.commitment)?;
                    result.status = ExecutionStatus::RolledBack;
                }
            }
        }

        // Update stored result
        {
            let mut executions = self
                .executions
                .write()
                .map_err(|_| ExecutorError::LockError)?;
            executions.insert(request_id, result.clone());
        }

        Ok(result)
    }

    /// Get execution status
    pub fn get_status(
        &self,
        request_id: &ExecutionRequestId,
    ) -> Result<Option<ExecutionResult>, ExecutorError> {
        let executions = self
            .executions
            .read()
            .map_err(|_| ExecutorError::LockError)?;
        Ok(executions.get(request_id).cloned())
    }

    /// Abort an execution
    pub fn abort(
        &self,
        request_id: &ExecutionRequestId,
        reason: &str,
    ) -> Result<(), ExecutorError> {
        let mut executions = self
            .executions
            .write()
            .map_err(|_| ExecutorError::LockError)?;

        if let Some(result) = executions.get_mut(request_id) {
            if !result.status.is_terminal() {
                result.status = ExecutionStatus::Aborted(reason.to_string());
                result.completed_at = Some(chrono::Utc::now());
                result.execution_trace.push(ExecutionEvent {
                    event_id: uuid::Uuid::new_v4().to_string(),
                    event_type: ExecutionEventType::Failed,
                    description: format!("Execution aborted: {}", reason),
                    timestamp: chrono::Utc::now(),
                    data: None,
                });
            }
        }

        Ok(())
    }

    /// Rollback an execution
    fn rollback(&self, commitment: &RcfCommitment) -> Result<(), ExecutorError> {
        let handlers = self.handlers.read().map_err(|_| ExecutorError::LockError)?;

        if let Some(handler) = handlers.get(&commitment.effect_domain) {
            handler.rollback(commitment)?;
        }

        Ok(())
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for execution handlers
pub trait ExecutionHandler {
    fn execute(
        &self,
        commitment: &RcfCommitment,
        params: &ExecutionParameters,
    ) -> Result<Consequence, ExecutorError>;

    fn rollback(&self, commitment: &RcfCommitment) -> Result<(), ExecutorError>;

    fn can_handle(&self, domain: &EffectDomain) -> bool;
}

/// A mock handler for testing
pub struct MockHandler {
    pub domain: EffectDomain,
    pub should_fail: bool,
}

impl MockHandler {
    pub fn new(domain: EffectDomain) -> Self {
        Self {
            domain,
            should_fail: false,
        }
    }
}

impl ExecutionHandler for MockHandler {
    fn execute(
        &self,
        commitment: &RcfCommitment,
        _params: &ExecutionParameters,
    ) -> Result<Consequence, ExecutorError> {
        if self.should_fail {
            return Err(ExecutorError::ExecutionFailed("Mock failure".to_string()));
        }

        Ok(Consequence {
            consequence_id: ConsequenceId::generate(),
            commitment_id: commitment.commitment_id.clone(),
            effect_domain: commitment.effect_domain.clone(),
            description: "Mock execution completed".to_string(),
            evidence: vec![Evidence {
                evidence_type: EvidenceType::Log,
                description: "Execution log".to_string(),
                data: vec![],
                timestamp: chrono::Utc::now(),
            }],
            occurred_at: chrono::Utc::now(),
            reversibility_status: match &commitment.reversibility {
                Reversibility::Reversible => ReversibilityStatus::Reversible,
                Reversibility::PartiallyReversible(_) => ReversibilityStatus::PartiallyReversible,
                Reversibility::Irreversible => ReversibilityStatus::Irreversible,
            },
        })
    }

    fn rollback(&self, _commitment: &RcfCommitment) -> Result<(), ExecutorError> {
        Ok(())
    }

    fn can_handle(&self, domain: &EffectDomain) -> bool {
        &self.domain == domain
    }
}

/// Executor errors
#[derive(Debug, Error)]
pub enum ExecutorError {
    #[error("No handler for domain: {0}")]
    NoHandler(EffectDomain),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Timeout")]
    Timeout,

    #[error("Already executing")]
    AlreadyExecuting,

    #[error("Rollback failed: {0}")]
    RollbackFailed(String),

    #[error("Lock error")]
    LockError,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcf_commitment::CommitmentBuilder;
    use rcf_types::{IdentityRef, ScopeConstraint};

    #[test]
    fn test_executor() {
        let executor = Executor::new();
        executor
            .register_handler(
                EffectDomain::Computation,
                MockHandler::new(EffectDomain::Computation),
            )
            .unwrap();

        let commitment =
            CommitmentBuilder::new(IdentityRef::new("test-agent"), EffectDomain::Computation)
                .with_scope(ScopeConstraint::default())
                .build()
                .unwrap();

        let request = ExecutionRequest {
            request_id: ExecutionRequestId::generate(),
            commitment,
            decision_id: "test-decision".to_string(),
            requested_at: chrono::Utc::now(),
            execution_parameters: ExecutionParameters::default(),
        };

        let result = executor.execute(request).unwrap();
        assert!(result.status.is_success());
        assert!(result.consequence.is_some());
    }
}
