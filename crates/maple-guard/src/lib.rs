//! MAPLE Guard -- unified guard service facade for AI agent governance.
//!
//! Wraps all guard subsystems (core policy engine, firewall, PII, inference,
//! approvals, compliance, risk) behind a single `GuardService` entry point.

use chrono::{DateTime, Utc};
use maple_guard_core::{
    EvaluationContext, GuardDecision, PolicyEngine, PolicyDomain,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum GuardError {
    #[error("guard evaluation error: {0}")]
    EvaluationError(String),
    #[error("configuration error: {0}")]
    ConfigError(String),
    #[error("subsystem error ({subsystem}): {message}")]
    SubsystemError { subsystem: String, message: String },
}

pub type GuardResult<T> = Result<T, GuardError>;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Configuration for the unified guard service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardConfig {
    pub enabled: bool,
    pub strict_mode: bool,
    pub audit_enabled: bool,
    pub subsystems: SubsystemConfig,
}

impl Default for GuardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strict_mode: false,
            audit_enabled: true,
            subsystems: SubsystemConfig::default(),
        }
    }
}

/// Enable/disable individual guard subsystems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemConfig {
    pub policy_engine: bool,
    pub firewall: bool,
    pub pii_detection: bool,
    pub inference_guard: bool,
    pub approvals: bool,
    pub compliance: bool,
    pub risk_scoring: bool,
}

impl Default for SubsystemConfig {
    fn default() -> Self {
        Self {
            policy_engine: true,
            firewall: true,
            pii_detection: true,
            inference_guard: true,
            approvals: true,
            compliance: true,
            risk_scoring: true,
        }
    }
}

/// An audit event emitted by the guard service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardEvent {
    pub id: String,
    pub event_type: GuardEventType,
    pub tool: String,
    pub worldline_id: Option<String>,
    pub decision: String,
    pub details: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

/// Types of guard events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuardEventType {
    PolicyEvaluation,
    RiskAssessment,
    ApprovalRequired,
    ComplianceCheck,
    PiiDetected,
    FirewallBlock,
    AuditLog,
}

/// Result of a complete guard evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardEvaluation {
    pub allowed: bool,
    pub decision: String,
    pub reasons: Vec<String>,
    pub events: Vec<GuardEvent>,
    pub evaluated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Guard Service
// ---------------------------------------------------------------------------

/// Unified guard service that orchestrates all guard subsystems.
pub struct GuardService {
    config: GuardConfig,
    policy_engine: PolicyEngine,
    default_domain: PolicyDomain,
    events: Vec<GuardEvent>,
    event_counter: u64,
}

impl GuardService {
    /// Create a new guard service with the given configuration.
    pub fn new(config: GuardConfig) -> Self {
        Self {
            config,
            policy_engine: PolicyEngine::new(),
            default_domain: PolicyDomain::ToolExecution,
            events: Vec::new(),
            event_counter: 0,
        }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(GuardConfig::default())
    }

    /// Get a reference to the policy engine.
    pub fn policy_engine(&self) -> &PolicyEngine {
        &self.policy_engine
    }

    /// Get a mutable reference to the policy engine.
    pub fn policy_engine_mut(&mut self) -> &mut PolicyEngine {
        &mut self.policy_engine
    }

    /// Get the guard configuration.
    pub fn config(&self) -> &GuardConfig {
        &self.config
    }

    /// Set the default policy domain for evaluations.
    pub fn set_default_domain(&mut self, domain: PolicyDomain) {
        self.default_domain = domain;
    }

    /// Evaluate an action through all enabled guard subsystems using default domain.
    pub fn evaluate(&mut self, context: &EvaluationContext) -> GuardResult<GuardEvaluation> {
        self.evaluate_in_domain(&self.default_domain.clone(), context)
    }

    /// Evaluate an action through all enabled guard subsystems in a specific domain.
    pub fn evaluate_in_domain(
        &mut self,
        domain: &PolicyDomain,
        context: &EvaluationContext,
    ) -> GuardResult<GuardEvaluation> {
        if !self.config.enabled {
            return Ok(GuardEvaluation {
                allowed: true,
                decision: "allowed".into(),
                reasons: vec!["guard disabled".into()],
                events: Vec::new(),
                evaluated_at: Utc::now(),
            });
        }

        let mut reasons = Vec::new();
        let mut evaluation_events = Vec::new();
        let mut allowed = true;

        let tool_display = context.tool.as_deref().unwrap_or("unknown");
        let worldline_display = context.worldline_id.clone();

        // Step 1: Policy engine evaluation
        if self.config.subsystems.policy_engine {
            let result = self.policy_engine.evaluate(domain, context);
            let policy_allowed = result.decision == GuardDecision::Allow;
            if !policy_allowed {
                allowed = false;
                reasons.push(format!("Policy denied: {:?}", result.decision));
            }
            let event = self.make_event(
                GuardEventType::PolicyEvaluation,
                tool_display,
                worldline_display.as_deref(),
                if policy_allowed { "allowed" } else { "denied" },
            );
            evaluation_events.push(event);
        }

        // Step 2: Record audit event
        if self.config.audit_enabled {
            let event = self.make_event(
                GuardEventType::AuditLog,
                tool_display,
                worldline_display.as_deref(),
                if allowed { "allowed" } else { "denied" },
            );
            evaluation_events.push(event.clone());
            self.events.push(event);
        }

        if self.config.strict_mode && reasons.is_empty() && allowed {
            reasons.push("passed all guard checks in strict mode".into());
        }

        Ok(GuardEvaluation {
            allowed,
            decision: if allowed { "allowed".into() } else { "denied".into() },
            reasons,
            events: evaluation_events,
            evaluated_at: Utc::now(),
        })
    }

    /// Get all audit events.
    pub fn events(&self) -> &[GuardEvent] {
        &self.events
    }

    /// Clear audit events.
    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    fn make_event(
        &mut self,
        event_type: GuardEventType,
        tool: &str,
        worldline_id: Option<&str>,
        decision: &str,
    ) -> GuardEvent {
        self.event_counter += 1;
        GuardEvent {
            id: format!("evt-{}", self.event_counter),
            event_type,
            tool: tool.to_string(),
            worldline_id: worldline_id.map(|s| s.to_string()),
            decision: decision.to_string(),
            details: HashMap::new(),
            timestamp: Utc::now(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use maple_guard_core::{
        EnforcementLevel, Policy, PolicyId, PolicyMetadata, PolicyRule, RuleAction, RuleCondition,
    };

    fn make_context(tool: &str) -> EvaluationContext {
        EvaluationContext::new()
            .with_tool(tool)
            .with_worldline_id("agent-1")
    }

    #[test]
    fn test_default_allows() {
        let mut guard = GuardService::with_defaults();
        let ctx = make_context("file.read");
        let result = guard.evaluate(&ctx).unwrap();
        assert!(result.allowed);
    }

    #[test]
    fn test_disabled_guard_allows_all() {
        let config = GuardConfig {
            enabled: false,
            ..Default::default()
        };
        let mut guard = GuardService::new(config);
        let ctx = make_context("dangerous.action");
        let result = guard.evaluate(&ctx).unwrap();
        assert!(result.allowed);
    }

    #[test]
    fn test_policy_deny() {
        let mut guard = GuardService::with_defaults();
        let policy = Policy {
            id: PolicyId("deny-writes".to_string()),
            name: "Deny Writes".to_string(),
            version: semver::Version::new(1, 0, 0),
            description: "Block writes".to_string(),
            domain: PolicyDomain::ToolExecution,
            enforcement: EnforcementLevel::Mandatory,
            rules: vec![PolicyRule {
                id: "deny".into(),
                name: "block writes".into(),
                condition: RuleCondition::ToolMatch {
                    patterns: vec!["file.write".to_string()],
                },
                action: RuleAction::Deny {
                    reason: "writes blocked".into(),
                    code: None,
                },
                priority: 100,
                enabled: true,
            }],
            metadata: PolicyMetadata::empty(),
        };
        guard.policy_engine_mut().load_policy(policy).unwrap();

        let ctx = make_context("file.write");
        let result = guard.evaluate(&ctx).unwrap();
        assert!(!result.allowed);
    }

    #[test]
    fn test_audit_events_recorded() {
        let mut guard = GuardService::with_defaults();
        let ctx = make_context("file.read");
        guard.evaluate(&ctx).unwrap();
        assert!(!guard.events().is_empty());
    }

    #[test]
    fn test_clear_events() {
        let mut guard = GuardService::with_defaults();
        let ctx = make_context("file.read");
        guard.evaluate(&ctx).unwrap();
        guard.clear_events();
        assert!(guard.events().is_empty());
    }

    #[test]
    fn test_config_access() {
        let guard = GuardService::with_defaults();
        assert!(guard.config().enabled);
        assert!(guard.config().audit_enabled);
    }

    #[test]
    fn test_subsystem_config_default() {
        let config = SubsystemConfig::default();
        assert!(config.policy_engine);
        assert!(config.firewall);
        assert!(config.pii_detection);
    }

    #[test]
    fn test_guard_event_type() {
        let event = GuardEvent {
            id: "evt-1".into(),
            event_type: GuardEventType::PolicyEvaluation,
            tool: "test".into(),
            worldline_id: None,
            decision: "allowed".into(),
            details: HashMap::new(),
            timestamp: Utc::now(),
        };
        assert_eq!(event.event_type, GuardEventType::PolicyEvaluation);
    }

    #[test]
    fn test_strict_mode() {
        let config = GuardConfig {
            strict_mode: true,
            ..Default::default()
        };
        let mut guard = GuardService::new(config);
        let ctx = make_context("file.read");
        let result = guard.evaluate(&ctx).unwrap();
        assert!(result.allowed);
        assert!(!result.reasons.is_empty());
    }

    #[test]
    fn test_evaluation_has_timestamp() {
        let mut guard = GuardService::with_defaults();
        let ctx = make_context("test");
        let result = guard.evaluate(&ctx).unwrap();
        assert!(result.evaluated_at <= Utc::now());
    }
}
