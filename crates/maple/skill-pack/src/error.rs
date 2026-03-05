//! Error types for skill pack operations.

/// Errors from skill pack loading, validation, and registration.
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    /// Invalid manifest file.
    #[error("invalid skill manifest: {0}")]
    InvalidManifest(String),

    /// Invalid policy file.
    #[error("invalid skill policy: {0}")]
    InvalidPolicy(String),

    /// Golden trace error.
    #[error("golden trace error: {0}")]
    GoldenTrace(String),

    /// I/O error reading skill pack files.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Validation failed.
    #[error("validation failed: {0}")]
    ValidationFailed(String),

    /// Skill not found in registry.
    #[error("skill not found: {0}")]
    NotFound(String),

    /// Duplicate skill name in registry.
    #[error("duplicate skill name: {0}")]
    DuplicateName(String),

    /// Missing required file in skill pack directory.
    #[error("missing required file: {0}")]
    MissingFile(String),
}
