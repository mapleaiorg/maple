//! MAPLE Guard Approvals -- human-in-the-loop approval workflows.
//!
//! Provides approval request management with configurable policies,
//! urgency levels, escalation chains, and timeout handling.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum ApprovalError {
    #[error("approval request not found: {0}")]
    NotFound(String),
    #[error("invalid state transition: cannot {operation} request in state {state:?}")]
    InvalidState { operation: String, state: ApprovalStatus },
    #[error("request expired: {0}")]
    Expired(String),
    #[error("policy error: {0}")]
    PolicyError(String),
}

pub type ApprovalResult<T> = Result<T, ApprovalError>;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Urgency level for an approval request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UrgencyLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl UrgencyLevel {
    /// Default timeout duration for this urgency level.
    pub fn default_timeout(&self) -> Duration {
        match self {
            Self::Low => Duration::hours(24),
            Self::Medium => Duration::hours(4),
            Self::High => Duration::hours(1),
            Self::Critical => Duration::minutes(15),
        }
    }
}

/// Status of an approval request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
    Expired,
    Escalated,
}

/// An approval request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub action: String,
    pub requester: String,
    pub reason: String,
    pub urgency: UrgencyLevel,
    pub status: ApprovalStatus,
    pub created_at: DateTime<Utc>,
    pub decided_at: Option<DateTime<Utc>>,
    pub decided_by: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub escalation_level: u32,
    pub metadata: HashMap<String, String>,
}

impl ApprovalRequest {
    pub fn new(
        action: impl Into<String>,
        requester: impl Into<String>,
        reason: impl Into<String>,
        urgency: UrgencyLevel,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            action: action.into(),
            requester: requester.into(),
            reason: reason.into(),
            urgency,
            status: ApprovalStatus::Pending,
            created_at: now,
            decided_at: None,
            decided_by: None,
            expires_at: now + urgency.default_timeout(),
            escalation_level: 0,
            metadata: HashMap::new(),
        }
    }

    /// Check if the request has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Policy defining when approval is required.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalPolicy {
    pub id: String,
    pub name: String,
    /// Action patterns that require approval (glob-style).
    pub requires_approval: Vec<String>,
    /// Timeout in seconds before auto-escalation.
    pub timeout_secs: u64,
    /// Chain of approvers for escalation.
    pub escalation_chain: Vec<String>,
    /// Whether to auto-deny on final timeout.
    pub auto_deny_on_timeout: bool,
}

impl ApprovalPolicy {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            requires_approval: Vec::new(),
            timeout_secs: 3600,
            escalation_chain: Vec::new(),
            auto_deny_on_timeout: true,
        }
    }

    /// Check if an action matches this policy.
    pub fn matches_action(&self, action: &str) -> bool {
        self.requires_approval.iter().any(|pattern| {
            if pattern == "*" {
                return true;
            }
            if pattern.ends_with('*') {
                let prefix = &pattern[..pattern.len() - 1];
                return action.starts_with(prefix);
            }
            pattern == action
        })
    }
}

// ---------------------------------------------------------------------------
// Approval Workflow
// ---------------------------------------------------------------------------

/// Manages approval workflows.
pub struct ApprovalWorkflow {
    requests: HashMap<String, ApprovalRequest>,
    policies: Vec<ApprovalPolicy>,
}

impl Default for ApprovalWorkflow {
    fn default() -> Self {
        Self::new()
    }
}

impl ApprovalWorkflow {
    pub fn new() -> Self {
        Self {
            requests: HashMap::new(),
            policies: Vec::new(),
        }
    }

    /// Add a policy.
    pub fn add_policy(&mut self, policy: ApprovalPolicy) {
        self.policies.push(policy);
    }

    /// Check if an action requires approval.
    pub fn requires_approval(&self, action: &str) -> bool {
        self.policies.iter().any(|p| p.matches_action(action))
    }

    /// Submit a new approval request.
    pub fn submit(&mut self, request: ApprovalRequest) -> ApprovalResult<String> {
        let id = request.id.clone();
        self.requests.insert(id.clone(), request);
        Ok(id)
    }

    /// Approve a request.
    pub fn approve(&mut self, id: &str, approver: &str) -> ApprovalResult<&ApprovalRequest> {
        let request = self
            .requests
            .get_mut(id)
            .ok_or_else(|| ApprovalError::NotFound(id.to_string()))?;

        if request.status != ApprovalStatus::Pending && request.status != ApprovalStatus::Escalated {
            return Err(ApprovalError::InvalidState {
                operation: "approve".into(),
                state: request.status,
            });
        }

        if request.is_expired() {
            request.status = ApprovalStatus::Expired;
            return Err(ApprovalError::Expired(id.to_string()));
        }

        request.status = ApprovalStatus::Approved;
        request.decided_at = Some(Utc::now());
        request.decided_by = Some(approver.to_string());
        Ok(request)
    }

    /// Deny a request.
    pub fn deny(&mut self, id: &str, approver: &str, reason: Option<&str>) -> ApprovalResult<&ApprovalRequest> {
        let request = self
            .requests
            .get_mut(id)
            .ok_or_else(|| ApprovalError::NotFound(id.to_string()))?;

        if request.status != ApprovalStatus::Pending && request.status != ApprovalStatus::Escalated {
            return Err(ApprovalError::InvalidState {
                operation: "deny".into(),
                state: request.status,
            });
        }

        request.status = ApprovalStatus::Denied;
        request.decided_at = Some(Utc::now());
        request.decided_by = Some(approver.to_string());
        if let Some(r) = reason {
            request.metadata.insert("deny_reason".into(), r.to_string());
        }
        Ok(request)
    }

    /// Escalate a request to the next level.
    pub fn escalate(&mut self, id: &str) -> ApprovalResult<&ApprovalRequest> {
        let request = self
            .requests
            .get_mut(id)
            .ok_or_else(|| ApprovalError::NotFound(id.to_string()))?;

        if request.status != ApprovalStatus::Pending {
            return Err(ApprovalError::InvalidState {
                operation: "escalate".into(),
                state: request.status,
            });
        }

        request.status = ApprovalStatus::Escalated;
        request.escalation_level += 1;
        // Extend timeout on escalation
        request.expires_at = Utc::now() + request.urgency.default_timeout();
        Ok(request)
    }

    /// Get a request by ID.
    pub fn get(&self, id: &str) -> ApprovalResult<&ApprovalRequest> {
        self.requests
            .get(id)
            .ok_or_else(|| ApprovalError::NotFound(id.to_string()))
    }

    /// List all requests, optionally filtered by status.
    pub fn list(&self, status_filter: Option<ApprovalStatus>) -> Vec<&ApprovalRequest> {
        match status_filter {
            Some(status) => self.requests.values().filter(|r| r.status == status).collect(),
            None => self.requests.values().collect(),
        }
    }

    /// Process expired requests.
    pub fn process_expired(&mut self) -> Vec<String> {
        let mut expired_ids = Vec::new();
        for (id, request) in &mut self.requests {
            if request.status == ApprovalStatus::Pending && request.is_expired() {
                request.status = ApprovalStatus::Expired;
                expired_ids.push(id.clone());
            }
        }
        expired_ids
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submit_request() {
        let mut wf = ApprovalWorkflow::new();
        let req = ApprovalRequest::new("deploy.production", "agent-1", "new release", UrgencyLevel::Medium);
        let id = wf.submit(req).unwrap();
        let fetched = wf.get(&id).unwrap();
        assert_eq!(fetched.status, ApprovalStatus::Pending);
    }

    #[test]
    fn test_approve_request() {
        let mut wf = ApprovalWorkflow::new();
        let req = ApprovalRequest::new("deploy", "agent-1", "test", UrgencyLevel::Low);
        let id = wf.submit(req).unwrap();
        let approved = wf.approve(&id, "admin").unwrap();
        assert_eq!(approved.status, ApprovalStatus::Approved);
        assert_eq!(approved.decided_by.as_deref(), Some("admin"));
    }

    #[test]
    fn test_deny_request() {
        let mut wf = ApprovalWorkflow::new();
        let req = ApprovalRequest::new("deploy", "agent-1", "test", UrgencyLevel::Low);
        let id = wf.submit(req).unwrap();
        let denied = wf.deny(&id, "admin", Some("not ready")).unwrap();
        assert_eq!(denied.status, ApprovalStatus::Denied);
    }

    #[test]
    fn test_escalate_request() {
        let mut wf = ApprovalWorkflow::new();
        let req = ApprovalRequest::new("deploy", "agent-1", "test", UrgencyLevel::High);
        let id = wf.submit(req).unwrap();
        let escalated = wf.escalate(&id).unwrap();
        assert_eq!(escalated.status, ApprovalStatus::Escalated);
        assert_eq!(escalated.escalation_level, 1);
    }

    #[test]
    fn test_cannot_approve_denied() {
        let mut wf = ApprovalWorkflow::new();
        let req = ApprovalRequest::new("deploy", "agent-1", "test", UrgencyLevel::Low);
        let id = wf.submit(req).unwrap();
        wf.deny(&id, "admin", None).unwrap();
        let result = wf.approve(&id, "admin2");
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_matching() {
        let mut policy = ApprovalPolicy::new("prod-deploys");
        policy.requires_approval = vec!["deploy.production*".into(), "delete.*".into()];
        assert!(policy.matches_action("deploy.production"));
        assert!(policy.matches_action("deploy.production.v2"));
        assert!(!policy.matches_action("deploy.staging"));
    }

    #[test]
    fn test_requires_approval() {
        let mut wf = ApprovalWorkflow::new();
        let mut policy = ApprovalPolicy::new("prod-policy");
        policy.requires_approval = vec!["deploy.prod*".into()];
        wf.add_policy(policy);
        assert!(wf.requires_approval("deploy.prod"));
        assert!(!wf.requires_approval("deploy.staging"));
    }

    #[test]
    fn test_list_by_status() {
        let mut wf = ApprovalWorkflow::new();
        let r1 = ApprovalRequest::new("a", "x", "r", UrgencyLevel::Low);
        let r2 = ApprovalRequest::new("b", "x", "r", UrgencyLevel::Low);
        let id1 = wf.submit(r1).unwrap();
        wf.submit(r2).unwrap();
        wf.approve(&id1, "admin").unwrap();
        let pending = wf.list(Some(ApprovalStatus::Pending));
        assert_eq!(pending.len(), 1);
        let approved = wf.list(Some(ApprovalStatus::Approved));
        assert_eq!(approved.len(), 1);
    }

    #[test]
    fn test_urgency_timeout() {
        assert!(UrgencyLevel::Critical.default_timeout() < UrgencyLevel::Low.default_timeout());
    }

    #[test]
    fn test_not_found() {
        let wf = ApprovalWorkflow::new();
        assert!(wf.get("nonexistent").is_err());
    }

    #[test]
    fn test_wildcard_policy() {
        let mut policy = ApprovalPolicy::new("all");
        policy.requires_approval = vec!["*".into()];
        assert!(policy.matches_action("anything"));
        assert!(policy.matches_action("deploy.prod"));
    }
}
