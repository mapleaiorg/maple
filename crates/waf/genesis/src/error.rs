/// Errors from the Genesis Protocol.
#[derive(Debug, thiserror::Error)]
pub enum GenesisError {
    #[error("substrate attestation failed: {0}")]
    SubstrateAttestationFailed(String),
    #[error("axiomatic anchoring failed: {0}")]
    AxiomaticAnchoringFailed(String),
    #[error("observer activation failed: {0}")]
    ObserverActivationFailed(String),
    #[error("reflexive awakening failed: {0}")]
    ReflexiveAwakeningFailed(String),
    #[error("insufficient resonance: {current:.3} < {minimum:.3}")]
    InsufficientResonance { current: f64, minimum: f64 },
    #[error("invariant violation during genesis: {0}")]
    InvariantViolation(String),
    #[error("genesis already completed")]
    AlreadyCompleted,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let e = GenesisError::InsufficientResonance {
            current: 0.3,
            minimum: 0.6,
        };
        assert!(format!("{}", e).contains("0.300"));
    }

    #[test]
    fn already_completed() {
        let e = GenesisError::AlreadyCompleted;
        assert!(format!("{}", e).contains("already"));
    }
}
