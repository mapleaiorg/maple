//! SAL engine with bounded execution history.
//!
//! `SalEngine` wraps a substrate implementation and maintains a
//! bounded FIFO of `ExecutionRecord`s. Provides `execute_operator()`,
//! `execute_wlir()`, `find()`, `all_records()`, and `summary()`.

use std::collections::VecDeque;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::SalResult;
use crate::traits::SubstrateAbstractionLayer;
use crate::types::{
    ExecutionId, ExecutionResult, OperatorInput, OperatorOutput, ProvenanceId, SalConfig,
    SalSummary, SubstrateId,
};

// ── Execution Record ────────────────────────────────────────────────

/// Record of a single execution on a substrate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub execution_id: ExecutionId,
    pub substrate_id: SubstrateId,
    pub operation: String,
    pub success: bool,
    pub execution_time_ms: u64,
    pub provenance_id: Option<ProvenanceId>,
    pub error_message: Option<String>,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
}

// ── SAL Engine ──────────────────────────────────────────────────────

/// Engine that wraps a substrate with bounded execution history.
pub struct SalEngine {
    substrate: Box<dyn SubstrateAbstractionLayer>,
    records: VecDeque<ExecutionRecord>,
    max_records: usize,
    record_provenance: bool,
}

impl SalEngine {
    /// Create with a specific substrate and default configuration.
    pub fn new(substrate: Box<dyn SubstrateAbstractionLayer>) -> Self {
        let config = SalConfig::default();
        Self {
            substrate,
            records: VecDeque::new(),
            max_records: config.max_tracked_records,
            record_provenance: config.record_provenance,
        }
    }

    /// Create with a specific substrate and configuration.
    pub fn with_config(
        substrate: Box<dyn SubstrateAbstractionLayer>,
        config: &SalConfig,
    ) -> Self {
        Self {
            substrate,
            records: VecDeque::new(),
            max_records: config.max_tracked_records,
            record_provenance: config.record_provenance,
        }
    }

    /// Execute an operator, recording the result.
    pub fn execute_operator(&mut self, input: &OperatorInput) -> SalResult<OperatorOutput> {
        let result = self.substrate.execute_operator(input);

        match &result {
            Ok(output) => {
                // Record provenance if enabled
                let prov_id = if self.record_provenance {
                    Some(output.provenance_id.clone())
                } else {
                    None
                };
                self.push_record(ExecutionRecord {
                    execution_id: ExecutionId::new(),
                    substrate_id: output.substrate_id.clone(),
                    operation: format!("operator:{}", input.operator_name),
                    success: true,
                    execution_time_ms: output.execution_time_ms,
                    provenance_id: prov_id,
                    error_message: None,
                    recorded_at: Utc::now(),
                });
            }
            Err(e) => {
                self.push_record(ExecutionRecord {
                    execution_id: ExecutionId::new(),
                    substrate_id: self.substrate.substrate_id(),
                    operation: format!("operator:{}", input.operator_name),
                    success: false,
                    execution_time_ms: 0,
                    provenance_id: None,
                    error_message: Some(e.to_string()),
                    recorded_at: Utc::now(),
                });
            }
        }

        result
    }

    /// Execute a WLIR module, recording the result.
    pub fn execute_wlir(
        &mut self,
        module_name: &str,
        entry_function: &str,
        args: Vec<String>,
    ) -> SalResult<ExecutionResult> {
        let result = self.substrate.execute_wlir(module_name, entry_function, args);

        match &result {
            Ok(exec_result) => {
                let prov_id = if self.record_provenance {
                    Some(exec_result.provenance_id.clone())
                } else {
                    None
                };
                self.push_record(ExecutionRecord {
                    execution_id: exec_result.execution_id.clone(),
                    substrate_id: exec_result.substrate_id.clone(),
                    operation: format!("wlir:{}::{}", module_name, entry_function),
                    success: true,
                    execution_time_ms: exec_result.execution_time_ms,
                    provenance_id: prov_id,
                    error_message: None,
                    recorded_at: Utc::now(),
                });
            }
            Err(e) => {
                self.push_record(ExecutionRecord {
                    execution_id: ExecutionId::new(),
                    substrate_id: self.substrate.substrate_id(),
                    operation: format!("wlir:{}::{}", module_name, entry_function),
                    success: false,
                    execution_time_ms: 0,
                    provenance_id: None,
                    error_message: Some(e.to_string()),
                    recorded_at: Utc::now(),
                });
            }
        }

        result
    }

    /// Push a record with bounded FIFO eviction.
    fn push_record(&mut self, record: ExecutionRecord) {
        if self.records.len() >= self.max_records {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }

    /// Find a record by execution ID.
    pub fn find(&self, id: &ExecutionId) -> Option<&ExecutionRecord> {
        self.records.iter().find(|r| &r.execution_id == id)
    }

    /// All execution records in order.
    pub fn all_records(&self) -> &VecDeque<ExecutionRecord> {
        &self.records
    }

    /// Number of tracked records.
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// Access the underlying substrate.
    pub fn substrate(&self) -> &dyn SubstrateAbstractionLayer {
        self.substrate.as_ref()
    }

    /// Generate summary statistics.
    pub fn summary(&self) -> SalSummary {
        let total = self.records.len();
        let successful = self.records.iter().filter(|r| r.success).count();
        let failed = total - successful;
        let provenance_count = self
            .records
            .iter()
            .filter(|r| r.provenance_id.is_some())
            .count();

        let total_time: u64 = self
            .records
            .iter()
            .filter(|r| r.success)
            .map(|r| r.execution_time_ms)
            .sum();

        let avg_time = if successful > 0 {
            total_time as f64 / successful as f64
        } else {
            0.0
        };

        SalSummary {
            total_executions: total,
            successful_executions: successful,
            failed_executions: failed,
            total_provenance_records: provenance_count,
            average_execution_time_ms: avg_time,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::CpuSubstrate;
    use crate::gpu::GpuSubstrate;
    use crate::types::SalConfig;
    use std::collections::HashMap;

    fn make_engine() -> SalEngine {
        SalEngine::new(Box::new(CpuSubstrate::new()))
    }

    fn sample_input(name: &str) -> OperatorInput {
        OperatorInput {
            operator_name: name.into(),
            arguments: vec!["arg1".into()],
            context: HashMap::new(),
        }
    }

    #[test]
    fn engine_execute_operator_stores_record() {
        let mut engine = make_engine();
        let _output = engine
            .execute_operator(&sample_input("transfer"))
            .unwrap();
        assert_eq!(engine.record_count(), 1);
        let record = &engine.all_records()[0];
        assert!(record.success);
        assert!(record.operation.contains("transfer"));
    }

    #[test]
    fn engine_execute_wlir_stores_record() {
        let mut engine = make_engine();
        let _result = engine
            .execute_wlir("test-mod", "main", vec![])
            .unwrap();
        assert_eq!(engine.record_count(), 1);
        let record = &engine.all_records()[0];
        assert!(record.operation.contains("wlir:test-mod::main"));
    }

    #[test]
    fn engine_bounded_fifo() {
        let config = SalConfig {
            max_tracked_records: 3,
            ..SalConfig::default()
        };
        let mut engine =
            SalEngine::with_config(Box::new(CpuSubstrate::new()), &config);

        for i in 0..5 {
            let _ = engine.execute_operator(&sample_input(&format!("op-{}", i)));
        }

        assert_eq!(engine.record_count(), 3);
        // Oldest should be evicted
        let ops: Vec<_> = engine
            .all_records()
            .iter()
            .map(|r| r.operation.clone())
            .collect();
        assert!(ops[0].contains("op-2"));
        assert!(ops[1].contains("op-3"));
        assert!(ops[2].contains("op-4"));
    }

    #[test]
    fn engine_summary() {
        let mut engine = make_engine();
        let _ = engine.execute_operator(&sample_input("op1"));
        let _ = engine.execute_operator(&sample_input("op2"));
        let summary = engine.summary();
        assert_eq!(summary.total_executions, 2);
        assert_eq!(summary.successful_executions, 2);
        assert_eq!(summary.failed_executions, 0);
    }

    #[test]
    fn engine_provenance_tracking() {
        let mut engine = make_engine();
        let _ = engine.execute_operator(&sample_input("op"));
        let summary = engine.summary();
        assert_eq!(summary.total_provenance_records, 1);
    }

    #[test]
    fn engine_provenance_disabled() {
        let config = SalConfig {
            record_provenance: false,
            ..SalConfig::default()
        };
        let mut engine =
            SalEngine::with_config(Box::new(CpuSubstrate::new()), &config);
        let _ = engine.execute_operator(&sample_input("op"));
        let summary = engine.summary();
        assert_eq!(summary.total_provenance_records, 0);
    }

    #[test]
    fn engine_substrate_access() {
        let engine = make_engine();
        assert_eq!(engine.substrate().name(), "cpu-substrate");
    }

    #[test]
    fn engine_with_gpu_substrate() {
        let mut engine = SalEngine::new(Box::new(GpuSubstrate::new()));
        let result = engine
            .execute_wlir("mod", "main", vec![])
            .unwrap();
        assert!(result.output_values[0].contains("gpu:"));
        assert_eq!(engine.record_count(), 1);
    }

    #[test]
    fn engine_summary_display() {
        let mut engine = make_engine();
        let _ = engine.execute_operator(&sample_input("op"));
        let summary = engine.summary();
        let display = summary.to_string();
        assert!(display.contains("executions=1"));
        assert!(display.contains("success=1"));
    }
}
