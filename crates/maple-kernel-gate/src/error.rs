use maple_kernel_fabric::FabricError;
use maple_mwl_types::CommitmentId;
use thiserror::Error;

/// Errors from the Commitment Gate pipeline.
#[derive(Error, Debug)]
pub enum GateError {
    #[error("declaration validation failed: {0}")]
    InvalidDeclaration(String),

    #[error("identity verification failed for worldline: {0}")]
    IdentityVerificationFailed(String),

    #[error("continuity chain not intact for worldline: {0}")]
    ContinuityBroken(String),

    #[error("insufficient capabilities: {0}")]
    InsufficientCapabilities(String),

    #[error("policy denied commitment: {0}")]
    PolicyDenied(String),

    #[error("risk threshold exceeded: {0}")]
    RiskThresholdExceeded(String),

    #[error("co-signature required but not collected")]
    CoSignatureRequired,

    #[error("commitment not found: {0}")]
    CommitmentNotFound(CommitmentId),

    #[error("stage {stage} failed: {reason}")]
    StageFailed { stage: String, reason: String },

    #[error("ledger error: {0}")]
    Ledger(#[from] LedgerError),

    #[error("fabric error: {0}")]
    Fabric(#[from] FabricError),

    #[error("serialization error: {0}")]
    Serialization(String),
}

/// Errors specific to the Commitment Ledger.
#[derive(Error, Debug)]
pub enum LedgerError {
    #[error("commitment not found in ledger: {0}")]
    NotFound(CommitmentId),

    #[error("ledger immutability violation: cannot modify existing entry {0}")]
    ImmutabilityViolation(CommitmentId),

    #[error("duplicate commitment ID: {0}")]
    DuplicateEntry(CommitmentId),

    #[error("invalid lifecycle transition for {commitment_id}: {from_status} -> {to_event}")]
    InvalidLifecycleTransition {
        commitment_id: CommitmentId,
        from_status: String,
        to_event: String,
    },
}
