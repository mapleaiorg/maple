//! Builder for PalmControlPlane
//!
//! The builder pattern allows flexible configuration of the control plane
//! with all its required dependencies.

use crate::control_plane::PalmControlPlane;
use crate::error::{ControlPlaneError, Result};
use palm_deployment::{DeploymentStateStore, PolicyGate};
use palm_health::HealthConfig;
use palm_registry::{AgentRegistry, InstanceRegistry};
use palm_state::{
    AasClient, ContinuityVerifier, CouplingRuntime, MigrationRuntime, RuntimeStateGatherer,
    RuntimeStateRestorer, StateManagerConfig, StateStorage,
};
use palm_types::PlatformProfile;
use std::sync::Arc;

/// Builder for constructing a PalmControlPlane with all dependencies
pub struct PalmControlPlaneBuilder {
    platform: PlatformProfile,
    agent_registry: Option<Arc<dyn AgentRegistry>>,
    instance_registry: Option<Arc<dyn InstanceRegistry>>,
    policy_gate: Option<Arc<dyn PolicyGate>>,
    state_storage: Option<Arc<dyn StateStorage>>,
    deployment_state_store: Option<Arc<dyn DeploymentStateStore>>,
    state_gatherer: Option<Arc<dyn RuntimeStateGatherer>>,
    state_restorer: Option<Arc<dyn RuntimeStateRestorer>>,
    continuity_verifier: Option<Arc<dyn ContinuityVerifier>>,
    coupling_runtime: Option<Arc<dyn CouplingRuntime>>,
    migration_runtime: Option<Arc<dyn MigrationRuntime>>,
    aas_client: Option<Arc<dyn AasClient>>,
    health_config: Option<HealthConfig>,
    state_config: Option<StateManagerConfig>,
}

impl PalmControlPlaneBuilder {
    /// Create a new builder for the given platform
    pub fn new(platform: PlatformProfile) -> Self {
        Self {
            platform,
            agent_registry: None,
            instance_registry: None,
            policy_gate: None,
            state_storage: None,
            deployment_state_store: None,
            state_gatherer: None,
            state_restorer: None,
            continuity_verifier: None,
            coupling_runtime: None,
            migration_runtime: None,
            aas_client: None,
            health_config: None,
            state_config: None,
        }
    }

    /// Set the agent registry
    pub fn with_agent_registry(mut self, registry: Arc<dyn AgentRegistry>) -> Self {
        self.agent_registry = Some(registry);
        self
    }

    /// Set the instance registry
    pub fn with_instance_registry(mut self, registry: Arc<dyn InstanceRegistry>) -> Self {
        self.instance_registry = Some(registry);
        self
    }

    /// Set the policy gate
    pub fn with_policy_gate(mut self, gate: Arc<dyn PolicyGate>) -> Self {
        self.policy_gate = Some(gate);
        self
    }

    /// Set the state storage
    pub fn with_state_storage(mut self, storage: Arc<dyn StateStorage>) -> Self {
        self.state_storage = Some(storage);
        self
    }

    /// Set the deployment state store
    pub fn with_deployment_state_store(mut self, store: Arc<dyn DeploymentStateStore>) -> Self {
        self.deployment_state_store = Some(store);
        self
    }

    /// Set the runtime state gatherer for checkpoints
    pub fn with_state_gatherer(mut self, gatherer: Arc<dyn RuntimeStateGatherer>) -> Self {
        self.state_gatherer = Some(gatherer);
        self
    }

    /// Set the runtime state restorer
    pub fn with_state_restorer(mut self, restorer: Arc<dyn RuntimeStateRestorer>) -> Self {
        self.state_restorer = Some(restorer);
        self
    }

    /// Set the continuity verifier
    pub fn with_continuity_verifier(mut self, verifier: Arc<dyn ContinuityVerifier>) -> Self {
        self.continuity_verifier = Some(verifier);
        self
    }

    /// Set the coupling runtime
    pub fn with_coupling_runtime(mut self, runtime: Arc<dyn CouplingRuntime>) -> Self {
        self.coupling_runtime = Some(runtime);
        self
    }

    /// Set the migration runtime
    pub fn with_migration_runtime(mut self, runtime: Arc<dyn MigrationRuntime>) -> Self {
        self.migration_runtime = Some(runtime);
        self
    }

    /// Set the AAS client
    pub fn with_aas_client(mut self, client: Arc<dyn AasClient>) -> Self {
        self.aas_client = Some(client);
        self
    }

    /// Set the health configuration
    pub fn with_health_config(mut self, config: HealthConfig) -> Self {
        self.health_config = Some(config);
        self
    }

    /// Set the state manager configuration
    pub fn with_state_config(mut self, config: StateManagerConfig) -> Self {
        self.state_config = Some(config);
        self
    }

    /// Build the control plane with all components
    pub fn build(self) -> Result<PalmControlPlane> {
        let agent_registry = self
            .agent_registry
            .ok_or_else(|| ControlPlaneError::InvalidRequest("agent_registry required".into()))?;
        let instance_registry = self.instance_registry.ok_or_else(|| {
            ControlPlaneError::InvalidRequest("instance_registry required".into())
        })?;
        let policy_gate = self
            .policy_gate
            .ok_or_else(|| ControlPlaneError::InvalidRequest("policy_gate required".into()))?;
        let state_storage = self
            .state_storage
            .ok_or_else(|| ControlPlaneError::InvalidRequest("state_storage required".into()))?;
        let deployment_state_store = self.deployment_state_store.ok_or_else(|| {
            ControlPlaneError::InvalidRequest("deployment_state_store required".into())
        })?;
        let state_gatherer = self
            .state_gatherer
            .ok_or_else(|| ControlPlaneError::InvalidRequest("state_gatherer required".into()))?;
        let state_restorer = self
            .state_restorer
            .ok_or_else(|| ControlPlaneError::InvalidRequest("state_restorer required".into()))?;
        let continuity_verifier = self.continuity_verifier.ok_or_else(|| {
            ControlPlaneError::InvalidRequest("continuity_verifier required".into())
        })?;
        let coupling_runtime = self
            .coupling_runtime
            .ok_or_else(|| ControlPlaneError::InvalidRequest("coupling_runtime required".into()))?;
        let migration_runtime = self.migration_runtime.ok_or_else(|| {
            ControlPlaneError::InvalidRequest("migration_runtime required".into())
        })?;
        let aas_client = self
            .aas_client
            .ok_or_else(|| ControlPlaneError::InvalidRequest("aas_client required".into()))?;

        let health_config = self
            .health_config
            .unwrap_or_else(|| HealthConfig::for_platform(self.platform));
        let state_config = self.state_config.unwrap_or_default();

        Ok(PalmControlPlane::new(
            self.platform,
            agent_registry,
            instance_registry,
            policy_gate,
            state_storage,
            deployment_state_store,
            state_gatherer,
            state_restorer,
            continuity_verifier,
            coupling_runtime,
            migration_runtime,
            aas_client,
            health_config,
            state_config,
        ))
    }
}

impl Default for PalmControlPlaneBuilder {
    fn default() -> Self {
        Self::new(PlatformProfile::Development)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use palm_deployment::{AllowAllPolicyGate, InMemoryDeploymentStateStore};
    use palm_registry::{InMemoryAgentRegistry, InMemoryInstanceRegistry};
    use palm_state::{
        checkpoint::MockRuntimeStateGatherer,
        commitment_reconcile::MockAasClient,
        coupling_restore::MockCouplingRuntime,
        migration::MockMigrationRuntime,
        restore::{MockContinuityVerifier, MockRuntimeStateRestorer},
        storage::InMemoryStateStorage,
    };

    #[test]
    fn test_builder_missing_fields() {
        let result = PalmControlPlaneBuilder::new(PlatformProfile::Development).build();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_complete() {
        let result = PalmControlPlaneBuilder::new(PlatformProfile::Development)
            .with_agent_registry(Arc::new(InMemoryAgentRegistry::new()))
            .with_instance_registry(Arc::new(InMemoryInstanceRegistry::new()))
            .with_policy_gate(Arc::new(AllowAllPolicyGate))
            .with_state_storage(Arc::new(InMemoryStateStorage::new()))
            .with_deployment_state_store(Arc::new(InMemoryDeploymentStateStore::new()))
            .with_state_gatherer(Arc::new(MockRuntimeStateGatherer::new()))
            .with_state_restorer(Arc::new(MockRuntimeStateRestorer::new()))
            .with_continuity_verifier(Arc::new(MockContinuityVerifier::new()))
            .with_coupling_runtime(Arc::new(MockCouplingRuntime::all_present()))
            .with_migration_runtime(Arc::new(MockMigrationRuntime::new("node-1")))
            .with_aas_client(Arc::new(MockAasClient::all_pending()))
            .build();

        assert!(result.is_ok());
    }
}
