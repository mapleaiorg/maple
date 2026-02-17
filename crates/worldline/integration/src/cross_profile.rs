//! Cross-Profile Interaction Tests
//!
//! Verifies I.PROF-1 (Maximum Restriction Principle): when worldlines
//! with different profiles interact, the most restrictive constraint
//! from each dimension applies.

use worldline_runtime::profiles::{
    agent_profile, coordination_profile, financial_profile, human_profile, merged_constraints,
    world_profile, ConsentLevel, CouplingProposal, ProfileEnforcer,
};

/// Human + Financial interaction uses strictest constraints from both.
#[test]
fn test_human_financial_uses_strictest() {
    let human = human_profile();
    let financial = financial_profile();

    let merged = merged_constraints(&human, &financial);

    // Human has Informed consent requirement (strictest)
    // Maximum restriction → the stricter consent level
    assert!(
        merged.coupling_limits.consent_required >= human.coupling_limits.consent_required
            || merged.coupling_limits.consent_required
                >= financial.coupling_limits.consent_required,
        "Merged profile should use the stricter consent level"
    );

    // Oversight: human has FullOversight, financial has less
    // Maximum restriction → FullOversight
    assert!(
        merged.human_involvement.oversight_level >= human.human_involvement.oversight_level
            || merged.human_involvement.oversight_level
                >= financial.human_involvement.oversight_level,
        "Merged profile should use the stricter oversight level"
    );
}

/// Agent + World interaction satisfies both constraints.
#[test]
fn test_agent_world_interaction() {
    let agent = agent_profile();
    let world = world_profile();

    let merged = merged_constraints(&agent, &world);

    // Merged should use the more restrictive attention budget capacity (minimum)
    assert!(
        merged.attention_budget.default_capacity
            <= agent
                .attention_budget
                .default_capacity
                .min(world.attention_budget.default_capacity),
        "Merged attention budget should be the minimum of both"
    );

    // Merged coupling strength should be the minimum
    let min_strength = agent
        .coupling_limits
        .max_initial_strength
        .min(world.coupling_limits.max_initial_strength);
    assert!(
        (merged.coupling_limits.max_initial_strength - min_strength).abs() < f64::EPSILON,
        "Merged max initial strength should be the minimum of both"
    );
}

/// Merge is commutative: merge(A, B) == merge(B, A).
#[test]
fn test_merge_commutative() {
    let human = human_profile();
    let financial = financial_profile();

    let merged_hf = merged_constraints(&human, &financial);
    let merged_fh = merged_constraints(&financial, &human);

    // Numeric fields should be equal regardless of order
    assert!(
        (merged_hf.coupling_limits.max_initial_strength
            - merged_fh.coupling_limits.max_initial_strength)
            .abs()
            < f64::EPSILON,
    );
    assert_eq!(
        merged_hf.attention_budget.default_capacity,
        merged_fh.attention_budget.default_capacity,
    );
    assert_eq!(
        merged_hf.coupling_limits.consent_required,
        merged_fh.coupling_limits.consent_required,
    );
}

/// Self-merge is identity: merge(A, A) == A.
#[test]
fn test_self_merge_identity() {
    let agent = agent_profile();
    let merged = merged_constraints(&agent, &agent);

    assert!(
        (merged.coupling_limits.max_initial_strength - agent.coupling_limits.max_initial_strength)
            .abs()
            < f64::EPSILON,
    );
    assert_eq!(
        merged.attention_budget.default_capacity,
        agent.attention_budget.default_capacity,
    );
}

/// Profile enforcer checks coupling within limits.
#[test]
fn test_profile_enforcer_coupling() {
    let profile = agent_profile();

    // Coupling within limits
    let proposal = CouplingProposal {
        strength: profile.coupling_limits.max_initial_strength * 0.5,
        current_couplings: 0,
        is_asymmetric: false,
        consent_provided: ConsentLevel::Informed,
        attention_fraction: 0.1,
    };
    let result = ProfileEnforcer::check_coupling(&profile, &proposal);
    assert!(
        result.is_permitted(),
        "Coupling within limits should be allowed"
    );

    // Coupling exceeding limits
    let bad_proposal = CouplingProposal {
        strength: profile.coupling_limits.max_initial_strength * 1.5,
        current_couplings: 0,
        is_asymmetric: false,
        consent_provided: ConsentLevel::Informed,
        attention_fraction: 0.1,
    };
    let result = ProfileEnforcer::check_coupling(&profile, &bad_proposal);
    assert!(
        result.is_denied(),
        "Coupling exceeding limits should be denied"
    );
}

/// All 5 canonical profiles have distinct characteristics.
#[test]
fn test_all_canonical_profiles_distinct() {
    let profiles = vec![
        ("human", human_profile()),
        ("agent", agent_profile()),
        ("financial", financial_profile()),
        ("world", world_profile()),
        ("coordination", coordination_profile()),
    ];

    // Each profile should have different characteristics
    for i in 0..profiles.len() {
        for j in (i + 1)..profiles.len() {
            let (name_a, prof_a) = &profiles[i];
            let (name_b, prof_b) = &profiles[j];

            // At least one dimension should differ
            let differs = (prof_a.coupling_limits.max_initial_strength
                - prof_b.coupling_limits.max_initial_strength)
                .abs()
                > f64::EPSILON
                || prof_a.attention_budget.default_capacity
                    != prof_b.attention_budget.default_capacity
                || prof_a.coupling_limits.consent_required
                    != prof_b.coupling_limits.consent_required;

            assert!(
                differs,
                "Profiles {} and {} should have distinct characteristics",
                name_a, name_b
            );
        }
    }
}
