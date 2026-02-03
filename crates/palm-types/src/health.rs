//! Health monitoring types
//!
//! Multi-dimensional health assessment for Resonance-native deployments.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Result of a health probe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    /// Probe type that was executed
    pub probe_type: String,

    /// Whether the probe succeeded
    pub success: bool,

    /// Probe latency
    #[serde(with = "duration_serde")]
    pub latency: Duration,

    /// Detailed probe information
    pub details: Option<ProbeDetails>,

    /// Timestamp of the probe
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Detailed probe results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProbeDetails {
    /// Presence-based probe details
    Presence {
        discoverability: f64,
        responsiveness: f64,
        stability: f64,
        coupling_readiness: f64,
    },

    /// Coupling-based probe details
    Coupling {
        available_slots: u32,
        current_couplings: u32,
    },

    /// Attention-based probe details
    Attention {
        total: u64,
        available: u64,
        allocated: u64,
    },

    /// Custom probe details
    Custom { data: serde_json::Value },
}

/// Aggregated health assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAssessment {
    /// Overall health score (0.0 to 1.0)
    pub overall_score: f64,

    /// Individual dimension scores
    pub dimensions: HealthDimensions,

    /// Active alerts
    pub alerts: Vec<HealthAlert>,

    /// Assessment timestamp
    pub assessed_at: chrono::DateTime<chrono::Utc>,
}

/// Health dimensions (multi-dimensional, not binary)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HealthDimensions {
    /// Presence health (0.0 to 1.0)
    pub presence: f64,

    /// Coupling health (0.0 to 1.0)
    pub coupling: f64,

    /// Attention health (0.0 to 1.0)
    pub attention: f64,

    /// Commitment fulfillment rate (0.0 to 1.0)
    pub commitment_fulfillment: f64,

    /// Resource health (0.0 to 1.0)
    pub resources: f64,
}

/// Health alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAlert {
    /// Alert severity
    pub severity: AlertSeverity,

    /// Alert category
    pub category: AlertCategory,

    /// Alert message
    pub message: String,

    /// First occurrence
    pub first_seen: chrono::DateTime<chrono::Utc>,

    /// Occurrence count
    pub count: u32,
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Alert categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertCategory {
    Presence,
    Coupling,
    Attention,
    Commitment,
    Resource,
    Policy,
}

/// Serde helper for Duration
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}
