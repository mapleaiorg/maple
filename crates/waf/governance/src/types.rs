//! Core governance types: approval requests, decisions, and status tracking.

use maple_waf_context_graph::GovernanceTier;
use serde::{Deserialize, Serialize};

/// A request for governance approval of a change or operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// Unique identifier for this approval request.
    pub id: String,
    /// The governance tier required for this change.
    pub governance_tier: GovernanceTier,
    /// Human-readable description of what is being approved.
    pub description: String,
    /// Identity of the entity requesting approval.
    pub requested_by: String,
    /// Timestamp in milliseconds when the request was created.
    pub timestamp_ms: u64,
}

/// The outcome of an approval decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalDecision {
    /// The request was approved.
    Approved,
    /// The request was denied with a reason.
    Denied(String),
    /// The request was escalated to a higher governance tier with a reason.
    Escalated(String),
}

/// Tracks the current status of an approval request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalStatus {
    /// The underlying approval request.
    pub request: ApprovalRequest,
    /// The decision, if one has been made.
    pub decision: Option<ApprovalDecision>,
    /// Timestamp in milliseconds when the decision was made.
    pub decided_at_ms: Option<u64>,
}

impl ApprovalStatus {
    /// Returns `true` if a decision has been made on this request.
    pub fn is_decided(&self) -> bool {
        self.decision.is_some()
    }

    /// Returns `true` if the request was approved.
    pub fn is_approved(&self) -> bool {
        matches!(self.decision, Some(ApprovalDecision::Approved))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> ApprovalRequest {
        ApprovalRequest {
            id: "req-001".into(),
            governance_tier: GovernanceTier::Tier2,
            description: "Update operator logic".into(),
            requested_by: "alice".into(),
            timestamp_ms: 1700000000000,
        }
    }

    #[test]
    fn approval_request_serialization_roundtrip() {
        let req = sample_request();
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: ApprovalRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, deserialized);
    }

    #[test]
    fn approval_decision_variants() {
        let approved = ApprovalDecision::Approved;
        let denied = ApprovalDecision::Denied("too risky".into());
        let escalated = ApprovalDecision::Escalated("needs Tier4 review".into());

        assert_eq!(approved, ApprovalDecision::Approved);
        assert_ne!(approved, denied);
        assert_ne!(denied, escalated);
    }

    #[test]
    fn status_undecided() {
        let status = ApprovalStatus {
            request: sample_request(),
            decision: None,
            decided_at_ms: None,
        };
        assert!(!status.is_decided());
        assert!(!status.is_approved());
    }

    #[test]
    fn status_approved() {
        let status = ApprovalStatus {
            request: sample_request(),
            decision: Some(ApprovalDecision::Approved),
            decided_at_ms: Some(1700000060000),
        };
        assert!(status.is_decided());
        assert!(status.is_approved());
    }

    #[test]
    fn status_denied_is_not_approved() {
        let status = ApprovalStatus {
            request: sample_request(),
            decision: Some(ApprovalDecision::Denied("policy violation".into())),
            decided_at_ms: Some(1700000060000),
        };
        assert!(status.is_decided());
        assert!(!status.is_approved());
    }
}
