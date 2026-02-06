//! CLI error types

use thiserror::Error;

/// CLI error type
#[derive(Error, Debug)]
pub enum CliError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Commitment error: {0}")]
    Commitment(String),

    #[error("Consequence error: {0}")]
    Consequence(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Conversation error: {0}")]
    Conversation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

/// CLI result type
pub type CliResult<T> = Result<T, CliError>;
