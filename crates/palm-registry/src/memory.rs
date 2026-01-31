//! In-memory implementations of registry traits
//!
//! These are suitable for development and testing. Production deployments
//! should use persistent backends.

use crate::agent::AgentRegistry;
use crate::discovery::{DiscoveryQuery, DiscoveryResult, DiscoveryService, RoutingStrategy};
use crate::error::{RegistryError, Result};
use crate::instance::InstanceRegistry;
use async_trait::async_trait;
use dashmap::DashMap;
use palm_types::{
    AgentInstance, AgentSpec, AgentSpecId, DeploymentId, HealthStatus, InstanceId, InstanceStatus,
};
use semver::Version;
use std::sync::atomic::{AtomicU64, Ordering};

/// In-memory agent registry
pub struct InMemoryAgentRegistry {
    specs: DashMap<AgentSpecId, AgentSpec>,
    by_name: DashMap<String, Vec<AgentSpecId>>,
}

impl InMemoryAgentRegistry {
    pub fn new() -> Self {
        Self {
            specs: DashMap::new(),
            by_name: DashMap::new(),
        }
    }
}

impl Default for InMemoryAgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRegistry for InMemoryAgentRegistry {
    async fn register(&self, spec: AgentSpec) -> Result<AgentSpecId> {
        let id = spec.id.clone();

        if self.specs.contains_key(&id) {
            return Err(RegistryError::SpecAlreadyExists(id));
        }

        self.specs.insert(id.clone(), spec.clone());

        // Index by name
        self.by_name
            .entry(spec.name.clone())
            .or_default()
            .push(id.clone());

        Ok(id)
    }

    async fn get(&self, id: &AgentSpecId) -> Result<Option<AgentSpec>> {
        Ok(self.specs.get(id).map(|s| s.clone()))
    }

    async fn get_by_name_version(
        &self,
        name: &str,
        version: &Version,
    ) -> Result<Option<AgentSpec>> {
        if let Some(ids) = self.by_name.get(name) {
            for id in ids.iter() {
                if let Some(spec) = self.specs.get(id) {
                    if &spec.version == version {
                        return Ok(Some(spec.clone()));
                    }
                }
            }
        }
        Ok(None)
    }

    async fn list(&self) -> Result<Vec<AgentSpec>> {
        Ok(self.specs.iter().map(|s| s.value().clone()).collect())
    }

    async fn list_versions(&self, name: &str) -> Result<Vec<AgentSpec>> {
        let mut result = Vec::new();
        if let Some(ids) = self.by_name.get(name) {
            for id in ids.iter() {
                if let Some(spec) = self.specs.get(id) {
                    result.push(spec.clone());
                }
            }
        }
        Ok(result)
    }

    async fn update(&self, spec: AgentSpec) -> Result<()> {
        if !self.specs.contains_key(&spec.id) {
            return Err(RegistryError::SpecNotFound(spec.id.clone()));
        }
        self.specs.insert(spec.id.clone(), spec);
        Ok(())
    }

    async fn delete(&self, id: &AgentSpecId) -> Result<()> {
        if let Some((_, spec)) = self.specs.remove(id) {
            // Remove from name index
            if let Some(mut ids) = self.by_name.get_mut(&spec.name) {
                ids.retain(|i| i != id);
            }
        }
        Ok(())
    }

    async fn exists(&self, id: &AgentSpecId) -> Result<bool> {
        Ok(self.specs.contains_key(id))
    }
}

/// In-memory instance registry
pub struct InMemoryInstanceRegistry {
    instances: DashMap<InstanceId, AgentInstance>,
    by_deployment: DashMap<DeploymentId, Vec<InstanceId>>,
}

impl InMemoryInstanceRegistry {
    pub fn new() -> Self {
        Self {
            instances: DashMap::new(),
            by_deployment: DashMap::new(),
        }
    }
}

impl Default for InMemoryInstanceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InstanceRegistry for InMemoryInstanceRegistry {
    async fn register(&self, instance: AgentInstance) -> Result<()> {
        let id = instance.id.clone();
        let deployment_id = instance.deployment_id.clone();

        if self.instances.contains_key(&id) {
            return Err(RegistryError::InstanceAlreadyExists(id));
        }

        self.instances.insert(id.clone(), instance);

        // Index by deployment
        self.by_deployment
            .entry(deployment_id)
            .or_default()
            .push(id);

        Ok(())
    }

    async fn get(&self, id: &InstanceId) -> Result<Option<AgentInstance>> {
        Ok(self.instances.get(id).map(|i| i.clone()))
    }

    async fn list_for_deployment(
        &self,
        deployment_id: &DeploymentId,
    ) -> Result<Vec<AgentInstance>> {
        let mut result = Vec::new();
        if let Some(ids) = self.by_deployment.get(deployment_id) {
            for id in ids.iter() {
                if let Some(instance) = self.instances.get(id) {
                    result.push(instance.clone());
                }
            }
        }
        Ok(result)
    }

    async fn list_all(&self) -> Result<Vec<AgentInstance>> {
        Ok(self.instances.iter().map(|i| i.value().clone()).collect())
    }

    async fn update_status(&self, id: &InstanceId, status: InstanceStatus) -> Result<()> {
        if let Some(mut instance) = self.instances.get_mut(id) {
            instance.status = status;
            Ok(())
        } else {
            Err(RegistryError::InstanceNotFound(id.clone()))
        }
    }

    async fn update_health(&self, id: &InstanceId, health: HealthStatus) -> Result<()> {
        if let Some(mut instance) = self.instances.get_mut(id) {
            instance.health = health;
            Ok(())
        } else {
            Err(RegistryError::InstanceNotFound(id.clone()))
        }
    }

    async fn update_heartbeat(&self, id: &InstanceId) -> Result<()> {
        if let Some(mut instance) = self.instances.get_mut(id) {
            instance.last_heartbeat = chrono::Utc::now();
            Ok(())
        } else {
            Err(RegistryError::InstanceNotFound(id.clone()))
        }
    }

    async fn remove(&self, id: &InstanceId) -> Result<()> {
        if let Some((_, instance)) = self.instances.remove(id) {
            // Remove from deployment index
            if let Some(mut ids) = self.by_deployment.get_mut(&instance.deployment_id) {
                ids.retain(|i| i != id);
            }
        }
        Ok(())
    }

    async fn count_for_deployment(&self, deployment_id: &DeploymentId) -> Result<u32> {
        Ok(self
            .by_deployment
            .get(deployment_id)
            .map(|ids| ids.len() as u32)
            .unwrap_or(0))
    }

    async fn count_healthy_for_deployment(&self, deployment_id: &DeploymentId) -> Result<u32> {
        let mut count = 0;
        if let Some(ids) = self.by_deployment.get(deployment_id) {
            for id in ids.iter() {
                if let Some(instance) = self.instances.get(id) {
                    if instance.health.is_healthy() {
                        count += 1;
                    }
                }
            }
        }
        Ok(count)
    }
}

/// In-memory discovery service
pub struct InMemoryDiscoveryService {
    instance_registry: std::sync::Arc<dyn InstanceRegistry>,
    weights: DashMap<InstanceId, f64>,
    round_robin_counter: AtomicU64,
}

impl InMemoryDiscoveryService {
    pub fn new(instance_registry: std::sync::Arc<dyn InstanceRegistry>) -> Self {
        Self {
            instance_registry,
            weights: DashMap::new(),
            round_robin_counter: AtomicU64::new(0),
        }
    }
}

#[async_trait]
impl DiscoveryService for InMemoryDiscoveryService {
    async fn discover_by_capability(
        &self,
        _capability: &str,
        query: &DiscoveryQuery,
    ) -> Result<Vec<DiscoveryResult>> {
        // For in-memory implementation, we don't have capability indexing
        // Return all healthy instances
        let instances = self.instance_registry.list_all().await?;
        self.filter_and_route(instances, query).await
    }

    async fn discover_for_deployment(
        &self,
        deployment_id: &DeploymentId,
        query: &DiscoveryQuery,
    ) -> Result<Vec<DiscoveryResult>> {
        let instances = self
            .instance_registry
            .list_for_deployment(deployment_id)
            .await?;
        self.filter_and_route(instances, query).await
    }

    async fn register_instance(&self, instance: &AgentInstance) -> Result<()> {
        self.weights.insert(instance.id.clone(), 1.0);
        Ok(())
    }

    async fn deregister_instance(&self, id: &InstanceId) -> Result<()> {
        self.weights.remove(id);
        Ok(())
    }

    async fn update_weight(&self, id: &InstanceId, weight: f64) -> Result<()> {
        self.weights.insert(id.clone(), weight);
        Ok(())
    }
}

impl InMemoryDiscoveryService {
    async fn filter_and_route(
        &self,
        instances: Vec<AgentInstance>,
        query: &DiscoveryQuery,
    ) -> Result<Vec<DiscoveryResult>> {
        // Filter instances
        let filtered: Vec<_> = instances
            .into_iter()
            .filter(|i| {
                // Filter by health
                if let Some(min_score) = query.min_health_score {
                    if !i.health.is_healthy() && min_score > 0.5 {
                        return false;
                    }
                }

                // Filter by zone
                if let Some(ref zone) = query.preferred_zone {
                    if i.placement.zone.as_ref() != Some(zone) {
                        // Don't filter out, just note preference
                    }
                }

                // Filter by capacity
                if query.require_capacity {
                    // Would need attention budget info here
                }

                true
            })
            .collect();

        // Convert to discovery results
        let mut results: Vec<DiscoveryResult> = filtered
            .into_iter()
            .map(|i| {
                let weight = self.weights.get(&i.id).map(|w| *w).unwrap_or(1.0);
                DiscoveryResult {
                    instance_id: i.id.clone(),
                    deployment_id: i.deployment_id.clone(),
                    weight,
                    health_score: if i.health.is_healthy() { 1.0 } else { 0.5 },
                    available_attention: 0, // Would need runtime integration
                    available_coupling_slots: 0,
                    zone: i.placement.zone.clone(),
                    region: i.placement.region.clone(),
                }
            })
            .collect();

        // Apply routing strategy
        match query.routing_strategy {
            RoutingStrategy::RoundRobin => {
                let idx = self.round_robin_counter.fetch_add(1, Ordering::SeqCst);
                if !results.is_empty() {
                    let start = (idx as usize) % results.len();
                    results.rotate_left(start);
                }
            }
            RoutingStrategy::Random => {
                // Simple shuffle based on current time
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as usize;
                if !results.is_empty() {
                    let len = results.len();
                    results.rotate_left(seed % len);
                }
            }
            RoutingStrategy::WeightedHealth => {
                results.sort_by(|a, b| {
                    b.health_score
                        .partial_cmp(&a.health_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            RoutingStrategy::LeastLoaded => {
                // Would need load metrics
            }
            RoutingStrategy::AttentionAware => {
                results.sort_by(|a, b| {
                    b.available_attention
                        .cmp(&a.available_attention)
                });
            }
            RoutingStrategy::ZoneAffinity => {
                if let Some(ref preferred) = query.preferred_zone {
                    results.sort_by(|a, b| {
                        let a_match = a.zone.as_ref() == Some(preferred);
                        let b_match = b.zone.as_ref() == Some(preferred);
                        b_match.cmp(&a_match)
                    });
                }
            }
        }

        // Apply limit
        if let Some(limit) = query.limit {
            results.truncate(limit as usize);
        }

        Ok(results)
    }
}
