//! Baseline type definitions for per-metric behavioral baselines.
//!
//! A baseline represents "normal behavior" learned from observation data.
//! Key types: `MetricId` (what's being tracked), `MetricBaseline` (learned state),
//! `PercentileEstimates` (distribution summary), `DistributionModel` (shape).

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Metric Identification ───────────────────────────────────────────────

/// Identifies a specific metric being baselined.
///
/// Format: `"{component}.{metric_name}"`, e.g. `"event-fabric.latency_ns"`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetricId(pub String);

impl MetricId {
    /// Create a metric ID from component and metric name.
    pub fn new(component: &str, metric: &str) -> Self {
        Self(format!("{}.{}", component, metric))
    }

    /// Extract the component portion (before the first dot).
    pub fn component(&self) -> &str {
        self.0.split('.').next().unwrap_or(&self.0)
    }

    /// Extract the metric name portion (after the first dot).
    pub fn metric(&self) -> &str {
        self.0.split('.').nth(1).unwrap_or(&self.0)
    }
}

impl std::fmt::Display for MetricId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Percentile Estimates ────────────────────────────────────────────────

/// Estimated percentiles from the observation buffer.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PercentileEstimates {
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub p999: f64,
}

// ── Distribution Model ──────────────────────────────────────────────────

/// Categorization of the observed value distribution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DistributionModel {
    /// Not enough data to classify.
    Unknown,
    /// Approximately normal (Gaussian) distribution.
    Normal { skewness: f64 },
    /// Two distinct modes detected (common for latency distributions).
    Bimodal { mode1: f64, mode2: f64 },
    /// Heavy-tailed distribution (extreme values more common than normal).
    HeavyTailed { tail_index: f64 },
}

impl Default for DistributionModel {
    fn default() -> Self {
        Self::Unknown
    }
}

// ── Seasonal Patterns ───────────────────────────────────────────────────

/// Type of seasonal pattern detected.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SeasonalPatternType {
    /// Hour-of-day pattern (24 buckets).
    HourOfDay,
}

/// A detected seasonal pattern in the metric.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeasonalPattern {
    /// The type of seasonal pattern.
    pub pattern_type: SeasonalPatternType,
    /// EWMA means per time bucket (24 entries for HourOfDay).
    pub coefficients: Vec<f64>,
}

// ── Metric Baseline ─────────────────────────────────────────────────────

/// Per-metric baseline representing learned "normal" behavior.
///
/// Updated online via EWMA (Exponentially Weighted Moving Average).
/// Per I.OBS-2: baselines are meaning input only — they never trigger action.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricBaseline {
    /// Which metric this baseline tracks.
    pub metric_id: MetricId,
    /// EWMA of the mean value.
    pub mean: f64,
    /// EWMA of the variance.
    pub variance: f64,
    /// Standard deviation (sqrt of variance, updated on each observation).
    pub std_dev: f64,
    /// Number of observations incorporated.
    pub sample_count: u64,
    /// Estimated percentiles from the observation buffer.
    pub percentiles: PercentileEstimates,
    /// Detected distribution shape.
    pub distribution: DistributionModel,
    /// Seasonal patterns (if detected).
    pub time_patterns: Vec<SeasonalPattern>,
    /// When the baseline was considered established (meets min samples + duration).
    pub established_at: Option<DateTime<Utc>>,
    /// When the first observation was recorded.
    pub first_seen: DateTime<Utc>,
    /// When the baseline was last updated.
    pub last_updated: DateTime<Utc>,
}

impl MetricBaseline {
    /// Create a new unestablished baseline for a metric.
    pub fn new(metric_id: MetricId) -> Self {
        let now = Utc::now();
        Self {
            metric_id,
            mean: 0.0,
            variance: 0.0,
            std_dev: 0.0,
            sample_count: 0,
            percentiles: PercentileEstimates::default(),
            distribution: DistributionModel::Unknown,
            time_patterns: Vec::new(),
            established_at: None,
            first_seen: now,
            last_updated: now,
        }
    }

    /// Whether this baseline has been established (enough data collected).
    pub fn is_established(&self) -> bool {
        self.established_at.is_some()
    }
}

// ── Configuration ───────────────────────────────────────────────────────

/// Configuration for the baseline engine.
#[derive(Clone, Debug)]
pub struct BaselineConfig {
    /// EWMA learning rate (alpha). Default: 0.01 (slow adaptation).
    pub learning_rate: f64,
    /// Minimum samples before a baseline is considered established.
    pub min_establishment_samples: u64,
    /// Minimum duration before a baseline is considered established.
    pub min_establishment_duration: Duration,
    /// Maximum values retained in the percentile buffer per metric.
    pub percentile_buffer_size: usize,
    /// Maximum number of metrics to track (memory bound).
    pub max_metrics: usize,
}

impl Default for BaselineConfig {
    fn default() -> Self {
        Self {
            learning_rate: super::DEFAULT_LEARNING_RATE,
            min_establishment_samples: super::DEFAULT_MIN_ESTABLISHMENT_SAMPLES,
            min_establishment_duration: Duration::from_secs(super::DEFAULT_MIN_ESTABLISHMENT_SECS),
            percentile_buffer_size: super::DEFAULT_PERCENTILE_BUFFER_SIZE,
            max_metrics: super::MAX_TRACKED_METRICS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_id_construction_and_parsing() {
        let mid = MetricId::new("event-fabric", "latency_ns");
        assert_eq!(mid.component(), "event-fabric");
        assert_eq!(mid.metric(), "latency_ns");
        assert_eq!(mid.to_string(), "event-fabric.latency_ns");
    }

    #[test]
    fn metric_id_display() {
        let mid = MetricId::new("system", "memory_bytes");
        assert_eq!(format!("{}", mid), "system.memory_bytes");
    }

    #[test]
    fn metric_id_equality_and_hash() {
        let a = MetricId::new("x", "y");
        let b = MetricId::new("x", "y");
        assert_eq!(a, b);

        let mut set = std::collections::HashSet::new();
        set.insert(a.clone());
        assert!(set.contains(&b));
    }

    #[test]
    fn metric_baseline_default_is_unestablished() {
        let b = MetricBaseline::new(MetricId::new("test", "metric"));
        assert!(!b.is_established());
        assert_eq!(b.sample_count, 0);
        assert_eq!(b.mean, 0.0);
    }

    #[test]
    fn percentile_estimates_default() {
        let p = PercentileEstimates::default();
        assert_eq!(p.p50, 0.0);
        assert_eq!(p.p99, 0.0);
    }

    #[test]
    fn distribution_model_variants() {
        let _ = DistributionModel::Unknown;
        let _ = DistributionModel::Normal { skewness: 0.1 };
        let _ = DistributionModel::Bimodal {
            mode1: 5.0,
            mode2: 50.0,
        };
        let _ = DistributionModel::HeavyTailed { tail_index: 3.0 };
    }

    #[test]
    fn baseline_config_default() {
        let cfg = BaselineConfig::default();
        assert!((cfg.learning_rate - 0.01).abs() < f64::EPSILON);
        assert_eq!(cfg.min_establishment_samples, 1000);
        assert_eq!(cfg.min_establishment_duration, Duration::from_secs(86_400));
        assert_eq!(cfg.percentile_buffer_size, 10_000);
        assert_eq!(cfg.max_metrics, 256);
    }

    #[test]
    fn metric_baseline_serialization() {
        let b = MetricBaseline::new(MetricId::new("gate", "latency"));
        let json = serde_json::to_string(&b).unwrap();
        let restored: MetricBaseline = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.metric_id, b.metric_id);
        assert!(!restored.is_established());
    }

    #[test]
    fn seasonal_pattern_serialization() {
        let p = SeasonalPattern {
            pattern_type: SeasonalPatternType::HourOfDay,
            coefficients: vec![1.0; 24],
        };
        let json = serde_json::to_string(&p).unwrap();
        let _: SeasonalPattern = serde_json::from_str(&json).unwrap();
    }
}
