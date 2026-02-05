//! Canonical Resonator profile definitions and validation.
//!
//! This crate centralizes profile semantics so platform runtimes can enforce
//! profile rules consistently without duplicating coupling or safety logic.

#![deny(unsafe_code)]
#![cfg_attr(feature = "strict-docs", warn(missing_docs))]
#![cfg_attr(not(feature = "strict-docs"), allow(missing_docs))]
#![warn(rust_2018_idioms)]

use rcf_types::EffectDomain;
use resonator_types::{
    AutonomyLevel, ConstraintType, ProfileConstraint, ResonatorProfile, RiskTolerance,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Canonical profile archetypes recognized by MAPLE.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileArchetype {
    Human,
    World,
    Coordination,
    IBank,
    Custom(String),
}

impl ProfileArchetype {
    /// Infer an archetype from a profile name.
    pub fn from_profile_name(name: &str) -> Self {
        let normalized = name.trim().to_ascii_lowercase();
        if normalized == "human" {
            Self::Human
        } else if normalized == "world" {
            Self::World
        } else if normalized == "coordination" {
            Self::Coordination
        } else if normalized == "ibank" {
            Self::IBank
        } else {
            Self::Custom(name.trim().to_string())
        }
    }
}

/// Runtime policy context used to validate profile eligibility.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileValidationContext {
    /// Whether human archetypes are permitted in this runtime.
    pub human_profiles_allowed: bool,
    /// Whether iBank archetypes are permitted in this runtime.
    pub ibank_profiles_allowed: bool,
    /// Maximum number of constraints allowed on a profile.
    pub max_constraints: usize,
    /// Require at least one domain declaration.
    pub require_domains: bool,
}

impl Default for ProfileValidationContext {
    fn default() -> Self {
        Self {
            human_profiles_allowed: true,
            ibank_profiles_allowed: true,
            max_constraints: 64,
            require_domains: true,
        }
    }
}

/// Validation errors returned by [`ProfileValidator`].
#[derive(Debug, Error)]
pub enum ProfileValidationError {
    #[error("profile name must not be empty")]
    EmptyName,
    #[error("profile description must not be empty")]
    EmptyDescription,
    #[error("profile declares too many constraints: {count} > {max}")]
    TooManyConstraints { count: usize, max: usize },
    #[error("profile archetype `{archetype:?}` requires at least one effect domain")]
    MissingDomains { archetype: ProfileArchetype },
    #[error("profile archetype `{archetype:?}` is disabled by runtime policy")]
    ArchetypeDisabled { archetype: ProfileArchetype },
    #[error("human profile must use full human oversight autonomy")]
    HumanAutonomyViolation,
    #[error("iBank profile cannot use aggressive risk tolerance")]
    IBankRiskViolation,
}

/// Validator interface for profile canonicalization.
pub trait ProfileValidator {
    /// Validate a concrete profile instance against runtime context.
    fn validate_profile(
        &self,
        profile: &ResonatorProfile,
        context: &ProfileValidationContext,
    ) -> Result<(), ProfileValidationError>;

    /// Validate a canonical archetype using its built-in template.
    fn validate_archetype(
        &self,
        archetype: ProfileArchetype,
        context: &ProfileValidationContext,
    ) -> Result<(), ProfileValidationError> {
        let profile = builtin_profile(archetype);
        self.validate_profile(&profile, context)
    }

    /// Check whether coupling is allowed between profile archetypes.
    fn can_couple(&self, source: &ProfileArchetype, target: &ProfileArchetype) -> bool;
}

/// Default deterministic profile validator used by MAPLE runtimes.
#[derive(Debug, Default, Clone)]
pub struct DefaultProfileValidator;

impl DefaultProfileValidator {
    fn detect_archetype(&self, profile: &ResonatorProfile) -> ProfileArchetype {
        ProfileArchetype::from_profile_name(&profile.name)
    }
}

impl ProfileValidator for DefaultProfileValidator {
    fn validate_profile(
        &self,
        profile: &ResonatorProfile,
        context: &ProfileValidationContext,
    ) -> Result<(), ProfileValidationError> {
        if profile.name.trim().is_empty() {
            return Err(ProfileValidationError::EmptyName);
        }

        if profile.description.trim().is_empty() {
            return Err(ProfileValidationError::EmptyDescription);
        }

        if profile.constraints.len() > context.max_constraints {
            return Err(ProfileValidationError::TooManyConstraints {
                count: profile.constraints.len(),
                max: context.max_constraints,
            });
        }

        let archetype = self.detect_archetype(profile);

        if context.require_domains && profile.domains.is_empty() {
            return Err(ProfileValidationError::MissingDomains { archetype });
        }

        match archetype {
            ProfileArchetype::Human => {
                if !context.human_profiles_allowed {
                    return Err(ProfileValidationError::ArchetypeDisabled { archetype });
                }

                if profile.autonomy_level != AutonomyLevel::FullHumanOversight {
                    return Err(ProfileValidationError::HumanAutonomyViolation);
                }
            }
            ProfileArchetype::IBank => {
                if !context.ibank_profiles_allowed {
                    return Err(ProfileValidationError::ArchetypeDisabled { archetype });
                }

                if profile.risk_tolerance == RiskTolerance::Aggressive {
                    return Err(ProfileValidationError::IBankRiskViolation);
                }
            }
            ProfileArchetype::World | ProfileArchetype::Coordination | ProfileArchetype::Custom(_) => {}
        }

        Ok(())
    }

    fn can_couple(&self, source: &ProfileArchetype, target: &ProfileArchetype) -> bool {
        use ProfileArchetype::*;

        match (source, target) {
            (Human, World) => true,
            (World, Human) | (World, World) => true,
            (Coordination, Coordination) => true,
            (IBank, IBank) => true,
            (Custom(a), Custom(b)) => a.eq_ignore_ascii_case(b),
            _ => false,
        }
    }
}

/// Build a canonical profile template for the given archetype.
pub fn builtin_profile(archetype: ProfileArchetype) -> ResonatorProfile {
    match archetype {
        ProfileArchetype::Human => ResonatorProfile {
            name: "Human".to_string(),
            description: "Human participant profile with strong agency protections.".to_string(),
            domains: vec![EffectDomain::Communication],
            risk_tolerance: RiskTolerance::Conservative,
            autonomy_level: AutonomyLevel::FullHumanOversight,
            constraints: vec![
                profile_constraint(
                    ConstraintType::Custom("presence_not_consent".to_string()),
                    "Presence alone does not imply consent to coupling.",
                    &[("value", "true")],
                ),
                profile_constraint(
                    ConstraintType::DomainRestriction,
                    "Human profiles only couple with world profiles.",
                    &[("allowed_coupling_targets", "world")],
                ),
            ],
        },
        ProfileArchetype::World => ResonatorProfile {
            name: "World".to_string(),
            description: "Experiential AI profile for human-facing environments.".to_string(),
            domains: vec![EffectDomain::Communication, EffectDomain::Data],
            risk_tolerance: RiskTolerance::Balanced,
            autonomy_level: AutonomyLevel::GuidedAutonomy,
            constraints: vec![profile_constraint(
                ConstraintType::Custom("prefer_reversible_consequences".to_string()),
                "World profile should prefer reversible consequences.",
                &[("value", "true")],
            )],
        },
        ProfileArchetype::Coordination => ResonatorProfile {
            name: "Coordination".to_string(),
            description: "Pure AI coordination profile for operational workloads.".to_string(),
            domains: vec![
                EffectDomain::Computation,
                EffectDomain::Data,
                EffectDomain::Infrastructure,
            ],
            risk_tolerance: RiskTolerance::Balanced,
            autonomy_level: AutonomyLevel::HighAutonomy,
            constraints: vec![
                profile_constraint(
                    ConstraintType::Custom("require_explicit_commitment".to_string()),
                    "Consequential operations require explicit commitments.",
                    &[("value", "true")],
                ),
                profile_constraint(
                    ConstraintType::Custom("require_audit_trail".to_string()),
                    "Coordination profile emits auditable actions.",
                    &[("value", "true")],
                ),
            ],
        },
        ProfileArchetype::IBank => ResonatorProfile {
            name: "IBank".to_string(),
            description: "Autonomous finance profile with strict compliance and risk bounds."
                .to_string(),
            domains: vec![
                EffectDomain::Finance,
                EffectDomain::Governance,
                EffectDomain::Data,
            ],
            risk_tolerance: RiskTolerance::Conservative,
            autonomy_level: AutonomyLevel::GuidedAutonomy,
            constraints: vec![
                profile_constraint(
                    ConstraintType::Custom("require_risk_assessment".to_string()),
                    "Financial operations require risk assessment before commitment.",
                    &[("value", "true")],
                ),
                profile_constraint(
                    ConstraintType::Custom("require_compliance_proof".to_string()),
                    "Financial operations require compliance proof artifacts.",
                    &[("value", "true")],
                ),
            ],
        },
        ProfileArchetype::Custom(name) => ResonatorProfile {
            name: name.clone(),
            description: format!("Custom profile template for `{name}`."),
            domains: vec![EffectDomain::Custom(name)],
            risk_tolerance: RiskTolerance::Balanced,
            autonomy_level: AutonomyLevel::GuidedAutonomy,
            constraints: vec![],
        },
    }
}

fn profile_constraint(
    constraint_type: ConstraintType,
    description: &str,
    parameters: &[(&str, &str)],
) -> ProfileConstraint {
    let parameters = parameters
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect::<HashMap<_, _>>();

    ProfileConstraint {
        constraint_type,
        description: description.to_string(),
        parameters,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_builtin_human_profile() {
        let validator = DefaultProfileValidator;
        let profile = builtin_profile(ProfileArchetype::Human);

        let result = validator.validate_profile(&profile, &ProfileValidationContext::default());
        assert!(result.is_ok(), "human profile should validate: {result:?}");
    }

    #[test]
    fn rejects_human_when_disabled() {
        let validator = DefaultProfileValidator;
        let mut context = ProfileValidationContext::default();
        context.human_profiles_allowed = false;

        let result = validator.validate_archetype(ProfileArchetype::Human, &context);
        assert!(matches!(
            result,
            Err(ProfileValidationError::ArchetypeDisabled {
                archetype: ProfileArchetype::Human
            })
        ));
    }

    #[test]
    fn rejects_ibank_aggressive_risk() {
        let validator = DefaultProfileValidator;
        let mut profile = builtin_profile(ProfileArchetype::IBank);
        profile.risk_tolerance = RiskTolerance::Aggressive;

        let result = validator.validate_profile(&profile, &ProfileValidationContext::default());
        assert!(matches!(
            result,
            Err(ProfileValidationError::IBankRiskViolation)
        ));
    }

    #[test]
    fn coupling_matrix_is_deterministic() {
        let validator = DefaultProfileValidator;

        assert!(validator.can_couple(&ProfileArchetype::Human, &ProfileArchetype::World));
        assert!(!validator.can_couple(
            &ProfileArchetype::Human,
            &ProfileArchetype::Coordination
        ));
        assert!(validator.can_couple(
            &ProfileArchetype::Coordination,
            &ProfileArchetype::Coordination
        ));
        assert!(!validator.can_couple(&ProfileArchetype::World, &ProfileArchetype::IBank));
    }
}
