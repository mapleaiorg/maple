//! Policy evaluation context
//!
//! Context carries all information needed for policy evaluation,
//! including actor identity, platform profile, and approval state.

use chrono::{DateTime, Utc};
use palm_types::PlatformProfile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context for policy evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluationContext {
    /// Identity of the requester
    pub actor_id: String,

    /// Type of actor (human, system, service)
    pub actor_type: ActorType,

    /// Platform profile
    pub platform: PlatformProfile,

    /// Environment (prod, staging, dev)
    pub environment: String,

    /// Whether human approval was obtained
    pub human_approval: Option<HumanApproval>,

    /// Request timestamp
    pub timestamp: DateTime<Utc>,

    /// Request ID for correlation
    pub request_id: String,

    /// Additional context data
    pub metadata: HashMap<String, String>,

    /// Resource quotas consumed by this actor
    pub quota_usage: Option<QuotaUsage>,
}

impl PolicyEvaluationContext {
    /// Create a new policy evaluation context
    pub fn new(actor_id: impl Into<String>, platform: PlatformProfile) -> Self {
        Self {
            actor_id: actor_id.into(),
            actor_type: ActorType::Human,
            platform,
            environment: "development".into(),
            human_approval: None,
            timestamp: Utc::now(),
            request_id: uuid::Uuid::new_v4().to_string(),
            metadata: HashMap::new(),
            quota_usage: None,
        }
    }

    /// Set the actor type
    pub fn with_actor_type(mut self, actor_type: ActorType) -> Self {
        self.actor_type = actor_type;
        self
    }

    /// Set the environment
    pub fn with_environment(mut self, environment: impl Into<String>) -> Self {
        self.environment = environment.into();
        self
    }

    /// Add human approval
    pub fn with_human_approval(mut self, approval: HumanApproval) -> Self {
        self.human_approval = Some(approval);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set quota usage
    pub fn with_quota_usage(mut self, usage: QuotaUsage) -> Self {
        self.quota_usage = Some(usage);
        self
    }

    /// Check if human approval was obtained
    pub fn has_human_approval(&self) -> bool {
        self.human_approval.is_some()
    }

    /// Check if this is a production environment
    pub fn is_production(&self) -> bool {
        self.environment == "production" || self.environment == "prod"
    }

    /// Check if actor is a system service
    pub fn is_system_actor(&self) -> bool {
        matches!(self.actor_type, ActorType::System | ActorType::Service(_))
    }
}

impl Default for PolicyEvaluationContext {
    fn default() -> Self {
        Self::new("anonymous", PlatformProfile::Development)
    }
}

/// Type of actor making the request
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorType {
    /// Human operator
    Human,

    /// System/automated process
    System,

    /// Named service
    Service(String),
}

/// Human approval record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanApproval {
    /// Approver's identity
    pub approver_id: String,

    /// When approval was granted
    pub approved_at: DateTime<Utc>,

    /// Approval reason/notes
    pub reason: Option<String>,

    /// Approval expiration (if any)
    pub expires_at: Option<DateTime<Utc>>,

    /// Scope of approval (specific operation, time-limited, etc.)
    pub scope: ApprovalScope,
}

impl HumanApproval {
    /// Create a new human approval
    pub fn new(approver_id: impl Into<String>) -> Self {
        Self {
            approver_id: approver_id.into(),
            approved_at: Utc::now(),
            reason: None,
            expires_at: None,
            scope: ApprovalScope::SingleOperation,
        }
    }

    /// Add a reason for approval
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Set expiration time
    pub fn with_expiration(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Set approval scope
    pub fn with_scope(mut self, scope: ApprovalScope) -> Self {
        self.scope = scope;
        self
    }

    /// Check if approval is still valid
    pub fn is_valid(&self) -> bool {
        match self.expires_at {
            Some(expires) => Utc::now() < expires,
            None => true,
        }
    }
}

/// Scope of approval
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalScope {
    /// Approval for a single operation
    SingleOperation,

    /// Time-limited approval for multiple operations
    TimeLimited {
        /// Duration in seconds
        duration_secs: u64,
    },

    /// Approval for specific operation types
    OperationTypes {
        /// Allowed operation types
        types: Vec<String>,
    },

    /// Blanket approval (admin override)
    Blanket,
}

/// Resource quota usage
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuotaUsage {
    /// Number of deployments
    pub deployments: u32,

    /// Maximum deployments allowed
    pub max_deployments: Option<u32>,

    /// Number of instances
    pub instances: u32,

    /// Maximum instances allowed
    pub max_instances: Option<u32>,

    /// Operations per hour
    pub operations_per_hour: u32,

    /// Maximum operations per hour
    pub max_operations_per_hour: Option<u32>,
}

impl QuotaUsage {
    /// Check if deployment quota is exceeded
    pub fn deployments_exceeded(&self) -> bool {
        match self.max_deployments {
            Some(max) => self.deployments >= max,
            None => false,
        }
    }

    /// Check if instance quota is exceeded
    pub fn instances_exceeded(&self) -> bool {
        match self.max_instances {
            Some(max) => self.instances >= max,
            None => false,
        }
    }

    /// Check if rate limit is exceeded
    pub fn rate_limit_exceeded(&self) -> bool {
        match self.max_operations_per_hour {
            Some(max) => self.operations_per_hour >= max,
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Mapleverse);
        assert_eq!(ctx.actor_id, "user-1");
        assert_eq!(ctx.platform, PlatformProfile::Mapleverse);
        assert!(!ctx.has_human_approval());
    }

    #[test]
    fn test_context_with_approval() {
        let approval = HumanApproval::new("admin-1")
            .with_reason("Emergency deployment");

        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Finalverse)
            .with_human_approval(approval);

        assert!(ctx.has_human_approval());
    }

    #[test]
    fn test_approval_validity() {
        let approval = HumanApproval::new("admin-1");
        assert!(approval.is_valid());

        let expired = HumanApproval::new("admin-1")
            .with_expiration(Utc::now() - chrono::Duration::hours(1));
        assert!(!expired.is_valid());
    }

    #[test]
    fn test_quota_checks() {
        let mut usage = QuotaUsage::default();
        usage.deployments = 5;
        usage.max_deployments = Some(10);
        assert!(!usage.deployments_exceeded());

        usage.deployments = 10;
        assert!(usage.deployments_exceeded());
    }

    #[test]
    fn test_production_check() {
        let prod = PolicyEvaluationContext::new("user-1", PlatformProfile::IBank)
            .with_environment("production");
        assert!(prod.is_production());

        let dev = PolicyEvaluationContext::new("user-1", PlatformProfile::Development);
        assert!(!dev.is_production());
    }
}
