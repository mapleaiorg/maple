//! Core types for the Substrate Abstraction Layer.
//!
//! Defines identifiers, capabilities, resource limits, execution results,
//! provenance records, operator input/output, and configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Identifiers ─────────────────────────────────────────────────────

/// Unique identifier for a substrate instance.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubstrateId(pub String);

impl SubstrateId {
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl std::fmt::Display for SubstrateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "substrate:{}", self.0)
    }
}

/// Unique identifier for a provenance record.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProvenanceId(pub String);

impl ProvenanceId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for ProvenanceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProvenanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "provenance:{}", self.0)
    }
}

/// Unique identifier for an execution.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecutionId(pub String);

impl ExecutionId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for ExecutionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ExecutionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "exec:{}", self.0)
    }
}

// ── Substrate Kind ──────────────────────────────────────────────────

/// Kind of substrate.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubstrateKind {
    Cpu,
    Gpu,
    Fpga,
    Hybrid,
    Custom(String),
}

impl std::fmt::Display for SubstrateKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cpu => write!(f, "cpu"),
            Self::Gpu => write!(f, "gpu"),
            Self::Fpga => write!(f, "fpga"),
            Self::Hybrid => write!(f, "hybrid"),
            Self::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

// ── Capabilities ────────────────────────────────────────────────────

/// Capabilities reported by a substrate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubstrateCapabilities {
    /// Maximum parallel execution lanes.
    pub parallelism: u32,
    /// Available memory in bytes.
    pub memory_bytes: u64,
    /// Estimated latency for simple operations (microseconds).
    pub base_latency_us: u64,
    /// Whether this substrate supports WLIR execution.
    pub supports_wlir: bool,
    /// Whether this substrate supports GPU-accelerated operators.
    pub supports_gpu_operators: bool,
    /// Additional capability flags.
    pub features: HashMap<String, bool>,
}

impl Default for SubstrateCapabilities {
    fn default() -> Self {
        Self {
            parallelism: 1,
            memory_bytes: 1024 * 1024 * 1024, // 1 GiB
            base_latency_us: 100,
            supports_wlir: true,
            supports_gpu_operators: false,
            features: HashMap::new(),
        }
    }
}

impl std::fmt::Display for SubstrateCapabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Capabilities(parallelism={}, memory={}MB, latency={}us, wlir={}, gpu={})",
            self.parallelism,
            self.memory_bytes / (1024 * 1024),
            self.base_latency_us,
            self.supports_wlir,
            self.supports_gpu_operators,
        )
    }
}

// ── Resource Limits ─────────────────────────────────────────────────

/// Resource limits for substrate execution (I.SAL-4).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory in bytes.
    pub max_memory_bytes: u64,
    /// Maximum execution time in milliseconds.
    pub max_execution_time_ms: u64,
    /// Maximum number of instructions to execute.
    pub max_instructions: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 512 * 1024 * 1024, // 512 MiB
            max_execution_time_ms: 30_000,         // 30 seconds
            max_instructions: 10_000_000,          // 10M instructions
        }
    }
}

// ── Operator Input / Output ─────────────────────────────────────────

/// Input to an operator execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperatorInput {
    pub operator_name: String,
    pub arguments: Vec<String>,
    pub context: HashMap<String, String>,
}

/// Output of an operator execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperatorOutput {
    pub result: String,
    pub execution_time_ms: u64,
    pub substrate_id: SubstrateId,
    pub provenance_id: ProvenanceId,
}

// ── Execution Result ────────────────────────────────────────────────

/// Result of executing WLIR or an operator on a substrate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub execution_id: ExecutionId,
    pub substrate_id: SubstrateId,
    pub output_values: Vec<String>,
    pub execution_time_ms: u64,
    pub instructions_executed: u64,
    pub memory_used_bytes: u64,
    pub provenance_id: ProvenanceId,
}

// ── Provenance Record ───────────────────────────────────────────────

/// A provenance record for substrate operations (I.SAL-3).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubstrateProvenanceRecord {
    pub provenance_id: ProvenanceId,
    pub execution_id: ExecutionId,
    pub substrate_id: SubstrateId,
    pub operation: String,
    pub input_hash: String,
    pub output_hash: String,
    pub execution_time_ms: u64,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
}

// ── Configuration ───────────────────────────────────────────────────

/// Configuration for the SAL engine.
#[derive(Clone, Debug)]
pub struct SalConfig {
    /// Default substrate kind.
    pub default_substrate: SubstrateKind,
    /// Resource limits for all executions.
    pub resource_limits: ResourceLimits,
    /// Whether to enforce commitment checks before execution.
    pub enforce_commitment_checks: bool,
    /// Whether to record provenance for every operation.
    pub record_provenance: bool,
    /// Maximum tracked execution records.
    pub max_tracked_records: usize,
}

impl Default for SalConfig {
    fn default() -> Self {
        Self {
            default_substrate: SubstrateKind::Cpu,
            resource_limits: ResourceLimits::default(),
            enforce_commitment_checks: true,
            record_provenance: true,
            max_tracked_records: 256,
        }
    }
}

// ── Summary ─────────────────────────────────────────────────────────

/// Summary statistics for SAL operations.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SalSummary {
    pub total_executions: usize,
    pub successful_executions: usize,
    pub failed_executions: usize,
    pub total_provenance_records: usize,
    pub average_execution_time_ms: f64,
}

impl std::fmt::Display for SalSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SAL(executions={}, success={}, failed={}, provenance={}, avg_time={:.1}ms)",
            self.total_executions,
            self.successful_executions,
            self.failed_executions,
            self.total_provenance_records,
            self.average_execution_time_ms,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substrate_id_display() {
        let id = SubstrateId::new("cpu-0");
        assert_eq!(id.to_string(), "substrate:cpu-0");
    }

    #[test]
    fn provenance_id_unique() {
        let a = ProvenanceId::new();
        let b = ProvenanceId::new();
        assert_ne!(a, b);
        assert!(a.to_string().starts_with("provenance:"));
    }

    #[test]
    fn execution_id_display() {
        let id = ExecutionId::new();
        assert!(id.to_string().starts_with("exec:"));
    }

    #[test]
    fn substrate_kind_display() {
        assert_eq!(SubstrateKind::Cpu.to_string(), "cpu");
        assert_eq!(SubstrateKind::Gpu.to_string(), "gpu");
        assert_eq!(SubstrateKind::Fpga.to_string(), "fpga");
        assert_eq!(SubstrateKind::Hybrid.to_string(), "hybrid");
        assert_eq!(SubstrateKind::Custom("tpu".into()).to_string(), "custom:tpu");
    }

    #[test]
    fn capabilities_default() {
        let caps = SubstrateCapabilities::default();
        assert_eq!(caps.parallelism, 1);
        assert!(caps.supports_wlir);
        assert!(!caps.supports_gpu_operators);
    }

    #[test]
    fn capabilities_display() {
        let caps = SubstrateCapabilities::default();
        let display = caps.to_string();
        assert!(display.contains("parallelism=1"));
        assert!(display.contains("wlir=true"));
    }

    #[test]
    fn resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_execution_time_ms, 30_000);
        assert_eq!(limits.max_instructions, 10_000_000);
    }

    #[test]
    fn sal_config_default() {
        let config = SalConfig::default();
        assert_eq!(config.default_substrate, SubstrateKind::Cpu);
        assert!(config.enforce_commitment_checks);
        assert!(config.record_provenance);
        assert_eq!(config.max_tracked_records, 256);
    }

    #[test]
    fn summary_display() {
        let s = SalSummary {
            total_executions: 10,
            successful_executions: 8,
            failed_executions: 2,
            total_provenance_records: 8,
            average_execution_time_ms: 12.5,
        };
        let display = s.to_string();
        assert!(display.contains("executions=10"));
        assert!(display.contains("success=8"));
        assert!(display.contains("12.5ms"));
    }

    #[test]
    fn operator_input_output() {
        let input = OperatorInput {
            operator_name: "transfer".into(),
            arguments: vec!["100".into(), "USD".into()],
            context: HashMap::new(),
        };
        assert_eq!(input.operator_name, "transfer");

        let output = OperatorOutput {
            result: "ok".into(),
            execution_time_ms: 5,
            substrate_id: SubstrateId::new("cpu-0"),
            provenance_id: ProvenanceId::new(),
        };
        assert_eq!(output.result, "ok");
    }

    #[test]
    fn execution_result_construction() {
        let result = ExecutionResult {
            execution_id: ExecutionId::new(),
            substrate_id: SubstrateId::new("gpu-0"),
            output_values: vec!["42".into()],
            execution_time_ms: 10,
            instructions_executed: 1000,
            memory_used_bytes: 4096,
            provenance_id: ProvenanceId::new(),
        };
        assert_eq!(result.output_values.len(), 1);
        assert_eq!(result.instructions_executed, 1000);
    }
}
