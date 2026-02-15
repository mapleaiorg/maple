//! EVOS substrate — unified view of all WorldLine subsystems.
//!
//! The `EvosSubstrate` represents the complete digital organism,
//! with a manifest of all subsystems and current bootstrap phase.

use maple_worldline_bootstrap::BootstrapPhase;
use serde::{Deserialize, Serialize};

use crate::types::{EvosId, SubsystemId};

// ── Subsystem Entry ─────────────────────────────────────────────────

/// Entry in the substrate manifest for one subsystem.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubsystemEntry {
    /// Subsystem identifier.
    pub id: SubsystemId,
    /// Crate version.
    pub version: String,
    /// Short description.
    pub description: String,
}

impl std::fmt::Display for SubsystemEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} v{}: {}", self.id, self.version, self.description)
    }
}

// ── Substrate Manifest ──────────────────────────────────────────────

/// Manifest listing all 14 WorldLine subsystems.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubstrateManifest {
    pub entries: Vec<SubsystemEntry>,
}

impl SubstrateManifest {
    /// Create the canonical manifest for the current WorldLine build.
    pub fn canonical() -> Self {
        let version = "0.1.2".to_string();
        let entries = vec![
            SubsystemEntry {
                id: SubsystemId::Observation,
                version: version.clone(),
                description: "Self-observation, anomaly detection, baselining".into(),
            },
            SubsystemEntry {
                id: SubsystemId::Meaning,
                version: version.clone(),
                description: "Hypothesis generation, evidence evaluation, convergence".into(),
            },
            SubsystemEntry {
                id: SubsystemId::Intent,
                version: version.clone(),
                description: "Intent stabilization, impact assessment, proposals".into(),
            },
            SubsystemEntry {
                id: SubsystemId::Commitment,
                version: version.clone(),
                description: "Observation periods, commitment lifecycle, declarations".into(),
            },
            SubsystemEntry {
                id: SubsystemId::Consequence,
                version: version.clone(),
                description: "Consequence execution, rollback, observation feedback".into(),
            },
            SubsystemEntry {
                id: SubsystemId::SelfModGate,
                version: version.clone(),
                description: "6-tier self-modification governance, rate limiting".into(),
            },
            SubsystemEntry {
                id: SubsystemId::Codegen,
                version: version.clone(),
                description: "Code generation, sandbox compilation, artifact building".into(),
            },
            SubsystemEntry {
                id: SubsystemId::Deployment,
                version: version.clone(),
                description: "Deployment strategies, rollback, health monitoring".into(),
            },
            SubsystemEntry {
                id: SubsystemId::Ir,
                version: version.clone(),
                description: "WLIR intermediate representation, 31 instructions".into(),
            },
            SubsystemEntry {
                id: SubsystemId::Compiler,
                version: version.clone(),
                description: "Adaptive compilation, 11 optimization passes".into(),
            },
            SubsystemEntry {
                id: SubsystemId::LangGen,
                version: version.clone(),
                description: "Domain-specific language generation pipeline".into(),
            },
            SubsystemEntry {
                id: SubsystemId::Sal,
                version: version.clone(),
                description: "Substrate abstraction, CPU/GPU/FPGA/Hybrid execution".into(),
            },
            SubsystemEntry {
                id: SubsystemId::Hardware,
                version: version.clone(),
                description: "EPU design, HDL generation, FPGA synthesis".into(),
            },
            SubsystemEntry {
                id: SubsystemId::Bootstrap,
                version,
                description: "6-phase bootstrap protocol, external to self-hosted".into(),
            },
        ];
        Self { entries }
    }

    /// Number of subsystems.
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Find a subsystem entry by ID.
    pub fn find(&self, id: &SubsystemId) -> Option<&SubsystemEntry> {
        self.entries.iter().find(|e| e.id == *id)
    }
}

impl std::fmt::Display for SubstrateManifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SubstrateManifest({} subsystems)", self.count())
    }
}

// ── EVOS Substrate ──────────────────────────────────────────────────

/// The complete EVOS digital organism substrate.
///
/// Provides a unified view of:
/// - All 14 subsystem identities and versions
/// - Current bootstrap phase
/// - Substrate identity
pub struct EvosSubstrate {
    /// Unique identifier for this substrate instance.
    pub id: EvosId,
    /// Manifest of all subsystems.
    pub manifest: SubstrateManifest,
    /// Current bootstrap phase.
    pub bootstrap_phase: BootstrapPhase,
}

impl EvosSubstrate {
    /// Create a new substrate at Phase 0 (external).
    pub fn new() -> Self {
        Self {
            id: EvosId::new(),
            manifest: SubstrateManifest::canonical(),
            bootstrap_phase: BootstrapPhase::Phase0ExternalSubstrate,
        }
    }

    /// Create with a specific bootstrap phase.
    pub fn at_phase(phase: BootstrapPhase) -> Self {
        Self {
            id: EvosId::new(),
            manifest: SubstrateManifest::canonical(),
            bootstrap_phase: phase,
        }
    }

    /// Whether the substrate has reached self-hosting.
    pub fn is_self_hosting(&self) -> bool {
        self.bootstrap_phase == BootstrapPhase::Phase5SubstrateSelfDescription
    }

    /// Number of subsystems.
    pub fn subsystem_count(&self) -> usize {
        self.manifest.count()
    }
}

impl Default for EvosSubstrate {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EvosSubstrate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EvosSubstrate(id={}, subsystems={}, bootstrap={})",
            self.id,
            self.subsystem_count(),
            self.bootstrap_phase,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_manifest_has_14() {
        let manifest = SubstrateManifest::canonical();
        assert_eq!(manifest.count(), 14);
    }

    #[test]
    fn manifest_find_subsystem() {
        let manifest = SubstrateManifest::canonical();
        let obs = manifest.find(&SubsystemId::Observation).unwrap();
        assert_eq!(obs.version, "0.1.2");
        assert!(obs.description.contains("anomaly"));
    }

    #[test]
    fn substrate_new_at_phase0() {
        let substrate = EvosSubstrate::new();
        assert_eq!(
            substrate.bootstrap_phase,
            BootstrapPhase::Phase0ExternalSubstrate
        );
        assert!(!substrate.is_self_hosting());
        assert_eq!(substrate.subsystem_count(), 14);
    }

    #[test]
    fn substrate_at_phase5() {
        let substrate =
            EvosSubstrate::at_phase(BootstrapPhase::Phase5SubstrateSelfDescription);
        assert!(substrate.is_self_hosting());
    }

    #[test]
    fn substrate_display() {
        let substrate = EvosSubstrate::new();
        let display = substrate.to_string();
        assert!(display.contains("evos:"));
        assert!(display.contains("subsystems=14"));
        assert!(display.contains("Phase0"));
    }

    #[test]
    fn manifest_display() {
        let manifest = SubstrateManifest::canonical();
        assert!(manifest.to_string().contains("14 subsystems"));
    }

    #[test]
    fn subsystem_entry_display() {
        let entry = SubsystemEntry {
            id: SubsystemId::Compiler,
            version: "0.1.2".into(),
            description: "Adaptive compilation".into(),
        };
        let display = entry.to_string();
        assert!(display.contains("compiler"));
        assert!(display.contains("0.1.2"));
    }

    #[test]
    fn manifest_all_subsystems_present() {
        let manifest = SubstrateManifest::canonical();
        for subsystem in SubsystemId::all() {
            assert!(
                manifest.find(subsystem).is_some(),
                "missing: {}",
                subsystem,
            );
        }
    }
}
