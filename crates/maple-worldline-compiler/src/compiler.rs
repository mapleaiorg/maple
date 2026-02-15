//! Adaptive compiler orchestrator.
//!
//! `AdaptiveCompiler` drives the full compilation pipeline:
//! verify module → select strategy → optimize → generate code → build artifact.
//! Supports both automatic strategy selection and explicit strategy override.

use crate::artifact::ArtifactBuilder;
use crate::codegen::{
    CodeGenerator, NativeCodeGen, OperatorCallCodeGen, WasmCodeGen, WlirInterpreterCodeGen,
};
use crate::error::{CompilerError, CompilerResult};
use crate::passes::OptimizationPipeline;
use crate::strategy::{CompilationStrategy, SimulatedStrategySelector, StrategySelector};
use crate::types::{CompilationId, CompilationTarget, CompilerConfig};
use maple_worldline_ir::module::WlirModule;
use maple_worldline_ir::types::VerificationStatus;

use crate::artifact::CompilationArtifact;

// ── Adaptive Compiler ───────────────────────────────────────────────

/// The adaptive compiler — orchestrates the full compilation pipeline.
///
/// Given a verified WLIR module, selects a strategy, runs optimization
/// passes, generates code for the target, and produces a `CompilationArtifact`.
pub struct AdaptiveCompiler {
    config: CompilerConfig,
    strategy_selector: Box<dyn StrategySelector>,
}

impl AdaptiveCompiler {
    /// Create a new adaptive compiler with default configuration.
    pub fn new() -> Self {
        Self {
            config: CompilerConfig::default(),
            strategy_selector: Box::new(SimulatedStrategySelector::new()),
        }
    }

    /// Create with a specific configuration.
    pub fn with_config(config: CompilerConfig) -> Self {
        Self {
            config,
            strategy_selector: Box::new(SimulatedStrategySelector::new()),
        }
    }

    /// Replace the strategy selector.
    pub fn with_strategy_selector(mut self, selector: Box<dyn StrategySelector>) -> Self {
        self.strategy_selector = selector;
        self
    }

    /// Access the current configuration.
    pub fn config(&self) -> &CompilerConfig {
        &self.config
    }

    /// Compile a verified WLIR module with automatic strategy selection.
    pub fn compile(&self, module: &WlirModule) -> CompilerResult<CompilationArtifact> {
        // Step 1: Verify module is verified
        self.verify_module(module)?;

        // Step 2: Select strategy
        let strategy = self.strategy_selector.select_strategy(module, &self.config);

        // Step 3–5: Compile with the selected strategy
        self.compile_inner(module, &strategy)
    }

    /// Compile a verified WLIR module with an explicit strategy.
    pub fn compile_with_strategy(
        &self,
        module: &WlirModule,
        strategy: &CompilationStrategy,
    ) -> CompilerResult<CompilationArtifact> {
        // Step 1: Verify module
        self.verify_module(module)?;

        // Steps 3–5: Use the provided strategy
        self.compile_inner(module, strategy)
    }

    /// Core compilation pipeline (steps 3–5).
    fn compile_inner(
        &self,
        module: &WlirModule,
        strategy: &CompilationStrategy,
    ) -> CompilerResult<CompilationArtifact> {
        let compilation_id = CompilationId::new();

        // Step 3: Build and run optimization pipeline
        let pipeline = OptimizationPipeline::from_strategy(strategy);
        let optimization_results = pipeline.apply_all(module)?;

        // Step 4: Select code generator based on strategy target
        let generator = self.generator_for_target(&strategy.target)?;
        let generated_code = generator.generate(module)?;

        // Step 5: Build artifact
        ArtifactBuilder::new(compilation_id)
            .module(module.id.clone(), module.name.clone())
            .strategy(strategy.clone())
            .optimization_results(optimization_results)
            .generated_code(generated_code)
            .build()
    }

    /// Verify that a module has been verified before compilation.
    fn verify_module(&self, module: &WlirModule) -> CompilerResult<()> {
        if self.config.enable_safety_checks
            && module.verification_status != VerificationStatus::FullyVerified
        {
            return Err(CompilerError::ModuleNotVerified(format!(
                "Module '{}' has verification status {:?}, expected Verified",
                module.name, module.verification_status
            )));
        }
        Ok(())
    }

    /// Get the appropriate code generator for a target.
    fn generator_for_target(
        &self,
        target: &CompilationTarget,
    ) -> CompilerResult<Box<dyn CodeGenerator>> {
        let gen: Box<dyn CodeGenerator> = match target {
            CompilationTarget::Native { arch } => {
                Box::new(NativeCodeGen::new(arch.clone()))
            }
            CompilationTarget::Wasm { env } => {
                Box::new(WasmCodeGen::new(env.clone()))
            }
            CompilationTarget::OperatorCall => {
                Box::new(OperatorCallCodeGen::new())
            }
            CompilationTarget::Interpreted => {
                Box::new(WlirInterpreterCodeGen::new())
            }
        };
        Ok(gen)
    }
}

impl Default for AdaptiveCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OptimizationLevel, TargetArch, WasmEnvironment};
    use maple_worldline_ir::instructions::WlirInstruction;
    use maple_worldline_ir::module::WlirFunction;
    use maple_worldline_ir::types::WlirType;

    fn verified_module() -> WlirModule {
        let mut module = WlirModule::new("test-mod", "1.0");
        let mut f = WlirFunction::new("main", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::Nop);
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);
        module.verification_status = VerificationStatus::FullyVerified;
        module
    }

    fn unverified_module() -> WlirModule {
        let mut module = WlirModule::new("unverified", "1.0");
        let f = WlirFunction::new("main", vec![], WlirType::Void);
        module.add_function(f);
        module
    }

    #[test]
    fn compile_verified_module() {
        let compiler = AdaptiveCompiler::new();
        let module = verified_module();
        let artifact = compiler.compile(&module).unwrap();
        assert_eq!(artifact.module_name, "test-mod");
        assert!(!artifact.generated_code.content.is_empty());
    }

    #[test]
    fn compile_rejects_unverified_module() {
        let compiler = AdaptiveCompiler::new();
        let module = unverified_module();
        let result = compiler.compile(&module);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not verified") || err.contains("Not verified"));
    }

    #[test]
    fn compile_with_safety_disabled_skips_verification() {
        let config = CompilerConfig {
            enable_safety_checks: false,
            ..CompilerConfig::default()
        };
        let compiler = AdaptiveCompiler::with_config(config);
        let module = unverified_module();
        // Should succeed because safety checks are disabled
        let result = compiler.compile(&module);
        assert!(result.is_ok());
    }

    #[test]
    fn compile_with_explicit_strategy() {
        let compiler = AdaptiveCompiler::new();
        let module = verified_module();
        let strategy = CompilationStrategy::new(
            "custom",
            CompilationTarget::Wasm {
                env: WasmEnvironment::Browser,
            },
            OptimizationLevel::Aggressive,
        );
        let artifact = compiler.compile_with_strategy(&module, &strategy).unwrap();
        assert_eq!(
            artifact.strategy.target,
            CompilationTarget::Wasm {
                env: WasmEnvironment::Browser
            }
        );
    }

    #[test]
    fn compile_to_native_x86() {
        let config = CompilerConfig {
            target: CompilationTarget::Native {
                arch: TargetArch::X86_64,
            },
            ..CompilerConfig::default()
        };
        let compiler = AdaptiveCompiler::with_config(config);
        let module = verified_module();
        let artifact = compiler.compile(&module).unwrap();
        assert!(artifact.generated_code.content.contains("push rbp"));
    }

    #[test]
    fn compile_to_native_aarch64() {
        let config = CompilerConfig {
            target: CompilationTarget::Native {
                arch: TargetArch::Aarch64,
            },
            ..CompilerConfig::default()
        };
        let compiler = AdaptiveCompiler::with_config(config);
        let module = verified_module();
        let artifact = compiler.compile(&module).unwrap();
        assert!(artifact.generated_code.content.contains("stp x29"));
    }

    #[test]
    fn compile_to_operator_call() {
        let config = CompilerConfig {
            target: CompilationTarget::OperatorCall,
            ..CompilerConfig::default()
        };
        let compiler = AdaptiveCompiler::with_config(config);
        let module = verified_module();
        let artifact = compiler.compile(&module).unwrap();
        assert!(artifact.generated_code.content.contains("fn main()"));
    }

    #[test]
    fn compile_to_interpreted() {
        let config = CompilerConfig {
            target: CompilationTarget::Interpreted,
            ..CompilerConfig::default()
        };
        let compiler = AdaptiveCompiler::with_config(config);
        let module = verified_module();
        let artifact = compiler.compile(&module).unwrap();
        assert!(artifact.generated_code.content.contains("test-mod"));
    }

    #[test]
    fn compile_with_no_optimization() {
        let config = CompilerConfig {
            optimization_level: OptimizationLevel::None,
            ..CompilerConfig::default()
        };
        let compiler = AdaptiveCompiler::with_config(config);
        let module = verified_module();
        let artifact = compiler.compile(&module).unwrap();
        assert!(artifact.optimization_results.is_empty());
    }

    #[test]
    fn compile_records_compilation_time() {
        let compiler = AdaptiveCompiler::new();
        let module = verified_module();
        let artifact = compiler.compile(&module).unwrap();
        // compilation_time_ms should be a non-negative number
        assert!(artifact.compilation_time_ms < 10_000);
        assert!(artifact.compiled_at > chrono::DateTime::UNIX_EPOCH);
    }
}
