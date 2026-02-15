//! Error types for the bootstrap protocol.

use thiserror::Error;

/// Errors that can occur during bootstrap operations.
#[derive(Debug, Error)]
pub enum BootstrapError {
    /// Phase transition failed (invalid direction, skip, etc.).
    #[error("phase transition failed: {0}")]
    PhaseTransitionFailed(String),

    /// Readiness check did not pass for the target phase.
    #[error("readiness check failed: {0}")]
    ReadinessCheckFailed(String),

    /// Substrate fingerprint mismatch or drift detected.
    #[error("fingerprint mismatch: {0}")]
    FingerprintMismatch(String),

    /// Gap detected in the provenance chain.
    #[error("provenance gap: {0}")]
    ProvenanceGap(String),

    /// Governance approval required before transition.
    #[error("governance required: {0}")]
    GovernanceRequired(String),

    /// Required substrate is unavailable.
    #[error("substrate unavailable: {0}")]
    SubstrateUnavailable(String),

    /// Invalid configuration.
    #[error("configuration error: {0}")]
    ConfigurationError(String),
}

/// Result type for bootstrap operations.
pub type BootstrapResult<T> = Result<T, BootstrapError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_phase_transition() {
        let e = BootstrapError::PhaseTransitionFailed("cannot skip phase 2".into());
        assert!(e.to_string().contains("cannot skip phase 2"));
    }

    #[test]
    fn error_display_readiness() {
        let e = BootstrapError::ReadinessCheckFailed("stability below threshold".into());
        assert!(e.to_string().contains("stability below threshold"));
    }

    #[test]
    fn error_display_fingerprint() {
        let e = BootstrapError::FingerprintMismatch("rustc version changed".into());
        assert!(e.to_string().contains("rustc version changed"));
    }

    #[test]
    fn error_display_provenance_gap() {
        let e = BootstrapError::ProvenanceGap("missing link between phase 1 and 3".into());
        assert!(e.to_string().contains("missing link"));
    }

    #[test]
    fn error_display_governance() {
        let e = BootstrapError::GovernanceRequired("tier 4 approval needed".into());
        assert!(e.to_string().contains("tier 4 approval needed"));
    }
}
