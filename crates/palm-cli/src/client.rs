//! HTTP client for PALM daemon

use crate::error::{CliError, CliResult};
use palm_types::*;
use reqwest::{Client, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// HTTP client for communicating with the PALM daemon
pub struct PalmClient {
    client: Client,
    base_url: String,
    platform: Option<String>,
}

/// PALM daemon status response
#[derive(Debug, Deserialize)]
pub struct DaemonStatus {
    /// Daemon version
    pub version: String,
    /// Uptime string
    pub uptime: String,
    /// Current platform
    pub platform: Option<String>,
}

/// Request to create a new deployment
#[derive(Debug, Serialize)]
struct CreateDeploymentRequest {
    spec_id: String,
    replicas: u32,
    strategy: String,
    platform: Option<String>,
}

impl PalmClient {
    /// Create a new PALM client
    pub fn new(endpoint: &str, platform: Option<String>) -> CliResult<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: endpoint.trim_end_matches('/').to_string(),
            platform,
        })
    }

    /// Check daemon health
    pub async fn health_check(&self) -> CliResult<DaemonStatus> {
        self.get("/health").await
    }

    // ========== Spec API ==========

    /// Register a new agent specification
    pub async fn register_spec(&self, spec: &AgentSpec) -> CliResult<AgentSpecId> {
        self.post("/api/v1/specs", spec).await
    }

    /// Get a spec by ID
    pub async fn get_spec(&self, spec_id: &str) -> CliResult<AgentSpec> {
        self.get(&format!("/api/v1/specs/{}", spec_id)).await
    }

    /// List all specs
    pub async fn list_specs(&self) -> CliResult<Vec<AgentSpec>> {
        self.get("/api/v1/specs").await
    }

    /// Deprecate a spec
    pub async fn deprecate_spec(&self, spec_id: &str) -> CliResult<()> {
        self.delete(&format!("/api/v1/specs/{}", spec_id)).await
    }

    // ========== Deployment API ==========

    /// Create a new deployment
    pub async fn create_deployment(
        &self,
        spec_id: &str,
        replicas: u32,
        strategy: &str,
    ) -> CliResult<Deployment> {
        let req = CreateDeploymentRequest {
            spec_id: spec_id.to_string(),
            replicas,
            strategy: strategy.to_string(),
            platform: self.platform.clone(),
        };
        self.post("/api/v1/deployments", &req).await
    }

    /// Get a deployment by ID
    pub async fn get_deployment(&self, deployment_id: &str) -> CliResult<Deployment> {
        self.get(&format!("/api/v1/deployments/{}", deployment_id))
            .await
    }

    /// List all deployments
    pub async fn list_deployments(&self) -> CliResult<Vec<Deployment>> {
        self.get("/api/v1/deployments").await
    }

    /// Scale a deployment
    pub async fn scale_deployment(&self, deployment_id: &str, replicas: u32) -> CliResult<()> {
        self.post(
            &format!("/api/v1/deployments/{}/scale", deployment_id),
            &serde_json::json!({ "replicas": replicas }),
        )
        .await
    }

    /// Update a deployment to a new spec
    pub async fn update_deployment(
        &self,
        deployment_id: &str,
        new_spec_id: &str,
        strategy: &str,
    ) -> CliResult<Deployment> {
        self.put(
            &format!("/api/v1/deployments/{}", deployment_id),
            &serde_json::json!({
                "spec_id": new_spec_id,
                "strategy": strategy,
            }),
        )
        .await
    }

    /// Rollback a deployment
    pub async fn rollback_deployment(
        &self,
        deployment_id: &str,
        target_version: Option<&str>,
    ) -> CliResult<Deployment> {
        self.post(
            &format!("/api/v1/deployments/{}/rollback", deployment_id),
            &serde_json::json!({ "target_version": target_version }),
        )
        .await
    }

    /// Delete a deployment
    pub async fn delete_deployment(&self, deployment_id: &str, force: bool) -> CliResult<()> {
        let url = if force {
            format!("/api/v1/deployments/{}?force=true", deployment_id)
        } else {
            format!("/api/v1/deployments/{}", deployment_id)
        };
        self.delete(&url).await
    }

    // ========== Instance API ==========

    /// Get an instance by ID
    pub async fn get_instance(&self, instance_id: &str) -> CliResult<AgentInstance> {
        self.get(&format!("/api/v1/instances/{}", instance_id)).await
    }

    /// List instances, optionally filtered by deployment
    pub async fn list_instances(
        &self,
        deployment_id: Option<&str>,
    ) -> CliResult<Vec<AgentInstance>> {
        let url = match deployment_id {
            Some(id) => format!("/api/v1/instances?deployment_id={}", id),
            None => "/api/v1/instances".to_string(),
        };
        self.get(&url).await
    }

    /// Restart an instance
    pub async fn restart_instance(&self, instance_id: &str, graceful: bool) -> CliResult<()> {
        self.post(
            &format!("/api/v1/instances/{}/restart", instance_id),
            &serde_json::json!({ "graceful": graceful }),
        )
        .await
    }

    /// Drain an instance
    pub async fn drain_instance(&self, instance_id: &str) -> CliResult<()> {
        self.post(
            &format!("/api/v1/instances/{}/drain", instance_id),
            &serde_json::json!({}),
        )
        .await
    }

    /// Migrate an instance to another node
    pub async fn migrate_instance(&self, instance_id: &str, to_node: &str) -> CliResult<String> {
        self.post(
            &format!("/api/v1/instances/{}/migrate", instance_id),
            &serde_json::json!({ "to_node": to_node }),
        )
        .await
    }

    // ========== State API ==========

    /// Create a checkpoint for an instance
    pub async fn create_checkpoint(&self, instance_id: &str) -> CliResult<String> {
        self.post(
            &format!("/api/v1/instances/{}/checkpoint", instance_id),
            &serde_json::json!({}),
        )
        .await
    }

    /// List snapshots for an instance
    pub async fn list_snapshots(&self, instance_id: &str) -> CliResult<Vec<SnapshotInfo>> {
        self.get(&format!("/api/v1/instances/{}/snapshots", instance_id))
            .await
    }

    /// Restore an instance from a snapshot
    pub async fn restore_snapshot(&self, instance_id: &str, snapshot_id: &str) -> CliResult<()> {
        self.post(
            &format!("/api/v1/instances/{}/restore", instance_id),
            &serde_json::json!({ "snapshot_id": snapshot_id }),
        )
        .await
    }

    // ========== Health API ==========

    /// Get health info for an instance
    pub async fn get_instance_health(&self, instance_id: &str) -> CliResult<InstanceHealthInfo> {
        self.get(&format!("/api/v1/instances/{}/health", instance_id))
            .await
    }

    /// List unhealthy instances
    pub async fn list_unhealthy(&self) -> CliResult<Vec<AgentInstance>> {
        self.get("/api/v1/health/unhealthy").await
    }

    // ========== Events API ==========

    /// Stream events from the daemon
    pub async fn stream_events(
        &self,
    ) -> CliResult<impl futures_util::Stream<Item = CliResult<PalmEventEnvelope>>> {
        use futures_util::StreamExt;

        let response = self
            .client
            .get(&format!("{}/api/v1/events/stream", self.base_url))
            .send()
            .await?;

        let stream = response.bytes_stream().map(|result| {
            result
                .map_err(CliError::from)
                .and_then(|bytes| serde_json::from_slice(&bytes).map_err(CliError::from))
        });

        Ok(stream)
    }

    // ========== Internal HTTP helpers ==========

    async fn get<T: DeserializeOwned>(&self, path: &str) -> CliResult<T> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client.get(&url).send().await?;
        self.handle_response(response).await
    }

    async fn post<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> CliResult<T> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client.post(&url).json(body).send().await?;
        self.handle_response(response).await
    }

    async fn put<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> CliResult<T> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client.put(&url).json(body).send().await?;
        self.handle_response(response).await
    }

    async fn delete<T: DeserializeOwned>(&self, path: &str) -> CliResult<T> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client.delete(&url).send().await?;
        self.handle_response(response).await
    }

    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> CliResult<T> {
        let status = response.status();

        if status.is_success() {
            Ok(response.json().await?)
        } else if status == StatusCode::NOT_FOUND {
            Err(CliError::NotFound("Resource not found".into()))
        } else {
            let message = response.text().await.unwrap_or_default();
            Err(CliError::Api {
                status: status.as_u16(),
                message,
            })
        }
    }
}

/// Snapshot information
#[derive(Debug, Deserialize, Serialize)]
pub struct SnapshotInfo {
    /// Snapshot ID
    pub id: String,
    /// Creation timestamp
    pub created_at: String,
    /// Reason for snapshot
    pub reason: String,
    /// Size in bytes
    pub size_bytes: u64,
}

/// Instance health information
#[derive(Debug, Deserialize)]
pub struct InstanceHealthInfo {
    /// Overall health status
    pub status: String,
    /// Last health check timestamp
    pub last_check: String,
    /// Individual probe results
    pub probes: Vec<ProbeResultInfo>,
}

/// Individual probe result
#[derive(Debug, Deserialize)]
pub struct ProbeResultInfo {
    /// Probe name
    pub name: String,
    /// Whether probe passed
    pub passed: bool,
    /// Additional details
    pub details: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = PalmClient::new("http://localhost:8080", None).unwrap();
        assert_eq!(client.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_client_endpoint_normalization() {
        let client = PalmClient::new("http://localhost:8080/", None).unwrap();
        assert_eq!(client.base_url, "http://localhost:8080");
    }
}
