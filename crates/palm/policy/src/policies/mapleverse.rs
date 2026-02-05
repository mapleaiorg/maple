//! Mapleverse throughput-first policy
//!
//! Mapleverse prioritizes throughput and low-latency operations.
//! This policy is permissive to enable high-velocity operations
//! while still maintaining basic safety guardrails.

use crate::context::PolicyEvaluationContext;
use crate::decision::PolicyDecision;
use crate::error::Result;
use crate::gate::PolicyGate;
use async_trait::async_trait;
use palm_types::policy::PalmOperation;
use palm_types::PlatformProfile;

/// Mapleverse throughput-first policy
///
/// Optimized for gaming workloads with:
/// - High operation velocity
/// - Relaxed approval requirements
/// - Fast rollback capability
#[derive(Debug)]
pub struct MapleverseThroughputPolicy {
    /// Maximum concurrent deployments
    max_concurrent_deployments: u32,

    /// Scale threshold requiring review (0 = no review)
    scale_review_threshold: u32,

    /// Allow auto-rollback
    allow_auto_rollback: bool,
}

impl MapleverseThroughputPolicy {
    /// Create a new Mapleverse throughput policy
    pub fn new() -> Self {
        Self {
            max_concurrent_deployments: 100,
            scale_review_threshold: 0, // No review required
            allow_auto_rollback: true,
        }
    }

    /// Set maximum concurrent deployments
    pub fn with_max_concurrent_deployments(mut self, max: u32) -> Self {
        self.max_concurrent_deployments = max;
        self
    }

    /// Set scale review threshold
    pub fn with_scale_review_threshold(mut self, threshold: u32) -> Self {
        self.scale_review_threshold = threshold;
        self
    }

    /// Check if operation is for the correct platform
    fn check_platform(&self, context: &PolicyEvaluationContext) -> Option<PolicyDecision> {
        if context.platform != PlatformProfile::Mapleverse {
            return Some(PolicyDecision::deny(
                "Operation not allowed for non-Mapleverse platform",
                self.id(),
            ));
        }
        None
    }

    /// Check scale operations
    fn check_scale(&self, target_replicas: u32) -> Option<PolicyDecision> {
        if self.scale_review_threshold > 0 && target_replicas > self.scale_review_threshold {
            return Some(PolicyDecision::requires_approval(
                vec!["platform-ops@mapleverse.example.com".into()],
                format!(
                    "Scale to {} replicas exceeds review threshold of {}",
                    target_replicas, self.scale_review_threshold
                ),
                self.id(),
            ));
        }
        None
    }
}

impl Default for MapleverseThroughputPolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PolicyGate for MapleverseThroughputPolicy {
    fn id(&self) -> &str {
        "mapleverse-throughput"
    }

    fn name(&self) -> &str {
        "Mapleverse Throughput Policy"
    }

    async fn evaluate(
        &self,
        operation: &PalmOperation,
        context: &PolicyEvaluationContext,
    ) -> Result<PolicyDecision> {
        // Verify platform (allow Development to pass through)
        if context.platform != PlatformProfile::Mapleverse
            && context.platform != PlatformProfile::Development
        {
            if let Some(decision) = self.check_platform(context) {
                return Ok(decision);
            }
        }

        // Check scale operations
        if let PalmOperation::ScaleDeployment {
            target_replicas, ..
        } = operation
        {
            if let Some(decision) = self.check_scale(*target_replicas) {
                return Ok(decision);
            }
        }

        // Mapleverse allows most operations with minimal friction
        // Rollback is always allowed for fast recovery
        if let PalmOperation::RollbackDeployment { .. } = operation {
            if self.allow_auto_rollback {
                return Ok(PolicyDecision::allow());
            }
        }

        Ok(PolicyDecision::allow())
    }

    fn description(&self) -> &str {
        "Throughput-first policy for Mapleverse gaming workloads"
    }

    fn priority(&self) -> u32 {
        100
    }

    fn applies_to(&self, _operation: &PalmOperation) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mapleverse_allows_deployments() {
        let policy = MapleverseThroughputPolicy::new();
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Mapleverse);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_mapleverse_allows_rollback() {
        let policy = MapleverseThroughputPolicy::new();
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Mapleverse);
        let op = PalmOperation::RollbackDeployment {
            deployment_id: "deploy-1".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_mapleverse_scale_no_review() {
        let policy = MapleverseThroughputPolicy::new();
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Mapleverse);
        let op = PalmOperation::ScaleDeployment {
            deployment_id: "deploy-1".into(),
            target_replicas: 50,
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_mapleverse_scale_with_threshold() {
        let policy = MapleverseThroughputPolicy::new().with_scale_review_threshold(10);
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Mapleverse);
        let op = PalmOperation::ScaleDeployment {
            deployment_id: "deploy-1".into(),
            target_replicas: 20,
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.requires_human_approval());
    }

    #[tokio::test]
    async fn test_mapleverse_allows_development() {
        let policy = MapleverseThroughputPolicy::new();
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Development);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }
}
