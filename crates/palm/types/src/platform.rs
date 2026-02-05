//! Platform profiles for different deployment contexts
//!
//! PALM supports multiple platforms with different safety requirements:
//! - Mapleverse: Pure AI agents with high autonomy
//! - Finalverse: Human-AI coexistence with strict agency protection
//! - iBank: Autonomous finance with full audit trails

use serde::{Deserialize, Serialize};

/// Platform profile determining deployment constraints and safety requirements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PlatformProfile {
    /// Pure AI agent coordination (Mapleverse)
    ///
    /// Highest autonomy, designed for agent-to-agent interactions
    /// where all participants are AI systems.
    #[default]
    Mapleverse,

    /// Human-AI coexistence (Finalverse)
    ///
    /// Strict human agency protection, meaningful consent,
    /// and safeguards for experiential environments.
    Finalverse,

    /// Autonomous financial operations (iBank)
    ///
    /// Maximum accountability, full audit trails,
    /// and strict reversibility requirements.
    IBank,

    /// Development and testing
    ///
    /// Relaxed constraints for development purposes.
    Development,
}

impl PlatformProfile {
    /// Does this platform require human agency protection?
    pub fn requires_agency_protection(&self) -> bool {
        matches!(self, PlatformProfile::Finalverse | PlatformProfile::IBank)
    }

    /// Does this platform require full audit trails?
    pub fn requires_audit_trail(&self) -> bool {
        matches!(self, PlatformProfile::IBank | PlatformProfile::Finalverse)
    }

    /// Does this platform prefer reversible operations?
    pub fn prefers_reversibility(&self) -> bool {
        matches!(self, PlatformProfile::IBank)
    }

    /// Get the default health check timeout for this platform
    pub fn default_health_timeout(&self) -> std::time::Duration {
        match self {
            PlatformProfile::Mapleverse => std::time::Duration::from_secs(30),
            PlatformProfile::Finalverse => std::time::Duration::from_secs(60),
            PlatformProfile::IBank => std::time::Duration::from_secs(120),
            PlatformProfile::Development => std::time::Duration::from_secs(10),
        }
    }

    /// Get the default replica count for this platform
    pub fn default_replicas(&self) -> u32 {
        match self {
            PlatformProfile::Mapleverse => 3,
            PlatformProfile::Finalverse => 2,
            PlatformProfile::IBank => 5,
            PlatformProfile::Development => 1,
        }
    }

    /// Get the minimum healthy replicas ratio
    pub fn min_healthy_ratio(&self) -> f64 {
        match self {
            PlatformProfile::Mapleverse => 0.5,
            PlatformProfile::Finalverse => 0.75,
            PlatformProfile::IBank => 0.9,
            PlatformProfile::Development => 0.0,
        }
    }
}

impl std::fmt::Display for PlatformProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlatformProfile::Mapleverse => write!(f, "mapleverse"),
            PlatformProfile::Finalverse => write!(f, "finalverse"),
            PlatformProfile::IBank => write!(f, "ibank"),
            PlatformProfile::Development => write!(f, "development"),
        }
    }
}
