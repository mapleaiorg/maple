//! Resilience controller for coordinating recovery actions.
//!
//! The ResilienceController evaluates health assessments and determines
//! appropriate recovery actions based on configuration and policy.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use palm_types::{InstanceId, PlatformProfile};
use tracing::{debug, info, instrument, warn};

use super::circuit_breaker::CircuitBreaker;
use super::recovery::{NotifySeverity, RecoveryAction, RecoveryContext, RecoveryOutcome};
use crate::assessment::{HealthAssessment, OverallHealth};
use crate::config::{HealthConfig, ResilienceConfig};
use crate::error::{HealthError, HealthResult};

/// Resilience controller for managing recovery actions.
pub struct ResilienceController {
    /// Configuration.
    config: ResilienceConfig,

    /// Platform profile for policy decisions.
    platform: PlatformProfile,

    /// Circuit breakers per instance.
    circuit_breakers: DashMap<InstanceId, Arc<CircuitBreaker>>,

    /// Recovery contexts per instance.
    recovery_contexts: DashMap<InstanceId, RecoveryContext>,

    /// Recovery executor.
    executor: Arc<dyn RecoveryExecutor>,

    /// Policy gate for approvals.
    policy_gate: Arc<dyn RecoveryPolicyGate>,
}

impl ResilienceController {
    /// Create a new resilience controller.
    pub fn new(
        config: ResilienceConfig,
        platform: PlatformProfile,
        executor: Arc<dyn RecoveryExecutor>,
    ) -> Self {
        Self {
            config,
            platform,
            circuit_breakers: DashMap::new(),
            recovery_contexts: DashMap::new(),
            executor,
            policy_gate: Arc::new(DefaultRecoveryPolicyGate),
        }
    }

    /// Create a controller with custom policy gate.
    pub fn with_policy_gate(
        config: ResilienceConfig,
        platform: PlatformProfile,
        executor: Arc<dyn RecoveryExecutor>,
        policy_gate: Arc<dyn RecoveryPolicyGate>,
    ) -> Self {
        Self {
            config,
            platform,
            circuit_breakers: DashMap::new(),
            recovery_contexts: DashMap::new(),
            executor,
            policy_gate,
        }
    }

    /// Create from health config.
    pub fn from_health_config(
        config: &HealthConfig,
        platform: PlatformProfile,
        executor: Arc<dyn RecoveryExecutor>,
    ) -> Self {
        Self::new(config.resilience.clone(), platform, executor)
    }

    /// Get or create circuit breaker for an instance.
    pub fn circuit_breaker(&self, instance_id: &InstanceId) -> Arc<CircuitBreaker> {
        self.circuit_breakers
            .entry(instance_id.clone())
            .or_insert_with(|| {
                Arc::new(CircuitBreaker::new(
                    instance_id.clone(),
                    self.config.circuit_breaker.clone(),
                ))
            })
            .clone()
    }

    /// Get or create recovery context for an instance.
    fn recovery_context(&self, instance_id: &InstanceId) -> RecoveryContext {
        self.recovery_contexts
            .entry(instance_id.clone())
            .or_insert_with(|| RecoveryContext::new(instance_id.clone()))
            .clone()
    }

    /// Evaluate a health assessment and determine recovery action.
    #[instrument(skip(self, assessment), fields(instance_id = %assessment.instance_id))]
    pub fn evaluate_recovery(&self, assessment: &HealthAssessment) -> RecoveryAction {
        let instance_id = &assessment.instance_id;
        let context = self.recovery_context(instance_id);

        // Check if we've exceeded max attempts
        if context.exceeded_max_attempts(self.config.max_recovery_attempts) {
            warn!(
                instance_id = %instance_id,
                attempts = context.attempt_count,
                "Max recovery attempts exceeded"
            );
            return RecoveryAction::Notify {
                severity: NotifySeverity::Critical,
                message: format!(
                    "Instance {} exceeded {} recovery attempts",
                    instance_id, self.config.max_recovery_attempts
                ),
            };
        }

        // Check cooldown
        if !context.cooldown_elapsed(self.config.recovery_delay) {
            debug!(
                instance_id = %instance_id,
                "Recovery cooldown not elapsed"
            );
            return RecoveryAction::None;
        }

        // Determine action based on health and platform
        self.determine_action(assessment, &context)
    }

    /// Determine the appropriate recovery action.
    fn determine_action(
        &self,
        assessment: &HealthAssessment,
        context: &RecoveryContext,
    ) -> RecoveryAction {
        match assessment.overall {
            OverallHealth::Healthy => RecoveryAction::None,

            OverallHealth::Degraded => {
                // Degraded: gentle intervention
                if assessment.should_isolate() {
                    RecoveryAction::Isolate {
                        reason: "Health degraded with declining trend".to_string(),
                    }
                } else {
                    RecoveryAction::None
                }
            }

            OverallHealth::Unhealthy => {
                // Unhealthy: determine severity and action
                self.determine_unhealthy_action(assessment, context)
            }

            OverallHealth::Unknown => {
                // Unknown: wait for more data, but warn if persistent
                if context.attempt_count > 0 {
                    RecoveryAction::Notify {
                        severity: NotifySeverity::Warning,
                        message: "Instance health remains unknown".to_string(),
                    }
                } else {
                    RecoveryAction::None
                }
            }
        }
    }

    /// Determine action for unhealthy instance.
    fn determine_unhealthy_action(
        &self,
        assessment: &HealthAssessment,
        context: &RecoveryContext,
    ) -> RecoveryAction {
        // Platform-specific policies
        match self.platform {
            PlatformProfile::IBank => {
                // IBank: maximum accountability, always require approval
                if self.config.require_human_approval {
                    return RecoveryAction::Notify {
                        severity: NotifySeverity::Error,
                        message: format!(
                            "Instance {} unhealthy, human approval required for recovery",
                            assessment.instance_id
                        ),
                    };
                }
            }
            PlatformProfile::Finalverse => {
                // Finalverse: prioritize safety over speed
                if assessment.should_isolate() {
                    return RecoveryAction::Isolate {
                        reason: "Unhealthy with safety concern".to_string(),
                    };
                }
            }
            _ => {}
        }

        // Escalating actions based on attempt count
        match context.attempt_count {
            0 => {
                // First attempt: graceful restart
                RecoveryAction::RestartGraceful {
                    drain_timeout_secs: self.config.drain_timeout.as_secs(),
                }
            }
            1 => {
                // Second attempt: try again with shorter timeout
                RecoveryAction::RestartGraceful {
                    drain_timeout_secs: self.config.drain_timeout.as_secs() / 2,
                }
            }
            2 => {
                // Third attempt: force restart
                RecoveryAction::RestartForce
            }
            _ => {
                // Beyond that: replace the instance
                RecoveryAction::Replace {
                    keep_old_until_healthy: true,
                }
            }
        }
    }

    /// Execute a recovery action.
    #[instrument(skip(self), fields(instance_id = %instance_id, action = %action))]
    pub async fn execute_recovery(
        &self,
        instance_id: &InstanceId,
        action: RecoveryAction,
    ) -> HealthResult<RecoveryOutcome> {
        // Skip if no action needed
        if matches!(action, RecoveryAction::None) {
            return Ok(RecoveryOutcome::success(
                instance_id.clone(),
                action,
                Utc::now(),
            ));
        }

        // Check policy gate
        if !self
            .policy_gate
            .allow_action(instance_id, &action, self.platform)
            .await?
        {
            info!(
                instance_id = %instance_id,
                action = %action,
                "Recovery action blocked by policy"
            );
            return Ok(RecoveryOutcome::failure(
                instance_id.clone(),
                action,
                Utc::now(),
                "Action blocked by policy",
            ));
        }

        // Check auto-recovery setting
        if !self.config.auto_recovery_enabled && !matches!(action, RecoveryAction::Notify { .. }) {
            info!(
                instance_id = %instance_id,
                action = %action,
                "Auto-recovery disabled, skipping action"
            );
            return Ok(RecoveryOutcome::failure(
                instance_id.clone(),
                action,
                Utc::now(),
                "Auto-recovery disabled",
            ));
        }

        let started_at = Utc::now();

        // Update recovery context
        if let Some(mut ctx) = self.recovery_contexts.get_mut(instance_id) {
            ctx.record_attempt(action.clone());
        }

        info!(
            instance_id = %instance_id,
            action = %action,
            "Executing recovery action"
        );

        // Execute via executor
        let result = self.executor.execute(instance_id, &action).await;

        match result {
            Ok(()) => {
                // Record success on circuit breaker
                if let Some(breaker) = self.circuit_breakers.get(instance_id) {
                    breaker.record_success();
                }

                Ok(RecoveryOutcome::success(
                    instance_id.clone(),
                    action,
                    started_at,
                ))
            }
            Err(e) => {
                // Record failure on circuit breaker
                if let Some(breaker) = self.circuit_breakers.get(instance_id) {
                    breaker.record_failure();
                }

                warn!(
                    instance_id = %instance_id,
                    error = %e,
                    "Recovery action failed"
                );

                Ok(RecoveryOutcome::failure(
                    instance_id.clone(),
                    action,
                    started_at,
                    e.to_string(),
                ))
            }
        }
    }

    /// Reset recovery state for an instance.
    pub fn reset_instance(&self, instance_id: &InstanceId) {
        self.recovery_contexts.remove(instance_id);
        if let Some(breaker) = self.circuit_breakers.get(instance_id) {
            breaker.reset();
        }
    }

    /// Remove an instance from tracking.
    pub fn remove_instance(&self, instance_id: &InstanceId) {
        self.recovery_contexts.remove(instance_id);
        self.circuit_breakers.remove(instance_id);
    }
}

/// Trait for executing recovery actions.
#[async_trait]
pub trait RecoveryExecutor: Send + Sync {
    /// Execute a recovery action.
    async fn execute(&self, instance_id: &InstanceId, action: &RecoveryAction) -> HealthResult<()>;
}

/// Trait for policy-based approval of recovery actions.
#[async_trait]
pub trait RecoveryPolicyGate: Send + Sync {
    /// Check if an action should be allowed.
    async fn allow_action(
        &self,
        instance_id: &InstanceId,
        action: &RecoveryAction,
        platform: PlatformProfile,
    ) -> HealthResult<bool>;
}

/// Default policy gate that allows all actions.
pub struct DefaultRecoveryPolicyGate;

#[async_trait]
impl RecoveryPolicyGate for DefaultRecoveryPolicyGate {
    async fn allow_action(
        &self,
        _instance_id: &InstanceId,
        _action: &RecoveryAction,
        _platform: PlatformProfile,
    ) -> HealthResult<bool> {
        Ok(true)
    }
}

/// No-op recovery executor for testing.
pub struct NoOpRecoveryExecutor;

#[async_trait]
impl RecoveryExecutor for NoOpRecoveryExecutor {
    async fn execute(
        &self,
        _instance_id: &InstanceId,
        action: &RecoveryAction,
    ) -> HealthResult<()> {
        debug!(action = %action, "No-op executing recovery action");
        Ok(())
    }
}

/// Recovery executor that fails specific actions.
pub struct FailingRecoveryExecutor {
    fail_actions: Vec<std::mem::Discriminant<RecoveryAction>>,
}

impl FailingRecoveryExecutor {
    /// Create an executor that fails specific action types.
    pub fn new(fail_actions: Vec<RecoveryAction>) -> Self {
        Self {
            fail_actions: fail_actions
                .into_iter()
                .map(|a| std::mem::discriminant(&a))
                .collect(),
        }
    }
}

#[async_trait]
impl RecoveryExecutor for FailingRecoveryExecutor {
    async fn execute(&self, instance_id: &InstanceId, action: &RecoveryAction) -> HealthResult<()> {
        if self.fail_actions.contains(&std::mem::discriminant(action)) {
            Err(HealthError::RecoveryFailed {
                instance_id: instance_id.clone(),
                reason: format!("Simulated failure for action: {}", action),
            })
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ResilienceConfig {
        ResilienceConfig {
            max_recovery_attempts: 3,
            recovery_delay: std::time::Duration::from_millis(10),
            auto_recovery_enabled: true,
            require_human_approval: false,
            circuit_breaker: crate::config::CircuitBreakerConfig::default(),
            enable_isolation: true,
            drain_timeout: std::time::Duration::from_secs(30),
        }
    }

    #[test]
    fn test_evaluate_recovery_healthy() {
        let executor = Arc::new(NoOpRecoveryExecutor);
        let controller =
            ResilienceController::new(test_config(), PlatformProfile::Development, executor);

        let mut assessment = HealthAssessment::new(InstanceId::generate());
        assessment.overall = OverallHealth::Healthy;

        let action = controller.evaluate_recovery(&assessment);
        assert!(matches!(action, RecoveryAction::None));
    }

    #[test]
    fn test_evaluate_recovery_unhealthy() {
        let executor = Arc::new(NoOpRecoveryExecutor);
        let controller =
            ResilienceController::new(test_config(), PlatformProfile::Development, executor);

        let mut assessment = HealthAssessment::new(InstanceId::generate());
        assessment.overall = OverallHealth::Unhealthy;

        let action = controller.evaluate_recovery(&assessment);
        assert!(matches!(action, RecoveryAction::RestartGraceful { .. }));
    }

    #[tokio::test]
    async fn test_execute_recovery() {
        let executor = Arc::new(NoOpRecoveryExecutor);
        let controller =
            ResilienceController::new(test_config(), PlatformProfile::Development, executor);

        let instance_id = InstanceId::generate();
        let action = RecoveryAction::RestartGraceful {
            drain_timeout_secs: 30,
        };

        let outcome = controller
            .execute_recovery(&instance_id, action)
            .await
            .unwrap();
        assert!(outcome.success);
    }
}
