//! Anomaly detection type definitions.
//!
//! Types for representing detected anomalies, their categories, severity levels,
//! and configuration for the anomaly detection pipeline.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::baseline::MetricId;

// ── Identifier Types ────────────────────────────────────────────────────

/// Identifies a system component (subsystem or operator).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComponentId(pub String);

impl std::fmt::Display for ComponentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a detected anomaly.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnomalyId(pub String);

impl AnomalyId {
    /// Generate a new unique anomaly ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for AnomalyId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AnomalyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "anomaly:{}", self.0)
    }
}

// ── Anomaly Severity ────────────────────────────────────────────────────

/// Severity of a detected anomaly.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AnomalySeverity {
    /// Noteworthy but not actionable.
    Info,
    /// Warrants investigation.
    Warning,
    /// Requires immediate attention.
    Critical,
}

impl std::fmt::Display for AnomalySeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

// ── Anomaly Category ────────────────────────────────────────────────────

/// Classification of detected anomalies.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnomalyCategory {
    /// Component is slower than baseline.
    LatencyRegression,
    /// Throughput has dropped.
    ThroughputDegradation,
    /// Memory usage growing without bound.
    MemoryLeak,
    /// Error rate spiking.
    ErrorRateSpike,
    /// Resource approaching limits.
    ResourceExhaustion,
    /// Code path executed far more than expected (optimization target).
    HotPath,
    /// Code that is never or rarely executed (dead code candidate).
    ColdCode,
    /// Operator consistently underperforming.
    OperatorBottleneck,
    /// Cross-metric correlation broken (structural change).
    CorrelationBreak,
}

impl std::fmt::Display for AnomalyCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LatencyRegression => write!(f, "latency-regression"),
            Self::ThroughputDegradation => write!(f, "throughput-degradation"),
            Self::MemoryLeak => write!(f, "memory-leak"),
            Self::ErrorRateSpike => write!(f, "error-rate-spike"),
            Self::ResourceExhaustion => write!(f, "resource-exhaustion"),
            Self::HotPath => write!(f, "hot-path"),
            Self::ColdCode => write!(f, "cold-code"),
            Self::OperatorBottleneck => write!(f, "operator-bottleneck"),
            Self::CorrelationBreak => write!(f, "correlation-break"),
        }
    }
}

// ── Raw Anomaly (pre-fusion) ────────────────────────────────────────────

/// Intermediate anomaly signal from a single detection algorithm.
///
/// Multiple `RawAnomaly` instances are fused into a single `PerformanceAnomaly`
/// by the anomaly fusion layer.
#[derive(Clone, Debug)]
pub struct RawAnomaly {
    /// Which metric triggered this anomaly.
    pub metric_id: MetricId,
    /// Classification.
    pub category: AnomalyCategory,
    /// Severity assessment.
    pub severity: AnomalySeverity,
    /// Detection confidence (0.0 = uncertain, 1.0 = definite).
    pub score: f64,
    /// Human-readable description.
    pub description: String,
    /// Which detection algorithm produced this.
    pub detector_name: String,
    /// When this was detected.
    pub detected_at: DateTime<Utc>,
}

// ── Performance Anomaly (final output) ──────────────────────────────────

/// A confirmed anomaly after fusion and deduplication.
///
/// This is the primary output of the anomaly detection pipeline,
/// consumed by the meaning formation engine (Prompt 13).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceAnomaly {
    /// Unique anomaly identifier.
    pub id: AnomalyId,
    /// Classification.
    pub category: AnomalyCategory,
    /// Severity assessment.
    pub severity: AnomalySeverity,
    /// Which component is affected.
    pub component: ComponentId,
    /// Which metric triggered this.
    pub metric_id: MetricId,
    /// Human-readable description.
    pub description: String,
    /// Fused detection confidence (0.0 to 1.0).
    pub score: f64,
    /// Fraction of detectors that agreed (0.0 to 1.0).
    pub detector_agreement: f64,
    /// When this anomaly was detected.
    pub detected_at: DateTime<Utc>,
    /// Baseline mean at time of detection.
    pub baseline_mean: f64,
    /// Observed value that triggered detection.
    pub observed_value: f64,
    /// Names of contributing detection algorithms.
    pub detectors: Vec<String>,
}

// ── Configuration ───────────────────────────────────────────────────────

/// Configuration for the anomaly detection pipeline.
#[derive(Clone, Debug)]
pub struct AnomalyDetectorConfig {
    /// Temporal clustering window for fusion (anomalies within this window
    /// for the same metric+category are deduplicated).
    pub fusion_window: std::time::Duration,
    /// Minimum fraction of detectors that must agree for an anomaly to be accepted.
    pub min_detector_agreement: f64,
    /// Z-score threshold for the statistical detector.
    pub z_score_threshold: f64,
    /// Fractional shift threshold for the percentile detector.
    pub percentile_shift_threshold: f64,
    /// CUSUM threshold multiplier for the trend detector.
    pub cusum_threshold: f64,
    /// Maximum retained anomalies (memory bound).
    pub max_anomalies: usize,
}

impl Default for AnomalyDetectorConfig {
    fn default() -> Self {
        Self {
            fusion_window: std::time::Duration::from_secs(super::DEFAULT_FUSION_WINDOW_SECS),
            min_detector_agreement: super::DEFAULT_MIN_AGREEMENT,
            z_score_threshold: super::DEFAULT_Z_SCORE_THRESHOLD,
            percentile_shift_threshold: 0.2,
            cusum_threshold: 5.0,
            max_anomalies: super::MAX_ANOMALIES,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anomaly_id_uniqueness() {
        let a = AnomalyId::new();
        let b = AnomalyId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn anomaly_category_all_variants() {
        let categories = vec![
            AnomalyCategory::LatencyRegression,
            AnomalyCategory::ThroughputDegradation,
            AnomalyCategory::MemoryLeak,
            AnomalyCategory::ErrorRateSpike,
            AnomalyCategory::ResourceExhaustion,
            AnomalyCategory::HotPath,
            AnomalyCategory::ColdCode,
            AnomalyCategory::OperatorBottleneck,
            AnomalyCategory::CorrelationBreak,
        ];
        assert_eq!(categories.len(), 9);
        // Each should have a distinct display string
        let displays: std::collections::HashSet<String> =
            categories.iter().map(|c| c.to_string()).collect();
        assert_eq!(displays.len(), 9);
    }

    #[test]
    fn anomaly_severity_ordering() {
        assert!(AnomalySeverity::Info < AnomalySeverity::Warning);
        assert!(AnomalySeverity::Warning < AnomalySeverity::Critical);
    }

    #[test]
    fn performance_anomaly_serialization() {
        let anomaly = PerformanceAnomaly {
            id: AnomalyId::new(),
            category: AnomalyCategory::LatencyRegression,
            severity: AnomalySeverity::Warning,
            component: ComponentId("event-fabric".into()),
            metric_id: MetricId::new("event-fabric", "latency_ns"),
            description: "z-score 4.2 exceeds threshold 3.0".into(),
            score: 0.7,
            detector_agreement: 0.6,
            detected_at: Utc::now(),
            baseline_mean: 5_000_000.0,
            observed_value: 25_000_000.0,
            detectors: vec!["statistical".into(), "percentile".into()],
        };
        let json = serde_json::to_string(&anomaly).unwrap();
        let restored: PerformanceAnomaly = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.category, AnomalyCategory::LatencyRegression);
        assert_eq!(restored.severity, AnomalySeverity::Warning);
    }

    #[test]
    fn component_id_from_metric_id() {
        let mid = MetricId::new("commitment-gate", "error_rate");
        let cid = ComponentId(mid.component().to_string());
        assert_eq!(cid.to_string(), "commitment-gate");
    }

    #[test]
    fn anomaly_detector_config_default() {
        let cfg = AnomalyDetectorConfig::default();
        assert!((cfg.min_detector_agreement - 0.5).abs() < f64::EPSILON);
        assert!((cfg.z_score_threshold - 3.0).abs() < f64::EPSILON);
        assert_eq!(cfg.max_anomalies, 256);
    }
}
