//! CLI error types

use thiserror::Error;

/// CLI error types
#[derive(Debug, Error)]
pub enum CliError {
    /// HTTP request error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// API error response
    #[error("API error: {status} - {message}")]
    Api {
        /// HTTP status code
        status: u16,
        /// Error message
        message: String,
    },

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// YAML parsing error
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// Result type for CLI operations
pub type CliResult<T> = Result<T, CliError>;
