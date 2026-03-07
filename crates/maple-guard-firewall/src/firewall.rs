//! Capability firewall — deny-by-default enforcement for tool/action calls.

use std::collections::HashMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::audit::AuditLog;
use crate::engine::glob_match;
use crate::grants::CapabilityGrant;

/// A request to execute a tool or capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub caller: String,
    pub tool: String,
    pub operation: String,
    pub resource: Option<String>,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// The firewall's decision on a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FirewallDecision {
    Allow {
        grant_id: String,
        modifications: Vec<String>,
    },
    Deny {
        reason: String,
        remediation: Option<String>,
    },
    PendingApproval {
        approval_id: String,
        approvers: Vec<String>,
        message: String,
    },
}

/// The capability firewall engine.
pub struct CapabilityFirewall {
    grants: Vec<CapabilityGrant>,
    audit_log: AuditLog,
}

impl CapabilityFirewall {
    pub fn new(audit_log: AuditLog) -> Self {
        Self {
            grants: Vec::new(),
            audit_log,
        }
    }

    pub fn add_grant(&mut self, grant: CapabilityGrant) {
        self.grants.push(grant);
    }

    pub fn evaluate(&self, request: &ToolCallRequest) -> FirewallDecision {
        let matching: Vec<&CapabilityGrant> = self
            .grants
            .iter()
            .filter(|g| g.grantee == request.caller)
            .filter(|g| self.grant_matches(g, request))
            .collect();

        if matching.is_empty() {
            let decision = FirewallDecision::Deny {
                reason: format!(
                    "no matching grant for '{}' to execute {}.{}",
                    request.caller, request.tool, request.operation
                ),
                remediation: Some(format!(
                    "Add a capability grant for tool '{}' operation '{}'",
                    request.tool, request.operation
                )),
            };
            self.audit_log.record(request, &decision);
            return decision;
        }

        let grant = matching[0];
        let now = Utc::now();

        if now < grant.valid_from {
            let decision = FirewallDecision::Deny {
                reason: format!("grant '{}' not yet valid", grant.id),
                remediation: None,
            };
            self.audit_log.record(request, &decision);
            return decision;
        }
        if let Some(until) = grant.valid_until {
            if now > until {
                let decision = FirewallDecision::Deny {
                    reason: format!("grant '{}' has expired", grant.id),
                    remediation: Some("Request a renewed capability grant".to_string()),
                };
                self.audit_log.record(request, &decision);
                return decision;
            }
        }

        if grant.requires_approval {
            let decision = FirewallDecision::PendingApproval {
                approval_id: uuid::Uuid::new_v4().to_string(),
                approvers: vec![grant.issuer.clone()],
                message: format!(
                    "Approval required for {}.{} by '{}'",
                    request.tool, request.operation, request.caller
                ),
            };
            self.audit_log.record(request, &decision);
            return decision;
        }

        let decision = FirewallDecision::Allow {
            grant_id: grant.id.clone(),
            modifications: vec![],
        };
        self.audit_log.record(request, &decision);
        decision
    }

    fn grant_matches(&self, grant: &CapabilityGrant, request: &ToolCallRequest) -> bool {
        if !glob_match(&grant.tool, &request.tool) {
            return false;
        }
        let op_match = grant.scope.operations.is_empty()
            || grant
                .scope
                .operations
                .iter()
                .any(|op| glob_match(op, &request.operation));
        if !op_match {
            return false;
        }
        if !grant.scope.resources.is_empty() {
            if let Some(ref resource) = request.resource {
                let resource_match = grant
                    .scope
                    .resources
                    .iter()
                    .any(|r| glob_match(r, resource));
                if !resource_match {
                    return false;
                }
            }
        }
        true
    }

    pub fn audit_log(&self) -> &AuditLog {
        &self.audit_log
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grants::GrantScope;

    fn make_grant(grantee: &str, tool: &str, operations: Vec<&str>) -> CapabilityGrant {
        CapabilityGrant {
            id: format!("grant-{}-{}", grantee, tool),
            grantee: grantee.to_string(),
            tool: tool.to_string(),
            scope: GrantScope::with_operations(
                operations.into_iter().map(String::from).collect(),
            ),
            requires_approval: false,
            rate_limit: None,
            valid_from: Utc::now() - chrono::Duration::hours(1),
            valid_until: None,
            conditions: vec![],
            issuer: "admin".to_string(),
            purpose: "testing".to_string(),
        }
    }

    #[test]
    fn test_deny_by_default() {
        let fw = CapabilityFirewall::new(AuditLog::new());
        let req = ToolCallRequest {
            caller: "agent-1".to_string(),
            tool: "file".to_string(),
            operation: "read".to_string(),
            resource: None,
            parameters: HashMap::new(),
        };
        assert!(matches!(fw.evaluate(&req), FirewallDecision::Deny { .. }));
    }

    #[test]
    fn test_allow_with_matching_grant() {
        let mut fw = CapabilityFirewall::new(AuditLog::new());
        fw.add_grant(make_grant("agent-1", "file", vec!["read"]));
        let req = ToolCallRequest {
            caller: "agent-1".to_string(),
            tool: "file".to_string(),
            operation: "read".to_string(),
            resource: None,
            parameters: HashMap::new(),
        };
        assert!(matches!(fw.evaluate(&req), FirewallDecision::Allow { .. }));
    }

    #[test]
    fn test_deny_wrong_operation() {
        let mut fw = CapabilityFirewall::new(AuditLog::new());
        fw.add_grant(make_grant("agent-1", "file", vec!["read"]));
        let req = ToolCallRequest {
            caller: "agent-1".to_string(),
            tool: "file".to_string(),
            operation: "write".to_string(),
            resource: None,
            parameters: HashMap::new(),
        };
        assert!(matches!(fw.evaluate(&req), FirewallDecision::Deny { .. }));
    }

    #[test]
    fn test_wildcard_tool() {
        let mut fw = CapabilityFirewall::new(AuditLog::new());
        fw.add_grant(make_grant("agent-1", "zendesk.*", vec!["read"]));
        let req = ToolCallRequest {
            caller: "agent-1".to_string(),
            tool: "zendesk.ticket".to_string(),
            operation: "read".to_string(),
            resource: None,
            parameters: HashMap::new(),
        };
        assert!(matches!(fw.evaluate(&req), FirewallDecision::Allow { .. }));
    }

    #[test]
    fn test_expired_grant() {
        let mut fw = CapabilityFirewall::new(AuditLog::new());
        let mut grant = make_grant("agent-1", "file", vec!["read"]);
        grant.valid_until = Some(Utc::now() - chrono::Duration::hours(1));
        fw.add_grant(grant);
        let req = ToolCallRequest {
            caller: "agent-1".to_string(),
            tool: "file".to_string(),
            operation: "read".to_string(),
            resource: None,
            parameters: HashMap::new(),
        };
        assert!(matches!(fw.evaluate(&req), FirewallDecision::Deny { .. }));
    }

    #[test]
    fn test_requires_approval() {
        let mut fw = CapabilityFirewall::new(AuditLog::new());
        let mut grant = make_grant("agent-1", "payment", vec!["transfer"]);
        grant.requires_approval = true;
        fw.add_grant(grant);
        let req = ToolCallRequest {
            caller: "agent-1".to_string(),
            tool: "payment".to_string(),
            operation: "transfer".to_string(),
            resource: None,
            parameters: HashMap::new(),
        };
        assert!(matches!(
            fw.evaluate(&req),
            FirewallDecision::PendingApproval { .. }
        ));
    }

    #[test]
    fn test_audit_log_records() {
        let log = AuditLog::new();
        let mut fw = CapabilityFirewall::new(log.clone());
        fw.add_grant(make_grant("agent-1", "file", vec!["read"]));
        let req = ToolCallRequest {
            caller: "agent-1".to_string(),
            tool: "file".to_string(),
            operation: "read".to_string(),
            resource: None,
            parameters: HashMap::new(),
        };
        fw.evaluate(&req);
        assert_eq!(log.entries().len(), 1);
    }
}
