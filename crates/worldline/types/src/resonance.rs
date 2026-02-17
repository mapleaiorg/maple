use serde::{Deserialize, Serialize};

/// The four semantic stages of the Resonance Ladder.
/// These are THE fundamental type classification in MWL.
///
/// Per Whitepaper §2.3: "This sequence is directional and non-collapsible."
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResonanceType {
    /// Interpretation, beliefs, uncertainty, evidence. Non-executable.
    Meaning,
    /// Goals, plans, constraints. Non-executable.
    Intent,
    /// Explicit obligation. The ONLY executable type (after approval).
    Commitment,
    /// Observable outcome. Emitted by reality, not agents.
    Consequence,
}

impl ResonanceType {
    /// Check if a transition from self to other is valid (non-collapsible).
    /// Meaning → Intent → Commitment → Consequence (forward only)
    pub fn can_transition_to(&self, other: &ResonanceType) -> bool {
        self.ordinal() < other.ordinal()
    }

    /// Is this type executable? Only Commitment is, and only after approval.
    pub fn is_executable(&self) -> bool {
        matches!(self, ResonanceType::Commitment)
    }

    /// Ordinal position in the Resonance Ladder.
    pub fn ordinal(&self) -> u8 {
        match self {
            ResonanceType::Meaning => 0,
            ResonanceType::Intent => 1,
            ResonanceType::Commitment => 2,
            ResonanceType::Consequence => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_transitions_valid() {
        assert!(ResonanceType::Meaning.can_transition_to(&ResonanceType::Intent));
        assert!(ResonanceType::Intent.can_transition_to(&ResonanceType::Commitment));
        assert!(ResonanceType::Commitment.can_transition_to(&ResonanceType::Consequence));
        assert!(ResonanceType::Meaning.can_transition_to(&ResonanceType::Consequence));
    }

    #[test]
    fn backward_transitions_invalid() {
        assert!(!ResonanceType::Intent.can_transition_to(&ResonanceType::Meaning));
        assert!(!ResonanceType::Commitment.can_transition_to(&ResonanceType::Meaning));
        assert!(!ResonanceType::Commitment.can_transition_to(&ResonanceType::Intent));
        assert!(!ResonanceType::Consequence.can_transition_to(&ResonanceType::Meaning));
        assert!(!ResonanceType::Consequence.can_transition_to(&ResonanceType::Commitment));
    }

    #[test]
    fn self_transition_invalid() {
        assert!(!ResonanceType::Meaning.can_transition_to(&ResonanceType::Meaning));
        assert!(!ResonanceType::Commitment.can_transition_to(&ResonanceType::Commitment));
    }

    #[test]
    fn only_commitment_is_executable() {
        assert!(!ResonanceType::Meaning.is_executable());
        assert!(!ResonanceType::Intent.is_executable());
        assert!(ResonanceType::Commitment.is_executable());
        assert!(!ResonanceType::Consequence.is_executable());
    }

    #[test]
    fn ordinal_ordering() {
        assert!(ResonanceType::Meaning.ordinal() < ResonanceType::Intent.ordinal());
        assert!(ResonanceType::Intent.ordinal() < ResonanceType::Commitment.ordinal());
        assert!(ResonanceType::Commitment.ordinal() < ResonanceType::Consequence.ordinal());
    }

    #[test]
    fn serialization_roundtrip() {
        for rt in [
            ResonanceType::Meaning,
            ResonanceType::Intent,
            ResonanceType::Commitment,
            ResonanceType::Consequence,
        ] {
            let json = serde_json::to_string(&rt).unwrap();
            let restored: ResonanceType = serde_json::from_str(&json).unwrap();
            assert_eq!(rt, restored);
        }
    }
}
