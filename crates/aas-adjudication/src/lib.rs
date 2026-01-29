//! AAS Adjudication - The heart of the authority layer
//!
//! This is where commitments are evaluated, approved, or denied.
//! The adjudicator is the gatekeeper between intention and action.

#![deny(unsafe_code)]

use aas_types::{
    AgentId, AdjudicatorInfo, AdjudicatorType, Condition, ConditionType, Decision,
    DecisionId, PolicyDecisionCard, RiskAssessment, RiskLevel, Rationale,
};
use rcl_commitment::{CommitmentId, RclCommitment};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;

/// The Adjudicator - decides if commitments can be executed
pub struct Adjudicator {
    pending_decisions: RwLock<HashMap<CommitmentId, PendingDecision>>,
    decision_history: RwLock<Vec<PolicyDecisionCard>>,
    human_review_queue: RwLock<Vec<CommitmentId>>,
}

impl Adjudicator {
    /// Create a new adjudicator
    pub fn new() -> Self {
        Self {
            pending_decisions: RwLock::new(HashMap::new()),
            decision_history: RwLock::new(Vec::new()),
            human_review_queue: RwLock::new(Vec::new()),
        }
    }

    /// Submit a commitment for adjudication
    pub fn submit(&self, commitment: RclCommitment, evaluation: PolicyEvaluationInput) -> Result<SubmissionReceipt, AdjudicationError> {
        let commitment_id = commitment.commitment_id.clone();

        let pending = PendingDecision {
            commitment,
            evaluation,
            submitted_at: chrono::Utc::now(),
            status: PendingStatus::Evaluating,
        };

        let mut pending_decisions = self.pending_decisions.write().map_err(|_| AdjudicationError::LockError)?;
        pending_decisions.insert(commitment_id.clone(), pending);

        Ok(SubmissionReceipt {
            commitment_id,
            submitted_at: chrono::Utc::now(),
        })
    }

    /// Process a commitment and produce a decision
    pub fn adjudicate(&self, commitment_id: &CommitmentId) -> Result<PolicyDecisionCard, AdjudicationError> {
        let mut pending_decisions = self.pending_decisions.write().map_err(|_| AdjudicationError::LockError)?;

        let pending = pending_decisions
            .remove(commitment_id)
            .ok_or_else(|| AdjudicationError::NotFound(commitment_id.0.clone()))?;

        // Determine decision based on policy evaluation
        let (decision, conditions) = self.determine_decision(&pending)?;

        // If human review is needed, queue it
        if decision == Decision::PendingHumanReview {
            let mut queue = self.human_review_queue.write().map_err(|_| AdjudicationError::LockError)?;
            queue.push(commitment_id.clone());
        }

        let card = PolicyDecisionCard {
            decision_id: DecisionId::generate(),
            commitment_id: commitment_id.clone(),
            decision,
            rationale: pending.evaluation.rationale,
            risk_assessment: pending.evaluation.risk_assessment,
            conditions,
            approval_expiration: self.calculate_expiration(&pending.commitment),
            decided_at: chrono::Utc::now(),
            adjudicator: AdjudicatorInfo {
                adjudicator_type: AdjudicatorType::Automated,
                adjudicator_id: "aas-adjudicator-v1".to_string(),
            },
        };

        // Record decision
        let mut history = self.decision_history.write().map_err(|_| AdjudicationError::LockError)?;
        history.push(card.clone());

        Ok(card)
    }

    /// Process human review decision
    pub fn record_human_decision(
        &self,
        commitment_id: &CommitmentId,
        approved: bool,
        reviewer_id: &str,
        notes: Option<String>,
    ) -> Result<PolicyDecisionCard, AdjudicationError> {
        // Remove from human review queue
        {
            let mut queue = self.human_review_queue.write().map_err(|_| AdjudicationError::LockError)?;
            queue.retain(|id| id != commitment_id);
        }

        let pending_decisions = self.pending_decisions.read().map_err(|_| AdjudicationError::LockError)?;

        // Get original evaluation if still pending, otherwise use defaults
        let (rationale, risk_assessment) = if let Some(pending) = pending_decisions.get(commitment_id) {
            (pending.evaluation.rationale.clone(), pending.evaluation.risk_assessment.clone())
        } else {
            (
                Rationale {
                    summary: notes.unwrap_or_else(|| "Human review decision".to_string()),
                    rule_references: vec![],
                },
                RiskAssessment {
                    overall_risk: RiskLevel::Medium,
                    risk_factors: vec![],
                    mitigations: vec![],
                },
            )
        };

        drop(pending_decisions);

        let card = PolicyDecisionCard {
            decision_id: DecisionId::generate(),
            commitment_id: commitment_id.clone(),
            decision: if approved {
                Decision::Approved
            } else {
                Decision::Denied
            },
            rationale,
            risk_assessment,
            conditions: vec![],
            approval_expiration: Some(chrono::Utc::now() + chrono::Duration::hours(24)),
            decided_at: chrono::Utc::now(),
            adjudicator: AdjudicatorInfo {
                adjudicator_type: AdjudicatorType::Human,
                adjudicator_id: reviewer_id.to_string(),
            },
        };

        // Record decision
        let mut history = self.decision_history.write().map_err(|_| AdjudicationError::LockError)?;
        history.push(card.clone());

        // Remove from pending
        let mut pending = self.pending_decisions.write().map_err(|_| AdjudicationError::LockError)?;
        pending.remove(commitment_id);

        Ok(card)
    }

    /// Get pending human review items
    pub fn get_human_review_queue(&self) -> Result<Vec<CommitmentId>, AdjudicationError> {
        let queue = self.human_review_queue.read().map_err(|_| AdjudicationError::LockError)?;
        Ok(queue.clone())
    }

    /// Get decision history
    pub fn get_decision_history(&self, limit: usize) -> Result<Vec<PolicyDecisionCard>, AdjudicationError> {
        let history = self.decision_history.read().map_err(|_| AdjudicationError::LockError)?;
        Ok(history.iter().rev().take(limit).cloned().collect())
    }

    /// Determine the decision based on policy evaluation
    fn determine_decision(&self, pending: &PendingDecision) -> Result<(Decision, Vec<Condition>), AdjudicationError> {
        let mut conditions = vec![];

        // Check if any rule triggered a denial
        if pending.evaluation.rule_results.iter().any(|r| {
            matches!(r.action, Some(RuleActionInput::Deny))
        }) {
            return Ok((Decision::Denied, conditions));
        }

        // Check if human approval is required
        if pending.evaluation.rule_results.iter().any(|r| {
            matches!(r.action, Some(RuleActionInput::RequireHumanApproval))
        }) {
            conditions.push(Condition {
                condition_type: ConditionType::HumanApproval,
                description: "Human approval required due to policy".to_string(),
            });
            return Ok((Decision::PendingHumanReview, conditions));
        }

        // Check if additional info is needed
        if pending.evaluation.rule_results.iter().any(|r| {
            matches!(r.action, Some(RuleActionInput::RequireAdditionalInfo))
        }) {
            conditions.push(Condition {
                condition_type: ConditionType::AdditionalVerification,
                description: "Additional information required".to_string(),
            });
            return Ok((Decision::PendingAdditionalInfo, conditions));
        }

        // Check risk level
        match pending.evaluation.risk_assessment.overall_risk {
            RiskLevel::Critical => {
                conditions.push(Condition {
                    condition_type: ConditionType::HumanApproval,
                    description: "Critical risk level requires human approval".to_string(),
                });
                Ok((Decision::PendingHumanReview, conditions))
            }
            RiskLevel::High => {
                conditions.push(Condition {
                    condition_type: ConditionType::Monitoring,
                    description: "High risk level - execution will be monitored".to_string(),
                });
                Ok((Decision::Approved, conditions))
            }
            _ => Ok((Decision::Approved, conditions)),
        }
    }

    /// Calculate approval expiration based on commitment
    fn calculate_expiration(&self, commitment: &RclCommitment) -> Option<chrono::DateTime<chrono::Utc>> {
        // Default expiration based on temporal validity
        if let Some(expires) = commitment.temporal_validity.valid_until {
            Some(expires)
        } else {
            // Default 24 hour expiration for approvals
            Some(chrono::Utc::now() + chrono::Duration::hours(24))
        }
    }
}

impl Default for Adjudicator {
    fn default() -> Self {
        Self::new()
    }
}

/// A pending decision awaiting adjudication
#[derive(Clone, Debug)]
struct PendingDecision {
    commitment: RclCommitment,
    evaluation: PolicyEvaluationInput,
    submitted_at: chrono::DateTime<chrono::Utc>,
    status: PendingStatus,
}

/// Status of a pending decision
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingStatus {
    Evaluating,
}

/// Input from policy evaluation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyEvaluationInput {
    pub rationale: Rationale,
    pub risk_assessment: RiskAssessment,
    pub rule_results: Vec<RuleResultInput>,
}

/// Rule result from policy evaluation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleResultInput {
    pub rule_id: String,
    pub triggered: bool,
    pub action: Option<RuleActionInput>,
}

/// Rule action input
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RuleActionInput {
    Allow,
    Deny,
    RequireHumanApproval,
    RequireAdditionalInfo,
}

/// Receipt for a submission
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmissionReceipt {
    pub commitment_id: CommitmentId,
    pub submitted_at: chrono::DateTime<chrono::Utc>,
}

/// Adjudication-related errors
#[derive(Debug, Error)]
pub enum AdjudicationError {
    #[error("Commitment not found: {0}")]
    NotFound(String),

    #[error("Already adjudicated: {0}")]
    AlreadyAdjudicated(String),

    #[error("Evaluation failed: {0}")]
    EvaluationFailed(String),

    #[error("Lock error")]
    LockError,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcl_commitment::CommitmentBuilder;
    use rcl_types::{EffectDomain, IdentityRef, ScopeConstraint};

    #[test]
    fn test_adjudication_flow() {
        let adjudicator = Adjudicator::new();

        let commitment = CommitmentBuilder::new(
            IdentityRef::new("test-agent"),
            EffectDomain::Computation,
        )
        .with_scope(ScopeConstraint::default())
        .build()
        .unwrap();

        let commitment_id = commitment.commitment_id.clone();

        let evaluation = PolicyEvaluationInput {
            rationale: Rationale {
                summary: "Test evaluation".to_string(),
                rule_references: vec![],
            },
            risk_assessment: RiskAssessment {
                overall_risk: RiskLevel::Low,
                risk_factors: vec![],
                mitigations: vec![],
            },
            rule_results: vec![],
        };

        // Submit
        let receipt = adjudicator.submit(commitment, evaluation).unwrap();
        assert_eq!(receipt.commitment_id, commitment_id);

        // Adjudicate
        let decision = adjudicator.adjudicate(&commitment_id).unwrap();
        assert_eq!(decision.decision, Decision::Approved);
    }
}
