/// Errors from the Autopoietic Kernel.
#[derive(Debug, thiserror::Error)]
pub enum KernelError {
    #[error("kernel not initialized — run genesis first")]
    NotInitialized,
    #[error("evolution step failed: {0}")]
    EvolutionFailed(String),
    #[error("emergency stop active")]
    EmergencyStop,
    #[error("resonance below minimum: {current:.3} < {minimum:.3}")]
    ResonanceBelowMinimum { current: f64, minimum: f64 },
    #[error("max evolution steps reached: {0}")]
    MaxStepsReached(u64),
    #[error("genesis error: {0}")]
    Genesis(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let e = KernelError::ResonanceBelowMinimum {
            current: 0.3,
            minimum: 0.6,
        };
        assert!(format!("{}", e).contains("0.300"));
    }
}
