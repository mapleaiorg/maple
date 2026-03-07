//! Audit logging for firewall decisions.
//!
//! Every decision (allow, deny, pending) is recorded with full context
//! for compliance and forensic analysis.

use crate::firewall::{FirewallDecision, ToolCallRequest};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// An audit log entry recording a firewall decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique entry identifier.
    pub id: String,
    /// Timestamp of the decision.
    pub timestamp: DateTime<Utc>,
    /// Caller identity.
    pub caller: String,
    /// Tool being accessed.
    pub tool: String,
    /// Operation attempted.
    pub operation: String,
    /// Resource targeted.
    pub resource: Option<String>,
    /// Decision outcome.
    pub outcome: AuditOutcome,
    /// Grant ID used (if allowed).
    pub grant_id: Option<String>,
    /// Denial reason (if denied).
    pub denial_reason: Option<String>,
    /// Remediation hint (if denied).
    pub remediation: Option<String>,
}

/// Outcome classification for audit entries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    /// Call was allowed.
    Allow,
    /// Call was denied.
    Deny,
    /// Call is pending approval.
    PendingApproval,
}

/// Audit logger that captures all firewall decisions.
///
/// Stores entries in memory for testing and inspection.
/// In production, this would write to a persistent store.
#[derive(Debug, Clone)]
pub struct AuditLog {
    entries: Arc<Mutex<Vec<AuditEntry>>>,
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLog {
    /// Create a new, empty audit log.
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Record a firewall decision.
    pub fn record(&self, request: &ToolCallRequest, decision: &FirewallDecision) {
        let (outcome, grant_id, denial_reason, remediation) = match decision {
            FirewallDecision::Allow {
                grant_id,
                modifications: _,
            } => (
                AuditOutcome::Allow,
                Some(grant_id.clone()),
                None,
                None,
            ),
            FirewallDecision::Deny {
                reason,
                remediation,
            } => (
                AuditOutcome::Deny,
                None,
                Some(reason.clone()),
                remediation.clone(),
            ),
            FirewallDecision::PendingApproval {
                approval_id,
                approvers: _,
                message,
            } => (
                AuditOutcome::PendingApproval,
                Some(approval_id.clone()),
                Some(message.clone()),
                None,
            ),
        };

        let entry = AuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            caller: request.caller.clone(),
            tool: request.tool.clone(),
            operation: request.operation.clone(),
            resource: request.resource.clone(),
            outcome,
            grant_id,
            denial_reason,
            remediation,
        };

        tracing::info!(
            caller = %entry.caller,
            tool = %entry.tool,
            operation = %entry.operation,
            outcome = ?entry.outcome,
            "firewall decision recorded"
        );

        self.entries.lock().expect("audit lock poisoned").push(entry);
    }

    /// Retrieve all audit entries.
    pub fn entries(&self) -> Vec<AuditEntry> {
        self.entries.lock().expect("audit lock poisoned").clone()
    }

    /// Retrieve entries filtered by caller.
    pub fn entries_for_caller(&self, caller: &str) -> Vec<AuditEntry> {
        self.entries
            .lock()
            .expect("audit lock poisoned")
            .iter()
            .filter(|e| e.caller == caller)
            .cloned()
            .collect()
    }

    /// Count entries by outcome.
    pub fn count_by_outcome(&self, outcome: &AuditOutcome) -> usize {
        self.entries
            .lock()
            .expect("audit lock poisoned")
            .iter()
            .filter(|e| &e.outcome == outcome)
            .count()
    }

    /// Clear all entries (for testing).
    pub fn clear(&self) {
        self.entries.lock().expect("audit lock poisoned").clear();
    }
}
