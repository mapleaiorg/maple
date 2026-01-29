//! RCL-Commitment Layer - The ONLY executable layer
//!
//! Commitments are the boundary between intention and action.
//! Only Commitments can be executed, and only after AAS approval.

#![deny(unsafe_code)]

use rcl_types::{
    CapabilityRef, ContinuityRef, EffectDomain, IdentityRef, ResonanceArtifact, ResonanceType,
    ResourceLimits, ScopeConstraint, TemporalValidity,
};
use serde::{Deserialize, Serialize};

/// Unique identifier for a commitment
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitmentId(pub String);

impl CommitmentId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn short(&self) -> String {
        self.0.chars().take(8).collect()
    }
}

impl std::fmt::Display for CommitmentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An RCL Commitment - the ONLY executable type
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RclCommitment {
    pub commitment_id: CommitmentId,
    pub principal: IdentityRef,
    pub continuity_ref: ContinuityRef,
    pub effect_domain: EffectDomain,
    pub intended_outcome: IntendedOutcome,
    pub scope: ScopeConstraint,
    pub targets: Vec<Target>,
    pub limits: ResourceLimits,
    pub temporal_validity: TemporalValidity,
    pub reversibility: Reversibility,
    pub required_capabilities: Vec<CapabilityRef>,
    pub evidence_requirements: EvidenceRequirements,
    pub audit: AuditMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent_ref: Option<String>,
    pub risk_classification: RiskClassification,
    pub policy_tags: Vec<String>,
    pub schema_version: String,
}

impl RclCommitment {
    pub fn builder(principal: IdentityRef, domain: EffectDomain) -> CommitmentBuilder {
        CommitmentBuilder::new(principal, domain)
    }

    pub fn validate(&self) -> Result<(), CommitmentValidationError> {
        Ok(())
    }

    #[inline]
    pub fn is_potentially_executable(&self) -> bool {
        true
    }

    pub fn is_valid_at(&self, time: chrono::DateTime<chrono::Utc>) -> bool {
        self.temporal_validity.is_valid_at(time)
    }
}

impl ResonanceArtifact for RclCommitment {
    fn resonance_type(&self) -> ResonanceType {
        ResonanceType::Commitment
    }

    fn artifact_id(&self) -> &str {
        &self.commitment_id.0
    }

    fn is_executable(&self) -> bool {
        true
    }
}

/// The intended outcome of a commitment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntendedOutcome {
    pub description: String,
    #[serde(default)]
    pub success_criteria: Vec<String>,
}

impl IntendedOutcome {
    pub fn new(desc: impl Into<String>) -> Self {
        Self {
            description: desc.into(),
            success_criteria: vec![],
        }
    }

    pub fn with_criteria(mut self, criteria: impl Into<String>) -> Self {
        self.success_criteria.push(criteria.into());
        self
    }
}

/// A target of a commitment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Target {
    pub target_type: TargetType,
    pub identifier: String,
}

impl Target {
    pub fn new(target_type: TargetType, id: impl Into<String>) -> Self {
        Self {
            target_type,
            identifier: id.into(),
        }
    }

    pub fn resource(id: impl Into<String>) -> Self {
        Self::new(TargetType::Resource, id)
    }

    pub fn identity(id: impl Into<String>) -> Self {
        Self::new(TargetType::Identity, id)
    }
}

/// Types of targets
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetType {
    Identity,
    Resource,
    System,
    Location,
}

/// Reversibility of an action
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Reversibility {
    #[default]
    Reversible,
    PartiallyReversible(String),
    Irreversible,
}

impl Reversibility {
    pub fn is_irreversible(&self) -> bool {
        matches!(self, Reversibility::Irreversible)
    }
}

/// Evidence requirements for audit
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvidenceRequirements {
    pub audit_level: AuditLevel,
}

impl EvidenceRequirements {
    pub fn standard() -> Self {
        Self {
            audit_level: AuditLevel::Standard,
        }
    }

    pub fn comprehensive() -> Self {
        Self {
            audit_level: AuditLevel::Comprehensive,
        }
    }
}

impl Default for EvidenceRequirements {
    fn default() -> Self {
        Self::standard()
    }
}

/// Audit level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AuditLevel {
    Minimal,
    #[default]
    Standard,
    Comprehensive,
    Forensic,
}

/// Audit metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditMetadata {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub created_by: IdentityRef,
    pub trace_id: String,
}

impl AuditMetadata {
    pub fn new(created_by: IdentityRef) -> Self {
        Self {
            created_at: chrono::Utc::now(),
            created_by,
            trace_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

/// Risk classification
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RiskClassification {
    Low,
    #[default]
    Medium,
    High,
    Critical,
}

/// Builder for commitments
pub struct CommitmentBuilder {
    principal: IdentityRef,
    effect_domain: EffectDomain,
    intended_outcome: Option<IntendedOutcome>,
    scope: Option<ScopeConstraint>,
    targets: Vec<Target>,
    limits: Option<ResourceLimits>,
    temporal_validity: Option<TemporalValidity>,
    reversibility: Reversibility,
    capabilities: Vec<CapabilityRef>,
    evidence_requirements: Option<EvidenceRequirements>,
    intent_ref: Option<String>,
    policy_tags: Vec<String>,
}

impl CommitmentBuilder {
    pub fn new(principal: IdentityRef, domain: EffectDomain) -> Self {
        Self {
            principal,
            effect_domain: domain,
            intended_outcome: None,
            scope: None,
            targets: vec![],
            limits: None,
            temporal_validity: None,
            reversibility: Reversibility::default(),
            capabilities: vec![],
            evidence_requirements: None,
            intent_ref: None,
            policy_tags: vec![],
        }
    }

    pub fn with_outcome(mut self, outcome: IntendedOutcome) -> Self {
        self.intended_outcome = Some(outcome);
        self
    }

    pub fn with_scope(mut self, scope: ScopeConstraint) -> Self {
        self.scope = Some(scope);
        self
    }

    pub fn with_target(mut self, target: Target) -> Self {
        self.targets.push(target);
        self
    }

    pub fn with_limits(mut self, limits: ResourceLimits) -> Self {
        self.limits = Some(limits);
        self
    }

    pub fn with_validity(mut self, validity: TemporalValidity) -> Self {
        self.temporal_validity = Some(validity);
        self
    }

    pub fn with_reversibility(mut self, reversibility: Reversibility) -> Self {
        self.reversibility = reversibility;
        self
    }

    pub fn with_capability(mut self, capability: CapabilityRef) -> Self {
        self.capabilities.push(capability);
        self
    }

    pub fn with_evidence(mut self, evidence: EvidenceRequirements) -> Self {
        self.evidence_requirements = Some(evidence);
        self
    }

    pub fn with_intent_ref(mut self, intent_ref: impl Into<String>) -> Self {
        self.intent_ref = Some(intent_ref.into());
        self
    }

    pub fn with_policy_tag(mut self, tag: impl Into<String>) -> Self {
        self.policy_tags.push(tag.into());
        self
    }

    pub fn build(self) -> Result<RclCommitment, CommitmentBuildError> {
        let intended_outcome = self
            .intended_outcome
            .unwrap_or_else(|| IntendedOutcome::new("Unspecified outcome"));

        let scope = self.scope.unwrap_or_default();

        // Auto-generate a default target if none specified
        let targets = if self.targets.is_empty() {
            vec![Target::resource("default")]
        } else {
            self.targets
        };

        let limits = self.limits.unwrap_or_default();
        let temporal_validity = self.temporal_validity.unwrap_or_else(TemporalValidity::unbounded);
        let evidence_requirements = self.evidence_requirements.unwrap_or_default();

        let commitment_id = CommitmentId::generate();
        let continuity_ref = self.principal.continuity_ref.clone();
        let audit = AuditMetadata::new(self.principal.clone());

        Ok(RclCommitment {
            commitment_id,
            principal: self.principal,
            continuity_ref,
            effect_domain: self.effect_domain,
            intended_outcome,
            scope,
            targets,
            limits,
            temporal_validity,
            reversibility: self.reversibility,
            required_capabilities: self.capabilities,
            evidence_requirements,
            audit,
            intent_ref: self.intent_ref,
            risk_classification: RiskClassification::default(),
            policy_tags: self.policy_tags,
            schema_version: rcl_types::SCHEMA_VERSION.to_string(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CommitmentBuildError {
    #[error("Missing required field: {0}")]
    MissingField(String),
}

#[derive(Debug, thiserror::Error)]
pub enum CommitmentValidationError {
    #[error("Wrong resonance type: {0:?}")]
    WrongResonanceType(ResonanceType),
    #[error("No capabilities")]
    NoCapabilities,
    #[error("Invalid scope")]
    InvalidScope,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_builder() {
        let identity = IdentityRef::new("test-agent");
        let commitment = CommitmentBuilder::new(identity, EffectDomain::Computation)
            .with_outcome(IntendedOutcome::new("Test outcome"))
            .with_scope(ScopeConstraint::default())
            .build()
            .unwrap();

        assert_eq!(commitment.effect_domain, EffectDomain::Computation);
    }
}
