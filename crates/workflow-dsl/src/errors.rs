//! DSL error types

/// Errors that can occur during DSL parsing, validation, or compilation
#[derive(Debug, thiserror::Error)]
pub enum DslError {
    #[error("Parse error at line {line}, column {col}: {message}")]
    ParseError {
        line: usize,
        col: usize,
        message: String,
    },

    #[error("Unexpected token: expected {expected}, found '{found}'")]
    UnexpectedToken { expected: String, found: String },

    #[error("Unexpected end of input: expected {0}")]
    UnexpectedEof(String),

    #[error("Unknown keyword: '{0}'")]
    UnknownKeyword(String),

    #[error("Unknown node type: '{0}'")]
    UnknownNodeType(String),

    #[error("Unknown receipt type: '{0}'")]
    UnknownReceiptType(String),

    #[error("Unknown escalation action: '{0}'")]
    UnknownEscalationAction(String),

    #[error("Unknown gate type: '{0}'")]
    UnknownGateType(String),

    #[error("Duplicate node ID: '{0}'")]
    DuplicateNodeId(String),

    #[error("Duplicate role ID: '{0}'")]
    DuplicateRoleId(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid value for '{field}': {message}")]
    InvalidValue { field: String, message: String },

    #[error("Compilation error: {0}")]
    CompilationError(String),

    #[error("Workflow error: {0}")]
    WorkflowError(#[from] workflow_types::WorkflowError),
}

/// Result type alias for DSL operations
pub type DslResult<T> = Result<T, DslError>;
