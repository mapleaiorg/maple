//! Profile Manager - validates and enforces profile constraints

use crate::config::ProfileConfig;
use crate::runtime_core::ResonatorSpec;
use crate::types::*;

/// Profile manager validates Resonator specifications against profile rules
pub struct ProfileManager {
    config: ProfileConfig,
}

impl ProfileManager {
    pub fn new(config: &ProfileConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Validate a Resonator spec against profile constraints
    pub fn validate_spec(&self, spec: &ResonatorSpec) -> Result<(), String> {
        // Check if profile is allowed
        if !self.config.allowed_profiles.contains(&spec.profile) {
            return Err(format!("Profile {:?} not allowed", spec.profile));
        }

        // Check human profiles if restricted
        if !self.config.human_profiles_allowed && spec.profile.requires_agency_protection() {
            return Err("Human profiles not allowed in this configuration".to_string());
        }

        // Profile-specific validation
        match spec.profile {
            ResonatorProfile::Human => {
                // Human profiles require extra validation
                if spec.attention.total_capacity == 0 {
                    return Err("Human profiles require non-zero attention capacity".to_string());
                }
            }
            ResonatorProfile::IBank => {
                // IBank profiles have strict requirements
                if !self.config.allow_ibank_profiles {
                    return Err("IBank profiles not allowed".to_string());
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Can these two profiles couple?
    pub fn can_couple(&self, profile_a: &ResonatorProfile, profile_b: &ResonatorProfile) -> bool {
        profile_a.can_couple_with(profile_b)
    }
}
