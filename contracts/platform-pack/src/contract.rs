//! Core Platform Pack contract definition

use crate::{
    PlatformPolicyConfig, PlatformHealthConfig, PlatformStateConfig,
    PlatformResourceConfig, PlatformRecoveryConfig, PlatformMetadata,
};
use palm_types::PlatformProfile;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// The Platform Pack trait - the primary contract for platform implementations
///
/// A Platform Pack provides the complete configuration and behavioral customization
/// needed to run PALM on a specific platform. Implementations must satisfy all
/// trait methods to be considered conformant.
#[async_trait]
pub trait PlatformPack: Send + Sync {
    /// Returns the platform profile this pack implements
    fn profile(&self) -> PlatformProfile;

    /// Returns platform metadata (name, version, description)
    fn metadata(&self) -> &PlatformMetadata;

    /// Returns the policy configuration for this platform
    fn policy_config(&self) -> &PlatformPolicyConfig;

    /// Returns the health monitoring configuration
    fn health_config(&self) -> &PlatformHealthConfig;

    /// Returns the state management configuration
    fn state_config(&self) -> &PlatformStateConfig;

    /// Returns the resource constraints configuration
    fn resource_config(&self) -> &PlatformResourceConfig;

    /// Returns the recovery behavior configuration
    fn recovery_config(&self) -> &PlatformRecoveryConfig;

    /// Returns the capabilities this platform supports
    fn capabilities(&self) -> &PlatformCapabilities;

    /// Validates that an agent spec is compatible with this platform
    async fn validate_agent_spec(&self, spec: &palm_types::AgentSpec) -> Result<(), PackError>;

    /// Called when the platform pack is loaded
    async fn on_load(&self) -> Result<(), PackError>;

    /// Called when the platform pack is unloaded
    async fn on_unload(&self) -> Result<(), PackError>;
}

/// Configuration bundle for a platform pack (serializable)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformPackConfig {
    /// Platform metadata
    pub metadata: PlatformMetadata,
    /// Policy configuration
    pub policy: PlatformPolicyConfig,
    /// Health monitoring configuration
    pub health: PlatformHealthConfig,
    /// State management configuration
    pub state: PlatformStateConfig,
    /// Resource constraints configuration
    pub resources: PlatformResourceConfig,
    /// Recovery behavior configuration
    pub recovery: PlatformRecoveryConfig,
    /// Platform capabilities
    pub capabilities: PlatformCapabilities,
}

impl PlatformPackConfig {
    /// Load configuration from TOML
    pub fn from_toml(content: &str) -> Result<Self, PackError> {
        toml::from_str(content).map_err(|e| PackError::ConfigParse(e.to_string()))
    }

    /// Load configuration from file
    pub async fn from_file(path: &str) -> Result<Self, PackError> {
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| PackError::Io(e.to_string()))?;
        Self::from_toml(&content)
    }

    /// Serialize to TOML
    pub fn to_toml(&self) -> Result<String, PackError> {
        toml::to_string_pretty(self).map_err(|e| PackError::ConfigParse(e.to_string()))
    }
}

impl Default for PlatformPackConfig {
    fn default() -> Self {
        Self {
            metadata: PlatformMetadata::default(),
            policy: PlatformPolicyConfig::default(),
            health: PlatformHealthConfig::default(),
            state: PlatformStateConfig::default(),
            resources: PlatformResourceConfig::default(),
            recovery: PlatformRecoveryConfig::default(),
            capabilities: PlatformCapabilities::default(),
        }
    }
}

/// Platform capabilities declaration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlatformCapabilities {
    /// Maximum concurrent deployments
    pub max_deployments: Option<u32>,

    /// Maximum instances per deployment
    pub max_instances_per_deployment: Option<u32>,

    /// Maximum total instances
    pub max_total_instances: Option<u32>,

    /// Supports live migration
    pub supports_migration: bool,

    /// Supports hot reload
    pub supports_hot_reload: bool,

    /// Supports canary deployments
    pub supports_canary: bool,

    /// Supports blue-green deployments
    pub supports_blue_green: bool,

    /// Supports human-in-the-loop approvals
    pub supports_human_approval: bool,

    /// Supports checkpoint/restore
    pub supports_checkpoints: bool,

    /// Supports cross-node migration
    pub supports_cross_node_migration: bool,

    /// Custom capabilities
    #[serde(default)]
    pub custom: std::collections::HashMap<String, serde_json::Value>,
}

impl PlatformCapabilities {
    /// Create capabilities for Mapleverse (high-throughput)
    pub fn mapleverse() -> Self {
        Self {
            max_deployments: Some(1000),
            max_instances_per_deployment: Some(500),
            max_total_instances: Some(10000),
            supports_migration: true,
            supports_hot_reload: true,
            supports_canary: true,
            supports_blue_green: true,
            supports_human_approval: false,
            supports_checkpoints: true,
            supports_cross_node_migration: true,
            custom: Default::default(),
        }
    }

    /// Create capabilities for Finalverse (safety-first)
    pub fn finalverse() -> Self {
        Self {
            max_deployments: Some(100),
            max_instances_per_deployment: Some(50),
            max_total_instances: Some(500),
            supports_migration: true,
            supports_hot_reload: false,
            supports_canary: true,
            supports_blue_green: true,
            supports_human_approval: true,
            supports_checkpoints: true,
            supports_cross_node_migration: false,
            custom: Default::default(),
        }
    }

    /// Create capabilities for iBank (accountability-focused)
    pub fn ibank() -> Self {
        Self {
            max_deployments: Some(50),
            max_instances_per_deployment: Some(20),
            max_total_instances: Some(200),
            supports_migration: false,
            supports_hot_reload: false,
            supports_canary: true,
            supports_blue_green: true,
            supports_human_approval: true,
            supports_checkpoints: true,
            supports_cross_node_migration: false,
            custom: Default::default(),
        }
    }
}

/// Errors that can occur in platform pack operations
#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("Configuration parse error: {0}")]
    ConfigParse(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Incompatible agent spec: {0}")]
    IncompatibleSpec(String),

    #[error("Capability not supported: {0}")]
    UnsupportedCapability(String),

    #[error("Platform error: {0}")]
    Platform(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PlatformPackConfig::default();
        assert!(config.metadata.name.is_empty() || config.metadata.name == "unknown");
    }

    #[test]
    fn test_capabilities_presets() {
        let mapleverse = PlatformCapabilities::mapleverse();
        assert!(mapleverse.supports_hot_reload);
        assert!(!mapleverse.supports_human_approval);

        let finalverse = PlatformCapabilities::finalverse();
        assert!(!finalverse.supports_hot_reload);
        assert!(finalverse.supports_human_approval);

        let ibank = PlatformCapabilities::ibank();
        assert!(ibank.supports_checkpoints);
        assert!(ibank.supports_human_approval);
    }
}
