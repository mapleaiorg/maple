//! Platform-specific health configuration

use serde::{Deserialize, Serialize};

/// Health monitoring configuration for a platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformHealthConfig {
    /// Health check interval (seconds)
    pub check_interval_secs: u64,

    /// Failure threshold before unhealthy
    pub failure_threshold: u32,

    /// Success threshold before healthy
    pub success_threshold: u32,

    /// Probe configurations
    pub probes: ProbeConfigs,

    /// Resonance-specific health checks
    pub resonance: ResonanceHealthConfig,
}

/// Probe configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeConfigs {
    /// Liveness probe settings
    pub liveness: ProbeSettings,

    /// Readiness probe settings
    pub readiness: ProbeSettings,

    /// Startup probe settings
    pub startup: Option<ProbeSettings>,

    /// Custom probes
    #[serde(default)]
    pub custom: Vec<CustomProbeConfig>,
}

/// Settings for a single probe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeSettings {
    /// Enable this probe
    pub enabled: bool,

    /// Probe timeout (seconds)
    pub timeout_secs: u64,

    /// Period between checks (seconds)
    pub period_secs: u64,

    /// Failure threshold
    pub failure_threshold: u32,

    /// Success threshold
    pub success_threshold: u32,
}

/// Custom probe configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProbeConfig {
    /// Probe name
    pub name: String,
    /// Probe type
    pub probe_type: String,
    /// Probe settings
    pub settings: ProbeSettings,
    /// Additional configuration
    pub config: serde_json::Value,
}

/// Resonance-specific health checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceHealthConfig {
    /// Check presence state
    pub check_presence: bool,

    /// Check coupling health
    pub check_coupling: bool,

    /// Check attention utilization
    pub check_attention: bool,

    /// Check continuity chain
    pub check_continuity: bool,

    /// Attention exhaustion threshold (0-1)
    pub attention_exhaustion_threshold: f64,

    /// Coupling intensity warning threshold
    pub coupling_intensity_warning: f64,
}

impl Default for PlatformHealthConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 30,
            failure_threshold: 3,
            success_threshold: 1,
            probes: ProbeConfigs {
                liveness: ProbeSettings {
                    enabled: true,
                    timeout_secs: 5,
                    period_secs: 10,
                    failure_threshold: 3,
                    success_threshold: 1,
                },
                readiness: ProbeSettings {
                    enabled: true,
                    timeout_secs: 5,
                    period_secs: 10,
                    failure_threshold: 3,
                    success_threshold: 1,
                },
                startup: None,
                custom: vec![],
            },
            resonance: ResonanceHealthConfig {
                check_presence: true,
                check_coupling: true,
                check_attention: true,
                check_continuity: true,
                attention_exhaustion_threshold: 0.9,
                coupling_intensity_warning: 0.8,
            },
        }
    }
}

impl PlatformHealthConfig {
    /// Create health config for Mapleverse (fast checks, high throughput)
    pub fn mapleverse() -> Self {
        Self {
            check_interval_secs: 15,
            failure_threshold: 5,
            success_threshold: 1,
            probes: ProbeConfigs {
                liveness: ProbeSettings {
                    enabled: true,
                    timeout_secs: 2,
                    period_secs: 5,
                    failure_threshold: 5,
                    success_threshold: 1,
                },
                readiness: ProbeSettings {
                    enabled: true,
                    timeout_secs: 2,
                    period_secs: 5,
                    failure_threshold: 3,
                    success_threshold: 1,
                },
                startup: Some(ProbeSettings {
                    enabled: true,
                    timeout_secs: 10,
                    period_secs: 5,
                    failure_threshold: 30,
                    success_threshold: 1,
                }),
                custom: vec![],
            },
            resonance: ResonanceHealthConfig {
                check_presence: true,
                check_coupling: true,
                check_attention: true,
                check_continuity: false, // Skip for speed
                attention_exhaustion_threshold: 0.95,
                coupling_intensity_warning: 0.9,
            },
        }
    }

    /// Create health config for Finalverse (thorough checks)
    pub fn finalverse() -> Self {
        Self {
            check_interval_secs: 30,
            failure_threshold: 2,
            success_threshold: 2,
            probes: ProbeConfigs {
                liveness: ProbeSettings {
                    enabled: true,
                    timeout_secs: 10,
                    period_secs: 15,
                    failure_threshold: 2,
                    success_threshold: 2,
                },
                readiness: ProbeSettings {
                    enabled: true,
                    timeout_secs: 10,
                    period_secs: 15,
                    failure_threshold: 2,
                    success_threshold: 2,
                },
                startup: Some(ProbeSettings {
                    enabled: true,
                    timeout_secs: 30,
                    period_secs: 10,
                    failure_threshold: 10,
                    success_threshold: 3,
                }),
                custom: vec![],
            },
            resonance: ResonanceHealthConfig {
                check_presence: true,
                check_coupling: true,
                check_attention: true,
                check_continuity: true,
                attention_exhaustion_threshold: 0.8,
                coupling_intensity_warning: 0.7,
            },
        }
    }

    /// Create health config for iBank (conservative, audit-friendly)
    pub fn ibank() -> Self {
        Self {
            check_interval_secs: 60,
            failure_threshold: 2,
            success_threshold: 3,
            probes: ProbeConfigs {
                liveness: ProbeSettings {
                    enabled: true,
                    timeout_secs: 15,
                    period_secs: 30,
                    failure_threshold: 2,
                    success_threshold: 3,
                },
                readiness: ProbeSettings {
                    enabled: true,
                    timeout_secs: 15,
                    period_secs: 30,
                    failure_threshold: 2,
                    success_threshold: 3,
                },
                startup: Some(ProbeSettings {
                    enabled: true,
                    timeout_secs: 60,
                    period_secs: 15,
                    failure_threshold: 5,
                    success_threshold: 3,
                }),
                custom: vec![
                    CustomProbeConfig {
                        name: "compliance_check".to_string(),
                        probe_type: "http".to_string(),
                        settings: ProbeSettings {
                            enabled: true,
                            timeout_secs: 30,
                            period_secs: 300,
                            failure_threshold: 1,
                            success_threshold: 1,
                        },
                        config: serde_json::json!({
                            "endpoint": "/health/compliance",
                            "expected_status": 200
                        }),
                    },
                ],
            },
            resonance: ResonanceHealthConfig {
                check_presence: true,
                check_coupling: true,
                check_attention: true,
                check_continuity: true,
                attention_exhaustion_threshold: 0.7,
                coupling_intensity_warning: 0.6,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_health() {
        let config = PlatformHealthConfig::default();
        assert_eq!(config.check_interval_secs, 30);
        assert!(config.probes.liveness.enabled);
    }

    #[test]
    fn test_mapleverse_health() {
        let config = PlatformHealthConfig::mapleverse();
        assert_eq!(config.check_interval_secs, 15);
        assert!(!config.resonance.check_continuity);
    }

    #[test]
    fn test_ibank_health() {
        let config = PlatformHealthConfig::ibank();
        assert!(!config.probes.custom.is_empty());
        assert_eq!(config.probes.custom[0].name, "compliance_check");
    }
}
