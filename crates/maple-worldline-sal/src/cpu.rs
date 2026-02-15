//! CPU substrate implementation.
//!
//! Default, portable substrate that executes operators and WLIR modules
//! on the host CPU. Sequential or multi-threaded. Serves as the
//! fallback for all other substrates.

use chrono::Utc;

use crate::error::SalResult;
use crate::traits::SubstrateAbstractionLayer;
use crate::types::{
    ExecutionId, ExecutionResult, OperatorInput, OperatorOutput, ProvenanceId,
    SubstrateCapabilities, SubstrateId,
};

/// CPU substrate — sequential execution on the host processor.
pub struct CpuSubstrate {
    id: SubstrateId,
    thread_count: u32,
}

impl CpuSubstrate {
    pub fn new() -> Self {
        Self {
            id: SubstrateId::new("cpu-default"),
            thread_count: 1,
        }
    }

    pub fn with_threads(thread_count: u32) -> Self {
        Self {
            id: SubstrateId::new(&format!("cpu-{}t", thread_count)),
            thread_count: thread_count.max(1),
        }
    }

    /// Thread count for this CPU substrate.
    pub fn thread_count(&self) -> u32 {
        self.thread_count
    }
}

impl Default for CpuSubstrate {
    fn default() -> Self {
        Self::new()
    }
}

impl SubstrateAbstractionLayer for CpuSubstrate {
    fn execute_operator(&self, input: &OperatorInput) -> SalResult<OperatorOutput> {
        let execution_start = Utc::now();

        // Simulated CPU execution
        let result = format!(
            "cpu:{}({}) -> ok",
            input.operator_name,
            input.arguments.join(", ")
        );

        let elapsed = (Utc::now() - execution_start)
            .num_milliseconds()
            .max(0) as u64;

        let prov_id = ProvenanceId::new();

        Ok(OperatorOutput {
            result,
            execution_time_ms: elapsed,
            substrate_id: self.id.clone(),
            provenance_id: prov_id,
        })
    }

    fn execute_wlir(
        &self,
        module_name: &str,
        entry_function: &str,
        args: Vec<String>,
    ) -> SalResult<ExecutionResult> {
        let execution_id = ExecutionId::new();
        let execution_start = Utc::now();

        // Simulated WLIR interpretation on CPU
        let output_values = vec![format!(
            "cpu:{}::{}({}) -> result",
            module_name,
            entry_function,
            args.join(", ")
        )];

        let instructions_executed = 100 + (args.len() as u64 * 10);
        let memory_used = 4096 + (args.len() as u64 * 256);

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
        SubstrateCapabilities {
            parallelism: self.thread_count,
            memory_bytes: 8 * 1024 * 1024 * 1024, // 8 GiB
            base_latency_us: 50,
            supports_wlir: true,
            supports_gpu_operators: false,
            features: Default::default(),
        }
    }

    fn record_provenance(
        &self,
        _operation: &str,
        _input_hash: &str,
        _output_hash: &str,
        _execution_time_ms: u64,
    ) -> SalResult<ProvenanceId> {
        Ok(ProvenanceId::new())
    }

    fn substrate_id(&self) -> SubstrateId {
        self.id.clone()
    }

    fn name(&self) -> &str {
        "cpu-substrate"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn cpu_substrate_creation() {
        let cpu = CpuSubstrate::new();
        assert_eq!(cpu.name(), "cpu-substrate");
        assert_eq!(cpu.thread_count(), 1);
    }

    #[test]
    fn cpu_substrate_with_threads() {
        let cpu = CpuSubstrate::with_threads(4);
        assert_eq!(cpu.thread_count(), 4);
        assert!(cpu.substrate_id().to_string().contains("cpu-4t"));
    }

    #[test]
    fn cpu_execute_operator() {
        let cpu = CpuSubstrate::new();
        let input = OperatorInput {
            operator_name: "transfer".into(),
            arguments: vec!["100".into(), "USD".into()],
            context: HashMap::new(),
        };
        let output = cpu.execute_operator(&input).unwrap();
        assert!(output.result.contains("transfer"));
        assert!(output.result.contains("100"));
        assert_eq!(output.substrate_id, cpu.substrate_id());
    }

    #[test]
    fn cpu_execute_wlir() {
        let cpu = CpuSubstrate::new();
        let result = cpu
            .execute_wlir("test-mod", "main", vec!["arg1".into()])
            .unwrap();
        assert!(result.output_values[0].contains("test-mod"));
        assert!(result.output_values[0].contains("main"));
        assert!(result.instructions_executed > 0);
        assert!(result.memory_used_bytes > 0);
    }

    #[test]
    fn cpu_capabilities() {
        let cpu = CpuSubstrate::new();
        let caps = cpu.capabilities();
        assert_eq!(caps.parallelism, 1);
        assert!(caps.supports_wlir);
        assert!(!caps.supports_gpu_operators);
    }

    #[test]
    fn cpu_capabilities_multi_thread() {
        let cpu = CpuSubstrate::with_threads(8);
        let caps = cpu.capabilities();
        assert_eq!(caps.parallelism, 8);
    }

    #[test]
    fn cpu_record_provenance() {
        let cpu = CpuSubstrate::new();
        let prov_id = cpu
            .record_provenance("op", "in-hash", "out-hash", 5)
            .unwrap();
        assert!(prov_id.to_string().starts_with("provenance:"));
    }

    #[test]
    fn cpu_substrate_default() {
        let cpu = CpuSubstrate::default();
        assert_eq!(cpu.thread_count(), 1);
    }
}
