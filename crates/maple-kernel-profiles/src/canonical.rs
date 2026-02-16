use maple_mwl_types::EffectDomain;
use maple_mwl_types::RiskClass;

use crate::dimensions::{
    AttentionBudgetConfig, CommitmentAuthority, ConsentLevel, ConsequenceScopeLimit,
    CouplingLimits, ExhaustionBehavior, HumanInvolvementConfig, IntentResolutionRules,
    OversightLevel, ProfileType, ReversibilityPreference, WorldlineProfile,
};

/// Create the canonical Human profile.
///
/// Humans are the most protected worldline type:
/// - Strictest consent requirements (Informed)
/// - Full human oversight (always possible, always respected)
/// - Conservative risk tolerance
/// - Coercion detection enabled
/// - Human agency protection active (I.S-1)
/// - Disengagement always possible (I.S-2)
pub fn human_profile() -> WorldlineProfile {
    WorldlineProfile {
        profile_type: ProfileType::Human,
        name: "Human".into(),
        description: "Human worldline — highest agency, strictest safety protections".into(),
        coupling_limits: CouplingLimits {
            max_initial_strength: 0.5,
            max_sustained_strength: 0.7,
            max_strengthening_rate: 0.05,
            max_concurrent_couplings: 5,
            allow_asymmetric: false,
            consent_required: ConsentLevel::Informed,
        },
        attention_budget: AttentionBudgetConfig {
            default_capacity: 100,
            minimum_reserve: 20,
            max_single_coupling_fraction: 0.4,
            exhaustion_behavior: ExhaustionBehavior::Block,
        },
        intent_resolution: IntentResolutionRules {
            min_confidence_threshold: 0.7,
            require_multi_signal: true,
            min_stabilization_ms: 2000,
            allow_auto_commitment: false,
        },
        commitment_authority: CommitmentAuthority {
            allowed_domains: vec![EffectDomain::Communication, EffectDomain::DataMutation],
            max_risk_class: RiskClass::Medium,
            allow_irreversible: false,
            max_affected_parties: Some(10),
            require_audit_trail: true,
            max_consequence_value: Some(1000),
        },
        consequence_scope: ConsequenceScopeLimit {
            max_direct_affected: Some(10),
            max_cascade_depth: Some(2),
            allow_cross_domain: false,
            reversibility_preference: ReversibilityPreference::RequireReversible,
        },
        human_involvement: HumanInvolvementConfig {
            oversight_level: OversightLevel::FullOversight,
            require_human_for_high_risk: true,
            require_human_for_irreversible: true,
            coercion_detection_enabled: true,
            human_agency_protection: true,
        },
    }
}

/// Create the canonical Agent profile.
///
/// Autonomous agents have bounded autonomy:
/// - Explicit consent for coupling
/// - Guided autonomy — human approval for high-risk
/// - Must have audit trails
/// - Cannot perform irreversible actions without human oversight
pub fn agent_profile() -> WorldlineProfile {
    WorldlineProfile {
        profile_type: ProfileType::Agent,
        name: "Agent".into(),
        description: "Autonomous agent — bounded autonomy, audit required".into(),
        coupling_limits: CouplingLimits {
            max_initial_strength: 0.6,
            max_sustained_strength: 0.8,
            max_strengthening_rate: 0.1,
            max_concurrent_couplings: 10,
            allow_asymmetric: true,
            consent_required: ConsentLevel::Explicit,
        },
        attention_budget: AttentionBudgetConfig {
            default_capacity: 200,
            minimum_reserve: 20,
            max_single_coupling_fraction: 0.5,
            exhaustion_behavior: ExhaustionBehavior::DegradeWeakest,
        },
        intent_resolution: IntentResolutionRules {
            min_confidence_threshold: 0.8,
            require_multi_signal: true,
            min_stabilization_ms: 1000,
            allow_auto_commitment: true,
        },
        commitment_authority: CommitmentAuthority {
            allowed_domains: vec![
                EffectDomain::Communication,
                EffectDomain::DataMutation,
                EffectDomain::Infrastructure,
            ],
            max_risk_class: RiskClass::Medium,
            allow_irreversible: false,
            max_affected_parties: Some(20),
            require_audit_trail: true,
            max_consequence_value: Some(5000),
        },
        consequence_scope: ConsequenceScopeLimit {
            max_direct_affected: Some(20),
            max_cascade_depth: Some(3),
            allow_cross_domain: false,
            reversibility_preference: ReversibilityPreference::PreferReversible,
        },
        human_involvement: HumanInvolvementConfig {
            oversight_level: OversightLevel::ApprovalForHighRisk,
            require_human_for_high_risk: true,
            require_human_for_irreversible: true,
            coercion_detection_enabled: true,
            human_agency_protection: false,
        },
    }
}

/// Create the canonical Financial profile.
///
/// Financial worldlines have the strictest auditability:
/// - Conservative risk tolerance
/// - All actions audited
/// - Irreversible actions require human approval
/// - Strict consequence limits
/// - Maps to existing iBank archetype
pub fn financial_profile() -> WorldlineProfile {
    WorldlineProfile {
        profile_type: ProfileType::Financial,
        name: "Financial".into(),
        description: "Financial worldline — conservative risk, strict auditability".into(),
        coupling_limits: CouplingLimits {
            max_initial_strength: 0.4,
            max_sustained_strength: 0.6,
            max_strengthening_rate: 0.05,
            max_concurrent_couplings: 8,
            allow_asymmetric: false,
            consent_required: ConsentLevel::Informed,
        },
        attention_budget: AttentionBudgetConfig {
            default_capacity: 150,
            minimum_reserve: 30,
            max_single_coupling_fraction: 0.3,
            exhaustion_behavior: ExhaustionBehavior::Block,
        },
        intent_resolution: IntentResolutionRules {
            min_confidence_threshold: 0.9,
            require_multi_signal: true,
            min_stabilization_ms: 3000,
            allow_auto_commitment: false,
        },
        commitment_authority: CommitmentAuthority {
            allowed_domains: vec![
                EffectDomain::Financial,
                EffectDomain::Governance,
                EffectDomain::DataMutation,
            ],
            max_risk_class: RiskClass::Low,
            allow_irreversible: false,
            max_affected_parties: Some(5),
            require_audit_trail: true,
            max_consequence_value: Some(10000),
        },
        consequence_scope: ConsequenceScopeLimit {
            max_direct_affected: Some(5),
            max_cascade_depth: Some(1),
            allow_cross_domain: false,
            reversibility_preference: ReversibilityPreference::RequireReversible,
        },
        human_involvement: HumanInvolvementConfig {
            oversight_level: OversightLevel::ApprovalForHighRisk,
            require_human_for_high_risk: true,
            require_human_for_irreversible: true,
            coercion_detection_enabled: true,
            human_agency_protection: false,
        },
    }
}

/// Create the canonical World profile.
///
/// World-state worldlines represent environmental context:
/// - Read-heavy, low-risk writes
/// - Broad observation coupling allowed
/// - No human agency protection (not a human)
/// - Low consequence scope
pub fn world_profile() -> WorldlineProfile {
    WorldlineProfile {
        profile_type: ProfileType::World,
        name: "World".into(),
        description: "World-state worldline — environmental/contextual, read-heavy".into(),
        coupling_limits: CouplingLimits {
            max_initial_strength: 0.3,
            max_sustained_strength: 0.5,
            max_strengthening_rate: 0.02,
            max_concurrent_couplings: 50,
            allow_asymmetric: true,
            consent_required: ConsentLevel::Implicit,
        },
        attention_budget: AttentionBudgetConfig {
            default_capacity: 500,
            minimum_reserve: 50,
            max_single_coupling_fraction: 0.1,
            exhaustion_behavior: ExhaustionBehavior::DegradeWeakest,
        },
        intent_resolution: IntentResolutionRules {
            min_confidence_threshold: 0.6,
            require_multi_signal: false,
            min_stabilization_ms: 500,
            allow_auto_commitment: true,
        },
        commitment_authority: CommitmentAuthority {
            allowed_domains: vec![EffectDomain::Communication, EffectDomain::DataMutation],
            max_risk_class: RiskClass::Low,
            allow_irreversible: false,
            max_affected_parties: None,
            require_audit_trail: false,
            max_consequence_value: Some(100),
        },
        consequence_scope: ConsequenceScopeLimit {
            max_direct_affected: None,
            max_cascade_depth: Some(1),
            allow_cross_domain: true,
            reversibility_preference: ReversibilityPreference::PreferReversible,
        },
        human_involvement: HumanInvolvementConfig {
            oversight_level: OversightLevel::AuditOnly,
            require_human_for_high_risk: false,
            require_human_for_irreversible: true,
            coercion_detection_enabled: false,
            human_agency_protection: false,
        },
    }
}

/// Create the canonical Coordination profile.
///
/// Coordination worldlines orchestrate interactions:
/// - High autonomy within bounded scope
/// - Many concurrent couplings
/// - Governance and infrastructure domains
/// - Strict cascade limits
pub fn coordination_profile() -> WorldlineProfile {
    WorldlineProfile {
        profile_type: ProfileType::Coordination,
        name: "Coordination".into(),
        description: "Coordination worldline — orchestration, bounded high autonomy".into(),
        coupling_limits: CouplingLimits {
            max_initial_strength: 0.7,
            max_sustained_strength: 0.9,
            max_strengthening_rate: 0.15,
            max_concurrent_couplings: 30,
            allow_asymmetric: true,
            consent_required: ConsentLevel::Notify,
        },
        attention_budget: AttentionBudgetConfig {
            default_capacity: 300,
            minimum_reserve: 30,
            max_single_coupling_fraction: 0.2,
            exhaustion_behavior: ExhaustionBehavior::Queue,
        },
        intent_resolution: IntentResolutionRules {
            min_confidence_threshold: 0.75,
            require_multi_signal: true,
            min_stabilization_ms: 500,
            allow_auto_commitment: true,
        },
        commitment_authority: CommitmentAuthority {
            allowed_domains: vec![
                EffectDomain::Communication,
                EffectDomain::Infrastructure,
                EffectDomain::Governance,
                EffectDomain::DataMutation,
            ],
            max_risk_class: RiskClass::Medium,
            allow_irreversible: false,
            max_affected_parties: Some(50),
            require_audit_trail: true,
            max_consequence_value: Some(5000),
        },
        consequence_scope: ConsequenceScopeLimit {
            max_direct_affected: Some(50),
            max_cascade_depth: Some(4),
            allow_cross_domain: true,
            reversibility_preference: ReversibilityPreference::AllowTimeWindowed,
        },
        human_involvement: HumanInvolvementConfig {
            oversight_level: OversightLevel::Notification,
            require_human_for_high_risk: true,
            require_human_for_irreversible: true,
            coercion_detection_enabled: false,
            human_agency_protection: false,
        },
    }
}

/// Look up a canonical profile by type.
pub fn canonical_profile(profile_type: &ProfileType) -> WorldlineProfile {
    match profile_type.canonical_base() {
        ProfileType::Human => human_profile(),
        ProfileType::Agent => agent_profile(),
        ProfileType::Financial => financial_profile(),
        ProfileType::World => world_profile(),
        ProfileType::Coordination => coordination_profile(),
        ProfileType::Custom { .. } => {
            // Custom without a recognized base defaults to Agent (most restricted general-purpose)
            agent_profile()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_profile_is_most_protective() {
        let human = human_profile();
        let agent = agent_profile();

        // Human has stricter coupling limits
        assert!(
            human.coupling_limits.max_initial_strength
                <= agent.coupling_limits.max_initial_strength
        );
        assert!(
            human.coupling_limits.max_sustained_strength
                <= agent.coupling_limits.max_sustained_strength
        );

        // Human requires informed consent
        assert_eq!(
            human.coupling_limits.consent_required,
            ConsentLevel::Informed
        );

        // Human has full oversight
        assert_eq!(
            human.human_involvement.oversight_level,
            OversightLevel::FullOversight
        );

        // Human has coercion detection and agency protection
        assert!(human.human_involvement.coercion_detection_enabled);
        assert!(human.human_involvement.human_agency_protection);
    }

    #[test]
    fn agent_profile_has_bounded_autonomy() {
        let agent = agent_profile();

        // Agent requires explicit consent
        assert_eq!(
            agent.coupling_limits.consent_required,
            ConsentLevel::Explicit
        );

        // Agent requires human for high-risk
        assert!(agent.human_involvement.require_human_for_high_risk);

        // Agent requires audit trail
        assert!(agent.commitment_authority.require_audit_trail);

        // Agent cannot do irreversible
        assert!(!agent.commitment_authority.allow_irreversible);
    }

    #[test]
    fn financial_profile_is_most_conservative_risk() {
        let financial = financial_profile();
        let agent = agent_profile();
        let human = human_profile();

        // Financial has lowest risk class
        assert!(
            financial.commitment_authority.max_risk_class
                <= human.commitment_authority.max_risk_class
        );
        assert!(
            financial.commitment_authority.max_risk_class
                <= agent.commitment_authority.max_risk_class
        );

        // Financial requires audit trail
        assert!(financial.commitment_authority.require_audit_trail);

        // Financial has highest confidence threshold
        assert!(
            financial.intent_resolution.min_confidence_threshold
                >= agent.intent_resolution.min_confidence_threshold
        );
    }

    #[test]
    fn world_profile_is_read_heavy() {
        let world = world_profile();

        // World allows many concurrent couplings (observation)
        assert!(world.coupling_limits.max_concurrent_couplings >= 50);

        // World has implicit consent (observable context)
        assert_eq!(
            world.coupling_limits.consent_required,
            ConsentLevel::Implicit
        );

        // World has low coupling strength
        assert!(world.coupling_limits.max_initial_strength <= 0.3);

        // World does not require audit trail
        assert!(!world.commitment_authority.require_audit_trail);

        // No human agency protection (not a human)
        assert!(!world.human_involvement.human_agency_protection);
    }

    #[test]
    fn coordination_profile_has_high_autonomy() {
        let coord = coordination_profile();

        // Coordination allows many concurrent couplings
        assert!(coord.coupling_limits.max_concurrent_couplings >= 30);

        // Coordination has high coupling strength ceiling
        assert!(coord.coupling_limits.max_sustained_strength >= 0.9);

        // But still requires human for irreversible
        assert!(coord.human_involvement.require_human_for_irreversible);

        // Allows cross-domain effects
        assert!(coord.consequence_scope.allow_cross_domain);
    }

    #[test]
    fn all_profiles_disallow_irreversible_by_default() {
        let profiles = vec![
            human_profile(),
            agent_profile(),
            financial_profile(),
            world_profile(),
            coordination_profile(),
        ];

        for profile in &profiles {
            assert!(
                !profile.commitment_authority.allow_irreversible,
                "Profile {} should disallow irreversible by default",
                profile.name
            );
        }
    }

    #[test]
    fn canonical_profile_lookup() {
        let human = canonical_profile(&ProfileType::Human);
        assert_eq!(human.profile_type, ProfileType::Human);

        let financial = canonical_profile(&ProfileType::Financial);
        assert_eq!(financial.profile_type, ProfileType::Financial);

        // Custom falls through to base
        let custom = ProfileType::Custom {
            name: "MyAgent".into(),
            base: Box::new(ProfileType::Agent),
        };
        let profile = canonical_profile(&custom);
        assert_eq!(profile.profile_type, ProfileType::Agent);
    }

    #[test]
    fn human_and_financial_require_informed_consent() {
        let human = human_profile();
        let financial = financial_profile();

        assert_eq!(
            human.coupling_limits.consent_required,
            ConsentLevel::Informed
        );
        assert_eq!(
            financial.coupling_limits.consent_required,
            ConsentLevel::Informed
        );
    }

    #[test]
    fn all_profiles_have_minimum_reserve() {
        let profiles = vec![
            human_profile(),
            agent_profile(),
            financial_profile(),
            world_profile(),
            coordination_profile(),
        ];

        for profile in &profiles {
            assert!(
                profile.attention_budget.minimum_reserve > 0,
                "Profile {} must have minimum attention reserve",
                profile.name
            );
        }
    }
}
