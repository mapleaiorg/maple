//! Error types for the `maple-init` crate.

use std::path::PathBuf;

/// Errors that can occur during package initialisation.
#[derive(Debug, thiserror::Error)]
pub enum InitError {
    /// An I/O operation failed (creating directories, writing files).
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    /// The generated template could not be parsed back — indicates a bug
    /// in the template generator.
    #[error("Template generation produced invalid YAML: {0}")]
    InvalidTemplate(String),
}
