use maple_mwl_types::{EffectDomain, RiskClass, Reversibility};
use tracing::{debug, warn};

use crate::dimensions::{
    ConsentLevel, OversightLevel, ReversibilityPreference, WorldlineProfile,
};
use crate::error::{
    EnforcementResult, ProfileViolation, ViolationDimension, ViolationSeverity,
};

/// ProfileEnforcer — validates operations against profile constraints.
///
/// The enforcer checks whether a proposed operation (coupling, commitment,
/// consequence) is within the bounds defined by a worldline's profile.
/// It never modifies the profile — only reads and validates.
pub struct ProfileEnforcer;

/// A proposed coupling to be validated.
#[derive(Clone, Debug)]
pub struct CouplingProposal {
    /// Proposed coupling strength
    pub strength: f64,
    /// Current number of active couplings for this worldline
    pub current_couplings: u32,
    /// Whether this is an asymmetric coupling
    pub is_asymmetric: bool,
    /// The consent level provided
    pub consent_provided: ConsentLevel,
    /// Attention fraction this coupling would consume
    pub attention_fraction: f64,
}

/// A proposed commitment to be validated.
#[derive(Clone, Debug)]
pub struct CommitmentProposal {
    /// Effect domain
    pub domain: EffectDomain,
    /// Risk class of the operation
    pub risk_class: RiskClass,
    /// Whether the commitment is irreversible
    pub reversibility: Reversibility,
    /// Number of affected parties
    pub affected_parties: u32,
    /// Consequence value (arbitrary units)
    pub consequence_value: Option<u64>,
    /// Whether human approval has been obtained
    pub has_human_approval: bool,
}

impl ProfileEnforcer {
    /// Check a coupling proposal against a profile.
    pub fn check_coupling(
        profile: &WorldlineProfile,
        proposal: &CouplingProposal,
    ) -> EnforcementResult {
        let mut violations = Vec::new();
        let mut warnings = Vec::new();

        // Check coupling strength
        if proposal.strength > profile.coupling_limits.max_initial_strength {
            violations.push(ProfileViolation {
                profile_type: profile.name.clone(),
                dimension: ViolationDimension::CouplingLimits,
                description: format!(
                    "Coupling strength {:.2} exceeds max initial {:.2}",
                    proposal.strength, profile.coupling_limits.max_initial_strength
                ),
                severity: ViolationSeverity::Violation,
            });
        }

        // Check concurrent couplings
        if proposal.current_couplings >= profile.coupling_limits.max_concurrent_couplings {
            violations.push(ProfileViolation {
                profile_type: profile.name.clone(),
                dimension: ViolationDimension::CouplingLimits,
                description: format!(
                    "Concurrent couplings {} would exceed limit {}",
                    proposal.current_couplings + 1,
                    profile.coupling_limits.max_concurrent_couplings
                ),
                severity: ViolationSeverity::Violation,
            });
        }

        // Check asymmetric coupling
        if proposal.is_asymmetric && !profile.coupling_limits.allow_asymmetric {
            violations.push(ProfileViolation {
                profile_type: profile.name.clone(),
                dimension: ViolationDimension::CouplingLimits,
                description: "Asymmetric coupling not permitted by profile".into(),
                severity: ViolationSeverity::Violation,
            });
        }

        // Check consent level
        if proposal.consent_provided < profile.coupling_limits.consent_required {
            violations.push(ProfileViolation {
                profile_type: profile.name.clone(),
                dimension: ViolationDimension::CouplingLimits,
                description: format!(
                    "Consent level {:?} insufficient, requires {:?}",
                    proposal.consent_provided, profile.coupling_limits.consent_required
                ),
                severity: ViolationSeverity::Critical,
            });
        }

        // Check attention fraction
        if proposal.attention_fraction > profile.attention_budget.max_single_coupling_fraction {
            violations.push(ProfileViolation {
                profile_type: profile.name.clone(),
                dimension: ViolationDimension::AttentionBudget,
                description: format!(
                    "Attention fraction {:.2} exceeds max single coupling fraction {:.2}",
                    proposal.attention_fraction,
                    profile.attention_budget.max_single_coupling_fraction
                ),
                severity: ViolationSeverity::Violation,
            });
        }

        // Warn if approaching limits
        let strength_ratio = proposal.strength / profile.coupling_limits.max_initial_strength;
        if strength_ratio > 0.8 && violations.is_empty() {
            warnings.push(format!(
                "Coupling strength at {:.0}% of profile limit",
                strength_ratio * 100.0
            ));
        }

        if !violations.is_empty() {
            warn!(
                profile = profile.name.as_str(),
                violations = violations.len(),
                "Coupling proposal denied by profile enforcer"
            );
            return EnforcementResult::Denied {
                reason: format!(
                    "{} profile violations for coupling proposal",
                    violations.len()
                ),
                violations,
            };
        }

        if !warnings.is_empty() {
            debug!(
                profile = profile.name.as_str(),
                warnings = warnings.len(),
                "Coupling proposal permitted with warnings"
            );
            return EnforcementResult::PermittedWithWarnings(warnings);
        }

        EnforcementResult::Permitted
    }

    /// Check a commitment proposal against a profile.
    pub fn check_commitment(
        profile: &WorldlineProfile,
        proposal: &CommitmentProposal,
    ) -> EnforcementResult {
        let mut violations = Vec::new();
        let mut warnings = Vec::new();

        // Check domain permission
        if !profile.commitment_authority.allowed_domains.contains(&proposal.domain) {
            violations.push(ProfileViolation {
                profile_type: profile.name.clone(),
                dimension: ViolationDimension::CommitmentAuthority,
                description: format!(
                    "Domain {:?} not in profile's allowed domains",
                    proposal.domain
                ),
                severity: ViolationSeverity::Violation,
            });
        }

        // Check risk class
        if proposal.risk_class > profile.commitment_authority.max_risk_class {
            let severity = if proposal.risk_class >= RiskClass::Critical {
                ViolationSeverity::Critical
            } else {
                ViolationSeverity::Violation
            };

            violations.push(ProfileViolation {
                profile_type: profile.name.clone(),
                dimension: ViolationDimension::CommitmentAuthority,
                description: format!(
                    "Risk class {:?} exceeds profile maximum {:?}",
                    proposal.risk_class, profile.commitment_authority.max_risk_class
                ),
                severity,
            });
        }

        // Check irreversibility
        let is_irreversible = matches!(proposal.reversibility, Reversibility::Irreversible);
        if is_irreversible && !profile.commitment_authority.allow_irreversible {
            violations.push(ProfileViolation {
                profile_type: profile.name.clone(),
                dimension: ViolationDimension::CommitmentAuthority,
                description: "Irreversible commitment not permitted by profile".into(),
                severity: ViolationSeverity::Critical,
            });
        }

        // Check reversibility preference
        match &profile.consequence_scope.reversibility_preference {
            ReversibilityPreference::RequireReversible => {
                if !matches!(proposal.reversibility, Reversibility::FullyReversible) {
                    violations.push(ProfileViolation {
                        profile_type: profile.name.clone(),
                        dimension: ViolationDimension::ConsequenceScope,
                        description: "Profile requires fully reversible commitments".into(),
                        severity: ViolationSeverity::Violation,
                    });
                }
            }
            ReversibilityPreference::PreferReversible => {
                if is_irreversible {
                    warnings.push("Profile prefers reversible commitments".into());
                }
            }
            _ => {}
        }

        // Check affected parties
        if let Some(max_parties) = profile.commitment_authority.max_affected_parties {
            if proposal.affected_parties > max_parties {
                violations.push(ProfileViolation {
                    profile_type: profile.name.clone(),
                    dimension: ViolationDimension::ConsequenceScope,
                    description: format!(
                        "Affected parties {} exceeds limit {}",
                        proposal.affected_parties, max_parties
                    ),
                    severity: ViolationSeverity::Violation,
                });
            }
        }

        // Check consequence value
        if let (Some(value), Some(max_value)) =
            (proposal.consequence_value, profile.commitment_authority.max_consequence_value)
        {
            if value > max_value {
                violations.push(ProfileViolation {
                    profile_type: profile.name.clone(),
                    dimension: ViolationDimension::CommitmentAuthority,
                    description: format!(
                        "Consequence value {} exceeds profile limit {}",
                        value, max_value
                    ),
                    severity: ViolationSeverity::Violation,
                });
            }
        }

        // Check human involvement requirements
        let needs_human_for_risk = profile.human_involvement.require_human_for_high_risk
            && proposal.risk_class >= RiskClass::High;
        let needs_human_for_irreversible =
            profile.human_involvement.require_human_for_irreversible && is_irreversible;

        if (needs_human_for_risk || needs_human_for_irreversible) && !proposal.has_human_approval {
            violations.push(ProfileViolation {
                profile_type: profile.name.clone(),
                dimension: ViolationDimension::HumanInvolvement,
                description: if needs_human_for_risk {
                    "Human approval required for high-risk operations".into()
                } else {
                    "Human approval required for irreversible operations".into()
                },
                severity: ViolationSeverity::Critical,
            });
        }

        if !violations.is_empty() {
            warn!(
                profile = profile.name.as_str(),
                violations = violations.len(),
                "Commitment proposal denied by profile enforcer"
            );
            return EnforcementResult::Denied {
                reason: format!(
                    "{} profile violations for commitment proposal",
                    violations.len()
                ),
                violations,
            };
        }

        if !warnings.is_empty() {
            return EnforcementResult::PermittedWithWarnings(warnings);
        }

        EnforcementResult::Permitted
    }

    /// Check if a profile requires human oversight for a given operation.
    pub fn requires_human_oversight(
        profile: &WorldlineProfile,
        risk_class: RiskClass,
        is_irreversible: bool,
    ) -> bool {
        match profile.human_involvement.oversight_level {
            OversightLevel::FullOversight => true,
            OversightLevel::ApprovalForHighRisk => {
                risk_class >= RiskClass::High || is_irreversible
            }
            OversightLevel::Notification => is_irreversible,
            OversightLevel::AuditOnly => false,
            OversightLevel::None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::{
        agent_profile, coordination_profile, financial_profile, human_profile, world_profile,
    };

    fn valid_coupling_proposal() -> CouplingProposal {
        CouplingProposal {
            strength: 0.3,
            current_couplings: 2,
            is_asymmetric: false,
            consent_provided: ConsentLevel::Informed,
            attention_fraction: 0.2,
        }
    }

    fn valid_commitment_proposal() -> CommitmentProposal {
        CommitmentProposal {
            domain: EffectDomain::Communication,
            risk_class: RiskClass::Low,
            reversibility: Reversibility::FullyReversible,
            affected_parties: 1,
            consequence_value: Some(10),
            has_human_approval: false,
        }
    }

    #[test]
    fn human_profile_permits_valid_coupling() {
        let profile = human_profile();
        let proposal = valid_coupling_proposal();
        let result = ProfileEnforcer::check_coupling(&profile, &proposal);
        assert!(result.is_permitted());
    }

    #[test]
    fn human_profile_denies_strong_coupling() {
        let profile = human_profile();
        let mut proposal = valid_coupling_proposal();
        proposal.strength = 0.9; // Exceeds human's 0.5 limit
        let result = ProfileEnforcer::check_coupling(&profile, &proposal);
        assert!(result.is_denied());
    }

    #[test]
    fn human_profile_denies_insufficient_consent() {
        let profile = human_profile();
        let mut proposal = valid_coupling_proposal();
        proposal.consent_provided = ConsentLevel::Implicit; // Human requires Informed
        let result = ProfileEnforcer::check_coupling(&profile, &proposal);
        assert!(result.is_denied());
    }

    #[test]
    fn human_profile_denies_asymmetric_coupling() {
        let profile = human_profile();
        let mut proposal = valid_coupling_proposal();
        proposal.is_asymmetric = true;
        let result = ProfileEnforcer::check_coupling(&profile, &proposal);
        assert!(result.is_denied());
    }

    #[test]
    fn agent_profile_permits_asymmetric_coupling() {
        let profile = agent_profile();
        let mut proposal = valid_coupling_proposal();
        proposal.is_asymmetric = true;
        proposal.consent_provided = ConsentLevel::Explicit;
        let result = ProfileEnforcer::check_coupling(&profile, &proposal);
        assert!(result.is_permitted());
    }

    #[test]
    fn world_profile_permits_many_couplings() {
        let profile = world_profile();
        let mut proposal = valid_coupling_proposal();
        proposal.current_couplings = 49; // World allows 50
        proposal.consent_provided = ConsentLevel::Implicit;
        proposal.strength = 0.1;
        proposal.attention_fraction = 0.05;
        let result = ProfileEnforcer::check_coupling(&profile, &proposal);
        assert!(result.is_permitted());
    }

    #[test]
    fn world_profile_denies_exceeding_concurrent_limit() {
        let profile = world_profile();
        let mut proposal = valid_coupling_proposal();
        proposal.current_couplings = 50; // At limit
        proposal.consent_provided = ConsentLevel::Implicit;
        proposal.strength = 0.1;
        proposal.attention_fraction = 0.05;
        let result = ProfileEnforcer::check_coupling(&profile, &proposal);
        assert!(result.is_denied());
    }

    #[test]
    fn human_profile_permits_valid_commitment() {
        let profile = human_profile();
        let proposal = valid_commitment_proposal();
        let result = ProfileEnforcer::check_commitment(&profile, &proposal);
        assert!(result.is_permitted());
    }

    #[test]
    fn human_profile_denies_financial_domain() {
        let profile = human_profile();
        let mut proposal = valid_commitment_proposal();
        proposal.domain = EffectDomain::Financial;
        let result = ProfileEnforcer::check_commitment(&profile, &proposal);
        assert!(result.is_denied());
    }

    #[test]
    fn human_profile_denies_high_risk_without_approval() {
        let profile = human_profile();
        let mut proposal = valid_commitment_proposal();
        proposal.risk_class = RiskClass::High;
        let result = ProfileEnforcer::check_commitment(&profile, &proposal);
        assert!(result.is_denied());
    }

    #[test]
    fn human_profile_denies_irreversible() {
        let profile = human_profile();
        let mut proposal = valid_commitment_proposal();
        proposal.reversibility = Reversibility::Irreversible;
        let result = ProfileEnforcer::check_commitment(&profile, &proposal);
        assert!(result.is_denied());
    }

    #[test]
    fn financial_profile_denies_medium_risk() {
        let profile = financial_profile();
        let mut proposal = valid_commitment_proposal();
        proposal.domain = EffectDomain::Financial;
        proposal.risk_class = RiskClass::Medium; // Financial max is Low
        let result = ProfileEnforcer::check_commitment(&profile, &proposal);
        assert!(result.is_denied());
    }

    #[test]
    fn financial_profile_permits_low_risk_financial() {
        let profile = financial_profile();
        let proposal = CommitmentProposal {
            domain: EffectDomain::Financial,
            risk_class: RiskClass::Low,
            reversibility: Reversibility::FullyReversible,
            affected_parties: 2,
            consequence_value: Some(100),
            has_human_approval: false,
        };
        let result = ProfileEnforcer::check_commitment(&profile, &proposal);
        assert!(result.is_permitted());
    }

    #[test]
    fn agent_profile_denies_exceeding_consequence_value() {
        let profile = agent_profile();
        let mut proposal = valid_commitment_proposal();
        proposal.consequence_value = Some(10000); // Agent limit is 5000
        let result = ProfileEnforcer::check_commitment(&profile, &proposal);
        assert!(result.is_denied());
    }

    #[test]
    fn agent_profile_denies_exceeding_affected_parties() {
        let profile = agent_profile();
        let mut proposal = valid_commitment_proposal();
        proposal.affected_parties = 25; // Agent limit is 20
        let result = ProfileEnforcer::check_commitment(&profile, &proposal);
        assert!(result.is_denied());
    }

    #[test]
    fn coordination_allows_infrastructure() {
        let profile = coordination_profile();
        let proposal = CommitmentProposal {
            domain: EffectDomain::Infrastructure,
            risk_class: RiskClass::Medium,
            reversibility: Reversibility::FullyReversible,
            affected_parties: 10,
            consequence_value: Some(1000),
            has_human_approval: false,
        };
        let result = ProfileEnforcer::check_commitment(&profile, &proposal);
        assert!(result.is_permitted());
    }

    #[test]
    fn requires_human_oversight_full() {
        let profile = human_profile();
        // Full oversight means always required
        assert!(ProfileEnforcer::requires_human_oversight(&profile, RiskClass::Low, false));
    }

    #[test]
    fn requires_human_oversight_approval_for_high_risk() {
        let profile = agent_profile();
        assert!(!ProfileEnforcer::requires_human_oversight(&profile, RiskClass::Low, false));
        assert!(ProfileEnforcer::requires_human_oversight(&profile, RiskClass::High, false));
        assert!(ProfileEnforcer::requires_human_oversight(&profile, RiskClass::Low, true));
    }

    #[test]
    fn requires_human_oversight_audit_only() {
        let profile = world_profile();
        assert!(!ProfileEnforcer::requires_human_oversight(&profile, RiskClass::High, false));
        assert!(!ProfileEnforcer::requires_human_oversight(&profile, RiskClass::Critical, true));
    }

    #[test]
    fn coupling_warning_when_approaching_limit() {
        let profile = agent_profile(); // max_initial: 0.6
        let mut proposal = valid_coupling_proposal();
        proposal.strength = 0.55; // 91% of limit
        proposal.consent_provided = ConsentLevel::Explicit;
        let result = ProfileEnforcer::check_coupling(&profile, &proposal);
        assert!(matches!(result, EnforcementResult::PermittedWithWarnings(_)));
    }

    #[test]
    fn commitment_warning_for_prefer_reversible() {
        let profile = agent_profile(); // PreferReversible
        let mut proposal = valid_commitment_proposal();
        proposal.reversibility = Reversibility::Irreversible;
        // Agent doesn't allow irreversible, so this should be denied
        let result = ProfileEnforcer::check_commitment(&profile, &proposal);
        assert!(result.is_denied());
    }
}
