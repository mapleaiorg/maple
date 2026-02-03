//! UAL Types - Universal Agent Language AST
//!
//! UAL is the interaction language (SQL-like DDL/DML) for humans and agents.
//! It compiles into formal artifacts (RCF commitments or PALM operations).

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};

/// A parsed UAL statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UalStatement {
    /// Commitment statement (compiles to RCF).
    Commit(CommitStatement),
    /// Operational statement (compiles to PALM operations).
    Operation(OperationStatement),
}

/// A UAL commitment statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitStatement {
    /// Principal identity declaring the commitment.
    pub principal: String,
    /// Effect domain (e.g., computation, finance).
    pub domain: String,
    /// Intended outcome description.
    pub outcome: String,
    /// Optional scope clause (e.g., GLOBAL or a specific target).
    pub scope: Option<String>,
    /// Optional targets.
    pub targets: Vec<String>,
    /// Optional policy tags.
    pub tags: Vec<String>,
    /// Optional reversibility flag.
    pub reversibility: Option<ReversibilitySpec>,
    /// Optional validity start (RFC3339).
    pub valid_from: Option<String>,
    /// Optional validity end (RFC3339).
    pub valid_until: Option<String>,
}

/// Reversibility clause for a commitment.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ReversibilitySpec {
    Reversible,
    Irreversible,
}

/// Operational statements that map to PALM operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationStatement {
    // Registry operations
    CreateSpec { spec_id: String, version: Option<String> },
    UpdateSpec { spec_id: String, version: Option<String> },
    DeprecateSpec { spec_id: String },

    // Deployment operations
    CreateDeployment { spec_id: String, replicas: u32 },
    UpdateDeployment { deployment_id: String },
    ScaleDeployment { deployment_id: String, target_replicas: u32 },
    DeleteDeployment { deployment_id: String },
    RollbackDeployment { deployment_id: String },
    PauseDeployment { deployment_id: String },
    ResumeDeployment { deployment_id: String },

    // Instance operations
    RestartInstance { instance_id: String },
    TerminateInstance { instance_id: String },
    MigrateInstance { instance_id: String },
    DrainInstance { instance_id: String },

    // State operations
    CreateCheckpoint { instance_id: String },
    RestoreCheckpoint { instance_id: String },
    DeleteCheckpoint { snapshot_id: String },

    // Health operations
    HealthCheck { instance_id: String },
    ForceRecovery { instance_id: String },

    // Administrative operations
    ConfigurePolicy { policy_name: String },
    ViewAuditLog { filter: String },
}
