use tracing::debug;

use crate::dimensions::{
    AttentionBudgetConfig, CommitmentAuthority, ConsequenceScopeLimit, CouplingLimits,
    ExhaustionBehavior, HumanInvolvementConfig, IntentResolutionRules, WorldlineProfile,
};

/// Merge two profiles using the Maximum Restriction Principle (I.PROF-1).
///
/// When two worldlines with different profiles interact, the resulting
/// constraints are the MOST RESTRICTIVE from each dimension. This ensures
/// that safety guarantees are never weakened by cross-profile interaction.
///
/// Per I.PROF-1: "In any cross-profile interaction, the most restrictive
/// constraint from either profile applies to every dimension."
pub fn merged_constraints(a: &WorldlineProfile, b: &WorldlineProfile) -> WorldlineProfile {
    debug!(
        profile_a = a.name.as_str(),
        profile_b = b.name.as_str(),
        "Merging profiles via Maximum Restriction Principle (I.PROF-1)"
    );

    WorldlineProfile {
        profile_type: a.profile_type.clone(),
        name: format!("{}+{}", a.name, b.name),
        description: format!(
            "Merged profile ({} + {}) via Maximum Restriction Principle",
            a.name, b.name
        ),
        coupling_limits: merge_coupling_limits(&a.coupling_limits, &b.coupling_limits),
        attention_budget: merge_attention_budget(&a.attention_budget, &b.attention_budget),
        intent_resolution: merge_intent_resolution(&a.intent_resolution, &b.intent_resolution),
        commitment_authority: merge_commitment_authority(
            &a.commitment_authority,
            &b.commitment_authority,
        ),
        consequence_scope: merge_consequence_scope(&a.consequence_scope, &b.consequence_scope),
        human_involvement: merge_human_involvement(&a.human_involvement, &b.human_involvement),
    }
}

/// Merge coupling limits: take the more restrictive of each field.
fn merge_coupling_limits(a: &CouplingLimits, b: &CouplingLimits) -> CouplingLimits {
    CouplingLimits {
        // Lower = more restrictive
        max_initial_strength: a.max_initial_strength.min(b.max_initial_strength),
        max_sustained_strength: a.max_sustained_strength.min(b.max_sustained_strength),
        max_strengthening_rate: a.max_strengthening_rate.min(b.max_strengthening_rate),
        max_concurrent_couplings: a.max_concurrent_couplings.min(b.max_concurrent_couplings),
        // Asymmetric: only allowed if BOTH allow it
        allow_asymmetric: a.allow_asymmetric && b.allow_asymmetric,
        // Higher consent = more restrictive
        consent_required: a.consent_required.clone().max(b.consent_required.clone()),
    }
}

/// Merge attention budget: take the more restrictive of each field.
fn merge_attention_budget(
    a: &AttentionBudgetConfig,
    b: &AttentionBudgetConfig,
) -> AttentionBudgetConfig {
    AttentionBudgetConfig {
        // Lower capacity = more restrictive
        default_capacity: a.default_capacity.min(b.default_capacity),
        // Higher reserve = more restrictive
        minimum_reserve: a.minimum_reserve.max(b.minimum_reserve),
        // Lower fraction = more restrictive
        max_single_coupling_fraction: a
            .max_single_coupling_fraction
            .min(b.max_single_coupling_fraction),
        // Most restrictive exhaustion behavior
        exhaustion_behavior: merge_exhaustion_behavior(
            &a.exhaustion_behavior,
            &b.exhaustion_behavior,
        ),
    }
}

/// Merge exhaustion behaviors: Block > EmergencyDecouple > Queue > DegradeWeakest.
fn merge_exhaustion_behavior(a: &ExhaustionBehavior, b: &ExhaustionBehavior) -> ExhaustionBehavior {
    fn severity(b: &ExhaustionBehavior) -> u8 {
        match b {
            ExhaustionBehavior::DegradeWeakest => 0,
            ExhaustionBehavior::Queue => 1,
            ExhaustionBehavior::EmergencyDecouple => 2,
            ExhaustionBehavior::Block => 3,
        }
    }
    // Higher severity = more restrictive
    if severity(a) >= severity(b) {
        a.clone()
    } else {
        b.clone()
    }
}

/// Merge intent resolution: take the more restrictive of each field.
fn merge_intent_resolution(
    a: &IntentResolutionRules,
    b: &IntentResolutionRules,
) -> IntentResolutionRules {
    IntentResolutionRules {
        // Higher threshold = more restrictive
        min_confidence_threshold: a.min_confidence_threshold.max(b.min_confidence_threshold),
        // Multi-signal required if EITHER requires it
        require_multi_signal: a.require_multi_signal || b.require_multi_signal,
        // Longer stabilization = more restrictive
        min_stabilization_ms: a.min_stabilization_ms.max(b.min_stabilization_ms),
        // Auto-commitment only if BOTH allow it
        allow_auto_commitment: a.allow_auto_commitment && b.allow_auto_commitment,
    }
}

/// Merge commitment authority: take the more restrictive of each field.
fn merge_commitment_authority(
    a: &CommitmentAuthority,
    b: &CommitmentAuthority,
) -> CommitmentAuthority {
    CommitmentAuthority {
        // Intersection of allowed domains (only domains in BOTH)
        allowed_domains: a
            .allowed_domains
            .iter()
            .filter(|d| b.allowed_domains.contains(d))
            .cloned()
            .collect(),
        // Lower risk class = more restrictive
        max_risk_class: a.max_risk_class.min(b.max_risk_class),
        // Irreversible only if BOTH allow it
        allow_irreversible: a.allow_irreversible && b.allow_irreversible,
        // Lower affected parties = more restrictive
        max_affected_parties: match (a.max_affected_parties, b.max_affected_parties) {
            (Some(x), Some(y)) => Some(x.min(y)),
            (Some(x), None) => Some(x),
            (None, Some(y)) => Some(y),
            (None, None) => None,
        },
        // Audit required if EITHER requires it
        require_audit_trail: a.require_audit_trail || b.require_audit_trail,
        // Lower consequence value = more restrictive
        max_consequence_value: match (a.max_consequence_value, b.max_consequence_value) {
            (Some(x), Some(y)) => Some(x.min(y)),
            (Some(x), None) => Some(x),
            (None, Some(y)) => Some(y),
            (None, None) => None,
        },
    }
}

/// Merge consequence scope: take the more restrictive of each field.
fn merge_consequence_scope(
    a: &ConsequenceScopeLimit,
    b: &ConsequenceScopeLimit,
) -> ConsequenceScopeLimit {
    ConsequenceScopeLimit {
        // Lower = more restrictive
        max_direct_affected: match (a.max_direct_affected, b.max_direct_affected) {
            (Some(x), Some(y)) => Some(x.min(y)),
            (Some(x), None) => Some(x),
            (None, Some(y)) => Some(y),
            (None, None) => None,
        },
        max_cascade_depth: match (a.max_cascade_depth, b.max_cascade_depth) {
            (Some(x), Some(y)) => Some(x.min(y)),
            (Some(x), None) => Some(x),
            (None, Some(y)) => Some(y),
            (None, None) => None,
        },
        // Cross-domain only if BOTH allow it
        allow_cross_domain: a.allow_cross_domain && b.allow_cross_domain,
        // More restrictive reversibility preference (lower enum ordinal)
        reversibility_preference: a
            .reversibility_preference
            .clone()
            .min(b.reversibility_preference.clone()),
    }
}

/// Merge human involvement: take the more restrictive of each field.
fn merge_human_involvement(
    a: &HumanInvolvementConfig,
    b: &HumanInvolvementConfig,
) -> HumanInvolvementConfig {
    HumanInvolvementConfig {
        // Higher oversight = more restrictive
        oversight_level: a.oversight_level.clone().max(b.oversight_level.clone()),
        // Required if EITHER requires it
        require_human_for_high_risk: a.require_human_for_high_risk || b.require_human_for_high_risk,
        require_human_for_irreversible: a.require_human_for_irreversible
            || b.require_human_for_irreversible,
        // Enabled if EITHER enables it
        coercion_detection_enabled: a.coercion_detection_enabled || b.coercion_detection_enabled,
        // Active if EITHER activates it
        human_agency_protection: a.human_agency_protection || b.human_agency_protection,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::{
        agent_profile, coordination_profile, financial_profile, human_profile, world_profile,
    };
    use crate::dimensions::{
        ConsentLevel, ExhaustionBehavior, OversightLevel, ReversibilityPreference,
    };
    use maple_mwl_types::{EffectDomain, RiskClass};

    #[test]
    fn merge_human_agent_takes_most_restrictive() {
        let human = human_profile();
        let agent = agent_profile();
        let merged = merged_constraints(&human, &agent);

        // Coupling: takes minimum (human is stricter)
        assert!(
            (merged.coupling_limits.max_initial_strength - 0.5).abs() < f64::EPSILON,
            "Should take human's lower max_initial_strength"
        );
        assert!(
            (merged.coupling_limits.max_sustained_strength - 0.7).abs() < f64::EPSILON,
            "Should take human's lower max_sustained_strength"
        );

        // Consent: takes higher (Informed > Explicit)
        assert_eq!(
            merged.coupling_limits.consent_required,
            ConsentLevel::Informed
        );

        // Oversight: takes higher (FullOversight > ApprovalForHighRisk)
        assert_eq!(
            merged.human_involvement.oversight_level,
            OversightLevel::FullOversight
        );

        // Coercion detection: enabled if either (both have it)
        assert!(merged.human_involvement.coercion_detection_enabled);

        // Human agency: active if either (human has it)
        assert!(merged.human_involvement.human_agency_protection);

        // Auto-commitment: only if both (human doesn't allow it)
        assert!(!merged.intent_resolution.allow_auto_commitment);
    }

    #[test]
    fn merge_agent_world_takes_most_restrictive() {
        let agent = agent_profile();
        let world = world_profile();
        let merged = merged_constraints(&agent, &world);

        // Coupling strength: world is lower
        assert!(
            (merged.coupling_limits.max_initial_strength - 0.3).abs() < f64::EPSILON,
            "Should take world's lower max_initial_strength"
        );

        // Concurrent couplings: agent is lower
        assert_eq!(merged.coupling_limits.max_concurrent_couplings, 10);

        // Consent: agent is higher (Explicit > Implicit)
        assert_eq!(
            merged.coupling_limits.consent_required,
            ConsentLevel::Explicit
        );

        // Confidence: agent is higher (0.8 > 0.6)
        assert!((merged.intent_resolution.min_confidence_threshold - 0.8).abs() < f64::EPSILON);

        // Audit: required if either (agent requires it)
        assert!(merged.commitment_authority.require_audit_trail);
    }

    #[test]
    fn merge_financial_coordination_is_strict() {
        let financial = financial_profile();
        let coord = coordination_profile();
        let merged = merged_constraints(&financial, &coord);

        // Risk class: financial is lower (Low < Medium)
        assert_eq!(merged.commitment_authority.max_risk_class, RiskClass::Low);

        // Confidence: financial is higher (0.9 > 0.75)
        assert!((merged.intent_resolution.min_confidence_threshold - 0.9).abs() < f64::EPSILON);

        // Domains: intersection only
        assert!(merged
            .commitment_authority
            .allowed_domains
            .contains(&EffectDomain::DataMutation));
        assert!(merged
            .commitment_authority
            .allowed_domains
            .contains(&EffectDomain::Governance));
        // Communication only in coord but not in financial's Financial domain
        assert!(!merged
            .commitment_authority
            .allowed_domains
            .contains(&EffectDomain::Financial));

        // Cross-domain: financial disallows it
        assert!(!merged.consequence_scope.allow_cross_domain);
    }

    #[test]
    fn merge_is_commutative_for_numeric_fields() {
        let human = human_profile();
        let agent = agent_profile();

        let ab = merged_constraints(&human, &agent);
        let ba = merged_constraints(&agent, &human);

        // Numeric constraints should be the same regardless of order
        assert!(
            (ab.coupling_limits.max_initial_strength - ba.coupling_limits.max_initial_strength)
                .abs()
                < f64::EPSILON
        );
        assert!(
            (ab.coupling_limits.max_sustained_strength - ba.coupling_limits.max_sustained_strength)
                .abs()
                < f64::EPSILON
        );
        assert_eq!(
            ab.coupling_limits.max_concurrent_couplings,
            ba.coupling_limits.max_concurrent_couplings
        );
        assert_eq!(
            ab.coupling_limits.consent_required,
            ba.coupling_limits.consent_required
        );
        assert_eq!(
            ab.human_involvement.oversight_level,
            ba.human_involvement.oversight_level
        );
        assert_eq!(
            ab.commitment_authority.max_risk_class,
            ba.commitment_authority.max_risk_class
        );
    }

    #[test]
    fn merge_same_profile_is_identity() {
        let agent = agent_profile();
        let merged = merged_constraints(&agent, &agent);

        assert!(
            (merged.coupling_limits.max_initial_strength
                - agent.coupling_limits.max_initial_strength)
                .abs()
                < f64::EPSILON
        );
        assert_eq!(
            merged.coupling_limits.max_concurrent_couplings,
            agent.coupling_limits.max_concurrent_couplings
        );
        assert_eq!(
            merged.commitment_authority.max_risk_class,
            agent.commitment_authority.max_risk_class
        );
        assert_eq!(
            merged.human_involvement.oversight_level,
            agent.human_involvement.oversight_level
        );
    }

    #[test]
    fn merge_asymmetric_requires_both() {
        let agent = agent_profile(); // allows asymmetric
        let human = human_profile(); // disallows asymmetric
        let merged = merged_constraints(&agent, &human);
        assert!(!merged.coupling_limits.allow_asymmetric);

        let coord = coordination_profile(); // allows asymmetric
        let merged2 = merged_constraints(&agent, &coord);
        assert!(merged2.coupling_limits.allow_asymmetric);
    }

    #[test]
    fn merge_domains_is_intersection() {
        let human = human_profile(); // Communication, DataMutation
        let financial = financial_profile(); // Financial, Governance, DataMutation
        let merged = merged_constraints(&human, &financial);

        // Only DataMutation is in both
        assert_eq!(merged.commitment_authority.allowed_domains.len(), 1);
        assert!(merged
            .commitment_authority
            .allowed_domains
            .contains(&EffectDomain::DataMutation));
    }

    #[test]
    fn merge_exhaustion_behavior_takes_most_restrictive() {
        let human = human_profile(); // Block
        let agent = agent_profile(); // DegradeWeakest
        let merged = merged_constraints(&human, &agent);
        assert_eq!(
            merged.attention_budget.exhaustion_behavior,
            ExhaustionBehavior::Block
        );

        let coord = coordination_profile(); // Queue
        let world = world_profile(); // DegradeWeakest
        let merged2 = merged_constraints(&coord, &world);
        assert_eq!(
            merged2.attention_budget.exhaustion_behavior,
            ExhaustionBehavior::Queue
        );
    }

    #[test]
    fn merge_reversibility_takes_most_restrictive() {
        let human = human_profile(); // RequireReversible
        let coord = coordination_profile(); // AllowTimeWindowed
        let merged = merged_constraints(&human, &coord);
        assert_eq!(
            merged.consequence_scope.reversibility_preference,
            ReversibilityPreference::RequireReversible
        );
    }

    #[test]
    fn merge_option_fields_take_some_over_none() {
        let world = world_profile(); // max_direct_affected: None
        let agent = agent_profile(); // max_direct_affected: Some(20)
        let merged = merged_constraints(&world, &agent);

        // When one is None (unlimited) and one is Some, take the Some (bounded)
        assert_eq!(merged.consequence_scope.max_direct_affected, Some(20));
    }
}
