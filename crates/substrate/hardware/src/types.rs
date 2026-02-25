//! Core types for hardware description and generation.
//!
//! Defines identifiers, FPGA targets, clock domains, resource utilization,
//! HDL formats, and configuration for hardware generation.

use serde::{Deserialize, Serialize};

// ── Identifiers ─────────────────────────────────────────────────────

/// Unique identifier for an EPU design.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EpuId(pub String);

impl EpuId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_name(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl Default for EpuId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EpuId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "epu:{}", self.0)
    }
}

/// Unique identifier for a bitstream.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BitstreamId(pub String);

impl BitstreamId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for BitstreamId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BitstreamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bitstream:{}", self.0)
    }
}

// ── FPGA Target ─────────────────────────────────────────────────────

/// Target FPGA device family.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FpgaTarget {
    /// Xilinx 7-series (Artix, Kintex, Virtex).
    Xilinx7Series,
    /// Xilinx UltraScale+.
    XilinxUltraScalePlus,
    /// Intel/Altera Stratix.
    IntelStratix,
    /// Lattice ECP5.
    LatticeEcp5,
    /// Simulation-only (no physical target).
    SimulationOnly,
}

impl std::fmt::Display for FpgaTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Xilinx7Series => write!(f, "xilinx-7series"),
            Self::XilinxUltraScalePlus => write!(f, "xilinx-ultrascale+"),
            Self::IntelStratix => write!(f, "intel-stratix"),
            Self::LatticeEcp5 => write!(f, "lattice-ecp5"),
            Self::SimulationOnly => write!(f, "simulation-only"),
        }
    }
}

// ── HDL Format ──────────────────────────────────────────────────────

/// Hardware description language format.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HdlFormat {
    Verilog,
    SystemVerilog,
    Chisel,
    Vhdl,
}

impl std::fmt::Display for HdlFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Verilog => write!(f, "verilog"),
            Self::SystemVerilog => write!(f, "systemverilog"),
            Self::Chisel => write!(f, "chisel"),
            Self::Vhdl => write!(f, "vhdl"),
        }
    }
}

// ── Clock Domain ────────────────────────────────────────────────────

/// Clock domain specification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClockDomain {
    pub name: String,
    pub frequency_mhz: f64,
    pub is_primary: bool,
}

// ── Resource Utilization ────────────────────────────────────────────

/// FPGA resource utilization estimates.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResourceUtilization {
    /// Lookup tables used.
    pub luts: u64,
    /// Flip-flops used.
    pub flip_flops: u64,
    /// Block RAM (in kilobits).
    pub bram_kb: u64,
    /// DSP slices used.
    pub dsp_slices: u64,
    /// Estimated power consumption (milliwatts).
    pub power_mw: u64,
}

impl std::fmt::Display for ResourceUtilization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Resources(LUTs={}, FFs={}, BRAM={}kb, DSPs={}, power={}mW)",
            self.luts, self.flip_flops, self.bram_kb, self.dsp_slices, self.power_mw,
        )
    }
}

// ── Simulation Result ───────────────────────────────────────────────

/// Result of a hardware simulation run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulationResult {
    pub passed: bool,
    pub cycles_simulated: u64,
    pub test_cases_passed: u32,
    pub test_cases_failed: u32,
    pub timing_met: bool,
    pub max_frequency_mhz: f64,
    pub warnings: Vec<String>,
}

// ── Configuration ───────────────────────────────────────────────────

/// Configuration for hardware generation.
#[derive(Clone, Debug)]
pub struct HardwareConfig {
    /// Target FPGA device.
    pub target: FpgaTarget,
    /// Preferred HDL format.
    pub hdl_format: HdlFormat,
    /// Target clock frequency (MHz).
    pub target_clock_mhz: f64,
    /// Whether to run simulation before synthesis.
    pub simulate_before_synthesis: bool,
    /// Whether to enforce governance checks.
    pub enforce_governance: bool,
    /// Maximum tracked generation records.
    pub max_tracked_records: usize,
}

impl Default for HardwareConfig {
    fn default() -> Self {
        Self {
            target: FpgaTarget::SimulationOnly,
            hdl_format: HdlFormat::SystemVerilog,
            target_clock_mhz: 100.0,
            simulate_before_synthesis: true,
            enforce_governance: true,
            max_tracked_records: 256,
        }
    }
}

// ── Summary ─────────────────────────────────────────────────────────

/// Summary of hardware generation activity.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HardwareSummary {
    pub total_generations: usize,
    pub successful_generations: usize,
    pub failed_generations: usize,
    pub total_simulations_run: usize,
    pub total_bitstreams_produced: usize,
}

impl std::fmt::Display for HardwareSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Hardware(generations={}, success={}, failed={}, simulations={}, bitstreams={})",
            self.total_generations,
            self.successful_generations,
            self.failed_generations,
            self.total_simulations_run,
            self.total_bitstreams_produced,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epu_id_display() {
        let id = EpuId::from_name("commitment-gate-v1");
        assert_eq!(id.to_string(), "epu:commitment-gate-v1");
    }

    #[test]
    fn epu_id_unique() {
        let a = EpuId::new();
        let b = EpuId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn bitstream_id_display() {
        let id = BitstreamId::new();
        assert!(id.to_string().starts_with("bitstream:"));
    }

    #[test]
    fn fpga_target_display() {
        assert_eq!(FpgaTarget::Xilinx7Series.to_string(), "xilinx-7series");
        assert_eq!(FpgaTarget::SimulationOnly.to_string(), "simulation-only");
        assert_eq!(FpgaTarget::LatticeEcp5.to_string(), "lattice-ecp5");
    }

    #[test]
    fn hdl_format_display() {
        assert_eq!(HdlFormat::Verilog.to_string(), "verilog");
        assert_eq!(HdlFormat::SystemVerilog.to_string(), "systemverilog");
        assert_eq!(HdlFormat::Chisel.to_string(), "chisel");
        assert_eq!(HdlFormat::Vhdl.to_string(), "vhdl");
    }

    #[test]
    fn resource_utilization_display() {
        let r = ResourceUtilization {
            luts: 5000,
            flip_flops: 3000,
            bram_kb: 128,
            dsp_slices: 10,
            power_mw: 500,
        };
        let display = r.to_string();
        assert!(display.contains("LUTs=5000"));
        assert!(display.contains("power=500mW"));
    }

    #[test]
    fn config_defaults() {
        let cfg = HardwareConfig::default();
        assert_eq!(cfg.target, FpgaTarget::SimulationOnly);
        assert_eq!(cfg.hdl_format, HdlFormat::SystemVerilog);
        assert_eq!(cfg.target_clock_mhz, 100.0);
        assert!(cfg.simulate_before_synthesis);
        assert!(cfg.enforce_governance);
    }

    #[test]
    fn summary_display() {
        let s = HardwareSummary {
            total_generations: 10,
            successful_generations: 8,
            failed_generations: 2,
            total_simulations_run: 15,
            total_bitstreams_produced: 5,
        };
        let display = s.to_string();
        assert!(display.contains("generations=10"));
        assert!(display.contains("bitstreams=5"));
    }

    #[test]
    fn clock_domain_creation() {
        let clk = ClockDomain {
            name: "sys_clk".into(),
            frequency_mhz: 100.0,
            is_primary: true,
        };
        assert_eq!(clk.frequency_mhz, 100.0);
        assert!(clk.is_primary);
    }

    #[test]
    fn simulation_result_creation() {
        let sim = SimulationResult {
            passed: true,
            cycles_simulated: 100_000,
            test_cases_passed: 50,
            test_cases_failed: 0,
            timing_met: true,
            max_frequency_mhz: 125.0,
            warnings: vec![],
        };
        assert!(sim.passed);
        assert!(sim.timing_met);
    }
}
