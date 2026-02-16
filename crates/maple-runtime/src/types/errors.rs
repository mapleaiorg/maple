//! Error types for MAPLE Resonance Runtime

use super::coupling::CouplingValidationError;
use super::ids::ResonatorId;
use thiserror::Error;

/// Runtime bootstrap errors
#[derive(Debug, Error)]
pub enum BootstrapError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Invariant validation failed: {0}")]
    InvariantValidationFailed(String),
}

/// Runtime shutdown errors
#[derive(Debug, Error)]
pub enum ShutdownError {
    #[error("Pending commitments could not be resolved")]
    PendingCommitments,

    #[error("Failed to persist state: {0}")]
    PersistenceError(String),

    #[error("Timeout waiting for shutdown")]
    Timeout,
}

/// Resonator registration errors
#[derive(Debug, Error)]
pub enum RegistrationError {
    #[error("Invalid specification: {0}")]
    InvalidSpec(String),

    #[error("Profile validation failed: {0}")]
    ProfileValidation(String),

    #[error("Invariant violation: {0}")]
    InvariantViolation(String),

    #[error("Identity conflict: {0}")]
    IdentityConflict(String),
}

/// Resonator resume errors
#[derive(Debug, Error)]
pub enum ResumeError {
    #[error("Invalid continuity proof")]
    InvalidContinuityProof,

    #[error("Continuity record not found")]
    ContinuityRecordNotFound,

    #[error("State restoration failed: {0}")]
    StateRestorationFailed(String),

    #[error("Commitment reconciliation failed: {0}")]
    CommitmentReconciliationFailed(String),
}

/// Presence errors
#[derive(Debug, Error)]
pub enum PresenceError {
    #[error("Intrusive presence signal")]
    IntrusiveSignal,

    #[error("Resonator not found: {0}")]
    ResonatorNotFound(ResonatorId),

    #[error("Signal rate limit exceeded")]
    RateLimitExceeded,
}

/// Coupling errors
#[derive(Debug, Error)]
pub enum CouplingError {
    #[error("Insufficient attention (requested: {requested}, available: {available})")]
    InsufficientAttention { requested: u64, available: u64 },

    #[error("Initial coupling strength too aggressive")]
    TooAggressiveInitialStrength,

    #[error("Strengthening rate too rapid")]
    StrengtheningTooRapid,

    #[error("Coupling not found")]
    NotFound,

    #[error("Profile mismatch: {0}")]
    ProfileMismatch(String),

    #[error("Coupling validation failed: {0}")]
    ValidationFailed(String),
}

/// Attention allocation errors
#[derive(Debug, Error)]
pub enum AttentionError {
    #[error("Insufficient attention (requested: {requested}, available: {available})")]
    InsufficientAttention { requested: u64, available: u64 },

    #[error("Resonator not found")]
    ResonatorNotFound,

    #[error("Attention exhausted")]
    Exhausted,

    #[error("Invalid allocation amount")]
    InvalidAmount,
}

/// Invariant violation errors
#[derive(Debug, Error)]
pub enum InvariantViolation {
    #[error("Presence required before meaning formation")]
    PresenceRequired,

    #[error("Insufficient meaning for intent stabilization")]
    InsufficientMeaning,

    #[error("Intent not stabilized before commitment")]
    UnstabilizedIntent,

    #[error("No commitment for consequence")]
    NoCommitment,

    #[error("Attention capacity exceeded")]
    AttentionExceeded,

    #[error("Safety priority violated")]
    SafetyPriority,

    #[error("Human agency violation")]
    HumanAgencyViolation,

    #[error("Silent failure detected")]
    SilentFailure,

    #[error("Implementation provenance violation")]
    ImplementationProvenanceViolation,
}

/// Commitment errors
#[derive(Debug, Error)]
pub enum CommitmentError {
    #[error("Commitment not found")]
    NotFound,

    #[error("Invalid commitment state transition")]
    InvalidStateTransition,

    #[error("Commitment already fulfilled")]
    AlreadyFulfilled,

    #[error("Commitment violated")]
    Violated,

    #[error("Risk assessment required")]
    RiskAssessmentRequired,

    #[error("Audit trail required")]
    AuditTrailRequired,

    #[error("Approval required")]
    ApprovalRequired,
}

/// Consequence errors
#[derive(Debug, Error)]
pub enum ConsequenceError {
    #[error("No commitment for consequence")]
    NoCommitment,

    #[error("Consequence not reversible")]
    NotReversible,

    #[error("Reversal failed: {0}")]
    ReversalFailed(String),
}

/// Scheduling errors
#[derive(Debug, Error)]
pub enum SchedulingError {
    #[error("Circuit breaker open")]
    CircuitBreakerOpen,

    #[error("Attention unavailable")]
    AttentionUnavailable,

    #[error("Queue full")]
    QueueFull,

    #[error("Task rejected: {0}")]
    TaskRejected(String),
}

/// Temporal coordination errors
#[derive(Debug, Error)]
pub enum TemporalError {
    #[error("Causal ordering violation")]
    CausalOrderingViolation,

    #[error("Temporal anchor not found")]
    AnchorNotFound,

    #[error("Causal cycle detected")]
    CausalCycle,
}

// Conversions between error types
impl From<AttentionError> for CouplingError {
    fn from(err: AttentionError) -> Self {
        match err {
            AttentionError::InsufficientAttention {
                requested,
                available,
            } => CouplingError::InsufficientAttention {
                requested,
                available,
            },
            _ => CouplingError::ValidationFailed(err.to_string()),
        }
    }
}

impl From<CouplingValidationError> for CouplingError {
    fn from(err: CouplingValidationError) -> Self {
        CouplingError::ValidationFailed(err.to_string())
    }
}
