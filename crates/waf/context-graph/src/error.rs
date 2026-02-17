use crate::types::{ContentHash, NodeContentType};

/// Errors from Context Graph operations.
#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("validation error: {0}")]
    Validation(#[from] ValidationError),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("node not found: {0}")]
    NodeNotFound(ContentHash),
    #[error("incomplete evolution chain: missing {0}")]
    IncompleteChain(NodeContentType),
    #[error("worldline mismatch: expected {expected}, got {actual}")]
    WorldlineMismatch {
        expected: String,
        actual: String,
    },
}

/// Errors from node or chain validation.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("content hash mismatch: expected {expected}, computed {computed}")]
    HashMismatch {
        expected: ContentHash,
        computed: ContentHash,
    },
    #[error("signature verification failed")]
    SignatureFailed,
    #[error("dangling parent reference: {0}")]
    DanglingParent(ContentHash),
    #[error("causal chain incomplete: {0}")]
    CausalChainIncomplete(String),
    #[error("temporal ordering violated: {0}")]
    TemporalOrderViolated(String),
    #[error("missing required field: {0}")]
    MissingField(String),
}

/// Errors from storage backends.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("node already exists: {0}")]
    AlreadyExists(ContentHash),
    #[error("node not found: {0}")]
    NotFound(ContentHash),
    #[error("storage I/O error: {0}")]
    Io(String),
    #[error("serialization error: {0}")]
    Serialization(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_error_display() {
        let e = GraphError::NodeNotFound(ContentHash::zero());
        assert!(format!("{}", e).contains("not found"));
    }

    #[test]
    fn validation_error_display() {
        let e = ValidationError::SignatureFailed;
        assert!(format!("{}", e).contains("signature"));
    }

    #[test]
    fn storage_error_display() {
        let e = StorageError::Io("disk full".into());
        assert!(format!("{}", e).contains("disk full"));
    }

    #[test]
    fn graph_error_from_validation() {
        let ve = ValidationError::SignatureFailed;
        let ge: GraphError = ve.into();
        assert!(matches!(ge, GraphError::Validation(_)));
    }

    #[test]
    fn graph_error_from_storage() {
        let se = StorageError::Io("test".into());
        let ge: GraphError = se.into();
        assert!(matches!(ge, GraphError::Storage(_)));
    }
}
