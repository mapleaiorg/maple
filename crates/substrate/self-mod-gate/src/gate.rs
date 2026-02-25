//! Self-modification gate — the commitment boundary.
//!
//! The `SelfModificationGate` orchestrates self-modification-specific checks
//! before delegating to the adjudication pipeline. Each check implements the
//! `SelfModCheck` trait and can be mandatory (failure = denial) or advisory.
//!
//! Built-in checks:
//! - `ProposalCompleteness` — proposal has changes, tests, and rollback
//! - `RollbackViability` — rollback plan is reasonable
//! - `SafetyInvariantPreservation` — safety systems not in affected files
//! - `GateIntegrityProtection` — gate code not in affected files
//! - `BoundedScope` — affected components explicitly listed and bounded
//! - `InvariantPreservation` — resonance architecture invariants preserved

use crate::commitment::SelfModificationCommitment;
use crate::rate_limiter::RegenerationRateLimiter;
use crate::types::{ApprovalRequirements, SelfModTier};

use std::collections::HashMap;

// ── Check Result ────────────────────────────────────────────────────────

/// Result of a single self-modification check.
#[derive(Clone, Debug)]
pub struct SelfModCheckResult {
    /// Whether the check passed.
    pub passed: bool,
    /// Name of the check.
    pub check_name: String,
    /// Details or reason for pass/fail.
    pub details: String,
}

impl SelfModCheckResult {
    /// Create a passing result.
    pub fn pass(name: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            passed: true,
            check_name: name.into(),
            details: details.into(),
        }
    }

    /// Create a failing result.
    pub fn fail(name: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            passed: false,
            check_name: name.into(),
            details: details.into(),
        }
    }
}

// ── Check Trait ─────────────────────────────────────────────────────────

/// Trait for self-modification checks.
///
/// Each check evaluates a `SelfModificationCommitment` and returns
/// a pass/fail result. Mandatory checks block approval on failure.
pub trait SelfModCheck: Send + Sync {
    /// Name of this check.
    fn name(&self) -> &str;

    /// Execute the check against a commitment.
    fn check(&self, commitment: &SelfModificationCommitment) -> SelfModCheckResult;

    /// Whether this check is mandatory (failure = denial).
    fn mandatory(&self) -> bool;
}

// ── Built-in Checks ─────────────────────────────────────────────────────

/// Checks that the proposal has code changes, tests, and a rollback plan.
pub struct ProposalCompleteness;

impl SelfModCheck for ProposalCompleteness {
    fn name(&self) -> &str {
        "proposal-completeness"
    }

    fn check(&self, commitment: &SelfModificationCommitment) -> SelfModCheckResult {
        if commitment.proposal.code_changes.is_empty() {
            return SelfModCheckResult::fail(self.name(), "Proposal has no code changes");
        }
        if commitment.proposal.required_tests.is_empty() {
            return SelfModCheckResult::fail(self.name(), "Proposal has no required tests");
        }
        if commitment.rollback_plan.steps.is_empty() {
            return SelfModCheckResult::fail(self.name(), "No rollback steps defined");
        }
        SelfModCheckResult::pass(
            self.name(),
            format!(
                "{} changes, {} tests, {} rollback steps",
                commitment.proposal.code_changes.len(),
                commitment.proposal.required_tests.len(),
                commitment.rollback_plan.steps.len(),
            ),
        )
    }

    fn mandatory(&self) -> bool {
        true
    }
}

/// Checks that the rollback plan is viable and has reasonable duration.
pub struct RollbackViability;

impl SelfModCheck for RollbackViability {
    fn name(&self) -> &str {
        "rollback-viability"
    }

    fn check(&self, commitment: &SelfModificationCommitment) -> SelfModCheckResult {
        let plan = &commitment.rollback_plan;
        if plan.steps.is_empty() {
            return SelfModCheckResult::fail(self.name(), "No rollback steps");
        }
        // Rollback should not take longer than the deployment itself
        if plan.estimated_duration_secs > commitment.max_deployment_duration_secs {
            return SelfModCheckResult::fail(
                self.name(),
                format!(
                    "Rollback duration ({}s) exceeds max deployment duration ({}s)",
                    plan.estimated_duration_secs, commitment.max_deployment_duration_secs,
                ),
            );
        }
        SelfModCheckResult::pass(
            self.name(),
            format!(
                "Rollback via {} in ~{}s",
                plan.strategy, plan.estimated_duration_secs,
            ),
        )
    }

    fn mandatory(&self) -> bool {
        true
    }
}

/// Checks that safety-critical files are not in the affected file list.
///
/// Enforces I.REGEN-1 (Non-Destruction): self-regeneration must not
/// destroy rollback capability or safety systems.
pub struct SafetyInvariantPreservation;

/// Files that are considered safety-critical and should not be modified
/// without explicit governance approval.
const SAFETY_CRITICAL_PATTERNS: &[&str] = &[
    "safety",
    "rollback",
    "emergency",
    "invariant",
    "consent",
    "coercion",
];

impl SelfModCheck for SafetyInvariantPreservation {
    fn name(&self) -> &str {
        "safety-invariant-preservation"
    }

    fn check(&self, commitment: &SelfModificationCommitment) -> SelfModCheckResult {
        let affected = commitment.affected_files();
        for file in &affected {
            let lower = file.to_lowercase();
            for pattern in SAFETY_CRITICAL_PATTERNS {
                if lower.contains(pattern) {
                    return SelfModCheckResult::fail(
                        self.name(),
                        format!(
                            "File '{}' matches safety-critical pattern '{}'",
                            file, pattern,
                        ),
                    );
                }
            }
        }
        SelfModCheckResult::pass(self.name(), "No safety-critical files affected")
    }

    fn mandatory(&self) -> bool {
        true
    }
}

/// Checks that gate code is not in the affected file list.
///
/// Enforces I.REGEN-3 (Gate Integrity): the Commitment Gate must not
/// be weakened by self-modification.
pub struct GateIntegrityProtection;

const GATE_CRITICAL_PATTERNS: &[&str] =
    &["gate", "adjudication", "commitment_gate", "self_mod_gate"];

impl SelfModCheck for GateIntegrityProtection {
    fn name(&self) -> &str {
        "gate-integrity-protection"
    }

    fn check(&self, commitment: &SelfModificationCommitment) -> SelfModCheckResult {
        let affected = commitment.affected_files();
        for file in &affected {
            let lower = file.to_lowercase();
            for pattern in GATE_CRITICAL_PATTERNS {
                if lower.contains(pattern) {
                    return SelfModCheckResult::fail(
                        self.name(),
                        format!(
                            "File '{}' matches gate-critical pattern '{}' — gate integrity at risk",
                            file, pattern,
                        ),
                    );
                }
            }
        }
        SelfModCheckResult::pass(self.name(), "Gate code not affected")
    }

    fn mandatory(&self) -> bool {
        true
    }
}

/// Checks that affected components are explicitly listed and bounded.
///
/// Enforces I.REGEN-5 (Bounded Scope): each commitment must have
/// bounded scope.
pub struct BoundedScope;

/// Maximum number of components that can be affected in a single commitment.
const MAX_AFFECTED_COMPONENTS: usize = 10;

impl SelfModCheck for BoundedScope {
    fn name(&self) -> &str {
        "bounded-scope"
    }

    fn check(&self, commitment: &SelfModificationCommitment) -> SelfModCheckResult {
        let components = commitment.affected_components();
        if components.is_empty() {
            return SelfModCheckResult::fail(self.name(), "No affected components listed");
        }
        if components.len() > MAX_AFFECTED_COMPONENTS {
            return SelfModCheckResult::fail(
                self.name(),
                format!(
                    "Too many affected components ({} > max {})",
                    components.len(),
                    MAX_AFFECTED_COMPONENTS,
                ),
            );
        }
        SelfModCheckResult::pass(
            self.name(),
            format!("{} components affected (within bound)", components.len(),),
        )
    }

    fn mandatory(&self) -> bool {
        true
    }
}

/// Checks that resonance architecture invariants are preserved.
///
/// Enforces I.REGEN-2 (Invariant Preservation): no generated code may
/// violate any of the RA invariants.
pub struct InvariantPreservation;

const INVARIANT_CRITICAL_PATTERNS: &[&str] = &["rate_limiter", "rate-limiter", "emergency_stop"];

impl SelfModCheck for InvariantPreservation {
    fn name(&self) -> &str {
        "invariant-preservation"
    }

    fn check(&self, commitment: &SelfModificationCommitment) -> SelfModCheckResult {
        let affected = commitment.affected_files();
        for file in &affected {
            let lower = file.to_lowercase();
            for pattern in INVARIANT_CRITICAL_PATTERNS {
                if lower.contains(pattern) {
                    return SelfModCheckResult::fail(
                        self.name(),
                        format!(
                            "File '{}' contains invariant-critical pattern '{}'",
                            file, pattern,
                        ),
                    );
                }
            }
        }
        SelfModCheckResult::pass(self.name(), "RA invariants preserved")
    }

    fn mandatory(&self) -> bool {
        true
    }
}

// ── Gate ─────────────────────────────────────────────────────────────────

/// The self-modification commitment gate.
///
/// Orchestrates self-modification checks and delegates to the
/// adjudication pipeline for tier-based approval decisions.
pub struct SelfModificationGate {
    /// Self-modification checks to run.
    pub(crate) checks: Vec<Box<dyn SelfModCheck>>,
    /// Rate limiter for modification frequency control.
    pub(crate) rate_limiter: RegenerationRateLimiter,
    /// Per-tier approval requirements.
    pub(crate) tier_requirements: HashMap<SelfModTier, ApprovalRequirements>,
}

impl SelfModificationGate {
    /// Create a new gate with default checks and rate limiter.
    pub fn new() -> Self {
        Self {
            checks: Self::default_checks(),
            rate_limiter: RegenerationRateLimiter::new(),
            tier_requirements: HashMap::new(),
        }
    }

    /// Create with a custom rate limiter.
    pub fn with_rate_limiter(mut self, rate_limiter: RegenerationRateLimiter) -> Self {
        self.rate_limiter = rate_limiter;
        self
    }

    /// Add a custom check.
    pub fn add_check(&mut self, check: Box<dyn SelfModCheck>) {
        self.checks.push(check);
    }

    /// Set tier-specific approval requirements.
    pub fn set_tier_requirements(&mut self, tier: SelfModTier, requirements: ApprovalRequirements) {
        self.tier_requirements.insert(tier, requirements);
    }

    /// Get tier-specific approval requirements, if set.
    pub fn tier_requirements(&self, tier: &SelfModTier) -> Option<&ApprovalRequirements> {
        self.tier_requirements.get(tier)
    }

    /// Default set of built-in checks.
    pub fn default_checks() -> Vec<Box<dyn SelfModCheck>> {
        vec![
            Box::new(ProposalCompleteness),
            Box::new(RollbackViability),
            Box::new(SafetyInvariantPreservation),
            Box::new(GateIntegrityProtection),
            Box::new(BoundedScope),
            Box::new(InvariantPreservation),
        ]
    }

    /// Run all checks against a commitment and return results.
    pub fn run_checks(
        &self,
        commitment: &SelfModificationCommitment,
    ) -> Vec<(SelfModCheckResult, bool)> {
        self.checks
            .iter()
            .map(|check| {
                let result = check.check(commitment);
                let mandatory = check.mandatory();
                (result, mandatory)
            })
            .collect()
    }
}

impl Default for SelfModificationGate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commitment::IntentChain;
    use maple_worldline_intent::intent::ImprovementEstimate;
    use maple_worldline_intent::proposal::*;
    use maple_worldline_intent::types::MeaningId;
    use maple_worldline_intent::types::{CodeChangeType, IntentId, ProposalId};

    fn make_commitment(files: Vec<&str>, components: Vec<&str>) -> SelfModificationCommitment {
        let changes: Vec<CodeChangeSpec> = files
            .iter()
            .map(|f| CodeChangeSpec {
                file_path: f.to_string(),
                change_type: CodeChangeType::ModifyFunction {
                    function_name: "test".into(),
                },
                description: "test change".into(),
                affected_regions: vec![],
                provenance: vec![MeaningId::new()],
            })
            .collect();

        let proposal = RegenerationProposal {
            id: ProposalId::new(),
            summary: "Test proposal".into(),
            rationale: "Testing".into(),
            affected_components: components.iter().map(|s| s.to_string()).collect(),
            code_changes: changes,
            required_tests: vec![TestSpec {
                name: "test_it".into(),
                description: "Test".into(),
                test_type: TestType::Unit,
            }],
            performance_gates: vec![],
            safety_checks: vec![],
            estimated_improvement: ImprovementEstimate {
                metric: "speed".into(),
                current_value: 10.0,
                projected_value: 8.0,
                confidence: 0.9,
                unit: "ms".into(),
            },
            risk_score: 0.1,
            rollback_plan: RollbackPlan {
                strategy: RollbackStrategy::GitRevert,
                steps: vec!["git revert HEAD".into()],
                estimated_duration_secs: 60,
            },
        };

        SelfModificationCommitment::new(
            proposal,
            SelfModTier::Tier0Configuration,
            crate::types::DeploymentStrategy::Immediate,
            RollbackPlan {
                strategy: RollbackStrategy::GitRevert,
                steps: vec!["git revert HEAD".into()],
                estimated_duration_secs: 60,
            },
            IntentChain {
                observation_ids: vec!["obs-1".into()],
                meaning_ids: vec![MeaningId::new()],
                intent_id: IntentId::new(),
            },
        )
        .unwrap()
    }

    #[test]
    fn proposal_completeness_passes() {
        let c = make_commitment(vec!["src/config.rs"], vec!["config"]);
        let check = ProposalCompleteness;
        let result = check.check(&c);
        assert!(result.passed);
    }

    #[test]
    fn safety_invariant_blocks_safety_files() {
        let c = make_commitment(vec!["src/safety/handler.rs"], vec!["safety"]);
        let check = SafetyInvariantPreservation;
        let result = check.check(&c);
        assert!(!result.passed);
        assert!(result.details.contains("safety"));
    }

    #[test]
    fn gate_integrity_blocks_gate_files() {
        let c = make_commitment(vec!["src/commitment_gate.rs"], vec!["gate"]);
        let check = GateIntegrityProtection;
        let result = check.check(&c);
        assert!(!result.passed);
    }

    #[test]
    fn gate_integrity_passes_normal_files() {
        let c = make_commitment(vec!["src/config.rs"], vec!["config"]);
        let check = GateIntegrityProtection;
        let result = check.check(&c);
        assert!(result.passed);
    }

    #[test]
    fn bounded_scope_passes_within_limit() {
        let c = make_commitment(vec!["src/a.rs"], vec!["module-a"]);
        let check = BoundedScope;
        let result = check.check(&c);
        assert!(result.passed);
    }

    #[test]
    fn bounded_scope_fails_too_many_components() {
        let components: Vec<&str> = (0..11).map(|_| "component").collect();
        let c = make_commitment(vec!["src/a.rs"], components);
        let check = BoundedScope;
        let result = check.check(&c);
        assert!(!result.passed);
    }

    #[test]
    fn default_gate_has_six_checks() {
        let gate = SelfModificationGate::new();
        assert_eq!(gate.checks.len(), 6);
    }

    #[test]
    fn gate_runs_all_checks() {
        let gate = SelfModificationGate::new();
        let c = make_commitment(vec!["src/config.rs"], vec!["config"]);
        let results = gate.run_checks(&c);
        assert_eq!(results.len(), 6);
        // All should pass for a normal commitment
        assert!(results.iter().all(|(r, _)| r.passed));
    }
}
