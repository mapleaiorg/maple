//! Policy decision types
//!
//! Decisions represent the outcome of policy evaluation, including
//! audit trails and approval requirements.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Policy evaluation decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyDecision {
    /// Operation is allowed
    Allow,

    /// Operation is denied
    Deny {
        /// Reason for denial
        reason: String,
        /// Policy that denied the operation
        policy_id: String,
    },

    /// Operation requires human approval
    RequiresApproval {
        /// Required approvers
        approvers: Vec<String>,
        /// Reason approval is required
        reason: String,
        /// Policy that requires approval
        policy_id: String,
    },

    /// Operation is held for review
    Hold {
        /// Reason for hold
        reason: String,
        /// Policy that placed hold
        policy_id: String,
        /// Automatic expiration
        expires_at: Option<DateTime<Utc>>,
    },
}

impl PolicyDecision {
    /// Create an allow decision
    pub fn allow() -> Self {
        Self::Allow
    }

    /// Create a deny decision
    pub fn deny(reason: impl Into<String>, policy_id: impl Into<String>) -> Self {
        Self::Deny {
            reason: reason.into(),
            policy_id: policy_id.into(),
        }
    }

    /// Create a requires approval decision
    pub fn requires_approval(
        approvers: Vec<String>,
        reason: impl Into<String>,
        policy_id: impl Into<String>,
    ) -> Self {
        Self::RequiresApproval {
            approvers,
            reason: reason.into(),
            policy_id: policy_id.into(),
        }
    }

    /// Create a hold decision
    pub fn hold(reason: impl Into<String>, policy_id: impl Into<String>) -> Self {
        Self::Hold {
            reason: reason.into(),
            policy_id: policy_id.into(),
            expires_at: None,
        }
    }

    /// Create a hold decision with expiration
    pub fn hold_until(
        reason: impl Into<String>,
        policy_id: impl Into<String>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self::Hold {
            reason: reason.into(),
            policy_id: policy_id.into(),
            expires_at: Some(expires_at),
        }
    }

    /// Check if the decision allows the operation
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow)
    }

    /// Check if the decision denies the operation
    pub fn is_denied(&self) -> bool {
        matches!(self, Self::Deny { .. })
    }

    /// Check if the decision requires approval
    pub fn requires_human_approval(&self) -> bool {
        matches!(self, Self::RequiresApproval { .. })
    }

    /// Check if the decision is on hold
    pub fn is_held(&self) -> bool {
        matches!(self, Self::Hold { .. })
    }

    /// Get the policy ID that made this decision (if any)
    pub fn policy_id(&self) -> Option<&str> {
        match self {
            Self::Allow => None,
            Self::Deny { policy_id, .. } => Some(policy_id),
            Self::RequiresApproval { policy_id, .. } => Some(policy_id),
            Self::Hold { policy_id, .. } => Some(policy_id),
        }
    }

    /// Get the reason for this decision (if any)
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Allow => None,
            Self::Deny { reason, .. } => Some(reason),
            Self::RequiresApproval { reason, .. } => Some(reason),
            Self::Hold { reason, .. } => Some(reason),
        }
    }
}

/// Audit card for policy decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecisionCard {
    /// Unique identifier for this decision
    pub id: String,

    /// The operation that was evaluated
    pub operation: String,

    /// The decision that was made
    pub decision: PolicyDecision,

    /// Context at time of evaluation
    pub actor_id: String,

    /// Platform where decision was made
    pub platform: String,

    /// Environment where decision was made
    pub environment: String,

    /// When the decision was made
    pub timestamp: DateTime<Utc>,

    /// All policies that were evaluated
    pub policies_evaluated: Vec<PolicyEvaluationRecord>,

    /// Request ID for correlation
    pub request_id: String,
}

impl PolicyDecisionCard {
    /// Create a new decision card
    pub fn new(
        operation: impl Into<String>,
        decision: PolicyDecision,
        actor_id: impl Into<String>,
        platform: impl Into<String>,
        environment: impl Into<String>,
        request_id: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            operation: operation.into(),
            decision,
            actor_id: actor_id.into(),
            platform: platform.into(),
            environment: environment.into(),
            timestamp: Utc::now(),
            policies_evaluated: Vec::new(),
            request_id: request_id.into(),
        }
    }

    /// Add a policy evaluation record
    pub fn add_evaluation(&mut self, record: PolicyEvaluationRecord) {
        self.policies_evaluated.push(record);
    }

    /// Get the final decision
    pub fn final_decision(&self) -> &PolicyDecision {
        &self.decision
    }

    /// Check if operation was allowed
    pub fn was_allowed(&self) -> bool {
        self.decision.is_allowed()
    }
}

/// Record of a single policy evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluationRecord {
    /// Policy identifier
    pub policy_id: String,

    /// Policy name
    pub policy_name: String,

    /// Decision from this policy
    pub decision: PolicyDecision,

    /// Evaluation duration in microseconds
    pub duration_us: u64,

    /// Additional notes from evaluation
    pub notes: Option<String>,
}

impl PolicyEvaluationRecord {
    /// Create a new evaluation record
    pub fn new(
        policy_id: impl Into<String>,
        policy_name: impl Into<String>,
        decision: PolicyDecision,
        duration_us: u64,
    ) -> Self {
        Self {
            policy_id: policy_id.into(),
            policy_name: policy_name.into(),
            decision,
            duration_us,
            notes: None,
        }
    }

    /// Add notes to the record
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_allow() {
        let decision = PolicyDecision::allow();
        assert!(decision.is_allowed());
        assert!(!decision.is_denied());
        assert!(decision.policy_id().is_none());
    }

    #[test]
    fn test_decision_deny() {
        let decision = PolicyDecision::deny("quota exceeded", "quota-policy");
        assert!(!decision.is_allowed());
        assert!(decision.is_denied());
        assert_eq!(decision.policy_id(), Some("quota-policy"));
        assert_eq!(decision.reason(), Some("quota exceeded"));
    }

    #[test]
    fn test_decision_requires_approval() {
        let decision = PolicyDecision::requires_approval(
            vec!["admin@example.com".into()],
            "production deployment",
            "prod-policy",
        );
        assert!(decision.requires_human_approval());
        assert!(!decision.is_allowed());
    }

    #[test]
    fn test_decision_hold() {
        let decision = PolicyDecision::hold("pending review", "review-policy");
        assert!(decision.is_held());
        assert!(!decision.is_allowed());
    }

    #[test]
    fn test_decision_card() {
        let decision = PolicyDecision::allow();
        let card = PolicyDecisionCard::new(
            "CreateDeployment",
            decision,
            "user-1",
            "Mapleverse",
            "production",
            "req-123",
        );

        assert!(card.was_allowed());
        assert!(!card.id.is_empty());
    }

    #[test]
    fn test_evaluation_record() {
        let record = PolicyEvaluationRecord::new(
            "base-invariant",
            "Base Invariant Policy",
            PolicyDecision::allow(),
            150,
        )
        .with_notes("All checks passed");

        assert_eq!(record.policy_id, "base-invariant");
        assert_eq!(record.duration_us, 150);
        assert!(record.notes.is_some());
    }
}
