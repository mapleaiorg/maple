//! Core types for the deployment pipeline.
//!
//! Defines deployment identifiers, phases, health snapshots, deployment records
//! with full lifecycle management, configuration, and summary statistics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use maple_worldline_self_mod_gate::ledger::{DeploymentStatus, PerformanceDelta};
use maple_worldline_self_mod_gate::types::{DeploymentStrategy, SelfModTier};

// ── Identifier ─────────────────────────────────────────────────────────

/// Unique identifier for a deployment session.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeploymentId(pub String);

impl DeploymentId {
    /// Generate a new unique deployment ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for DeploymentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DeploymentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "deploy:{}", self.0)
    }
}

// ── Deployment Phase ───────────────────────────────────────────────────

/// Current phase of a deployment.
///
/// Deployments progress through multiple phases depending on strategy:
/// Validating → Deploying → Monitoring → Promoting → Complete
/// On failure: → RollingBack → RolledBack
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DeploymentPhase {
    /// Validating the artifact before deployment.
    Validating,
    /// Deploying files at a specific traffic fraction.
    Deploying {
        /// Fraction of traffic receiving the deployment (0.0–1.0).
        traffic_fraction: f64,
    },
    /// Monitoring health after deployment at a traffic fraction.
    Monitoring {
        /// Current traffic fraction being observed.
        traffic_fraction: f64,
        /// Seconds elapsed in this monitoring window.
        elapsed_secs: u64,
    },
    /// Promoting from one traffic fraction to another.
    Promoting {
        /// Previous traffic fraction.
        from: f64,
        /// Target traffic fraction.
        to: f64,
    },
    /// Deployment completed successfully.
    Complete,
    /// Rolling back a failed deployment.
    RollingBack,
    /// Rollback completed.
    RolledBack,
}

impl std::fmt::Display for DeploymentPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validating => write!(f, "validating"),
            Self::Deploying { traffic_fraction } => {
                write!(f, "deploying({:.0}%)", traffic_fraction * 100.0)
            }
            Self::Monitoring {
                traffic_fraction,
                elapsed_secs,
            } => write!(
                f,
                "monitoring({:.0}%, {}s)",
                traffic_fraction * 100.0,
                elapsed_secs
            ),
            Self::Promoting { from, to } => {
                write!(f, "promoting({:.0}%→{:.0}%)", from * 100.0, to * 100.0)
            }
            Self::Complete => write!(f, "complete"),
            Self::RollingBack => write!(f, "rolling-back"),
            Self::RolledBack => write!(f, "rolled-back"),
        }
    }
}

// ── Health Snapshot ────────────────────────────────────────────────────

/// A single health measurement taken during deployment monitoring.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthSnapshot {
    /// Name of the metric being measured.
    pub metric: String,
    /// Current measured value.
    pub value: f64,
    /// Baseline value (pre-deployment).
    pub baseline: f64,
    /// Whether this metric is considered healthy.
    pub healthy: bool,
    /// When this measurement was taken.
    pub measured_at: DateTime<Utc>,
}

impl HealthSnapshot {
    /// Absolute delta from baseline (value - baseline).
    pub fn delta(&self) -> f64 {
        self.value - self.baseline
    }

    /// Regression percentage relative to baseline.
    /// Positive means regression (value exceeded baseline).
    /// For metrics where lower is better (e.g. latency), positive delta = regression.
    pub fn regression_pct(&self) -> f64 {
        if self.baseline == 0.0 {
            return 0.0;
        }
        ((self.value - self.baseline) / self.baseline) * 100.0
    }
}

// ── Deployment Record ──────────────────────────────────────────────────

/// A complete record of a deployment attempt.
///
/// Tracks the full lifecycle from artifact acceptance through strategy
/// execution, health monitoring, and final outcome (success/failure/rollback).
#[derive(Clone, Debug)]
pub struct DeploymentRecord {
    /// Unique deployment ID.
    pub id: DeploymentId,
    /// Codegen session ID that produced the artifact.
    pub codegen_id: String,
    /// Commitment ID this deployment fulfills.
    pub commitment_id: String,
    /// Self-modification tier.
    pub tier: SelfModTier,
    /// Deployment strategy used.
    pub strategy: DeploymentStrategy,
    /// Current deployment status.
    pub status: DeploymentStatus,
    /// Current phase.
    pub current_phase: DeploymentPhase,
    /// Phases completed so far.
    pub phases_completed: Vec<DeploymentPhase>,
    /// Health snapshots collected during monitoring.
    pub health_snapshots: Vec<HealthSnapshot>,
    /// Performance deltas (before/after).
    pub performance_deltas: Vec<PerformanceDelta>,
    /// Files deployed.
    pub files_deployed: Vec<String>,
    /// Whether rollback was triggered.
    pub rollback_triggered: bool,
    /// Rollback reason (if any).
    pub rollback_reason: Option<String>,
    /// When the deployment started.
    pub started_at: DateTime<Utc>,
    /// When the deployment completed (success/failure/rollback).
    pub completed_at: Option<DateTime<Utc>>,
}

impl DeploymentRecord {
    /// Create a new deployment record.
    pub fn new(
        codegen_id: String,
        commitment_id: String,
        tier: SelfModTier,
        strategy: DeploymentStrategy,
        files: Vec<String>,
    ) -> Self {
        Self {
            id: DeploymentId::new(),
            codegen_id,
            commitment_id,
            tier,
            strategy,
            status: DeploymentStatus::Pending,
            current_phase: DeploymentPhase::Validating,
            phases_completed: vec![],
            health_snapshots: vec![],
            performance_deltas: vec![],
            files_deployed: files,
            rollback_triggered: false,
            rollback_reason: None,
            started_at: Utc::now(),
            completed_at: None,
        }
    }

    /// Transition to in-progress status.
    pub fn mark_in_progress(&mut self) {
        self.status = DeploymentStatus::InProgress;
    }

    /// Advance to the next phase.
    pub fn advance_phase(&mut self, next: DeploymentPhase) {
        let prev = std::mem::replace(&mut self.current_phase, next);
        self.phases_completed.push(prev);
    }

    /// Mark deployment as succeeded.
    pub fn mark_succeeded(&mut self) {
        self.status = DeploymentStatus::Succeeded;
        self.advance_phase(DeploymentPhase::Complete);
        self.completed_at = Some(Utc::now());
    }

    /// Mark deployment as failed.
    pub fn mark_failed(&mut self, reason: String) {
        self.status = DeploymentStatus::Failed(reason);
        self.completed_at = Some(Utc::now());
    }

    /// Mark deployment as rolled back.
    pub fn mark_rolled_back(&mut self, reason: String) {
        self.rollback_triggered = true;
        self.rollback_reason = Some(reason.clone());
        self.status = DeploymentStatus::RolledBack(reason);
        self.advance_phase(DeploymentPhase::RolledBack);
        self.completed_at = Some(Utc::now());
    }

    /// Add a health snapshot.
    pub fn add_health_snapshot(&mut self, snapshot: HealthSnapshot) {
        self.health_snapshots.push(snapshot);
    }

    /// Add a performance delta.
    pub fn add_performance_delta(&mut self, delta: PerformanceDelta) {
        self.performance_deltas.push(delta);
    }

    /// Total duration in milliseconds (if completed).
    pub fn duration_ms(&self) -> Option<i64> {
        self.completed_at
            .map(|end| (end - self.started_at).num_milliseconds())
    }

    /// Number of phases completed.
    pub fn phase_count(&self) -> usize {
        self.phases_completed.len()
    }
}

// ── Configuration ──────────────────────────────────────────────────────

/// Configuration for the deployment pipeline.
#[derive(Clone, Debug)]
pub struct DeploymentConfig {
    /// Maximum deployment duration (seconds) before timeout.
    pub max_deployment_timeout_secs: u64,
    /// Health check interval during monitoring (seconds).
    pub health_check_interval_secs: u64,
    /// Maximum acceptable regression percentage before rollback.
    pub max_regression_pct: f64,
    /// Whether to automatically rollback on health failures.
    pub auto_rollback: bool,
    /// Traffic fractions for staged rollout strategy.
    pub staged_fractions: Vec<f64>,
    /// Maximum tracked deployment records (bounded FIFO).
    pub max_tracked_records: usize,
}

impl Default for DeploymentConfig {
    fn default() -> Self {
        Self {
            max_deployment_timeout_secs: 3600,
            health_check_interval_secs: 30,
            max_regression_pct: 5.0,
            auto_rollback: true,
            staged_fractions: vec![0.10, 0.25, 0.50, 1.0],
            max_tracked_records: 256,
        }
    }
}

// ── Summary ────────────────────────────────────────────────────────────

/// Summary statistics for the deployment pipeline.
#[derive(Clone, Debug, Default)]
pub struct DeploymentSummary {
    /// Total deployments.
    pub total: usize,
    /// Pending deployments.
    pub pending: usize,
    /// In-progress deployments.
    pub in_progress: usize,
    /// Succeeded deployments.
    pub succeeded: usize,
    /// Failed deployments.
    pub failed: usize,
    /// Rolled-back deployments.
    pub rolled_back: usize,
    /// Total files deployed across all sessions.
    pub total_files_deployed: usize,
}

impl std::fmt::Display for DeploymentSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeploymentSummary(total={}, succeeded={}, failed={}, rolled_back={}, files={})",
            self.total,
            self.succeeded,
            self.failed,
            self.rolled_back,
            self.total_files_deployed,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deployment_id_uniqueness() {
        let a = DeploymentId::new();
        let b = DeploymentId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn deployment_id_display_format() {
        let id = DeploymentId::new();
        assert!(id.to_string().starts_with("deploy:"));
    }

    #[test]
    fn deployment_phase_display() {
        assert_eq!(DeploymentPhase::Validating.to_string(), "validating");
        assert_eq!(
            DeploymentPhase::Deploying { traffic_fraction: 0.1 }.to_string(),
            "deploying(10%)"
        );
        assert_eq!(
            DeploymentPhase::Monitoring {
                traffic_fraction: 0.25,
                elapsed_secs: 60
            }
            .to_string(),
            "monitoring(25%, 60s)"
        );
        assert_eq!(
            DeploymentPhase::Promoting { from: 0.25, to: 0.50 }.to_string(),
            "promoting(25%→50%)"
        );
        assert_eq!(DeploymentPhase::Complete.to_string(), "complete");
        assert_eq!(DeploymentPhase::RollingBack.to_string(), "rolling-back");
        assert_eq!(DeploymentPhase::RolledBack.to_string(), "rolled-back");
    }

    #[test]
    fn health_snapshot_delta() {
        let snap = HealthSnapshot {
            metric: "latency_p99".into(),
            value: 12.0,
            baseline: 10.0,
            healthy: false,
            measured_at: Utc::now(),
        };
        assert!((snap.delta() - 2.0).abs() < 0.01);
        assert!((snap.regression_pct() - 20.0).abs() < 0.01);
    }

    #[test]
    fn health_snapshot_zero_baseline() {
        let snap = HealthSnapshot {
            metric: "errors".into(),
            value: 5.0,
            baseline: 0.0,
            healthy: false,
            measured_at: Utc::now(),
        };
        assert_eq!(snap.regression_pct(), 0.0);
    }

    #[test]
    fn deployment_record_lifecycle() {
        let mut record = DeploymentRecord::new(
            "codegen-1".into(),
            "commit-1".into(),
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
            vec!["src/config.rs".into()],
        );

        assert!(matches!(record.status, DeploymentStatus::Pending));
        assert!(matches!(record.current_phase, DeploymentPhase::Validating));
        assert!(record.completed_at.is_none());

        record.mark_in_progress();
        assert!(matches!(record.status, DeploymentStatus::InProgress));

        record.advance_phase(DeploymentPhase::Deploying { traffic_fraction: 1.0 });
        assert_eq!(record.phase_count(), 1);

        record.mark_succeeded();
        assert!(matches!(record.status, DeploymentStatus::Succeeded));
        assert!(record.completed_at.is_some());
        assert!(record.duration_ms().is_some());
    }

    #[test]
    fn deployment_record_failure() {
        let mut record = DeploymentRecord::new(
            "codegen-2".into(),
            "commit-2".into(),
            SelfModTier::Tier1OperatorInternal,
            DeploymentStrategy::Canary { traffic_fraction: 0.05 },
            vec!["src/handler.rs".into()],
        );

        record.mark_in_progress();
        record.mark_failed("compilation error".into());
        assert!(matches!(record.status, DeploymentStatus::Failed(_)));
        assert!(record.completed_at.is_some());
    }

    #[test]
    fn deployment_record_rollback() {
        let mut record = DeploymentRecord::new(
            "codegen-3".into(),
            "commit-3".into(),
            SelfModTier::Tier2ApiChange,
            DeploymentStrategy::Staged,
            vec!["src/api.rs".into()],
        );

        record.mark_in_progress();
        record.advance_phase(DeploymentPhase::Deploying { traffic_fraction: 0.1 });
        record.advance_phase(DeploymentPhase::RollingBack);
        record.mark_rolled_back("regression detected".into());

        assert!(record.rollback_triggered);
        assert_eq!(record.rollback_reason.as_deref(), Some("regression detected"));
        assert!(matches!(record.status, DeploymentStatus::RolledBack(_)));
    }

    #[test]
    fn deployment_record_health_snapshots() {
        let mut record = DeploymentRecord::new(
            "codegen-4".into(),
            "commit-4".into(),
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
            vec!["src/config.rs".into()],
        );

        record.add_health_snapshot(HealthSnapshot {
            metric: "latency".into(),
            value: 9.0,
            baseline: 10.0,
            healthy: true,
            measured_at: Utc::now(),
        });
        record.add_performance_delta(PerformanceDelta {
            metric: "latency".into(),
            before: 10.0,
            after: 9.0,
            unit: "ms".into(),
        });

        assert_eq!(record.health_snapshots.len(), 1);
        assert_eq!(record.performance_deltas.len(), 1);
    }

    #[test]
    fn config_defaults() {
        let cfg = DeploymentConfig::default();
        assert_eq!(cfg.max_deployment_timeout_secs, 3600);
        assert_eq!(cfg.health_check_interval_secs, 30);
        assert!((cfg.max_regression_pct - 5.0).abs() < 0.01);
        assert!(cfg.auto_rollback);
        assert_eq!(cfg.staged_fractions, vec![0.10, 0.25, 0.50, 1.0]);
        assert_eq!(cfg.max_tracked_records, 256);
    }

    #[test]
    fn summary_display() {
        let s = DeploymentSummary {
            total: 10,
            pending: 0,
            in_progress: 1,
            succeeded: 7,
            failed: 1,
            rolled_back: 1,
            total_files_deployed: 35,
        };
        let display = s.to_string();
        assert!(display.contains("total=10"));
        assert!(display.contains("succeeded=7"));
        assert!(display.contains("rolled_back=1"));
    }

    #[test]
    fn summary_default() {
        let s = DeploymentSummary::default();
        assert_eq!(s.total, 0);
        assert_eq!(s.succeeded, 0);
    }
}
