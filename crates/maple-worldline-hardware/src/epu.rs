//! EVOS Processing Unit (EPU) specification.
//!
//! Defines the hardware-accelerated EPU with three core components:
//! - **Commitment Gate Unit**: Hardware commitment boundary enforcement
//! - **Provenance Unit**: Hardware provenance recording
//! - **Event Interface**: Hardware event emission and routing
//!
//! The EPU is the hardware counterpart of the WorldLine substrate cycle.

use serde::{Deserialize, Serialize};

use crate::error::HardwareResult;
use crate::types::{ClockDomain, EpuId, ResourceUtilization};

// ── EPU Component ───────────────────────────────────────────────────

/// A hardware component within the EPU.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpuComponent {
    pub name: String,
    pub kind: EpuComponentKind,
    pub port_count: u32,
    pub pipeline_stages: u32,
    pub estimated_resources: ResourceUtilization,
}

/// Kind of EPU component.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpuComponentKind {
    /// Hardware commitment gate — enforces boundaries in hardware.
    CommitmentGate,
    /// Hardware provenance unit — records provenance at wire speed.
    ProvenanceUnit,
    /// Hardware event interface — routes events between components.
    EventInterface,
    /// Operator execution unit — runs WLIR operators.
    OperatorUnit,
    /// Memory controller — manages memory tiers.
    MemoryController,
}

impl std::fmt::Display for EpuComponentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommitmentGate => write!(f, "commitment-gate"),
            Self::ProvenanceUnit => write!(f, "provenance-unit"),
            Self::EventInterface => write!(f, "event-interface"),
            Self::OperatorUnit => write!(f, "operator-unit"),
            Self::MemoryController => write!(f, "memory-controller"),
        }
    }
}

// ── EPU Specification ───────────────────────────────────────────────

/// Full specification of an EVOS Processing Unit.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpuSpec {
    pub id: EpuId,
    pub name: String,
    pub version: String,
    pub components: Vec<EpuComponent>,
    pub clock_domains: Vec<ClockDomain>,
    pub total_resources: ResourceUtilization,
    pub target_latency_ns: u64,
    pub description: String,
}

impl EpuSpec {
    /// Whether this EPU has all three required core components.
    pub fn has_required_components(&self) -> bool {
        let has_gate = self
            .components
            .iter()
            .any(|c| c.kind == EpuComponentKind::CommitmentGate);
        let has_prov = self
            .components
            .iter()
            .any(|c| c.kind == EpuComponentKind::ProvenanceUnit);
        let has_event = self
            .components
            .iter()
            .any(|c| c.kind == EpuComponentKind::EventInterface);
        has_gate && has_prov && has_event
    }

    /// Total pipeline stages across all components.
    pub fn total_pipeline_stages(&self) -> u32 {
        self.components.iter().map(|c| c.pipeline_stages).sum()
    }
}

// ── EPU Designer Trait ──────────────────────────────────────────────

/// Trait for designing EPU specifications.
pub trait EpuDesigner: Send + Sync {
    /// Design an EPU specification for the given target latency.
    fn design(&self, name: &str, target_latency_ns: u64) -> HardwareResult<EpuSpec>;

    /// Name of this designer.
    fn name(&self) -> &str;
}

/// Simulated EPU designer for deterministic testing.
pub struct SimulatedEpuDesigner;

impl SimulatedEpuDesigner {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimulatedEpuDesigner {
    fn default() -> Self {
        Self::new()
    }
}

impl EpuDesigner for SimulatedEpuDesigner {
    fn design(&self, name: &str, target_latency_ns: u64) -> HardwareResult<EpuSpec> {
        let commitment_gate = EpuComponent {
            name: "commitment_gate_0".into(),
            kind: EpuComponentKind::CommitmentGate,
            port_count: 4,
            pipeline_stages: 3,
            estimated_resources: ResourceUtilization {
                luts: 2000,
                flip_flops: 1500,
                bram_kb: 32,
                dsp_slices: 0,
                power_mw: 50,
            },
        };

        let provenance_unit = EpuComponent {
            name: "provenance_unit_0".into(),
            kind: EpuComponentKind::ProvenanceUnit,
            port_count: 2,
            pipeline_stages: 2,
            estimated_resources: ResourceUtilization {
                luts: 1500,
                flip_flops: 1000,
                bram_kb: 64,
                dsp_slices: 0,
                power_mw: 30,
            },
        };

        let event_interface = EpuComponent {
            name: "event_interface_0".into(),
            kind: EpuComponentKind::EventInterface,
            port_count: 8,
            pipeline_stages: 1,
            estimated_resources: ResourceUtilization {
                luts: 1000,
                flip_flops: 500,
                bram_kb: 16,
                dsp_slices: 0,
                power_mw: 20,
            },
        };

        let operator_unit = EpuComponent {
            name: "operator_unit_0".into(),
            kind: EpuComponentKind::OperatorUnit,
            port_count: 4,
            pipeline_stages: 5,
            estimated_resources: ResourceUtilization {
                luts: 3000,
                flip_flops: 2000,
                bram_kb: 128,
                dsp_slices: 8,
                power_mw: 100,
            },
        };

        let components = vec![
            commitment_gate,
            provenance_unit,
            event_interface,
            operator_unit,
        ];

        let total_resources = ResourceUtilization {
            luts: components.iter().map(|c| c.estimated_resources.luts).sum(),
            flip_flops: components.iter().map(|c| c.estimated_resources.flip_flops).sum(),
            bram_kb: components.iter().map(|c| c.estimated_resources.bram_kb).sum(),
            dsp_slices: components.iter().map(|c| c.estimated_resources.dsp_slices).sum(),
            power_mw: components.iter().map(|c| c.estimated_resources.power_mw).sum(),
        };

        Ok(EpuSpec {
            id: EpuId::from_name(name),
            name: name.to_string(),
            version: "1.0.0".into(),
            components,
            clock_domains: vec![ClockDomain {
                name: "sys_clk".into(),
                frequency_mhz: 100.0,
                is_primary: true,
            }],
            total_resources,
            target_latency_ns,
            description: format!("EPU '{}' with target latency {}ns", name, target_latency_ns),
        })
    }

    fn name(&self) -> &str {
        "simulated-epu-designer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_kind_display() {
        assert_eq!(EpuComponentKind::CommitmentGate.to_string(), "commitment-gate");
        assert_eq!(EpuComponentKind::ProvenanceUnit.to_string(), "provenance-unit");
        assert_eq!(EpuComponentKind::EventInterface.to_string(), "event-interface");
        assert_eq!(EpuComponentKind::OperatorUnit.to_string(), "operator-unit");
        assert_eq!(EpuComponentKind::MemoryController.to_string(), "memory-controller");
    }

    #[test]
    fn simulated_designer_produces_valid_epu() {
        let designer = SimulatedEpuDesigner::new();
        let spec = designer.design("test-epu", 100).unwrap();
        assert_eq!(spec.name, "test-epu");
        assert!(spec.has_required_components());
        assert_eq!(spec.components.len(), 4);
    }

    #[test]
    fn epu_has_required_components() {
        let designer = SimulatedEpuDesigner::new();
        let spec = designer.design("full", 50).unwrap();
        assert!(spec.has_required_components());
    }

    #[test]
    fn epu_total_pipeline_stages() {
        let designer = SimulatedEpuDesigner::new();
        let spec = designer.design("pipeline", 100).unwrap();
        assert_eq!(spec.total_pipeline_stages(), 3 + 2 + 1 + 5);
    }

    #[test]
    fn epu_resource_totals() {
        let designer = SimulatedEpuDesigner::new();
        let spec = designer.design("resources", 100).unwrap();
        assert_eq!(spec.total_resources.luts, 2000 + 1500 + 1000 + 3000);
        assert_eq!(spec.total_resources.dsp_slices, 8);
    }

    #[test]
    fn epu_clock_domain() {
        let designer = SimulatedEpuDesigner::new();
        let spec = designer.design("clk", 100).unwrap();
        assert_eq!(spec.clock_domains.len(), 1);
        assert!(spec.clock_domains[0].is_primary);
        assert_eq!(spec.clock_domains[0].frequency_mhz, 100.0);
    }

    #[test]
    fn epu_target_latency() {
        let designer = SimulatedEpuDesigner::new();
        let spec = designer.design("fast", 50).unwrap();
        assert_eq!(spec.target_latency_ns, 50);
    }

    #[test]
    fn designer_name() {
        let designer = SimulatedEpuDesigner::new();
        assert_eq!(designer.name(), "simulated-epu-designer");
    }
}
