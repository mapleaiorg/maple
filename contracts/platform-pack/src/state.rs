//! Platform-specific state management configuration

use serde::{Deserialize, Serialize};

/// State management configuration for a platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformStateConfig {
    /// Checkpoint configuration
    pub checkpoint: CheckpointConfig,

    /// State retention policy
    pub retention: RetentionConfig,

    /// State migration settings
    pub migration: MigrationConfig,

    /// Serialization settings
    pub serialization: SerializationConfig,
}

/// Checkpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointConfig {
    /// Enable automatic checkpointing
    pub auto_checkpoint: bool,

    /// Checkpoint interval (seconds)
    pub interval_secs: u64,

    /// Maximum checkpoint size (bytes)
    pub max_size_bytes: u64,

    /// Compression algorithm
    pub compression: CompressionAlgorithm,

    /// Enable incremental checkpoints
    pub incremental: bool,

    /// Maximum number of checkpoints to retain
    pub max_retained: u32,
}

/// Compression algorithm for state serialization
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CompressionAlgorithm {
    #[default]
    None,
    Gzip,
    Zstd,
    Lz4,
    Snappy,
}

/// State retention configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// Retention period for checkpoints (days)
    pub checkpoint_retention_days: u32,

    /// Retention period for audit logs (days)
    pub audit_retention_days: u32,

    /// Retention period for metrics (days)
    pub metrics_retention_days: u32,

    /// Enable automatic cleanup
    pub auto_cleanup: bool,

    /// Keep checkpoints for terminated instances (days)
    pub terminated_instance_retention_days: u32,
}

/// State migration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationConfig {
    /// Enable live migration
    pub enable_live_migration: bool,

    /// Migration timeout (seconds)
    pub timeout_secs: u64,

    /// Maximum concurrent migrations
    pub max_concurrent: u32,

    /// Pre-copy memory pages threshold
    pub precopy_threshold: f64,

    /// Require source verification after migration
    pub verify_source_after: bool,
}

/// Serialization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializationConfig {
    /// Serialization format
    pub format: SerializationFormat,

    /// Enable encryption at rest
    pub encrypt_at_rest: bool,

    /// Encryption key identifier (if encryption enabled)
    pub encryption_key_id: Option<String>,

    /// Enable integrity verification
    pub verify_integrity: bool,

    /// Hash algorithm for integrity
    pub integrity_algorithm: IntegrityAlgorithm,
}

/// Serialization format
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SerializationFormat {
    #[default]
    Bincode,
    MessagePack,
    Cbor,
    Json,
}

/// Integrity verification algorithm
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum IntegrityAlgorithm {
    #[default]
    Sha256,
    Sha384,
    Sha512,
    Blake3,
}

impl Default for PlatformStateConfig {
    fn default() -> Self {
        Self {
            checkpoint: CheckpointConfig {
                auto_checkpoint: true,
                interval_secs: 300,
                max_size_bytes: 100 * 1024 * 1024, // 100 MB
                compression: CompressionAlgorithm::Zstd,
                incremental: false,
                max_retained: 10,
            },
            retention: RetentionConfig {
                checkpoint_retention_days: 7,
                audit_retention_days: 90,
                metrics_retention_days: 30,
                auto_cleanup: true,
                terminated_instance_retention_days: 1,
            },
            migration: MigrationConfig {
                enable_live_migration: false,
                timeout_secs: 300,
                max_concurrent: 1,
                precopy_threshold: 0.9,
                verify_source_after: true,
            },
            serialization: SerializationConfig {
                format: SerializationFormat::Bincode,
                encrypt_at_rest: false,
                encryption_key_id: None,
                verify_integrity: true,
                integrity_algorithm: IntegrityAlgorithm::Sha256,
            },
        }
    }
}

impl PlatformStateConfig {
    /// Create state config for Mapleverse (fast checkpoints, high throughput)
    pub fn mapleverse() -> Self {
        Self {
            checkpoint: CheckpointConfig {
                auto_checkpoint: true,
                interval_secs: 60,
                max_size_bytes: 500 * 1024 * 1024, // 500 MB
                compression: CompressionAlgorithm::Lz4, // Fast compression
                incremental: true,
                max_retained: 5,
            },
            retention: RetentionConfig {
                checkpoint_retention_days: 3,
                audit_retention_days: 30,
                metrics_retention_days: 7,
                auto_cleanup: true,
                terminated_instance_retention_days: 0, // Immediate cleanup
            },
            migration: MigrationConfig {
                enable_live_migration: true,
                timeout_secs: 120,
                max_concurrent: 10,
                precopy_threshold: 0.95,
                verify_source_after: false, // Skip for speed
            },
            serialization: SerializationConfig {
                format: SerializationFormat::Bincode,
                encrypt_at_rest: false,
                encryption_key_id: None,
                verify_integrity: false, // Skip for speed
                integrity_algorithm: IntegrityAlgorithm::Sha256,
            },
        }
    }

    /// Create state config for Finalverse (reliable checkpoints, safety focus)
    pub fn finalverse() -> Self {
        Self {
            checkpoint: CheckpointConfig {
                auto_checkpoint: true,
                interval_secs: 180,
                max_size_bytes: 200 * 1024 * 1024, // 200 MB
                compression: CompressionAlgorithm::Zstd,
                incremental: false, // Full checkpoints for reliability
                max_retained: 20,
            },
            retention: RetentionConfig {
                checkpoint_retention_days: 30,
                audit_retention_days: 365,
                metrics_retention_days: 90,
                auto_cleanup: true,
                terminated_instance_retention_days: 7,
            },
            migration: MigrationConfig {
                enable_live_migration: true,
                timeout_secs: 600,
                max_concurrent: 2,
                precopy_threshold: 0.99,
                verify_source_after: true,
            },
            serialization: SerializationConfig {
                format: SerializationFormat::MessagePack,
                encrypt_at_rest: true,
                encryption_key_id: None, // Set by deployment
                verify_integrity: true,
                integrity_algorithm: IntegrityAlgorithm::Sha384,
            },
        }
    }

    /// Create state config for iBank (compliance-focused, long retention)
    pub fn ibank() -> Self {
        Self {
            checkpoint: CheckpointConfig {
                auto_checkpoint: true,
                interval_secs: 300,
                max_size_bytes: 100 * 1024 * 1024, // 100 MB
                compression: CompressionAlgorithm::Zstd,
                incremental: false, // Full checkpoints for compliance
                max_retained: 50,
            },
            retention: RetentionConfig {
                checkpoint_retention_days: 365,      // 1 year
                audit_retention_days: 2555,          // 7 years (regulatory)
                metrics_retention_days: 365,
                auto_cleanup: false, // Manual cleanup with approval
                terminated_instance_retention_days: 90,
            },
            migration: MigrationConfig {
                enable_live_migration: false, // No live migration for compliance
                timeout_secs: 900,
                max_concurrent: 1,
                precopy_threshold: 0.99,
                verify_source_after: true,
            },
            serialization: SerializationConfig {
                format: SerializationFormat::Cbor, // Good for archival
                encrypt_at_rest: true,
                encryption_key_id: None, // Set by deployment
                verify_integrity: true,
                integrity_algorithm: IntegrityAlgorithm::Sha512,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let config = PlatformStateConfig::default();
        assert!(config.checkpoint.auto_checkpoint);
        assert_eq!(config.retention.checkpoint_retention_days, 7);
    }

    #[test]
    fn test_mapleverse_state() {
        let config = PlatformStateConfig::mapleverse();
        assert!(config.checkpoint.incremental);
        assert!(config.migration.enable_live_migration);
        assert_eq!(config.retention.terminated_instance_retention_days, 0);
    }

    #[test]
    fn test_finalverse_state() {
        let config = PlatformStateConfig::finalverse();
        assert!(config.serialization.encrypt_at_rest);
        assert!(config.serialization.verify_integrity);
    }

    #[test]
    fn test_ibank_state() {
        let config = PlatformStateConfig::ibank();
        assert_eq!(config.retention.audit_retention_days, 2555); // 7 years
        assert!(!config.migration.enable_live_migration);
        assert!(!config.retention.auto_cleanup);
    }
}
