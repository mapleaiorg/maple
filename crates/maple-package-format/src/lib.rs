pub mod content_hash;
pub mod layout;

pub use content_hash::*;
pub use layout::*;

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum PackageFormatError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid digest format: {0}")]
    InvalidDigest(String),
    #[error("Tar error: {0}")]
    Tar(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Not a directory: {0}")]
    NotADirectory(PathBuf),
}
