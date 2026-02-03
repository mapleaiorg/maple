//! Policy types for deployment governance
//!
//! Policies define constraints and gates for deployment operations.

use crate::PlatformProfile;
use serde::{Deserialize, Serialize};

/// PALM operation that requires policy validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PalmOperation {
    // ========== Registry Operations ==========
    /// Create a new spec
    CreateSpec { spec_id: String },
    /// Update an existing spec
    UpdateSpec { spec_id: String },
    /// Deprecate a spec
    DeprecateSpec { spec_id: String },

    // ========== Deployment Operations ==========
    /// Create a new deployment
    CreateDeployment { spec_id: String },

    /// Update an existing deployment
    UpdateDeployment { deployment_id: String },

    /// Scale a deployment
    ScaleDeployment {
        deployment_id: String,
        target_replicas: u32,
    },

    /// Delete a deployment
    DeleteDeployment { deployment_id: String },

    /// Rollback a deployment
    RollbackDeployment { deployment_id: String },

    /// Pause a deployment
    PauseDeployment { deployment_id: String },

    /// Resume a deployment
    ResumeDeployment { deployment_id: String },

    // ========== Instance Operations ==========
    /// Restart an instance
    RestartInstance { instance_id: String },

    /// Terminate an instance
    TerminateInstance { instance_id: String },

    /// Migrate an instance
    MigrateInstance { instance_id: String },

    /// Drain an instance
    DrainInstance { instance_id: String },

    // ========== State Operations ==========
    /// Create a checkpoint
    CreateCheckpoint { instance_id: String },

    /// Restore from a checkpoint
    RestoreCheckpoint { instance_id: String },

    /// Delete a checkpoint/snapshot
    DeleteCheckpoint { snapshot_id: String },

    // ========== Health Operations ==========
    /// Trigger a health check
    HealthCheck { instance_id: String },

    /// Force recovery
    ForceRecovery { instance_id: String },

    // ========== Administrative Operations ==========
    /// Configure a policy
    ConfigurePolicy { policy_name: String },

    /// View audit logs
    ViewAuditLog { filter: String },
}

/// Context for policy evaluation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyContext {
    /// Identity of the requester
    pub requester_id: Option<String>,

    /// Platform profile
    pub platform: Option<PlatformProfile>,

    /// Environment (prod, staging, dev)
    pub environment: Option<String>,

    /// Whether human approval was obtained
    pub human_approval: bool,

    /// Approver identity if human approval was obtained
    pub approver_id: Option<String>,

    /// Additional context data
    pub metadata: std::collections::HashMap<String, String>,
}

impl PolicyContext {
    /// Check if human approval was obtained
    pub fn has_human_approval(&self) -> bool {
        self.human_approval
    }

    /// Create a context with human approval
    pub fn with_human_approval(mut self, approver_id: impl Into<String>) -> Self {
        self.human_approval = true;
        self.approver_id = Some(approver_id.into());
        self
    }
}

/// Policy error returned when operation is denied
#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
pub enum PolicyError {
    #[error("Operation denied: {reason}")]
    Denied { reason: String },

    #[error("Missing required approval: {approver}")]
    MissingApproval { approver: String },

    #[error("Resource quota exceeded: {resource}")]
    QuotaExceeded { resource: String },

    #[error("Platform constraint violated: {constraint}")]
    PlatformConstraint { constraint: String },

    #[error("Time restriction violated: {restriction}")]
    TimeRestriction { restriction: String },

    #[error("Policy evaluation failed: {reason}")]
    EvaluationFailed { reason: String },
}

/// Policy decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyDecision {
    /// Operation is allowed
    Allow,

    /// Operation is denied
    Deny { reason: String },

    /// Operation requires manual approval
    RequiresApproval {
        approvers: Vec<String>,
        reason: String,
    },
}

/// Policy rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Rule identifier
    pub id: String,

    /// Rule name
    pub name: String,

    /// Rule description
    pub description: String,

    /// Operations this rule applies to
    pub operations: Vec<OperationType>,

    /// Conditions for rule activation
    pub conditions: Vec<PolicyCondition>,

    /// Action when rule matches
    pub action: PolicyAction,

    /// Rule priority (higher = evaluated first)
    pub priority: u32,

    /// Whether rule is enabled
    pub enabled: bool,
}

/// Operation types for policy matching
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationType {
    Create,
    Update,
    Scale,
    Delete,
    Rollback,
    All,
}

/// Policy condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyCondition {
    /// Platform matches
    Platform(PlatformProfile),

    /// Environment matches
    Environment(String),

    /// Scale exceeds threshold
    ScaleExceeds(u32),

    /// Time outside allowed window
    TimeRestriction {
        allowed_start_hour: u8,
        allowed_end_hour: u8,
    },

    /// Custom condition
    Custom { expression: String },
}

/// Policy action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyAction {
    /// Allow the operation
    Allow,

    /// Deny the operation
    Deny { reason: String },

    /// Require approval
    RequireApproval { approvers: Vec<String> },

    /// Add audit requirement
    Audit,
}
