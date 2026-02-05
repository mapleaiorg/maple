//! AAS Service - The unified Agent Accountability Service
//!
//! This is the ONLY authority layer in the Maple framework.
//! All commitments MUST pass through AAS before execution.

#![deny(unsafe_code)]

use aas_adjudication::{
    AdjudicationError, Adjudicator, PolicyEvaluationInput, RuleActionInput, RuleResultInput,
};
use aas_capability::{CapabilityCheckResult, CapabilityError, CapabilityRegistry, GrantRequest};
use aas_identity::{IdentityError, IdentityRegistry, RegistrationRequest, VerificationResult};
use aas_ledger::{AccountabilityLedger, LedgerError, LedgerQuery, LedgerStatistics};
use aas_policy::{EvaluationContext, PolicyEngine, PolicyError};
use aas_types::{AgentId, CommitmentOutcome, LedgerEntry, PolicyDecisionCard};
use maple_storage::MapleStorage;
use rcf_commitment::{CommitmentId, RcfCommitment};
use rcf_types::{EffectDomain, ScopeConstraint};
use std::sync::Arc;
use thiserror::Error;

/// The Agent Accountability Service
pub struct AasService {
    identity: Arc<IdentityRegistry>,
    capability: Arc<CapabilityRegistry>,
    policy: Arc<PolicyEngine>,
    adjudicator: Arc<Adjudicator>,
    ledger: Arc<AccountabilityLedger>,
}

impl AasService {
    /// Create a new AAS instance
    pub fn new() -> Self {
        Self {
            identity: Arc::new(IdentityRegistry::new()),
            capability: Arc::new(CapabilityRegistry::new()),
            policy: Arc::new(PolicyEngine::with_defaults()),
            adjudicator: Arc::new(Adjudicator::new()),
            ledger: Arc::new(AccountabilityLedger::new()),
        }
    }

    /// Create with an explicit MAPLE storage backend for durable ledger state.
    pub fn with_storage(storage: Arc<dyn MapleStorage>) -> Self {
        Self {
            identity: Arc::new(IdentityRegistry::new()),
            capability: Arc::new(CapabilityRegistry::new()),
            policy: Arc::new(PolicyEngine::with_defaults()),
            adjudicator: Arc::new(Adjudicator::new()),
            ledger: Arc::new(AccountabilityLedger::with_storage(storage)),
        }
    }

    /// Create with custom components
    pub fn with_components(
        identity: IdentityRegistry,
        capability: CapabilityRegistry,
        policy: PolicyEngine,
        adjudicator: Adjudicator,
        ledger: AccountabilityLedger,
    ) -> Self {
        Self {
            identity: Arc::new(identity),
            capability: Arc::new(capability),
            policy: Arc::new(policy),
            adjudicator: Arc::new(adjudicator),
            ledger: Arc::new(ledger),
        }
    }

    // ============ Identity Operations ============

    /// Register a new agent
    pub fn register_agent(
        &self,
        request: RegistrationRequest,
    ) -> Result<aas_identity::RegisteredAgent, AasError> {
        self.identity.register(request).map_err(AasError::Identity)
    }

    /// Verify an identity
    pub fn verify_identity(
        &self,
        identity: &rcf_types::IdentityRef,
    ) -> Result<VerificationResult, AasError> {
        self.identity.verify(identity).map_err(AasError::Identity)
    }

    // ============ Capability Operations ============

    /// Grant a capability
    pub fn grant_capability(
        &self,
        request: GrantRequest,
    ) -> Result<aas_capability::CapabilityGrant, AasError> {
        self.capability.grant(request).map_err(AasError::Capability)
    }

    /// Check if an agent has a capability
    pub fn check_capability(
        &self,
        agent_id: &AgentId,
        domain: &EffectDomain,
        scope: &ScopeConstraint,
    ) -> Result<CapabilityCheckResult, AasError> {
        self.capability
            .check(agent_id, domain, scope)
            .map_err(AasError::Capability)
    }

    // ============ Commitment Processing ============

    /// Submit a commitment for adjudication
    /// This is the main entry point for the commitment boundary
    pub async fn submit_commitment(
        &self,
        commitment: RcfCommitment,
    ) -> Result<PolicyDecisionCard, AasError> {
        // Step 1: Verify identity
        let verification = self.identity.verify(&commitment.principal)?;
        if !verification.valid {
            return Err(AasError::IdentityVerificationFailed(
                verification.issues.join(", "),
            ));
        }

        // Step 2: Check capabilities
        let agent_id = AgentId::new(&commitment.principal.id);
        let cap_check =
            self.capability
                .check(&agent_id, &commitment.effect_domain, &commitment.scope)?;

        if !cap_check.authorized {
            return Err(AasError::CapabilityDenied(
                cap_check.denial_reason.unwrap_or_default(),
            ));
        }

        // Step 3: Evaluate against policies
        let context = EvaluationContext {
            agent_id: agent_id.clone(),
            capabilities: cap_check.capability_id.into_iter().collect(),
            metadata: Default::default(),
        };

        let evaluation = self.policy.evaluate(&commitment, &context)?;

        // Step 4: Submit to adjudicator
        let policy_input = PolicyEvaluationInput {
            rationale: evaluation.rationale.clone(),
            risk_assessment: evaluation.risk_assessment.clone(),
            rule_results: evaluation
                .rule_results
                .iter()
                .map(|r| RuleResultInput {
                    rule_id: r.rule_id.clone(),
                    triggered: r.triggered,
                    action: r.action.as_ref().map(|a| match a {
                        aas_policy::RuleAction::Allow => RuleActionInput::Allow,
                        aas_policy::RuleAction::Deny => RuleActionInput::Deny,
                        aas_policy::RuleAction::RequireHumanApproval => {
                            RuleActionInput::RequireHumanApproval
                        }
                        aas_policy::RuleAction::RequireAdditionalInfo => {
                            RuleActionInput::RequireAdditionalInfo
                        }
                        aas_policy::RuleAction::AddCondition(_) => RuleActionInput::Allow,
                    }),
                })
                .collect(),
        };

        let commitment_id = commitment.commitment_id.clone();
        self.adjudicator.submit(commitment.clone(), policy_input)?;

        // Step 5: Get decision
        let decision = self.adjudicator.adjudicate(&commitment_id)?;

        // Step 6: Record in ledger
        self.ledger
            .record_commitment(commitment, decision.clone())
            .await?;

        Ok(decision)
    }

    /// Record that execution has started
    pub async fn record_execution_started(
        &self,
        commitment_id: &CommitmentId,
    ) -> Result<(), AasError> {
        self.ledger
            .record_execution_started(commitment_id)
            .await
            .map_err(AasError::Ledger)
    }

    /// Record outcome (consequence)
    pub async fn record_outcome(
        &self,
        commitment_id: &CommitmentId,
        outcome: CommitmentOutcome,
    ) -> Result<(), AasError> {
        self.ledger
            .record_outcome(commitment_id, outcome)
            .await
            .map_err(AasError::Ledger)
    }

    // ============ Human Review Operations ============

    /// Get items pending human review
    pub fn get_pending_reviews(&self) -> Result<Vec<CommitmentId>, AasError> {
        self.adjudicator
            .get_human_review_queue()
            .map_err(AasError::Adjudication)
    }

    /// Record a human review decision
    pub fn record_human_decision(
        &self,
        commitment_id: &CommitmentId,
        approved: bool,
        reviewer_id: &str,
        notes: Option<String>,
    ) -> Result<PolicyDecisionCard, AasError> {
        self.adjudicator
            .record_human_decision(commitment_id, approved, reviewer_id, notes)
            .map_err(AasError::Adjudication)
    }

    // ============ Query Operations ============

    /// Get ledger entry by commitment
    pub async fn get_commitment(
        &self,
        commitment_id: &CommitmentId,
    ) -> Result<Option<LedgerEntry>, AasError> {
        self.ledger
            .get_by_commitment(commitment_id)
            .await
            .map_err(AasError::Ledger)
    }

    /// Get all entries for an agent
    pub async fn get_agent_history(
        &self,
        agent_id: &AgentId,
    ) -> Result<Vec<LedgerEntry>, AasError> {
        self.ledger
            .get_by_agent(agent_id)
            .await
            .map_err(AasError::Ledger)
    }

    /// Query the ledger
    pub async fn query_ledger(&self, query: LedgerQuery) -> Result<Vec<LedgerEntry>, AasError> {
        self.ledger.query(query).await.map_err(AasError::Ledger)
    }

    /// Get ledger statistics
    pub async fn statistics(&self) -> Result<LedgerStatistics, AasError> {
        self.ledger.statistics().await.map_err(AasError::Ledger)
    }

    // ============ Component Access ============

    /// Get identity registry
    pub fn identity(&self) -> &IdentityRegistry {
        &self.identity
    }

    /// Get capability registry
    pub fn capability(&self) -> &CapabilityRegistry {
        &self.capability
    }

    /// Get policy engine
    pub fn policy(&self) -> &PolicyEngine {
        &self.policy
    }

    /// Get adjudicator
    pub fn adjudicator(&self) -> &Adjudicator {
        &self.adjudicator
    }

    /// Get ledger
    pub fn ledger(&self) -> &AccountabilityLedger {
        &self.ledger
    }
}

impl Default for AasService {
    fn default() -> Self {
        Self::new()
    }
}

/// AAS service errors
#[derive(Debug, Error)]
pub enum AasError {
    #[error("Identity error: {0}")]
    Identity(#[from] IdentityError),

    #[error("Capability error: {0}")]
    Capability(#[from] CapabilityError),

    #[error("Policy error: {0}")]
    Policy(#[from] PolicyError),

    #[error("Adjudication error: {0}")]
    Adjudication(#[from] AdjudicationError),

    #[error("Ledger error: {0}")]
    Ledger(#[from] LedgerError),

    #[error("Identity verification failed: {0}")]
    IdentityVerificationFailed(String),

    #[error("Capability denied: {0}")]
    CapabilityDenied(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use aas_identity::AgentType;
    use rcf_commitment::CommitmentBuilder;
    use rcf_types::TemporalValidity;

    #[tokio::test]
    async fn test_full_commitment_flow() {
        let aas = AasService::new();

        // Step 1: Register an agent
        let agent = aas
            .register_agent(RegistrationRequest {
                agent_type: AgentType::Resonator,
                metadata: Default::default(),
            })
            .unwrap();

        // Step 2: Grant a capability
        aas.grant_capability(GrantRequest {
            grantee: agent.agent_id.clone(),
            domain: EffectDomain::Computation,
            scope: ScopeConstraint::global(),
            validity: TemporalValidity::unbounded(),
            issuer: AgentId::new("system"),
            conditions: vec![],
        })
        .unwrap();

        // Step 3: Create and submit a commitment
        let commitment =
            CommitmentBuilder::new(agent.identity_ref.clone(), EffectDomain::Computation)
                .with_scope(ScopeConstraint::default())
                .build()
                .unwrap();

        let decision = aas.submit_commitment(commitment.clone()).await.unwrap();

        // Step 4: Verify decision
        assert!(decision.decision.allows_execution());

        // Step 5: Record execution
        aas.record_execution_started(&commitment.commitment_id)
            .await
            .unwrap();

        aas.record_outcome(
            &commitment.commitment_id,
            CommitmentOutcome {
                success: true,
                description: "Test completed".to_string(),
                completed_at: chrono::Utc::now(),
            },
        )
        .await
        .unwrap();

        // Step 6: Verify ledger entry
        let entry = aas.get_commitment(&commitment.commitment_id).await.unwrap();
        assert!(entry.is_some());
        assert!(entry.unwrap().outcome.unwrap().success);
    }
}
