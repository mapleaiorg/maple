//! Commitment Builder
//!
//! Builder pattern for constructing valid RCF-Commitment artifacts.

use crate::types::*;
use crate::{CommitmentId, RcfCommitment};
use rcf_types::{
    CapabilityRef, ContinuityRef, EffectDomain, IdentityRef, ResonanceType, ResourceLimits,
    TemporalAnchor,
};

/// Builder for RcfCommitment
///
/// Ensures all required fields are provided before building.
#[derive(Default)]
pub struct CommitmentBuilder {
    declaring_identity: Option<IdentityRef>,
    effect_domain: Option<EffectDomain>,
    intended_outcome: Option<IntendedOutcome>,
    scope: Option<CommitmentScope>,
    targets: Vec<Target>,
    limits: Option<ResourceLimits>,
    effective_from: Option<TemporalAnchor>,
    expires_at: Option<TemporalAnchor>,
    capabilities: Vec<CapabilityRef>,
    evidence_requirements: Option<EvidenceRequirements>,
    intent_ref: Option<String>,
    human_cosign: Option<HumanCosignRequirement>,
    risk_classification: Option<RiskClassification>,
    policy_tags: Vec<String>,
}

impl CommitmentBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the declaring identity (REQUIRED)
    pub fn declaring_identity(mut self, identity: IdentityRef) -> Self {
        self.declaring_identity = Some(identity);
        self
    }

    /// Set the effect domain (REQUIRED)
    pub fn effect_domain(mut self, domain: EffectDomain) -> Self {
        self.effect_domain = Some(domain);
        self
    }

    /// Set the intended outcome (REQUIRED)
    pub fn intended_outcome(mut self, outcome: IntendedOutcome) -> Self {
        self.intended_outcome = Some(outcome);
        self
    }

    /// Set the scope (REQUIRED)
    pub fn scope(mut self, scope: CommitmentScope) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Add a target (at least one REQUIRED)
    pub fn target(mut self, target: Target) -> Self {
        self.targets.push(target);
        self
    }

    /// Add multiple targets
    pub fn targets(mut self, targets: impl IntoIterator<Item = Target>) -> Self {
        self.targets.extend(targets);
        self
    }

    /// Set resource limits (REQUIRED)
    pub fn limits(mut self, limits: ResourceLimits) -> Self {
        self.limits = Some(limits);
        self
    }

    /// Set when the commitment becomes effective
    pub fn effective_from(mut self, anchor: TemporalAnchor) -> Self {
        self.effective_from = Some(anchor);
        self
    }

    /// Set when the commitment expires (REQUIRED)
    pub fn expires_at(mut self, anchor: TemporalAnchor) -> Self {
        self.expires_at = Some(anchor);
        self
    }

    /// Set expiration as seconds from now
    pub fn expires_in_secs(mut self, secs: i64) -> Self {
        let expires = TemporalAnchor::at(chrono::Utc::now() + chrono::Duration::seconds(secs));
        self.expires_at = Some(expires);
        self
    }

    /// Add a required capability (at least one REQUIRED)
    pub fn capability(mut self, cap: CapabilityRef) -> Self {
        self.capabilities.push(cap);
        self
    }

    /// Add multiple capabilities
    pub fn capabilities(mut self, caps: impl IntoIterator<Item = CapabilityRef>) -> Self {
        self.capabilities.extend(caps);
        self
    }

    /// Set evidence requirements (REQUIRED)
    pub fn evidence_requirements(mut self, reqs: EvidenceRequirements) -> Self {
        self.evidence_requirements = Some(reqs);
        self
    }

    /// Set intent reference (optional)
    pub fn intent_ref(mut self, intent_id: impl Into<String>) -> Self {
        self.intent_ref = Some(intent_id.into());
        self
    }

    /// Set human co-signature requirement (optional)
    pub fn human_cosign(mut self, cosign: HumanCosignRequirement) -> Self {
        self.human_cosign = Some(cosign);
        self
    }

    /// Set risk classification (defaults to Medium)
    pub fn risk_classification(mut self, risk: RiskClassification) -> Self {
        self.risk_classification = Some(risk);
        self
    }

    /// Add a policy tag
    pub fn policy_tag(mut self, tag: impl Into<String>) -> Self {
        self.policy_tags.push(tag.into());
        self
    }

    /// Add multiple policy tags
    pub fn policy_tags(mut self, tags: impl IntoIterator<Item = String>) -> Self {
        self.policy_tags.extend(tags);
        self
    }

    /// Build the commitment
    ///
    /// Validates all required fields are present and returns the commitment.
    pub fn build(self) -> Result<RcfCommitment, CommitmentBuildError> {
        // Validate and extract all required fields

        let declaring_identity = self
            .declaring_identity
            .ok_or(CommitmentBuildError::MissingDeclaringIdentity)?;

        let effect_domain = self
            .effect_domain
            .ok_or(CommitmentBuildError::MissingEffectDomain)?;

        let intended_outcome = self
            .intended_outcome
            .ok_or(CommitmentBuildError::MissingIntendedOutcome)?;

        let scope = self.scope.ok_or(CommitmentBuildError::MissingScope)?;

        if self.targets.is_empty() {
            return Err(CommitmentBuildError::MissingTargets);
        }

        let limits = self.limits.ok_or(CommitmentBuildError::MissingLimits)?;

        let expires_at = self
            .expires_at
            .ok_or(CommitmentBuildError::MissingExpiration)?;

        if self.capabilities.is_empty() {
            return Err(CommitmentBuildError::NoCapabilities);
        }

        let evidence_requirements = self
            .evidence_requirements
            .ok_or(CommitmentBuildError::MissingEvidenceRequirements)?;

        // Set defaults for optional fields
        let continuity_ref = declaring_identity.continuity_ref.clone();
        let declared_at = TemporalAnchor::now();
        let effective_from = self.effective_from.unwrap_or_else(|| declared_at.clone());
        let risk_classification = self.risk_classification.unwrap_or_default();

        // Create audit metadata
        let audit = AuditMetadata::new(declaring_identity.clone());

        // Generate commitment ID
        let id = CommitmentId::generate();

        // Build the commitment (without hash first)
        let mut commitment = RcfCommitment {
            id,
            resonance_type: ResonanceType::Commitment,
            declaring_identity,
            continuity_ref,
            effect_domain,
            intended_outcome,
            scope,
            targets: self.targets,
            limits,
            declared_at,
            effective_from,
            expires_at,
            required_capabilities: self.capabilities,
            evidence_requirements,
            audit,
            intent_ref: self.intent_ref,
            human_cosign: self.human_cosign,
            risk_classification,
            policy_tags: self.policy_tags,
            declaration_hash: [0u8; 32], // Placeholder
            schema_version: rcf_types::SCHEMA_VERSION.to_string(),
        };

        // Compute and set the declaration hash
        commitment.declaration_hash = commitment.compute_hash();

        Ok(commitment)
    }
}

/// Errors that can occur when building a commitment
#[derive(Debug, thiserror::Error)]
pub enum CommitmentBuildError {
    /// Missing declaring identity
    #[error("Missing required field: declaring_identity")]
    MissingDeclaringIdentity,

    /// Missing effect domain
    #[error("Missing required field: effect_domain")]
    MissingEffectDomain,

    /// Missing intended outcome
    #[error("Missing required field: intended_outcome")]
    MissingIntendedOutcome,

    /// Missing scope
    #[error("Missing required field: scope")]
    MissingScope,

    /// Missing targets
    #[error("Missing required field: targets (at least one target required)")]
    MissingTargets,

    /// Missing limits
    #[error("Missing required field: limits")]
    MissingLimits,

    /// Missing expiration
    #[error("Missing required field: expires_at")]
    MissingExpiration,

    /// No capabilities specified
    #[error("At least one capability must be specified")]
    NoCapabilities,

    /// Missing evidence requirements
    #[error("Missing required field: evidence_requirements")]
    MissingEvidenceRequirements,

    /// Intent confidence too low
    #[error("Intent confidence ({confidence:.2}) is below required threshold ({required:.2})")]
    InsufficientIntentConfidence { confidence: f64, required: f64 },
}

impl From<CommitmentBuildError> for rcf_types::RcfError {
    fn from(err: CommitmentBuildError) -> Self {
        rcf_types::RcfError::ValidationError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcf_types::{ScopeConstraint, TemporalValidity};

    fn create_test_identity() -> IdentityRef {
        IdentityRef::new("test-agent")
    }

    fn create_test_capability() -> CapabilityRef {
        CapabilityRef::new(
            "cap-001",
            EffectDomain::Communication,
            ScopeConstraint::wildcard(),
            TemporalValidity::from_now_secs(3600),
            create_test_identity(),
        )
    }

    #[test]
    fn test_builder_success() {
        let commitment = CommitmentBuilder::new()
            .declaring_identity(create_test_identity())
            .effect_domain(EffectDomain::Communication)
            .intended_outcome(IntendedOutcome::new("Test outcome"))
            .scope(CommitmentScope::new("Test scope"))
            .target(Target::new(TargetType::Resource, "test-resource"))
            .limits(ResourceLimits::new())
            .expires_in_secs(3600)
            .capability(create_test_capability())
            .evidence_requirements(EvidenceRequirements::standard())
            .build();

        assert!(commitment.is_ok());
    }

    #[test]
    fn test_builder_missing_identity() {
        let result = CommitmentBuilder::new()
            .effect_domain(EffectDomain::Communication)
            .intended_outcome(IntendedOutcome::new("Test"))
            .scope(CommitmentScope::new("Test"))
            .target(Target::new(TargetType::Resource, "test"))
            .limits(ResourceLimits::new())
            .expires_in_secs(3600)
            .capability(create_test_capability())
            .evidence_requirements(EvidenceRequirements::standard())
            .build();

        assert!(matches!(
            result,
            Err(CommitmentBuildError::MissingDeclaringIdentity)
        ));
    }

    #[test]
    fn test_builder_missing_capabilities() {
        let result = CommitmentBuilder::new()
            .declaring_identity(create_test_identity())
            .effect_domain(EffectDomain::Communication)
            .intended_outcome(IntendedOutcome::new("Test"))
            .scope(CommitmentScope::new("Test"))
            .target(Target::new(TargetType::Resource, "test"))
            .limits(ResourceLimits::new())
            .expires_in_secs(3600)
            .evidence_requirements(EvidenceRequirements::standard())
            .build();

        assert!(matches!(result, Err(CommitmentBuildError::NoCapabilities)));
    }

    #[test]
    fn test_builder_with_all_options() {
        let commitment = CommitmentBuilder::new()
            .declaring_identity(create_test_identity())
            .effect_domain(EffectDomain::Finance)
            .intended_outcome(
                IntendedOutcome::new("Transfer funds")
                    .with_criterion("Amount matches request")
                    .with_effect(ExpectedEffect::new("debit", "source-account")),
            )
            .scope(
                CommitmentScope::new("Fund transfer")
                    .with_boundary(ScopeBoundary::new("amount", "max $1000")),
            )
            .target(Target::new(TargetType::Resource, "bank-account"))
            .limits(ResourceLimits::new().with_max_value(100000))
            .expires_in_secs(3600)
            .capability(create_test_capability())
            .evidence_requirements(EvidenceRequirements::comprehensive())
            .intent_ref("intent-123")
            .human_cosign(HumanCosignRequirement::new(1))
            .risk_classification(RiskClassification::High)
            .policy_tag("finance")
            .policy_tag("transfer")
            .build();

        assert!(commitment.is_ok());
        let commitment = commitment.unwrap();
        assert_eq!(commitment.intent_ref, Some("intent-123".to_string()));
        assert!(commitment.human_cosign.is_some());
        assert_eq!(commitment.risk_classification, RiskClassification::High);
        assert_eq!(commitment.policy_tags.len(), 2);
    }
}
