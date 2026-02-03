//! Health probes for Resonator instances.
//!
//! Probes measure the multi-dimensional health of Resonator instances:
//! - Presence: Is the agent "present" in the resonance field?
//! - Coupling: Can the agent couple with other agents?
//! - Attention: Does the agent have attention budget remaining?
//!
//! These are Resonance-native concepts, not traditional liveness/readiness probes.

mod attention;
mod coupling;
mod custom;
mod presence;

pub use attention::AttentionProbe;
pub use coupling::CouplingProbe;
pub use custom::{CustomProbe, CustomProbeFactory};
pub use presence::PresenceProbe;

use async_trait::async_trait;
use palm_types::InstanceId;
use serde::{Deserialize, Serialize};

use crate::error::HealthResult;

/// Result of a probe execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    /// Instance that was probed.
    pub instance_id: InstanceId,

    /// Type of probe that was executed.
    pub probe_type: ProbeType,

    /// Whether the probe succeeded.
    pub success: bool,

    /// Measured value (0.0-1.0), if applicable.
    pub value: Option<f64>,

    /// Latency of the probe in milliseconds.
    pub latency_ms: u64,

    /// Optional message with details.
    pub message: Option<String>,

    /// Timestamp of the probe.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ProbeResult {
    /// Create a successful probe result.
    pub fn success(
        instance_id: InstanceId,
        probe_type: ProbeType,
        value: f64,
        latency_ms: u64,
    ) -> Self {
        Self {
            instance_id,
            probe_type,
            success: true,
            value: Some(value),
            latency_ms,
            message: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a failed probe result.
    pub fn failure(
        instance_id: InstanceId,
        probe_type: ProbeType,
        message: impl Into<String>,
        latency_ms: u64,
    ) -> Self {
        Self {
            instance_id,
            probe_type,
            success: false,
            value: None,
            latency_ms,
            message: Some(message.into()),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a timeout probe result.
    pub fn timeout(instance_id: InstanceId, probe_type: ProbeType, timeout_ms: u64) -> Self {
        Self {
            instance_id,
            probe_type,
            success: false,
            value: None,
            latency_ms: timeout_ms,
            message: Some(format!("Probe timed out after {}ms", timeout_ms)),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Type of health probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProbeType {
    /// Presence gradient probe.
    Presence,
    /// Coupling capacity probe.
    Coupling,
    /// Attention budget probe.
    Attention,
    /// Custom application-defined probe.
    Custom,
}

impl std::fmt::Display for ProbeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbeType::Presence => write!(f, "presence"),
            ProbeType::Coupling => write!(f, "coupling"),
            ProbeType::Attention => write!(f, "attention"),
            ProbeType::Custom => write!(f, "custom"),
        }
    }
}

/// Trait for health probes.
#[async_trait]
pub trait Probe: Send + Sync {
    /// Get the probe type.
    fn probe_type(&self) -> ProbeType;

    /// Execute the probe against an instance.
    async fn execute(&self, instance_id: InstanceId) -> HealthResult<ProbeResult>;

    /// Get the probe name for logging.
    fn name(&self) -> &str {
        match self.probe_type() {
            ProbeType::Presence => "presence",
            ProbeType::Coupling => "coupling",
            ProbeType::Attention => "attention",
            ProbeType::Custom => "custom",
        }
    }
}

/// Collection of probes for an instance.
pub struct ProbeSet {
    probes: Vec<Box<dyn Probe>>,
}

impl ProbeSet {
    /// Create a new empty probe set.
    pub fn new() -> Self {
        Self { probes: Vec::new() }
    }

    /// Create a default probe set with all standard probes.
    pub fn default_set() -> Self {
        let mut set = Self::new();
        set.add_probe(Box::new(PresenceProbe::new()));
        set.add_probe(Box::new(CouplingProbe::new()));
        set.add_probe(Box::new(AttentionProbe::new()));
        set
    }

    /// Add a probe to the set.
    pub fn add_probe(&mut self, probe: Box<dyn Probe>) {
        self.probes.push(probe);
    }

    /// Get all probes.
    pub fn probes(&self) -> &[Box<dyn Probe>] {
        &self.probes
    }

    /// Execute all probes and return results.
    pub async fn execute_all(&self, instance_id: InstanceId) -> Vec<HealthResult<ProbeResult>> {
        let mut results = Vec::with_capacity(self.probes.len());

        for probe in &self.probes {
            results.push(probe.execute(instance_id.clone()).await);
        }

        results
    }
}

impl Default for ProbeSet {
    fn default() -> Self {
        Self::default_set()
    }
}
