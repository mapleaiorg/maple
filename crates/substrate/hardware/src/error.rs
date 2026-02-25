//! Hardware error types.
//!
//! Covers all failure modes: EPU specification, HDL generation,
//! simulation, synthesis, bitstream generation, and governance.

use thiserror::Error;

/// Errors that can occur during hardware generation.
#[derive(Debug, Error)]
pub enum HardwareError {
    /// EPU specification is invalid.
    #[error("EPU specification error: {0}")]
    EpuSpecError(String),

    /// HDL generation failed.
    #[error("HDL generation failed: {0}")]
    HdlGenerationFailed(String),

    /// Hardware simulation failed.
    #[error("Simulation failed: {0}")]
    SimulationFailed(String),

    /// FPGA synthesis failed.
    #[error("Synthesis failed: {0}")]
    SynthesisFailed(String),

    /// Bitstream generation failed.
    #[error("Bitstream generation failed: {0}")]
    BitstreamFailed(String),

    /// Governance check failed (Tier 4-5 required).
    #[error("Governance required: {0}")]
    GovernanceRequired(String),

    /// Resource limit exceeded.
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

/// Result type for hardware operations.
pub type HardwareResult<T> = Result<T, HardwareError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let e = HardwareError::EpuSpecError("missing commitment gate".into());
        assert!(e.to_string().contains("missing commitment gate"));

        let e = HardwareError::GovernanceRequired("Tier 5 review needed".into());
        assert!(e.to_string().contains("Tier 5"));
    }

    #[test]
    fn all_error_variants_display() {
        let errors: Vec<HardwareError> = vec![
            HardwareError::EpuSpecError("a".into()),
            HardwareError::HdlGenerationFailed("b".into()),
            HardwareError::SimulationFailed("c".into()),
            HardwareError::SynthesisFailed("d".into()),
            HardwareError::BitstreamFailed("e".into()),
            HardwareError::GovernanceRequired("f".into()),
            HardwareError::ResourceExhausted("g".into()),
            HardwareError::ConfigurationError("h".into()),
        ];
        for error in &errors {
            assert!(!error.to_string().is_empty());
        }
        assert_eq!(errors.len(), 8);
    }

    #[test]
    fn error_is_std_error() {
        let e: Box<dyn std::error::Error> =
            Box::new(HardwareError::SimulationFailed("timing".into()));
        assert!(e.to_string().contains("timing"));
    }

    #[test]
    fn result_type_works() {
        let ok: HardwareResult<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: HardwareResult<i32> = Err(HardwareError::ConfigurationError("bad".into()));
        assert!(err.is_err());
    }

    #[test]
    fn error_debug_format() {
        let e = HardwareError::BitstreamFailed("place-and-route".into());
        let debug = format!("{:?}", e);
        assert!(debug.contains("BitstreamFailed"));
    }
}
