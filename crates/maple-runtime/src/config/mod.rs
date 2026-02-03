//! Configuration for MAPLE Resonance Runtime

pub use crate::types::*;
use serde::{Deserialize, Serialize};

/// Complete runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub profiles: ProfileConfig,
    pub attention: AttentionConfig,
    pub presence: PresenceConfig,
    pub coupling: CouplingConfig,
    pub commitment: CommitmentConfig,
    pub temporal: TemporalConfig,
    pub scheduling: SchedulingConfig,
    pub registry: RegistryConfig,
    pub invariants: InvariantConfig,
    pub safety: SafetyConfig,
    pub telemetry: TelemetryConfig,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            profiles: ProfileConfig::default(),
            attention: AttentionConfig::default(),
            presence: PresenceConfig::default(),
            coupling: CouplingConfig::default(),
            commitment: CommitmentConfig::default(),
            temporal: TemporalConfig::default(),
            scheduling: SchedulingConfig::default(),
            registry: RegistryConfig::default(),
            invariants: InvariantConfig::default(),
            safety: SafetyConfig::default(),
            telemetry: TelemetryConfig::default(),
        }
    }
}

/// Profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub default_profile: ResonatorProfile,
    pub allowed_profiles: Vec<ResonatorProfile>,
    pub human_profiles_allowed: bool,
    pub allow_ibank_profiles: bool,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            default_profile: ResonatorProfile::Coordination,
            allowed_profiles: vec![
                ResonatorProfile::Human,
                ResonatorProfile::World,
                ResonatorProfile::Coordination,
            ],
            human_profiles_allowed: true,
            allow_ibank_profiles: false,
        }
    }
}

/// Presence fabric configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceConfig {
    pub min_signal_interval_ms: u64,
    pub enable_gradient_updates: bool,
}

impl Default for PresenceConfig {
    fn default() -> Self {
        Self {
            min_signal_interval_ms: 1000,
            enable_gradient_updates: true,
        }
    }
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub enable_persistence: bool,
    pub persistence_path: Option<String>,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            enable_persistence: false,
            persistence_path: None,
        }
    }
}

/// Invariant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantConfig {
    pub enabled: bool,
    pub strict_mode: bool,
}

impl Default for InvariantConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strict_mode: true,
        }
    }
}

/// Safety configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub human_agency_protection: bool,
    pub commitment_accountability: bool,
    pub strict_invariants: bool,
    pub coercion_detection: bool,
    pub emotional_exploitation_prevention: bool,
    pub audit_all_commitments: bool,
    pub risk_bounded_consequences: bool,
    pub reversibility_preference: ReversibilityPreference,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            human_agency_protection: true,
            commitment_accountability: true,
            strict_invariants: true,
            coercion_detection: false,
            emotional_exploitation_prevention: false,
            audit_all_commitments: false,
            risk_bounded_consequences: false,
            reversibility_preference: ReversibilityPreference::None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ReversibilityPreference {
    None,
    PreferReversible,
    RequireReversible,
}

/// Scheduling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingConfig {
    pub max_queue_size: usize,
    pub enable_circuit_breakers: bool,
    pub circuit_breaker_threshold: u32,
    pub worker_count: usize,
}

impl Default for SchedulingConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 10000,
            enable_circuit_breakers: true,
            circuit_breaker_threshold: 5,
            worker_count: 4,
        }
    }
}

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub metrics_enabled: bool,
    pub tracing_enabled: bool,
    pub detailed_metrics: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            metrics_enabled: true,
            tracing_enabled: true,
            detailed_metrics: false,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// PLATFORM-SPECIFIC CONFIGURATIONS
// ═══════════════════════════════════════════════════════════════════

/// Mapleverse configuration (Pure AI Agents)
pub fn mapleverse_runtime_config() -> RuntimeConfig {
    RuntimeConfig {
        profiles: ProfileConfig {
            default_profile: ResonatorProfile::Coordination,
            allowed_profiles: vec![ResonatorProfile::Coordination, ResonatorProfile::World],
            // No human profiles in pure AI Mapleverse
            human_profiles_allowed: false,
            allow_ibank_profiles: false,
        },
        attention: AttentionConfig {
            default_capacity: 10000,
            allow_unlimited: false,
            exhaustion_behavior: ExhaustionBehavior::GracefulDegrade,
            enable_rebalancing: true,
        },
        safety: SafetyConfig {
            // AI-only, so different safety profile
            human_agency_protection: false,
            commitment_accountability: true,
            strict_invariants: true,
            coercion_detection: false,
            emotional_exploitation_prevention: false,
            audit_all_commitments: false,
            risk_bounded_consequences: false,
            reversibility_preference: ReversibilityPreference::None,
        },
        ..Default::default()
    }
}

/// Finalverse configuration (Human-AI Coexistence)
pub fn finalverse_runtime_config() -> RuntimeConfig {
    RuntimeConfig {
        profiles: ProfileConfig {
            default_profile: ResonatorProfile::World,
            allowed_profiles: vec![
                ResonatorProfile::Human,
                ResonatorProfile::World,
                ResonatorProfile::Coordination,
            ],
            human_profiles_allowed: true,
            allow_ibank_profiles: false,
        },
        safety: SafetyConfig {
            // Human-facing requires strict agency protection
            human_agency_protection: true,
            commitment_accountability: true,
            strict_invariants: true,
            // Extra protections for experiential environment
            coercion_detection: true,
            emotional_exploitation_prevention: true,
            audit_all_commitments: false,
            risk_bounded_consequences: true,
            reversibility_preference: ReversibilityPreference::PreferReversible,
        },
        ..Default::default()
    }
}

/// iBank configuration (Autonomous Finance)
pub fn ibank_runtime_config() -> RuntimeConfig {
    RuntimeConfig {
        profiles: ProfileConfig {
            default_profile: ResonatorProfile::IBank,
            allowed_profiles: vec![ResonatorProfile::IBank],
            // No human Resonators in iBank (AI-only)
            human_profiles_allowed: false,
            allow_ibank_profiles: true,
        },
        safety: SafetyConfig {
            human_agency_protection: false,  // No humans
            commitment_accountability: true, // Critical for finance
            strict_invariants: true,
            coercion_detection: false,
            emotional_exploitation_prevention: false,
            // Financial-specific
            audit_all_commitments: true,
            risk_bounded_consequences: true,
            reversibility_preference: ReversibilityPreference::PreferReversible,
        },
        commitment: CommitmentConfig {
            // Strict financial accountability
            require_audit_trail: true,
            require_risk_assessment: true,
            max_consequence_value: Some(MonetaryValue::new(1_000_000)), // Limit autonomous decisions
            allow_revocation: true,
            require_consent_for_revocation: true,
        },
        ..Default::default()
    }
}
