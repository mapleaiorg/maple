/// Errors from the Evolution Engine.
#[derive(Debug, thiserror::Error)]
pub enum EvolutionError {
    #[error("synthesis failed: {0}")]
    SynthesisFailed(String),
    #[error("no viable hypothesis found")]
    NoViableHypothesis,
    #[error("safety check failed: {0}")]
    SafetyViolation(String),
    #[error("LLM provider error: {0}")]
    LlmError(String),
    #[error("hardware context unavailable: {0}")]
    HardwareUnavailable(String),
    #[error("timeout: synthesis exceeded {0}ms")]
    Timeout(u64),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let e = EvolutionError::NoViableHypothesis;
        assert!(format!("{}", e).contains("no viable"));
    }

    #[test]
    fn timeout_display() {
        let e = EvolutionError::Timeout(5000);
        assert!(format!("{}", e).contains("5000"));
    }
}
