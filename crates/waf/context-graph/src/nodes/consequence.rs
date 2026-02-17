use crate::types::ContentHash;
use serde::{Deserialize, Serialize};

/// Records the production outcome of a deployed change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsequenceNode {
    /// Health status after deployment.
    pub status: HealthStatus,
    /// Actual resonance measured in production.
    pub actual_resonance: f64,
    /// Target resonance that was aimed for.
    pub target_resonance: f64,
    /// Node ID to revert to if rollback is needed.
    pub rollback_pointer: Option<ContentHash>,
    /// Detailed metrics collected during observation window.
    pub metrics: DeploymentMetrics,
    /// Duration (seconds) the observation window ran before declaring status.
    pub observation_duration_secs: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HealthStatus {
    Stable,
    Degraded,
    CriticalFailure,
    RollbackTriggered,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stable => write!(f, "Stable"),
            Self::Degraded => write!(f, "Degraded"),
            Self::CriticalFailure => write!(f, "Critical Failure"),
            Self::RollbackTriggered => write!(f, "Rollback Triggered"),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DeploymentMetrics {
    pub latency_p50_ms: f64,
    pub latency_p99_ms: f64,
    pub error_rate: f64,
    pub throughput_rps: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_pct: f64,
    pub invariants_checked: usize,
    pub invariants_passed: usize,
}

impl ConsequenceNode {
    pub fn stable(actual_resonance: f64) -> Self {
        Self {
            status: HealthStatus::Stable,
            actual_resonance,
            target_resonance: 0.0,
            rollback_pointer: None,
            metrics: DeploymentMetrics::default(),
            observation_duration_secs: 0,
        }
    }

    pub fn degraded(actual_resonance: f64, rollback_pointer: ContentHash) -> Self {
        Self {
            status: HealthStatus::Degraded,
            actual_resonance,
            target_resonance: 0.0,
            rollback_pointer: Some(rollback_pointer),
            metrics: DeploymentMetrics::default(),
            observation_duration_secs: 0,
        }
    }

    pub fn resonance_delta(&self) -> f64 {
        self.actual_resonance - self.target_resonance
    }

    pub fn is_healthy(&self) -> bool {
        matches!(self.status, HealthStatus::Stable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consequence_stable() {
        let c = ConsequenceNode::stable(0.95);
        assert!(c.is_healthy());
        assert_eq!(c.status, HealthStatus::Stable);
    }

    #[test]
    fn consequence_degraded() {
        let c = ConsequenceNode::degraded(0.3, ContentHash::hash(b"rollback"));
        assert!(!c.is_healthy());
        assert!(c.rollback_pointer.is_some());
    }

    #[test]
    fn health_status_display() {
        assert_eq!(format!("{}", HealthStatus::Stable), "Stable");
        assert_eq!(format!("{}", HealthStatus::RollbackTriggered), "Rollback Triggered");
    }

    #[test]
    fn consequence_serde() {
        let c = ConsequenceNode::stable(0.9);
        let json = serde_json::to_string(&c).unwrap();
        let restored: ConsequenceNode = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.actual_resonance, 0.9);
    }

    #[test]
    fn deployment_metrics_default() {
        let m = DeploymentMetrics::default();
        assert_eq!(m.error_rate, 0.0);
        assert_eq!(m.invariants_checked, 0);
    }
}
