//! GPU substrate implementation.
//!
//! Parallel operator execution with automatic CPU fallback.
//! Tracks vectorization and device memory usage for profiling.
//! Falls back to CPU when GPU is unavailable or the operation
//! is not GPU-compatible.

use chrono::Utc;

use crate::cpu::CpuSubstrate;
use crate::error::SalResult;
use crate::traits::SubstrateAbstractionLayer;
use crate::types::{
    ExecutionId, ExecutionResult, OperatorInput, OperatorOutput, ProvenanceId,
    SubstrateCapabilities, SubstrateId,
};

/// GPU substrate — parallel execution with CPU fallback.
pub struct GpuSubstrate {
    id: SubstrateId,
    /// Whether the GPU device is available (simulated).
    device_available: bool,
    /// Number of compute units.
    compute_units: u32,
    /// CPU fallback for non-GPU-compatible operations.
    cpu_fallback: CpuSubstrate,
}

impl GpuSubstrate {
    pub fn new() -> Self {
        Self {
            id: SubstrateId::new("gpu-default"),
            device_available: true,
            compute_units: 256,
            cpu_fallback: CpuSubstrate::new(),
        }
    }

    /// Create a GPU substrate with simulated device unavailability.
    pub fn unavailable() -> Self {
        Self {
            id: SubstrateId::new("gpu-unavailable"),
            device_available: false,
            compute_units: 0,
            cpu_fallback: CpuSubstrate::new(),
        }
    }

    /// Whether the GPU device is available.
    pub fn is_available(&self) -> bool {
        self.device_available
    }

    /// Number of compute units.
    pub fn compute_units(&self) -> u32 {
        self.compute_units
    }

    /// Whether an operator can run on GPU (simulated heuristic).
    fn is_gpu_compatible(&self, input: &OperatorInput) -> bool {
        // Simulated: operators with "parallel" or "batch" in context are GPU-compatible
        input.context.contains_key("parallel")
            || input.context.contains_key("batch")
            || input.arguments.len() > 2
    }
}

impl Default for GpuSubstrate {
    fn default() -> Self {
        Self::new()
    }
}

impl SubstrateAbstractionLayer for GpuSubstrate {
    fn execute_operator(&self, input: &OperatorInput) -> SalResult<OperatorOutput> {
        // Fallback to CPU if device not available or operation not compatible
        if !self.device_available || !self.is_gpu_compatible(input) {
            tracing::info!("GPU: falling back to CPU for operator '{}'", input.operator_name);
            return self.cpu_fallback.execute_operator(input);
        }

        let execution_start = Utc::now();

        // Simulated GPU parallel execution
        let result = format!(
            "gpu:{}({}) -> ok [{}cu]",
            input.operator_name,
            input.arguments.join(", "),
            self.compute_units
        );

        let elapsed = (Utc::now() - execution_start)
            .num_milliseconds()
            .max(0) as u64;

        Ok(OperatorOutput {
            result,
            execution_time_ms: elapsed,
            substrate_id: self.id.clone(),
            provenance_id: ProvenanceId::new(),
        })
    }

    fn execute_wlir(
        &self,
        module_name: &str,
        entry_function: &str,
        args: Vec<String>,
    ) -> SalResult<ExecutionResult> {
        // WLIR always falls back to CPU interpretation on GPU substrate
        if !self.device_available {
            tracing::info!("GPU: falling back to CPU for WLIR execution");
            return self.cpu_fallback.execute_wlir(module_name, entry_function, args);
        }

        let execution_id = ExecutionId::new();
        let execution_start = Utc::now();

        // Simulated GPU-accelerated WLIR execution
        let output_values = vec![format!(
            "gpu:{}::{}({}) -> result [vectorized]",
            module_name,
            entry_function,
            args.join(", ")
        )];

        let instructions_executed = 100 + (args.len() as u64 * 10);
        let memory_used = 8192 + (args.len() as u64 * 512); // GPU uses more memory

        let elapsed = (Utc::now() - execution_start)
            .num_milliseconds()
            .max(0) as u64;

        Ok(ExecutionResult {
            execution_id,
            substrate_id: self.id.clone(),
            output_values,
            execution_time_ms: elapsed,
            instructions_executed,
            memory_used_bytes: memory_used,
            provenance_id: ProvenanceId::new(),
        })
    }

    fn capabilities(&self) -> SubstrateCapabilities {
        if !self.device_available {
            return self.cpu_fallback.capabilities();
        }
        SubstrateCapabilities {
            parallelism: self.compute_units,
            memory_bytes: 16 * 1024 * 1024 * 1024, // 16 GiB VRAM
            base_latency_us: 10, // GPU is faster for parallel work
            supports_wlir: true,
            supports_gpu_operators: true,
            features: Default::default(),
        }
    }

    fn record_provenance(
        &self,
        operation: &str,
        input_hash: &str,
        output_hash: &str,
        execution_time_ms: u64,
    ) -> SalResult<ProvenanceId> {
        // Provenance is always recorded on CPU side
        self.cpu_fallback
            .record_provenance(operation, input_hash, output_hash, execution_time_ms)
    }

    fn substrate_id(&self) -> SubstrateId {
        self.id.clone()
    }

    fn name(&self) -> &str {
        "gpu-substrate"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn gpu_substrate_creation() {
        let gpu = GpuSubstrate::new();
        assert!(gpu.is_available());
        assert_eq!(gpu.compute_units(), 256);
        assert_eq!(gpu.name(), "gpu-substrate");
    }

    #[test]
    fn gpu_unavailable_creation() {
        let gpu = GpuSubstrate::unavailable();
        assert!(!gpu.is_available());
        assert_eq!(gpu.compute_units(), 0);
    }

    #[test]
    fn gpu_execute_operator_gpu_compatible() {
        let gpu = GpuSubstrate::new();
        let mut context = HashMap::new();
        context.insert("parallel".into(), "true".into());
        let input = OperatorInput {
            operator_name: "batch_transfer".into(),
            arguments: vec!["100".into()],
            context,
        };
        let output = gpu.execute_operator(&input).unwrap();
        assert!(output.result.contains("gpu:"));
        assert!(output.result.contains("256cu"));
    }

    #[test]
    fn gpu_execute_operator_cpu_fallback() {
        let gpu = GpuSubstrate::new();
        let input = OperatorInput {
            operator_name: "simple".into(),
            arguments: vec!["1".into()],
            context: HashMap::new(),
        };
        // Not GPU-compatible, should fall back to CPU
        let output = gpu.execute_operator(&input).unwrap();
        assert!(output.result.contains("cpu:"));
    }

    #[test]
    fn gpu_unavailable_falls_back_to_cpu() {
        let gpu = GpuSubstrate::unavailable();
        let mut context = HashMap::new();
        context.insert("parallel".into(), "true".into());
        let input = OperatorInput {
            operator_name: "batch".into(),
            arguments: vec!["data".into()],
            context,
        };
        let output = gpu.execute_operator(&input).unwrap();
        // Should fall back to CPU even with parallel context
        assert!(output.result.contains("cpu:"));
    }

    #[test]
    fn gpu_execute_wlir() {
        let gpu = GpuSubstrate::new();
        let result = gpu
            .execute_wlir("mod", "main", vec!["arg".into()])
            .unwrap();
        assert!(result.output_values[0].contains("gpu:"));
        assert!(result.output_values[0].contains("vectorized"));
    }

    #[test]
    fn gpu_execute_wlir_unavailable_fallback() {
        let gpu = GpuSubstrate::unavailable();
        let result = gpu
            .execute_wlir("mod", "main", vec![])
            .unwrap();
        assert!(result.output_values[0].contains("cpu:"));
    }

    #[test]
    fn gpu_capabilities_available() {
        let gpu = GpuSubstrate::new();
        let caps = gpu.capabilities();
        assert_eq!(caps.parallelism, 256);
        assert!(caps.supports_gpu_operators);
    }

    #[test]
    fn gpu_capabilities_unavailable_falls_back() {
        let gpu = GpuSubstrate::unavailable();
        let caps = gpu.capabilities();
        // Falls back to CPU capabilities
        assert!(!caps.supports_gpu_operators);
        assert_eq!(caps.parallelism, 1);
    }

    #[test]
    fn gpu_record_provenance() {
        let gpu = GpuSubstrate::new();
        let prov_id = gpu
            .record_provenance("op", "in", "out", 5)
            .unwrap();
        assert!(prov_id.to_string().starts_with("provenance:"));
    }
}
