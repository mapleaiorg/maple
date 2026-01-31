//! Agent specifications for deployment
//!
//! An AgentSpec defines what to deploy - the template for creating instances.

use crate::{AgentSpecId, PlatformProfile};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Specification for an agent deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    /// Unique identifier for this spec
    pub id: AgentSpecId,

    /// Human-readable name
    pub name: String,

    /// Semantic version
    pub version: semver::Version,

    /// Target platform
    pub platform: PlatformProfile,

    /// Resonator profile configuration
    pub resonator_profile: ResonatorProfileConfig,

    /// Resource requirements
    pub resources: ResourceRequirements,

    /// Health check configuration
    pub health: HealthConfig,

    /// Capability requirements
    pub capabilities: Vec<CapabilityRef>,

    /// Environment variables
    pub env: HashMap<String, String>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,

    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl AgentSpec {
    /// Create a new agent spec with defaults
    pub fn new(name: impl Into<String>, version: semver::Version) -> Self {
        Self {
            id: AgentSpecId::generate(),
            name: name.into(),
            version,
            platform: PlatformProfile::default(),
            resonator_profile: ResonatorProfileConfig::default(),
            resources: ResourceRequirements::default(),
            health: HealthConfig::default(),
            capabilities: Vec::new(),
            env: HashMap::new(),
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
        }
    }

    /// Validate the spec
    pub fn validate(&self) -> Result<(), SpecValidationError> {
        if self.name.is_empty() {
            return Err(SpecValidationError::EmptyName);
        }

        if self.resources.attention_budget == 0 {
            return Err(SpecValidationError::InvalidResourceRequirements(
                "attention_budget must be > 0".into(),
            ));
        }

        if self.resources.coupling_slots == 0 {
            return Err(SpecValidationError::InvalidResourceRequirements(
                "coupling_slots must be > 0".into(),
            ));
        }

        Ok(())
    }
}

/// Resonator profile configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResonatorProfileConfig {
    /// Profile type (Human, Coordination, etc.)
    pub profile_type: String,

    /// Risk tolerance level
    pub risk_tolerance: RiskTolerance,

    /// Autonomy level
    pub autonomy_level: AutonomyLevel,

    /// Additional profile parameters
    pub parameters: HashMap<String, String>,
}

/// Risk tolerance levels
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskTolerance {
    Conservative,
    #[default]
    Balanced,
    Aggressive,
}

/// Autonomy levels
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutonomyLevel {
    /// All commitments require human approval
    FullHumanOversight,
    /// Low-risk commitments can be auto-approved
    #[default]
    GuidedAutonomy,
    /// Only high-risk commitments need review
    HighAutonomy,
}

/// Resource requirements for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// Attention budget (finite capacity for resonance)
    pub attention_budget: u64,

    /// Maximum coupling slots
    pub coupling_slots: u32,

    /// Memory limit in bytes (optional)
    pub memory_limit: Option<u64>,

    /// CPU limit as millicores (optional)
    pub cpu_limit: Option<u32>,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            attention_budget: 10000,
            coupling_slots: 100,
            memory_limit: None,
            cpu_limit: None,
        }
    }
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Readiness probe configuration
    pub readiness: ProbeConfig,

    /// Liveness probe configuration
    pub liveness: ProbeConfig,

    /// Startup probe configuration (optional)
    pub startup: Option<ProbeConfig>,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            readiness: ProbeConfig {
                probe_type: ProbeType::PresenceGradient { min_score: 0.5 },
                initial_delay: Duration::from_secs(5),
                period: Duration::from_secs(10),
                timeout: Duration::from_secs(5),
                success_threshold: 1,
                failure_threshold: 3,
            },
            liveness: ProbeConfig {
                probe_type: ProbeType::PresenceGradient { min_score: 0.3 },
                initial_delay: Duration::from_secs(10),
                period: Duration::from_secs(30),
                timeout: Duration::from_secs(10),
                success_threshold: 1,
                failure_threshold: 3,
            },
            startup: None,
        }
    }
}

/// Configuration for a health probe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeConfig {
    /// Type of probe to execute
    pub probe_type: ProbeType,

    /// Initial delay before first probe
    #[serde(with = "duration_serde")]
    pub initial_delay: Duration,

    /// Time between probes
    #[serde(with = "duration_serde")]
    pub period: Duration,

    /// Probe timeout
    #[serde(with = "duration_serde")]
    pub timeout: Duration,

    /// Minimum consecutive successes for healthy
    pub success_threshold: u32,

    /// Minimum consecutive failures for unhealthy
    pub failure_threshold: u32,
}

/// Types of health probes (Resonance-native)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProbeType {
    /// Check presence gradient (discoverability, responsiveness, stability)
    PresenceGradient {
        /// Minimum combined presence score (0.0 to 1.0)
        min_score: f64,
    },

    /// Check coupling capacity
    CouplingCapacity {
        /// Minimum available coupling slots
        min_available_slots: u32,
    },

    /// Check attention availability
    AttentionAvailable {
        /// Minimum attention ratio (0.0 to 1.0)
        min_ratio: f64,
    },

    /// Custom probe via endpoint
    Custom {
        /// Endpoint to call
        endpoint: String,
    },
}

/// Reference to a capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRef {
    pub name: String,
    pub version: String,
}

/// Spec validation errors
#[derive(Debug, thiserror::Error)]
pub enum SpecValidationError {
    #[error("Agent name cannot be empty")]
    EmptyName,

    #[error("Invalid resource requirements: {0}")]
    InvalidResourceRequirements(String),

    #[error("Invalid health config: {0}")]
    InvalidHealthConfig(String),
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
