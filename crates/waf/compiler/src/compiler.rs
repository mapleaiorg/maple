use maple_waf_evolution_engine::{HardwareContext, Hypothesis};
use serde::{Deserialize, Serialize};

use crate::error::CompilerError;
use crate::sandbox::{CompilationSandbox, SimulatedSandbox};
use crate::strategy::CompilationStrategy;
use crate::types::{CompilationConfig, ExecutableArtifact};

/// Top-level WAF compiler.
///
/// Combines a [`CompilationSandbox`], [`CompilationStrategy`], and
/// [`CompilationConfig`] into a single entry point for compiling hypotheses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WafCompiler {
    /// The sandbox backend used for compilation.
    pub sandbox: SimulatedSandbox,
    /// Strategy for selecting compilation targets.
    pub strategy: CompilationStrategy,
    /// Compilation configuration.
    pub config: CompilationConfig,
}

impl WafCompiler {
    /// Create a new compiler with the given configuration.
    pub fn new(config: CompilationConfig) -> Self {
        Self {
            sandbox: SimulatedSandbox,
            strategy: CompilationStrategy,
            config,
        }
    }

    /// Create a compiler with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(CompilationConfig::default())
    }

    /// Compile a hypothesis for the given hardware context.
    ///
    /// The compilation target is determined automatically by
    /// [`CompilationStrategy::select_target`] based on the hardware
    /// capabilities and the hypothesis substrate.
    pub async fn compile(
        &self,
        hypothesis: &Hypothesis,
        hardware: &HardwareContext,
    ) -> Result<ExecutableArtifact, CompilerError> {
        let target = CompilationStrategy::select_target(hardware, &hypothesis.substrate);
        self.sandbox.compile(hypothesis, target).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_waf_context_graph::SubstrateType;

    fn simulated_hw() -> HardwareContext {
        HardwareContext::simulated()
    }

    #[tokio::test]
    async fn compiler_compiles_rust_hypothesis() {
        let compiler = WafCompiler::with_defaults();
        let h = Hypothesis::new(
            "h-compile-1",
            "add allocator",
            SubstrateType::Rust,
            "fn alloc() { /* optimised */ }",
        );
        let artifact = compiler.compile(&h, &simulated_hw()).await.unwrap();
        assert!(artifact.verify_hash());
        assert_eq!(artifact.substrate, SubstrateType::Rust);
    }

    #[tokio::test]
    async fn compiler_compiles_wasm_hypothesis() {
        let compiler = WafCompiler::new(CompilationConfig::wasm());
        let h = Hypothesis::new(
            "h-compile-2",
            "wasm module",
            SubstrateType::Wasm,
            "(module (func $f))",
        );
        let artifact = compiler.compile(&h, &simulated_hw()).await.unwrap();
        assert_eq!(
            artifact.metadata.get("target").unwrap(),
            "Wasm",
        );
    }

    #[tokio::test]
    async fn compiler_rejects_empty_code() {
        let compiler = WafCompiler::with_defaults();
        let h = Hypothesis::new("h-empty", "empty", SubstrateType::Rust, "");
        let result = compiler.compile(&h, &simulated_hw()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn compiler_strategy_respects_hardware() {
        let compiler = WafCompiler::with_defaults();
        let h = Hypothesis::new("h-hw", "test", SubstrateType::Rust, "code");

        // Powerful hardware -> Native
        let powerful = HardwareContext::simulated(); // 8 cores, 16384 MB
        let artifact = compiler.compile(&h, &powerful).await.unwrap();
        assert_eq!(artifact.metadata.get("target").unwrap(), "Native");

        // Weak hardware -> Wasm
        let weak = HardwareContext {
            cpu_cores: 2,
            memory_mb: 1024,
            ..Default::default()
        };
        let artifact = compiler.compile(&h, &weak).await.unwrap();
        assert_eq!(artifact.metadata.get("target").unwrap(), "Wasm");
    }
}
