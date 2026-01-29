//! Resonator profile types

use serde::{Deserialize, Serialize};

/// Resonator profile determines constraints and behaviors
///
/// Different profiles enable different use cases:
/// - Human: Agency-first, consent required, safety priority
/// - World/Finalverse: Experience-focused, reversible consequences
/// - Coordination/Mapleverse: Explicit commitments, strong accountability
/// - IBank: AI-only, strict auditability, risk-bounded
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResonatorProfile {
    /// Human Resonator - maximum agency protection
    Human,

    /// World/Finalverse Resonator - experiential environments
    World,

    /// Coordination/Mapleverse Resonator - pure AI agents
    Coordination,

    /// IBank Resonator - autonomous financial operations
    IBank,
}

impl ResonatorProfile {
    /// Does this profile require human agency protection?
    pub fn requires_agency_protection(&self) -> bool {
        matches!(self, ResonatorProfile::Human)
    }

    /// Does this profile require strict audit trails?
    pub fn requires_audit_trail(&self) -> bool {
        matches!(self, ResonatorProfile::IBank)
    }

    /// Does this profile prefer reversible consequences?
    pub fn prefers_reversibility(&self) -> bool {
        matches!(self, ResonatorProfile::World | ResonatorProfile::IBank)
    }

    /// Can this profile form couplings with other profiles?
    pub fn can_couple_with(&self, other: &ResonatorProfile) -> bool {
        match (self, other) {
            // Humans can couple with World and Coordination, but not IBank
            (ResonatorProfile::Human, ResonatorProfile::IBank) => false,
            (ResonatorProfile::IBank, ResonatorProfile::Human) => false,

            // IBank only couples with IBank
            (ResonatorProfile::IBank, ResonatorProfile::IBank) => true,
            (ResonatorProfile::IBank, _) => false,
            (_, ResonatorProfile::IBank) => false,

            // All other combinations allowed
            _ => true,
        }
    }
}

impl Default for ResonatorProfile {
    fn default() -> Self {
        ResonatorProfile::Coordination
    }
}
