//! Operation types for policy evaluation
//!
//! This module defines all operations that can be performed through the control plane.
//! These are used for policy evaluation and audit logging.

use palm_state::StateSnapshotId;
use palm_types::{AgentSpecId, DeploymentId, InstanceId, PlatformProfile};
use serde::{Deserialize, Serialize};

/// Node identifier for migration
pub type NodeId = String;

/// All operations that can be performed through the control plane
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlPlaneOperation {
    // ========== Registry Operations ==========
    /// Register a new agent specification
    RegisterSpec {
        /// Spec identifier
        spec_id: AgentSpecId,
    },
    /// Update an existing spec
    UpdateSpec {
        /// Spec identifier
        spec_id: AgentSpecId,
    },
    /// Deprecate a spec version
    DeprecateSpec {
        /// Spec identifier
        spec_id: AgentSpecId,
    },

    // ========== Deployment Operations ==========
    /// Create a new deployment
    CreateDeployment {
        /// Spec to deploy
        spec_id: AgentSpecId,
        /// Initial replica count
        replicas: u32,
    },
    /// Update a deployment to a new spec version
    UpdateDeployment {
        /// Deployment to update
        deployment_id: DeploymentId,
        /// New spec version
        new_spec_id: AgentSpecId,
    },
    /// Scale a deployment
    ScaleDeployment {
        /// Deployment to scale
        deployment_id: DeploymentId,
        /// Target replica count
        replicas: u32,
    },
    /// Delete a deployment
    DeleteDeployment {
        /// Deployment to delete
        deployment_id: DeploymentId,
    },
    /// Pause a deployment
    PauseDeployment {
        /// Deployment to pause
        deployment_id: DeploymentId,
    },
    /// Resume a paused deployment
    ResumeDeployment {
        /// Deployment to resume
        deployment_id: DeploymentId,
    },
    /// Rollback to a previous version
    RollbackDeployment {
        /// Deployment to rollback
        deployment_id: DeploymentId,
        /// Target version (None = previous)
        target_version: Option<String>,
    },

    // ========== Instance Operations ==========
    /// Restart an instance
    RestartInstance {
        /// Instance to restart
        instance_id: InstanceId,
        /// Whether to restart gracefully
        graceful: bool,
    },
    /// Terminate an instance
    TerminateInstance {
        /// Instance to terminate
        instance_id: InstanceId,
        /// Whether to terminate gracefully
        graceful: bool,
    },
    /// Migrate an instance to another node
    MigrateInstance {
        /// Instance to migrate
        instance_id: InstanceId,
        /// Target node
        to_node: NodeId,
    },
    /// Drain an instance before termination
    DrainInstance {
        /// Instance to drain
        instance_id: InstanceId,
    },

    // ========== State Operations ==========
    /// Create a checkpoint for an instance
    CreateCheckpoint {
        /// Instance to checkpoint
        instance_id: InstanceId,
    },
    /// Restore from a checkpoint
    RestoreFromCheckpoint {
        /// Instance to restore
        instance_id: InstanceId,
        /// Snapshot to restore from
        snapshot_id: StateSnapshotId,
    },
    /// Delete a snapshot
    DeleteSnapshot {
        /// Snapshot to delete
        snapshot_id: StateSnapshotId,
    },

    // ========== Health Operations ==========
    /// Trigger a health check
    TriggerHealthCheck {
        /// Instance to check
        instance_id: InstanceId,
    },
    /// Force recovery action
    ForceRecovery {
        /// Instance to recover
        instance_id: InstanceId,
    },

    // ========== Administrative Operations ==========
    /// Configure a policy
    ConfigurePolicy {
        /// Policy name
        policy_name: String,
    },
    /// View audit logs
    ViewAuditLog {
        /// Filter expression
        filter: String,
    },
}

impl ControlPlaneOperation {
    /// Get the operation category for policy evaluation
    pub fn category(&self) -> OperationCategory {
        match self {
            Self::RegisterSpec { .. } | Self::UpdateSpec { .. } | Self::DeprecateSpec { .. } => {
                OperationCategory::Registry
            }
            Self::CreateDeployment { .. }
            | Self::UpdateDeployment { .. }
            | Self::ScaleDeployment { .. }
            | Self::DeleteDeployment { .. }
            | Self::PauseDeployment { .. }
            | Self::ResumeDeployment { .. }
            | Self::RollbackDeployment { .. } => OperationCategory::Deployment,
            Self::RestartInstance { .. }
            | Self::TerminateInstance { .. }
            | Self::MigrateInstance { .. }
            | Self::DrainInstance { .. } => OperationCategory::Instance,
            Self::CreateCheckpoint { .. }
            | Self::RestoreFromCheckpoint { .. }
            | Self::DeleteSnapshot { .. } => OperationCategory::State,
            Self::TriggerHealthCheck { .. } | Self::ForceRecovery { .. } => {
                OperationCategory::Health
            }
            Self::ConfigurePolicy { .. } | Self::ViewAuditLog { .. } => {
                OperationCategory::Administrative
            }
        }
    }

    /// Check if operation is destructive
    pub fn is_destructive(&self) -> bool {
        matches!(
            self,
            Self::DeleteDeployment { .. }
                | Self::TerminateInstance {
                    graceful: false,
                    ..
                }
                | Self::DeleteSnapshot { .. }
        )
    }

    /// Check if operation modifies state
    pub fn is_mutating(&self) -> bool {
        !matches!(
            self,
            Self::ViewAuditLog { .. } | Self::TriggerHealthCheck { .. }
        )
    }

    /// Check if operation requires human approval for a given platform
    pub fn requires_human_approval(&self, platform: &PlatformProfile) -> bool {
        match platform {
            PlatformProfile::IBank => {
                // iBank requires approval for destructive and state-changing operations
                self.is_destructive()
                    || matches!(
                        self,
                        Self::RestoreFromCheckpoint { .. }
                            | Self::RollbackDeployment { .. }
                            | Self::ForceRecovery { .. }
                    )
            }
            PlatformProfile::Finalverse => {
                // Finalverse requires approval for operations affecting human-coupled agents
                self.is_destructive()
            }
            PlatformProfile::Mapleverse => {
                // Mapleverse only requires approval for force operations
                matches!(
                    self,
                    Self::TerminateInstance {
                        graceful: false,
                        ..
                    } | Self::ForceRecovery { .. }
                )
            }
            PlatformProfile::Development => {
                // Development never requires approval
                false
            }
        }
    }

    /// Get a human-readable description of the operation
    pub fn description(&self) -> String {
        match self {
            Self::RegisterSpec { spec_id } => format!("Register spec {}", spec_id),
            Self::UpdateSpec { spec_id } => format!("Update spec {}", spec_id),
            Self::DeprecateSpec { spec_id } => format!("Deprecate spec {}", spec_id),
            Self::CreateDeployment { spec_id, replicas } => {
                format!(
                    "Create deployment of {} with {} replicas",
                    spec_id, replicas
                )
            }
            Self::UpdateDeployment {
                deployment_id,
                new_spec_id,
            } => format!("Update deployment {} to {}", deployment_id, new_spec_id),
            Self::ScaleDeployment {
                deployment_id,
                replicas,
            } => format!(
                "Scale deployment {} to {} replicas",
                deployment_id, replicas
            ),
            Self::DeleteDeployment { deployment_id } => {
                format!("Delete deployment {}", deployment_id)
            }
            Self::PauseDeployment { deployment_id } => {
                format!("Pause deployment {}", deployment_id)
            }
            Self::ResumeDeployment { deployment_id } => {
                format!("Resume deployment {}", deployment_id)
            }
            Self::RollbackDeployment {
                deployment_id,
                target_version,
            } => {
                if let Some(v) = target_version {
                    format!("Rollback deployment {} to version {}", deployment_id, v)
                } else {
                    format!("Rollback deployment {} to previous version", deployment_id)
                }
            }
            Self::RestartInstance {
                instance_id,
                graceful,
            } => {
                let mode = if *graceful {
                    "gracefully"
                } else {
                    "forcefully"
                };
                format!("Restart instance {} {}", instance_id, mode)
            }
            Self::TerminateInstance {
                instance_id,
                graceful,
            } => {
                let mode = if *graceful {
                    "gracefully"
                } else {
                    "forcefully"
                };
                format!("Terminate instance {} {}", instance_id, mode)
            }
            Self::MigrateInstance {
                instance_id,
                to_node,
            } => format!("Migrate instance {} to node {}", instance_id, to_node),
            Self::DrainInstance { instance_id } => format!("Drain instance {}", instance_id),
            Self::CreateCheckpoint { instance_id } => {
                format!("Create checkpoint for {}", instance_id)
            }
            Self::RestoreFromCheckpoint {
                instance_id,
                snapshot_id,
            } => format!(
                "Restore instance {} from snapshot {}",
                instance_id, snapshot_id
            ),
            Self::DeleteSnapshot { snapshot_id } => format!("Delete snapshot {}", snapshot_id),
            Self::TriggerHealthCheck { instance_id } => {
                format!("Trigger health check for {}", instance_id)
            }
            Self::ForceRecovery { instance_id } => {
                format!("Force recovery for {}", instance_id)
            }
            Self::ConfigurePolicy { policy_name } => format!("Configure policy {}", policy_name),
            Self::ViewAuditLog { filter } => format!("View audit log with filter: {}", filter),
        }
    }
}

/// Operation categories for policy matching
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationCategory {
    /// Registry operations (specs)
    Registry,
    /// Deployment operations
    Deployment,
    /// Instance operations
    Instance,
    /// State operations (checkpoints, restore)
    State,
    /// Health operations
    Health,
    /// Administrative operations
    Administrative,
}

impl std::fmt::Display for OperationCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Registry => write!(f, "registry"),
            Self::Deployment => write!(f, "deployment"),
            Self::Instance => write!(f, "instance"),
            Self::State => write!(f, "state"),
            Self::Health => write!(f, "health"),
            Self::Administrative => write!(f, "administrative"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_categories() {
        let op = ControlPlaneOperation::CreateDeployment {
            spec_id: AgentSpecId::generate(),
            replicas: 3,
        };
        assert_eq!(op.category(), OperationCategory::Deployment);

        let op = ControlPlaneOperation::RestoreFromCheckpoint {
            instance_id: InstanceId::generate(),
            snapshot_id: StateSnapshotId::generate(),
        };
        assert_eq!(op.category(), OperationCategory::State);
    }

    #[test]
    fn test_destructive_operations() {
        let op = ControlPlaneOperation::DeleteDeployment {
            deployment_id: DeploymentId::generate(),
        };
        assert!(op.is_destructive());

        let op = ControlPlaneOperation::TerminateInstance {
            instance_id: InstanceId::generate(),
            graceful: false,
        };
        assert!(op.is_destructive());

        let op = ControlPlaneOperation::TerminateInstance {
            instance_id: InstanceId::generate(),
            graceful: true,
        };
        assert!(!op.is_destructive());
    }

    #[test]
    fn test_human_approval_requirements() {
        let op = ControlPlaneOperation::DeleteDeployment {
            deployment_id: DeploymentId::generate(),
        };

        assert!(op.requires_human_approval(&PlatformProfile::IBank));
        assert!(op.requires_human_approval(&PlatformProfile::Finalverse));
        assert!(!op.requires_human_approval(&PlatformProfile::Development));
    }

    #[test]
    fn test_operation_description() {
        let op = ControlPlaneOperation::ScaleDeployment {
            deployment_id: DeploymentId::generate(),
            replicas: 5,
        };
        let desc = op.description();
        assert!(desc.contains("Scale deployment"));
        assert!(desc.contains("5 replicas"));
    }
}
