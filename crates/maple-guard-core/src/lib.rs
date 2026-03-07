//! MAPLE Guard Core — policy-as-code engine with declarative rule language.
//!
//! Provides the foundational policy engine that all guard domains plug into.
//! Policies are DENY-FIRST: if any rule denies, the action is blocked.

pub mod engine;
pub mod policy;

pub use engine::{
    EvaluationContext, EvaluationResult, GuardDecision, GuardError, PolicyEngine,
    ResolvedAction, RuleEvaluationResult,
};
pub use policy::{
    EnforcementLevel, Policy, PolicyDomain, PolicyId, PolicyMetadata, PolicyRule,
    RuleAction, RuleCondition,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn make_deny_policy(domain: PolicyDomain, condition: RuleCondition) -> Policy {
        Policy {
            id: PolicyId("test-deny".to_string()),
            name: "Test Deny".to_string(),
            version: semver::Version::new(1, 0, 0),
            description: "Test deny policy".to_string(),
            domain,
            enforcement: EnforcementLevel::Mandatory,
            rules: vec![PolicyRule {
                id: "rule-1".to_string(),
                name: "Deny Rule".to_string(),
                condition,
                action: RuleAction::Deny {
                    reason: "blocked by test policy".to_string(),
                    code: None,
                },
                priority: 100,
                enabled: true,
            }],
            metadata: PolicyMetadata::empty(),
        }
    }

    #[test]
    fn test_empty_engine_allows_all() {
        let engine = PolicyEngine::new();
        let ctx = EvaluationContext::new().with_tool("file.read");
        let result = engine.evaluate(&PolicyDomain::ToolExecution, &ctx);
        assert_eq!(result.decision, GuardDecision::Allow);
    }

    #[test]
    fn test_always_deny_blocks() {
        let mut engine = PolicyEngine::new();
        engine
            .load_policy(make_deny_policy(
                PolicyDomain::ToolExecution,
                RuleCondition::Always,
            ))
            .unwrap();
        let ctx = EvaluationContext::new().with_tool("file.write");
        let result = engine.evaluate(&PolicyDomain::ToolExecution, &ctx);
        assert!(matches!(result.decision, GuardDecision::Deny { .. }));
    }

    #[test]
    fn test_tool_match_deny() {
        let mut engine = PolicyEngine::new();
        engine
            .load_policy(make_deny_policy(
                PolicyDomain::ToolExecution,
                RuleCondition::ToolMatch {
                    patterns: vec!["banking.*".to_string()],
                },
            ))
            .unwrap();

        // Should deny banking.transfer
        let ctx = EvaluationContext::new().with_tool("banking.transfer");
        let result = engine.evaluate(&PolicyDomain::ToolExecution, &ctx);
        assert!(matches!(result.decision, GuardDecision::Deny { .. }));

        // Should allow file.read
        let ctx2 = EvaluationContext::new().with_tool("file.read");
        let result2 = engine.evaluate(&PolicyDomain::ToolExecution, &ctx2);
        assert_eq!(result2.decision, GuardDecision::Allow);
    }

    #[test]
    fn test_domain_isolation() {
        let mut engine = PolicyEngine::new();
        engine
            .load_policy(make_deny_policy(
                PolicyDomain::Inference,
                RuleCondition::Always,
            ))
            .unwrap();

        // Inference domain should be denied
        let ctx = EvaluationContext::new();
        let result = engine.evaluate(&PolicyDomain::Inference, &ctx);
        assert!(matches!(result.decision, GuardDecision::Deny { .. }));

        // ToolExecution domain should be allowed (no policies loaded)
        let result2 = engine.evaluate(&PolicyDomain::ToolExecution, &ctx);
        assert_eq!(result2.decision, GuardDecision::Allow);
    }

    #[test]
    fn test_risk_threshold() {
        let mut engine = PolicyEngine::new();
        engine
            .load_policy(make_deny_policy(
                PolicyDomain::Global,
                RuleCondition::RiskThreshold { min_score: 0.8 },
            ))
            .unwrap();

        // High risk should deny
        let ctx_high = EvaluationContext::new().with_risk_score(0.9);
        let result = engine.evaluate(&PolicyDomain::ToolExecution, &ctx_high);
        assert!(matches!(result.decision, GuardDecision::Deny { .. }));

        // Low risk should allow
        let ctx_low = EvaluationContext::new().with_risk_score(0.3);
        let result2 = engine.evaluate(&PolicyDomain::ToolExecution, &ctx_low);
        assert_eq!(result2.decision, GuardDecision::Allow);
    }

    #[test]
    fn test_load_and_list_policies() {
        let mut engine = PolicyEngine::new();
        assert_eq!(engine.total_policy_count(), 0);

        engine
            .load_policy(make_deny_policy(
                PolicyDomain::ToolExecution,
                RuleCondition::Always,
            ))
            .unwrap();
        assert_eq!(engine.total_policy_count(), 1);
        assert_eq!(engine.policy_count(&PolicyDomain::ToolExecution), 1);
        assert_eq!(engine.policy_count(&PolicyDomain::Inference), 0);
    }

    #[test]
    fn test_remove_policy() {
        let mut engine = PolicyEngine::new();
        engine
            .load_policy(make_deny_policy(
                PolicyDomain::ToolExecution,
                RuleCondition::Always,
            ))
            .unwrap();
        assert_eq!(engine.total_policy_count(), 1);

        engine.remove_policy(&PolicyId("test-deny".to_string()));
        assert_eq!(engine.total_policy_count(), 0);
    }

    #[test]
    fn test_advisory_does_not_block() {
        let mut engine = PolicyEngine::new();
        let mut policy = make_deny_policy(PolicyDomain::ToolExecution, RuleCondition::Always);
        policy.enforcement = EnforcementLevel::Advisory;
        engine.load_policy(policy).unwrap();

        let ctx = EvaluationContext::new().with_tool("file.delete");
        let result = engine.evaluate(&PolicyDomain::ToolExecution, &ctx);
        // Advisory should not deny
        assert_eq!(result.decision, GuardDecision::Allow);
    }

    #[test]
    fn test_glob_match_utility() {
        assert!(engine::glob_match("file.*", "file.read"));
        assert!(engine::glob_match("file.*", "file.write"));
        assert!(!engine::glob_match("file.*", "net.connect"));
        assert!(engine::glob_match("*", "anything"));
    }

    #[test]
    fn test_never_condition_does_not_match() {
        let mut engine = PolicyEngine::new();
        engine
            .load_policy(make_deny_policy(
                PolicyDomain::ToolExecution,
                RuleCondition::Never,
            ))
            .unwrap();
        let ctx = EvaluationContext::new().with_tool("file.write");
        let result = engine.evaluate(&PolicyDomain::ToolExecution, &ctx);
        assert_eq!(result.decision, GuardDecision::Allow);
    }
}
