use serde::{Deserialize, Serialize};

/// Errors that can occur during WAF compilation.
#[derive(Clone, Debug, Serialize, Deserialize, thiserror::Error)]
pub enum CompilerError {
    /// General compilation failure with a descriptive message.
    #[error("compilation failed: {0}")]
    CompilationFailed(String),

    /// The compilation attempted an operation that violates sandbox constraints.
    #[error("sandbox violation: {0}")]
    SandboxViolation(String),

    /// The target substrate is not supported by the compiler.
    #[error("unsupported substrate: {0}")]
    UnsupportedSubstrate(String),

    /// Compilation exceeded the allowed time budget (in seconds).
    #[error("compilation timed out after {0}s")]
    Timeout(u64),

    /// A system resource (memory, disk, etc.) was exhausted.
    #[error("resource exhausted: {0}")]
    ResourceExhausted(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let e1 = CompilerError::CompilationFailed("syntax error".into());
        assert_eq!(format!("{e1}"), "compilation failed: syntax error");

        let e2 = CompilerError::SandboxViolation("filesystem access denied".into());
        assert!(format!("{e2}").contains("sandbox violation"));

        let e3 = CompilerError::Timeout(30);
        assert_eq!(format!("{e3}"), "compilation timed out after 30s");
    }

    #[test]
    fn error_serde_roundtrip() {
        let errors = vec![
            CompilerError::CompilationFailed("parse error".into()),
            CompilerError::UnsupportedSubstrate("FPGA".into()),
            CompilerError::ResourceExhausted("OOM at 8GB".into()),
            CompilerError::Timeout(60),
            CompilerError::SandboxViolation("net access".into()),
        ];
        for err in &errors {
            let json = serde_json::to_string(err).unwrap();
            let restored: CompilerError = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{err}"), format!("{restored}"));
        }
    }

    #[test]
    fn error_is_clone_and_debug() {
        let e = CompilerError::ResourceExhausted("disk full".into());
        let cloned = e.clone();
        assert_eq!(format!("{e:?}"), format!("{cloned:?}"));
    }
}
