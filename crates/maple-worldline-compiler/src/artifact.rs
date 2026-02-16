//! Compilation artifact building.
//!
//! `CompilationArtifact` is the output of a full compilation run,
//! containing the generated code, optimization results, strategy used,
//! and timing metadata. Built via `ArtifactBuilder`.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::codegen::GeneratedCode;
use crate::error::{CompilerError, CompilerResult};
use crate::passes::PassResult;
use crate::strategy::CompilationStrategy;
use crate::types::CompilationId;
use maple_worldline_ir::types::ModuleId;

// ── Compilation Artifact ─────────────────────────────────────────────

/// Output of a complete compilation run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompilationArtifact {
    pub compilation_id: CompilationId,
    pub module_id: ModuleId,
    pub module_name: String,
    pub strategy: CompilationStrategy,
    pub optimization_results: Vec<PassResult>,
    pub generated_code: GeneratedCode,
    pub compilation_time_ms: u64,
    pub compiled_at: chrono::DateTime<chrono::Utc>,
}

impl CompilationArtifact {
    /// Total number of optimizations actually applied.
    pub fn total_optimizations_applied(&self) -> u32 {
        self.optimization_results
            .iter()
            .filter(|r| r.applied)
            .map(|r| r.changes_made)
            .sum()
    }

    /// One-line summary of optimizations.
    pub fn optimizations_summary(&self) -> String {
        let applied = self
            .optimization_results
            .iter()
            .filter(|r| r.applied)
            .count();
        let total = self.optimization_results.len();
        let changes = self.total_optimizations_applied();
        format!(
            "{}/{} passes applied, {} total changes",
            applied, total, changes
        )
    }
}

// ── Artifact Builder ─────────────────────────────────────────────────

/// Builder for constructing compilation artifacts.
pub struct ArtifactBuilder {
    compilation_id: CompilationId,
    module_id: Option<ModuleId>,
    module_name: Option<String>,
    strategy: Option<CompilationStrategy>,
    optimization_results: Vec<PassResult>,
    generated_code: Option<GeneratedCode>,
    start_time: chrono::DateTime<chrono::Utc>,
}

impl ArtifactBuilder {
    pub fn new(compilation_id: CompilationId) -> Self {
        Self {
            compilation_id,
            module_id: None,
            module_name: None,
            strategy: None,
            optimization_results: Vec::new(),
            generated_code: None,
            start_time: Utc::now(),
        }
    }

    pub fn module(mut self, id: ModuleId, name: String) -> Self {
        self.module_id = Some(id);
        self.module_name = Some(name);
        self
    }

    pub fn strategy(mut self, strategy: CompilationStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    pub fn optimization_results(mut self, results: Vec<PassResult>) -> Self {
        self.optimization_results = results;
        self
    }

    pub fn generated_code(mut self, code: GeneratedCode) -> Self {
        self.generated_code = Some(code);
        self
    }

    pub fn build(self) -> CompilerResult<CompilationArtifact> {
        let module_id = self
            .module_id
            .ok_or_else(|| CompilerError::ConfigurationError("module_id is required".into()))?;
        let module_name = self
            .module_name
            .ok_or_else(|| CompilerError::ConfigurationError("module_name is required".into()))?;
        let strategy = self
            .strategy
            .ok_or_else(|| CompilerError::ConfigurationError("strategy is required".into()))?;
        let generated_code = self.generated_code.ok_or_else(|| {
            CompilerError::ConfigurationError("generated_code is required".into())
        })?;

        let now = Utc::now();
        let compilation_time_ms = (now - self.start_time).num_milliseconds().max(0) as u64;

        Ok(CompilationArtifact {
            compilation_id: self.compilation_id,
            module_id,
            module_name,
            strategy,
            optimization_results: self.optimization_results,
            generated_code,
            compilation_time_ms,
            compiled_at: now,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::PassId;
    use crate::types::{CompilationTarget, OptimizationLevel, TargetArch};

    fn sample_strategy() -> CompilationStrategy {
        CompilationStrategy::new(
            "test",
            CompilationTarget::Native {
                arch: TargetArch::X86_64,
            },
            OptimizationLevel::Basic,
        )
    }

    fn sample_code() -> GeneratedCode {
        GeneratedCode {
            target: CompilationTarget::Native {
                arch: TargetArch::X86_64,
            },
            content: "push rbp\nret\n".into(),
            content_hash: GeneratedCode::compute_hash("push rbp\nret\n"),
            size_bytes: 13,
            generated_at: Utc::now(),
        }
    }

    fn sample_pass_results() -> Vec<PassResult> {
        vec![
            PassResult {
                pass_id: PassId::ConstantFolding,
                applied: true,
                changes_made: 3,
                description: "3 folded".into(),
            },
            PassResult {
                pass_id: PassId::DeadCodeElimination,
                applied: false,
                changes_made: 0,
                description: "none".into(),
            },
        ]
    }

    #[test]
    fn artifact_creation() {
        let artifact = ArtifactBuilder::new(CompilationId::new())
            .module(ModuleId::new(), "test-mod".into())
            .strategy(sample_strategy())
            .optimization_results(sample_pass_results())
            .generated_code(sample_code())
            .build()
            .unwrap();
        assert_eq!(artifact.module_name, "test-mod");
    }

    #[test]
    fn artifact_total_optimizations() {
        let artifact = ArtifactBuilder::new(CompilationId::new())
            .module(ModuleId::new(), "test".into())
            .strategy(sample_strategy())
            .optimization_results(sample_pass_results())
            .generated_code(sample_code())
            .build()
            .unwrap();
        assert_eq!(artifact.total_optimizations_applied(), 3);
    }

    #[test]
    fn artifact_optimizations_summary() {
        let artifact = ArtifactBuilder::new(CompilationId::new())
            .module(ModuleId::new(), "test".into())
            .strategy(sample_strategy())
            .optimization_results(sample_pass_results())
            .generated_code(sample_code())
            .build()
            .unwrap();
        let summary = artifact.optimizations_summary();
        assert!(summary.contains("1/2 passes applied"));
        assert!(summary.contains("3 total changes"));
    }

    #[test]
    fn artifact_builder_missing_module_fails() {
        let result = ArtifactBuilder::new(CompilationId::new())
            .strategy(sample_strategy())
            .generated_code(sample_code())
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn artifact_builder_missing_code_fails() {
        let result = ArtifactBuilder::new(CompilationId::new())
            .module(ModuleId::new(), "test".into())
            .strategy(sample_strategy())
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn pass_result_display() {
        let r = PassResult {
            pass_id: PassId::ConstantFolding,
            applied: true,
            changes_made: 5,
            description: "5 folded".into(),
        };
        let display = r.to_string();
        assert!(display.contains("constant-folding"));
        assert!(display.contains("applied"));
        assert!(display.contains("5 changes"));
    }
}
