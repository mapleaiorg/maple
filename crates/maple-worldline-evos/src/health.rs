//! Health monitoring for all EVOS subsystems.
//!
//! Provides health checking across all 14 WorldLine subsystems,
//! with aggregation into an overall health report.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::EvosResult;
use crate::types::{SubsystemId, SubsystemStatus};

// ── Subsystem Health ────────────────────────────────────────────────

/// Health status of a single subsystem.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubsystemHealth {
    /// Which subsystem.
    pub subsystem: SubsystemId,
    /// Current status.
    pub status: SubsystemStatus,
    /// When this health check was performed.
    pub last_check: chrono::DateTime<chrono::Utc>,
    /// Human-readable message.
    pub message: String,
}

impl std::fmt::Display for SubsystemHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.status, self.subsystem, self.message)
    }
}

// ── Health Report ───────────────────────────────────────────────────

/// Aggregated health report across all subsystems.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthReport {
    /// Individual subsystem health entries.
    pub entries: Vec<SubsystemHealth>,
    /// When this report was generated.
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

impl HealthReport {
    /// Overall status — worst status across all subsystems.
    pub fn overall_status(&self) -> SubsystemStatus {
        self.entries
            .iter()
            .max_by_key(|e| e.status.severity())
            .map(|e| e.status.clone())
            .unwrap_or(SubsystemStatus::Unknown)
    }

    /// How many subsystems are healthy.
    pub fn healthy_count(&self) -> usize {
        self.entries.iter().filter(|e| e.status.is_healthy()).count()
    }

    /// How many subsystems are degraded.
    pub fn degraded_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.status, SubsystemStatus::Degraded(_)))
            .count()
    }

    /// How many subsystems have failed.
    pub fn failed_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.status, SubsystemStatus::Failed(_)))
            .count()
    }

    /// Whether all subsystems allow cycle progression.
    pub fn all_allow_progression(&self) -> bool {
        self.entries.iter().all(|e| e.status.allows_progression())
    }

    /// Subsystems that are blocking cycle progression.
    pub fn blocking_subsystems(&self) -> Vec<&SubsystemId> {
        self.entries
            .iter()
            .filter(|e| !e.status.allows_progression())
            .map(|e| &e.subsystem)
            .collect()
    }
}

impl std::fmt::Display for HealthReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HealthReport(total={}, healthy={}, degraded={}, failed={})",
            self.entries.len(),
            self.healthy_count(),
            self.degraded_count(),
            self.failed_count(),
        )
    }
}

// ── Health Checker Trait ─────────────────────────────────────────────

/// Trait for checking the health of all subsystems.
pub trait HealthChecker: Send + Sync {
    /// Check health of all subsystems.
    fn check_all(&self) -> EvosResult<HealthReport>;

    /// Check health of a specific subsystem.
    fn check_one(&self, subsystem: &SubsystemId) -> EvosResult<SubsystemHealth>;

    /// Name of this checker.
    fn name(&self) -> &str;
}

/// Simulated health checker for deterministic testing.
pub struct SimulatedHealthChecker {
    /// Override statuses for specific subsystems.
    overrides: Vec<(SubsystemId, SubsystemStatus)>,
}

impl SimulatedHealthChecker {
    /// Create a checker where all subsystems are healthy.
    pub fn all_healthy() -> Self {
        Self {
            overrides: Vec::new(),
        }
    }

    /// Create a checker with specific subsystem overrides.
    pub fn with_overrides(overrides: Vec<(SubsystemId, SubsystemStatus)>) -> Self {
        Self { overrides }
    }

    fn status_for(&self, subsystem: &SubsystemId) -> SubsystemStatus {
        self.overrides
            .iter()
            .find(|(s, _)| s == subsystem)
            .map(|(_, status)| status.clone())
            .unwrap_or(SubsystemStatus::Healthy)
    }
}

impl Default for SimulatedHealthChecker {
    fn default() -> Self {
        Self::all_healthy()
    }
}

impl HealthChecker for SimulatedHealthChecker {
    fn check_all(&self) -> EvosResult<HealthReport> {
        let now = Utc::now();
        let entries = SubsystemId::all()
            .iter()
            .map(|s| {
                let status = self.status_for(s);
                let message = match &status {
                    SubsystemStatus::Healthy => "operating normally".into(),
                    SubsystemStatus::Degraded(msg) => msg.clone(),
                    SubsystemStatus::Failed(msg) => msg.clone(),
                    SubsystemStatus::Unknown => "status unknown".into(),
                };
                SubsystemHealth {
                    subsystem: s.clone(),
                    status,
                    last_check: now,
                    message,
                }
            })
            .collect();

        Ok(HealthReport {
            entries,
            generated_at: now,
        })
    }

    fn check_one(&self, subsystem: &SubsystemId) -> EvosResult<SubsystemHealth> {
        let status = self.status_for(subsystem);
        let message = match &status {
            SubsystemStatus::Healthy => "operating normally".into(),
            SubsystemStatus::Degraded(msg) => msg.clone(),
            SubsystemStatus::Failed(msg) => msg.clone(),
            SubsystemStatus::Unknown => "status unknown".into(),
        };
        Ok(SubsystemHealth {
            subsystem: subsystem.clone(),
            status,
            last_check: Utc::now(),
            message,
        })
    }

    fn name(&self) -> &str {
        "simulated-health-checker"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_healthy_report() {
        let checker = SimulatedHealthChecker::all_healthy();
        let report = checker.check_all().unwrap();
        assert_eq!(report.entries.len(), 14);
        assert_eq!(report.healthy_count(), 14);
        assert_eq!(report.degraded_count(), 0);
        assert_eq!(report.failed_count(), 0);
        assert!(report.overall_status().is_healthy());
        assert!(report.all_allow_progression());
    }

    #[test]
    fn degraded_subsystem() {
        let checker = SimulatedHealthChecker::with_overrides(vec![(
            SubsystemId::Meaning,
            SubsystemStatus::Degraded("high latency".into()),
        )]);
        let report = checker.check_all().unwrap();
        assert_eq!(report.healthy_count(), 13);
        assert_eq!(report.degraded_count(), 1);
        assert!(report.all_allow_progression()); // Degraded still allows
    }

    #[test]
    fn failed_subsystem_blocks() {
        let checker = SimulatedHealthChecker::with_overrides(vec![(
            SubsystemId::Compiler,
            SubsystemStatus::Failed("out of memory".into()),
        )]);
        let report = checker.check_all().unwrap();
        assert!(!report.all_allow_progression());
        let blocking = report.blocking_subsystems();
        assert_eq!(blocking.len(), 1);
        assert_eq!(*blocking[0], SubsystemId::Compiler);
    }

    #[test]
    fn overall_status_worst() {
        let checker = SimulatedHealthChecker::with_overrides(vec![
            (SubsystemId::Sal, SubsystemStatus::Degraded("slow".into())),
            (
                SubsystemId::Hardware,
                SubsystemStatus::Failed("offline".into()),
            ),
        ]);
        let report = checker.check_all().unwrap();
        assert!(matches!(report.overall_status(), SubsystemStatus::Failed(_)));
    }

    #[test]
    fn check_one_subsystem() {
        let checker = SimulatedHealthChecker::with_overrides(vec![(
            SubsystemId::Intent,
            SubsystemStatus::Degraded("backpressure".into()),
        )]);
        let health = checker.check_one(&SubsystemId::Intent).unwrap();
        assert!(matches!(health.status, SubsystemStatus::Degraded(_)));

        let health = checker.check_one(&SubsystemId::Observation).unwrap();
        assert!(health.status.is_healthy());
    }

    #[test]
    fn health_report_display() {
        let checker = SimulatedHealthChecker::all_healthy();
        let report = checker.check_all().unwrap();
        let display = report.to_string();
        assert!(display.contains("total=14"));
        assert!(display.contains("healthy=14"));
    }

    #[test]
    fn subsystem_health_display() {
        let health = SubsystemHealth {
            subsystem: SubsystemId::Observation,
            status: SubsystemStatus::Healthy,
            last_check: Utc::now(),
            message: "operating normally".into(),
        };
        let display = health.to_string();
        assert!(display.contains("healthy"));
        assert!(display.contains("observation"));
    }

    #[test]
    fn checker_name() {
        let checker = SimulatedHealthChecker::all_healthy();
        assert_eq!(checker.name(), "simulated-health-checker");
    }

    #[test]
    fn multiple_failures() {
        let checker = SimulatedHealthChecker::with_overrides(vec![
            (
                SubsystemId::Observation,
                SubsystemStatus::Failed("crash".into()),
            ),
            (
                SubsystemId::Meaning,
                SubsystemStatus::Failed("timeout".into()),
            ),
            (SubsystemId::Ir, SubsystemStatus::Unknown),
        ]);
        let report = checker.check_all().unwrap();
        assert_eq!(report.failed_count(), 2);
        assert_eq!(report.healthy_count(), 11);
        assert!(!report.all_allow_progression());
        assert_eq!(report.blocking_subsystems().len(), 3); // 2 failed + 1 unknown
    }

    #[test]
    fn empty_overrides_all_healthy() {
        let checker = SimulatedHealthChecker::with_overrides(vec![]);
        let report = checker.check_all().unwrap();
        assert_eq!(report.healthy_count(), 14);
    }
}
