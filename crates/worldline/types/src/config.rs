//! Kernel and module deployment configuration.
//!
//! Provides configuration types for selecting which kernel modules are active
//! and tuning operational parameters (WAL policy, clock drift, API binding).

use serde::{Deserialize, Serialize};

/// Deployment profile determining which kernel modules are active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeploymentProfile {
    /// Event Fabric + Gate + Memory only (dev/testing).
    Minimal,
    /// All 7 kernel modules (enterprise).
    Standard,
    /// Standard + financial extensions (banking/DvP).
    Financial,
    /// Standard + air-gapped + HSM (government/sovereign).
    Sovereign,
    /// Multiple kernels with cross-kernel protocols.
    Federated,
}

/// Top-level kernel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelConfig {
    /// Deployment profile controlling active modules.
    pub profile: DeploymentProfile,
    /// Directory for WAL segments and persistent state.
    pub data_dir: String,
    /// REST API bind address.
    pub bind_address: String,
    /// REST API port.
    pub port: u16,
    /// Maximum allowed clock drift in milliseconds.
    pub max_clock_drift_ms: i64,
    /// Whether to fsync every WAL write.
    pub wal_sync_every_write: bool,
    /// Maximum WAL segment size in bytes.
    pub wal_max_segment_bytes: u64,
    /// Event router channel capacity.
    pub router_capacity: usize,
}

impl KernelConfig {
    /// Minimal development configuration.
    pub fn minimal() -> Self {
        Self {
            profile: DeploymentProfile::Minimal,
            data_dir: "/tmp/maple-dev".into(),
            bind_address: "127.0.0.1".into(),
            port: 9100,
            max_clock_drift_ms: 500,
            wal_sync_every_write: false,
            wal_max_segment_bytes: 64 * 1024 * 1024,
            router_capacity: 1024,
        }
    }

    /// Standard production configuration.
    pub fn standard() -> Self {
        Self {
            profile: DeploymentProfile::Standard,
            data_dir: "/var/lib/maple".into(),
            bind_address: "0.0.0.0".into(),
            port: 9100,
            max_clock_drift_ms: 100,
            wal_sync_every_write: true,
            wal_max_segment_bytes: 256 * 1024 * 1024,
            router_capacity: 4096,
        }
    }
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self::minimal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_config() {
        let cfg = KernelConfig::minimal();
        assert_eq!(cfg.profile, DeploymentProfile::Minimal);
        assert!(!cfg.wal_sync_every_write);
        assert_eq!(cfg.port, 9100);
    }

    #[test]
    fn standard_config() {
        let cfg = KernelConfig::standard();
        assert_eq!(cfg.profile, DeploymentProfile::Standard);
        assert!(cfg.wal_sync_every_write);
        assert_eq!(cfg.bind_address, "0.0.0.0");
    }

    #[test]
    fn default_is_minimal() {
        let cfg = KernelConfig::default();
        assert_eq!(cfg.profile, DeploymentProfile::Minimal);
    }

    #[test]
    fn config_serde_roundtrip() {
        let cfg = KernelConfig::standard();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: KernelConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.profile, cfg.profile);
        assert_eq!(back.port, cfg.port);
        assert_eq!(back.wal_sync_every_write, cfg.wal_sync_every_write);
    }

    #[test]
    fn deployment_profile_serde_roundtrip() {
        for profile in [
            DeploymentProfile::Minimal,
            DeploymentProfile::Standard,
            DeploymentProfile::Financial,
            DeploymentProfile::Sovereign,
            DeploymentProfile::Federated,
        ] {
            let json = serde_json::to_string(&profile).unwrap();
            let back: DeploymentProfile = serde_json::from_str(&json).unwrap();
            assert_eq!(profile, back);
        }
    }
}
