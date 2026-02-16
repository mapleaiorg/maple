use maple_mwl_types::EffectDomain;
use maple_mwl_types::RiskClass;

use crate::canonical::{
    agent_profile, coordination_profile, financial_profile, human_profile, world_profile,
};
use crate::dimensions::{OversightLevel, ProfileType, ReversibilityPreference, WorldlineProfile};

/// Platform configuration — maps to existing platform runtime configs.
///
/// Existing platforms (Mapleverse, Finalverse, iBank) have runtime configurations
/// that specify which profiles are active and their constraints. This module
/// provides backward-compatible mappings from platform configs to the new
/// WorldlineProfile system.
#[derive(Clone, Debug)]
pub struct PlatformProfileConfig {
    /// Platform name
    pub name: String,
    /// Profiles active on this platform
    pub active_profiles: Vec<WorldlineProfile>,
    /// Whether safety is enforced at platform level
    pub enforce_safety: bool,
}

/// Create the Mapleverse platform profile configuration.
///
/// Mapleverse: No human worldlines, coordination + world only.
/// Focused on autonomous agent interaction and world-state management.
pub fn mapleverse_platform() -> PlatformProfileConfig {
    let mut coord = coordination_profile();
    let mut world = world_profile();

    // Mapleverse-specific tuning: higher autonomy since no humans
    coord.coupling_limits.max_concurrent_couplings = 40;
    coord.attention_budget.default_capacity = 400;
    coord.human_involvement.oversight_level = OversightLevel::AuditOnly;
    coord.human_involvement.require_human_for_high_risk = false;

    // World profile stays mostly default but with higher capacity
    world.attention_budget.default_capacity = 1000;

    // Agent profile for Mapleverse agents
    let mut agent = agent_profile();
    agent.coupling_limits.max_concurrent_couplings = 15;
    agent.human_involvement.oversight_level = OversightLevel::Notification;
    agent.human_involvement.require_human_for_high_risk = false;

    PlatformProfileConfig {
        name: "Mapleverse".into(),
        active_profiles: vec![agent, coord, world],
        enforce_safety: true,
    }
}

/// Create the Finalverse platform profile configuration.
///
/// Finalverse: All profiles active, full safety enforcement.
/// This is the most comprehensive platform with human interaction.
pub fn finalverse_platform() -> PlatformProfileConfig {
    let human = human_profile();
    let agent = agent_profile();
    let financial = financial_profile();
    let world = world_profile();
    let coord = coordination_profile();

    PlatformProfileConfig {
        name: "Finalverse".into(),
        active_profiles: vec![human, agent, financial, world, coord],
        enforce_safety: true,
    }
}

/// Create the iBank platform profile configuration.
///
/// iBank: Financial worldlines only, strict audit, conservative risk.
pub fn ibank_platform() -> PlatformProfileConfig {
    let mut financial = financial_profile();

    // iBank-specific: even stricter financial controls
    financial.intent_resolution.min_confidence_threshold = 0.95;
    financial.commitment_authority.max_consequence_value = Some(50000);
    financial.consequence_scope.reversibility_preference =
        ReversibilityPreference::RequireReversible;

    // iBank also has a limited agent profile for automation
    let mut agent = agent_profile();
    agent.commitment_authority.allowed_domains =
        vec![EffectDomain::Financial, EffectDomain::DataMutation];
    agent.commitment_authority.max_risk_class = RiskClass::Low;
    agent.commitment_authority.require_audit_trail = true;
    agent.human_involvement.oversight_level = OversightLevel::ApprovalForHighRisk;

    PlatformProfileConfig {
        name: "iBank".into(),
        active_profiles: vec![financial, agent],
        enforce_safety: true,
    }
}

/// Get the default profile for a platform by profile type.
pub fn platform_profile(
    platform: &PlatformProfileConfig,
    profile_type: &ProfileType,
) -> Option<WorldlineProfile> {
    platform
        .active_profiles
        .iter()
        .find(|p| &p.profile_type == profile_type)
        .cloned()
}

/// Check if a profile type is active on a platform.
pub fn is_profile_active(platform: &PlatformProfileConfig, profile_type: &ProfileType) -> bool {
    platform
        .active_profiles
        .iter()
        .any(|p| &p.profile_type == profile_type)
}

/// List all active profile types on a platform.
pub fn active_profile_types(platform: &PlatformProfileConfig) -> Vec<ProfileType> {
    platform
        .active_profiles
        .iter()
        .map(|p| p.profile_type.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapleverse_has_no_human_profiles() {
        let platform = mapleverse_platform();
        assert!(!is_profile_active(&platform, &ProfileType::Human));
        assert!(!is_profile_active(&platform, &ProfileType::Financial));
    }

    #[test]
    fn mapleverse_has_agent_coord_world() {
        let platform = mapleverse_platform();
        assert!(is_profile_active(&platform, &ProfileType::Agent));
        assert!(is_profile_active(&platform, &ProfileType::Coordination));
        assert!(is_profile_active(&platform, &ProfileType::World));
    }

    #[test]
    fn mapleverse_enforces_safety() {
        let platform = mapleverse_platform();
        assert!(platform.enforce_safety);
    }

    #[test]
    fn mapleverse_coordination_has_higher_concurrency() {
        let platform = mapleverse_platform();
        let coord = platform_profile(&platform, &ProfileType::Coordination).unwrap();
        let default_coord = coordination_profile();
        assert!(
            coord.coupling_limits.max_concurrent_couplings
                > default_coord.coupling_limits.max_concurrent_couplings
        );
    }

    #[test]
    fn finalverse_has_all_profiles() {
        let platform = finalverse_platform();
        assert!(is_profile_active(&platform, &ProfileType::Human));
        assert!(is_profile_active(&platform, &ProfileType::Agent));
        assert!(is_profile_active(&platform, &ProfileType::Financial));
        assert!(is_profile_active(&platform, &ProfileType::World));
        assert!(is_profile_active(&platform, &ProfileType::Coordination));
        assert_eq!(platform.active_profiles.len(), 5);
    }

    #[test]
    fn finalverse_enforces_safety() {
        let platform = finalverse_platform();
        assert!(platform.enforce_safety);
    }

    #[test]
    fn finalverse_human_profile_is_canonical() {
        let platform = finalverse_platform();
        let human = platform_profile(&platform, &ProfileType::Human).unwrap();
        let canonical = human_profile();

        // Should match canonical (no platform-specific tuning)
        assert!(
            (human.coupling_limits.max_initial_strength
                - canonical.coupling_limits.max_initial_strength)
                .abs()
                < f64::EPSILON
        );
        assert_eq!(
            human.coupling_limits.consent_required,
            canonical.coupling_limits.consent_required
        );
    }

    #[test]
    fn ibank_has_financial_and_agent_only() {
        let platform = ibank_platform();
        let types = active_profile_types(&platform);
        assert_eq!(types.len(), 2);
        assert!(is_profile_active(&platform, &ProfileType::Financial));
        assert!(is_profile_active(&platform, &ProfileType::Agent));
        assert!(!is_profile_active(&platform, &ProfileType::Human));
        assert!(!is_profile_active(&platform, &ProfileType::World));
    }

    #[test]
    fn ibank_financial_is_stricter_than_canonical() {
        let platform = ibank_platform();
        let ibank_fin = platform_profile(&platform, &ProfileType::Financial).unwrap();
        let canonical_fin = financial_profile();

        // iBank has higher confidence threshold
        assert!(
            ibank_fin.intent_resolution.min_confidence_threshold
                > canonical_fin.intent_resolution.min_confidence_threshold
        );
    }

    #[test]
    fn ibank_agent_is_restricted_to_financial() {
        let platform = ibank_platform();
        let agent = platform_profile(&platform, &ProfileType::Agent).unwrap();

        // Agent on iBank can only do Financial and DataMutation
        assert!(agent
            .commitment_authority
            .allowed_domains
            .contains(&EffectDomain::Financial));
        assert!(agent
            .commitment_authority
            .allowed_domains
            .contains(&EffectDomain::DataMutation));
        assert!(!agent
            .commitment_authority
            .allowed_domains
            .contains(&EffectDomain::Infrastructure));
    }

    #[test]
    fn ibank_enforces_safety() {
        let platform = ibank_platform();
        assert!(platform.enforce_safety);
    }

    #[test]
    fn platform_profile_returns_none_for_inactive() {
        let platform = mapleverse_platform();
        assert!(platform_profile(&platform, &ProfileType::Human).is_none());
    }
}
