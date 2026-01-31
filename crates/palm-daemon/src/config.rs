//! Configuration for palm-daemon

use palm_types::PlatformProfile;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Main daemon configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,

    /// Storage configuration
    #[serde(default)]
    pub storage: StorageConfig,

    /// Scheduler configuration
    #[serde(default)]
    pub scheduler: SchedulerConfig,

    /// Platform profile
    #[serde(default)]
    pub platform: PlatformProfile,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            scheduler: SchedulerConfig::default(),
            platform: PlatformProfile::Development,
            logging: LoggingConfig::default(),
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Listen address
    pub listen_addr: SocketAddr,

    /// Enable CORS
    #[serde(default = "default_true")]
    pub enable_cors: bool,

    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub request_timeout_secs: u64,

    /// Maximum request body size in bytes
    #[serde(default = "default_max_body_size")]
    pub max_body_size: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:8080".parse().unwrap(),
            enable_cors: true,
            request_timeout_secs: 30,
            max_body_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StorageConfig {
    /// In-memory storage (for development/testing)
    Memory,

    /// PostgreSQL storage
    Postgres {
        /// Connection URL
        url: String,

        /// Maximum connections in pool
        #[serde(default = "default_pool_size")]
        max_connections: u32,

        /// Connection timeout in seconds
        #[serde(default = "default_connection_timeout")]
        connect_timeout_secs: u64,
    },
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig::Memory
    }
}

/// Scheduler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Reconciliation interval in seconds
    #[serde(default = "default_reconcile_interval")]
    pub reconcile_interval_secs: u64,

    /// Health check interval in seconds
    #[serde(default = "default_health_interval")]
    pub health_check_interval_secs: u64,

    /// Metrics collection interval in seconds
    #[serde(default = "default_metrics_interval")]
    pub metrics_interval_secs: u64,

    /// Maximum concurrent reconciliations
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_reconciliations: usize,

    /// Enable auto-healing
    #[serde(default = "default_true")]
    pub auto_healing_enabled: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            reconcile_interval_secs: 10,
            health_check_interval_secs: 5,
            metrics_interval_secs: 15,
            max_concurrent_reconciliations: 10,
            auto_healing_enabled: true,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    #[serde(default = "default_log_level")]
    pub level: String,

    /// JSON format
    #[serde(default)]
    pub json: bool,

    /// Include timestamps
    #[serde(default = "default_true")]
    pub timestamps: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json: false,
            timestamps: true,
        }
    }
}

// Default value helpers
fn default_true() -> bool {
    true
}

fn default_request_timeout() -> u64 {
    30
}

fn default_max_body_size() -> usize {
    10 * 1024 * 1024
}

fn default_pool_size() -> u32 {
    10
}

fn default_connection_timeout() -> u64 {
    5
}

fn default_reconcile_interval() -> u64 {
    10
}

fn default_health_interval() -> u64 {
    5
}

fn default_metrics_interval() -> u64 {
    15
}

fn default_max_concurrent() -> usize {
    10
}

fn default_log_level() -> String {
    "info".to_string()
}

impl DaemonConfig {
    /// Load configuration from file
    pub fn load(path: Option<&str>) -> Result<Self, config::ConfigError> {
        let mut builder = config::Config::builder();

        // Add default configuration
        builder = builder.add_source(config::Config::try_from(&DaemonConfig::default())?);

        // Add file configuration if provided
        if let Some(path) = path {
            builder = builder.add_source(config::File::with_name(path).required(false));
        }

        // Add environment variables with PALM_ prefix
        builder = builder.add_source(
            config::Environment::with_prefix("PALM")
                .separator("_")
                .try_parsing(true),
        );

        builder.build()?.try_deserialize()
    }

    /// Create a development configuration
    pub fn development() -> Self {
        Self {
            platform: PlatformProfile::Development,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DaemonConfig::default();
        assert_eq!(config.server.listen_addr.port(), 8080);
        assert!(matches!(config.storage, StorageConfig::Memory));
        assert!(matches!(config.platform, PlatformProfile::Development));
    }

    #[test]
    fn test_server_defaults() {
        let config = ServerConfig::default();
        assert!(config.enable_cors);
        assert_eq!(config.request_timeout_secs, 30);
    }

    #[test]
    fn test_scheduler_defaults() {
        let config = SchedulerConfig::default();
        assert_eq!(config.reconcile_interval_secs, 10);
        assert!(config.auto_healing_enabled);
    }
}
