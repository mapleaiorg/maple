//! Consequence Tracker for MAPLE Resonators
//!
//! This module implements consequence tracking for the Resonance Architecture.
//! Consequences are the observable effects that result from executed commitments.
//! This enforces Invariant #4: "Commitment precedes Consequence" - no consequence
//! can be recorded without a valid, active commitment.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    CONSEQUENCE TRACKER                          │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │   Commitment ──> ConsequenceRequest ──> Validation ──> Record   │
//! │        │                                      │                 │
//! │        v                                      v                 │
//! │   ContractEngine                      ConsequenceStore          │
//! │   (verifies active)                   (immutable audit)         │
//! │                                                                 │
//! │   ConsequenceReceipt ──> ReceiptHash ──> ImmutableLedger        │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Components
//!
//! - [`ConsequenceTracker`]: Main tracker orchestrating consequence recording
//! - [`ConsequenceStore`]: Immutable storage for consequences with audit trails
//! - [`ConsequenceReceipt`]: Cryptographic receipt proving consequence execution
//! - [`ConsequenceValidator`]: Validates commitment binding before recording
//!
//! # Invariant Enforcement
//!
//! - Invariant #4: Commitment must exist and be active before consequence
//! - Invariant #8: All failures are explicit and recorded

#![deny(unsafe_code)]

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use rcf_commitment::CommitmentId;
use resonator_commitment::{ContractEngine, ContractStatus, StoredContract};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Unique identifier for a consequence instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConsequenceId(pub String);

impl ConsequenceId {
    pub fn generate() -> Self {
        Self(format!("consequence-{}", uuid::Uuid::new_v4()))
    }

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for ConsequenceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type of consequence (effect domain).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsequenceType {
    /// Computation-only effect (no external side effects).
    Computation,
    /// Data modification effect.
    DataMutation,
    /// Financial transaction effect.
    Financial,
    /// Communication effect (messages, notifications).
    Communication,
    /// External system interaction.
    ExternalSystem,
    /// Custom effect type.
    Custom(String),
}

/// Severity of a consequence for risk assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConsequenceSeverity {
    /// No permanent effects, safe to retry.
    Negligible,
    /// Minor effects, easily reversible.
    Minor,
    /// Moderate effects, may require manual intervention to reverse.
    Moderate,
    /// Significant effects, difficult to reverse.
    Significant,
    /// Irreversible or critical effects.
    Critical,
}

/// Status of a consequence execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsequenceStatus {
    /// Consequence is pending execution.
    Pending,
    /// Consequence is currently executing.
    Executing,
    /// Consequence completed successfully.
    Succeeded,
    /// Consequence failed during execution.
    Failed { reason: String },
    /// Consequence was rolled back.
    RolledBack { reason: String },
    /// Consequence is in an unknown state (requires investigation).
    Unknown { details: String },
}

impl ConsequenceStatus {
    /// Check if consequence is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ConsequenceStatus::Succeeded
                | ConsequenceStatus::Failed { .. }
                | ConsequenceStatus::RolledBack { .. }
        )
    }

    /// Check if consequence succeeded.
    pub fn is_success(&self) -> bool {
        matches!(self, ConsequenceStatus::Succeeded)
    }
}

/// A consequence request before execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsequenceRequest {
    /// Unique request identifier.
    pub request_id: String,
    /// The commitment this consequence is bound to.
    pub commitment_id: CommitmentId,
    /// Type of consequence.
    pub consequence_type: ConsequenceType,
    /// Severity assessment.
    pub severity: ConsequenceSeverity,
    /// Description of the intended effect.
    pub description: String,
    /// The capability being invoked.
    pub capability_id: String,
    /// Parameters for the capability.
    pub parameters: serde_json::Value,
    /// When the request was created.
    pub requested_at: DateTime<Utc>,
    /// Requestor identity.
    pub requestor: String,
}

impl ConsequenceRequest {
    pub fn new(
        commitment_id: CommitmentId,
        capability_id: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            request_id: format!("req-{}", uuid::Uuid::new_v4()),
            commitment_id,
            consequence_type: ConsequenceType::Computation,
            severity: ConsequenceSeverity::Minor,
            description: String::new(),
            capability_id: capability_id.into(),
            parameters,
            requested_at: Utc::now(),
            requestor: String::new(),
        }
    }

    pub fn with_type(mut self, consequence_type: ConsequenceType) -> Self {
        self.consequence_type = consequence_type;
        self
    }

    pub fn with_severity(mut self, severity: ConsequenceSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_requestor(mut self, requestor: impl Into<String>) -> Self {
        self.requestor = requestor.into();
        self
    }
}

/// A recorded consequence with full audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedConsequence {
    /// Unique consequence identifier.
    pub id: ConsequenceId,
    /// The original request.
    pub request: ConsequenceRequest,
    /// Current status.
    pub status: ConsequenceStatus,
    /// When execution started.
    pub started_at: Option<DateTime<Utc>>,
    /// When execution completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Execution duration in milliseconds.
    pub duration_ms: Option<i64>,
    /// Result payload if successful.
    pub result: Option<serde_json::Value>,
    /// Error details if failed.
    pub error: Option<ConsequenceError>,
    /// The receipt proving execution.
    pub receipt: Option<ConsequenceReceipt>,
    /// Audit trail of status changes.
    pub audit_trail: Vec<ConsequenceAuditEntry>,
}

impl RecordedConsequence {
    fn new(request: ConsequenceRequest) -> Self {
        let id = ConsequenceId::generate();
        let now = Utc::now();
        Self {
            id: id.clone(),
            request,
            status: ConsequenceStatus::Pending,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            result: None,
            error: None,
            receipt: None,
            audit_trail: vec![ConsequenceAuditEntry {
                timestamp: now,
                action: AuditAction::Created,
                old_status: None,
                new_status: ConsequenceStatus::Pending,
                actor: None,
                details: None,
            }],
        }
    }

    fn record_start(&mut self) {
        let now = Utc::now();
        let old_status = self.status.clone();
        self.status = ConsequenceStatus::Executing;
        self.started_at = Some(now);
        self.audit_trail.push(ConsequenceAuditEntry {
            timestamp: now,
            action: AuditAction::Started,
            old_status: Some(old_status),
            new_status: self.status.clone(),
            actor: None,
            details: None,
        });
    }

    fn record_success(&mut self, result: serde_json::Value, receipt: ConsequenceReceipt) {
        let now = Utc::now();
        let old_status = self.status.clone();
        self.status = ConsequenceStatus::Succeeded;
        self.completed_at = Some(now);
        if let Some(started) = self.started_at {
            self.duration_ms = Some((now - started).num_milliseconds());
        }
        self.result = Some(result);
        self.receipt = Some(receipt);
        self.audit_trail.push(ConsequenceAuditEntry {
            timestamp: now,
            action: AuditAction::Completed,
            old_status: Some(old_status),
            new_status: self.status.clone(),
            actor: None,
            details: None,
        });
    }

    fn record_failure(&mut self, error: ConsequenceError) {
        let now = Utc::now();
        let old_status = self.status.clone();
        self.status = ConsequenceStatus::Failed {
            reason: error.message.clone(),
        };
        self.completed_at = Some(now);
        if let Some(started) = self.started_at {
            self.duration_ms = Some((now - started).num_milliseconds());
        }
        self.error = Some(error);
        self.audit_trail.push(ConsequenceAuditEntry {
            timestamp: now,
            action: AuditAction::Failed,
            old_status: Some(old_status),
            new_status: self.status.clone(),
            actor: None,
            details: None,
        });
    }
}

/// Audit trail entry for consequence lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsequenceAuditEntry {
    /// When this audit entry was created.
    pub timestamp: DateTime<Utc>,
    /// The action that occurred.
    pub action: AuditAction,
    /// Previous status.
    pub old_status: Option<ConsequenceStatus>,
    /// New status after action.
    pub new_status: ConsequenceStatus,
    /// Who performed the action.
    pub actor: Option<String>,
    /// Additional details.
    pub details: Option<String>,
}

/// Actions recorded in the audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    Created,
    Started,
    Completed,
    Failed,
    RolledBack,
    Investigated,
}

/// Error details for a failed consequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsequenceError {
    /// Error code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Whether this error is retryable.
    pub retryable: bool,
    /// Suggested remediation.
    pub remediation: Option<String>,
}

/// Cryptographic receipt proving consequence execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsequenceReceipt {
    /// Unique receipt identifier.
    pub receipt_id: String,
    /// The consequence this receipt is for.
    pub consequence_id: ConsequenceId,
    /// The commitment this is bound to.
    pub commitment_id: CommitmentId,
    /// SHA-256 hash of the execution payload.
    pub execution_hash: String,
    /// When the receipt was issued.
    pub issued_at: DateTime<Utc>,
    /// The capability that was executed.
    pub capability_id: String,
    /// Compact summary for audit purposes.
    pub summary: String,
}

impl ConsequenceReceipt {
    /// Compute deterministic hash of the receipt.
    pub fn compute_hash(&self) -> String {
        let payload = serde_json::json!({
            "consequence_id": self.consequence_id.0,
            "commitment_id": self.commitment_id.0,
            "capability_id": self.capability_id,
            "issued_at": self.issued_at.to_rfc3339(),
            "summary": self.summary,
        });

        let mut hasher = Sha256::new();
        hasher.update(payload.to_string().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Verify the receipt hash matches.
    pub fn verify(&self) -> bool {
        self.execution_hash == self.compute_hash()
    }
}

/// Validates that consequences are properly bound to commitments.
#[derive(Debug, Clone)]
pub struct ConsequenceValidator {
    /// Minimum severity that requires explicit validation.
    min_validation_severity: ConsequenceSeverity,
}

impl ConsequenceValidator {
    pub fn new() -> Self {
        Self {
            min_validation_severity: ConsequenceSeverity::Minor,
        }
    }

    pub fn with_min_severity(mut self, severity: ConsequenceSeverity) -> Self {
        self.min_validation_severity = severity;
        self
    }

    /// Validate a consequence request against its commitment.
    ///
    /// Enforces Invariant #4: Commitment must exist and be active.
    pub fn validate(
        &self,
        request: &ConsequenceRequest,
        contract: &StoredContract,
    ) -> Result<ValidationResult, ConsequenceTrackerError> {
        let mut issues = Vec::new();

        // Check contract status
        if !matches!(contract.status, ContractStatus::Active | ContractStatus::Executing) {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Error,
                code: "CONTRACT_NOT_ACTIVE".to_string(),
                message: format!(
                    "Contract {} is in state {:?}, must be Active or Executing",
                    request.commitment_id, contract.status
                ),
            });
        }

        // Check temporal validity
        if !contract.contract.is_valid_at(Utc::now()) {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Error,
                code: "CONTRACT_EXPIRED".to_string(),
                message: format!("Contract {} has expired", request.commitment_id),
            });
        }

        // Check capability binding
        let capability_ref = &request.capability_id;
        let has_capability = contract
            .contract
            .required_capabilities
            .iter()
            .any(|cap| cap.capability_id == *capability_ref);
        if !has_capability {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Error,
                code: "CAPABILITY_NOT_BOUND".to_string(),
                message: format!(
                    "Capability {} is not bound to contract {}",
                    capability_ref, request.commitment_id
                ),
            });
        }

        // Check severity threshold
        if request.severity >= self.min_validation_severity {
            // Additional validation for higher severity consequences
            if request.description.is_empty() {
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Warning,
                    code: "MISSING_DESCRIPTION".to_string(),
                    message: "High-severity consequences should have descriptions".to_string(),
                });
            }
        }

        let has_errors = issues.iter().any(|i| i.severity == IssueSeverity::Error);

        Ok(ValidationResult {
            valid: !has_errors,
            issues,
            validated_at: Utc::now(),
        })
    }
}

impl Default for ConsequenceValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of consequence validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the consequence is valid.
    pub valid: bool,
    /// List of issues found.
    pub issues: Vec<ValidationIssue>,
    /// When validation occurred.
    pub validated_at: DateTime<Utc>,
}

/// A validation issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Issue severity.
    pub severity: IssueSeverity,
    /// Issue code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Severity of a validation issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
}

/// Storage abstraction for consequences.
pub trait ConsequenceStore: Send + Sync {
    fn store(&self, consequence: RecordedConsequence) -> Result<(), ConsequenceTrackerError>;
    fn get(&self, id: &ConsequenceId) -> Result<Option<RecordedConsequence>, ConsequenceTrackerError>;
    fn update(&self, consequence: RecordedConsequence) -> Result<(), ConsequenceTrackerError>;
    fn list_by_commitment(
        &self,
        commitment_id: &CommitmentId,
    ) -> Result<Vec<RecordedConsequence>, ConsequenceTrackerError>;
    fn list_by_status(
        &self,
        status: &ConsequenceStatus,
    ) -> Result<Vec<RecordedConsequence>, ConsequenceTrackerError>;
    fn get_receipts_for_commitment(
        &self,
        commitment_id: &CommitmentId,
    ) -> Result<Vec<ConsequenceReceipt>, ConsequenceTrackerError>;
}

/// In-memory consequence store for development/testing.
#[derive(Default)]
pub struct InMemoryConsequenceStore {
    consequences: RwLock<HashMap<ConsequenceId, RecordedConsequence>>,
    by_commitment: RwLock<HashMap<CommitmentId, Vec<ConsequenceId>>>,
}

impl InMemoryConsequenceStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ConsequenceStore for InMemoryConsequenceStore {
    fn store(&self, consequence: RecordedConsequence) -> Result<(), ConsequenceTrackerError> {
        let mut consequences = self
            .consequences
            .write()
            .map_err(|_| ConsequenceTrackerError::StoreLockError)?;
        let mut by_commitment = self
            .by_commitment
            .write()
            .map_err(|_| ConsequenceTrackerError::StoreLockError)?;

        let id = consequence.id.clone();
        let commitment_id = consequence.request.commitment_id.clone();

        if consequences.contains_key(&id) {
            return Err(ConsequenceTrackerError::ConsequenceAlreadyExists(id.0));
        }

        consequences.insert(id.clone(), consequence);
        by_commitment
            .entry(commitment_id)
            .or_default()
            .push(id);

        Ok(())
    }

    fn get(&self, id: &ConsequenceId) -> Result<Option<RecordedConsequence>, ConsequenceTrackerError> {
        let consequences = self
            .consequences
            .read()
            .map_err(|_| ConsequenceTrackerError::StoreLockError)?;
        Ok(consequences.get(id).cloned())
    }

    fn update(&self, consequence: RecordedConsequence) -> Result<(), ConsequenceTrackerError> {
        let mut consequences = self
            .consequences
            .write()
            .map_err(|_| ConsequenceTrackerError::StoreLockError)?;

        if !consequences.contains_key(&consequence.id) {
            return Err(ConsequenceTrackerError::ConsequenceNotFound(
                consequence.id.0.clone(),
            ));
        }

        consequences.insert(consequence.id.clone(), consequence);
        Ok(())
    }

    fn list_by_commitment(
        &self,
        commitment_id: &CommitmentId,
    ) -> Result<Vec<RecordedConsequence>, ConsequenceTrackerError> {
        let consequences = self
            .consequences
            .read()
            .map_err(|_| ConsequenceTrackerError::StoreLockError)?;
        let by_commitment = self
            .by_commitment
            .read()
            .map_err(|_| ConsequenceTrackerError::StoreLockError)?;

        let ids = by_commitment.get(commitment_id).cloned().unwrap_or_default();
        Ok(ids
            .iter()
            .filter_map(|id| consequences.get(id).cloned())
            .collect())
    }

    fn list_by_status(
        &self,
        status: &ConsequenceStatus,
    ) -> Result<Vec<RecordedConsequence>, ConsequenceTrackerError> {
        let consequences = self
            .consequences
            .read()
            .map_err(|_| ConsequenceTrackerError::StoreLockError)?;
        Ok(consequences
            .values()
            .filter(|c| std::mem::discriminant(&c.status) == std::mem::discriminant(status))
            .cloned()
            .collect())
    }

    fn get_receipts_for_commitment(
        &self,
        commitment_id: &CommitmentId,
    ) -> Result<Vec<ConsequenceReceipt>, ConsequenceTrackerError> {
        let consequences = self.list_by_commitment(commitment_id)?;
        Ok(consequences
            .into_iter()
            .filter_map(|c| c.receipt)
            .collect())
    }
}

/// Configuration for the consequence tracker.
#[derive(Debug, Clone)]
pub struct ConsequenceTrackerConfig {
    /// Maximum consequences per commitment.
    pub max_consequences_per_commitment: usize,
    /// History retention limit.
    pub history_retention_limit: usize,
    /// Enable detailed audit logging.
    pub detailed_audit: bool,
}

impl Default for ConsequenceTrackerConfig {
    fn default() -> Self {
        Self {
            max_consequences_per_commitment: 1000,
            history_retention_limit: 10000,
            detailed_audit: true,
        }
    }
}

/// The main consequence tracker.
///
/// Orchestrates consequence recording with commitment validation,
/// cryptographic receipts, and immutable audit trails.
#[derive(Clone)]
pub struct ConsequenceTracker {
    /// Contract engine for commitment verification.
    contract_engine: Arc<dyn ContractEngine>,
    /// Consequence storage.
    store: Arc<dyn ConsequenceStore>,
    /// Validator for commitment binding.
    validator: ConsequenceValidator,
    /// Configuration.
    config: ConsequenceTrackerConfig,
    /// Recent events for monitoring.
    recent_events: Arc<RwLock<VecDeque<ConsequenceEvent>>>,
}

/// Events emitted by the consequence tracker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsequenceEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: ConsequenceEventType,
    pub consequence_id: Option<ConsequenceId>,
    pub commitment_id: CommitmentId,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsequenceEventType {
    RequestReceived,
    ValidationPassed,
    ValidationFailed,
    ExecutionStarted,
    ExecutionSucceeded,
    ExecutionFailed,
    ReceiptIssued,
}

impl ConsequenceTracker {
    /// Create a new consequence tracker.
    pub fn new(
        contract_engine: Arc<dyn ContractEngine>,
        store: Arc<dyn ConsequenceStore>,
    ) -> Self {
        Self {
            contract_engine,
            store,
            validator: ConsequenceValidator::default(),
            config: ConsequenceTrackerConfig::default(),
            recent_events: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(mut self, config: ConsequenceTrackerConfig) -> Self {
        self.config = config;
        self
    }

    /// Create with custom validator.
    pub fn with_validator(mut self, validator: ConsequenceValidator) -> Self {
        self.validator = validator;
        self
    }

    /// Record that a consequence is being requested.
    ///
    /// Validates Invariant #4: Commitment must exist and be active.
    pub fn request_consequence(
        &self,
        request: ConsequenceRequest,
    ) -> Result<RecordedConsequence, ConsequenceTrackerError> {
        self.emit_event(ConsequenceEvent {
            timestamp: Utc::now(),
            event_type: ConsequenceEventType::RequestReceived,
            consequence_id: None,
            commitment_id: request.commitment_id.clone(),
            details: Some(request.capability_id.clone()),
        });

        // Invariant #4: Commitment must exist and be active
        let contract = self
            .contract_engine
            .get_contract(&request.commitment_id)
            .map_err(|e| ConsequenceTrackerError::ContractEngineError(e.to_string()))?
            .ok_or_else(|| {
                ConsequenceTrackerError::CommitmentNotFound(request.commitment_id.0.clone())
            })?;

        // Validate the request
        let validation = self.validator.validate(&request, &contract)?;
        if !validation.valid {
            self.emit_event(ConsequenceEvent {
                timestamp: Utc::now(),
                event_type: ConsequenceEventType::ValidationFailed,
                consequence_id: None,
                commitment_id: request.commitment_id.clone(),
                details: Some(
                    validation
                        .issues
                        .iter()
                        .map(|i| i.message.clone())
                        .collect::<Vec<_>>()
                        .join("; "),
                ),
            });
            return Err(ConsequenceTrackerError::ValidationFailed(validation.issues));
        }

        self.emit_event(ConsequenceEvent {
            timestamp: Utc::now(),
            event_type: ConsequenceEventType::ValidationPassed,
            consequence_id: None,
            commitment_id: request.commitment_id.clone(),
            details: None,
        });

        // Check limits
        let existing = self.store.list_by_commitment(&request.commitment_id)?;
        if existing.len() >= self.config.max_consequences_per_commitment {
            return Err(ConsequenceTrackerError::LimitExceeded(
                "max consequences per commitment".to_string(),
            ));
        }

        // Create and store the consequence record
        let consequence = RecordedConsequence::new(request);
        self.store.store(consequence.clone())?;

        Ok(consequence)
    }

    /// Record that consequence execution has started.
    pub fn start_execution(
        &self,
        consequence_id: &ConsequenceId,
    ) -> Result<RecordedConsequence, ConsequenceTrackerError> {
        let mut consequence = self
            .store
            .get(consequence_id)?
            .ok_or_else(|| ConsequenceTrackerError::ConsequenceNotFound(consequence_id.0.clone()))?;

        if !matches!(consequence.status, ConsequenceStatus::Pending) {
            return Err(ConsequenceTrackerError::InvalidStateTransition {
                from: format!("{:?}", consequence.status),
                to: "Executing".to_string(),
            });
        }

        consequence.record_start();
        self.store.update(consequence.clone())?;

        self.emit_event(ConsequenceEvent {
            timestamp: Utc::now(),
            event_type: ConsequenceEventType::ExecutionStarted,
            consequence_id: Some(consequence_id.clone()),
            commitment_id: consequence.request.commitment_id.clone(),
            details: None,
        });

        Ok(consequence)
    }

    /// Record successful consequence execution.
    pub fn record_success(
        &self,
        consequence_id: &ConsequenceId,
        result: serde_json::Value,
        summary: impl Into<String>,
    ) -> Result<ConsequenceReceipt, ConsequenceTrackerError> {
        let mut consequence = self
            .store
            .get(consequence_id)?
            .ok_or_else(|| ConsequenceTrackerError::ConsequenceNotFound(consequence_id.0.clone()))?;

        if !matches!(consequence.status, ConsequenceStatus::Executing) {
            return Err(ConsequenceTrackerError::InvalidStateTransition {
                from: format!("{:?}", consequence.status),
                to: "Succeeded".to_string(),
            });
        }

        // Create cryptographic receipt
        let receipt = ConsequenceReceipt {
            receipt_id: format!("receipt-{}", uuid::Uuid::new_v4()),
            consequence_id: consequence_id.clone(),
            commitment_id: consequence.request.commitment_id.clone(),
            execution_hash: String::new(), // Will be computed
            issued_at: Utc::now(),
            capability_id: consequence.request.capability_id.clone(),
            summary: summary.into(),
        };

        // Compute the hash
        let mut receipt = receipt;
        receipt.execution_hash = receipt.compute_hash();

        consequence.record_success(result, receipt.clone());
        self.store.update(consequence.clone())?;

        self.emit_event(ConsequenceEvent {
            timestamp: Utc::now(),
            event_type: ConsequenceEventType::ExecutionSucceeded,
            consequence_id: Some(consequence_id.clone()),
            commitment_id: consequence.request.commitment_id.clone(),
            details: None,
        });

        self.emit_event(ConsequenceEvent {
            timestamp: Utc::now(),
            event_type: ConsequenceEventType::ReceiptIssued,
            consequence_id: Some(consequence_id.clone()),
            commitment_id: consequence.request.commitment_id.clone(),
            details: Some(receipt.receipt_id.clone()),
        });

        Ok(receipt)
    }

    /// Record failed consequence execution.
    pub fn record_failure(
        &self,
        consequence_id: &ConsequenceId,
        error: ConsequenceError,
    ) -> Result<RecordedConsequence, ConsequenceTrackerError> {
        let mut consequence = self
            .store
            .get(consequence_id)?
            .ok_or_else(|| ConsequenceTrackerError::ConsequenceNotFound(consequence_id.0.clone()))?;

        if !matches!(
            consequence.status,
            ConsequenceStatus::Pending | ConsequenceStatus::Executing
        ) {
            return Err(ConsequenceTrackerError::InvalidStateTransition {
                from: format!("{:?}", consequence.status),
                to: "Failed".to_string(),
            });
        }

        consequence.record_failure(error);
        self.store.update(consequence.clone())?;

        self.emit_event(ConsequenceEvent {
            timestamp: Utc::now(),
            event_type: ConsequenceEventType::ExecutionFailed,
            consequence_id: Some(consequence_id.clone()),
            commitment_id: consequence.request.commitment_id.clone(),
            details: consequence.error.as_ref().map(|e| e.message.clone()),
        });

        Ok(consequence)
    }

    /// Get a consequence by ID.
    pub fn get_consequence(
        &self,
        id: &ConsequenceId,
    ) -> Result<Option<RecordedConsequence>, ConsequenceTrackerError> {
        self.store.get(id)
    }

    /// Get all consequences for a commitment.
    pub fn get_consequences_for_commitment(
        &self,
        commitment_id: &CommitmentId,
    ) -> Result<Vec<RecordedConsequence>, ConsequenceTrackerError> {
        self.store.list_by_commitment(commitment_id)
    }

    /// Get all receipts for a commitment.
    pub fn get_receipts_for_commitment(
        &self,
        commitment_id: &CommitmentId,
    ) -> Result<Vec<ConsequenceReceipt>, ConsequenceTrackerError> {
        self.store.get_receipts_for_commitment(commitment_id)
    }

    /// Get recent events for monitoring.
    pub fn recent_events(&self, limit: usize) -> Vec<ConsequenceEvent> {
        self.recent_events
            .read()
            .map(|guard| guard.iter().rev().take(limit).cloned().collect())
            .unwrap_or_default()
    }

    fn emit_event(&self, event: ConsequenceEvent) {
        if let Ok(mut guard) = self.recent_events.write() {
            guard.push_back(event);
            while guard.len() > self.config.history_retention_limit {
                guard.pop_front();
            }
        }
    }
}

/// Consequence tracker errors.
#[derive(Debug, Error)]
pub enum ConsequenceTrackerError {
    #[error("Commitment not found: {0}")]
    CommitmentNotFound(String),

    #[error("Consequence not found: {0}")]
    ConsequenceNotFound(String),

    #[error("Consequence already exists: {0}")]
    ConsequenceAlreadyExists(String),

    #[error("Validation failed: {0:?}")]
    ValidationFailed(Vec<ValidationIssue>),

    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Contract engine error: {0}")]
    ContractEngineError(String),

    #[error("Store lock error")]
    StoreLockError,

    #[error("Limit exceeded: {0}")]
    LimitExceeded(String),

    #[error("Receipt verification failed")]
    ReceiptVerificationFailed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcf_commitment::CommitmentBuilder;
    use rcf_types::{CapabilityRef, EffectDomain, IdentityRef, ScopeConstraint, TemporalValidity};
    use resonator_commitment::InMemoryContractEngine;

    fn make_contract_with_capability(capability_id: &str) -> rcf_commitment::RcfCommitment {
        CommitmentBuilder::new(IdentityRef::new("agent-a"), EffectDomain::Computation)
            .with_scope(ScopeConstraint::default())
            .with_capability(CapabilityRef {
                capability_id: capability_id.to_string(),
                domain: EffectDomain::Computation,
                scope: ScopeConstraint::default(),
                validity: TemporalValidity::unbounded(),
                issuer: IdentityRef::new("admin"),
            })
            .build()
            .unwrap()
    }

    #[test]
    fn test_consequence_request_validation() {
        let engine = Arc::new(InMemoryContractEngine::new());
        let store = Arc::new(InMemoryConsequenceStore::new());
        let tracker = ConsequenceTracker::new(engine.clone(), store);

        // Register a contract
        let contract = make_contract_with_capability("test_capability");
        let commitment_id = contract.commitment_id.clone();
        engine.register_contract(contract).unwrap();

        // Create a consequence request
        let request = ConsequenceRequest::new(
            commitment_id.clone(),
            "test_capability",
            serde_json::json!({"test": "data"}),
        )
        .with_description("Test consequence")
        .with_severity(ConsequenceSeverity::Minor);

        // Request should succeed
        let consequence = tracker.request_consequence(request).unwrap();
        assert!(matches!(consequence.status, ConsequenceStatus::Pending));
        assert_eq!(consequence.request.commitment_id, commitment_id);
    }

    #[test]
    fn test_consequence_lifecycle() {
        let engine = Arc::new(InMemoryContractEngine::new());
        let store = Arc::new(InMemoryConsequenceStore::new());
        let tracker = ConsequenceTracker::new(engine.clone(), store);

        // Register a contract
        let contract = make_contract_with_capability("test_capability");
        let commitment_id = contract.commitment_id.clone();
        engine.register_contract(contract).unwrap();

        // Create consequence
        let request = ConsequenceRequest::new(
            commitment_id,
            "test_capability",
            serde_json::json!({}),
        );
        let consequence = tracker.request_consequence(request).unwrap();
        let id = consequence.id.clone();

        // Start execution
        let consequence = tracker.start_execution(&id).unwrap();
        assert!(matches!(consequence.status, ConsequenceStatus::Executing));

        // Record success
        let receipt = tracker
            .record_success(&id, serde_json::json!({"result": "ok"}), "Test completed")
            .unwrap();

        assert!(receipt.verify());

        // Verify final state
        let final_consequence = tracker.get_consequence(&id).unwrap().unwrap();
        assert!(matches!(final_consequence.status, ConsequenceStatus::Succeeded));
        assert!(final_consequence.receipt.is_some());
        assert!(final_consequence.result.is_some());
    }

    #[test]
    fn test_consequence_failure() {
        let engine = Arc::new(InMemoryContractEngine::new());
        let store = Arc::new(InMemoryConsequenceStore::new());
        let tracker = ConsequenceTracker::new(engine.clone(), store);

        // Register a contract
        let contract = make_contract_with_capability("test_capability");
        let commitment_id = contract.commitment_id.clone();
        engine.register_contract(contract).unwrap();

        // Create consequence
        let request = ConsequenceRequest::new(
            commitment_id,
            "test_capability",
            serde_json::json!({}),
        );
        let consequence = tracker.request_consequence(request).unwrap();
        let id = consequence.id.clone();

        // Start and fail
        tracker.start_execution(&id).unwrap();
        let error = ConsequenceError {
            code: "TEST_ERROR".to_string(),
            message: "Test failure".to_string(),
            retryable: true,
            remediation: Some("Retry later".to_string()),
        };
        let consequence = tracker.record_failure(&id, error).unwrap();

        assert!(matches!(consequence.status, ConsequenceStatus::Failed { .. }));
        assert!(consequence.error.is_some());
    }

    #[test]
    fn test_invariant_4_commitment_precedes_consequence() {
        let engine = Arc::new(InMemoryContractEngine::new());
        let store = Arc::new(InMemoryConsequenceStore::new());
        let tracker = ConsequenceTracker::new(engine, store);

        // Try to create consequence without a contract
        let request = ConsequenceRequest::new(
            CommitmentId("nonexistent".to_string()),
            "test_capability",
            serde_json::json!({}),
        );

        let result = tracker.request_consequence(request);
        assert!(matches!(
            result,
            Err(ConsequenceTrackerError::CommitmentNotFound(_))
        ));
    }

    #[test]
    fn test_capability_binding_validation() {
        let engine = Arc::new(InMemoryContractEngine::new());
        let store = Arc::new(InMemoryConsequenceStore::new());
        let tracker = ConsequenceTracker::new(engine.clone(), store);

        // Register a contract with one capability
        let contract = make_contract_with_capability("allowed_capability");
        let commitment_id = contract.commitment_id.clone();
        engine.register_contract(contract).unwrap();

        // Try to use a different capability
        let request = ConsequenceRequest::new(
            commitment_id,
            "different_capability", // Not bound to the contract
            serde_json::json!({}),
        );

        let result = tracker.request_consequence(request);
        assert!(matches!(
            result,
            Err(ConsequenceTrackerError::ValidationFailed(_))
        ));
    }

    #[test]
    fn test_receipt_verification() {
        let receipt = ConsequenceReceipt {
            receipt_id: "test-receipt".to_string(),
            consequence_id: ConsequenceId::new("test-consequence"),
            commitment_id: CommitmentId("test-commitment".to_string()),
            execution_hash: String::new(),
            issued_at: Utc::now(),
            capability_id: "test_capability".to_string(),
            summary: "Test summary".to_string(),
        };

        let mut receipt = receipt;
        receipt.execution_hash = receipt.compute_hash();

        assert!(receipt.verify());

        // Tamper with receipt
        receipt.summary = "Modified summary".to_string();
        assert!(!receipt.verify());
    }

    #[test]
    fn test_events_emitted() {
        let engine = Arc::new(InMemoryContractEngine::new());
        let store = Arc::new(InMemoryConsequenceStore::new());
        let tracker = ConsequenceTracker::new(engine.clone(), store);

        // Register a contract
        let contract = make_contract_with_capability("test_capability");
        let commitment_id = contract.commitment_id.clone();
        engine.register_contract(contract).unwrap();

        // Create consequence
        let request = ConsequenceRequest::new(
            commitment_id,
            "test_capability",
            serde_json::json!({}),
        );
        let consequence = tracker.request_consequence(request).unwrap();
        let id = consequence.id.clone();

        tracker.start_execution(&id).unwrap();
        tracker
            .record_success(&id, serde_json::json!({}), "Done")
            .unwrap();

        // Check events
        let events = tracker.recent_events(10);
        assert!(events.len() >= 4); // Request, validation, start, success, receipt
    }
}
