//! MAPLE Fleet Stack -- multi-service topology manager (docker-compose-like for agents).
//!
//! Defines stacks of agent services with dependency ordering, YAML parsing,
//! and lifecycle management with topological sort for deploy/teardown.

pub mod error;

use chrono::{DateTime, Utc};
pub use error::{StackError, StackResult};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Unique identifier for a stack.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StackId(pub String);

impl StackId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for StackId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for StackId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Lifecycle state of a stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StackState {
    Pending,
    Deploying,
    Running,
    Degraded,
    Stopping,
    Stopped,
}

/// Resource limits for a service in a stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceResources {
    pub memory_mb: u64,
    pub cpu_cores: f64,
}

impl Default for ServiceResources {
    fn default() -> Self {
        Self {
            memory_mb: 256,
            cpu_cores: 0.5,
        }
    }
}

/// Health check configuration for a service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealthCheck {
    pub interval_secs: u64,
    pub timeout_secs: u64,
    pub retries: u32,
}

impl Default for ServiceHealthCheck {
    fn default() -> Self {
        Self {
            interval_secs: 30,
            timeout_secs: 5,
            retries: 3,
        }
    }
}

/// Definition of a single service within a stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDefinition {
    pub agent_ref: String,
    #[serde(default = "default_replicas")]
    pub replicas: u32,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub resources: ServiceResources,
    #[serde(default)]
    pub health_check: ServiceHealthCheck,
}

fn default_replicas() -> u32 {
    1
}

/// Network definition for inter-service communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDefinition {
    pub driver: String,
    #[serde(default)]
    pub options: HashMap<String, String>,
}

/// A complete stack definition (parsed from YAML).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackDefinition {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    pub services: BTreeMap<String, ServiceDefinition>,
    #[serde(default)]
    pub networks: HashMap<String, NetworkDefinition>,
}

fn default_version() -> String {
    "1.0".to_string()
}

impl StackDefinition {
    /// Parse a stack definition from a YAML string.
    pub fn from_yaml(yaml: &str) -> StackResult<Self> {
        let def: StackDefinition = serde_yaml::from_str(yaml)?;
        def.validate()?;
        Ok(def)
    }

    /// Validate that all `depends_on` references point to existing services
    /// and that there are no circular dependencies.
    pub fn validate(&self) -> StackResult<()> {
        // Check for unknown dependencies
        for (name, svc) in &self.services {
            for dep in &svc.depends_on {
                if !self.services.contains_key(dep) {
                    return Err(StackError::UnknownDependency {
                        service: name.clone(),
                        dependency: dep.clone(),
                    });
                }
            }
        }
        // Check for circular dependencies
        self.topological_sort()?;
        Ok(())
    }

    /// Return services in topological order (dependencies first).
    pub fn topological_sort(&self) -> StackResult<Vec<String>> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();

        for name in self.services.keys() {
            in_degree.entry(name.as_str()).or_insert(0);
            adjacency.entry(name.as_str()).or_default();
        }

        for (name, svc) in &self.services {
            for dep in &svc.depends_on {
                adjacency.entry(dep.as_str()).or_default().push(name.as_str());
                *in_degree.entry(name.as_str()).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&name, _)| name)
            .collect();

        let mut sorted = Vec::new();
        while let Some(node) = queue.pop_front() {
            sorted.push(node.to_string());
            if let Some(neighbors) = adjacency.get(node) {
                for &neighbor in neighbors {
                    let deg = in_degree.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        if sorted.len() != self.services.len() {
            return Err(StackError::CircularDependency {
                cycle: "circular dependency detected among services".to_string(),
            });
        }

        Ok(sorted)
    }
}

// ---------------------------------------------------------------------------
// Stack runtime
// ---------------------------------------------------------------------------

/// Runtime state of a deployed stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stack {
    pub id: StackId,
    pub definition: StackDefinition,
    pub state: StackState,
    pub created_at: DateTime<Utc>,
    pub deployed_at: Option<DateTime<Utc>>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub service_states: HashMap<String, ServiceState>,
}

/// State of an individual service within a running stack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceState {
    Pending,
    Starting,
    Running,
    Failed(String),
    Stopped,
}

/// Manages stack deployments.
pub struct StackManager {
    stacks: HashMap<StackId, Stack>,
}

impl Default for StackManager {
    fn default() -> Self {
        Self::new()
    }
}

impl StackManager {
    pub fn new() -> Self {
        Self {
            stacks: HashMap::new(),
        }
    }

    /// Deploy a stack from a definition.
    pub fn deploy(&mut self, definition: StackDefinition) -> StackResult<Stack> {
        let order = definition.topological_sort()?;
        let id = StackId::new();
        let mut service_states = HashMap::new();
        for svc_name in &order {
            service_states.insert(svc_name.clone(), ServiceState::Running);
        }
        let stack = Stack {
            id: id.clone(),
            definition,
            state: StackState::Running,
            created_at: Utc::now(),
            deployed_at: Some(Utc::now()),
            stopped_at: None,
            service_states,
        };
        self.stacks.insert(id, stack.clone());
        Ok(stack)
    }

    /// Get the status of a stack.
    pub fn status(&self, id: &StackId) -> StackResult<&Stack> {
        self.stacks
            .get(id)
            .ok_or_else(|| StackError::LifecycleError(format!("stack not found: {}", id)))
    }

    /// Tear down a running stack (reverse topological order).
    pub fn teardown(&mut self, id: &StackId) -> StackResult<Stack> {
        let stack = self
            .stacks
            .get_mut(id)
            .ok_or_else(|| StackError::LifecycleError(format!("stack not found: {}", id)))?;

        if stack.state == StackState::Stopped {
            return Err(StackError::InvalidStateTransition {
                operation: "teardown".into(),
                current_state: "Stopped".into(),
            });
        }

        stack.state = StackState::Stopping;
        // Reverse topological order for teardown
        let mut order = stack.definition.topological_sort()?;
        order.reverse();
        for svc_name in &order {
            stack
                .service_states
                .insert(svc_name.clone(), ServiceState::Stopped);
        }
        stack.state = StackState::Stopped;
        stack.stopped_at = Some(Utc::now());
        Ok(stack.clone())
    }

    /// Mark a service as failed, potentially degrading the stack.
    pub fn mark_service_failed(&mut self, id: &StackId, service: &str, reason: &str) -> StackResult<()> {
        let stack = self
            .stacks
            .get_mut(id)
            .ok_or_else(|| StackError::LifecycleError(format!("stack not found: {}", id)))?;
        stack
            .service_states
            .insert(service.to_string(), ServiceState::Failed(reason.to_string()));
        // Check if any services are failed => Degraded
        let has_failed = stack.service_states.values().any(|s| matches!(s, ServiceState::Failed(_)));
        if has_failed && stack.state == StackState::Running {
            stack.state = StackState::Degraded;
        }
        Ok(())
    }

    /// List all stacks.
    pub fn list(&self) -> Vec<&Stack> {
        self.stacks.values().collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_yaml() -> &'static str {
        r#"
name: my-stack
version: "1.0"
services:
  frontend:
    agent_ref: "agent/frontend:1.0"
    replicas: 2
    depends_on:
      - backend
  backend:
    agent_ref: "agent/backend:1.0"
    replicas: 1
    depends_on:
      - database
  database:
    agent_ref: "agent/postgres:14"
    replicas: 1
"#
    }

    #[test]
    fn test_parse_yaml() {
        let def = StackDefinition::from_yaml(sample_yaml()).unwrap();
        assert_eq!(def.name, "my-stack");
        assert_eq!(def.services.len(), 3);
    }

    #[test]
    fn test_topological_sort() {
        let def = StackDefinition::from_yaml(sample_yaml()).unwrap();
        let order = def.topological_sort().unwrap();
        let db_pos = order.iter().position(|s| s == "database").unwrap();
        let be_pos = order.iter().position(|s| s == "backend").unwrap();
        let fe_pos = order.iter().position(|s| s == "frontend").unwrap();
        assert!(db_pos < be_pos);
        assert!(be_pos < fe_pos);
    }

    #[test]
    fn test_circular_dependency() {
        let yaml = r#"
name: bad-stack
services:
  a:
    agent_ref: "a"
    depends_on: ["b"]
  b:
    agent_ref: "b"
    depends_on: ["a"]
"#;
        let result = StackDefinition::from_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_dependency() {
        let yaml = r#"
name: bad-stack
services:
  a:
    agent_ref: "a"
    depends_on: ["nonexistent"]
"#;
        let result = StackDefinition::from_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_deploy_stack() {
        let def = StackDefinition::from_yaml(sample_yaml()).unwrap();
        let mut mgr = StackManager::new();
        let stack = mgr.deploy(def).unwrap();
        assert_eq!(stack.state, StackState::Running);
        assert_eq!(stack.service_states.len(), 3);
    }

    #[test]
    fn test_teardown_stack() {
        let def = StackDefinition::from_yaml(sample_yaml()).unwrap();
        let mut mgr = StackManager::new();
        let stack = mgr.deploy(def).unwrap();
        let torn = mgr.teardown(&stack.id).unwrap();
        assert_eq!(torn.state, StackState::Stopped);
        assert!(torn.service_states.values().all(|s| *s == ServiceState::Stopped));
    }

    #[test]
    fn test_status() {
        let def = StackDefinition::from_yaml(sample_yaml()).unwrap();
        let mut mgr = StackManager::new();
        let stack = mgr.deploy(def).unwrap();
        let status = mgr.status(&stack.id).unwrap();
        assert_eq!(status.state, StackState::Running);
    }

    #[test]
    fn test_mark_service_failed() {
        let def = StackDefinition::from_yaml(sample_yaml()).unwrap();
        let mut mgr = StackManager::new();
        let stack = mgr.deploy(def).unwrap();
        mgr.mark_service_failed(&stack.id, "backend", "OOM").unwrap();
        let status = mgr.status(&stack.id).unwrap();
        assert_eq!(status.state, StackState::Degraded);
    }

    #[test]
    fn test_list_stacks() {
        let mut mgr = StackManager::new();
        assert!(mgr.list().is_empty());
        let def = StackDefinition::from_yaml(sample_yaml()).unwrap();
        mgr.deploy(def).unwrap();
        assert_eq!(mgr.list().len(), 1);
    }

    #[test]
    fn test_no_services_stack() {
        let yaml = r#"
name: empty
services: {}
"#;
        let def = StackDefinition::from_yaml(yaml).unwrap();
        let order = def.topological_sort().unwrap();
        assert!(order.is_empty());
    }

    #[test]
    fn test_default_replicas() {
        let yaml = r#"
name: defaults
services:
  svc:
    agent_ref: "agent/test"
"#;
        let def = StackDefinition::from_yaml(yaml).unwrap();
        assert_eq!(def.services["svc"].replicas, 1);
    }
}
