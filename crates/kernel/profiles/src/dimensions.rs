use maple_mwl_types::{EffectDomain, RiskClass};
use serde::{Deserialize, Serialize};

/// A WorldLine Profile — the complete behavioral specification for a worldline type.
///
/// Profiles define structural constraints across six dimensions that govern
/// how a worldline can interact with others and affect world-state.
///
/// Per I.PROF-1 (Maximum Restriction Principle): When multiple profiles
/// interact, the most restrictive constraint wins for each dimension.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldlineProfile {
    /// Profile identifier
    pub profile_type: ProfileType,
    /// Human-readable name
    pub name: String,
    /// Description
    pub description: String,
    /// Coupling limits — how strongly this worldline can couple
    pub coupling_limits: CouplingLimits,
    /// Attention budget configuration
    pub attention_budget: AttentionBudgetConfig,
    /// Intent resolution rules
    pub intent_resolution: IntentResolutionRules,
    /// Commitment authority
    pub commitment_authority: CommitmentAuthority,
    /// Consequence scope limits
    pub consequence_scope: ConsequenceScopeLimit,
    /// Human involvement configuration
    pub human_involvement: HumanInvolvementConfig,
}

/// Canonical profile types.
///
/// Five canonical profiles represent the fundamental worldline archetypes.
/// Custom profiles can be created but MUST inherit constraints from at least
/// one canonical profile.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProfileType {
    /// Human worldline — highest agency, strictest safety protections
    Human,
    /// Autonomous agent worldline — bounded autonomy, audit required
    Agent,
    /// Financial worldline — conservative risk, strict auditability
    Financial,
    /// World-state worldline — environmental/contextual, read-heavy
    World,
    /// Coordination worldline — orchestration, high autonomy, bounded scope
    Coordination,
    /// Custom profile inheriting from a canonical base
    Custom {
        name: String,
        base: Box<ProfileType>,
    },
}

impl ProfileType {
    pub fn as_str(&self) -> &str {
        match self {
            ProfileType::Human => "Human",
            ProfileType::Agent => "Agent",
            ProfileType::Financial => "Financial",
            ProfileType::World => "World",
            ProfileType::Coordination => "Coordination",
            ProfileType::Custom { name, .. } => name.as_str(),
        }
    }

    /// Get the canonical base type (follows Custom chain to root).
    pub fn canonical_base(&self) -> &ProfileType {
        match self {
            ProfileType::Custom { base, .. } => base.canonical_base(),
            other => other,
        }
    }
}

/// Coupling limits — structural bounds on interaction strength.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CouplingLimits {
    /// Maximum initial coupling strength (0.0–1.0)
    pub max_initial_strength: f64,
    /// Maximum coupling strength after strengthening (0.0–1.0)
    pub max_sustained_strength: f64,
    /// Maximum rate of coupling strengthening per minute (0.0–1.0)
    pub max_strengthening_rate: f64,
    /// Maximum concurrent couplings
    pub max_concurrent_couplings: u32,
    /// Whether asymmetric coupling is allowed (source stronger than target)
    pub allow_asymmetric: bool,
    /// Required consent level for coupling
    pub consent_required: ConsentLevel,
}

/// Consent requirements for coupling.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConsentLevel {
    /// No explicit consent needed (e.g., world-state observation)
    Implicit,
    /// Notification required but not blocking
    Notify,
    /// Explicit consent required before coupling
    Explicit,
    /// Informed consent with full disclosure of implications
    Informed,
}

/// Attention budget configuration for a profile.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttentionBudgetConfig {
    /// Default attention capacity
    pub default_capacity: u64,
    /// Minimum reserved attention (cannot be allocated)
    pub minimum_reserve: u64,
    /// Maximum fraction any single coupling can consume (0.0–1.0)
    pub max_single_coupling_fraction: f64,
    /// Behavior when budget is exhausted
    pub exhaustion_behavior: ExhaustionBehavior,
}

/// What happens when attention budget is exhausted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExhaustionBehavior {
    /// Block new couplings
    Block,
    /// Queue new couplings
    Queue,
    /// Degrade weakest coupling
    DegradeWeakest,
    /// Emergency decouple all non-essential
    EmergencyDecouple,
}

/// Intent resolution rules — how intents are stabilized and confirmed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntentResolutionRules {
    /// Minimum confidence threshold for intent to be actionable (0.0–1.0)
    pub min_confidence_threshold: f64,
    /// Whether multi-signal confirmation is required
    pub require_multi_signal: bool,
    /// Minimum stabilization window (ms) before intent is considered stable
    pub min_stabilization_ms: u64,
    /// Whether intent can auto-escalate to commitment without review
    pub allow_auto_commitment: bool,
}

/// Commitment authority — what this profile can commit to.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitmentAuthority {
    /// Allowed effect domains
    pub allowed_domains: Vec<EffectDomain>,
    /// Maximum risk class this profile can authorize
    pub max_risk_class: RiskClass,
    /// Whether irreversible commitments are allowed
    pub allow_irreversible: bool,
    /// Maximum number of affected parties per commitment
    pub max_affected_parties: Option<u32>,
    /// Whether audit trail is required for all commitments
    pub require_audit_trail: bool,
    /// Maximum consequence value (arbitrary units, None = unlimited)
    pub max_consequence_value: Option<u64>,
}

/// Consequence scope limits — bounds on world-state effects.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsequenceScopeLimit {
    /// Maximum number of worldlines that can be directly affected
    pub max_direct_affected: Option<u32>,
    /// Maximum cascade depth (how far effects can propagate)
    pub max_cascade_depth: Option<u32>,
    /// Whether cross-domain effects are permitted
    pub allow_cross_domain: bool,
    /// Reversibility preference
    pub reversibility_preference: ReversibilityPreference,
}

/// Profile's preference for reversibility.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReversibilityPreference {
    /// Must be fully reversible
    RequireReversible,
    /// Prefer reversible, allow conditional
    PreferReversible,
    /// Allow time-windowed irreversibility
    AllowTimeWindowed,
    /// Allow all (including irreversible)
    AllowAll,
}

/// Human involvement configuration — when and how humans participate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HumanInvolvementConfig {
    /// Human oversight level
    pub oversight_level: OversightLevel,
    /// Whether human approval is required for high-risk operations
    pub require_human_for_high_risk: bool,
    /// Whether human approval is required for irreversible operations
    pub require_human_for_irreversible: bool,
    /// Whether coercion detection is active
    pub coercion_detection_enabled: bool,
    /// Whether the human agency protocol applies
    pub human_agency_protection: bool,
}

/// Human oversight levels.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OversightLevel {
    /// No human oversight (autonomous world-state)
    None,
    /// Audit trail only — human reviews after the fact
    AuditOnly,
    /// Human is notified of significant actions
    Notification,
    /// Human approves high-risk actions
    ApprovalForHighRisk,
    /// Human approves all non-trivial actions
    FullOversight,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_type_as_str() {
        assert_eq!(ProfileType::Human.as_str(), "Human");
        assert_eq!(ProfileType::Agent.as_str(), "Agent");
        assert_eq!(ProfileType::Financial.as_str(), "Financial");
        assert_eq!(ProfileType::World.as_str(), "World");
        assert_eq!(ProfileType::Coordination.as_str(), "Coordination");
    }

    #[test]
    fn custom_profile_canonical_base() {
        let custom = ProfileType::Custom {
            name: "MyAgent".into(),
            base: Box::new(ProfileType::Agent),
        };
        assert_eq!(*custom.canonical_base(), ProfileType::Agent);

        // Nested custom
        let nested = ProfileType::Custom {
            name: "SpecialAgent".into(),
            base: Box::new(custom),
        };
        assert_eq!(*nested.canonical_base(), ProfileType::Agent);
    }

    #[test]
    fn consent_level_ordering() {
        assert!(ConsentLevel::Implicit < ConsentLevel::Notify);
        assert!(ConsentLevel::Notify < ConsentLevel::Explicit);
        assert!(ConsentLevel::Explicit < ConsentLevel::Informed);
    }

    #[test]
    fn oversight_level_ordering() {
        assert!(OversightLevel::None < OversightLevel::AuditOnly);
        assert!(OversightLevel::AuditOnly < OversightLevel::Notification);
        assert!(OversightLevel::Notification < OversightLevel::ApprovalForHighRisk);
        assert!(OversightLevel::ApprovalForHighRisk < OversightLevel::FullOversight);
    }

    #[test]
    fn reversibility_preference_ordering() {
        assert!(
            ReversibilityPreference::RequireReversible < ReversibilityPreference::PreferReversible
        );
        assert!(
            ReversibilityPreference::PreferReversible < ReversibilityPreference::AllowTimeWindowed
        );
        assert!(ReversibilityPreference::AllowTimeWindowed < ReversibilityPreference::AllowAll);
    }

    #[test]
    fn profile_serialization_roundtrip() {
        let profile_type = ProfileType::Custom {
            name: "TestProfile".into(),
            base: Box::new(ProfileType::Human),
        };
        let json = serde_json::to_string(&profile_type).unwrap();
        let restored: ProfileType = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.as_str(), "TestProfile");
    }

    #[test]
    fn coupling_limits_serialization() {
        let limits = CouplingLimits {
            max_initial_strength: 0.5,
            max_sustained_strength: 0.7,
            max_strengthening_rate: 0.1,
            max_concurrent_couplings: 5,
            allow_asymmetric: false,
            consent_required: ConsentLevel::Explicit,
        };
        let json = serde_json::to_string(&limits).unwrap();
        let restored: CouplingLimits = serde_json::from_str(&json).unwrap();
        assert!((restored.max_initial_strength - 0.5).abs() < f64::EPSILON);
        assert_eq!(restored.consent_required, ConsentLevel::Explicit);
    }

    #[test]
    fn exhaustion_behavior_variants() {
        let behaviors = vec![
            ExhaustionBehavior::Block,
            ExhaustionBehavior::Queue,
            ExhaustionBehavior::DegradeWeakest,
            ExhaustionBehavior::EmergencyDecouple,
        ];
        for b in &behaviors {
            let json = serde_json::to_string(b).unwrap();
            let restored: ExhaustionBehavior = serde_json::from_str(&json).unwrap();
            assert_eq!(*b, restored);
        }
    }
}
