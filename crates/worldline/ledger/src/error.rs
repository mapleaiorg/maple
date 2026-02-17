use thiserror::Error;

/// Errors returned by canonical WorldLine ledger interfaces.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LedgerError {
    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("invalid range: from_seq {from} is greater than to_seq {to}")]
    InvalidRange { from: u64, to: u64 },

    #[error("receipt hash collision")]
    HashCollision,

    #[error("commitment receipt not found")]
    MissingCommitmentReceipt,

    #[error("commitment receipt must be accepted before outcomes can be appended")]
    CommitmentNotAccepted,

    #[error("commitment receipt must be rejected before a rejection outcome can be appended")]
    CommitmentNotRejected,

    #[error("snapshot anchor was not found for the worldline stream")]
    MissingSnapshotAnchor,

    #[error("stream integrity violation at seq {seq}: {reason}")]
    IntegrityViolation { seq: u64, reason: String },
}
