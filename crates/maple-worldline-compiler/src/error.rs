//! Compiler error types.
//!
//! Covers all failure modes in the adaptive compilation pipeline:
//! module verification, optimization, code generation, strategy
//! selection, safety violations, and configuration.

use thiserror::Error;

/// Errors that can occur during adaptive compilation.
#[derive(Debug, Error)]
pub enum CompilerError {
    /// Module is not verified and cannot be compiled.
    #[error("Module not verified: {0}")]
    ModuleNotVerified(String),

    /// An optimization pass failed.
    #[error("Optimization failed: {0}")]
    OptimizationFailed(String),

    /// Code generation failed.
    #[error("Code generation failed: {0}")]
    CodeGenerationFailed(String),

    /// Invalid compilation strategy.
    #[error("Invalid strategy: {0}")]
    InvalidStrategy(String),

    /// Safety violation detected during compilation (I.COMPILE-2).
    #[error("Safety violation: {0}")]
    SafetyViolation(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

/// Result type for compiler operations.
pub type CompilerResult<T> = Result<T, CompilerError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let e = CompilerError::ModuleNotVerified("unverified".into());
        assert!(e.to_string().contains("unverified"));

        let e = CompilerError::SafetyViolation("commitment scope mismatch".into());
        assert!(e.to_string().contains("commitment scope mismatch"));
    }

    #[test]
    fn all_error_variants_display() {
        let errors: Vec<CompilerError> = vec![
            CompilerError::ModuleNotVerified("a".into()),
            CompilerError::OptimizationFailed("b".into()),
            CompilerError::CodeGenerationFailed("c".into()),
            CompilerError::InvalidStrategy("d".into()),
            CompilerError::SafetyViolation("e".into()),
            CompilerError::ConfigurationError("f".into()),
        ];
        for error in &errors {
            assert!(!error.to_string().is_empty());
        }
        assert_eq!(errors.len(), 6);
    }

    #[test]
    fn error_is_std_error() {
        let e: Box<dyn std::error::Error> =
            Box::new(CompilerError::OptimizationFailed("dead code".into()));
        assert!(e.to_string().contains("dead code"));
    }

    #[test]
    fn result_type_works() {
        let ok: CompilerResult<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: CompilerResult<i32> = Err(CompilerError::ConfigurationError("bad".into()));
        assert!(err.is_err());
    }

    #[test]
    fn error_debug_format() {
        let e = CompilerError::CodeGenerationFailed("target unsupported".into());
        let debug = format!("{:?}", e);
        assert!(debug.contains("CodeGenerationFailed"));
    }
}
