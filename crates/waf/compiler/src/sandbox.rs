use async_trait::async_trait;
use maple_waf_evolution_engine::Hypothesis;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::CompilerError;
use crate::types::{CompilationTarget, ExecutableArtifact};

/// Trait for sandboxed compilation backends.
///
/// Implementations must ensure that compilation is isolated from the host
/// environment.  The sandbox is the last defence before untrusted synthesised
/// code reaches an execution substrate.
#[async_trait]
pub trait CompilationSandbox: Send + Sync {
    /// Compile a hypothesis into an executable artifact for the given target.
    async fn compile(
        &self,
        hypothesis: &Hypothesis,
        target: CompilationTarget,
    ) -> Result<ExecutableArtifact, CompilerError>;
}

/// A simulated (in-process) sandbox used for testing and development.
///
/// Produces artifacts by hashing the hypothesis code bytes directly — no
/// real compilation takes place.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SimulatedSandbox;

#[async_trait]
impl CompilationSandbox for SimulatedSandbox {
    async fn compile(
        &self,
        hypothesis: &Hypothesis,
        target: CompilationTarget,
    ) -> Result<ExecutableArtifact, CompilerError> {
        let binary = hypothesis.code.as_bytes().to_vec();
        if binary.is_empty() {
            return Err(CompilerError::CompilationFailed(
                "empty hypothesis code".into(),
            ));
        }

        let mut metadata = HashMap::new();
        metadata.insert("hypothesis_id".into(), hypothesis.id.clone());
        metadata.insert("target".into(), format!("{target}"));
        metadata.insert("simulated".into(), "true".into());

        let compiled_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(ExecutableArtifact::new(
            hypothesis.substrate,
            binary,
            metadata,
            compiled_at_ms,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_waf_context_graph::SubstrateType;

    fn sample_hypothesis() -> Hypothesis {
        Hypothesis::new(
            "h-sandbox-1",
            "test compilation",
            SubstrateType::Rust,
            "fn main() {}",
        )
    }

    #[tokio::test]
    async fn simulated_sandbox_produces_artifact() {
        let sandbox = SimulatedSandbox;
        let h = sample_hypothesis();
        let artifact = sandbox
            .compile(&h, CompilationTarget::Native)
            .await
            .unwrap();
        assert!(artifact.verify_hash());
        assert_eq!(artifact.substrate, SubstrateType::Rust);
        assert_eq!(artifact.binary, b"fn main() {}");
    }

    #[tokio::test]
    async fn simulated_sandbox_metadata_contains_hypothesis_id() {
        let sandbox = SimulatedSandbox;
        let h = sample_hypothesis();
        let artifact = sandbox.compile(&h, CompilationTarget::Wasm).await.unwrap();
        assert_eq!(
            artifact.metadata.get("hypothesis_id").unwrap(),
            "h-sandbox-1"
        );
        assert_eq!(artifact.metadata.get("target").unwrap(), "Wasm");
        assert_eq!(artifact.metadata.get("simulated").unwrap(), "true");
    }

    #[tokio::test]
    async fn simulated_sandbox_empty_code_fails() {
        let sandbox = SimulatedSandbox;
        let h = Hypothesis::new("h-empty", "empty", SubstrateType::Wasm, "");
        let result = sandbox.compile(&h, CompilationTarget::Wasm).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(format!("{err}").contains("empty hypothesis code"));
    }

    #[tokio::test]
    async fn simulated_sandbox_compiled_at_is_recent() {
        let before_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let sandbox = SimulatedSandbox;
        let h = sample_hypothesis();
        let artifact = sandbox
            .compile(&h, CompilationTarget::Native)
            .await
            .unwrap();

        let after_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        assert!(artifact.compiled_at_ms >= before_ms);
        assert!(artifact.compiled_at_ms <= after_ms);
    }
}
