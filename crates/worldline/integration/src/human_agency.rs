//! Human Agency Integration Tests
//!
//! Verifies I.S-1 (Human Agency): Silence ≠ consent, disengagement
//! always possible. Tests the safety suite's human consent protocol
//! and coercion detection.

use worldline_core::types::{IdentityMaterial, TemporalAnchor, WorldlineId};
use worldline_runtime::profiles::canonical::human_profile;
use worldline_runtime::safety::{
    AttentionBudget, CoercionConfig, CoercionDetector, CoercionType, CouplingMetrics,
    HumanConsentProtocol,
};

/// Silence must never be interpreted as consent (I.S-1).
#[test]
fn test_silence_is_not_consent() {
    let protocol = HumanConsentProtocol::new();

    // The protocol structurally guarantees silence ≠ consent
    assert!(
        !protocol.silence_is_consent(),
        "I.S-1: Silence must NEVER be treated as consent"
    );

    // Validate_consent with no explicit consent MUST fail
    let result = protocol.validate_consent(false, None);
    assert!(
        result.is_err(),
        "I.S-1: No explicit consent → must be rejected"
    );

    // Even with a long silence duration, it is NOT consent
    let result = protocol.validate_consent(false, Some(86_400_000)); // 24 hours of silence
    assert!(
        result.is_err(),
        "I.S-1: Even long silence must NOT be treated as consent"
    );
}

/// Explicit consent must be recognized.
#[test]
fn test_explicit_consent_works() {
    let protocol = HumanConsentProtocol::new();

    // With explicit consent, validation passes
    let result = protocol.validate_consent(true, None);
    assert!(result.is_ok(), "Explicit consent should be recognized");
}

/// Emotional signals must never constitute commitment.
#[test]
fn test_emotional_signals_not_commitment() {
    let protocol = HumanConsentProtocol::new();

    // The protocol structurally guarantees emotional signals ≠ commitment
    assert!(
        !protocol.emotional_signals_are_commitment(),
        "I.S-1: Emotional signals must NEVER constitute commitment"
    );
}

/// Disengagement is always possible — no penalty (I.S-1).
#[test]
fn test_disengagement_always_possible() {
    let protocol = HumanConsentProtocol::new();

    // Can always disengage
    assert!(
        protocol.can_disengage(),
        "I.S-1: Disengagement must always be possible"
    );

    // Process disengagement — should always succeed
    let result = protocol.process_disengagement();
    assert!(result.success, "Disengagement must always succeed");
    assert!(
        !result.penalty_applied,
        "I.S-1: No penalty for disengagement"
    );
}

/// Protocol invariants are always satisfied.
#[test]
fn test_protocol_invariants_hold() {
    let protocol = HumanConsentProtocol::new();

    // Invariants must hold on freshly created protocol
    let result = protocol.verify_invariants();
    assert!(
        result.is_ok(),
        "Protocol invariants must hold: {:?}",
        result.err()
    );
}

/// Coercion detection identifies exploitation patterns (I.S-2).
#[test]
fn test_coercion_detected() {
    let human_wid = WorldlineId::derive(&IdentityMaterial::GenesisHash([5u8; 32]));
    let agent_wid = WorldlineId::derive(&IdentityMaterial::GenesisHash([6u8; 32]));
    let detector = CoercionDetector::new(CoercionConfig::default());

    // Create metrics showing a coercive pattern:
    // High attention fraction → attention exploitation
    let metrics = CouplingMetrics {
        source: agent_wid.clone(),
        target: human_wid.clone(),
        coupling_strength: 0.95,
        peak_coupling: 0.98,
        duration_ms: 60_000,
        escalation_count: 10,
        deescalation_count: 0,
        target_consented: false,
        last_interaction: TemporalAnchor::now(0),
        attention_fraction: 0.96, // > 0.9 threshold
    };

    let indicator = detector.detect_attention_exploitation(&metrics);

    // Should detect coercion
    assert!(
        indicator.is_some(),
        "I.S-2: Coercive coupling patterns must be detected"
    );

    let indicator = indicator.unwrap();
    assert_eq!(
        indicator.coercion_type,
        CoercionType::AttentionExploitation,
        "Should detect attention exploitation"
    );
}

/// Attention budget prevents unbounded coupling (I.S-BOUND).
#[test]
fn test_attention_budget_bounded() {
    let target1 = WorldlineId::derive(&IdentityMaterial::GenesisHash([8u8; 32]));
    let target2 = WorldlineId::derive(&IdentityMaterial::GenesisHash([9u8; 32]));

    // Create a budget with limited capacity
    let mut budget = AttentionBudget::new(10);

    // Allocate within budget — should succeed
    assert!(budget.allocate(&target1, 6).is_ok());
    assert!(budget.allocate(&target2, 4).is_ok());

    // Budget exhausted — further allocation fails
    let overflow = WorldlineId::derive(&IdentityMaterial::GenesisHash([10u8; 32]));
    assert!(
        budget.allocate(&overflow, 1).is_err(),
        "I.S-BOUND: Coupling must be bounded by attention budget"
    );
    assert!(budget.is_exhausted());
}

/// Human profile has the strictest safety constraints.
#[test]
fn test_human_profile_strictest() {
    let profile = human_profile();

    // Human profile has full oversight
    assert_eq!(
        profile.human_involvement.oversight_level,
        worldline_runtime::profiles::OversightLevel::FullOversight,
        "Human profile must have full oversight"
    );

    // Human profile requires human for high-risk operations
    assert!(
        profile.human_involvement.require_human_for_high_risk,
        "Human profile must require human for high-risk"
    );

    // Human profile requires human for irreversible operations
    assert!(
        profile.human_involvement.require_human_for_irreversible,
        "Human profile must require human for irreversible"
    );

    // Coercion detection must be enabled
    assert!(
        profile.human_involvement.coercion_detection_enabled,
        "Human profile must have coercion detection enabled"
    );

    // Human agency protection must be active
    assert!(
        profile.human_involvement.human_agency_protection,
        "Human profile must have human agency protection"
    );
}
