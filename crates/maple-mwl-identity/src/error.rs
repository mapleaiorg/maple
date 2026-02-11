use thiserror::Error;

/// Errors from continuity chain operations.
#[derive(Error, Debug)]
pub enum ContinuityError {
    #[error("no active segment to end")]
    NoActiveSegment,

    #[error("segment already active (index {0})")]
    SegmentAlreadyActive(u32),

    #[error("chain integrity violation at segment {index}: {reason}")]
    IntegrityViolation { index: u32, reason: String },

    #[error("empty chain — no segments")]
    EmptyChain,
}

/// Errors from identity management operations.
#[derive(Error, Debug)]
pub enum IdentityError {
    #[error("worldline not found: {0}")]
    NotFound(String),

    #[error("worldline already exists")]
    AlreadyExists,

    #[error("continuity error: {0}")]
    Continuity(#[from] ContinuityError),

    #[error("worldline is not active (no current segment)")]
    NotActive,
}
