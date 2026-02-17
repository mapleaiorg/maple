use crate::types::{DissonanceCategory, DissonanceEvent, DissonanceThresholds, SystemMetrics};

/// Analyzes system metrics and detects dissonance events.
pub struct DissonanceDetector {
    thresholds: DissonanceThresholds,
}

impl DissonanceDetector {
    pub fn new(thresholds: DissonanceThresholds) -> Self {
        Self { thresholds }
    }

    pub fn with_default_thresholds() -> Self {
        Self::new(DissonanceThresholds::default())
    }

    /// Analyze metrics and return all detected dissonance events.
    pub fn detect(&self, metrics: &SystemMetrics) -> Vec<DissonanceEvent> {
        let mut events = Vec::new();
        events.extend(self.detect_semantic(metrics));
        events.extend(self.detect_computational(metrics));
        events.extend(self.detect_policy_drift(metrics));
        events
    }

    /// Detect semantic dissonance (API friction, error patterns).
    fn detect_semantic(&self, metrics: &SystemMetrics) -> Vec<DissonanceEvent> {
        let mut events = Vec::new();

        if metrics.api_friction_score > self.thresholds.api_friction_high {
            let severity = ((metrics.api_friction_score - self.thresholds.api_friction_high)
                / (1.0 - self.thresholds.api_friction_high))
                .min(1.0);
            events.push(DissonanceEvent::new(
                DissonanceCategory::Semantic,
                severity,
                format!(
                    "API friction score {} exceeds threshold {}",
                    metrics.api_friction_score, self.thresholds.api_friction_high
                ),
                "api_friction_score",
                metrics.api_friction_score,
                self.thresholds.api_friction_high,
            ));
        }

        if metrics.error_rate > self.thresholds.error_rate_high {
            let severity = ((metrics.error_rate - self.thresholds.error_rate_high)
                / (1.0 - self.thresholds.error_rate_high))
                .min(1.0);
            events.push(DissonanceEvent::new(
                DissonanceCategory::Semantic,
                severity,
                format!(
                    "Error rate {:.3} exceeds threshold {:.3}",
                    metrics.error_rate, self.thresholds.error_rate_high
                ),
                "error_rate",
                metrics.error_rate,
                self.thresholds.error_rate_high,
            ));
        }

        events
    }

    /// Detect computational dissonance (CPU, memory, latency).
    fn detect_computational(&self, metrics: &SystemMetrics) -> Vec<DissonanceEvent> {
        let mut events = Vec::new();

        if metrics.cpu_usage_pct > self.thresholds.cpu_high {
            let severity =
                ((metrics.cpu_usage_pct - self.thresholds.cpu_high) / (100.0 - self.thresholds.cpu_high))
                    .min(1.0);
            events.push(DissonanceEvent::new(
                DissonanceCategory::Computational,
                severity,
                format!(
                    "CPU usage {:.1}% exceeds threshold {:.1}%",
                    metrics.cpu_usage_pct, self.thresholds.cpu_high
                ),
                "cpu_usage_pct",
                metrics.cpu_usage_pct,
                self.thresholds.cpu_high,
            ));
        }

        if metrics.memory_usage_mb > self.thresholds.memory_high_mb {
            let severity = ((metrics.memory_usage_mb - self.thresholds.memory_high_mb)
                / self.thresholds.memory_high_mb)
                .min(1.0);
            events.push(DissonanceEvent::new(
                DissonanceCategory::Computational,
                severity,
                format!(
                    "Memory usage {:.0} MB exceeds threshold {:.0} MB",
                    metrics.memory_usage_mb, self.thresholds.memory_high_mb
                ),
                "memory_usage_mb",
                metrics.memory_usage_mb,
                self.thresholds.memory_high_mb,
            ));
        }

        if metrics.latency_p99_ms > self.thresholds.latency_p99_high_ms {
            let severity = ((metrics.latency_p99_ms - self.thresholds.latency_p99_high_ms)
                / self.thresholds.latency_p99_high_ms)
                .min(1.0);
            events.push(DissonanceEvent::new(
                DissonanceCategory::Computational,
                severity,
                format!(
                    "P99 latency {:.1} ms exceeds threshold {:.1} ms",
                    metrics.latency_p99_ms, self.thresholds.latency_p99_high_ms
                ),
                "latency_p99_ms",
                metrics.latency_p99_ms,
                self.thresholds.latency_p99_high_ms,
            ));
        }

        events
    }

    /// Detect policy drift (denial trends, boundary proximity).
    fn detect_policy_drift(&self, metrics: &SystemMetrics) -> Vec<DissonanceEvent> {
        let mut events = Vec::new();

        if metrics.policy_denial_rate > self.thresholds.policy_denial_high {
            let severity = ((metrics.policy_denial_rate - self.thresholds.policy_denial_high)
                / (1.0 - self.thresholds.policy_denial_high))
                .min(1.0);
            events.push(DissonanceEvent::new(
                DissonanceCategory::PolicyDrift,
                severity,
                format!(
                    "Policy denial rate {:.3} exceeds threshold {:.3}",
                    metrics.policy_denial_rate, self.thresholds.policy_denial_high
                ),
                "policy_denial_rate",
                metrics.policy_denial_rate,
                self.thresholds.policy_denial_high,
            ));
        }

        if metrics.resonance < self.thresholds.resonance_min && metrics.resonance > 0.0 {
            let severity = ((self.thresholds.resonance_min - metrics.resonance)
                / self.thresholds.resonance_min)
                .min(1.0);
            events.push(DissonanceEvent::new(
                DissonanceCategory::PolicyDrift,
                severity,
                format!(
                    "Resonance {:.3} below minimum {:.3}",
                    metrics.resonance, self.thresholds.resonance_min
                ),
                "resonance",
                metrics.resonance,
                self.thresholds.resonance_min,
            ));
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn healthy_metrics() -> SystemMetrics {
        SystemMetrics {
            cpu_usage_pct: 40.0,
            memory_usage_mb: 2048.0,
            latency_p50_ms: 10.0,
            latency_p99_ms: 50.0,
            error_rate: 0.01,
            throughput_rps: 1000.0,
            api_friction_score: 0.1,
            policy_denial_rate: 0.02,
            resonance: 0.9,
        }
    }

    #[test]
    fn no_dissonance_for_healthy_metrics() {
        let detector = DissonanceDetector::with_default_thresholds();
        let events = detector.detect(&healthy_metrics());
        assert!(events.is_empty());
    }

    #[test]
    fn detects_high_cpu() {
        let detector = DissonanceDetector::with_default_thresholds();
        let mut metrics = healthy_metrics();
        metrics.cpu_usage_pct = 95.0;
        let events = detector.detect(&metrics);
        assert!(events.iter().any(|e| e.category == DissonanceCategory::Computational
            && e.source_metric == "cpu_usage_pct"));
    }

    #[test]
    fn detects_high_memory() {
        let detector = DissonanceDetector::with_default_thresholds();
        let mut metrics = healthy_metrics();
        metrics.memory_usage_mb = 6000.0;
        let events = detector.detect(&metrics);
        assert!(events.iter().any(|e| e.source_metric == "memory_usage_mb"));
    }

    #[test]
    fn detects_high_latency() {
        let detector = DissonanceDetector::with_default_thresholds();
        let mut metrics = healthy_metrics();
        metrics.latency_p99_ms = 800.0;
        let events = detector.detect(&metrics);
        assert!(events.iter().any(|e| e.source_metric == "latency_p99_ms"));
    }

    #[test]
    fn detects_high_error_rate() {
        let detector = DissonanceDetector::with_default_thresholds();
        let mut metrics = healthy_metrics();
        metrics.error_rate = 0.15;
        let events = detector.detect(&metrics);
        assert!(events.iter().any(|e| e.category == DissonanceCategory::Semantic
            && e.source_metric == "error_rate"));
    }

    #[test]
    fn detects_api_friction() {
        let detector = DissonanceDetector::with_default_thresholds();
        let mut metrics = healthy_metrics();
        metrics.api_friction_score = 0.6;
        let events = detector.detect(&metrics);
        assert!(events.iter().any(|e| e.source_metric == "api_friction_score"));
    }

    #[test]
    fn detects_policy_denial() {
        let detector = DissonanceDetector::with_default_thresholds();
        let mut metrics = healthy_metrics();
        metrics.policy_denial_rate = 0.25;
        let events = detector.detect(&metrics);
        assert!(events.iter().any(|e| e.category == DissonanceCategory::PolicyDrift
            && e.source_metric == "policy_denial_rate"));
    }

    #[test]
    fn detects_low_resonance() {
        let detector = DissonanceDetector::with_default_thresholds();
        let mut metrics = healthy_metrics();
        metrics.resonance = 0.3;
        let events = detector.detect(&metrics);
        assert!(events.iter().any(|e| e.source_metric == "resonance"));
    }

    #[test]
    fn severity_bounded_zero_one() {
        let detector = DissonanceDetector::with_default_thresholds();
        let mut metrics = healthy_metrics();
        metrics.cpu_usage_pct = 100.0;
        metrics.memory_usage_mb = 99999.0;
        metrics.latency_p99_ms = 99999.0;
        metrics.error_rate = 1.0;
        metrics.api_friction_score = 1.0;
        metrics.policy_denial_rate = 1.0;
        metrics.resonance = 0.01;
        let events = detector.detect(&metrics);
        for e in &events {
            assert!(e.severity >= 0.0 && e.severity <= 1.0, "severity out of range: {}", e.severity);
        }
    }

    #[test]
    fn multiple_dissonance_events() {
        let detector = DissonanceDetector::with_default_thresholds();
        let metrics = SystemMetrics {
            cpu_usage_pct: 95.0,
            memory_usage_mb: 6000.0,
            latency_p50_ms: 100.0,
            latency_p99_ms: 800.0,
            error_rate: 0.15,
            throughput_rps: 500.0,
            api_friction_score: 0.5,
            policy_denial_rate: 0.2,
            resonance: 0.3,
        };
        let events = detector.detect(&metrics);
        assert!(events.len() >= 5); // cpu, mem, latency, error, friction, denial, resonance
    }

    #[test]
    fn custom_thresholds() {
        let thresholds = DissonanceThresholds {
            cpu_high: 50.0,
            ..Default::default()
        };
        let detector = DissonanceDetector::new(thresholds);
        let mut metrics = healthy_metrics();
        metrics.cpu_usage_pct = 60.0; // Above custom threshold of 50
        let events = detector.detect(&metrics);
        assert!(events.iter().any(|e| e.source_metric == "cpu_usage_pct"));
    }
}
