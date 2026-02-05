//! Finalverse safety-first policy
//!
//! Finalverse prioritizes safety and deliberate operations.
//! This policy requires human approval for critical operations
//! and enforces strict change management.

use crate::context::{HumanApproval, PolicyEvaluationContext};
use crate::decision::PolicyDecision;
use crate::error::Result;
use crate::gate::PolicyGate;
use async_trait::async_trait;
use palm_types::policy::PalmOperation;
use palm_types::PlatformProfile;

/// Finalverse safety-first policy
///
/// Optimized for safety-critical workloads with:
/// - Human approval for all production changes
/// - Strict rollback controls
/// - Mandatory review for scaling
#[derive(Debug)]
pub struct FinalverseSafetyPolicy {
    /// Require approval for all production deployments
    require_production_approval: bool,

    /// Scale threshold requiring approval
    scale_approval_threshold: u32,

    /// Require approval for rollbacks
    require_rollback_approval: bool,

    /// Allowed approvers
    allowed_approvers: Vec<String>,
}

impl FinalverseSafetyPolicy {
    /// Create a new Finalverse safety policy
    pub fn new() -> Self {
        Self {
            require_production_approval: true,
            scale_approval_threshold: 5,
            require_rollback_approval: true,
            allowed_approvers: vec![
                "safety-team@finalverse.example.com".into(),
                "platform-ops@finalverse.example.com".into(),
            ],
        }
    }

    /// Set whether to require production approval
    pub fn with_require_production_approval(mut self, require: bool) -> Self {
        self.require_production_approval = require;
        self
    }

    /// Set the scale approval threshold
    pub fn with_scale_approval_threshold(mut self, threshold: u32) -> Self {
        self.scale_approval_threshold = threshold;
        self
    }

    /// Set whether to require rollback approval
    pub fn with_require_rollback_approval(mut self, require: bool) -> Self {
        self.require_rollback_approval = require;
        self
    }

    /// Add an allowed approver
    pub fn with_approver(mut self, approver: impl Into<String>) -> Self {
        self.allowed_approvers.push(approver.into());
        self
    }

    /// Check if the approval is valid
    fn is_valid_approval(&self, approval: &HumanApproval) -> bool {
        // Check if approver is in the allowed list
        if !self.allowed_approvers.is_empty() {
            if !self.allowed_approvers.contains(&approval.approver_id) {
                return false;
            }
        }

        // Check if approval is still valid (not expired)
        approval.is_valid()
    }

    /// Check production deployment requirements
    fn check_production_deployment(
        &self,
        context: &PolicyEvaluationContext,
    ) -> Option<PolicyDecision> {
        if !self.require_production_approval {
            return None;
        }

        if !context.is_production() {
            return None;
        }

        // Check for valid human approval
        match &context.human_approval {
            Some(approval) if self.is_valid_approval(approval) => None,
            Some(_) => Some(PolicyDecision::deny(
                "Human approval is invalid or expired",
                self.id(),
            )),
            None => Some(PolicyDecision::requires_approval(
                self.allowed_approvers.clone(),
                "Production deployments require human approval",
                self.id(),
            )),
        }
    }

    /// Check scale operation requirements
    fn check_scale(
        &self,
        target_replicas: u32,
        context: &PolicyEvaluationContext,
    ) -> Option<PolicyDecision> {
        if target_replicas <= self.scale_approval_threshold {
            return None;
        }

        // Check for valid human approval
        match &context.human_approval {
            Some(approval) if self.is_valid_approval(approval) => None,
            Some(_) => Some(PolicyDecision::deny(
                "Human approval is invalid or expired",
                self.id(),
            )),
            None => Some(PolicyDecision::requires_approval(
                self.allowed_approvers.clone(),
                format!(
                    "Scaling to {} replicas requires human approval (threshold: {})",
                    target_replicas, self.scale_approval_threshold
                ),
                self.id(),
            )),
        }
    }

    /// Check rollback requirements
    fn check_rollback(&self, context: &PolicyEvaluationContext) -> Option<PolicyDecision> {
        if !self.require_rollback_approval {
            return None;
        }

        // Allow system actors to rollback without approval (for automated recovery)
        if context.is_system_actor() {
            return None;
        }

        // Check for valid human approval
        match &context.human_approval {
            Some(approval) if self.is_valid_approval(approval) => None,
            Some(_) => Some(PolicyDecision::deny(
                "Human approval is invalid or expired",
                self.id(),
            )),
            None => Some(PolicyDecision::requires_approval(
                self.allowed_approvers.clone(),
                "Rollback operations require human approval in Finalverse",
                self.id(),
            )),
        }
    }

    /// Check destructive operations (terminate, delete)
    fn check_destructive(&self, context: &PolicyEvaluationContext) -> Option<PolicyDecision> {
        // Always require approval for destructive operations in production
        if !context.is_production() {
            return None;
        }

        match &context.human_approval {
            Some(approval) if self.is_valid_approval(approval) => None,
            Some(_) => Some(PolicyDecision::deny(
                "Human approval is invalid or expired",
                self.id(),
            )),
            None => Some(PolicyDecision::requires_approval(
                self.allowed_approvers.clone(),
                "Destructive operations require human approval in production",
                self.id(),
            )),
        }
    }
}

impl Default for FinalverseSafetyPolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PolicyGate for FinalverseSafetyPolicy {
    fn id(&self) -> &str {
        "finalverse-safety"
    }

    fn name(&self) -> &str {
        "Finalverse Safety Policy"
    }

    async fn evaluate(
        &self,
        operation: &PalmOperation,
        context: &PolicyEvaluationContext,
    ) -> Result<PolicyDecision> {
        // Skip for non-Finalverse platforms (except development)
        if context.platform != PlatformProfile::Finalverse
            && context.platform != PlatformProfile::Development
        {
            return Ok(PolicyDecision::allow());
        }

        // For development, be more lenient
        if context.platform == PlatformProfile::Development && !context.is_production() {
            return Ok(PolicyDecision::allow());
        }

        match operation {
            PalmOperation::CreateDeployment { .. } | PalmOperation::UpdateDeployment { .. } => {
                if let Some(decision) = self.check_production_deployment(context) {
                    return Ok(decision);
                }
            }

            PalmOperation::ScaleDeployment {
                target_replicas, ..
            } => {
                if let Some(decision) = self.check_scale(*target_replicas, context) {
                    return Ok(decision);
                }
            }

            PalmOperation::RollbackDeployment { .. } => {
                if let Some(decision) = self.check_rollback(context) {
                    return Ok(decision);
                }
            }

            PalmOperation::DeleteDeployment { .. }
            | PalmOperation::TerminateInstance { .. }
            | PalmOperation::DeleteCheckpoint { .. } => {
                if let Some(decision) = self.check_destructive(context) {
                    return Ok(decision);
                }
            }

            _ => {}
        }

        Ok(PolicyDecision::allow())
    }

    fn description(&self) -> &str {
        "Safety-first policy for Finalverse critical workloads"
    }

    fn priority(&self) -> u32 {
        150 // Higher than throughput policies
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ActorType;

    #[tokio::test]
    async fn test_finalverse_allows_non_production() {
        let policy = FinalverseSafetyPolicy::new();
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Finalverse)
            .with_environment("staging");
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_finalverse_requires_production_approval() {
        let policy = FinalverseSafetyPolicy::new();
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Finalverse)
            .with_environment("production");
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.requires_human_approval());
    }

    #[tokio::test]
    async fn test_finalverse_allows_with_approval() {
        let policy = FinalverseSafetyPolicy::new();
        let approval = HumanApproval::new("safety-team@finalverse.example.com");
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Finalverse)
            .with_environment("production")
            .with_human_approval(approval);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_finalverse_scale_threshold() {
        let policy = FinalverseSafetyPolicy::new();
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Finalverse);

        // Below threshold - allowed
        let op_small = PalmOperation::ScaleDeployment {
            deployment_id: "deploy-1".into(),
            target_replicas: 3,
        };
        let decision = policy.evaluate(&op_small, &ctx).await.unwrap();
        assert!(decision.is_allowed());

        // Above threshold - requires approval
        let op_large = PalmOperation::ScaleDeployment {
            deployment_id: "deploy-1".into(),
            target_replicas: 10,
        };
        let decision = policy.evaluate(&op_large, &ctx).await.unwrap();
        assert!(decision.requires_human_approval());
    }

    #[tokio::test]
    async fn test_finalverse_rollback_requires_approval() {
        let policy = FinalverseSafetyPolicy::new();
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Finalverse);
        let op = PalmOperation::RollbackDeployment {
            deployment_id: "deploy-1".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.requires_human_approval());
    }

    #[tokio::test]
    async fn test_finalverse_system_rollback_allowed() {
        let policy = FinalverseSafetyPolicy::new();
        let ctx = PolicyEvaluationContext::new("health-monitor", PlatformProfile::Finalverse)
            .with_actor_type(ActorType::System);
        let op = PalmOperation::RollbackDeployment {
            deployment_id: "deploy-1".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_finalverse_destructive_requires_approval() {
        let policy = FinalverseSafetyPolicy::new();
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Finalverse)
            .with_environment("production");
        let op = PalmOperation::DeleteDeployment {
            deployment_id: "deploy-1".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.requires_human_approval());
    }

    #[tokio::test]
    async fn test_finalverse_invalid_approver() {
        let policy = FinalverseSafetyPolicy::new();
        let approval = HumanApproval::new("random-user@example.com");
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Finalverse)
            .with_environment("production")
            .with_human_approval(approval);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_denied());
    }
}
