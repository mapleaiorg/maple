//! Error types for the code generation engine.

use thiserror::Error;

/// Errors that can occur during code generation.
#[derive(Debug, Error)]
pub enum CodegenError {
    /// Commitment not approved for code generation.
    #[error("commitment not approved: {0}")]
    CommitmentNotApproved(String),

    /// Commitment validation failed (missing provenance, etc.).
    #[error("commitment validation failed: {0}")]
    CommitmentValidationFailed(String),

    /// Code generation failed for a specific change spec.
    #[error("generation failed: {0}")]
    GenerationFailed(String),

    /// Sandbox compilation failed.
    #[error("compilation failed: {0}")]
    CompilationFailed(String),

    /// Test validation failed.
    #[error("test validation failed: {0}")]
    TestValidationFailed(String),

    /// Performance gate not met.
    #[error("performance gate failed: {0}")]
    PerformanceGateFailed(String),

    /// Safety check violation during codegen.
    #[error("safety violation: {0}")]
    SafetyViolation(String),

    /// Artifact assembly failed.
    #[error("artifact assembly failed: {0}")]
    ArtifactAssemblyFailed(String),

    /// Configuration error.
    #[error("configuration error: {0}")]
    ConfigurationError(String),
}

/// Convenience result type for codegen operations.
pub type CodegenResult<T> = Result<T, CodegenError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_commitment_not_approved() {
        let err = CodegenError::CommitmentNotApproved("decision is Denied".into());
        assert_eq!(
            err.to_string(),
            "commitment not approved: decision is Denied"
        );
    }

    #[test]
    fn error_display_generation_failed() {
        let err = CodegenError::GenerationFailed("LLM timeout".into());
        assert_eq!(err.to_string(), "generation failed: LLM timeout");
    }

    #[test]
    fn error_display_compilation_failed() {
        let err = CodegenError::CompilationFailed("type mismatch".into());
        assert_eq!(err.to_string(), "compilation failed: type mismatch");
    }

    #[test]
    fn error_display_test_validation() {
        let err = CodegenError::TestValidationFailed("2/5 tests failed".into());
        assert_eq!(err.to_string(), "test validation failed: 2/5 tests failed");
    }

    #[test]
    fn codegen_result_ok() {
        let r: CodegenResult<u32> = Ok(42);
        assert_eq!(r.unwrap(), 42);
    }
}
