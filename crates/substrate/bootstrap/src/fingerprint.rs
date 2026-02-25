//! Substrate fingerprint — records the exact external substrate.
//!
//! A fingerprint captures the build environment (rustc version, target triple,
//! OS, CPU architecture, Cargo.lock hash, etc.) so the bootstrap protocol can
//! detect drift and maintain accountability.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::BootstrapResult;

// ── Substrate Fingerprint ───────────────────────────────────────────

/// Fingerprint of the external substrate used to build the system.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstrateFingerprint {
    /// Rust compiler version (e.g., "1.75.0").
    pub rustc_version: String,
    /// Target triple (e.g., "x86_64-unknown-linux-gnu").
    pub target_triple: String,
    /// Operating system name.
    pub os: String,
    /// CPU architecture.
    pub cpu_arch: String,
    /// Hash of Cargo.lock (dependency snapshot).
    pub cargo_lock_hash: String,
    /// Timestamp when this fingerprint was captured.
    pub captured_at: chrono::DateTime<chrono::Utc>,
    /// Enabled feature flags.
    pub features: Vec<String>,
}

impl SubstrateFingerprint {
    /// Detect drift between this fingerprint and another.
    ///
    /// Returns a list of fields that differ.
    pub fn drift_from(&self, other: &Self) -> Vec<String> {
        let mut drifts = Vec::new();
        if self.rustc_version != other.rustc_version {
            drifts.push(format!(
                "rustc_version: {} → {}",
                self.rustc_version, other.rustc_version
            ));
        }
        if self.target_triple != other.target_triple {
            drifts.push(format!(
                "target_triple: {} → {}",
                self.target_triple, other.target_triple
            ));
        }
        if self.os != other.os {
            drifts.push(format!("os: {} → {}", self.os, other.os));
        }
        if self.cpu_arch != other.cpu_arch {
            drifts.push(format!("cpu_arch: {} → {}", self.cpu_arch, other.cpu_arch));
        }
        if self.cargo_lock_hash != other.cargo_lock_hash {
            drifts.push(format!(
                "cargo_lock_hash: {} → {}",
                self.cargo_lock_hash, other.cargo_lock_hash
            ));
        }
        if self.features != other.features {
            drifts.push(format!(
                "features: {:?} → {:?}",
                self.features, other.features
            ));
        }
        drifts
    }

    /// Whether this fingerprint is identical to another (ignoring timestamp).
    pub fn matches(&self, other: &Self) -> bool {
        self.drift_from(other).is_empty()
    }
}

impl std::fmt::Display for SubstrateFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Fingerprint(rustc={}, target={}, os={}, arch={})",
            self.rustc_version, self.target_triple, self.os, self.cpu_arch,
        )
    }
}

// ── Fingerprint Collector Trait ─────────────────────────────────────

/// Trait for collecting substrate fingerprints.
pub trait FingerprintCollector: Send + Sync {
    /// Collect the current substrate fingerprint.
    fn collect(&self) -> BootstrapResult<SubstrateFingerprint>;

    /// Name of this collector.
    fn name(&self) -> &str;
}

/// Simulated fingerprint collector for deterministic testing.
pub struct SimulatedFingerprintCollector {
    fingerprint: SubstrateFingerprint,
}

impl SimulatedFingerprintCollector {
    /// Create with a default simulated fingerprint.
    pub fn new() -> Self {
        Self {
            fingerprint: SubstrateFingerprint {
                rustc_version: "1.75.0".into(),
                target_triple: "x86_64-unknown-linux-gnu".into(),
                os: "linux".into(),
                cpu_arch: "x86_64".into(),
                cargo_lock_hash: "abc123def456".into(),
                captured_at: Utc::now(),
                features: vec!["default".into(), "serde".into()],
            },
        }
    }

    /// Create with a custom fingerprint.
    pub fn with_fingerprint(fingerprint: SubstrateFingerprint) -> Self {
        Self { fingerprint }
    }
}

impl Default for SimulatedFingerprintCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl FingerprintCollector for SimulatedFingerprintCollector {
    fn collect(&self) -> BootstrapResult<SubstrateFingerprint> {
        Ok(self.fingerprint.clone())
    }

    fn name(&self) -> &str {
        "simulated-fingerprint-collector"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_fingerprint() -> SubstrateFingerprint {
        SubstrateFingerprint {
            rustc_version: "1.75.0".into(),
            target_triple: "x86_64-unknown-linux-gnu".into(),
            os: "linux".into(),
            cpu_arch: "x86_64".into(),
            cargo_lock_hash: "abc123".into(),
            captured_at: Utc::now(),
            features: vec!["default".into()],
        }
    }

    #[test]
    fn fingerprint_display() {
        let fp = sample_fingerprint();
        let display = fp.to_string();
        assert!(display.contains("rustc=1.75.0"));
        assert!(display.contains("x86_64"));
    }

    #[test]
    fn fingerprint_matches_identical() {
        let fp1 = sample_fingerprint();
        let fp2 = sample_fingerprint();
        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn fingerprint_drift_rustc() {
        let fp1 = sample_fingerprint();
        let mut fp2 = sample_fingerprint();
        fp2.rustc_version = "1.76.0".into();
        let drifts = fp1.drift_from(&fp2);
        assert_eq!(drifts.len(), 1);
        assert!(drifts[0].contains("rustc_version"));
    }

    #[test]
    fn fingerprint_drift_multiple() {
        let fp1 = sample_fingerprint();
        let mut fp2 = sample_fingerprint();
        fp2.rustc_version = "1.76.0".into();
        fp2.os = "macos".into();
        fp2.cpu_arch = "aarch64".into();
        let drifts = fp1.drift_from(&fp2);
        assert_eq!(drifts.len(), 3);
    }

    #[test]
    fn fingerprint_drift_features() {
        let fp1 = sample_fingerprint();
        let mut fp2 = sample_fingerprint();
        fp2.features.push("gpu".into());
        let drifts = fp1.drift_from(&fp2);
        assert_eq!(drifts.len(), 1);
        assert!(drifts[0].contains("features"));
    }

    #[test]
    fn simulated_collector() {
        let collector = SimulatedFingerprintCollector::new();
        let fp = collector.collect().unwrap();
        assert_eq!(fp.rustc_version, "1.75.0");
        assert_eq!(fp.os, "linux");
    }

    #[test]
    fn simulated_collector_custom() {
        let custom = SubstrateFingerprint {
            rustc_version: "1.80.0".into(),
            target_triple: "aarch64-apple-darwin".into(),
            os: "macos".into(),
            cpu_arch: "aarch64".into(),
            cargo_lock_hash: "xyz789".into(),
            captured_at: Utc::now(),
            features: vec![],
        };
        let collector = SimulatedFingerprintCollector::with_fingerprint(custom.clone());
        let fp = collector.collect().unwrap();
        assert_eq!(fp.rustc_version, "1.80.0");
        assert_eq!(fp.os, "macos");
    }

    #[test]
    fn collector_name() {
        let collector = SimulatedFingerprintCollector::new();
        assert_eq!(collector.name(), "simulated-fingerprint-collector");
    }
}
