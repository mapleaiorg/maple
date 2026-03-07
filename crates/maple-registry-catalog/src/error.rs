//! Error types for the catalog crate.

/// Errors produced by catalog and discovery operations.
#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    /// An error from the underlying OCI registry client.
    #[error("registry error: {0}")]
    Registry(#[from] maple_registry_client::RegistryError),

    /// The requested package was not found in the catalog.
    #[error("package not found: {0}")]
    NotFound(String),

    /// An I/O error occurred (e.g. reading/writing the cache file).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A JSON serialization or deserialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
