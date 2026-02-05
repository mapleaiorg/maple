//! IBank accountability-first policy
//!
//! IBank prioritizes accountability and comprehensive audit trails.
//! This policy ensures all operations are properly recorded and
//! attributed to specific actors.

use crate::context::PolicyEvaluationContext;
use crate::decision::PolicyDecision;
use crate::error::Result;
use crate::gate::PolicyGate;
use async_trait::async_trait;
use chrono::Timelike;
use palm_types::policy::PalmOperation;
use palm_types::PlatformProfile;

/// IBank accountability-first policy
///
/// Optimized for financial/regulated workloads with:
/// - Comprehensive audit trail requirements
/// - Actor attribution for all operations
/// - Strict change window enforcement
#[derive(Debug)]
pub struct IBankAccountabilityPolicy {
    /// Require actor attribution for all operations
    require_actor_attribution: bool,

    /// Block anonymous operations
    block_anonymous: bool,

    /// Allowed change hours (24-hour format, 0-23)
    change_window_start: u8,
    change_window_end: u8,

    /// Enforce change windows
    enforce_change_windows: bool,

    /// Maximum operations per actor per hour
    max_operations_per_actor: u32,
}

impl IBankAccountabilityPolicy {
    /// Create a new IBank accountability policy
    pub fn new() -> Self {
        Self {
            require_actor_attribution: true,
            block_anonymous: true,
            change_window_start: 6, // 6 AM
            change_window_end: 22,  // 10 PM
            enforce_change_windows: true,
            max_operations_per_actor: 100,
        }
    }

    /// Set whether to require actor attribution
    pub fn with_require_actor_attribution(mut self, require: bool) -> Self {
        self.require_actor_attribution = require;
        self
    }

    /// Set whether to block anonymous operations
    pub fn with_block_anonymous(mut self, block: bool) -> Self {
        self.block_anonymous = block;
        self
    }

    /// Set the change window hours
    pub fn with_change_window(mut self, start: u8, end: u8) -> Self {
        self.change_window_start = start.min(23);
        self.change_window_end = end.min(23);
        self
    }

    /// Set whether to enforce change windows
    pub fn with_enforce_change_windows(mut self, enforce: bool) -> Self {
        self.enforce_change_windows = enforce;
        self
    }

    /// Set maximum operations per actor per hour
    pub fn with_max_operations_per_actor(mut self, max: u32) -> Self {
        self.max_operations_per_actor = max;
        self
    }

    /// Check actor attribution
    fn check_attribution(&self, context: &PolicyEvaluationContext) -> Option<PolicyDecision> {
        if !self.require_actor_attribution {
            return None;
        }

        // Check for anonymous actors
        if self.block_anonymous && context.actor_id == "anonymous" {
            return Some(PolicyDecision::deny(
                "Anonymous operations are not allowed in IBank",
                self.id(),
            ));
        }

        // Ensure actor ID is not empty
        if context.actor_id.is_empty() {
            return Some(PolicyDecision::deny(
                "Actor attribution is required for all operations",
                self.id(),
            ));
        }

        None
    }

    /// Check change window restrictions
    fn check_change_window(&self, context: &PolicyEvaluationContext) -> Option<PolicyDecision> {
        if !self.enforce_change_windows {
            return None;
        }

        // Only enforce for production
        if !context.is_production() {
            return None;
        }

        // Allow system actors to operate outside change windows
        if context.is_system_actor() {
            return None;
        }

        let hour = context.timestamp.hour() as u8;
        let in_window = if self.change_window_start <= self.change_window_end {
            hour >= self.change_window_start && hour < self.change_window_end
        } else {
            // Handle overnight windows (e.g., 22:00 - 06:00)
            hour >= self.change_window_start || hour < self.change_window_end
        };

        if !in_window {
            return Some(PolicyDecision::deny(
                format!(
                    "Operations not allowed outside change window ({:02}:00 - {:02}:00)",
                    self.change_window_start, self.change_window_end
                ),
                self.id(),
            ));
        }

        None
    }

    /// Check operation rate limits per actor
    fn check_rate_limit(&self, context: &PolicyEvaluationContext) -> Option<PolicyDecision> {
        if self.max_operations_per_actor == 0 {
            return None;
        }

        if let Some(ref usage) = context.quota_usage {
            if usage.operations_per_hour > self.max_operations_per_actor {
                return Some(PolicyDecision::deny(
                    format!(
                        "Actor rate limit exceeded: {} operations/hour (max: {})",
                        usage.operations_per_hour, self.max_operations_per_actor
                    ),
                    self.id(),
                ));
            }
        }

        None
    }

    /// Check for high-risk operations that need extra scrutiny
    fn check_high_risk_operation(
        &self,
        operation: &PalmOperation,
        context: &PolicyEvaluationContext,
    ) -> Option<PolicyDecision> {
        // High-risk operations in production require human approval
        if !context.is_production() {
            return None;
        }

        let is_high_risk = matches!(
            operation,
            PalmOperation::DeleteDeployment { .. }
                | PalmOperation::DeleteCheckpoint { .. }
                | PalmOperation::TerminateInstance { .. }
        );

        if is_high_risk && !context.has_human_approval() {
            return Some(PolicyDecision::requires_approval(
                vec![
                    "risk-management@ibank.example.com".into(),
                    "compliance@ibank.example.com".into(),
                ],
                "High-risk operations require approval from risk management",
                self.id(),
            ));
        }

        None
    }
}

impl Default for IBankAccountabilityPolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PolicyGate for IBankAccountabilityPolicy {
    fn id(&self) -> &str {
        "ibank-accountability"
    }

    fn name(&self) -> &str {
        "IBank Accountability Policy"
    }

    async fn evaluate(
        &self,
        operation: &PalmOperation,
        context: &PolicyEvaluationContext,
    ) -> Result<PolicyDecision> {
        // Skip for non-IBank platforms (except development)
        if context.platform != PlatformProfile::IBank
            && context.platform != PlatformProfile::Development
        {
            return Ok(PolicyDecision::allow());
        }

        // For development, be more lenient but still check attribution
        if context.platform == PlatformProfile::Development {
            if let Some(decision) = self.check_attribution(context) {
                return Ok(decision);
            }
            return Ok(PolicyDecision::allow());
        }

        // Check actor attribution (always required)
        if let Some(decision) = self.check_attribution(context) {
            return Ok(decision);
        }

        // Check rate limits
        if let Some(decision) = self.check_rate_limit(context) {
            return Ok(decision);
        }

        // Check change window for mutable operations
        let is_mutable = matches!(
            operation,
            PalmOperation::CreateDeployment { .. }
                | PalmOperation::UpdateDeployment { .. }
                | PalmOperation::ScaleDeployment { .. }
                | PalmOperation::DeleteDeployment { .. }
                | PalmOperation::RollbackDeployment { .. }
                | PalmOperation::PauseDeployment { .. }
                | PalmOperation::ResumeDeployment { .. }
                | PalmOperation::TerminateInstance { .. }
                | PalmOperation::MigrateInstance { .. }
        );

        if is_mutable {
            if let Some(decision) = self.check_change_window(context) {
                return Ok(decision);
            }
        }

        // Check high-risk operations
        if let Some(decision) = self.check_high_risk_operation(operation, context) {
            return Ok(decision);
        }

        Ok(PolicyDecision::allow())
    }

    fn description(&self) -> &str {
        "Accountability-first policy for IBank regulated workloads"
    }

    fn priority(&self) -> u32 {
        150 // Same priority as safety policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ActorType, HumanApproval, QuotaUsage};
    use chrono::{TimeZone, Utc};

    #[tokio::test]
    async fn test_ibank_blocks_anonymous() {
        let policy = IBankAccountabilityPolicy::new();
        let ctx = PolicyEvaluationContext::new("anonymous", PlatformProfile::IBank);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_denied());
        assert!(decision.reason().unwrap().contains("Anonymous"));
    }

    #[tokio::test]
    async fn test_ibank_allows_attributed() {
        let policy = IBankAccountabilityPolicy::new();
        let ctx = PolicyEvaluationContext::new("user-12345", PlatformProfile::IBank);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_ibank_rate_limit() {
        let policy = IBankAccountabilityPolicy::new();
        let mut usage = QuotaUsage::default();
        usage.operations_per_hour = 150;

        let ctx =
            PolicyEvaluationContext::new("user-1", PlatformProfile::IBank).with_quota_usage(usage);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_denied());
        assert!(decision.reason().unwrap().contains("rate limit"));
    }

    #[tokio::test]
    async fn test_ibank_change_window_production() {
        let policy = IBankAccountabilityPolicy::new().with_change_window(9, 17); // 9 AM - 5 PM

        // Create context at 3 AM (outside window)
        let mut ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::IBank)
            .with_environment("production");
        ctx.timestamp = Utc.with_ymd_and_hms(2026, 1, 15, 3, 0, 0).unwrap();

        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_denied());
        assert!(decision.reason().unwrap().contains("change window"));
    }

    #[tokio::test]
    async fn test_ibank_change_window_non_production() {
        let policy = IBankAccountabilityPolicy::new().with_change_window(9, 17); // 9 AM - 5 PM

        // Create context at 3 AM (outside window) but in staging
        let mut ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::IBank)
            .with_environment("staging");
        ctx.timestamp = Utc.with_ymd_and_hms(2026, 1, 15, 3, 0, 0).unwrap();

        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_ibank_system_actor_bypasses_window() {
        let policy = IBankAccountabilityPolicy::new().with_change_window(9, 17);

        let mut ctx = PolicyEvaluationContext::new("health-monitor", PlatformProfile::IBank)
            .with_actor_type(ActorType::System)
            .with_environment("production");
        ctx.timestamp = Utc.with_ymd_and_hms(2026, 1, 15, 3, 0, 0).unwrap();

        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_ibank_high_risk_requires_approval() {
        let policy = IBankAccountabilityPolicy::new();
        // Set timestamp within change window (6 AM - 10 PM)
        let mut ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::IBank)
            .with_environment("production");
        ctx.timestamp = Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap(); // 12:00 noon
        let op = PalmOperation::DeleteDeployment {
            deployment_id: "deploy-1".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.requires_human_approval());
    }

    #[tokio::test]
    async fn test_ibank_high_risk_with_approval() {
        let policy = IBankAccountabilityPolicy::new();
        let approval = HumanApproval::new("risk-management@ibank.example.com");
        // Set timestamp within change window (6 AM - 10 PM)
        let mut ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::IBank)
            .with_environment("production")
            .with_human_approval(approval);
        ctx.timestamp = Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap(); // 12:00 noon
        let op = PalmOperation::DeleteDeployment {
            deployment_id: "deploy-1".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_ibank_read_operations_outside_window() {
        let policy = IBankAccountabilityPolicy::new().with_change_window(9, 17);

        let mut ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::IBank)
            .with_environment("production");
        ctx.timestamp = Utc.with_ymd_and_hms(2026, 1, 15, 3, 0, 0).unwrap();

        // ViewAuditLog is a read operation, should be allowed
        let op = PalmOperation::ViewAuditLog {
            filter: "deployment-id:123".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }
}
