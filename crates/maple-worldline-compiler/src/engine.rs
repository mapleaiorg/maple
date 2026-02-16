//! Adaptive compiler engine with bounded compilation history.
//!
//! `AdaptiveCompilerEngine` wraps `AdaptiveCompiler` and maintains a
//! bounded FIFO of `CompilationRecord`s. Provides `compile()`, `find()`,
//! `all_records()`, and `summary()`.

use std::collections::VecDeque;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::artifact::CompilationArtifact;
use crate::compiler::AdaptiveCompiler;
use crate::error::CompilerResult;
use crate::strategy::CompilationStrategy;
use crate::types::{CompilationId, CompilationStatus, CompilerConfig, CompilerSummary};
use maple_worldline_ir::module::WlirModule;
use maple_worldline_ir::types::ModuleId;

// ── Compilation Record ──────────────────────────────────────────────

/// Record of a single compilation run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompilationRecord {
    pub compilation_id: CompilationId,
    pub module_id: ModuleId,
    pub module_name: String,
    pub status: CompilationStatus,
    pub artifact: Option<CompilationArtifact>,
    pub error_message: Option<String>,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
}

// ── Adaptive Compiler Engine ────────────────────────────────────────

/// Engine that wraps an `AdaptiveCompiler` with bounded compilation history.
pub struct AdaptiveCompilerEngine {
    compiler: AdaptiveCompiler,
    records: VecDeque<CompilationRecord>,
    max_records: usize,
}

impl AdaptiveCompilerEngine {
    /// Create a new engine with default configuration.
    pub fn new() -> Self {
        let config = CompilerConfig::default();
        let max = config.max_tracked_records;
        Self {
            compiler: AdaptiveCompiler::with_config(config),
            records: VecDeque::new(),
            max_records: max,
        }
    }

    /// Create with a specific configuration.
    pub fn with_config(config: CompilerConfig) -> Self {
        let max = config.max_tracked_records;
        Self {
            compiler: AdaptiveCompiler::with_config(config),
            records: VecDeque::new(),
            max_records: max,
        }
    }

    /// Access the underlying compiler.
    pub fn compiler(&self) -> &AdaptiveCompiler {
        &self.compiler
    }

    /// Compile a module, recording the result.
    pub fn compile(&mut self, module: &WlirModule) -> CompilerResult<CompilationArtifact> {
        let result = self.compiler.compile(module);
        self.record_result(module, &result);
        result
    }

    /// Compile with an explicit strategy, recording the result.
    pub fn compile_with_strategy(
        &mut self,
        module: &WlirModule,
        strategy: &CompilationStrategy,
    ) -> CompilerResult<CompilationArtifact> {
        let result = self.compiler.compile_with_strategy(module, strategy);
        self.record_result(module, &result);
        result
    }

    /// Record a compilation result.
    fn record_result(&mut self, module: &WlirModule, result: &CompilerResult<CompilationArtifact>) {
        let record = match result {
            Ok(artifact) => CompilationRecord {
                compilation_id: artifact.compilation_id.clone(),
                module_id: module.id.clone(),
                module_name: module.name.clone(),
                status: CompilationStatus::Complete,
                artifact: Some(artifact.clone()),
                error_message: None,
                recorded_at: Utc::now(),
            },
            Err(e) => CompilationRecord {
                compilation_id: CompilationId::new(),
                module_id: module.id.clone(),
                module_name: module.name.clone(),
                status: CompilationStatus::Failed(e.to_string()),
                artifact: None,
                error_message: Some(e.to_string()),
                recorded_at: Utc::now(),
            },
        };

        // Bounded FIFO: evict oldest if at capacity
        if self.records.len() >= self.max_records {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }

    /// Find a record by compilation ID.
    pub fn find(&self, id: &CompilationId) -> Option<&CompilationRecord> {
        self.records.iter().find(|r| &r.compilation_id == id)
    }

    /// Find records for a specific module.
    pub fn find_by_module(&self, module_id: &ModuleId) -> Vec<&CompilationRecord> {
        self.records
            .iter()
            .filter(|r| &r.module_id == module_id)
            .collect()
    }

    /// All compilation records in order.
    pub fn all_records(&self) -> &VecDeque<CompilationRecord> {
        &self.records
    }

    /// Number of tracked records.
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// Generate a summary of compilation history.
    pub fn summary(&self) -> CompilerSummary {
        let total = self.records.len();
        let successful = self.records.iter().filter(|r| r.artifact.is_some()).count();
        let failed = total - successful;

        let total_optimizations: usize = self
            .records
            .iter()
            .filter_map(|r| r.artifact.as_ref())
            .map(|a| a.total_optimizations_applied() as usize)
            .sum();

        let total_time: u64 = self
            .records
            .iter()
            .filter_map(|r| r.artifact.as_ref())
            .map(|a| a.compilation_time_ms)
            .sum();

        let avg_time = if successful > 0 {
            total_time as f64 / successful as f64
        } else {
            0.0
        };

        CompilerSummary {
            total_compilations: total,
            successful_compilations: successful,
            failed_compilations: failed,
            total_optimizations_applied: total_optimizations,
            average_compilation_time_ms: avg_time,
        }
    }
}

impl Default for AdaptiveCompilerEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CompilationTarget, OptimizationLevel};
    use maple_worldline_ir::instructions::WlirInstruction;
    use maple_worldline_ir::module::WlirFunction;
    use maple_worldline_ir::types::{VerificationStatus, WlirType};

    fn verified_module(name: &str) -> WlirModule {
        let mut module = WlirModule::new(name, "1.0");
        let mut f = WlirFunction::new("main", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::Nop);
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);
        module.verification_status = VerificationStatus::FullyVerified;
        module
    }

    #[test]
    fn engine_compile_stores_record() {
        let mut engine = AdaptiveCompilerEngine::new();
        let module = verified_module("alpha");
        let artifact = engine.compile(&module).unwrap();
        assert_eq!(engine.record_count(), 1);
        let record = engine.find(&artifact.compilation_id);
        assert!(record.is_some());
        assert_eq!(record.unwrap().module_name, "alpha");
    }

    #[test]
    fn engine_compile_failure_stores_record() {
        let mut engine = AdaptiveCompilerEngine::new();
        let module = WlirModule::new("unverified", "1.0");
        let result = engine.compile(&module);
        assert!(result.is_err());
        assert_eq!(engine.record_count(), 1);
        let record = &engine.all_records()[0];
        assert!(record.error_message.is_some());
    }

    #[test]
    fn engine_bounded_fifo() {
        let config = CompilerConfig {
            max_tracked_records: 3,
            ..CompilerConfig::default()
        };
        let mut engine = AdaptiveCompilerEngine::with_config(config);

        for i in 0..5 {
            let module = verified_module(&format!("mod-{}", i));
            let _ = engine.compile(&module);
        }

        assert_eq!(engine.record_count(), 3);
        // Oldest records should have been evicted
        let names: Vec<_> = engine
            .all_records()
            .iter()
            .map(|r| r.module_name.clone())
            .collect();
        assert_eq!(names, vec!["mod-2", "mod-3", "mod-4"]);
    }

    #[test]
    fn engine_find_by_module() {
        let mut engine = AdaptiveCompilerEngine::new();
        let module = verified_module("findme");
        let _ = engine.compile(&module).unwrap();
        let _ = engine.compile(&module).unwrap();
        let results = engine.find_by_module(&module.id);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn engine_summary() {
        let mut engine = AdaptiveCompilerEngine::new();
        let good = verified_module("good");
        let bad = WlirModule::new("bad", "1.0");
        let _ = engine.compile(&good);
        let _ = engine.compile(&bad);
        let summary = engine.summary();
        assert_eq!(summary.total_compilations, 2);
        assert_eq!(summary.successful_compilations, 1);
        assert_eq!(summary.failed_compilations, 1);
    }

    #[test]
    fn engine_compile_with_strategy() {
        let mut engine = AdaptiveCompilerEngine::new();
        let module = verified_module("explicit");
        let strategy = CompilationStrategy::new(
            "wasm-aggro",
            CompilationTarget::Wasm {
                env: crate::types::WasmEnvironment::Edge,
            },
            OptimizationLevel::Aggressive,
        );
        let artifact = engine.compile_with_strategy(&module, &strategy).unwrap();
        assert_eq!(
            artifact.strategy.target,
            CompilationTarget::Wasm {
                env: crate::types::WasmEnvironment::Edge
            }
        );
        assert_eq!(engine.record_count(), 1);
    }

    #[test]
    fn engine_summary_display() {
        let mut engine = AdaptiveCompilerEngine::new();
        let module = verified_module("disp");
        let _ = engine.compile(&module);
        let summary = engine.summary();
        let display = summary.to_string();
        assert!(display.contains("compilations=1"));
        assert!(display.contains("success=1"));
    }

    #[test]
    fn engine_default() {
        let engine = AdaptiveCompilerEngine::default();
        assert_eq!(engine.record_count(), 0);
        let summary = engine.summary();
        assert_eq!(summary.total_compilations, 0);
    }
}
