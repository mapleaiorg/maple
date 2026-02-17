/// Errors from the Resonance Monitor.
#[derive(Debug, thiserror::Error)]
pub enum MonitorError {
    #[error("detector error: {0}")]
    DetectorFailed(String),
    #[error("emergency stop triggered: {0}")]
    EmergencyStop(String),
    #[error("cooldown active for {category}: {remaining_secs}s remaining")]
    CooldownActive {
        category: String,
        remaining_secs: u64,
    },
    #[error("metrics unavailable: {0}")]
    MetricsUnavailable(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let e = MonitorError::EmergencyStop("resonance below minimum".into());
        assert!(format!("{}", e).contains("resonance"));
    }

    #[test]
    fn cooldown_display() {
        let e = MonitorError::CooldownActive {
            category: "Computational".into(),
            remaining_secs: 30,
        };
        let msg = format!("{}", e);
        assert!(msg.contains("Computational"));
        assert!(msg.contains("30"));
    }
}
