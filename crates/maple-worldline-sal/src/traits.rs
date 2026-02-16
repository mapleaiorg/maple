//! SAL trait interface.
//!
//! Defines the `SubstrateAbstractionLayer` trait that all substrate
//! implementations must satisfy. This provides platform independence —
//! WLIR instructions execute identically regardless of substrate (I.SAL-1).

use crate::error::SalResult;
use crate::types::{
    ExecutionResult, OperatorInput, OperatorOutput, ProvenanceId, SubstrateCapabilities,
    SubstrateId,
};

/// The core substrate abstraction trait.
///
/// Any computational substrate (CPU, GPU, FPGA, hybrid) implements this
/// trait. The SAL guarantees:
///
/// - **I.SAL-1** (Substrate Opacity): Identical results regardless of substrate
/// - **I.SAL-2** (Commitment Gate Preservation): Gates never optimized away
/// - **I.SAL-3** (Provenance Completeness): All operations recorded
/// - **I.SAL-4** (Resource Limits): Graceful failure on exhaustion
pub trait SubstrateAbstractionLayer: Send + Sync {
    /// Execute an operator on this substrate.
    fn execute_operator(&self, input: &OperatorInput) -> SalResult<OperatorOutput>;

    /// Execute a WLIR module entry point on this substrate.
    fn execute_wlir(
        &self,
        module_name: &str,
        entry_function: &str,
        args: Vec<String>,
    ) -> SalResult<ExecutionResult>;

    /// Report this substrate's capabilities.
    fn capabilities(&self) -> SubstrateCapabilities;

    /// Record a provenance entry for an operation (I.SAL-3).
    fn record_provenance(
        &self,
        operation: &str,
        input_hash: &str,
        output_hash: &str,
        execution_time_ms: u64,
    ) -> SalResult<ProvenanceId>;

    /// Unique identifier for this substrate instance.
    fn substrate_id(&self) -> SubstrateId;

    /// Human-readable name.
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::CpuSubstrate;
    use std::collections::HashMap;

    #[test]
    fn trait_object_can_be_boxed() {
        let substrate: Box<dyn SubstrateAbstractionLayer> = Box::new(CpuSubstrate::new());
        assert_eq!(substrate.name(), "cpu-substrate");
    }

    #[test]
    fn trait_provides_capabilities() {
        let substrate: Box<dyn SubstrateAbstractionLayer> = Box::new(CpuSubstrate::new());
        let caps = substrate.capabilities();
        assert!(caps.supports_wlir);
    }

    #[test]
    fn trait_provides_substrate_id() {
        let substrate: Box<dyn SubstrateAbstractionLayer> = Box::new(CpuSubstrate::new());
        let id = substrate.substrate_id();
        assert!(id.to_string().contains("cpu"));
    }

    #[test]
    fn trait_execute_operator() {
        let substrate: Box<dyn SubstrateAbstractionLayer> = Box::new(CpuSubstrate::new());
        let input = OperatorInput {
            operator_name: "noop".into(),
            arguments: vec![],
            context: HashMap::new(),
        };
        let output = substrate.execute_operator(&input).unwrap();
        assert!(!output.result.is_empty());
    }

    #[test]
    fn trait_execute_wlir() {
        let substrate: Box<dyn SubstrateAbstractionLayer> = Box::new(CpuSubstrate::new());
        let result = substrate
            .execute_wlir("test-module", "main", vec![])
            .unwrap();
        assert!(!result.output_values.is_empty());
    }

    #[test]
    fn trait_record_provenance() {
        let substrate: Box<dyn SubstrateAbstractionLayer> = Box::new(CpuSubstrate::new());
        let prov_id = substrate
            .record_provenance("test-op", "hash-in", "hash-out", 5)
            .unwrap();
        assert!(prov_id.to_string().starts_with("provenance:"));
    }
}
