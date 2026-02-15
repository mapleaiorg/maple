//! Core types for the adaptive compiler.
//!
//! Defines identifiers, compilation targets, optimization levels,
//! compilation status, configuration, profiling data, and summaries.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use maple_worldline_ir::types::FunctionId;

// ── Identifiers ──────────────────────────────────────────────────────

/// Unique identifier for a compilation run.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompilationId(pub String);

impl CompilationId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for CompilationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CompilationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "compile:{}", self.0)
    }
}

/// Unique identifier for a compilation strategy.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StrategyId(pub String);

impl StrategyId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_name(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl Default for StrategyId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for StrategyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "strategy:{}", self.0)
    }
}

// ── Compilation Target ───────────────────────────────────────────────

/// Target architecture for native compilation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetArch {
    X86_64,
    Aarch64,
}

impl std::fmt::Display for TargetArch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::X86_64 => write!(f, "x86-64"),
            Self::Aarch64 => write!(f, "aarch64"),
        }
    }
}

/// WASM execution environment.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WasmEnvironment {
    Browser,
    Edge,
}

impl std::fmt::Display for WasmEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Browser => write!(f, "browser"),
            Self::Edge => write!(f, "edge"),
        }
    }
}

/// Target for compilation output.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompilationTarget {
    /// Native machine code (x86-64 or ARM64).
    Native { arch: TargetArch },
    /// WebAssembly for browser or edge environments.
    Wasm { env: WasmEnvironment },
    /// Direct Rust operator calls.
    OperatorCall,
    /// WLIR interpreter (development/debug).
    Interpreted,
}

impl std::fmt::Display for CompilationTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native { arch } => write!(f, "native-{}", arch),
            Self::Wasm { env } => write!(f, "wasm-{}", env),
            Self::OperatorCall => write!(f, "operator-call"),
            Self::Interpreted => write!(f, "interpreted"),
        }
    }
}

// ── Optimization Level ───────────────────────────────────────────────

/// Optimization level for the compilation pipeline.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationLevel {
    /// No optimization passes.
    None,
    /// Standard passes only.
    Basic,
    /// Standard + EVOS-specific passes.
    Aggressive,
}

impl std::fmt::Display for OptimizationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Basic => write!(f, "basic"),
            Self::Aggressive => write!(f, "aggressive"),
        }
    }
}

// ── Compilation Status ───────────────────────────────────────────────

/// Status of a compilation run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompilationStatus {
    Started,
    Optimizing,
    Generating,
    Complete,
    Failed(String),
}

impl std::fmt::Display for CompilationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Started => write!(f, "started"),
            Self::Optimizing => write!(f, "optimizing"),
            Self::Generating => write!(f, "generating"),
            Self::Complete => write!(f, "complete"),
            Self::Failed(reason) => write!(f, "failed: {}", reason),
        }
    }
}

// ── Configuration ────────────────────────────────────────────────────

/// Configuration for the adaptive compiler.
#[derive(Clone, Debug)]
pub struct CompilerConfig {
    /// Default compilation target.
    pub target: CompilationTarget,
    /// Optimization level.
    pub optimization_level: OptimizationLevel,
    /// Whether to enforce safety checks during compilation.
    pub enable_safety_checks: bool,
    /// Maximum optimization iterations.
    pub max_optimization_iterations: u32,
    /// Whether to preserve debug info in output.
    pub preserve_debug_info: bool,
    /// Maximum tracked compilation records.
    pub max_tracked_records: usize,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            target: CompilationTarget::Native {
                arch: TargetArch::X86_64,
            },
            optimization_level: OptimizationLevel::Basic,
            enable_safety_checks: true,
            max_optimization_iterations: 10,
            preserve_debug_info: false,
            max_tracked_records: 256,
        }
    }
}

// ── Profiling Data ───────────────────────────────────────────────────

/// Profiling data used for strategy evolution.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProfilingData {
    /// Functions identified as hot (frequently executed).
    pub hot_functions: Vec<FunctionId>,
    /// Memory tier usage counts.
    pub memory_tier_usage: HashMap<String, u64>,
    /// Operator call frequency.
    pub operator_call_frequency: HashMap<String, u64>,
    /// Total commitment boundary crossings observed.
    pub commitment_boundary_crossings: u64,
}

// ── Summary ──────────────────────────────────────────────────────────

/// Summary statistics for the compiler engine.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CompilerSummary {
    pub total_compilations: usize,
    pub successful_compilations: usize,
    pub failed_compilations: usize,
    pub total_optimizations_applied: usize,
    pub average_compilation_time_ms: f64,
}

impl std::fmt::Display for CompilerSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Compiler(compilations={}, success={}, failed={}, optimizations={}, avg_time={:.1}ms)",
            self.total_compilations,
            self.successful_compilations,
            self.failed_compilations,
            self.total_optimizations_applied,
            self.average_compilation_time_ms,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compilation_id_creation() {
        let a = CompilationId::new();
        let b = CompilationId::new();
        assert_ne!(a, b);
        assert!(a.to_string().starts_with("compile:"));
    }

    #[test]
    fn strategy_id_from_name() {
        let id = StrategyId::from_name("fast-native");
        assert_eq!(id.to_string(), "strategy:fast-native");
    }

    #[test]
    fn compilation_target_native_x86() {
        let target = CompilationTarget::Native { arch: TargetArch::X86_64 };
        assert_eq!(target.to_string(), "native-x86-64");
    }

    #[test]
    fn compilation_target_wasm_browser() {
        let target = CompilationTarget::Wasm { env: WasmEnvironment::Browser };
        assert_eq!(target.to_string(), "wasm-browser");
    }

    #[test]
    fn compilation_target_all_variants() {
        let targets = vec![
            CompilationTarget::Native { arch: TargetArch::X86_64 },
            CompilationTarget::Native { arch: TargetArch::Aarch64 },
            CompilationTarget::Wasm { env: WasmEnvironment::Browser },
            CompilationTarget::Wasm { env: WasmEnvironment::Edge },
            CompilationTarget::OperatorCall,
            CompilationTarget::Interpreted,
        ];
        for t in &targets {
            assert!(!t.to_string().is_empty());
        }
        assert_eq!(targets.len(), 6);
    }

    #[test]
    fn optimization_level_display() {
        assert_eq!(OptimizationLevel::None.to_string(), "none");
        assert_eq!(OptimizationLevel::Basic.to_string(), "basic");
        assert_eq!(OptimizationLevel::Aggressive.to_string(), "aggressive");
    }

    #[test]
    fn compilation_status_display() {
        assert_eq!(CompilationStatus::Started.to_string(), "started");
        assert_eq!(CompilationStatus::Complete.to_string(), "complete");
        assert!(CompilationStatus::Failed("oops".into()).to_string().contains("oops"));
    }

    #[test]
    fn config_defaults() {
        let cfg = CompilerConfig::default();
        assert_eq!(cfg.optimization_level, OptimizationLevel::Basic);
        assert!(cfg.enable_safety_checks);
        assert_eq!(cfg.max_optimization_iterations, 10);
        assert_eq!(cfg.max_tracked_records, 256);
    }

    #[test]
    fn profiling_data_default() {
        let pd = ProfilingData::default();
        assert!(pd.hot_functions.is_empty());
        assert_eq!(pd.commitment_boundary_crossings, 0);
    }

    #[test]
    fn summary_display() {
        let s = CompilerSummary {
            total_compilations: 10,
            successful_compilations: 8,
            failed_compilations: 2,
            total_optimizations_applied: 50,
            average_compilation_time_ms: 123.4,
        };
        let display = s.to_string();
        assert!(display.contains("compilations=10"));
        assert!(display.contains("success=8"));
        assert!(display.contains("123.4ms"));
    }
}
