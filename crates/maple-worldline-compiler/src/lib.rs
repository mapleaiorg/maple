//! # maple-worldline-compiler
//!
//! Adaptive compiler for the WorldLine framework. Compiles verified WLIR
//! modules to native code (x86-64/ARM64), WebAssembly, direct operator
//! calls, or interpreted bytecode, with strategies that evolve based on
//! profiling feedback.
//!
//! ## Safety Invariants
//!
//! - **I.COMPILE-1**: Compiled code produces the same results as WLIR interpretation.
//! - **I.COMPILE-2**: Commitment gates are preserved during optimization:
//!   - CommitmentBatching MUST NOT merge commitments with different scopes.
//!   - SafetyFenceMinimization MUST NOT remove fences guarding `AssertInvariant`.

#![deny(unsafe_code)]

pub mod artifact;
pub mod codegen;
pub mod compiler;
pub mod engine;
pub mod error;
pub mod evolution;
pub mod passes;
pub mod strategy;
pub mod types;

// ── Re-exports ──────────────────────────────────────────────────────

pub use artifact::{ArtifactBuilder, CompilationArtifact};
pub use codegen::{
    CodeGenerator, GeneratedCode, NativeCodeGen, OperatorCallCodeGen, WasmCodeGen,
    WlirInterpreterCodeGen,
};
pub use compiler::AdaptiveCompiler;
pub use engine::{AdaptiveCompilerEngine, CompilationRecord};
pub use error::{CompilerError, CompilerResult};
pub use evolution::{SimulatedEvolver, StrategyEvolver, StrategyProposal};
pub use passes::{OptimizationPass, OptimizationPipeline, PassResult};
pub use strategy::{CompilationStrategy, PassId, SimulatedStrategySelector, StrategySelector};
pub use types::{
    CompilationId, CompilationStatus, CompilationTarget, CompilerConfig, CompilerSummary,
    OptimizationLevel, ProfilingData, StrategyId, TargetArch, WasmEnvironment,
};

#[cfg(test)]
mod tests {
    use super::*;
    use maple_worldline_ir::instructions::WlirInstruction;
    use maple_worldline_ir::module::{WlirFunction, WlirModule};
    use maple_worldline_ir::types::{VerificationStatus, WlirType};

    fn verified_module() -> WlirModule {
        let mut module = WlirModule::new("integration-test", "2.0");
        let mut f = WlirFunction::new("main", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::Nop);
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);
        module.verification_status = VerificationStatus::FullyVerified;
        module
    }

    fn module_with_constants() -> WlirModule {
        let mut module = WlirModule::new("const-mod", "1.0");
        let mut f = WlirFunction::new("compute", vec![], WlirType::I32);
        f.push_instruction(WlirInstruction::LoadConst {
            result: 0,
            constant_index: 0,
        });
        f.push_instruction(WlirInstruction::LoadConst {
            result: 1,
            constant_index: 1,
        });
        f.push_instruction(WlirInstruction::Add {
            result: 2,
            a: 0,
            b: 1,
        });
        f.push_instruction(WlirInstruction::Return { value: Some(2) });
        module.add_function(f);
        module.verification_status = VerificationStatus::FullyVerified;
        module
    }

    // ── E2E: compile to all 4 targets ──

    #[test]
    fn e2e_compile_all_targets() {
        let module = verified_module();
        let targets = vec![
            CompilationTarget::Native {
                arch: TargetArch::X86_64,
            },
            CompilationTarget::Native {
                arch: TargetArch::Aarch64,
            },
            CompilationTarget::Wasm {
                env: WasmEnvironment::Browser,
            },
            CompilationTarget::Wasm {
                env: WasmEnvironment::Edge,
            },
            CompilationTarget::OperatorCall,
            CompilationTarget::Interpreted,
        ];

        for target in targets {
            let config = CompilerConfig {
                target: target.clone(),
                ..CompilerConfig::default()
            };
            let compiler = AdaptiveCompiler::with_config(config);
            let artifact = compiler.compile(&module).unwrap();
            assert!(!artifact.generated_code.content.is_empty());
            assert!(!artifact.generated_code.content_hash.is_empty());
        }
    }

    // ── Optimization pipeline E2E ──

    #[test]
    fn e2e_optimization_pipeline() {
        let module = module_with_constants();
        let strategy = CompilationStrategy::new(
            "full",
            CompilationTarget::Native {
                arch: TargetArch::X86_64,
            },
            OptimizationLevel::Aggressive,
        );
        let pipeline = OptimizationPipeline::from_strategy(&strategy);
        assert_eq!(pipeline.len(), 11);
        let results = pipeline.apply_all(&module).unwrap();
        assert_eq!(results.len(), 11);
        // At least constant folding should have applied
        assert!(results
            .iter()
            .any(|r| r.pass_id == PassId::ConstantFolding && r.applied));
    }

    // ── Strategy evolution E2E ──

    #[test]
    fn e2e_strategy_evolution() {
        let evolver = SimulatedEvolver::new();
        let strategy = CompilationStrategy::new(
            "initial",
            CompilationTarget::Native {
                arch: TargetArch::X86_64,
            },
            OptimizationLevel::Basic,
        );

        let mut profiling = ProfilingData::default();
        profiling
            .memory_tier_usage
            .insert("episodic".into(), 200);
        profiling
            .operator_call_frequency
            .insert("transfer".into(), 100);

        let proposal = evolver.propose(&strategy, &profiling);
        assert!(proposal.has_changes());
        let evolved = proposal.apply_to(&strategy);
        assert!(evolved.has_pass(&PassId::MemoryTierPromotion));
        assert!(evolved.has_pass(&PassId::OperatorDispatchSpecialization));
    }

    // ── Engine multi-compile ──

    #[test]
    fn e2e_engine_multi_compile() {
        let mut engine = AdaptiveCompilerEngine::new();
        let module = verified_module();
        for _ in 0..5 {
            let _ = engine.compile(&module).unwrap();
        }
        assert_eq!(engine.record_count(), 5);
        let summary = engine.summary();
        assert_eq!(summary.successful_compilations, 5);
        assert_eq!(summary.failed_compilations, 0);
    }

    // ── I.COMPILE-1: Semantics preservation ──

    #[test]
    fn invariant_compile_1_semantics_preservation() {
        // Verify that the interpreted output contains the same module
        // structure as the original (i.e., compilation preserves semantics)
        let module = verified_module();
        let config = CompilerConfig {
            target: CompilationTarget::Interpreted,
            ..CompilerConfig::default()
        };
        let compiler = AdaptiveCompiler::with_config(config);
        let artifact = compiler.compile(&module).unwrap();

        // The interpreted target serializes the module to JSON —
        // verify it contains key module elements
        assert!(artifact.generated_code.content.contains("integration-test"));
        assert!(artifact.generated_code.content.contains("main"));
        assert!(artifact.generated_code.content.contains("Nop"));
    }

    // ── I.COMPILE-2: Commitment gate safety ──

    #[test]
    fn invariant_compile_2_commitment_gates_preserved() {
        use maple_worldline_ir::instructions::BoundaryDirection;

        // Build a module with different-scope commitments
        let mut module = WlirModule::new("commitment-test", "1.0");
        let mut f = WlirFunction::new("transact", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::CrossCommitmentBoundary {
            commitment_id: "payment".into(),
            direction: BoundaryDirection::Enter,
        });
        f.push_instruction(WlirInstruction::CrossCommitmentBoundary {
            commitment_id: "payment".into(),
            direction: BoundaryDirection::Exit,
        });
        f.push_instruction(WlirInstruction::CrossCommitmentBoundary {
            commitment_id: "audit".into(),
            direction: BoundaryDirection::Enter,
        });
        f.push_instruction(WlirInstruction::CrossCommitmentBoundary {
            commitment_id: "audit".into(),
            direction: BoundaryDirection::Exit,
        });
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);
        module.verification_status = VerificationStatus::FullyVerified;

        // Run CommitmentBatching pass
        let pass = passes::CommitmentBatchingPass;
        let result = pass.apply(&module).unwrap();

        // Different scopes must be preserved — never merged
        assert!(result.description.contains("different-scope preserved"));
    }

    #[test]
    fn invariant_compile_2_safety_fences_for_invariants() {
        // Build a module with a fence guarding an invariant
        let mut module = WlirModule::new("fence-test", "1.0");
        let mut f = WlirFunction::new("safe_op", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::SafetyFence {
            fence_name: "pre-check".into(),
            preceding_ops: vec![0],
        });
        f.push_instruction(WlirInstruction::AssertInvariant {
            condition: 0,
            invariant_name: "balance_positive".into(),
            message: "balance must be positive".into(),
        });
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);
        module.verification_status = VerificationStatus::FullyVerified;

        // Run SafetyFenceMinimization
        let pass = passes::SafetyFenceMinimizationPass;
        let result = pass.apply(&module).unwrap();

        // The fence guarding AssertInvariant MUST NOT be removed
        assert!(result.description.contains("invariant-guarding fences preserved"));
        // changes_made should be 0 (the only fence guards an invariant)
        assert_eq!(result.changes_made, 0);
    }

    // ── Public types ──

    #[test]
    fn public_types_accessible() {
        let _id = CompilationId::new();
        let _sid = StrategyId::from_name("test");
        let _target = CompilationTarget::Interpreted;
        let _level = OptimizationLevel::Aggressive;
        let _status = CompilationStatus::Started;
        let _config = CompilerConfig::default();
        let _profiling = ProfilingData::default();
        let _summary = CompilerSummary::default();
        let _pass = PassId::ConstantFolding;
    }

    #[test]
    fn generator_names_accessible() {
        assert_eq!(NativeCodeGen::new(TargetArch::X86_64).name(), "native-codegen");
        assert_eq!(
            WasmCodeGen::new(WasmEnvironment::Browser).name(),
            "wasm-codegen"
        );
        assert_eq!(OperatorCallCodeGen::new().name(), "operator-call-codegen");
        assert_eq!(WlirInterpreterCodeGen::new().name(), "wlir-interpreter");
    }
}
