use serde::{Deserialize, Serialize};

/// Categories of dissonance detected in the system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DissonanceCategory {
    /// Semantic: API friction, error patterns, workarounds.
    Semantic,
    /// Computational: CPU waste, memory pressure, latency spikes.
    Computational,
    /// Policy: boundary proximity, denial trends, drift.
    PolicyDrift,
}

impl std::fmt::Display for DissonanceCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Semantic => write!(f, "Semantic"),
            Self::Computational => write!(f, "Computational"),
            Self::PolicyDrift => write!(f, "Policy Drift"),
        }
    }
}

/// A detected dissonance event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DissonanceEvent {
    /// Category of dissonance.
    pub category: DissonanceCategory,
    /// Severity score in [0.0, 1.0] where 1.0 is maximum dissonance.
    pub severity: f64,
    /// Human-readable description.
    pub description: String,
    /// Source metric name.
    pub source_metric: String,
    /// Current value of the metric.
    pub current_value: f64,
    /// Threshold that was exceeded.
    pub threshold: f64,
    /// Timestamp (millis since epoch).
    pub timestamp_ms: u64,
}

impl DissonanceEvent {
    pub fn new(
        category: DissonanceCategory,
        severity: f64,
        description: impl Into<String>,
        source_metric: impl Into<String>,
        current_value: f64,
        threshold: f64,
    ) -> Self {
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_millis() as u64;
        Self {
            category,
            severity: severity.clamp(0.0, 1.0),
            description: description.into(),
            source_metric: source_metric.into(),
            current_value,
            threshold,
            timestamp_ms,
        }
    }
}

/// System-wide metrics snapshot for analysis.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage_pct: f64,
    pub memory_usage_mb: f64,
    pub latency_p50_ms: f64,
    pub latency_p99_ms: f64,
    pub error_rate: f64,
    pub throughput_rps: f64,
    pub api_friction_score: f64,
    pub policy_denial_rate: f64,
    pub resonance: f64,
}

/// Thresholds for dissonance detection.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DissonanceThresholds {
    pub cpu_high: f64,
    pub memory_high_mb: f64,
    pub latency_p99_high_ms: f64,
    pub error_rate_high: f64,
    pub api_friction_high: f64,
    pub policy_denial_high: f64,
    pub resonance_min: f64,
}

impl Default for DissonanceThresholds {
    fn default() -> Self {
        Self {
            cpu_high: 80.0,
            memory_high_mb: 4096.0,
            latency_p99_high_ms: 500.0,
            error_rate_high: 0.05,
            api_friction_high: 0.3,
            policy_denial_high: 0.1,
            resonance_min: 0.6,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dissonance_event_severity_clamped() {
        let e = DissonanceEvent::new(
            DissonanceCategory::Computational,
            1.5,
            "test",
            "cpu",
            90.0,
            80.0,
        );
        assert_eq!(e.severity, 1.0);

        let e2 = DissonanceEvent::new(
            DissonanceCategory::Semantic,
            -0.5,
            "test",
            "metric",
            0.0,
            0.0,
        );
        assert_eq!(e2.severity, 0.0);
    }

    #[test]
    fn dissonance_category_display() {
        assert_eq!(format!("{}", DissonanceCategory::Semantic), "Semantic");
        assert_eq!(format!("{}", DissonanceCategory::PolicyDrift), "Policy Drift");
    }

    #[test]
    fn default_thresholds() {
        let t = DissonanceThresholds::default();
        assert_eq!(t.cpu_high, 80.0);
        assert_eq!(t.resonance_min, 0.6);
    }

    #[test]
    fn system_metrics_default() {
        let m = SystemMetrics::default();
        assert_eq!(m.cpu_usage_pct, 0.0);
        assert_eq!(m.resonance, 0.0);
    }

    #[test]
    fn dissonance_event_serde() {
        let e = DissonanceEvent::new(
            DissonanceCategory::Computational,
            0.7,
            "high cpu",
            "cpu_pct",
            85.0,
            80.0,
        );
        let json = serde_json::to_string(&e).unwrap();
        let restored: DissonanceEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.severity, 0.7);
        assert_eq!(restored.category, DissonanceCategory::Computational);
    }
}
