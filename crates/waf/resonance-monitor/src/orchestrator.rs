use crate::detector::DissonanceDetector;
use crate::intent_builder::IntentBuilder;
use crate::types::{DissonanceEvent, DissonanceThresholds, SystemMetrics};
use maple_waf_context_graph::IntentNode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Orchestrates continuous dissonance monitoring.
///
/// Collects metrics, detects dissonance, and produces IntentNodes
/// for the context graph when thresholds are exceeded.
pub struct MonitorOrchestrator {
    detector: DissonanceDetector,
    /// Emergency stop flag — if set, no intents are produced.
    emergency_stop: Arc<AtomicBool>,
    /// Cooldown between intent generation (ms).
    cooldown_ms: u64,
    /// Last intent generation time (ms since epoch).
    last_intent_ms: u64,
    /// History of detected events (ring buffer).
    history: Vec<DissonanceEvent>,
    /// Max history size.
    max_history: usize,
}

impl MonitorOrchestrator {
    pub fn new(thresholds: DissonanceThresholds) -> Self {
        Self {
            detector: DissonanceDetector::new(thresholds),
            emergency_stop: Arc::new(AtomicBool::new(false)),
            cooldown_ms: 5000,
            last_intent_ms: 0,
            history: Vec::new(),
            max_history: 1000,
        }
    }

    pub fn with_cooldown_ms(mut self, ms: u64) -> Self {
        self.cooldown_ms = ms;
        self
    }

    /// Get the emergency stop handle (can be shared across threads).
    pub fn emergency_stop_handle(&self) -> Arc<AtomicBool> {
        self.emergency_stop.clone()
    }

    /// Trigger emergency stop.
    pub fn trigger_emergency_stop(&self) {
        self.emergency_stop.store(true, Ordering::SeqCst);
    }

    /// Clear emergency stop.
    pub fn clear_emergency_stop(&self) {
        self.emergency_stop.store(false, Ordering::SeqCst);
    }

    /// Is emergency stop active?
    pub fn is_stopped(&self) -> bool {
        self.emergency_stop.load(Ordering::SeqCst)
    }

    /// Process a metrics snapshot and return any generated intents.
    pub fn process_metrics(&mut self, metrics: &SystemMetrics, now_ms: u64) -> Vec<IntentNode> {
        if self.is_stopped() {
            return Vec::new();
        }

        let events = self.detector.detect(metrics);

        // Record in history.
        for event in &events {
            if self.history.len() >= self.max_history {
                self.history.remove(0);
            }
            self.history.push(event.clone());
        }

        // Check cooldown (skip for first invocation when last_intent_ms == 0).
        if self.last_intent_ms > 0 && now_ms.saturating_sub(self.last_intent_ms) < self.cooldown_ms
        {
            return Vec::new();
        }

        if events.is_empty() {
            return Vec::new();
        }

        self.last_intent_ms = now_ms;

        // Convert to intents.
        events
            .iter()
            .map(|e| IntentBuilder::from_dissonance(e))
            .collect()
    }

    /// Get the event history.
    pub fn history(&self) -> &[DissonanceEvent] {
        &self.history
    }

    /// Clear history.
    pub fn clear_history(&mut self) {
        self.history.clear();
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

    fn stressed_metrics() -> SystemMetrics {
        SystemMetrics {
            cpu_usage_pct: 95.0,
            memory_usage_mb: 6000.0,
            latency_p50_ms: 100.0,
            latency_p99_ms: 800.0,
            error_rate: 0.15,
            throughput_rps: 500.0,
            api_friction_score: 0.5,
            policy_denial_rate: 0.2,
            resonance: 0.3,
        }
    }

    #[test]
    fn no_intents_for_healthy() {
        let mut orch = MonitorOrchestrator::new(DissonanceThresholds::default());
        let intents = orch.process_metrics(&healthy_metrics(), 1000);
        assert!(intents.is_empty());
    }

    #[test]
    fn generates_intents_for_stress() {
        let mut orch = MonitorOrchestrator::new(DissonanceThresholds::default());
        let intents = orch.process_metrics(&stressed_metrics(), 1000);
        assert!(!intents.is_empty());
    }

    #[test]
    fn cooldown_prevents_repeated_intents() {
        let mut orch =
            MonitorOrchestrator::new(DissonanceThresholds::default()).with_cooldown_ms(5000);
        let intents1 = orch.process_metrics(&stressed_metrics(), 1000);
        assert!(!intents1.is_empty());

        // Within cooldown — no intents.
        let intents2 = orch.process_metrics(&stressed_metrics(), 3000);
        assert!(intents2.is_empty());

        // After cooldown — intents again.
        let intents3 = orch.process_metrics(&stressed_metrics(), 7000);
        assert!(!intents3.is_empty());
    }

    #[test]
    fn emergency_stop_blocks_intents() {
        let mut orch = MonitorOrchestrator::new(DissonanceThresholds::default());
        orch.trigger_emergency_stop();
        let intents = orch.process_metrics(&stressed_metrics(), 1000);
        assert!(intents.is_empty());
    }

    #[test]
    fn clear_emergency_stop() {
        let mut orch = MonitorOrchestrator::new(DissonanceThresholds::default());
        orch.trigger_emergency_stop();
        assert!(orch.is_stopped());
        orch.clear_emergency_stop();
        assert!(!orch.is_stopped());
        let intents = orch.process_metrics(&stressed_metrics(), 1000);
        assert!(!intents.is_empty());
    }

    #[test]
    fn history_recorded() {
        let mut orch = MonitorOrchestrator::new(DissonanceThresholds::default());
        orch.process_metrics(&stressed_metrics(), 1000);
        assert!(!orch.history().is_empty());
    }

    #[test]
    fn history_cleared() {
        let mut orch = MonitorOrchestrator::new(DissonanceThresholds::default());
        orch.process_metrics(&stressed_metrics(), 1000);
        orch.clear_history();
        assert!(orch.history().is_empty());
    }

    #[test]
    fn emergency_stop_handle_shared() {
        let orch = MonitorOrchestrator::new(DissonanceThresholds::default());
        let handle = orch.emergency_stop_handle();
        handle.store(true, Ordering::SeqCst);
        assert!(orch.is_stopped());
    }
}
