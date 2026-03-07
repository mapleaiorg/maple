//! Error types for the fleet stack crate.

use thiserror::Error;

/// Errors that can occur during stack operations.
#[derive(Debug, Error)]
pub enum StackError {
    /// Stack definition YAML parsing failed.
    #[error("failed to parse stack definition: {0}")]
    ParseError(String),

    /// Stack definition validation failed.
    #[error("stack validation error: {0}")]
    ValidationError(String),

    /// A circular dependency was detected among services.
    #[error("circular dependency detected: {cycle}")]
    CircularDependency {
        /// Human-readable description of the dependency cycle.
        cycle: String,
    },

    /// A service referenced in `depends_on` does not exist.
    #[error("unknown dependency: service '{service}' depends on '{dependency}' which does not exist")]
    UnknownDependency {
        /// The service that has the broken dependency.
        service: String,
        /// The dependency target that was not found.
        dependency: String,
    },

    /// Duplicate service names in a stack definition.
    #[error("duplicate service name: '{name}'")]
    DuplicateServiceName {
        /// The duplicated name.
        name: String,
    },

    /// A lifecycle operation failed.
    #[error("lifecycle error: {0}")]
    LifecycleError(String),

    /// The stack is not in the expected state for the requested operation.
    #[error("invalid state transition: cannot {operation} stack in state {current_state}")]
    InvalidStateTransition {
        /// The operation that was attempted.
        operation: String,
        /// The current state of the stack.
        current_state: String,
    },

    /// A service instance failed to start.
    #[error("instance error for service '{service}': {reason}")]
    InstanceError {
        /// The service whose instance failed.
        service: String,
        /// The failure reason.
        reason: String,
    },

    /// YAML serialization/deserialization error.
    #[error("YAML error: {0}")]
    YamlError(#[from] serde_yaml::Error),
}

/// Result alias for stack operations.
pub type StackResult<T> = Result<T, StackError>;
