//! Canonical WorldLine profiles defining behavioral archetypes.
//!
//! Every worldline has exactly one profile that determines its default
//! attention budget, coupling limits, and behavioral constraints.

use serde::{Deserialize, Serialize};

/// The five canonical WorldLine profiles.
///
/// Each profile defines a behavioral archetype with distinct defaults
/// for attention budget, coupling capacity, and operational constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorldlineProfile {
    /// Human principal — maximum agency, minimum constraint.
    Human,
    /// AI agent — bounded autonomy, commitment required for all actions.
    Agent,
    /// Financial instrument — strict accountability, DvP atomicity.
    Financial,
    /// World/environment — coordination role, no direct action capability.
    World,
    /// Coordinator — orchestrates multi-worldline workflows.
    Coordination,
}

impl WorldlineProfile {
    /// Default attention budget for this profile.
    pub fn default_attention_budget(&self) -> f64 {
        match self {
            Self::Human => 50.0,
            Self::Agent => 100.0,
            Self::Financial => 200.0,
            Self::World => 500.0,
            Self::Coordination => 300.0,
        }
    }

    /// Default maximum number of simultaneous couplings.
    pub fn default_max_couplings(&self) -> usize {
        match self {
            Self::Human => 10,
            Self::Agent => 50,
            Self::Financial => 100,
            Self::World => 1000,
            Self::Coordination => 200,
        }
    }
}

impl std::fmt::Display for WorldlineProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Human => write!(f, "Human"),
            Self::Agent => write!(f, "Agent"),
            Self::Financial => write!(f, "Financial"),
            Self::World => write!(f, "World"),
            Self::Coordination => write!(f, "Coordination"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_display() {
        assert_eq!(WorldlineProfile::Human.to_string(), "Human");
        assert_eq!(WorldlineProfile::Agent.to_string(), "Agent");
        assert_eq!(WorldlineProfile::Financial.to_string(), "Financial");
        assert_eq!(WorldlineProfile::World.to_string(), "World");
        assert_eq!(WorldlineProfile::Coordination.to_string(), "Coordination");
    }

    #[test]
    fn profile_defaults() {
        assert_eq!(WorldlineProfile::Human.default_attention_budget(), 50.0);
        assert_eq!(WorldlineProfile::Agent.default_max_couplings(), 50);
        assert_eq!(WorldlineProfile::Financial.default_attention_budget(), 200.0);
        assert_eq!(WorldlineProfile::World.default_max_couplings(), 1000);
    }

    #[test]
    fn profile_serde_roundtrip() {
        for profile in [
            WorldlineProfile::Human,
            WorldlineProfile::Agent,
            WorldlineProfile::Financial,
            WorldlineProfile::World,
            WorldlineProfile::Coordination,
        ] {
            let json = serde_json::to_string(&profile).unwrap();
            let back: WorldlineProfile = serde_json::from_str(&json).unwrap();
            assert_eq!(profile, back);
        }
    }
}
