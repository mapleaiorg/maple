//! Resonance Type System
use serde::{Deserialize, Serialize};
use std::fmt;

/// Resonance type marker - Meaning ≠ Intent ≠ Commitment ≠ Consequence
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResonanceType {
    Meaning,
    Intent,
    Commitment,
    Consequence,
}

impl ResonanceType {
    #[inline]
    pub const fn escalation_level(&self) -> u8 {
        match self {
            ResonanceType::Meaning => 0,
            ResonanceType::Intent => 1,
            ResonanceType::Commitment => 2,
            ResonanceType::Consequence => 3,
        }
    }

    #[inline]
    pub const fn can_transition_to(&self, target: &ResonanceType) -> bool {
        matches!(
            (self, target),
            (ResonanceType::Meaning, ResonanceType::Intent)
                | (ResonanceType::Intent, ResonanceType::Commitment)
                | (ResonanceType::Commitment, ResonanceType::Consequence)
        )
    }

    #[inline]
    pub const fn is_potentially_executable(&self) -> bool {
        matches!(self, ResonanceType::Commitment)
    }

    #[inline]
    pub const fn is_below_commitment_boundary(&self) -> bool {
        self.escalation_level() < crate::COMMITMENT_BOUNDARY_LEVEL
    }

    #[inline]
    pub const fn is_at_or_above_commitment_boundary(&self) -> bool {
        self.escalation_level() >= crate::COMMITMENT_BOUNDARY_LEVEL
    }

    pub const fn name(&self) -> &'static str {
        match self {
            ResonanceType::Meaning => "Meaning",
            ResonanceType::Intent => "Intent",
            ResonanceType::Commitment => "Commitment",
            ResonanceType::Consequence => "Consequence",
        }
    }
}

impl fmt::Display for ResonanceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Trait for resonance artifacts
pub trait ResonanceArtifact {
    fn resonance_type(&self) -> ResonanceType;
    fn artifact_id(&self) -> &str;
    fn is_executable(&self) -> bool {
        self.resonance_type().is_potentially_executable()
    }
}
