//! FPGA bitstream generator — synthesis and place-and-route.
//!
//! Takes simulated/validated HDL and produces a bitstream for
//! the target FPGA device.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::epu::EpuSpec;
use crate::error::HardwareResult;
use crate::hdl::GeneratedHdl;
use crate::types::{BitstreamId, FpgaTarget, ResourceUtilization, SimulationResult};

// ── Bitstream ───────────────────────────────────────────────────────

/// A generated FPGA bitstream.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bitstream {
    pub id: BitstreamId,
    pub target: FpgaTarget,
    pub epu_name: String,
    pub size_bytes: u64,
    pub resource_utilization: ResourceUtilization,
    pub max_frequency_mhz: f64,
    pub content_hash: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

impl std::fmt::Display for Bitstream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Bitstream({}, target={}, {}KB, freq={:.1}MHz)",
            self.epu_name,
            self.target,
            self.size_bytes / 1024,
            self.max_frequency_mhz,
        )
    }
}

// ── Bitstream Generator Trait ───────────────────────────────────────

/// Trait for generating FPGA bitstreams from HDL.
pub trait BitstreamGenerator: Send + Sync {
    /// Generate a bitstream for the target FPGA.
    fn generate(
        &self,
        spec: &EpuSpec,
        hdl: &GeneratedHdl,
        target: &FpgaTarget,
        sim_result: Option<&SimulationResult>,
    ) -> HardwareResult<Bitstream>;

    /// Name of this generator.
    fn name(&self) -> &str;
}

/// Simulated bitstream generator.
pub struct SimulatedBitstreamGenerator;

impl SimulatedBitstreamGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimulatedBitstreamGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl BitstreamGenerator for SimulatedBitstreamGenerator {
    fn generate(
        &self,
        spec: &EpuSpec,
        hdl: &GeneratedHdl,
        target: &FpgaTarget,
        sim_result: Option<&SimulationResult>,
    ) -> HardwareResult<Bitstream> {
        // Simulated bitstream size based on HDL source size
        let size_bytes = (hdl.source.len() as u64) * 100; // ~100x expansion

        let max_freq = sim_result.map(|s| s.max_frequency_mhz).unwrap_or(100.0);

        Ok(Bitstream {
            id: BitstreamId::new(),
            target: target.clone(),
            epu_name: spec.name.clone(),
            size_bytes,
            resource_utilization: spec.total_resources.clone(),
            max_frequency_mhz: max_freq,
            content_hash: hdl.content_hash.clone(),
            generated_at: Utc::now(),
        })
    }

    fn name(&self) -> &str {
        "simulated-bitstream-generator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epu::{EpuDesigner, SimulatedEpuDesigner};
    use crate::hdl::{HdlGenerator, SimulatedHdlGenerator};
    use crate::types::HdlFormat;

    fn sample_spec() -> EpuSpec {
        SimulatedEpuDesigner::new()
            .design("fpga-test", 100)
            .unwrap()
    }

    fn sample_hdl(spec: &EpuSpec) -> GeneratedHdl {
        SimulatedHdlGenerator::new()
            .generate(spec, &HdlFormat::Verilog)
            .unwrap()
    }

    #[test]
    fn generate_bitstream() {
        let gen = SimulatedBitstreamGenerator::new();
        let spec = sample_spec();
        let hdl = sample_hdl(&spec);
        let bs = gen
            .generate(&spec, &hdl, &FpgaTarget::SimulationOnly, None)
            .unwrap();
        assert_eq!(bs.epu_name, "fpga-test");
        assert!(bs.size_bytes > 0);
    }

    #[test]
    fn bitstream_with_simulation_result() {
        let gen = SimulatedBitstreamGenerator::new();
        let spec = sample_spec();
        let hdl = sample_hdl(&spec);
        let sim = SimulationResult {
            passed: true,
            cycles_simulated: 1000,
            test_cases_passed: 5,
            test_cases_failed: 0,
            timing_met: true,
            max_frequency_mhz: 150.0,
            warnings: vec![],
        };
        let bs = gen
            .generate(&spec, &hdl, &FpgaTarget::Xilinx7Series, Some(&sim))
            .unwrap();
        assert_eq!(bs.max_frequency_mhz, 150.0);
        assert_eq!(bs.target, FpgaTarget::Xilinx7Series);
    }

    #[test]
    fn bitstream_display() {
        let gen = SimulatedBitstreamGenerator::new();
        let spec = sample_spec();
        let hdl = sample_hdl(&spec);
        let bs = gen
            .generate(&spec, &hdl, &FpgaTarget::SimulationOnly, None)
            .unwrap();
        let display = bs.to_string();
        assert!(display.contains("fpga-test"));
        assert!(display.contains("simulation-only"));
    }

    #[test]
    fn bitstream_resource_utilization() {
        let gen = SimulatedBitstreamGenerator::new();
        let spec = sample_spec();
        let hdl = sample_hdl(&spec);
        let bs = gen
            .generate(&spec, &hdl, &FpgaTarget::SimulationOnly, None)
            .unwrap();
        assert_eq!(bs.resource_utilization.luts, spec.total_resources.luts);
    }

    #[test]
    fn bitstream_all_fpga_targets() {
        let gen = SimulatedBitstreamGenerator::new();
        let spec = sample_spec();
        let hdl = sample_hdl(&spec);
        let targets = vec![
            FpgaTarget::Xilinx7Series,
            FpgaTarget::XilinxUltraScalePlus,
            FpgaTarget::IntelStratix,
            FpgaTarget::LatticeEcp5,
            FpgaTarget::SimulationOnly,
        ];
        for target in targets {
            let bs = gen.generate(&spec, &hdl, &target, None).unwrap();
            assert_eq!(bs.target, target);
        }
    }

    #[test]
    fn generator_name() {
        let gen = SimulatedBitstreamGenerator::new();
        assert_eq!(gen.name(), "simulated-bitstream-generator");
    }
}
