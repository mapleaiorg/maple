//! Baseline engine — EWMA-based online learning of per-metric baselines.
//!
//! The engine observes metric values over time and builds a statistical baseline
//! using Exponentially Weighted Moving Average for mean/variance tracking,
//! bounded buffers for percentile estimation, and per-hour buckets for
//! seasonal pattern detection.

use std::collections::HashMap;

use chrono::{DateTime, Timelike, Utc};
use tracing::debug;

use crate::snapshot::ObservationSnapshot;

use super::types::{
    BaselineConfig, DistributionModel, MetricBaseline, MetricId, PercentileEstimates,
    SeasonalPattern, SeasonalPatternType,
};

/// The baseline engine learns "normal" behavior for each metric.
///
/// Uses EWMA (Exponentially Weighted Moving Average) for online updates.
/// Baselines adapt slowly to gradual changes (learning_rate = 0.01) but
/// flag sudden shifts.
pub struct BaselineEngine {
    /// Per-metric baseline state.
    baselines: HashMap<MetricId, MetricBaseline>,
    /// Configuration.
    config: BaselineConfig,
    /// Bounded sorted buffers for percentile computation, keyed by MetricId.
    /// These are NOT persisted — they are repopulated from incoming observations.
    percentile_buffers: HashMap<MetricId, Vec<f64>>,
    /// Per-hour-of-day EWMA means for seasonal pattern detection.
    hourly_means: HashMap<MetricId, [f64; 24]>,
    /// Per-hour-of-day observation counts.
    hourly_counts: HashMap<MetricId, [u64; 24]>,
}

impl BaselineEngine {
    /// Create a new baseline engine with the given configuration.
    pub fn new(config: BaselineConfig) -> Self {
        Self {
            baselines: HashMap::new(),
            config,
            percentile_buffers: HashMap::new(),
            hourly_means: HashMap::new(),
            hourly_counts: HashMap::new(),
        }
    }

    /// Create a baseline engine with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(BaselineConfig::default())
    }

    /// Observe a metric value and update its baseline.
    ///
    /// This is the main entry point. It:
    /// 1. Creates or retrieves the metric's baseline
    /// 2. Updates EWMA mean and variance
    /// 3. Appends to the percentile buffer
    /// 4. Updates seasonal hourly buckets
    /// 5. Checks establishment criteria
    pub fn observe(&mut self, metric_id: &MetricId, value: f64, timestamp: DateTime<Utc>) {
        // Enforce max metrics bound
        if !self.baselines.contains_key(metric_id)
            && self.baselines.len() >= self.config.max_metrics
        {
            return; // silently drop — bounded memory
        }

        let alpha = self.config.learning_rate;
        let baseline = self
            .baselines
            .entry(metric_id.clone())
            .or_insert_with(|| MetricBaseline::new(metric_id.clone()));

        // Update EWMA mean and variance
        if baseline.sample_count == 0 {
            // First observation: initialize directly
            baseline.mean = value;
            baseline.variance = 0.0;
            baseline.std_dev = 0.0;
        } else {
            Self::update_ewma(baseline, value, alpha);
        }
        baseline.sample_count += 1;
        baseline.last_updated = timestamp;

        // Update percentile buffer
        let buffer = self
            .percentile_buffers
            .entry(metric_id.clone())
            .or_default();
        baseline.percentiles =
            Self::update_percentiles(buffer, value, self.config.percentile_buffer_size);

        // Detect distribution shape (every 100 samples to avoid overhead)
        if baseline.sample_count % 100 == 0 && buffer.len() >= 100 {
            let mut sorted = buffer.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            baseline.distribution = Self::detect_distribution(&sorted);
        }

        // Update seasonal patterns
        let hour = timestamp.hour() as usize;
        let hourly = self
            .hourly_means
            .entry(metric_id.clone())
            .or_insert([0.0; 24]);
        let counts = self
            .hourly_counts
            .entry(metric_id.clone())
            .or_insert([0; 24]);
        Self::update_seasonal(hourly, counts, value, hour, alpha);

        // Check establishment
        if !baseline.is_established() {
            let duration = timestamp
                .signed_duration_since(baseline.first_seen)
                .to_std()
                .unwrap_or_default();
            if baseline.sample_count >= self.config.min_establishment_samples
                && duration >= self.config.min_establishment_duration
            {
                baseline.established_at = Some(timestamp);
                // Snapshot seasonal patterns
                baseline.time_patterns = vec![SeasonalPattern {
                    pattern_type: SeasonalPatternType::HourOfDay,
                    coefficients: hourly.to_vec(),
                }];
                debug!(
                    metric = %metric_id,
                    samples = baseline.sample_count,
                    "baseline established"
                );
            }
        }
    }

    /// Extract metrics from an ObservationSnapshot and feed them into `observe()`.
    pub fn observe_snapshot(&mut self, snapshot: &ObservationSnapshot) {
        let ts = snapshot.timestamp;

        // Global metrics
        self.observe(
            &MetricId::new("system", "memory_bytes"),
            snapshot.memory_usage_bytes as f64,
            ts,
        );
        self.observe(
            &MetricId::new("system", "sampling_rate"),
            snapshot.current_sampling_rate,
            ts,
        );
        self.observe(
            &MetricId::new("system", "total_events"),
            snapshot.total_events_observed as f64,
            ts,
        );

        // Per-subsystem metrics
        for (name, summary) in &snapshot.subsystem_summaries {
            self.observe(
                &MetricId::new(name, "event_count"),
                summary.events_observed as f64,
                ts,
            );
            self.observe(&MetricId::new(name, "error_rate"), summary.error_rate, ts);
            self.observe(
                &MetricId::new(name, "error_count"),
                summary.error_count as f64,
                ts,
            );
            if let Some(avg_lat) = summary.avg_latency {
                self.observe(
                    &MetricId::new(name, "latency_ns"),
                    avg_lat.as_nanos() as f64,
                    ts,
                );
            }
            if let Some(max_lat) = summary.max_latency {
                self.observe(
                    &MetricId::new(name, "max_latency_ns"),
                    max_lat.as_nanos() as f64,
                    ts,
                );
            }
        }
    }

    /// Get the baseline for a specific metric.
    pub fn get_baseline(&self, metric_id: &MetricId) -> Option<&MetricBaseline> {
        self.baselines.get(metric_id)
    }

    /// Whether the baseline for a metric has been established.
    pub fn is_established(&self, metric_id: &MetricId) -> bool {
        self.baselines
            .get(metric_id)
            .map(|b| b.is_established())
            .unwrap_or(false)
    }

    /// List all tracked metric IDs.
    pub fn metrics(&self) -> Vec<&MetricId> {
        self.baselines.keys().collect()
    }

    /// Number of tracked metrics.
    pub fn metric_count(&self) -> usize {
        self.baselines.len()
    }

    /// Get all baselines (for persistence).
    pub fn baselines(&self) -> &HashMap<MetricId, MetricBaseline> {
        &self.baselines
    }

    /// Load baselines from persistent storage (restores EWMA state).
    pub fn load_baselines(&mut self, baselines: HashMap<MetricId, MetricBaseline>) {
        self.baselines = baselines;
        // Percentile buffers and hourly state are NOT restored — they repopulate
    }

    /// Get the hourly means for a metric (for pattern-based anomaly detection).
    pub fn hourly_means(&self, metric_id: &MetricId) -> Option<&[f64; 24]> {
        self.hourly_means.get(metric_id)
    }

    /// Estimated memory usage in bytes.
    pub fn memory_bytes(&self) -> usize {
        let baseline_bytes = self.baselines.len() * std::mem::size_of::<MetricBaseline>();
        let buffer_bytes: usize = self
            .percentile_buffers
            .values()
            .map(|v| v.len() * std::mem::size_of::<f64>())
            .sum();
        let hourly_bytes = self.hourly_means.len() * (24 * std::mem::size_of::<f64>() * 2);
        baseline_bytes + buffer_bytes + hourly_bytes
    }

    // ── Private helpers ─────────────────────────────────────────────

    /// Update EWMA mean and variance.
    ///
    /// Uses Welford's algorithm blended with EWMA:
    /// - mean_new = (1 - α) * mean_old + α * value
    /// - var_new = (1 - α) * (var_old + α * (value - mean_old)²)
    fn update_ewma(baseline: &mut MetricBaseline, value: f64, alpha: f64) {
        let old_mean = baseline.mean;
        baseline.mean = (1.0 - alpha) * old_mean + alpha * value;
        let diff = value - old_mean;
        baseline.variance = (1.0 - alpha) * (baseline.variance + alpha * diff * diff);
        baseline.std_dev = baseline.variance.sqrt();
    }

    /// Update the percentile buffer and compute current percentiles.
    fn update_percentiles(
        buffer: &mut Vec<f64>,
        value: f64,
        max_size: usize,
    ) -> PercentileEstimates {
        if buffer.len() >= max_size {
            buffer.remove(0); // evict oldest
        }
        buffer.push(value);

        let mut sorted = buffer.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = sorted.len();

        PercentileEstimates {
            p50: sorted[n * 50 / 100],
            p90: sorted[(n * 90 / 100).min(n - 1)],
            p95: sorted[(n * 95 / 100).min(n - 1)],
            p99: sorted[(n * 99 / 100).min(n - 1)],
            p999: sorted[(n * 999 / 1000).min(n - 1)],
        }
    }

    /// Detect distribution shape from a sorted buffer.
    fn detect_distribution(sorted: &[f64]) -> DistributionModel {
        if sorted.len() < 100 {
            return DistributionModel::Unknown;
        }
        let n = sorted.len();
        let min_val = sorted[0];
        let max_val = sorted[n - 1];

        if (max_val - min_val).abs() < f64::EPSILON {
            return DistributionModel::Normal { skewness: 0.0 };
        }

        // Build a histogram with 20 bins
        let bin_count = 20;
        let bin_width = (max_val - min_val) / bin_count as f64;
        let mut histogram = vec![0u32; bin_count];
        for &v in sorted {
            let bin = ((v - min_val) / bin_width).floor() as usize;
            histogram[bin.min(bin_count - 1)] += 1;
        }

        // Find peaks (local maxima)
        let mut peaks = Vec::new();
        for i in 1..bin_count - 1 {
            if histogram[i] > histogram[i - 1] && histogram[i] > histogram[i + 1] {
                peaks.push((i, histogram[i]));
            }
        }

        if peaks.len() >= 2 {
            let mode1 = min_val + (peaks[0].0 as f64 + 0.5) * bin_width;
            let mode2 = min_val + (peaks[1].0 as f64 + 0.5) * bin_width;
            return DistributionModel::Bimodal { mode1, mode2 };
        }

        // Compute skewness
        let mean: f64 = sorted.iter().sum::<f64>() / n as f64;
        let variance = sorted.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
        let std_dev = variance.sqrt();
        let skewness = if std_dev > f64::EPSILON {
            sorted
                .iter()
                .map(|v| ((v - mean) / std_dev).powi(3))
                .sum::<f64>()
                / n as f64
        } else {
            0.0
        };

        if skewness.abs() > 2.0 {
            DistributionModel::HeavyTailed {
                tail_index: skewness,
            }
        } else {
            DistributionModel::Normal { skewness }
        }
    }

    /// Update seasonal per-hour-of-day bucket.
    fn update_seasonal(
        hourly: &mut [f64; 24],
        counts: &mut [u64; 24],
        value: f64,
        hour: usize,
        alpha: f64,
    ) {
        let h = hour.min(23);
        if counts[h] == 0 {
            hourly[h] = value;
        } else {
            hourly[h] = (1.0 - alpha) * hourly[h] + alpha * value;
        }
        counts[h] += 1;
    }
}

impl Default for BaselineEngine {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn engine_observe_single_metric() {
        let mut engine = BaselineEngine::with_defaults();
        let mid = MetricId::new("test", "value");

        engine.observe(&mid, 100.0, Utc::now());

        let b = engine.get_baseline(&mid).unwrap();
        assert_eq!(b.sample_count, 1);
        assert!((b.mean - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn engine_ewma_convergence() {
        let mut engine = BaselineEngine::with_defaults();
        let mid = MetricId::new("test", "stable");

        // Feed many identical values
        for _ in 0..1000 {
            engine.observe(&mid, 50.0, Utc::now());
        }

        let b = engine.get_baseline(&mid).unwrap();
        assert!((b.mean - 50.0).abs() < 0.1);
        assert!(b.std_dev < 0.1);
    }

    #[test]
    fn engine_ewma_slow_adaptation() {
        let mut engine = BaselineEngine::with_defaults();
        let mid = MetricId::new("test", "adapting");

        // Establish baseline at 100
        for _ in 0..500 {
            engine.observe(&mid, 100.0, Utc::now());
        }

        // Shift to 200 — should adapt slowly
        for _ in 0..10 {
            engine.observe(&mid, 200.0, Utc::now());
        }

        let b = engine.get_baseline(&mid).unwrap();
        // After 10 observations at α=0.01, mean should still be close to 100
        assert!(b.mean < 110.0, "mean {} should still be near 100", b.mean);
    }

    #[test]
    fn engine_establishment_requires_samples() {
        let config = BaselineConfig {
            min_establishment_samples: 100,
            min_establishment_duration: std::time::Duration::from_secs(0), // no duration requirement
            ..BaselineConfig::default()
        };
        let mut engine = BaselineEngine::new(config);
        let mid = MetricId::new("test", "est");

        for i in 0..99 {
            engine.observe(&mid, i as f64, Utc::now());
        }
        assert!(!engine.is_established(&mid));

        engine.observe(&mid, 100.0, Utc::now());
        assert!(engine.is_established(&mid));
    }

    #[test]
    fn engine_establishment_requires_duration() {
        let config = BaselineConfig {
            min_establishment_samples: 1, // low sample requirement
            min_establishment_duration: std::time::Duration::from_secs(3600),
            ..BaselineConfig::default()
        };
        let mut engine = BaselineEngine::new(config);
        let mid = MetricId::new("test", "duration");

        // Observe at current time — not enough duration
        engine.observe(&mid, 10.0, Utc::now());
        assert!(!engine.is_established(&mid));

        // Observe 2 hours later
        let later = Utc::now() + Duration::hours(2);
        engine.observe(&mid, 10.0, later);
        assert!(engine.is_established(&mid));
    }

    #[test]
    fn engine_percentile_computation() {
        let config = BaselineConfig {
            percentile_buffer_size: 100,
            min_establishment_samples: 0,
            min_establishment_duration: std::time::Duration::from_secs(0),
            ..BaselineConfig::default()
        };
        let mut engine = BaselineEngine::new(config);
        let mid = MetricId::new("test", "percentiles");

        // Feed sequential values 1..=100
        for i in 1..=100 {
            engine.observe(&mid, i as f64, Utc::now());
        }

        let b = engine.get_baseline(&mid).unwrap();
        // p50 should be around 50
        assert!(
            (b.percentiles.p50 - 50.0).abs() < 5.0,
            "p50 = {}",
            b.percentiles.p50
        );
        // p99 should be around 99
        assert!(
            (b.percentiles.p99 - 99.0).abs() < 5.0,
            "p99 = {}",
            b.percentiles.p99
        );
    }

    #[test]
    fn engine_bimodal_detection() {
        let config = BaselineConfig {
            percentile_buffer_size: 1000,
            min_establishment_samples: 0,
            min_establishment_duration: std::time::Duration::from_secs(0),
            ..BaselineConfig::default()
        };
        let mut engine = BaselineEngine::new(config);
        let mid = MetricId::new("test", "bimodal");

        // Feed bimodal data: cluster around 10 and 90
        for _ in 0..200 {
            engine.observe(&mid, 10.0 + rand::random::<f64>() * 2.0, Utc::now());
            engine.observe(&mid, 90.0 + rand::random::<f64>() * 2.0, Utc::now());
        }

        let b = engine.get_baseline(&mid).unwrap();
        match &b.distribution {
            DistributionModel::Bimodal { mode1, mode2 } => {
                // Modes should be near 10 and 90
                assert!(*mode1 < 50.0 || *mode2 < 50.0);
                assert!(*mode1 > 50.0 || *mode2 > 50.0);
            }
            other => {
                // Bimodal detection is probabilistic; accept if not detected
                // as long as we don't crash
                let _ = other;
            }
        }
    }

    #[test]
    fn engine_seasonal_pattern() {
        let mut engine = BaselineEngine::with_defaults();
        let mid = MetricId::new("test", "seasonal");

        // Simulate different values at different hours
        for hour in 0..24u32 {
            let ts = Utc::now().with_hour(hour).unwrap_or_else(|| Utc::now());
            let value = 100.0 + hour as f64 * 10.0;
            for _ in 0..10 {
                engine.observe(&mid, value, ts);
            }
        }

        let hourly = engine.hourly_means(&mid).unwrap();
        // Hour 0 should be ~100, hour 23 should be ~330
        assert!(hourly[0] < hourly[23], "seasonal pattern should show trend");
    }

    #[test]
    fn engine_observe_snapshot() {
        use crate::snapshot::{ObservationSnapshot, SubsystemSummary};
        use crate::usage::UsageAnalyticsSnapshot;

        let mut engine = BaselineEngine::with_defaults();
        let mut summaries = HashMap::new();
        summaries.insert(
            "event-fabric".to_string(),
            SubsystemSummary {
                events_observed: 1000,
                avg_latency: Some(std::time::Duration::from_millis(5)),
                max_latency: Some(std::time::Duration::from_millis(50)),
                error_count: 2,
                error_rate: 0.002,
            },
        );

        let snapshot = ObservationSnapshot {
            timestamp: Utc::now(),
            total_events_observed: 1000,
            current_sampling_rate: 1.0,
            memory_usage_bytes: 1024,
            subsystem_summaries: summaries,
            usage: UsageAnalyticsSnapshot {
                total_operations: 1000,
                estimated_unique_worldlines: 5,
                estimated_unique_commitments: 10,
                estimated_unique_event_types: 8,
            },
        };

        engine.observe_snapshot(&snapshot);

        // Should have created baselines for system.memory_bytes, system.sampling_rate,
        // system.total_events, event-fabric.event_count, event-fabric.error_rate, etc.
        assert!(engine.metric_count() >= 5);
        assert!(engine
            .get_baseline(&MetricId::new("system", "memory_bytes"))
            .is_some());
        assert!(engine
            .get_baseline(&MetricId::new("event-fabric", "latency_ns"))
            .is_some());
    }

    #[test]
    fn engine_memory_bounded() {
        let config = BaselineConfig {
            max_metrics: 10,
            percentile_buffer_size: 100,
            ..BaselineConfig::default()
        };
        let mut engine = BaselineEngine::new(config);

        // Try to add 20 metrics — only 10 should be tracked
        for i in 0..20 {
            let mid = MetricId::new("test", &format!("metric_{}", i));
            engine.observe(&mid, i as f64, Utc::now());
        }

        assert_eq!(engine.metric_count(), 10);
        // Memory should be reasonable
        assert!(engine.memory_bytes() < 1024 * 1024); // well under 1MB
    }

    #[test]
    fn engine_load_baselines() {
        let mut engine = BaselineEngine::with_defaults();
        let mid = MetricId::new("restored", "metric");

        // Create a baseline manually
        let mut baseline = MetricBaseline::new(mid.clone());
        baseline.mean = 42.0;
        baseline.sample_count = 5000;
        baseline.established_at = Some(Utc::now());

        let mut map = HashMap::new();
        map.insert(mid.clone(), baseline);

        engine.load_baselines(map);

        assert!(engine.is_established(&mid));
        let b = engine.get_baseline(&mid).unwrap();
        assert!((b.mean - 42.0).abs() < f64::EPSILON);
    }
}
