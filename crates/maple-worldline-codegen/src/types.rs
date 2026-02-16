//! Core types for the code generation engine.
//!
//! Defines identifiers, status enums, generated code artifacts,
//! generation records, compilation/test/performance results, and configuration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use maple_worldline_self_mod_gate::types::SelfModTier;

// ── Identifier ─────────────────────────────────────────────────────────

/// Unique identifier for a code generation session.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CodegenId(pub String);

impl CodegenId {
    /// Generate a new unique codegen ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for CodegenId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CodegenId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "codegen:{}", self.0)
    }
}

// ── Status ─────────────────────────────────────────────────────────────

/// Status of a code generation session.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CodegenStatus {
    /// Pending generation.
    Pending,
    /// Code generated, awaiting compilation.
    Generated,
    /// Compilation passed, awaiting test validation.
    Compiled,
    /// All validation passed — artifact ready.
    Validated,
    /// Generation or validation failed.
    Failed(String),
}

impl CodegenStatus {
    /// Whether this is a terminal (completed) status.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Validated | Self::Failed(_))
    }

    /// Whether this is a successful terminal status.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Validated)
    }
}

impl std::fmt::Display for CodegenStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Generated => write!(f, "generated"),
            Self::Compiled => write!(f, "compiled"),
            Self::Validated => write!(f, "validated"),
            Self::Failed(reason) => write!(f, "failed: {}", reason),
        }
    }
}

// ── Generated Code ─────────────────────────────────────────────────────

/// A single piece of generated code corresponding to one CodeChangeSpec.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneratedCode {
    /// Index of the CodeChangeSpec this was generated from.
    pub change_spec_index: usize,
    /// File path being modified/created.
    pub file_path: String,
    /// The generated source code content.
    pub content: String,
    /// Description of what was generated.
    pub description: String,
    /// Hash of the generated content (for integrity verification).
    pub content_hash: String,
    /// When this code was generated.
    pub generated_at: DateTime<Utc>,
}

impl GeneratedCode {
    /// Compute a simple content hash (format-based checksum).
    pub fn compute_hash(content: &str) -> String {
        // Simple hash using Rust's built-in hasher for integrity detection.
        // Not cryptographic — for that, use sha2 (in the consequence crate).
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Content size in bytes.
    pub fn size_bytes(&self) -> usize {
        self.content.len()
    }
}

// ── Compilation Result ─────────────────────────────────────────────────

/// Result of compiling a single generated file in the sandbox.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompilationResult {
    /// File that was compiled.
    pub file_path: String,
    /// Whether compilation succeeded.
    pub success: bool,
    /// Compiler diagnostics (warnings, errors).
    pub diagnostics: Vec<String>,
    /// Compilation duration in milliseconds.
    pub duration_ms: i64,
}

// ── Test Result ────────────────────────────────────────────────────────

/// Result of running a single test.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestResult {
    /// Name of the test.
    pub test_name: String,
    /// Whether the test passed.
    pub passed: bool,
    /// Test output/log.
    pub output: String,
    /// Test duration in milliseconds.
    pub duration_ms: i64,
}

// ── Performance Result ─────────────────────────────────────────────────

/// Result of checking a single performance gate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceResult {
    /// Metric name.
    pub metric: String,
    /// Measured value.
    pub measured_value: f64,
    /// Gate threshold.
    pub threshold: f64,
    /// Whether the gate was met.
    pub passed: bool,
    /// Description of the result.
    pub description: String,
}

// ── Generation Record ──────────────────────────────────────────────────

/// Record of a complete code generation session.
///
/// Tracks the full lifecycle from initial request through generation,
/// compilation, validation, and final status.
#[derive(Clone, Debug)]
pub struct GenerationRecord {
    /// Unique ID for this codegen session.
    pub id: CodegenId,
    /// The commitment ID this generation was for.
    pub commitment_id: String,
    /// Self-modification tier.
    pub tier: SelfModTier,
    /// Current status.
    pub status: CodegenStatus,
    /// Generated code artifacts.
    pub generated_code: Vec<GeneratedCode>,
    /// Sandbox compilation results per file.
    pub compilation_results: Vec<CompilationResult>,
    /// Test validation results.
    pub test_results: Vec<TestResult>,
    /// Performance gate results.
    pub performance_results: Vec<PerformanceResult>,
    /// Total generation duration in milliseconds.
    pub duration_ms: Option<i64>,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
    /// When generation completed (success or failure).
    pub completed_at: Option<DateTime<Utc>>,
}

impl GenerationRecord {
    /// Create a new generation record.
    pub fn new(commitment_id: String, tier: SelfModTier) -> Self {
        Self {
            id: CodegenId::new(),
            commitment_id,
            tier,
            status: CodegenStatus::Pending,
            generated_code: vec![],
            compilation_results: vec![],
            test_results: vec![],
            performance_results: vec![],
            duration_ms: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    /// Mark as generated (code produced, awaiting compilation).
    pub fn mark_generated(&mut self) {
        self.status = CodegenStatus::Generated;
    }

    /// Mark as compiled (compilation passed).
    pub fn mark_compiled(&mut self) {
        self.status = CodegenStatus::Compiled;
    }

    /// Mark as validated (all validation passed).
    pub fn mark_validated(&mut self) {
        self.status = CodegenStatus::Validated;
        self.completed_at = Some(Utc::now());
        if let Some(completed) = self.completed_at {
            self.duration_ms = Some((completed - self.created_at).num_milliseconds());
        }
    }

    /// Mark as failed.
    pub fn mark_failed(&mut self, reason: String) {
        self.status = CodegenStatus::Failed(reason);
        self.completed_at = Some(Utc::now());
        if let Some(completed) = self.completed_at {
            self.duration_ms = Some((completed - self.created_at).num_milliseconds());
        }
    }
}

// ── Configuration ──────────────────────────────────────────────────────

/// Configuration for the codegen engine.
#[derive(Clone, Debug)]
pub struct CodegenConfig {
    /// Maximum time for a single code generation (seconds).
    pub max_generation_timeout_secs: u64,
    /// Maximum time for sandbox compilation (seconds).
    pub max_compilation_timeout_secs: u64,
    /// Whether all tests must pass for validation.
    pub require_all_tests_pass: bool,
    /// Whether all performance gates must be met.
    pub require_performance_gates: bool,
    /// Whether safety checks are enforced.
    pub enforce_safety_checks: bool,
    /// Maximum tracked generation records (bounded FIFO).
    pub max_tracked_records: usize,
    /// Maximum generated code size in bytes per file.
    pub max_code_size_bytes: usize,
}

impl Default for CodegenConfig {
    fn default() -> Self {
        Self {
            max_generation_timeout_secs: 300,
            max_compilation_timeout_secs: 120,
            require_all_tests_pass: true,
            require_performance_gates: true,
            enforce_safety_checks: true,
            max_tracked_records: 256,
            max_code_size_bytes: 1_048_576, // 1 MB
        }
    }
}

// ── Summary ────────────────────────────────────────────────────────────

/// Summary statistics for the codegen engine.
#[derive(Clone, Debug, Default)]
pub struct CodegenSummary {
    /// Total generation sessions.
    pub total: usize,
    /// Pending sessions.
    pub pending: usize,
    /// Succeeded sessions.
    pub succeeded: usize,
    /// Failed sessions.
    pub failed: usize,
    /// Total files generated across all sessions.
    pub total_files_generated: usize,
    /// Total tests run across all sessions.
    pub total_tests_run: usize,
    /// Total tests passed across all sessions.
    pub total_tests_passed: usize,
}

impl std::fmt::Display for CodegenSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CodegenSummary(total={}, succeeded={}, failed={}, files={}, tests={}/{})",
            self.total,
            self.succeeded,
            self.failed,
            self.total_files_generated,
            self.total_tests_passed,
            self.total_tests_run,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codegen_id_uniqueness() {
        let a = CodegenId::new();
        let b = CodegenId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn codegen_id_display_format() {
        let id = CodegenId::new();
        assert!(id.to_string().starts_with("codegen:"));
    }

    #[test]
    fn codegen_status_display() {
        assert_eq!(CodegenStatus::Pending.to_string(), "pending");
        assert_eq!(CodegenStatus::Generated.to_string(), "generated");
        assert_eq!(CodegenStatus::Compiled.to_string(), "compiled");
        assert_eq!(CodegenStatus::Validated.to_string(), "validated");
        assert!(CodegenStatus::Failed("oops".into())
            .to_string()
            .contains("oops"));
    }

    #[test]
    fn codegen_status_terminal() {
        assert!(!CodegenStatus::Pending.is_terminal());
        assert!(!CodegenStatus::Generated.is_terminal());
        assert!(!CodegenStatus::Compiled.is_terminal());
        assert!(CodegenStatus::Validated.is_terminal());
        assert!(CodegenStatus::Failed("x".into()).is_terminal());
    }

    #[test]
    fn codegen_status_success() {
        assert!(CodegenStatus::Validated.is_success());
        assert!(!CodegenStatus::Pending.is_success());
        assert!(!CodegenStatus::Failed("x".into()).is_success());
    }

    #[test]
    fn generated_code_hash() {
        let h1 = GeneratedCode::compute_hash("fn foo() {}");
        let h2 = GeneratedCode::compute_hash("fn foo() {}");
        let h3 = GeneratedCode::compute_hash("fn bar() {}");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
        assert_eq!(h1.len(), 16); // 16 hex chars
    }

    #[test]
    fn generated_code_size() {
        let code = GeneratedCode {
            change_spec_index: 0,
            file_path: "src/foo.rs".into(),
            content: "fn foo() {}".into(),
            description: "test".into(),
            content_hash: "abc".into(),
            generated_at: Utc::now(),
        };
        assert_eq!(code.size_bytes(), 11);
    }

    #[test]
    fn generation_record_lifecycle() {
        let mut record = GenerationRecord::new("commit-1".into(), SelfModTier::Tier0Configuration);
        assert!(matches!(record.status, CodegenStatus::Pending));
        assert!(record.completed_at.is_none());

        record.mark_generated();
        assert!(matches!(record.status, CodegenStatus::Generated));

        record.mark_compiled();
        assert!(matches!(record.status, CodegenStatus::Compiled));

        record.mark_validated();
        assert!(matches!(record.status, CodegenStatus::Validated));
        assert!(record.completed_at.is_some());
        assert!(record.duration_ms.is_some());
    }

    #[test]
    fn generation_record_failure() {
        let mut record =
            GenerationRecord::new("commit-2".into(), SelfModTier::Tier1OperatorInternal);
        record.mark_failed("compilation error".into());
        assert!(matches!(record.status, CodegenStatus::Failed(_)));
        assert!(record.completed_at.is_some());
    }

    #[test]
    fn config_defaults() {
        let cfg = CodegenConfig::default();
        assert_eq!(cfg.max_generation_timeout_secs, 300);
        assert_eq!(cfg.max_compilation_timeout_secs, 120);
        assert!(cfg.require_all_tests_pass);
        assert!(cfg.require_performance_gates);
        assert!(cfg.enforce_safety_checks);
        assert_eq!(cfg.max_tracked_records, 256);
        assert_eq!(cfg.max_code_size_bytes, 1_048_576);
    }

    #[test]
    fn summary_default() {
        let s = CodegenSummary::default();
        assert_eq!(s.total, 0);
        assert_eq!(s.succeeded, 0);
        assert_eq!(s.failed, 0);
    }

    #[test]
    fn summary_display() {
        let s = CodegenSummary {
            total: 10,
            succeeded: 8,
            failed: 2,
            pending: 0,
            total_files_generated: 25,
            total_tests_run: 100,
            total_tests_passed: 95,
        };
        let display = s.to_string();
        assert!(display.contains("total=10"));
        assert!(display.contains("succeeded=8"));
    }
}
