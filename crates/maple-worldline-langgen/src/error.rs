//! Language generation error types.
//!
//! Covers all failure modes in the language generation pipeline:
//! domain analysis, grammar synthesis, type system design, semantic
//! mapping, parser generation, compiler generation, and evolution.

use thiserror::Error;

/// Errors that can occur during language generation.
#[derive(Debug, Error)]
pub enum LangGenError {
    /// Domain analysis failed.
    #[error("Domain analysis failed: {0}")]
    DomainAnalysisFailed(String),

    /// Grammar synthesis failed.
    #[error("Grammar synthesis failed: {0}")]
    GrammarSynthesisFailed(String),

    /// Type system design failed.
    #[error("Type system design failed: {0}")]
    TypeSystemDesignFailed(String),

    /// Semantic rule mapping failed.
    #[error("Semantic mapping failed: {0}")]
    SemanticMappingFailed(String),

    /// Parser generation failed.
    #[error("Parser generation failed: {0}")]
    ParserGenerationFailed(String),

    /// Compiler generation failed.
    #[error("Compiler generation failed: {0}")]
    CompilerGenerationFailed(String),

    /// Language evolution failed.
    #[error("Evolution failed: {0}")]
    EvolutionFailed(String),

    /// Governance check failed (always Tier4SubstrateChange).
    #[error("Governance rejected: {0}")]
    GovernanceRejected(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Invalid language specification.
    #[error("Invalid specification: {0}")]
    InvalidSpecification(String),
}

/// Result type for language generation operations.
pub type LangGenResult<T> = Result<T, LangGenError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let e = LangGenError::DomainAnalysisFailed("no patterns".into());
        assert!(e.to_string().contains("no patterns"));

        let e = LangGenError::GovernanceRejected("tier4 required".into());
        assert!(e.to_string().contains("tier4 required"));
    }

    #[test]
    fn all_error_variants_display() {
        let errors: Vec<LangGenError> = vec![
            LangGenError::DomainAnalysisFailed("a".into()),
            LangGenError::GrammarSynthesisFailed("b".into()),
            LangGenError::TypeSystemDesignFailed("c".into()),
            LangGenError::SemanticMappingFailed("d".into()),
            LangGenError::ParserGenerationFailed("e".into()),
            LangGenError::CompilerGenerationFailed("f".into()),
            LangGenError::EvolutionFailed("g".into()),
            LangGenError::GovernanceRejected("h".into()),
            LangGenError::ConfigurationError("i".into()),
            LangGenError::InvalidSpecification("j".into()),
        ];
        for error in &errors {
            assert!(!error.to_string().is_empty());
        }
        assert_eq!(errors.len(), 10);
    }

    #[test]
    fn error_is_std_error() {
        let e: Box<dyn std::error::Error> =
            Box::new(LangGenError::TypeSystemDesignFailed("bad coercion".into()));
        assert!(e.to_string().contains("bad coercion"));
    }

    #[test]
    fn result_type_works() {
        let ok: LangGenResult<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: LangGenResult<i32> = Err(LangGenError::EvolutionFailed("stale".into()));
        assert!(err.is_err());
    }

    #[test]
    fn error_debug_format() {
        let e = LangGenError::GrammarSynthesisFailed("keyword conflict".into());
        let debug = format!("{:?}", e);
        assert!(debug.contains("GrammarSynthesisFailed"));
    }
}
