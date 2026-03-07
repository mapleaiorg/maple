//! MAPLE Fleet Rollout -- canary deployment and rollout strategies.
//!
//! Supports canary, blue-green, rolling, and immediate rollout strategies
//! with metric-based thresholds and auto-rollback.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum RolloutError {
    #[error("rollout not found: {0}")]
    NotFound(String),
    #[error("invalid rollout state: cannot {operation} in state {state:?}")]
    InvalidState { operation: String, state: RolloutStatus },
    #[error("threshold exceeded: {metric} = {value}, threshold = {threshold}")]
    ThresholdExceeded { metric: String, value: f64, threshold: f64 },
    #[error("rollout error: {0}")]
    Internal(String),
}

pub type RolloutResult<T> = Result<T, RolloutError>;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Strategy for how a rollout proceeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RolloutStrategy {
    /// Gradually shift traffic to the new version.
    Canary,
    /// Deploy new version alongside old, then switch.
    BlueGreen,
    /// Replace instances in rolling batches.
    Rolling,
    /// Replace everything at once.
    Immediate,
}

/// Status of a rollout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RolloutStatus {
    Pending,
    InProgress,
    Paused,
    Completed,
    RolledBack,
}

/// A single step in a rollout plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolloutStep {
    pub name: String,
    pub traffic_percentage: u8,
    pub duration_secs: u64,
    pub validation_required: bool,
}

/// A threshold that determines success or failure of a rollout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricThreshold {
    pub metric_name: String,
    pub max_value: Option<f64>,
    pub min_value: Option<f64>,
}

impl MetricThreshold {
    /// Check if a metric value satisfies this threshold.
    pub fn check(&self, value: f64) -> bool {
        if let Some(max) = self.max_value {
            if value > max {
                return false;
            }
        }
        if let Some(min) = self.min_value {
            if value < min {
                return false;
            }
        }
        true
    }
}

/// Traffic split configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficSplit {
    pub old_version_weight: u8,
    pub new_version_weight: u8,
}

impl TrafficSplit {
    pub fn new(old: u8, new: u8) -> Self {
        Self {
            old_version_weight: old,
            new_version_weight: new,
        }
    }

    /// Returns the total weight.
    pub fn total(&self) -> u16 {
        self.old_version_weight as u16 + self.new_version_weight as u16
    }

    /// Returns new version percentage (0-100).
    pub fn new_version_percentage(&self) -> f64 {
        if self.total() == 0 {
            return 0.0;
        }
        (self.new_version_weight as f64 / self.total() as f64) * 100.0
    }
}

/// A complete rollout plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolloutPlan {
    pub id: String,
    pub name: String,
    pub strategy: RolloutStrategy,
    pub old_version: String,
    pub new_version: String,
    pub steps: Vec<RolloutStep>,
    pub thresholds: Vec<MetricThreshold>,
    pub auto_rollback: bool,
    pub traffic_split: TrafficSplit,
    pub status: RolloutStatus,
    pub current_step: usize,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub metrics: HashMap<String, f64>,
}

impl RolloutPlan {
    pub fn new(
        name: impl Into<String>,
        strategy: RolloutStrategy,
        old_version: impl Into<String>,
        new_version: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            strategy,
            old_version: old_version.into(),
            new_version: new_version.into(),
            steps: Vec::new(),
            thresholds: Vec::new(),
            auto_rollback: true,
            traffic_split: TrafficSplit::new(100, 0),
            status: RolloutStatus::Pending,
            current_step: 0,
            created_at: Utc::now(),
            completed_at: None,
            metrics: HashMap::new(),
        }
    }

    /// Add a step to the rollout plan.
    pub fn add_step(&mut self, step: RolloutStep) {
        self.steps.push(step);
    }

    /// Add a metric threshold.
    pub fn add_threshold(&mut self, threshold: MetricThreshold) {
        self.thresholds.push(threshold);
    }
}

// ---------------------------------------------------------------------------
// Rollout Manager
// ---------------------------------------------------------------------------

/// Manages rollout plans and their execution.
pub struct RolloutManager {
    rollouts: HashMap<String, RolloutPlan>,
}

impl Default for RolloutManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RolloutManager {
    pub fn new() -> Self {
        Self {
            rollouts: HashMap::new(),
        }
    }

    /// Register a new rollout plan.
    pub fn register(&mut self, plan: RolloutPlan) -> RolloutResult<()> {
        self.rollouts.insert(plan.id.clone(), plan);
        Ok(())
    }

    /// Start a rollout.
    pub fn start(&mut self, id: &str) -> RolloutResult<&RolloutPlan> {
        let plan = self
            .rollouts
            .get_mut(id)
            .ok_or_else(|| RolloutError::NotFound(id.to_string()))?;
        if plan.status != RolloutStatus::Pending {
            return Err(RolloutError::InvalidState {
                operation: "start".into(),
                state: plan.status,
            });
        }
        plan.status = RolloutStatus::InProgress;
        plan.current_step = 0;
        if !plan.steps.is_empty() {
            plan.traffic_split.new_version_weight = plan.steps[0].traffic_percentage;
            plan.traffic_split.old_version_weight = 100 - plan.steps[0].traffic_percentage;
        }
        Ok(plan)
    }

    /// Advance to the next step.
    pub fn advance(&mut self, id: &str) -> RolloutResult<&RolloutPlan> {
        let plan = self
            .rollouts
            .get_mut(id)
            .ok_or_else(|| RolloutError::NotFound(id.to_string()))?;
        if plan.status != RolloutStatus::InProgress {
            return Err(RolloutError::InvalidState {
                operation: "advance".into(),
                state: plan.status,
            });
        }

        // Check thresholds before advancing
        for threshold in &plan.thresholds {
            if let Some(&value) = plan.metrics.get(&threshold.metric_name) {
                if !threshold.check(value) && plan.auto_rollback {
                    plan.status = RolloutStatus::RolledBack;
                    plan.traffic_split = TrafficSplit::new(100, 0);
                    plan.completed_at = Some(Utc::now());
                    return Err(RolloutError::ThresholdExceeded {
                        metric: threshold.metric_name.clone(),
                        value,
                        threshold: threshold.max_value.unwrap_or(threshold.min_value.unwrap_or(0.0)),
                    });
                }
            }
        }

        plan.current_step += 1;
        if plan.current_step >= plan.steps.len() {
            plan.status = RolloutStatus::Completed;
            plan.traffic_split = TrafficSplit::new(0, 100);
            plan.completed_at = Some(Utc::now());
        } else {
            let step = &plan.steps[plan.current_step];
            plan.traffic_split.new_version_weight = step.traffic_percentage;
            plan.traffic_split.old_version_weight = 100 - step.traffic_percentage;
        }
        Ok(plan)
    }

    /// Pause a rollout.
    pub fn pause(&mut self, id: &str) -> RolloutResult<&RolloutPlan> {
        let plan = self
            .rollouts
            .get_mut(id)
            .ok_or_else(|| RolloutError::NotFound(id.to_string()))?;
        if plan.status != RolloutStatus::InProgress {
            return Err(RolloutError::InvalidState {
                operation: "pause".into(),
                state: plan.status,
            });
        }
        plan.status = RolloutStatus::Paused;
        Ok(plan)
    }

    /// Roll back a rollout.
    pub fn rollback(&mut self, id: &str) -> RolloutResult<&RolloutPlan> {
        let plan = self
            .rollouts
            .get_mut(id)
            .ok_or_else(|| RolloutError::NotFound(id.to_string()))?;
        plan.status = RolloutStatus::RolledBack;
        plan.traffic_split = TrafficSplit::new(100, 0);
        plan.completed_at = Some(Utc::now());
        Ok(plan)
    }

    /// Record a metric value for a rollout.
    pub fn record_metric(&mut self, id: &str, metric: &str, value: f64) -> RolloutResult<()> {
        let plan = self
            .rollouts
            .get_mut(id)
            .ok_or_else(|| RolloutError::NotFound(id.to_string()))?;
        plan.metrics.insert(metric.to_string(), value);
        Ok(())
    }

    /// Get a rollout by ID.
    pub fn get(&self, id: &str) -> RolloutResult<&RolloutPlan> {
        self.rollouts
            .get(id)
            .ok_or_else(|| RolloutError::NotFound(id.to_string()))
    }

    /// List all rollouts.
    pub fn list(&self) -> Vec<&RolloutPlan> {
        self.rollouts.values().collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_plan() -> RolloutPlan {
        let mut plan = RolloutPlan::new("deploy-v2", RolloutStrategy::Canary, "v1.0", "v2.0");
        plan.add_step(RolloutStep {
            name: "10% canary".into(),
            traffic_percentage: 10,
            duration_secs: 300,
            validation_required: true,
        });
        plan.add_step(RolloutStep {
            name: "50% canary".into(),
            traffic_percentage: 50,
            duration_secs: 600,
            validation_required: true,
        });
        plan.add_step(RolloutStep {
            name: "100% rollout".into(),
            traffic_percentage: 100,
            duration_secs: 0,
            validation_required: false,
        });
        plan
    }

    #[test]
    fn test_create_plan() {
        let plan = make_plan();
        assert_eq!(plan.status, RolloutStatus::Pending);
        assert_eq!(plan.steps.len(), 3);
    }

    #[test]
    fn test_start_rollout() {
        let mut mgr = RolloutManager::new();
        let plan = make_plan();
        let id = plan.id.clone();
        mgr.register(plan).unwrap();
        let started = mgr.start(&id).unwrap();
        assert_eq!(started.status, RolloutStatus::InProgress);
        assert_eq!(started.traffic_split.new_version_weight, 10);
    }

    #[test]
    fn test_advance_rollout() {
        let mut mgr = RolloutManager::new();
        let plan = make_plan();
        let id = plan.id.clone();
        mgr.register(plan).unwrap();
        mgr.start(&id).unwrap();
        let advanced = mgr.advance(&id).unwrap();
        assert_eq!(advanced.traffic_split.new_version_weight, 50);
    }

    #[test]
    fn test_complete_rollout() {
        let mut mgr = RolloutManager::new();
        let plan = make_plan();
        let id = plan.id.clone();
        mgr.register(plan).unwrap();
        mgr.start(&id).unwrap();
        mgr.advance(&id).unwrap();
        mgr.advance(&id).unwrap();
        let completed = mgr.advance(&id).unwrap();
        assert_eq!(completed.status, RolloutStatus::Completed);
        assert_eq!(completed.traffic_split.new_version_weight, 100);
    }

    #[test]
    fn test_pause_rollout() {
        let mut mgr = RolloutManager::new();
        let plan = make_plan();
        let id = plan.id.clone();
        mgr.register(plan).unwrap();
        mgr.start(&id).unwrap();
        let paused = mgr.pause(&id).unwrap();
        assert_eq!(paused.status, RolloutStatus::Paused);
    }

    #[test]
    fn test_rollback() {
        let mut mgr = RolloutManager::new();
        let plan = make_plan();
        let id = plan.id.clone();
        mgr.register(plan).unwrap();
        mgr.start(&id).unwrap();
        let rb = mgr.rollback(&id).unwrap();
        assert_eq!(rb.status, RolloutStatus::RolledBack);
        assert_eq!(rb.traffic_split.new_version_weight, 0);
    }

    #[test]
    fn test_metric_threshold_check() {
        let t = MetricThreshold {
            metric_name: "error_rate".into(),
            max_value: Some(0.05),
            min_value: None,
        };
        assert!(t.check(0.01));
        assert!(t.check(0.05));
        assert!(!t.check(0.06));
    }

    #[test]
    fn test_auto_rollback_on_threshold() {
        let mut mgr = RolloutManager::new();
        let mut plan = make_plan();
        plan.add_threshold(MetricThreshold {
            metric_name: "error_rate".into(),
            max_value: Some(0.05),
            min_value: None,
        });
        let id = plan.id.clone();
        mgr.register(plan).unwrap();
        mgr.start(&id).unwrap();
        mgr.record_metric(&id, "error_rate", 0.10).unwrap();
        let result = mgr.advance(&id);
        assert!(result.is_err());
        let plan = mgr.get(&id).unwrap();
        assert_eq!(plan.status, RolloutStatus::RolledBack);
    }

    #[test]
    fn test_traffic_split() {
        let split = TrafficSplit::new(70, 30);
        assert_eq!(split.total(), 100);
        assert!((split.new_version_percentage() - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_list_rollouts() {
        let mut mgr = RolloutManager::new();
        assert!(mgr.list().is_empty());
        mgr.register(make_plan()).unwrap();
        assert_eq!(mgr.list().len(), 1);
    }
}
