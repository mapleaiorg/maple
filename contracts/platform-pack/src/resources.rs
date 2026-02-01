//! Platform-specific resource configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Resource configuration for a platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformResourceConfig {
    /// Default resource allocations
    pub defaults: ResourceDefaults,

    /// Resource limits
    pub limits: ResourceLimits,

    /// Scaling configuration
    pub scaling: ScalingConfig,

    /// Resource affinity rules
    pub affinity: AffinityConfig,
}

/// Default resource allocations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDefaults {
    /// Default CPU allocation (millicores)
    pub cpu_millicores: u32,

    /// Default memory allocation (megabytes)
    pub memory_mb: u32,

    /// Default storage allocation (megabytes)
    pub storage_mb: u32,

    /// Default network bandwidth (kbps)
    pub network_kbps: u32,

    /// Default GPU allocation (fraction, 0-1)
    pub gpu_fraction: f64,

    /// Default attention pool allocation
    pub attention_pool_size: u32,
}

/// Resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum CPU per instance (millicores)
    pub max_cpu_millicores: u32,

    /// Maximum memory per instance (megabytes)
    pub max_memory_mb: u32,

    /// Maximum storage per instance (megabytes)
    pub max_storage_mb: u32,

    /// Maximum total CPU across all instances (millicores)
    pub total_cpu_millicores: u32,

    /// Maximum total memory across all instances (megabytes)
    pub total_memory_mb: u32,

    /// Maximum network bandwidth per instance (kbps)
    pub max_network_kbps: u32,

    /// Resource request to limit ratio
    pub request_limit_ratio: f64,
}

/// Scaling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingConfig {
    /// Enable horizontal scaling
    pub enable_horizontal: bool,

    /// Enable vertical scaling
    pub enable_vertical: bool,

    /// Minimum instances
    pub min_instances: u32,

    /// Maximum instances
    pub max_instances: u32,

    /// Scale-up threshold (resource utilization 0-1)
    pub scale_up_threshold: f64,

    /// Scale-down threshold (resource utilization 0-1)
    pub scale_down_threshold: f64,

    /// Scale-up cooldown (seconds)
    pub scale_up_cooldown_secs: u64,

    /// Scale-down cooldown (seconds)
    pub scale_down_cooldown_secs: u64,

    /// Scaling metric
    pub scaling_metric: ScalingMetric,
}

/// Scaling metric type
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScalingMetric {
    #[default]
    CpuUtilization,
    MemoryUtilization,
    RequestRate,
    QueueDepth,
    AttentionUtilization,
    Custom(String),
}

/// Affinity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffinityConfig {
    /// Required node labels
    pub required_labels: HashMap<String, String>,

    /// Preferred node labels (soft requirement)
    pub preferred_labels: HashMap<String, String>,

    /// Anti-affinity rules (don't co-locate with)
    pub anti_affinity: Vec<String>,

    /// Topology spread constraints
    pub topology_spread: TopologySpread,

    /// Zone awareness
    pub zone_aware: bool,
}

/// Topology spread configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TopologySpread {
    /// Enable topology spread
    pub enabled: bool,

    /// Maximum skew between zones
    pub max_skew: u32,

    /// Topology key (e.g., "zone", "rack")
    pub topology_key: String,

    /// When unsatisfied: DoNotSchedule or ScheduleAnyway
    pub when_unsatisfied: TopologyPolicy,
}

/// Topology spread policy
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TopologyPolicy {
    #[default]
    DoNotSchedule,
    ScheduleAnyway,
}

impl Default for PlatformResourceConfig {
    fn default() -> Self {
        Self {
            defaults: ResourceDefaults {
                cpu_millicores: 500,
                memory_mb: 512,
                storage_mb: 1024,
                network_kbps: 10000,
                gpu_fraction: 0.0,
                attention_pool_size: 100,
            },
            limits: ResourceLimits {
                max_cpu_millicores: 4000,
                max_memory_mb: 8192,
                max_storage_mb: 10240,
                total_cpu_millicores: 32000,
                total_memory_mb: 65536,
                max_network_kbps: 100000,
                request_limit_ratio: 0.8,
            },
            scaling: ScalingConfig {
                enable_horizontal: true,
                enable_vertical: false,
                min_instances: 1,
                max_instances: 10,
                scale_up_threshold: 0.8,
                scale_down_threshold: 0.3,
                scale_up_cooldown_secs: 60,
                scale_down_cooldown_secs: 300,
                scaling_metric: ScalingMetric::CpuUtilization,
            },
            affinity: AffinityConfig {
                required_labels: HashMap::new(),
                preferred_labels: HashMap::new(),
                anti_affinity: vec![],
                topology_spread: TopologySpread::default(),
                zone_aware: false,
            },
        }
    }
}

impl PlatformResourceConfig {
    /// Create resource config for Mapleverse (high resources, aggressive scaling)
    pub fn mapleverse() -> Self {
        Self {
            defaults: ResourceDefaults {
                cpu_millicores: 1000,
                memory_mb: 2048,
                storage_mb: 2048,
                network_kbps: 50000,
                gpu_fraction: 0.0,
                attention_pool_size: 500,
            },
            limits: ResourceLimits {
                max_cpu_millicores: 16000,
                max_memory_mb: 65536,
                max_storage_mb: 102400,
                total_cpu_millicores: 256000,
                total_memory_mb: 524288,
                max_network_kbps: 1000000,
                request_limit_ratio: 0.5, // Allow bursting
            },
            scaling: ScalingConfig {
                enable_horizontal: true,
                enable_vertical: true,
                min_instances: 2,
                max_instances: 500,
                scale_up_threshold: 0.6,
                scale_down_threshold: 0.2,
                scale_up_cooldown_secs: 30,
                scale_down_cooldown_secs: 120,
                scaling_metric: ScalingMetric::RequestRate,
            },
            affinity: AffinityConfig {
                required_labels: HashMap::new(),
                preferred_labels: [("tier".to_string(), "compute".to_string())]
                    .into_iter()
                    .collect(),
                anti_affinity: vec![],
                topology_spread: TopologySpread {
                    enabled: true,
                    max_skew: 3,
                    topology_key: "zone".to_string(),
                    when_unsatisfied: TopologyPolicy::ScheduleAnyway,
                },
                zone_aware: true,
            },
        }
    }

    /// Create resource config for Finalverse (balanced, safety margins)
    pub fn finalverse() -> Self {
        Self {
            defaults: ResourceDefaults {
                cpu_millicores: 1000,
                memory_mb: 4096,
                storage_mb: 4096,
                network_kbps: 20000,
                gpu_fraction: 0.0,
                attention_pool_size: 200,
            },
            limits: ResourceLimits {
                max_cpu_millicores: 8000,
                max_memory_mb: 32768,
                max_storage_mb: 51200,
                total_cpu_millicores: 64000,
                total_memory_mb: 131072,
                max_network_kbps: 500000,
                request_limit_ratio: 0.9, // Conservative bursting
            },
            scaling: ScalingConfig {
                enable_horizontal: true,
                enable_vertical: false,
                min_instances: 2,
                max_instances: 50,
                scale_up_threshold: 0.7,
                scale_down_threshold: 0.4,
                scale_up_cooldown_secs: 120,
                scale_down_cooldown_secs: 600,
                scaling_metric: ScalingMetric::CpuUtilization,
            },
            affinity: AffinityConfig {
                required_labels: [("security".to_string(), "standard".to_string())]
                    .into_iter()
                    .collect(),
                preferred_labels: HashMap::new(),
                anti_affinity: vec!["database".to_string()],
                topology_spread: TopologySpread {
                    enabled: true,
                    max_skew: 1,
                    topology_key: "zone".to_string(),
                    when_unsatisfied: TopologyPolicy::DoNotSchedule,
                },
                zone_aware: true,
            },
        }
    }

    /// Create resource config for iBank (conservative, compliance-focused)
    pub fn ibank() -> Self {
        Self {
            defaults: ResourceDefaults {
                cpu_millicores: 2000,
                memory_mb: 8192,
                storage_mb: 10240,
                network_kbps: 10000,
                gpu_fraction: 0.0,
                attention_pool_size: 100,
            },
            limits: ResourceLimits {
                max_cpu_millicores: 4000,
                max_memory_mb: 16384,
                max_storage_mb: 51200,
                total_cpu_millicores: 32000,
                total_memory_mb: 65536,
                max_network_kbps: 100000,
                request_limit_ratio: 1.0, // No bursting
            },
            scaling: ScalingConfig {
                enable_horizontal: true,
                enable_vertical: false,
                min_instances: 3, // Always maintain redundancy
                max_instances: 20,
                scale_up_threshold: 0.6,
                scale_down_threshold: 0.5, // Narrow band to avoid thrashing
                scale_up_cooldown_secs: 300,
                scale_down_cooldown_secs: 900, // 15 minutes
                scaling_metric: ScalingMetric::CpuUtilization,
            },
            affinity: AffinityConfig {
                required_labels: [
                    ("security".to_string(), "pci-dss".to_string()),
                    ("compliance".to_string(), "financial".to_string()),
                ]
                .into_iter()
                .collect(),
                preferred_labels: HashMap::new(),
                anti_affinity: vec![
                    "database".to_string(),
                    "external".to_string(),
                ],
                topology_spread: TopologySpread {
                    enabled: true,
                    max_skew: 1,
                    topology_key: "rack".to_string(),
                    when_unsatisfied: TopologyPolicy::DoNotSchedule,
                },
                zone_aware: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_resources() {
        let config = PlatformResourceConfig::default();
        assert_eq!(config.defaults.cpu_millicores, 500);
        assert!(config.scaling.enable_horizontal);
    }

    #[test]
    fn test_mapleverse_resources() {
        let config = PlatformResourceConfig::mapleverse();
        assert_eq!(config.scaling.max_instances, 500);
        assert!(config.scaling.enable_vertical);
        assert_eq!(config.limits.request_limit_ratio, 0.5);
    }

    #[test]
    fn test_finalverse_resources() {
        let config = PlatformResourceConfig::finalverse();
        assert!(!config.affinity.required_labels.is_empty());
        assert!(config.affinity.topology_spread.enabled);
    }

    #[test]
    fn test_ibank_resources() {
        let config = PlatformResourceConfig::ibank();
        assert_eq!(config.scaling.min_instances, 3);
        assert_eq!(config.limits.request_limit_ratio, 1.0);
        assert!(config.affinity.required_labels.contains_key("security"));
    }
}
