//! Hardware simulation via Verilator (simulated).
//!
//! Validates generated HDL before synthesis by running test vectors
//! and verifying timing constraints.

use serde::{Deserialize, Serialize};

use crate::epu::EpuSpec;
use crate::error::HardwareResult;
use crate::hdl::GeneratedHdl;
use crate::types::SimulationResult;

// ── Test Vector ─────────────────────────────────────────────────────

/// A test vector for hardware simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestVector {
    pub name: String,
    pub input_values: Vec<u64>,
    pub expected_outputs: Vec<u64>,
    pub cycles: u64,
}

// ── Hardware Simulator Trait ────────────────────────────────────────

/// Trait for simulating hardware designs.
pub trait HardwareSimulator: Send + Sync {
    /// Simulate the generated HDL against the EPU spec.
    fn simulate(
        &self,
        spec: &EpuSpec,
        hdl: &GeneratedHdl,
        test_vectors: &[TestVector],
    ) -> HardwareResult<SimulationResult>;

    /// Name of this simulator.
    fn name(&self) -> &str;
}

/// Simulated hardware simulator (Verilator stand-in).
pub struct SimulatedHardwareSimulator;

impl SimulatedHardwareSimulator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimulatedHardwareSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl HardwareSimulator for SimulatedHardwareSimulator {
    fn simulate(
        &self,
        spec: &EpuSpec,
        _hdl: &GeneratedHdl,
        test_vectors: &[TestVector],
    ) -> HardwareResult<SimulationResult> {
        let total_cycles: u64 = test_vectors.iter().map(|tv| tv.cycles).sum();
        let test_count = test_vectors.len() as u32;

        // Simulated: all tests pass, timing met if target latency is reasonable
        let max_freq = if spec.target_latency_ns > 0 {
            1000.0 / spec.target_latency_ns as f64 * 1000.0 // MHz
        } else {
            100.0
        };

        let primary_clock = spec
            .clock_domains
            .iter()
            .find(|c| c.is_primary)
            .map(|c| c.frequency_mhz)
            .unwrap_or(100.0);

        let timing_met = max_freq >= primary_clock;

        Ok(SimulationResult {
            passed: true,
            cycles_simulated: total_cycles.max(1000),
            test_cases_passed: test_count,
            test_cases_failed: 0,
            timing_met,
            max_frequency_mhz: max_freq.min(500.0),
            warnings: vec![],
        })
    }

    fn name(&self) -> &str {
        "simulated-verilator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epu::{EpuDesigner, SimulatedEpuDesigner};
    use crate::hdl::{HdlGenerator, SimulatedHdlGenerator};
    use crate::types::HdlFormat;

    fn sample_spec() -> EpuSpec {
        SimulatedEpuDesigner::new().design("sim-test", 100).unwrap()
    }

    fn sample_hdl(spec: &EpuSpec) -> GeneratedHdl {
        SimulatedHdlGenerator::new()
            .generate(spec, &HdlFormat::SystemVerilog)
            .unwrap()
    }

    fn sample_vectors() -> Vec<TestVector> {
        vec![
            TestVector {
                name: "reset".into(),
                input_values: vec![0],
                expected_outputs: vec![0],
                cycles: 100,
            },
            TestVector {
                name: "basic_op".into(),
                input_values: vec![1, 2],
                expected_outputs: vec![3],
                cycles: 500,
            },
        ]
    }

    #[test]
    fn simulation_passes() {
        let sim = SimulatedHardwareSimulator::new();
        let spec = sample_spec();
        let hdl = sample_hdl(&spec);
        let result = sim.simulate(&spec, &hdl, &sample_vectors()).unwrap();
        assert!(result.passed);
        assert_eq!(result.test_cases_passed, 2);
        assert_eq!(result.test_cases_failed, 0);
    }

    #[test]
    fn simulation_cycles_counted() {
        let sim = SimulatedHardwareSimulator::new();
        let spec = sample_spec();
        let hdl = sample_hdl(&spec);
        let result = sim.simulate(&spec, &hdl, &sample_vectors()).unwrap();
        assert!(result.cycles_simulated >= 600); // 100 + 500
    }

    #[test]
    fn simulation_timing_met() {
        let sim = SimulatedHardwareSimulator::new();
        let spec = sample_spec();
        let hdl = sample_hdl(&spec);
        let result = sim.simulate(&spec, &hdl, &sample_vectors()).unwrap();
        assert!(result.timing_met);
        assert!(result.max_frequency_mhz > 0.0);
    }

    #[test]
    fn simulation_empty_vectors() {
        let sim = SimulatedHardwareSimulator::new();
        let spec = sample_spec();
        let hdl = sample_hdl(&spec);
        let result = sim.simulate(&spec, &hdl, &[]).unwrap();
        assert!(result.passed);
        assert_eq!(result.test_cases_passed, 0);
    }

    #[test]
    fn simulator_name() {
        let sim = SimulatedHardwareSimulator::new();
        assert_eq!(sim.name(), "simulated-verilator");
    }
}
