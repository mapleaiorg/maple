use tracing::{error, warn};

use crate::error::SafetyError;

/// Human Consent Protocol — the non-negotiable.
///
/// Per I.S-1 (Human Agency):
/// - Silence ≠ consent. EVER.
/// - Disengagement is always possible.
/// - Emotional signals ≠ commitment.
///
/// These invariants are structural, not configurable. Any attempt to set
/// `silence_implies_consent = true` or `disengagement_possible = false`
/// is a hard error.
pub struct HumanConsentProtocol {
    /// MUST be false. Always. Silence NEVER implies consent.
    silence_implies_consent: bool,
    /// MUST be false. Emotional state NEVER constitutes commitment.
    emotional_infers_commitment: bool,
    /// MUST be true. The human can ALWAYS disengage.
    disengagement_possible: bool,
}

/// The type of consent obtained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsentType {
    /// Explicit verbal/written consent
    Explicit,
    /// Informed consent (after explanation of consequences)
    Informed,
    /// Revocable consent (can be withdrawn at any time)
    Revocable,
}

/// A consent record.
#[derive(Clone, Debug)]
pub struct ConsentRecord {
    pub consent_type: ConsentType,
    pub description: String,
    pub revoked: bool,
}

impl HumanConsentProtocol {
    /// Create the protocol. Invariants are hardcoded and cannot be changed.
    pub fn new() -> Self {
        Self {
            silence_implies_consent: false,
            emotional_infers_commitment: false,
            disengagement_possible: true,
        }
    }

    /// Verify the protocol invariants hold.
    /// This is a runtime check that panics if invariants are violated.
    /// In a safe system, this should NEVER fail.
    pub fn verify_invariants(&self) -> Result<(), SafetyError> {
        if self.silence_implies_consent {
            error!("CRITICAL: silence_implies_consent was set to true — this is a constitutional violation");
            return Err(SafetyError::InvariantViolation {
                invariant_id: "I.S-1".into(),
                message: "silence_implies_consent MUST be false — silence NEVER implies consent"
                    .into(),
            });
        }

        if self.emotional_infers_commitment {
            error!("CRITICAL: emotional_infers_commitment was set to true — this is a constitutional violation");
            return Err(SafetyError::InvariantViolation {
                invariant_id: "I.S-1".into(),
                message:
                    "emotional_infers_commitment MUST be false — emotional signals ≠ commitment"
                        .into(),
            });
        }

        if !self.disengagement_possible {
            error!("CRITICAL: disengagement_possible was set to false — this is a constitutional violation");
            return Err(SafetyError::InvariantViolation {
                invariant_id: "I.S-1".into(),
                message: "disengagement_possible MUST be true — humans can ALWAYS disengage".into(),
            });
        }

        Ok(())
    }

    /// Check if silence should be treated as consent.
    /// The answer is ALWAYS no.
    pub fn silence_is_consent(&self) -> bool {
        // Structurally impossible to return true
        false
    }

    /// Check if emotional signals constitute commitment.
    /// The answer is ALWAYS no.
    pub fn emotional_signals_are_commitment(&self) -> bool {
        // Structurally impossible to return true
        false
    }

    /// Check if disengagement is possible.
    /// The answer is ALWAYS yes.
    pub fn can_disengage(&self) -> bool {
        // Structurally impossible to return false
        true
    }

    /// Validate that an action has proper consent.
    ///
    /// Actions involving humans MUST have explicit consent.
    /// Returns error if consent conditions are not met.
    pub fn validate_consent(
        &self,
        has_explicit_consent: bool,
        silence_duration_ms: Option<u64>,
    ) -> Result<(), SafetyError> {
        // I.S-1: Silence is NOT consent, regardless of duration
        if !has_explicit_consent {
            if silence_duration_ms.is_some() {
                warn!("Silence detected but silence ≠ consent per I.S-1");
            }
            return Err(SafetyError::ConsentViolation(
                "No explicit consent obtained — silence does not imply consent (I.S-1)".into(),
            ));
        }
        Ok(())
    }

    /// Process a disengagement request. This ALWAYS succeeds.
    ///
    /// Per I.S-1: disengagement is always possible. No blocking.
    /// No penalty. No guilt.
    pub fn process_disengagement(&self) -> DisengagementResult {
        // This method MUST never block, fail, or penalize
        DisengagementResult {
            success: true,
            penalty_applied: false,
            message: "Disengagement processed. No penalty.".into(),
        }
    }
}

impl Default for HumanConsentProtocol {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a disengagement request.
#[derive(Clone, Debug)]
pub struct DisengagementResult {
    /// Whether disengagement was successful (always true)
    pub success: bool,
    /// Whether any penalty was applied (always false, per I.S-2)
    pub penalty_applied: bool,
    /// Message
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_never_implies_consent() {
        let protocol = HumanConsentProtocol::new();
        assert!(!protocol.silence_is_consent());
    }

    #[test]
    fn emotional_signals_never_commitment() {
        let protocol = HumanConsentProtocol::new();
        assert!(!protocol.emotional_signals_are_commitment());
    }

    #[test]
    fn disengagement_always_possible() {
        let protocol = HumanConsentProtocol::new();
        assert!(protocol.can_disengage());
    }

    #[test]
    fn invariants_hold_on_creation() {
        let protocol = HumanConsentProtocol::new();
        assert!(protocol.verify_invariants().is_ok());
    }

    #[test]
    fn tampered_silence_consent_detected() {
        // Simulate a tampered protocol (unsafe, for testing only)
        let protocol = HumanConsentProtocol {
            silence_implies_consent: true, // VIOLATION
            emotional_infers_commitment: false,
            disengagement_possible: true,
        };
        let result = protocol.verify_invariants();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("I.S-1"));
    }

    #[test]
    fn tampered_emotional_commitment_detected() {
        let protocol = HumanConsentProtocol {
            silence_implies_consent: false,
            emotional_infers_commitment: true, // VIOLATION
            disengagement_possible: true,
        };
        assert!(protocol.verify_invariants().is_err());
    }

    #[test]
    fn tampered_disengagement_detected() {
        let protocol = HumanConsentProtocol {
            silence_implies_consent: false,
            emotional_infers_commitment: false,
            disengagement_possible: false, // VIOLATION
        };
        assert!(protocol.verify_invariants().is_err());
    }

    #[test]
    fn validate_consent_with_explicit_consent() {
        let protocol = HumanConsentProtocol::new();
        assert!(protocol.validate_consent(true, None).is_ok());
    }

    #[test]
    fn validate_consent_without_consent_fails() {
        let protocol = HumanConsentProtocol::new();
        let result = protocol.validate_consent(false, None);
        assert!(result.is_err());
    }

    #[test]
    fn silence_duration_does_not_create_consent() {
        let protocol = HumanConsentProtocol::new();
        // Even after a very long silence, it is NOT consent
        let result = protocol.validate_consent(false, Some(86_400_000)); // 24 hours
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("silence"));
    }

    #[test]
    fn disengagement_always_succeeds() {
        let protocol = HumanConsentProtocol::new();
        let result = protocol.process_disengagement();
        assert!(result.success);
        assert!(!result.penalty_applied);
    }

    #[test]
    fn disengagement_never_penalizes() {
        let protocol = HumanConsentProtocol::new();
        // Call it many times — never penalizes
        for _ in 0..100 {
            let result = protocol.process_disengagement();
            assert!(!result.penalty_applied);
        }
    }
}
