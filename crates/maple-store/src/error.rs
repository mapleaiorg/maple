//! Error types for the MAPLE local package store.

/// Errors that can occur during store operations.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// An I/O error occurred while reading or writing store files.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization failed.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// A blob with the expected digest was not found in the store.
    #[error("blob not found: {digest}")]
    BlobNotFound {
        /// The hex digest of the missing blob.
        digest: String,
    },

    /// A manifest for the given package/tag was not found.
    #[error("manifest not found: {name}:{tag}")]
    ManifestNotFound {
        /// Package name.
        name: String,
        /// Tag or version reference.
        tag: String,
    },

    /// The package index is corrupted or could not be parsed.
    #[error("index corruption: {0}")]
    IndexCorruption(String),
}
