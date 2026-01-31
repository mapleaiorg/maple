//! Health assessment types and computation.
//!
//! Multi-dimensional health assessment for Resonator instances.

use chrono::{DateTime, Utc};
use palm_types::InstanceId;
use serde::{Deserialize, Serialize};

use crate::config::HealthThresholds;

/// Overall health status of an instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverallHealth {
    /// Instance is fully healthy across all dimensions.
    Healthy,

    /// Instance is operational but with degraded performance.
    Degraded,

    /// Instance is unhealthy and may need intervention.
    Unhealthy,

    /// Health status is unknown (probes haven't completed).
    Unknown,
}

impl std::fmt::Display for OverallHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OverallHealth::Healthy => write!(f, "healthy"),
            OverallHealth::Degraded => write!(f, "degraded"),
            OverallHealth::Unhealthy => write!(f, "unhealthy"),
            OverallHealth::Unknown => write!(f, "unknown"),
        }
    }
}

/// Individual dimension health.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DimensionHealth {
    /// Dimension is healthy.
    Healthy,
    /// Dimension is degraded.
    Degraded,
    /// Dimension is unhealthy.
    Unhealthy,
    /// Dimension status is unknown.
    Unknown,
}

/// Health assessment for a single dimension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionAssessment {
    /// Current value (0.0-1.0).
    pub value: f64,

    /// Health status for this dimension.
    pub status: DimensionHealth,

    /// Trend direction (-1.0 to 1.0, negative = declining).
    pub trend: f64,

    /// Time of last measurement.
    pub last_measured: DateTime<Utc>,

    /// Number of consecutive failures.
    pub consecutive_failures: u32,

    /// Number of consecutive successes.
    pub consecutive_successes: u32,
}

impl DimensionAssessment {
    /// Create a new unknown assessment.
    pub fn unknown() -> Self {
        Self {
            value: 0.0,
            status: DimensionHealth::Unknown,
            trend: 0.0,
            last_measured: Utc::now(),
            consecutive_failures: 0,
            consecutive_successes: 0,
        }
    }

    /// Update assessment with a new measurement.
    pub fn update(&mut self, value: f64, healthy_threshold: f64, degraded_threshold: f64) {
        // Calculate trend based on difference from previous value
        let old_value = self.value;
        self.trend = (value - old_value).clamp(-1.0, 1.0);

        self.value = value;
        self.last_measured = Utc::now();

        // Determine status
        if value >= healthy_threshold {
            self.status = DimensionHealth::Healthy;
            self.consecutive_successes += 1;
            self.consecutive_failures = 0;
        } else if value >= degraded_threshold {
            self.status = DimensionHealth::Degraded;
            // Degraded doesn't reset counters
        } else {
            self.status = DimensionHealth::Unhealthy;
            self.consecutive_failures += 1;
            self.consecutive_successes = 0;
        }
    }

    /// Record a probe failure.
    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        self.consecutive_successes = 0;
        self.status = DimensionHealth::Unhealthy;
        self.last_measured = Utc::now();
    }
}

/// Complete health assessment for an instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAssessment {
    /// Instance being assessed.
    pub instance_id: InstanceId,

    /// Overall health status.
    pub overall: OverallHealth,

    /// Overall health score (0.0-1.0).
    pub overall_score: f64,

    /// Presence gradient assessment.
    pub presence: DimensionAssessment,

    /// Coupling capacity assessment.
    pub coupling: DimensionAssessment,

    /// Attention budget assessment.
    pub attention: DimensionAssessment,

    /// Time of this assessment.
    pub assessed_at: DateTime<Utc>,

    /// Whether the instance is currently isolated.
    pub is_isolated: bool,

    /// Number of recovery attempts made.
    pub recovery_attempts: u32,

    /// Additional context or notes.
    pub notes: Vec<String>,
}

impl HealthAssessment {
    /// Create a new assessment with unknown status.
    pub fn new(instance_id: InstanceId) -> Self {
        Self {
            instance_id,
            overall: OverallHealth::Unknown,
            overall_score: 0.0,
            presence: DimensionAssessment::unknown(),
            coupling: DimensionAssessment::unknown(),
            attention: DimensionAssessment::unknown(),
            assessed_at: Utc::now(),
            is_isolated: false,
            recovery_attempts: 0,
            notes: Vec::new(),
        }
    }

    /// Recompute overall health from individual dimensions.
    pub fn recompute_overall(&mut self, thresholds: &HealthThresholds) {
        self.overall_score = thresholds.calculate_overall_score(
            self.presence.value,
            self.coupling.value,
            self.attention.value,
        );

        self.overall = if self.presence.status == DimensionHealth::Unknown
            || self.coupling.status == DimensionHealth::Unknown
            || self.attention.status == DimensionHealth::Unknown
        {
            OverallHealth::Unknown
        } else if thresholds.is_healthy(
            self.presence.value,
            self.coupling.value,
            self.attention.value,
        ) {
            OverallHealth::Healthy
        } else if thresholds.is_degraded(
            self.presence.value,
            self.coupling.value,
            self.attention.value,
        ) {
            OverallHealth::Degraded
        } else {
            OverallHealth::Unhealthy
        };

        self.assessed_at = Utc::now();
    }

    /// Check if the instance needs recovery action.
    pub fn needs_recovery(&self) -> bool {
        matches!(self.overall, OverallHealth::Unhealthy)
            || self.presence.consecutive_failures >= 3
            || self.coupling.consecutive_failures >= 3
            || self.attention.consecutive_failures >= 3
    }

    /// Check if the instance should be isolated.
    pub fn should_isolate(&self) -> bool {
        // Isolate if critically unhealthy and declining
        self.overall == OverallHealth::Unhealthy
            && (self.presence.trend < -0.3
                || self.coupling.trend < -0.3
                || self.attention.trend < -0.3)
    }

    /// Add a note to the assessment.
    pub fn add_note(&mut self, note: impl Into<String>) {
        self.notes.push(note.into());
    }
}

/// Summary of health across a fleet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetHealthSummary {
    /// Total number of instances.
    pub total_instances: usize,

    /// Number of healthy instances.
    pub healthy_count: usize,

    /// Number of degraded instances.
    pub degraded_count: usize,

    /// Number of unhealthy instances.
    pub unhealthy_count: usize,

    /// Number of instances with unknown status.
    pub unknown_count: usize,

    /// Number of isolated instances.
    pub isolated_count: usize,

    /// Average health score across fleet.
    pub average_score: f64,

    /// Minimum health score in fleet.
    pub min_score: f64,

    /// Maximum health score in fleet.
    pub max_score: f64,

    /// Time of this summary.
    pub summarized_at: DateTime<Utc>,
}

impl FleetHealthSummary {
    /// Create summary from a collection of assessments.
    pub fn from_assessments(assessments: &[HealthAssessment]) -> Self {
        let total_instances = assessments.len();

        if total_instances == 0 {
            return Self {
                total_instances: 0,
                healthy_count: 0,
                degraded_count: 0,
                unhealthy_count: 0,
                unknown_count: 0,
                isolated_count: 0,
                average_score: 0.0,
                min_score: 0.0,
                max_score: 0.0,
                summarized_at: Utc::now(),
            };
        }

        let mut healthy_count = 0;
        let mut degraded_count = 0;
        let mut unhealthy_count = 0;
        let mut unknown_count = 0;
        let mut isolated_count = 0;
        let mut total_score = 0.0;
        let mut min_score = f64::MAX;
        let mut max_score = f64::MIN;

        for assessment in assessments {
            match assessment.overall {
                OverallHealth::Healthy => healthy_count += 1,
                OverallHealth::Degraded => degraded_count += 1,
                OverallHealth::Unhealthy => unhealthy_count += 1,
                OverallHealth::Unknown => unknown_count += 1,
            }

            if assessment.is_isolated {
                isolated_count += 1;
            }

            total_score += assessment.overall_score;
            min_score = min_score.min(assessment.overall_score);
            max_score = max_score.max(assessment.overall_score);
        }

        Self {
            total_instances,
            healthy_count,
            degraded_count,
            unhealthy_count,
            unknown_count,
            isolated_count,
            average_score: total_score / total_instances as f64,
            min_score,
            max_score,
            summarized_at: Utc::now(),
        }
    }

    /// Calculate the percentage of healthy instances.
    pub fn healthy_percentage(&self) -> f64 {
        if self.total_instances == 0 {
            return 0.0;
        }
        (self.healthy_count as f64 / self.total_instances as f64) * 100.0
    }

    /// Check if the fleet is healthy (majority healthy, none unhealthy).
    pub fn is_fleet_healthy(&self) -> bool {
        self.unhealthy_count == 0 && self.healthy_percentage() >= 80.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use palm_types::InstanceId;

    #[test]
    fn test_dimension_assessment_update() {
        let mut assessment = DimensionAssessment::unknown();

        // Update to healthy
        assessment.update(0.9, 0.8, 0.5);
        assert_eq!(assessment.status, DimensionHealth::Healthy);
        assert_eq!(assessment.consecutive_successes, 1);
        assert_eq!(assessment.consecutive_failures, 0);

        // Update to degraded
        assessment.update(0.6, 0.8, 0.5);
        assert_eq!(assessment.status, DimensionHealth::Degraded);

        // Update to unhealthy
        assessment.update(0.3, 0.8, 0.5);
        assert_eq!(assessment.status, DimensionHealth::Unhealthy);
        assert_eq!(assessment.consecutive_failures, 1);
    }

    #[test]
    fn test_health_assessment_recompute() {
        let instance_id = InstanceId::generate();
        let mut assessment = HealthAssessment::new(instance_id);
        let thresholds = HealthThresholds::default();

        // Set all dimensions to healthy values
        assessment.presence.update(0.9, 0.8, 0.5);
        assessment.coupling.update(0.8, 0.7, 0.4);
        assessment.attention.update(0.7, 0.6, 0.3);

        assessment.recompute_overall(&thresholds);

        assert_eq!(assessment.overall, OverallHealth::Healthy);
        assert!(assessment.overall_score > 0.7);
    }

    #[test]
    fn test_fleet_health_summary() {
        let assessments = vec![
            {
                let mut a = HealthAssessment::new(InstanceId::generate());
                a.overall = OverallHealth::Healthy;
                a.overall_score = 0.9;
                a
            },
            {
                let mut a = HealthAssessment::new(InstanceId::generate());
                a.overall = OverallHealth::Healthy;
                a.overall_score = 0.85;
                a
            },
            {
                let mut a = HealthAssessment::new(InstanceId::generate());
                a.overall = OverallHealth::Degraded;
                a.overall_score = 0.6;
                a
            },
        ];

        let summary = FleetHealthSummary::from_assessments(&assessments);

        assert_eq!(summary.total_instances, 3);
        assert_eq!(summary.healthy_count, 2);
        assert_eq!(summary.degraded_count, 1);
        assert_eq!(summary.unhealthy_count, 0);
        assert!(summary.healthy_percentage() > 60.0);
    }
}
