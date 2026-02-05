//! Discovery service trait and types
//!
//! The DiscoveryService enables finding agent instances by capability.

use crate::error::Result;
use async_trait::async_trait;
use palm_types::{AgentInstance, DeploymentId, InstanceId};
use serde::{Deserialize, Serialize};

/// Discovery service for finding agent instances
#[async_trait]
pub trait DiscoveryService: Send + Sync {
    /// Discover instances by capability
    async fn discover_by_capability(
        &self,
        capability: &str,
        query: &DiscoveryQuery,
    ) -> Result<Vec<DiscoveryResult>>;

    /// Discover instances for a deployment
    async fn discover_for_deployment(
        &self,
        deployment_id: &DeploymentId,
        query: &DiscoveryQuery,
    ) -> Result<Vec<DiscoveryResult>>;

    /// Register an instance for discovery
    async fn register_instance(&self, instance: &AgentInstance) -> Result<()>;

    /// Deregister an instance from discovery
    async fn deregister_instance(&self, id: &InstanceId) -> Result<()>;

    /// Update instance weight in discovery
    async fn update_weight(&self, id: &InstanceId, weight: f64) -> Result<()>;
}

/// Query parameters for discovery
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscoveryQuery {
    /// Maximum results to return
    pub limit: Option<u32>,

    /// Minimum health score (0.0 to 1.0)
    pub min_health_score: Option<f64>,

    /// Preferred zone
    pub preferred_zone: Option<String>,

    /// Preferred region
    pub preferred_region: Option<String>,

    /// Include only instances with available capacity
    pub require_capacity: bool,

    /// Routing strategy
    pub routing_strategy: RoutingStrategy,
}

/// Routing strategies for discovery
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingStrategy {
    /// Round-robin distribution
    #[default]
    RoundRobin,

    /// Random selection
    Random,

    /// Weighted by health/capacity
    WeightedHealth,

    /// Prefer least loaded
    LeastLoaded,

    /// Attention-aware (prefer instances with more attention budget)
    AttentionAware,

    /// Zone-affinity (prefer same zone)
    ZoneAffinity,
}

/// Result of a discovery query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    /// Instance ID
    pub instance_id: InstanceId,

    /// Deployment ID
    pub deployment_id: DeploymentId,

    /// Discovery weight (for weighted routing)
    pub weight: f64,

    /// Health score
    pub health_score: f64,

    /// Available attention budget
    pub available_attention: u64,

    /// Available coupling slots
    pub available_coupling_slots: u32,

    /// Zone
    pub zone: Option<String>,

    /// Region
    pub region: Option<String>,
}
