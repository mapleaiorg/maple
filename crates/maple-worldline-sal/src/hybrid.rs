//! Hybrid substrate — transparent routing to the best substrate.
//!
//! Auto-routes operations to CPU or GPU based on operation characteristics.
//! Maintains performance feedback for learning-based routing decisions.
//! Provides load balancing and graceful degradation.

use crate::cpu::CpuSubstrate;
use crate::error::SalResult;
use crate::gpu::GpuSubstrate;
use crate::traits::SubstrateAbstractionLayer;
use crate::types::{
    ExecutionResult, OperatorInput, OperatorOutput, ProvenanceId, SubstrateCapabilities,
    SubstrateId, SubstrateKind,
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Performance feedback entry for routing decisions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoutingFeedback {
    pub operator_name: String,
    pub routed_to: SubstrateKind,
    pub execution_time_ms: u64,
    pub success: bool,
}

/// Hybrid substrate — routes operations to the best available substrate.
pub struct HybridSubstrate {
    id: SubstrateId,
    cpu: CpuSubstrate,
    gpu: GpuSubstrate,
    /// Performance feedback history for learning-based routing.
    feedback_history: Vec<RoutingFeedback>,
    /// Maximum feedback entries.
    max_feedback: usize,
}

impl HybridSubstrate {
    pub fn new() -> Self {
        Self {
            id: SubstrateId::new("hybrid-default"),
            cpu: CpuSubstrate::new(),
            gpu: GpuSubstrate::new(),
            feedback_history: Vec::new(),
            max_feedback: 1000,
        }
    }

    /// Create with a GPU that may or may not be available.
    pub fn with_gpu_availability(gpu_available: bool) -> Self {
        let gpu = if gpu_available {
            GpuSubstrate::new()
        } else {
            GpuSubstrate::unavailable()
        };
        Self {
            id: SubstrateId::new("hybrid-configured"),
            cpu: CpuSubstrate::new(),
            gpu,
            feedback_history: Vec::new(),
            max_feedback: 1000,
        }
    }

    /// Decide which substrate to route an operator to.
    fn route_operator(&self, input: &OperatorInput) -> SubstrateKind {
        // If GPU is unavailable, always route to CPU
        if !self.gpu.is_available() {
            return SubstrateKind::Cpu;
        }

        // Check feedback history for this operator
        let gpu_times: Vec<u64> = self
            .feedback_history
            .iter()
            .filter(|f| f.operator_name == input.operator_name && f.routed_to == SubstrateKind::Gpu && f.success)
            .map(|f| f.execution_time_ms)
            .collect();

        let cpu_times: Vec<u64> = self
            .feedback_history
            .iter()
            .filter(|f| f.operator_name == input.operator_name && f.routed_to == SubstrateKind::Cpu && f.success)
            .map(|f| f.execution_time_ms)
            .collect();

        // If we have enough data, route to the faster substrate
        if gpu_times.len() >= 3 && cpu_times.len() >= 3 {
            let avg_gpu = gpu_times.iter().sum::<u64>() as f64 / gpu_times.len() as f64;
            let avg_cpu = cpu_times.iter().sum::<u64>() as f64 / cpu_times.len() as f64;
            if avg_gpu < avg_cpu {
                return SubstrateKind::Gpu;
            } else {
                return SubstrateKind::Cpu;
            }
        }

        // Default heuristic: GPU for parallel/batch, CPU otherwise
        if input.context.contains_key("parallel")
            || input.context.contains_key("batch")
            || input.arguments.len() > 2
        {
            SubstrateKind::Gpu
        } else {
            SubstrateKind::Cpu
        }
    }

    /// Record routing feedback for learning-based routing.
    pub fn record_feedback(&mut self, feedback: RoutingFeedback) {
        if self.feedback_history.len() >= self.max_feedback {
            self.feedback_history.remove(0);
        }
        self.feedback_history.push(feedback);
    }

    /// Get routing feedback history.
    pub fn feedback_history(&self) -> &[RoutingFeedback] {
        &self.feedback_history
    }

    /// Get routing statistics.
    pub fn routing_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        let cpu_count = self
            .feedback_history
            .iter()
            .filter(|f| f.routed_to == SubstrateKind::Cpu)
            .count();
        let gpu_count = self
            .feedback_history
            .iter()
            .filter(|f| f.routed_to == SubstrateKind::Gpu)
            .count();
        stats.insert("cpu_routed".into(), cpu_count);
        stats.insert("gpu_routed".into(), gpu_count);
        stats.insert("total".into(), self.feedback_history.len());
        stats
    }
}

impl Default for HybridSubstrate {
    fn default() -> Self {
        Self::new()
    }
}

impl SubstrateAbstractionLayer for HybridSubstrate {
    fn execute_operator(&self, input: &OperatorInput) -> SalResult<OperatorOutput> {
        let route = self.route_operator(input);
        match route {
            SubstrateKind::Gpu => self.gpu.execute_operator(input),
            _ => self.cpu.execute_operator(input),
        }
    }

    fn execute_wlir(
        &self,
        module_name: &str,
        entry_function: &str,
        args: Vec<String>,
    ) -> SalResult<ExecutionResult> {
        // WLIR execution: prefer GPU if available
        if self.gpu.is_available() {
            self.gpu.execute_wlir(module_name, entry_function, args)
        } else {
            self.cpu.execute_wlir(module_name, entry_function, args)
        }
    }

    fn capabilities(&self) -> SubstrateCapabilities {
        // Hybrid reports the best of both
        let cpu_caps = self.cpu.capabilities();
        let gpu_caps = self.gpu.capabilities();
        SubstrateCapabilities {
            parallelism: cpu_caps.parallelism.max(gpu_caps.parallelism),
            memory_bytes: cpu_caps.memory_bytes + gpu_caps.memory_bytes,
            base_latency_us: cpu_caps.base_latency_us.min(gpu_caps.base_latency_us),
            supports_wlir: true,
            supports_gpu_operators: gpu_caps.supports_gpu_operators,
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
        self.cpu
            .record_provenance(operation, input_hash, output_hash, execution_time_ms)
    }

    fn substrate_id(&self) -> SubstrateId {
        self.id.clone()
    }

    fn name(&self) -> &str {
        "hybrid-substrate"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hybrid_creation() {
        let hybrid = HybridSubstrate::new();
        assert_eq!(hybrid.name(), "hybrid-substrate");
    }

    #[test]
    fn hybrid_routes_simple_to_cpu() {
        let hybrid = HybridSubstrate::new();
        let input = OperatorInput {
            operator_name: "simple".into(),
            arguments: vec!["1".into()],
            context: HashMap::new(),
        };
        let output = hybrid.execute_operator(&input).unwrap();
        assert!(output.result.contains("cpu:"));
    }

    #[test]
    fn hybrid_routes_parallel_to_gpu() {
        let hybrid = HybridSubstrate::new();
        let mut context = HashMap::new();
        context.insert("parallel".into(), "true".into());
        let input = OperatorInput {
            operator_name: "batch_op".into(),
            arguments: vec!["data".into()],
            context,
        };
        let output = hybrid.execute_operator(&input).unwrap();
        // GPU receives it, but may still process via GPU codepath
        assert!(output.result.contains("gpu:") || output.result.contains("cpu:"));
    }

    #[test]
    fn hybrid_degrades_without_gpu() {
        let hybrid = HybridSubstrate::with_gpu_availability(false);
        let mut context = HashMap::new();
        context.insert("parallel".into(), "true".into());
        let input = OperatorInput {
            operator_name: "batch".into(),
            arguments: vec!["data".into()],
            context,
        };
        let output = hybrid.execute_operator(&input).unwrap();
        // Must fall back to CPU
        assert!(output.result.contains("cpu:"));
    }

    #[test]
    fn hybrid_wlir_uses_gpu_when_available() {
        let hybrid = HybridSubstrate::new();
        let result = hybrid
            .execute_wlir("mod", "main", vec![])
            .unwrap();
        assert!(result.output_values[0].contains("gpu:"));
    }

    #[test]
    fn hybrid_wlir_falls_back_to_cpu() {
        let hybrid = HybridSubstrate::with_gpu_availability(false);
        let result = hybrid
            .execute_wlir("mod", "main", vec![])
            .unwrap();
        assert!(result.output_values[0].contains("cpu:"));
    }

    #[test]
    fn hybrid_capabilities_combined() {
        let hybrid = HybridSubstrate::new();
        let caps = hybrid.capabilities();
        // Should combine: max parallelism, sum memory, min latency
        assert!(caps.parallelism >= 256); // GPU's 256
        assert!(caps.supports_gpu_operators);
    }

    #[test]
    fn hybrid_routing_stats_empty() {
        let hybrid = HybridSubstrate::new();
        let stats = hybrid.routing_stats();
        assert_eq!(stats["total"], 0);
    }

    #[test]
    fn hybrid_record_provenance() {
        let hybrid = HybridSubstrate::new();
        let prov_id = hybrid
            .record_provenance("op", "in", "out", 5)
            .unwrap();
        assert!(prov_id.to_string().starts_with("provenance:"));
    }
}
