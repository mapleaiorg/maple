//! Base invariant policy
//!
//! This policy enforces invariants that apply to ALL platforms:
//! - Rate limiting
//! - Resource quotas
//! - Basic validation

use crate::context::PolicyEvaluationContext;
use crate::decision::PolicyDecision;
use crate::error::Result;
use crate::gate::PolicyGate;
use async_trait::async_trait;
use palm_types::policy::PalmOperation;

/// Base invariant policy that applies to all platforms
#[derive(Debug)]
pub struct BaseInvariantPolicy {
    /// Maximum operations per hour (0 = unlimited)
    max_operations_per_hour: u32,

    /// Maximum deployments per actor (0 = unlimited)
    max_deployments: u32,

    /// Maximum instances per deployment (0 = unlimited)
    max_instances_per_deployment: u32,
}

impl BaseInvariantPolicy {
    /// Create a new base invariant policy with default limits
    pub fn new() -> Self {
        Self {
            max_operations_per_hour: 1000,
            max_deployments: 100,
            max_instances_per_deployment: 50,
        }
    }

    /// Set the maximum operations per hour
    pub fn with_max_operations_per_hour(mut self, max: u32) -> Self {
        self.max_operations_per_hour = max;
        self
    }

    /// Set the maximum deployments
    pub fn with_max_deployments(mut self, max: u32) -> Self {
        self.max_deployments = max;
        self
    }

    /// Set the maximum instances per deployment
    pub fn with_max_instances_per_deployment(mut self, max: u32) -> Self {
        self.max_instances_per_deployment = max;
        self
    }

    /// Check rate limiting
    fn check_rate_limit(&self, context: &PolicyEvaluationContext) -> Option<PolicyDecision> {
        if self.max_operations_per_hour == 0 {
            return None;
        }

        if let Some(ref usage) = context.quota_usage {
            if usage.rate_limit_exceeded() {
                return Some(PolicyDecision::deny(
                    format!(
                        "Rate limit exceeded: {} operations/hour (max: {})",
                        usage.operations_per_hour,
                        usage.max_operations_per_hour.unwrap_or(self.max_operations_per_hour)
                    ),
                    self.id(),
                ));
            }
        }
        None
    }

    /// Check deployment quota
    fn check_deployment_quota(&self, context: &PolicyEvaluationContext) -> Option<PolicyDecision> {
        if self.max_deployments == 0 {
            return None;
        }

        if let Some(ref usage) = context.quota_usage {
            if usage.deployments_exceeded() {
                return Some(PolicyDecision::deny(
                    format!(
                        "Deployment quota exceeded: {} deployments (max: {})",
                        usage.deployments,
                        usage.max_deployments.unwrap_or(self.max_deployments)
                    ),
                    self.id(),
                ));
            }
        }
        None
    }

    /// Check scale operation limits
    fn check_scale_limits(&self, target_replicas: u32) -> Option<PolicyDecision> {
        if self.max_instances_per_deployment == 0 {
            return None;
        }

        if target_replicas > self.max_instances_per_deployment {
            return Some(PolicyDecision::deny(
                format!(
                    "Scale target {} exceeds maximum {} instances per deployment",
                    target_replicas, self.max_instances_per_deployment
                ),
                self.id(),
            ));
        }
        None
    }
}

impl Default for BaseInvariantPolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PolicyGate for BaseInvariantPolicy {
    fn id(&self) -> &str {
        "base-invariant"
    }

    fn name(&self) -> &str {
        "Base Invariant Policy"
    }

    async fn evaluate(
        &self,
        operation: &PalmOperation,
        context: &PolicyEvaluationContext,
    ) -> Result<PolicyDecision> {
        // Check rate limiting for all operations
        if let Some(decision) = self.check_rate_limit(context) {
            return Ok(decision);
        }

        // Check deployment quota for create operations
        if let PalmOperation::CreateDeployment { .. } = operation {
            if let Some(decision) = self.check_deployment_quota(context) {
                return Ok(decision);
            }
        }

        // Check scale limits
        if let PalmOperation::ScaleDeployment { target_replicas, .. } = operation {
            if let Some(decision) = self.check_scale_limits(*target_replicas) {
                return Ok(decision);
            }
        }

        Ok(PolicyDecision::allow())
    }

    fn description(&self) -> &str {
        "Enforces base invariants that apply to all platforms"
    }

    fn priority(&self) -> u32 {
        200 // Higher priority - base checks run first
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::QuotaUsage;
    use palm_types::PlatformProfile;

    #[tokio::test]
    async fn test_base_policy_allows_normal_operation() {
        let policy = BaseInvariantPolicy::new();
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Development);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_base_policy_rate_limit() {
        let policy = BaseInvariantPolicy::new();
        let mut usage = QuotaUsage::default();
        usage.operations_per_hour = 1001;
        usage.max_operations_per_hour = Some(1000);

        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Development)
            .with_quota_usage(usage);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_denied());
        assert!(decision.reason().unwrap().contains("Rate limit"));
    }

    #[tokio::test]
    async fn test_base_policy_deployment_quota() {
        let policy = BaseInvariantPolicy::new();
        let mut usage = QuotaUsage::default();
        usage.deployments = 100;
        usage.max_deployments = Some(100);

        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Development)
            .with_quota_usage(usage);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_denied());
        assert!(decision.reason().unwrap().contains("Deployment quota"));
    }

    #[tokio::test]
    async fn test_base_policy_scale_limit() {
        let policy = BaseInvariantPolicy::new();
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Development);
        let op = PalmOperation::ScaleDeployment {
            deployment_id: "deploy-1".into(),
            target_replicas: 100,
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_denied());
        assert!(decision.reason().unwrap().contains("exceeds maximum"));
    }

    #[tokio::test]
    async fn test_base_policy_custom_limits() {
        let policy = BaseInvariantPolicy::new()
            .with_max_instances_per_deployment(200);

        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Development);
        let op = PalmOperation::ScaleDeployment {
            deployment_id: "deploy-1".into(),
            target_replicas: 100,
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }
}
