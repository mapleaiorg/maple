//! Discovery Routing Manager
//!
//! NOTE: This is Resonance-native routing (discovery + coupling admission),
//! NOT HTTP/L7 traffic routing. "Traffic" here means "which instances are
//! discoverable and can accept new couplings".

use dashmap::DashMap;
use palm_types::{DeploymentId, InstanceId};
use tracing::info;

/// Manages which instances are discoverable and their routing weights
pub struct DiscoveryRoutingManager {
    /// Deployment -> (old instances, new instances, new_percentage)
    splits: DashMap<DeploymentId, TrafficSplit>,
    /// Instance -> discoverable
    instance_status: DashMap<InstanceId, InstanceRoutingStatus>,
}

#[derive(Clone)]
struct TrafficSplit {
    old_instances: Vec<InstanceId>,
    new_instances: Vec<InstanceId>,
    new_percentage: u32,
}

#[derive(Clone)]
struct InstanceRoutingStatus {
    discoverable: bool,
    weight: f64,
}

impl DiscoveryRoutingManager {
    /// Create a new routing manager
    pub fn new() -> Self {
        Self {
            splits: DashMap::new(),
            instance_status: DashMap::new(),
        }
    }

    /// Add an instance to discovery
    pub async fn add_instance(&self, instance_id: &InstanceId) {
        self.instance_status.insert(
            instance_id.clone(),
            InstanceRoutingStatus {
                discoverable: true,
                weight: 1.0,
            },
        );
    }

    /// Remove an instance from discovery
    pub async fn remove_instance(&self, instance_id: &InstanceId) {
        if let Some(mut s) = self.instance_status.get_mut(instance_id) {
            s.discoverable = false;
        }
    }

    /// Set traffic split between old and new instances
    pub async fn set_traffic_split(
        &self,
        deployment_id: &DeploymentId,
        old_instances: Vec<InstanceId>,
        new_instances: Vec<InstanceId>,
        new_percentage: u32,
    ) -> std::result::Result<(), String> {
        // Update weights based on split
        let old_weight = (100 - new_percentage) as f64 / 100.0;
        let new_weight = new_percentage as f64 / 100.0;

        // Normalize by instance count
        let old_per_instance = if old_instances.is_empty() {
            0.0
        } else {
            old_weight / old_instances.len() as f64
        };

        let new_per_instance = if new_instances.is_empty() {
            0.0
        } else {
            new_weight / new_instances.len() as f64
        };

        for id in &old_instances {
            if let Some(mut s) = self.instance_status.get_mut(id) {
                s.weight = old_per_instance;
                s.discoverable = old_per_instance > 0.0;
            }
        }

        for id in &new_instances {
            self.instance_status.insert(
                id.clone(),
                InstanceRoutingStatus {
                    discoverable: new_per_instance > 0.0,
                    weight: new_per_instance,
                },
            );
        }

        self.splits.insert(
            deployment_id.clone(),
            TrafficSplit {
                old_instances,
                new_instances,
                new_percentage,
            },
        );

        info!(
            deployment_id = %deployment_id,
            new_percentage = new_percentage,
            "Traffic split updated"
        );

        Ok(())
    }

    /// Check if an instance is discoverable
    pub async fn is_discoverable(&self, instance_id: &InstanceId) -> bool {
        self.instance_status
            .get(instance_id)
            .map(|s| s.discoverable)
            .unwrap_or(false)
    }

    /// Get routing weight for an instance
    pub async fn get_weight(&self, instance_id: &InstanceId) -> f64 {
        self.instance_status
            .get(instance_id)
            .map(|s| s.weight)
            .unwrap_or(0.0)
    }

    /// Get current traffic split for a deployment
    pub async fn get_split(
        &self,
        deployment_id: &DeploymentId,
    ) -> Option<(u32, Vec<InstanceId>, Vec<InstanceId>)> {
        self.splits.get(deployment_id).map(|s| {
            (
                s.new_percentage,
                s.old_instances.clone(),
                s.new_instances.clone(),
            )
        })
    }

    /// Clear traffic split for a deployment
    pub async fn clear_split(&self, deployment_id: &DeploymentId) {
        self.splits.remove(deployment_id);
    }
}

impl Default for DiscoveryRoutingManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_traffic_split() {
        let manager = DiscoveryRoutingManager::new();
        let deployment_id = DeploymentId::generate();

        let old = vec![InstanceId::generate(), InstanceId::generate()];
        let new = vec![InstanceId::generate()];

        // Add instances first
        for id in &old {
            manager.add_instance(id).await;
        }
        for id in &new {
            manager.add_instance(id).await;
        }

        // Set 30% to new
        manager
            .set_traffic_split(&deployment_id, old.clone(), new.clone(), 30)
            .await
            .unwrap();

        // New instance should have higher weight per instance
        let new_weight = manager.get_weight(&new[0]).await;
        let old_weight = manager.get_weight(&old[0]).await;

        assert!(new_weight > 0.0);
        assert!(old_weight > 0.0);
    }
}
