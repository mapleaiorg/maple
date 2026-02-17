use serde::{Deserialize, Serialize};

/// Kernel runtime metrics.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct KernelMetrics {
    /// Total evolution steps attempted.
    pub steps_attempted: u64,
    /// Successful evolutions.
    pub evolutions_succeeded: u64,
    /// Failed evolutions.
    pub evolutions_failed: u64,
    /// Rollbacks performed.
    pub rollbacks: u64,
    /// Current resonance.
    pub current_resonance: f64,
    /// Resonance history (last N values).
    pub resonance_history: Vec<f64>,
    /// Maximum resonance history entries.
    max_history: usize,
}

impl KernelMetrics {
    pub fn new(max_history: usize) -> Self {
        Self {
            max_history,
            ..Default::default()
        }
    }

    pub fn record_resonance(&mut self, resonance: f64) {
        self.current_resonance = resonance;
        self.resonance_history.push(resonance);
        if self.resonance_history.len() > self.max_history {
            self.resonance_history.remove(0);
        }
    }

    pub fn record_success(&mut self) {
        self.steps_attempted += 1;
        self.evolutions_succeeded += 1;
    }

    pub fn record_failure(&mut self) {
        self.steps_attempted += 1;
        self.evolutions_failed += 1;
    }

    pub fn record_rollback(&mut self) {
        self.rollbacks += 1;
    }

    pub fn success_rate(&self) -> f64 {
        if self.steps_attempted == 0 {
            return 1.0;
        }
        self.evolutions_succeeded as f64 / self.steps_attempted as f64
    }

    pub fn avg_resonance(&self) -> f64 {
        if self.resonance_history.is_empty() {
            return 0.0;
        }
        self.resonance_history.iter().sum::<f64>() / self.resonance_history.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_metrics() {
        let mut m = KernelMetrics::new(10);
        m.record_success();
        m.record_success();
        m.record_failure();
        assert_eq!(m.steps_attempted, 3);
        assert_eq!(m.evolutions_succeeded, 2);
        assert!((m.success_rate() - 0.6667).abs() < 0.01);
    }

    #[test]
    fn resonance_history() {
        let mut m = KernelMetrics::new(3);
        m.record_resonance(0.8);
        m.record_resonance(0.9);
        m.record_resonance(0.85);
        m.record_resonance(0.7); // Pushes out 0.8
        assert_eq!(m.resonance_history.len(), 3);
        assert_eq!(m.resonance_history[0], 0.9);
    }

    #[test]
    fn avg_resonance() {
        let mut m = KernelMetrics::new(10);
        m.record_resonance(0.8);
        m.record_resonance(0.9);
        assert!((m.avg_resonance() - 0.85).abs() < 0.001);
    }

    #[test]
    fn empty_metrics() {
        let m = KernelMetrics::new(10);
        assert_eq!(m.success_rate(), 1.0);
        assert_eq!(m.avg_resonance(), 0.0);
    }

    #[test]
    fn metrics_serde() {
        let mut m = KernelMetrics::new(10);
        m.record_success();
        m.record_resonance(0.9);
        let json = serde_json::to_string(&m).unwrap();
        let restored: KernelMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.steps_attempted, 1);
    }
}
