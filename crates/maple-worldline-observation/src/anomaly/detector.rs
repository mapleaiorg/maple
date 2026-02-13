//! Anomaly detection algorithms and the fusion/dedup pipeline.
//!
//! Provides:
//! - `AnomalyAlgorithm` trait for pluggable detection
//! - 5 built-in algorithms: Statistical (z-score), Percentile, Trend (CUSUM), Pattern, Correlation
//! - `AnomalyDetector` that runs all algorithms and fuses results

use std::collections::HashMap;

use chrono::{Timelike, Utc};

use crate::baseline::{BaselineEngine, MetricBaseline, MetricId};
use crate::snapshot::ObservationSnapshot;

use super::types::{
    AnomalyCategory, AnomalyDetectorConfig, AnomalyId, AnomalySeverity, ComponentId,
    PerformanceAnomaly, RawAnomaly,
};

// ── Trait ────────────────────────────────────────────────────────────────

/// Pluggable anomaly detection algorithm.
///
/// Each algorithm inspects a current metric value against its established
/// baseline and produces zero or more raw anomaly signals.
pub trait AnomalyAlgorithm {
    /// Detect anomalies for a single metric observation.
    fn detect(
        &mut self,
        metric_id: &MetricId,
        current: f64,
        baseline: &MetricBaseline,
    ) -> Vec<RawAnomaly>;

    /// Name of this detection algorithm (for provenance tracking).
    fn name(&self) -> &str;
}

// ── Helper ──────────────────────────────────────────────────────────────

/// Infer anomaly category from metric name.
fn categorize_metric(metric_id: &MetricId) -> AnomalyCategory {
    let metric = metric_id.metric();
    if metric.contains("latency") {
        AnomalyCategory::LatencyRegression
    } else if metric.contains("error") {
        AnomalyCategory::ErrorRateSpike
    } else if metric.contains("memory") {
        AnomalyCategory::MemoryLeak
    } else if metric.contains("event_count") || metric.contains("throughput") {
        AnomalyCategory::ThroughputDegradation
    } else {
        AnomalyCategory::LatencyRegression
    }
}

// ── 1. Statistical Anomaly (Z-score) ────────────────────────────────────

/// Z-score based anomaly detection.
///
/// Flags values that deviate more than `z_threshold` standard deviations
/// from the baseline mean. Only operates on established baselines.
pub struct StatisticalAnomaly {
    /// Z-score threshold (default: 3.0).
    pub z_threshold: f64,
}

impl StatisticalAnomaly {
    pub fn new(z_threshold: f64) -> Self {
        Self { z_threshold }
    }
}

impl AnomalyAlgorithm for StatisticalAnomaly {
    fn detect(
        &mut self,
        metric_id: &MetricId,
        current: f64,
        baseline: &MetricBaseline,
    ) -> Vec<RawAnomaly> {
        if baseline.std_dev < f64::EPSILON || !baseline.is_established() {
            return vec![];
        }

        let z_score = (current - baseline.mean).abs() / baseline.std_dev;
        if z_score > self.z_threshold {
            let severity = if z_score > self.z_threshold * 2.0 {
                AnomalySeverity::Critical
            } else if z_score > self.z_threshold * 1.5 {
                AnomalySeverity::Warning
            } else {
                AnomalySeverity::Info
            };

            vec![RawAnomaly {
                metric_id: metric_id.clone(),
                category: categorize_metric(metric_id),
                severity,
                score: (z_score / (self.z_threshold * 3.0)).min(1.0),
                description: format!(
                    "z-score {:.2} exceeds threshold {:.1} (mean={:.2}, std={:.2}, current={:.2})",
                    z_score, self.z_threshold, baseline.mean, baseline.std_dev, current
                ),
                detector_name: self.name().to_string(),
                detected_at: Utc::now(),
            }]
        } else {
            vec![]
        }
    }

    fn name(&self) -> &str {
        "statistical"
    }
}

// ── 2. Percentile Anomaly ───────────────────────────────────────────────

/// Percentile shift detection.
///
/// Flags when the current value exceeds `p99 * (1 + shift_threshold)`.
pub struct PercentileAnomaly {
    /// Fractional shift threshold (default: 0.2 = 20%).
    pub shift_threshold: f64,
}

impl PercentileAnomaly {
    pub fn new(shift_threshold: f64) -> Self {
        Self { shift_threshold }
    }
}

impl AnomalyAlgorithm for PercentileAnomaly {
    fn detect(
        &mut self,
        metric_id: &MetricId,
        current: f64,
        baseline: &MetricBaseline,
    ) -> Vec<RawAnomaly> {
        if !baseline.is_established() || baseline.percentiles.p99 < f64::EPSILON {
            return vec![];
        }

        let threshold = baseline.percentiles.p99 * (1.0 + self.shift_threshold);
        if current > threshold {
            let excess_ratio = current / baseline.percentiles.p99;
            let severity = if excess_ratio > 3.0 {
                AnomalySeverity::Critical
            } else if excess_ratio > 2.0 {
                AnomalySeverity::Warning
            } else {
                AnomalySeverity::Info
            };

            vec![RawAnomaly {
                metric_id: metric_id.clone(),
                category: categorize_metric(metric_id),
                severity,
                score: ((excess_ratio - 1.0) / 2.0).min(1.0),
                description: format!(
                    "current {:.2} exceeds p99 {:.2} by {:.1}%",
                    current,
                    baseline.percentiles.p99,
                    (excess_ratio - 1.0) * 100.0
                ),
                detector_name: self.name().to_string(),
                detected_at: Utc::now(),
            }]
        } else {
            vec![]
        }
    }

    fn name(&self) -> &str {
        "percentile"
    }
}

// ── 3. Trend Anomaly (CUSUM) ────────────────────────────────────────────

/// CUSUM (Cumulative Sum) based trend detection.
///
/// Detects persistent small shifts that z-score would miss. Maintains
/// two per-metric accumulators (high/low) and fires when cumulative
/// deviation exceeds the threshold.
pub struct TrendAnomaly {
    /// Threshold multiplier (default: 5.0 × std_dev).
    pub threshold: f64,
    /// CUSUM high accumulators per metric.
    cusum_high: HashMap<MetricId, f64>,
    /// CUSUM low accumulators per metric.
    cusum_low: HashMap<MetricId, f64>,
}

impl TrendAnomaly {
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
            cusum_high: HashMap::new(),
            cusum_low: HashMap::new(),
        }
    }
}

impl AnomalyAlgorithm for TrendAnomaly {
    fn detect(
        &mut self,
        metric_id: &MetricId,
        current: f64,
        baseline: &MetricBaseline,
    ) -> Vec<RawAnomaly> {
        if !baseline.is_established() || baseline.std_dev < f64::EPSILON {
            return vec![];
        }

        let detector_name = "trend".to_string();
        let drift = baseline.std_dev * 0.5;
        let deviation = current - baseline.mean;
        let alert_threshold = self.threshold * baseline.std_dev;

        let cusum_h = self.cusum_high.entry(metric_id.clone()).or_insert(0.0);
        *cusum_h = (*cusum_h + deviation - drift).max(0.0);
        let cusum_h_val = *cusum_h;
        if cusum_h_val > alert_threshold {
            *cusum_h = 0.0;
        }

        let cusum_l = self.cusum_low.entry(metric_id.clone()).or_insert(0.0);
        *cusum_l = (*cusum_l - deviation - drift).max(0.0);
        let cusum_l_val = *cusum_l;
        if cusum_l_val > alert_threshold {
            *cusum_l = 0.0;
        }

        let mut anomalies = vec![];

        if cusum_h_val > alert_threshold {
            anomalies.push(RawAnomaly {
                metric_id: metric_id.clone(),
                category: categorize_metric(metric_id),
                severity: AnomalySeverity::Warning,
                score: (cusum_h_val / (alert_threshold * 2.0)).min(1.0),
                description: format!(
                    "upward trend detected: CUSUM_high {:.2} exceeds threshold {:.2}",
                    cusum_h_val, alert_threshold
                ),
                detector_name: detector_name.clone(),
                detected_at: Utc::now(),
            });
        }

        if cusum_l_val > alert_threshold {
            anomalies.push(RawAnomaly {
                metric_id: metric_id.clone(),
                category: categorize_metric(metric_id),
                severity: AnomalySeverity::Warning,
                score: (cusum_l_val / (alert_threshold * 2.0)).min(1.0),
                description: format!(
                    "downward trend detected: CUSUM_low {:.2} exceeds threshold {:.2}",
                    cusum_l_val, alert_threshold
                ),
                detector_name,
                detected_at: Utc::now(),
            });
        }

        anomalies
    }

    fn name(&self) -> &str {
        "trend"
    }
}

// ── 4. Pattern Anomaly (Seasonal) ───────────────────────────────────────

/// Seasonal pattern anomaly detection.
///
/// Compares the current value against the hour-specific baseline from
/// the seasonal pattern. Requires the baseline to have seasonal patterns.
pub struct PatternAnomaly {
    /// Deviation threshold in standard deviations (default: 2.0).
    pub deviation_threshold: f64,
}

impl PatternAnomaly {
    pub fn new(deviation_threshold: f64) -> Self {
        Self {
            deviation_threshold,
        }
    }
}

impl AnomalyAlgorithm for PatternAnomaly {
    fn detect(
        &mut self,
        metric_id: &MetricId,
        current: f64,
        baseline: &MetricBaseline,
    ) -> Vec<RawAnomaly> {
        if !baseline.is_established() || baseline.time_patterns.is_empty() {
            return vec![];
        }
        if baseline.std_dev < f64::EPSILON {
            return vec![];
        }

        let hour = Utc::now().hour() as usize;
        for pattern in &baseline.time_patterns {
            if pattern.coefficients.len() > hour {
                let hourly_mean = pattern.coefficients[hour];
                if hourly_mean.abs() < f64::EPSILON {
                    continue; // no data for this hour
                }
                let deviation = (current - hourly_mean).abs() / baseline.std_dev;
                if deviation > self.deviation_threshold {
                    return vec![RawAnomaly {
                        metric_id: metric_id.clone(),
                        category: categorize_metric(metric_id),
                        severity: if deviation > self.deviation_threshold * 2.0 {
                            AnomalySeverity::Warning
                        } else {
                            AnomalySeverity::Info
                        },
                        score: (deviation / (self.deviation_threshold * 3.0)).min(1.0),
                        description: format!(
                            "seasonal deviation: current {:.2} vs hour-{} mean {:.2} ({:.1} std devs)",
                            current, hour, hourly_mean, deviation
                        ),
                        detector_name: self.name().to_string(),
                        detected_at: Utc::now(),
                    }];
                }
            }
        }

        vec![]
    }

    fn name(&self) -> &str {
        "pattern"
    }
}

// ── 5. Correlation Anomaly ──────────────────────────────────────────────

/// Cross-metric correlation anomaly detection.
///
/// Tracks expected ratios between metric pairs and flags when the
/// ratio deviates beyond the tolerance.
pub struct CorrelationAnomaly {
    /// Expected ratios between metric pairs.
    pub expected_ratios: HashMap<(MetricId, MetricId), f64>,
    /// Recent values cache for ratio computation.
    recent_values: HashMap<MetricId, f64>,
    /// Tolerance for ratio deviation (default: 0.3 = 30%).
    pub tolerance: f64,
}

impl CorrelationAnomaly {
    pub fn new(tolerance: f64) -> Self {
        Self {
            expected_ratios: HashMap::new(),
            recent_values: HashMap::new(),
            tolerance,
        }
    }

    /// Register an expected ratio between two metrics.
    pub fn add_expected_ratio(&mut self, metric_a: MetricId, metric_b: MetricId, ratio: f64) {
        self.expected_ratios.insert((metric_a, metric_b), ratio);
    }
}

impl AnomalyAlgorithm for CorrelationAnomaly {
    fn detect(
        &mut self,
        metric_id: &MetricId,
        current: f64,
        baseline: &MetricBaseline,
    ) -> Vec<RawAnomaly> {
        if !baseline.is_established() {
            return vec![];
        }

        // Cache the current value
        self.recent_values.insert(metric_id.clone(), current);

        let mut anomalies = vec![];

        // Check all registered pairs involving this metric
        for ((a, b), expected_ratio) in &self.expected_ratios {
            if a != metric_id && b != metric_id {
                continue;
            }
            let val_a = self.recent_values.get(a);
            let val_b = self.recent_values.get(b);

            if let (Some(&va), Some(&vb)) = (val_a, val_b) {
                if vb.abs() < f64::EPSILON {
                    continue;
                }
                let actual_ratio = va / vb;
                let deviation = (actual_ratio - expected_ratio).abs() / expected_ratio.abs().max(f64::EPSILON);

                if deviation > self.tolerance {
                    anomalies.push(RawAnomaly {
                        metric_id: metric_id.clone(),
                        category: AnomalyCategory::CorrelationBreak,
                        severity: if deviation > self.tolerance * 2.0 {
                            AnomalySeverity::Warning
                        } else {
                            AnomalySeverity::Info
                        },
                        score: (deviation / (self.tolerance * 3.0)).min(1.0),
                        description: format!(
                            "correlation break: {}/{} ratio {:.2} vs expected {:.2} ({:.1}% deviation)",
                            a, b, actual_ratio, expected_ratio, deviation * 100.0
                        ),
                        detector_name: self.name().to_string(),
                        detected_at: Utc::now(),
                    });
                }
            }
        }

        anomalies
    }

    fn name(&self) -> &str {
        "correlation"
    }
}

// ── Anomaly Detector (orchestrator) ─────────────────────────────────────

/// The main anomaly detection engine.
///
/// Runs all registered algorithms against each metric observation,
/// then fuses the raw signals into coherent `PerformanceAnomaly` reports
/// using temporal clustering and detector agreement thresholds.
pub struct AnomalyDetector {
    /// Detection algorithms.
    algorithms: Vec<Box<dyn AnomalyAlgorithm>>,
    /// Configuration.
    config: AnomalyDetectorConfig,
    /// Recent anomalies for deduplication (bounded).
    recent_anomalies: Vec<PerformanceAnomaly>,
}

impl AnomalyDetector {
    /// Create a detector with default algorithms.
    pub fn new(config: AnomalyDetectorConfig) -> Self {
        let z = config.z_score_threshold;
        let p = config.percentile_shift_threshold;
        let c = config.cusum_threshold;

        let algorithms: Vec<Box<dyn AnomalyAlgorithm>> = vec![
            Box::new(StatisticalAnomaly::new(z)),
            Box::new(PercentileAnomaly::new(p)),
            Box::new(TrendAnomaly::new(c)),
            Box::new(PatternAnomaly::new(2.0)),
            Box::new(CorrelationAnomaly::new(0.3)),
        ];

        Self {
            algorithms,
            config,
            recent_anomalies: Vec::new(),
        }
    }

    /// Create with custom algorithms.
    pub fn with_algorithms(
        config: AnomalyDetectorConfig,
        algorithms: Vec<Box<dyn AnomalyAlgorithm>>,
    ) -> Self {
        Self {
            algorithms,
            config,
            recent_anomalies: Vec::new(),
        }
    }

    /// Detect anomalies for a single metric value against its baseline.
    pub fn detect(
        &mut self,
        metric_id: &MetricId,
        current: f64,
        baseline: &MetricBaseline,
    ) -> Vec<PerformanceAnomaly> {
        // Run all algorithms
        let mut all_raws = Vec::new();
        for algo in &mut self.algorithms {
            let raws = algo.detect(metric_id, current, baseline);
            all_raws.extend(raws);
        }

        // Fuse
        let fused = self.fuse_anomalies(all_raws, current, baseline);

        // Store and bound
        for anomaly in &fused {
            self.recent_anomalies.push(anomaly.clone());
        }
        if self.recent_anomalies.len() > self.config.max_anomalies {
            let excess = self.recent_anomalies.len() - self.config.max_anomalies;
            self.recent_anomalies.drain(0..excess);
        }

        fused
    }

    /// Detect anomalies from a full observation snapshot.
    ///
    /// Extracts metrics from the snapshot and runs detection for each metric
    /// that has an established baseline.
    pub fn detect_from_snapshot(
        &mut self,
        snapshot: &ObservationSnapshot,
        engine: &BaselineEngine,
    ) -> Vec<PerformanceAnomaly> {
        let mut all_anomalies = Vec::new();

        // System metrics
        let system_metrics = vec![
            (
                MetricId::new("system", "memory_bytes"),
                snapshot.memory_usage_bytes as f64,
            ),
            (
                MetricId::new("system", "sampling_rate"),
                snapshot.current_sampling_rate,
            ),
            (
                MetricId::new("system", "total_events"),
                snapshot.total_events_observed as f64,
            ),
        ];

        for (mid, value) in system_metrics {
            if let Some(baseline) = engine.get_baseline(&mid) {
                let anomalies = self.detect(&mid, value, baseline);
                all_anomalies.extend(anomalies);
            }
        }

        // Per-subsystem metrics
        for (name, summary) in &snapshot.subsystem_summaries {
            let metrics: Vec<(MetricId, f64)> = vec![
                (
                    MetricId::new(name, "event_count"),
                    summary.events_observed as f64,
                ),
                (MetricId::new(name, "error_rate"), summary.error_rate),
            ];

            for (mid, value) in metrics {
                if let Some(baseline) = engine.get_baseline(&mid) {
                    let anomalies = self.detect(&mid, value, baseline);
                    all_anomalies.extend(anomalies);
                }
            }

            if let Some(avg_lat) = summary.avg_latency {
                let mid = MetricId::new(name, "latency_ns");
                if let Some(baseline) = engine.get_baseline(&mid) {
                    let anomalies = self.detect(&mid, avg_lat.as_nanos() as f64, baseline);
                    all_anomalies.extend(anomalies);
                }
            }
        }

        all_anomalies
    }

    /// Get recently detected anomalies.
    pub fn recent_anomalies(&self) -> &[PerformanceAnomaly] {
        &self.recent_anomalies
    }

    /// Number of algorithms registered.
    pub fn algorithm_count(&self) -> usize {
        self.algorithms.len()
    }

    // ── Private: Fusion ─────────────────────────────────────────────

    /// Fuse raw anomaly signals into confirmed anomalies.
    ///
    /// Groups by (metric_id, category), requires minimum detector agreement,
    /// and deduplicates against recent anomalies within the fusion window.
    fn fuse_anomalies(
        &self,
        raws: Vec<RawAnomaly>,
        current: f64,
        baseline: &MetricBaseline,
    ) -> Vec<PerformanceAnomaly> {
        if raws.is_empty() {
            return vec![];
        }

        // Group by (metric_id, category)
        let mut groups: HashMap<(MetricId, AnomalyCategory), Vec<&RawAnomaly>> = HashMap::new();
        for raw in &raws {
            groups
                .entry((raw.metric_id.clone(), raw.category.clone()))
                .or_default()
                .push(raw);
        }

        let total_algorithms = self.algorithms.len().max(1) as f64;
        let now = Utc::now();
        let mut result = Vec::new();

        for ((metric_id, category), group) in groups {
            let agreement = group.len() as f64 / total_algorithms;
            if agreement < self.config.min_detector_agreement {
                continue;
            }

            // Temporal dedup: check if same metric+category exists within fusion window
            let dominated = self.recent_anomalies.iter().any(|existing| {
                existing.metric_id == metric_id
                    && existing.category == category
                    && (now - existing.detected_at).num_seconds()
                        < self.config.fusion_window.as_secs() as i64
            });
            if dominated {
                continue;
            }

            // Take highest severity from group
            let severity = group
                .iter()
                .map(|r| &r.severity)
                .max()
                .cloned()
                .unwrap_or(AnomalySeverity::Info);
            let avg_score =
                group.iter().map(|r| r.score).sum::<f64>() / group.len() as f64;
            let detectors: Vec<String> = group.iter().map(|r| r.detector_name.clone()).collect();
            let description = group
                .iter()
                .map(|r| r.description.as_str())
                .collect::<Vec<_>>()
                .join("; ");

            result.push(PerformanceAnomaly {
                id: AnomalyId::new(),
                category,
                severity,
                component: ComponentId(metric_id.component().to_string()),
                metric_id,
                description,
                score: avg_score,
                detector_agreement: agreement,
                detected_at: now,
                baseline_mean: baseline.mean,
                observed_value: current,
                detectors,
            });
        }

        result
    }
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self::new(AnomalyDetectorConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::types::PercentileEstimates;
    use chrono::Utc;

    /// Helper: create an established baseline with given mean/std.
    fn make_baseline(metric: &str, mean: f64, std_dev: f64) -> MetricBaseline {
        let mid = MetricId::new("test", metric);
        let mut b = MetricBaseline::new(mid);
        b.mean = mean;
        b.std_dev = std_dev;
        b.variance = std_dev * std_dev;
        b.sample_count = 5000;
        b.established_at = Some(Utc::now());
        b.percentiles = PercentileEstimates {
            p50: mean,
            p90: mean + 1.3 * std_dev,
            p95: mean + 1.6 * std_dev,
            p99: mean + 2.3 * std_dev,
            p999: mean + 3.1 * std_dev,
        };
        b
    }

    // ── Statistical detector tests ──────────────────────────────────

    #[test]
    fn statistical_detects_high_z_score() {
        let mut algo = StatisticalAnomaly::new(3.0);
        let mid = MetricId::new("test", "latency_ns");
        let baseline = make_baseline("latency_ns", 100.0, 10.0);

        // 5 std devs away
        let raws = algo.detect(&mid, 150.0, &baseline);
        assert_eq!(raws.len(), 1);
        assert_eq!(raws[0].detector_name, "statistical");
    }

    #[test]
    fn statistical_ignores_normal_value() {
        let mut algo = StatisticalAnomaly::new(3.0);
        let mid = MetricId::new("test", "latency_ns");
        let baseline = make_baseline("latency_ns", 100.0, 10.0);

        // Within 1 std dev
        let raws = algo.detect(&mid, 105.0, &baseline);
        assert!(raws.is_empty());
    }

    #[test]
    fn statistical_ignores_unestablished_baseline() {
        let mut algo = StatisticalAnomaly::new(3.0);
        let mid = MetricId::new("test", "latency_ns");
        let mut baseline = make_baseline("latency_ns", 100.0, 10.0);
        baseline.established_at = None; // unestablished

        let raws = algo.detect(&mid, 200.0, &baseline);
        assert!(raws.is_empty());
    }

    // ── Percentile detector tests ───────────────────────────────────

    #[test]
    fn percentile_detects_beyond_p99() {
        let mut algo = PercentileAnomaly::new(0.2);
        let mid = MetricId::new("test", "latency_ns");
        let baseline = make_baseline("latency_ns", 100.0, 10.0);

        // p99 = 100 + 2.3*10 = 123. Threshold = 123 * 1.2 = 147.6
        let raws = algo.detect(&mid, 200.0, &baseline);
        assert_eq!(raws.len(), 1);
    }

    #[test]
    fn percentile_ignores_normal() {
        let mut algo = PercentileAnomaly::new(0.2);
        let mid = MetricId::new("test", "latency_ns");
        let baseline = make_baseline("latency_ns", 100.0, 10.0);

        let raws = algo.detect(&mid, 110.0, &baseline);
        assert!(raws.is_empty());
    }

    // ── Trend (CUSUM) detector tests ────────────────────────────────

    #[test]
    fn cusum_detects_persistent_drift() {
        let mut algo = TrendAnomaly::new(5.0);
        let mid = MetricId::new("test", "latency_ns");
        let baseline = make_baseline("latency_ns", 100.0, 10.0);

        // Feed values consistently above mean
        let mut detected = false;
        for _ in 0..100 {
            let raws = algo.detect(&mid, 115.0, &baseline);
            if !raws.is_empty() {
                detected = true;
                break;
            }
        }
        assert!(detected, "CUSUM should detect persistent upward drift");
    }

    #[test]
    fn cusum_no_alert_for_noise() {
        let mut algo = TrendAnomaly::new(5.0);
        let mid = MetricId::new("test", "latency_ns");
        let baseline = make_baseline("latency_ns", 100.0, 10.0);

        // Feed values around the mean (slight random variation)
        let mut detected = false;
        for i in 0..50 {
            let value = 100.0 + if i % 2 == 0 { 3.0 } else { -3.0 };
            let raws = algo.detect(&mid, value, &baseline);
            if !raws.is_empty() {
                detected = true;
            }
        }
        assert!(!detected, "CUSUM should not alert on balanced noise");
    }

    // ── Pattern detector tests ──────────────────────────────────────

    #[test]
    fn pattern_detects_off_hour_spike() {
        let mut algo = PatternAnomaly::new(2.0);
        let mid = MetricId::new("test", "latency_ns");
        let mut baseline = make_baseline("latency_ns", 100.0, 10.0);

        // Set up seasonal pattern — current hour has mean 50
        let hour = Utc::now().hour() as usize;
        let mut coefficients = vec![100.0; 24];
        coefficients[hour] = 50.0;
        baseline.time_patterns = vec![crate::baseline::SeasonalPattern {
            pattern_type: crate::baseline::SeasonalPatternType::HourOfDay,
            coefficients,
        }];

        // Current value 100 vs hour mean 50 = 5 std devs
        let raws = algo.detect(&mid, 100.0, &baseline);
        assert!(
            !raws.is_empty(),
            "should detect seasonal deviation"
        );
    }

    // ── Correlation detector tests ──────────────────────────────────

    #[test]
    fn correlation_detects_ratio_break() {
        let mut algo = CorrelationAnomaly::new(0.3);

        let mid_a = MetricId::new("test", "event_count");
        let mid_b = MetricId::new("test", "latency_ns");
        algo.add_expected_ratio(mid_a.clone(), mid_b.clone(), 10.0);

        let baseline_a = make_baseline("event_count", 1000.0, 50.0);
        let baseline_b = make_baseline("latency_ns", 100.0, 10.0);

        // Record normal value for B
        algo.detect(&mid_b, 100.0, &baseline_b);

        // Record value for A that breaks the ratio (expected: 10 * 100 = 1000, actual: 500)
        let raws = algo.detect(&mid_a, 500.0, &baseline_a);
        assert!(
            !raws.is_empty(),
            "should detect correlation break"
        );
    }

    // ── Fusion tests ────────────────────────────────────────────────

    #[test]
    fn fusion_requires_agreement() {
        let config = AnomalyDetectorConfig {
            min_detector_agreement: 0.5, // need at least 50% agreement
            ..AnomalyDetectorConfig::default()
        };
        let mut detector = AnomalyDetector::new(config);

        let mid = MetricId::new("test", "latency_ns");
        let baseline = make_baseline("latency_ns", 100.0, 10.0);

        // A value within normal range won't trigger enough detectors
        let anomalies = detector.detect(&mid, 105.0, &baseline);
        assert!(anomalies.is_empty(), "normal value should not trigger anomaly");
    }

    #[test]
    fn fusion_temporal_dedup() {
        let config = AnomalyDetectorConfig {
            fusion_window: std::time::Duration::from_secs(300),
            min_detector_agreement: 0.2, // low threshold for testing
            ..AnomalyDetectorConfig::default()
        };
        let mut detector = AnomalyDetector::new(config);

        let mid = MetricId::new("test", "latency_ns");
        let baseline = make_baseline("latency_ns", 100.0, 10.0);

        // First detection should fire
        let first = detector.detect(&mid, 200.0, &baseline);
        // Second detection within 5 minutes should be deduplicated
        let second = detector.detect(&mid, 200.0, &baseline);

        assert!(!first.is_empty(), "first detection should fire");
        assert!(second.is_empty(), "second detection should be deduplicated");
    }

    #[test]
    fn detect_from_snapshot_integration() {
        use crate::snapshot::{ObservationSnapshot, SubsystemSummary};
        use crate::usage::UsageAnalyticsSnapshot;

        // Build a baseline engine with low establishment threshold
        let config = crate::baseline::BaselineConfig {
            min_establishment_samples: 10,
            min_establishment_duration: std::time::Duration::from_secs(0),
            ..crate::baseline::BaselineConfig::default()
        };
        let mut engine = BaselineEngine::new(config);

        // Build baseline with normal snapshots
        for _ in 0..20 {
            let mut summaries = HashMap::new();
            summaries.insert(
                "event-fabric".to_string(),
                SubsystemSummary {
                    events_observed: 100,
                    avg_latency: Some(std::time::Duration::from_millis(5)),
                    max_latency: Some(std::time::Duration::from_millis(50)),
                    error_count: 1,
                    error_rate: 0.01,
                },
            );

            let snap = ObservationSnapshot {
                timestamp: Utc::now(),
                total_events_observed: 100,
                current_sampling_rate: 1.0,
                memory_usage_bytes: 1024,
                subsystem_summaries: summaries,
                usage: UsageAnalyticsSnapshot {
                    total_operations: 100,
                    estimated_unique_worldlines: 5,
                    estimated_unique_commitments: 10,
                    estimated_unique_event_types: 8,
                },
            };
            engine.observe_snapshot(&snap);
        }

        // Now inject an anomalous snapshot
        let mut summaries = HashMap::new();
        summaries.insert(
            "event-fabric".to_string(),
            SubsystemSummary {
                events_observed: 100,
                avg_latency: Some(std::time::Duration::from_millis(500)), // 100x spike!
                max_latency: Some(std::time::Duration::from_millis(5000)),
                error_count: 50,
                error_rate: 0.5, // 50x spike!
            },
        );

        let anomalous_snap = ObservationSnapshot {
            timestamp: Utc::now(),
            total_events_observed: 100,
            current_sampling_rate: 1.0,
            memory_usage_bytes: 1024,
            subsystem_summaries: summaries,
            usage: UsageAnalyticsSnapshot {
                total_operations: 100,
                estimated_unique_worldlines: 5,
                estimated_unique_commitments: 10,
                estimated_unique_event_types: 8,
            },
        };

        let mut detector = AnomalyDetector::new(AnomalyDetectorConfig {
            min_detector_agreement: 0.2, // low for testing
            ..AnomalyDetectorConfig::default()
        });

        let anomalies = detector.detect_from_snapshot(&anomalous_snap, &engine);
        assert!(
            !anomalies.is_empty(),
            "should detect anomalies in spiked snapshot"
        );
    }

    #[test]
    fn detector_algorithm_count() {
        let detector = AnomalyDetector::default();
        assert_eq!(detector.algorithm_count(), 5);
    }

    #[test]
    fn detector_recent_anomalies_bounded() {
        let config = AnomalyDetectorConfig {
            max_anomalies: 5,
            min_detector_agreement: 0.2,
            fusion_window: std::time::Duration::from_secs(0), // disable dedup for this test
            ..AnomalyDetectorConfig::default()
        };
        let mut detector = AnomalyDetector::new(config);
        let baseline = make_baseline("latency_ns", 100.0, 10.0);

        for i in 0..20 {
            let mid = MetricId::new("test", &format!("metric_{}", i));
            let mut b = baseline.clone();
            b.metric_id = mid.clone();
            detector.detect(&mid, 300.0, &b);
        }

        assert!(
            detector.recent_anomalies().len() <= 5,
            "recent anomalies should be bounded"
        );
    }
}
