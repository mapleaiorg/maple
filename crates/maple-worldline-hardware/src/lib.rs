//! # maple-worldline-hardware
//!
//! Hardware Description Operators for EVOS Processing Unit (EPU)
//! generation targeting FPGA/ASIC. Enables WorldLine to describe
//! and produce its own hardware substrate.
//!
//! ## Pipeline
//!
//! ```text
//! EPU Design → Governance (Tier 4-5) → HDL Generation → Simulation → Bitstream
//! ```
//!
//! ## Governance
//!
//! Hardware generation is always Tier 4 (SubstrateChange) or Tier 5
//! (ArchitecturalChange) — human review is required.
//!
//! ## Target
//!
//! 10x commitment gate latency improvement on FPGA vs CPU substrate.

#![deny(unsafe_code)]

pub mod engine;
pub mod epu;
pub mod error;
pub mod fpga;
pub mod governance;
pub mod hdl;
pub mod simulation;
pub mod types;

// ── Re-exports ──────────────────────────────────────────────────────

pub use engine::{GenerationRecord, HardwareEngine};
pub use epu::{EpuComponent, EpuComponentKind, EpuDesigner, EpuSpec, SimulatedEpuDesigner};
pub use error::{HardwareError, HardwareResult};
pub use fpga::{Bitstream, BitstreamGenerator, SimulatedBitstreamGenerator};
pub use governance::{
    GovernanceDecision, GovernanceRecord, GovernanceRequest, HardwareGovernance,
    HardwareGovernanceTier, SimulatedHardwareGovernance,
};
pub use hdl::{GeneratedHdl, HdlGenerator, SimulatedHdlGenerator};
pub use simulation::{HardwareSimulator, SimulatedHardwareSimulator, TestVector};
pub use types::{
    BitstreamId, ClockDomain, EpuId, FpgaTarget, HardwareConfig, HardwareSummary, HdlFormat,
    ResourceUtilization, SimulationResult,
};

#[cfg(test)]
mod tests {
    use super::*;

    // ── E2E: Full pipeline ──

    #[test]
    fn e2e_full_pipeline_simulation_only() {
        let mut engine = HardwareEngine::new();
        let (spec, hdl, sim, bs) = engine.generate("e2e-epu", 100).unwrap();

        // EPU has all required components
        assert!(spec.has_required_components());

        // HDL was generated
        assert!(!hdl.source.is_empty());
        assert!(hdl.line_count > 0);

        // Simulation passed
        let sim = sim.unwrap();
        assert!(sim.passed);
        assert!(sim.timing_met);

        // No bitstream for simulation-only target
        assert!(bs.is_none());
    }

    #[test]
    fn e2e_full_pipeline_with_bitstream() {
        let config = HardwareConfig {
            target: FpgaTarget::XilinxUltraScalePlus,
            enforce_governance: false,
            ..HardwareConfig::default()
        };
        let mut engine = HardwareEngine::with_config(config);
        let (spec, hdl, sim, bs) = engine.generate("fpga-epu", 50).unwrap();

        assert!(spec.has_required_components());
        assert!(!hdl.source.is_empty());
        assert!(sim.unwrap().passed);
        let bs = bs.unwrap();
        assert_eq!(bs.target, FpgaTarget::XilinxUltraScalePlus);
        assert!(bs.size_bytes > 0);
    }

    // ── EPU core components ──

    #[test]
    fn epu_has_commitment_gate() {
        let designer = SimulatedEpuDesigner::new();
        let spec = designer.design("gate-test", 100).unwrap();
        let gate = spec
            .components
            .iter()
            .find(|c| c.kind == EpuComponentKind::CommitmentGate);
        assert!(gate.is_some());
        assert!(gate.unwrap().pipeline_stages > 0);
    }

    #[test]
    fn epu_has_provenance_unit() {
        let designer = SimulatedEpuDesigner::new();
        let spec = designer.design("prov-test", 100).unwrap();
        let prov = spec
            .components
            .iter()
            .find(|c| c.kind == EpuComponentKind::ProvenanceUnit);
        assert!(prov.is_some());
    }

    #[test]
    fn epu_has_event_interface() {
        let designer = SimulatedEpuDesigner::new();
        let spec = designer.design("event-test", 100).unwrap();
        let event = spec
            .components
            .iter()
            .find(|c| c.kind == EpuComponentKind::EventInterface);
        assert!(event.is_some());
    }

    // ── HDL generation all formats ──

    #[test]
    fn e2e_hdl_all_formats() {
        let designer = SimulatedEpuDesigner::new();
        let spec = designer.design("multi-hdl", 100).unwrap();
        let gen = SimulatedHdlGenerator::new();

        let formats = vec![
            HdlFormat::Verilog,
            HdlFormat::SystemVerilog,
            HdlFormat::Chisel,
            HdlFormat::Vhdl,
        ];
        for fmt in formats {
            let hdl = gen.generate(&spec, &fmt).unwrap();
            assert_eq!(hdl.format, fmt);
            assert!(!hdl.source.is_empty());
            assert!(!hdl.content_hash.is_empty());
        }
    }

    // ── Governance ──

    #[test]
    fn governance_blocks_without_approval() {
        let config = HardwareConfig {
            enforce_governance: true,
            ..HardwareConfig::default()
        };
        let mut engine = HardwareEngine::with_config(config)
            .with_governance(Box::new(SimulatedHardwareGovernance::strict()));
        let result = engine.generate("blocked-epu", 100);
        assert!(result.is_err());
    }

    #[test]
    fn governance_approves_in_test_mode() {
        let mut engine = HardwareEngine::new(); // auto-approve
        let result = engine.generate("approved-epu", 100);
        assert!(result.is_ok());
    }

    // ── Engine multi-generate ──

    #[test]
    fn e2e_engine_multi_generate() {
        let mut engine = HardwareEngine::new();
        for i in 0..3 {
            let _ = engine.generate(&format!("epu-{}", i), 100).unwrap();
        }
        let summary = engine.summary();
        assert_eq!(summary.total_generations, 3);
        assert_eq!(summary.successful_generations, 3);
        assert_eq!(summary.total_simulations_run, 3);
    }

    // ── Public types ──

    #[test]
    fn public_types_accessible() {
        let _id = EpuId::from_name("test");
        let _bs_id = BitstreamId::new();
        let _target = FpgaTarget::SimulationOnly;
        let _fmt = HdlFormat::Verilog;
        let _tier = HardwareGovernanceTier::Tier4SubstrateChange;
        let _decision = GovernanceDecision::Approved;
        let _config = HardwareConfig::default();
        let _summary = HardwareSummary::default();
        let _resources = ResourceUtilization::default();
    }
}
