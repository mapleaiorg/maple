//! Hardware generation engine with bounded history.
//!
//! `HardwareEngine` orchestrates the full EPU generation pipeline:
//! design → HDL → simulate → synthesize → bitstream, with governance
//! checks and bounded FIFO record keeping.

use std::collections::VecDeque;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::epu::{EpuDesigner, EpuSpec, SimulatedEpuDesigner};
use crate::error::HardwareResult;
use crate::fpga::{Bitstream, BitstreamGenerator, SimulatedBitstreamGenerator};
use crate::governance::{
    enforce_governance, HardwareGovernance, SimulatedHardwareGovernance,
};
use crate::hdl::{GeneratedHdl, HdlGenerator, SimulatedHdlGenerator};
use crate::simulation::{HardwareSimulator, SimulatedHardwareSimulator, TestVector};
use crate::types::{EpuId, HardwareConfig, HardwareSummary, SimulationResult};

// ── Generation Record ───────────────────────────────────────────────

/// Record of a hardware generation run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerationRecord {
    pub epu_id: EpuId,
    pub epu_name: String,
    pub hdl_generated: bool,
    pub simulation_passed: Option<bool>,
    pub bitstream_produced: bool,
    pub governance_approved: bool,
    pub error_message: Option<String>,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
}

// ── Hardware Engine ─────────────────────────────────────────────────

/// Engine orchestrating the full hardware generation pipeline.
pub struct HardwareEngine {
    config: HardwareConfig,
    designer: Box<dyn EpuDesigner>,
    hdl_gen: Box<dyn HdlGenerator>,
    simulator: Box<dyn HardwareSimulator>,
    bitstream_gen: Box<dyn BitstreamGenerator>,
    governance: Box<dyn HardwareGovernance>,
    records: VecDeque<GenerationRecord>,
    max_records: usize,
}

impl HardwareEngine {
    /// Create with default configuration and simulated components.
    pub fn new() -> Self {
        let config = HardwareConfig::default();
        let max = config.max_tracked_records;
        Self {
            config,
            designer: Box::new(SimulatedEpuDesigner::new()),
            hdl_gen: Box::new(SimulatedHdlGenerator::new()),
            simulator: Box::new(SimulatedHardwareSimulator::new()),
            bitstream_gen: Box::new(SimulatedBitstreamGenerator::new()),
            governance: Box::new(SimulatedHardwareGovernance::new()),
            records: VecDeque::new(),
            max_records: max,
        }
    }

    /// Create with specific configuration.
    pub fn with_config(config: HardwareConfig) -> Self {
        let max = config.max_tracked_records;
        Self {
            config,
            designer: Box::new(SimulatedEpuDesigner::new()),
            hdl_gen: Box::new(SimulatedHdlGenerator::new()),
            simulator: Box::new(SimulatedHardwareSimulator::new()),
            bitstream_gen: Box::new(SimulatedBitstreamGenerator::new()),
            governance: Box::new(SimulatedHardwareGovernance::new()),
            records: VecDeque::new(),
            max_records: max,
        }
    }

    /// Set governance implementation.
    pub fn with_governance(mut self, gov: Box<dyn HardwareGovernance>) -> Self {
        self.governance = gov;
        self
    }

    /// Full pipeline: design → governance → HDL → simulate → bitstream.
    pub fn generate(
        &mut self,
        name: &str,
        target_latency_ns: u64,
    ) -> HardwareResult<(EpuSpec, GeneratedHdl, Option<SimulationResult>, Option<Bitstream>)> {
        // Step 1: Design EPU
        let spec = self.designer.design(name, target_latency_ns)?;

        // Step 2: Governance check
        if self.config.enforce_governance {
            let _gov_record = enforce_governance(self.governance.as_ref(), &spec)?;
        }

        // Step 3: Generate HDL
        let hdl = self.hdl_gen.generate(&spec, &self.config.hdl_format)?;

        // Step 4: Simulate (optional)
        let sim_result = if self.config.simulate_before_synthesis {
            let vectors = default_test_vectors();
            let result = self.simulator.simulate(&spec, &hdl, &vectors)?;
            Some(result)
        } else {
            None
        };

        // Step 5: Generate bitstream (if not simulation-only)
        let bitstream = if self.config.target != crate::types::FpgaTarget::SimulationOnly {
            let bs = self.bitstream_gen.generate(
                &spec,
                &hdl,
                &self.config.target,
                sim_result.as_ref(),
            )?;
            Some(bs)
        } else {
            None
        };

        // Record success
        self.push_record(GenerationRecord {
            epu_id: spec.id.clone(),
            epu_name: name.to_string(),
            hdl_generated: true,
            simulation_passed: sim_result.as_ref().map(|s| s.passed),
            bitstream_produced: bitstream.is_some(),
            governance_approved: true,
            error_message: None,
            recorded_at: Utc::now(),
        });

        Ok((spec, hdl, sim_result, bitstream))
    }

    /// Push a record with bounded FIFO eviction.
    fn push_record(&mut self, record: GenerationRecord) {
        if self.records.len() >= self.max_records {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }

    /// All generation records.
    pub fn all_records(&self) -> &VecDeque<GenerationRecord> {
        &self.records
    }

    /// Number of tracked records.
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// Summary statistics.
    pub fn summary(&self) -> HardwareSummary {
        let total = self.records.len();
        let successful = self.records.iter().filter(|r| r.hdl_generated).count();
        let failed = total - successful;
        let simulations = self
            .records
            .iter()
            .filter(|r| r.simulation_passed.is_some())
            .count();
        let bitstreams = self
            .records
            .iter()
            .filter(|r| r.bitstream_produced)
            .count();

        HardwareSummary {
            total_generations: total,
            successful_generations: successful,
            failed_generations: failed,
            total_simulations_run: simulations,
            total_bitstreams_produced: bitstreams,
        }
    }
}

impl Default for HardwareEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Default test vectors for simulation.
fn default_test_vectors() -> Vec<TestVector> {
    vec![
        TestVector {
            name: "reset_test".into(),
            input_values: vec![0],
            expected_outputs: vec![0],
            cycles: 100,
        },
        TestVector {
            name: "commitment_gate_test".into(),
            input_values: vec![1, 0],
            expected_outputs: vec![1],
            cycles: 200,
        },
        TestVector {
            name: "provenance_record_test".into(),
            input_values: vec![0xDE, 0xAD],
            expected_outputs: vec![0xBE, 0xEF],
            cycles: 300,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FpgaTarget;

    #[test]
    fn engine_generate_simulation_only() {
        let mut engine = HardwareEngine::new();
        let (spec, hdl, sim, bs) = engine.generate("test-epu", 100).unwrap();
        assert_eq!(spec.name, "test-epu");
        assert!(!hdl.source.is_empty());
        assert!(sim.is_some());
        assert!(sim.unwrap().passed);
        assert!(bs.is_none()); // SimulationOnly target
        assert_eq!(engine.record_count(), 1);
    }

    #[test]
    fn engine_generate_with_bitstream() {
        let config = HardwareConfig {
            target: FpgaTarget::Xilinx7Series,
            enforce_governance: false,
            ..HardwareConfig::default()
        };
        let mut engine = HardwareEngine::with_config(config);
        let (_, _, _, bs) = engine.generate("fpga-epu", 50).unwrap();
        assert!(bs.is_some());
        assert_eq!(bs.unwrap().target, FpgaTarget::Xilinx7Series);
    }

    #[test]
    fn engine_governance_required() {
        let config = HardwareConfig {
            enforce_governance: true,
            ..HardwareConfig::default()
        };
        let engine = HardwareEngine::with_config(config)
            .with_governance(Box::new(SimulatedHardwareGovernance::strict()));
        let mut engine = engine;
        let result = engine.generate("strict-epu", 100);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("pending human review"));
    }

    #[test]
    fn engine_bounded_fifo() {
        let config = HardwareConfig {
            max_tracked_records: 3,
            enforce_governance: false,
            ..HardwareConfig::default()
        };
        let mut engine = HardwareEngine::with_config(config);
        for i in 0..5 {
            let _ = engine.generate(&format!("epu-{}", i), 100);
        }
        assert_eq!(engine.record_count(), 3);
        let names: Vec<_> = engine
            .all_records()
            .iter()
            .map(|r| r.epu_name.clone())
            .collect();
        assert_eq!(names, vec!["epu-2", "epu-3", "epu-4"]);
    }

    #[test]
    fn engine_summary() {
        let mut engine = HardwareEngine::new();
        let _ = engine.generate("epu-1", 100);
        let _ = engine.generate("epu-2", 50);
        let summary = engine.summary();
        assert_eq!(summary.total_generations, 2);
        assert_eq!(summary.successful_generations, 2);
        assert_eq!(summary.total_simulations_run, 2);
    }

    #[test]
    fn engine_skip_simulation() {
        let config = HardwareConfig {
            simulate_before_synthesis: false,
            enforce_governance: false,
            ..HardwareConfig::default()
        };
        let mut engine = HardwareEngine::with_config(config);
        let (_, _, sim, _) = engine.generate("no-sim", 100).unwrap();
        assert!(sim.is_none());
    }

    #[test]
    fn engine_default() {
        let engine = HardwareEngine::default();
        assert_eq!(engine.record_count(), 0);
    }
}
