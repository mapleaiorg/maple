//! Profile Manager - validates and enforces profile constraints

use crate::config::ProfileConfig;
use crate::runtime_core::ResonatorSpec;
use crate::types::*;
use resonator_profiles::{
    DefaultProfileValidator, ProfileArchetype, ProfileValidationContext, ProfileValidator,
};

/// Profile manager validates Resonator specifications against profile rules
pub struct ProfileManager {
    config: ProfileConfig,
    validator: DefaultProfileValidator,
    validation_context: ProfileValidationContext,
}

impl ProfileManager {
    pub fn new(config: &ProfileConfig) -> Self {
        let validation_context = ProfileValidationContext {
            human_profiles_allowed: config.human_profiles_allowed,
            ibank_profiles_allowed: config.allow_ibank_profiles,
            ..Default::default()
        };

        Self {
            config: config.clone(),
            validator: DefaultProfileValidator,
            validation_context,
        }
    }

    /// Validate a Resonator spec against profile constraints
    pub fn validate_spec(&self, spec: &ResonatorSpec) -> Result<(), String> {
        // Check if profile is allowed
        if !self.config.allowed_profiles.contains(&spec.profile) {
            return Err(format!("Profile {:?} not allowed", spec.profile));
        }

        // Canonical profile validation is centralized in resonator-profiles so all runtimes
        // apply the same archetype semantics.
        self.validator
            .validate_archetype(map_runtime_profile(spec.profile), &self.validation_context)
            .map_err(|e| e.to_string())?;

        // Runtime-local safety checks that depend on runtime-specific spec fields.
        if spec.profile == ResonatorProfile::Human && spec.attention.total_capacity == 0 {
            return Err("Human profiles require non-zero attention capacity".to_string());
        }

        Ok(())
    }

    /// Can these two profiles couple?
    pub fn can_couple(&self, profile_a: &ResonatorProfile, profile_b: &ResonatorProfile) -> bool {
        self.validator
            .can_couple(&map_runtime_profile(*profile_a), &map_runtime_profile(*profile_b))
    }
}

fn map_runtime_profile(profile: ResonatorProfile) -> ProfileArchetype {
    match profile {
        ResonatorProfile::Human => ProfileArchetype::Human,
        ResonatorProfile::World => ProfileArchetype::World,
        ResonatorProfile::Coordination => ProfileArchetype::Coordination,
        ResonatorProfile::IBank => ProfileArchetype::IBank,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ibank_runtime_config, mapleverse_runtime_config};
    use crate::runtime_core::ResonatorSpec;

    #[test]
    fn mapleverse_rejects_human_profile() {
        let runtime_config = mapleverse_runtime_config();
        let manager = ProfileManager::new(&runtime_config.profiles);
        let mut spec = ResonatorSpec::default();
        spec.profile = ResonatorProfile::Human;

        let result = manager.validate_spec(&spec);
        assert!(result.is_err());
    }

    #[test]
    fn ibank_accepts_ibank_profile() {
        let runtime_config = ibank_runtime_config();
        let manager = ProfileManager::new(&runtime_config.profiles);
        let mut spec = ResonatorSpec::default();
        spec.profile = ResonatorProfile::IBank;

        let result = manager.validate_spec(&spec);
        assert!(result.is_ok(), "iBank profile should validate: {result:?}");
    }

    #[test]
    fn coupling_uses_canonical_policy() {
        let manager = ProfileManager::new(&ProfileConfig::default());

        assert!(manager.can_couple(&ResonatorProfile::Human, &ResonatorProfile::World));
        assert!(!manager.can_couple(
            &ResonatorProfile::Human,
            &ResonatorProfile::Coordination
        ));
        assert!(!manager.can_couple(&ResonatorProfile::World, &ResonatorProfile::IBank));
        assert!(manager.can_couple(
            &ResonatorProfile::Coordination,
            &ResonatorProfile::Coordination
        ));
    }
}
