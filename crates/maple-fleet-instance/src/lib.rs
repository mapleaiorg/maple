//! MAPLE Fleet Instance Manager -- agent lifecycle engine.
//!
//! Manages the complete lifecycle of agent instances: creation, starting,
//! health monitoring, stopping, and termination.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors produced by the instance manager.
#[derive(Debug, Error)]
pub enum InstanceError {
    #[error("instance not found: {0}")]
    NotFound(InstanceId),
    #[error("invalid state transition from {from:?} to {to:?}")]
    InvalidTransition { from: InstanceState, to: InstanceState },
    #[error("resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
    #[error("health check failed for instance {0}: {1}")]
    HealthCheckFailed(InstanceId, String),
    #[error("internal error: {0}")]
    Internal(String),
}

pub type InstanceResult<T> = Result<T, InstanceError>;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Unique identifier for an agent instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InstanceId(pub String);

impl InstanceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for InstanceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for InstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Lifecycle state of an agent instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstanceState {
    Created,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
    Terminated,
}

impl InstanceState {
    /// Returns `true` if this state can transition to `next`.
    pub fn can_transition_to(self, next: InstanceState) -> bool {
        use InstanceState::*;
        matches!(
            (self, next),
            (Created, Starting)
                | (Starting, Running)
                | (Starting, Failed)
                | (Running, Stopping)
                | (Running, Failed)
                | (Stopping, Stopped)
                | (Stopping, Failed)
                | (Stopped, Terminated)
                | (Failed, Terminated)
                | (Created, Terminated)
                | (Stopped, Starting)
        )
    }
}

/// Resource limits applied to an instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory_mb: u64,
    pub cpu_cores: f64,
    pub max_tokens_per_min: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_mb: 512,
            cpu_cores: 1.0,
            max_tokens_per_min: 10_000,
        }
    }
}

/// Health check configuration for an instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Interval in seconds between health checks.
    pub interval_secs: u64,
    /// Number of consecutive failures before marking unhealthy.
    pub failure_threshold: u32,
    /// Number of consecutive successes before marking healthy.
    pub success_threshold: u32,
    /// Timeout in seconds for each health check probe.
    pub timeout_secs: u64,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            interval_secs: 30,
            failure_threshold: 3,
            success_threshold: 1,
            timeout_secs: 5,
        }
    }
}

/// Health status of an instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub last_check: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self {
            healthy: true,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_check: None,
            last_error: None,
        }
    }
}

/// Configuration for creating a new instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfig {
    pub agent_ref: String,
    pub environment: HashMap<String, String>,
    pub resources: ResourceLimits,
    pub health_check: HealthCheckConfig,
    pub labels: HashMap<String, String>,
}

impl InstanceConfig {
    pub fn new(agent_ref: impl Into<String>) -> Self {
        Self {
            agent_ref: agent_ref.into(),
            environment: HashMap::new(),
            resources: ResourceLimits::default(),
            health_check: HealthCheckConfig::default(),
            labels: HashMap::new(),
        }
    }
}

/// A running (or previously running) agent instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: InstanceId,
    pub config: InstanceConfig,
    pub state: InstanceState,
    pub health: HealthStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub stopped_at: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// Instance Manager
// ---------------------------------------------------------------------------

/// Manages the lifecycle of agent instances.
pub struct InstanceManager {
    instances: Arc<RwLock<HashMap<InstanceId, Instance>>>,
}

impl Default for InstanceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InstanceManager {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new instance in `Created` state.
    pub fn create(&self, config: InstanceConfig) -> InstanceResult<Instance> {
        let id = InstanceId::new();
        let instance = Instance {
            id: id.clone(),
            config,
            state: InstanceState::Created,
            health: HealthStatus::default(),
            created_at: Utc::now(),
            started_at: None,
            stopped_at: None,
        };
        let mut map = self.instances.write().map_err(|e| InstanceError::Internal(e.to_string()))?;
        map.insert(id, instance.clone());
        Ok(instance)
    }

    /// Transition an instance to `Starting` and then `Running`.
    pub fn start(&self, id: &InstanceId) -> InstanceResult<Instance> {
        let mut map = self.instances.write().map_err(|e| InstanceError::Internal(e.to_string()))?;
        let instance = map.get_mut(id).ok_or_else(|| InstanceError::NotFound(id.clone()))?;
        if !instance.state.can_transition_to(InstanceState::Starting) {
            return Err(InstanceError::InvalidTransition {
                from: instance.state,
                to: InstanceState::Starting,
            });
        }
        instance.state = InstanceState::Starting;
        // Simulate immediate start
        instance.state = InstanceState::Running;
        instance.started_at = Some(Utc::now());
        Ok(instance.clone())
    }

    /// Stop a running instance.
    pub fn stop(&self, id: &InstanceId) -> InstanceResult<Instance> {
        let mut map = self.instances.write().map_err(|e| InstanceError::Internal(e.to_string()))?;
        let instance = map.get_mut(id).ok_or_else(|| InstanceError::NotFound(id.clone()))?;
        if !instance.state.can_transition_to(InstanceState::Stopping) {
            return Err(InstanceError::InvalidTransition {
                from: instance.state,
                to: InstanceState::Stopping,
            });
        }
        instance.state = InstanceState::Stopping;
        instance.state = InstanceState::Stopped;
        instance.stopped_at = Some(Utc::now());
        Ok(instance.clone())
    }

    /// Terminate an instance (final state).
    pub fn terminate(&self, id: &InstanceId) -> InstanceResult<Instance> {
        let mut map = self.instances.write().map_err(|e| InstanceError::Internal(e.to_string()))?;
        let instance = map.get_mut(id).ok_or_else(|| InstanceError::NotFound(id.clone()))?;
        if !instance.state.can_transition_to(InstanceState::Terminated) {
            return Err(InstanceError::InvalidTransition {
                from: instance.state,
                to: InstanceState::Terminated,
            });
        }
        instance.state = InstanceState::Terminated;
        instance.stopped_at = Some(Utc::now());
        Ok(instance.clone())
    }

    /// Mark an instance as failed.
    pub fn mark_failed(&self, id: &InstanceId, reason: &str) -> InstanceResult<Instance> {
        let mut map = self.instances.write().map_err(|e| InstanceError::Internal(e.to_string()))?;
        let instance = map.get_mut(id).ok_or_else(|| InstanceError::NotFound(id.clone()))?;
        if !instance.state.can_transition_to(InstanceState::Failed) {
            return Err(InstanceError::InvalidTransition {
                from: instance.state,
                to: InstanceState::Failed,
            });
        }
        instance.state = InstanceState::Failed;
        instance.health.healthy = false;
        instance.health.last_error = Some(reason.to_string());
        Ok(instance.clone())
    }

    /// Record a health check result.
    pub fn record_health_check(&self, id: &InstanceId, healthy: bool, error: Option<String>) -> InstanceResult<HealthStatus> {
        let mut map = self.instances.write().map_err(|e| InstanceError::Internal(e.to_string()))?;
        let instance = map.get_mut(id).ok_or_else(|| InstanceError::NotFound(id.clone()))?;
        instance.health.last_check = Some(Utc::now());
        if healthy {
            instance.health.consecutive_successes += 1;
            instance.health.consecutive_failures = 0;
            if instance.health.consecutive_successes >= instance.config.health_check.success_threshold {
                instance.health.healthy = true;
            }
        } else {
            instance.health.consecutive_failures += 1;
            instance.health.consecutive_successes = 0;
            instance.health.last_error = error;
            if instance.health.consecutive_failures >= instance.config.health_check.failure_threshold {
                instance.health.healthy = false;
            }
        }
        Ok(instance.health.clone())
    }

    /// Get an instance by ID.
    pub fn get(&self, id: &InstanceId) -> InstanceResult<Instance> {
        let map = self.instances.read().map_err(|e| InstanceError::Internal(e.to_string()))?;
        map.get(id).cloned().ok_or_else(|| InstanceError::NotFound(id.clone()))
    }

    /// List all instances, optionally filtered by state.
    pub fn list(&self, state_filter: Option<InstanceState>) -> InstanceResult<Vec<Instance>> {
        let map = self.instances.read().map_err(|e| InstanceError::Internal(e.to_string()))?;
        let iter = map.values();
        let instances: Vec<Instance> = match state_filter {
            Some(state) => iter.filter(|i| i.state == state).cloned().collect(),
            None => iter.cloned().collect(),
        };
        Ok(instances)
    }

    /// Return the total number of instances.
    pub fn count(&self) -> InstanceResult<usize> {
        let map = self.instances.read().map_err(|e| InstanceError::Internal(e.to_string()))?;
        Ok(map.len())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manager() -> InstanceManager {
        InstanceManager::new()
    }

    #[test]
    fn test_create_instance() {
        let mgr = make_manager();
        let inst = mgr.create(InstanceConfig::new("agent/hello:1.0")).unwrap();
        assert_eq!(inst.state, InstanceState::Created);
        assert_eq!(inst.config.agent_ref, "agent/hello:1.0");
    }

    #[test]
    fn test_start_instance() {
        let mgr = make_manager();
        let inst = mgr.create(InstanceConfig::new("agent/hello:1.0")).unwrap();
        let running = mgr.start(&inst.id).unwrap();
        assert_eq!(running.state, InstanceState::Running);
        assert!(running.started_at.is_some());
    }

    #[test]
    fn test_stop_instance() {
        let mgr = make_manager();
        let inst = mgr.create(InstanceConfig::new("agent/hello:1.0")).unwrap();
        mgr.start(&inst.id).unwrap();
        let stopped = mgr.stop(&inst.id).unwrap();
        assert_eq!(stopped.state, InstanceState::Stopped);
        assert!(stopped.stopped_at.is_some());
    }

    #[test]
    fn test_terminate_instance() {
        let mgr = make_manager();
        let inst = mgr.create(InstanceConfig::new("agent/hello:1.0")).unwrap();
        mgr.start(&inst.id).unwrap();
        mgr.stop(&inst.id).unwrap();
        let terminated = mgr.terminate(&inst.id).unwrap();
        assert_eq!(terminated.state, InstanceState::Terminated);
    }

    #[test]
    fn test_invalid_transition() {
        let mgr = make_manager();
        let inst = mgr.create(InstanceConfig::new("agent/hello:1.0")).unwrap();
        // Cannot stop an instance that is not running
        let res = mgr.stop(&inst.id);
        assert!(res.is_err());
    }

    #[test]
    fn test_not_found() {
        let mgr = make_manager();
        let fake = InstanceId("nonexistent".to_string());
        assert!(mgr.get(&fake).is_err());
    }

    #[test]
    fn test_list_instances() {
        let mgr = make_manager();
        mgr.create(InstanceConfig::new("a1")).unwrap();
        let i2 = mgr.create(InstanceConfig::new("a2")).unwrap();
        mgr.start(&i2.id).unwrap();
        let all = mgr.list(None).unwrap();
        assert_eq!(all.len(), 2);
        let running = mgr.list(Some(InstanceState::Running)).unwrap();
        assert_eq!(running.len(), 1);
    }

    #[test]
    fn test_mark_failed() {
        let mgr = make_manager();
        let inst = mgr.create(InstanceConfig::new("agent/fail")).unwrap();
        mgr.start(&inst.id).unwrap();
        let failed = mgr.mark_failed(&inst.id, "out of memory").unwrap();
        assert_eq!(failed.state, InstanceState::Failed);
        assert!(!failed.health.healthy);
    }

    #[test]
    fn test_health_check_recording() {
        let mgr = make_manager();
        let inst = mgr.create(InstanceConfig::new("agent/hc")).unwrap();
        mgr.start(&inst.id).unwrap();
        mgr.record_health_check(&inst.id, false, Some("timeout".into())).unwrap();
        mgr.record_health_check(&inst.id, false, Some("timeout".into())).unwrap();
        let status = mgr.record_health_check(&inst.id, false, Some("timeout".into())).unwrap();
        // After 3 failures (default threshold), should be unhealthy
        assert!(!status.healthy);
        assert_eq!(status.consecutive_failures, 3);
    }

    #[test]
    fn test_instance_count() {
        let mgr = make_manager();
        assert_eq!(mgr.count().unwrap(), 0);
        mgr.create(InstanceConfig::new("a1")).unwrap();
        mgr.create(InstanceConfig::new("a2")).unwrap();
        assert_eq!(mgr.count().unwrap(), 2);
    }

    #[test]
    fn test_state_transition_valid_paths() {
        assert!(InstanceState::Created.can_transition_to(InstanceState::Starting));
        assert!(InstanceState::Starting.can_transition_to(InstanceState::Running));
        assert!(InstanceState::Running.can_transition_to(InstanceState::Stopping));
        assert!(InstanceState::Stopping.can_transition_to(InstanceState::Stopped));
        assert!(InstanceState::Stopped.can_transition_to(InstanceState::Terminated));
        assert!(!InstanceState::Terminated.can_transition_to(InstanceState::Running));
    }
}
