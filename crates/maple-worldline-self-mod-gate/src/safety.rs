//! Safety invariants for self-modification (I.REGEN-1 through I.REGEN-7).
//!
//! These invariants define the non-negotiable safety boundaries of
//! self-modification. They are derived from the Resonance Constitution's
//! self-regeneration principles.
//!
//! | ID        | Name                         | Enforcement                          |
//! |-----------|------------------------------|--------------------------------------|
//! | I.REGEN-1 | Non-Destruction              | Rollback mechanism not affected       |
//! | I.REGEN-2 | Resonance Invariants         | No invariant violations               |
//! | I.REGEN-3 | Gate Integrity               | Gate code not weakened                |
//! | I.REGEN-4 | Observability                | Full provenance chain                 |
//! | I.REGEN-5 | Bounded Scope                | Bounded affected components           |
//! | I.REGEN-6 | Rate Limiting Preserved      | Rate limiter not affected             |
//! | I.REGEN-7 | Human Override Available      | Emergency stop functioning            |

use crate::commitment::SelfModificationCommitment;

// ── Safety Result ──────────────────────────────────────────────────────

/// Result of a safety invariant check.
#[derive(Clone, Debug)]
pub struct SafetyResult {
    /// Whether the invariant holds.
    pub passed: bool,
    /// Name of the invariant (e.g. "I.REGEN-1").
    pub invariant_name: String,
    /// Details about the check.
    pub details: String,
}

impl SafetyResult {
    /// Create a passing result.
    pub fn pass(name: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            passed: true,
            invariant_name: name.into(),
            details: details.into(),
        }
    }

    /// Create a failing result.
    pub fn fail(name: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            passed: false,
            invariant_name: name.into(),
            details: details.into(),
        }
    }
}

// ── Safety Invariants ──────────────────────────────────────────────────

/// Maximum number of affected components (for I.REGEN-5).
const MAX_BOUNDED_COMPONENTS: usize = 10;

/// Patterns that indicate rollback-related code.
const ROLLBACK_PATTERNS: &[&str] = &["rollback", "revert", "undo", "restore"];

/// Patterns that indicate gate-related code.
const GATE_PATTERNS: &[&str] = &["gate", "adjudication", "commitment_gate", "self_mod_gate"];

/// Patterns that indicate rate-limiter code.
const RATE_LIMITER_PATTERNS: &[&str] = &["rate_limiter", "rate-limiter", "rate_limit"];

/// Patterns that indicate emergency stop / human override code.
const EMERGENCY_PATTERNS: &[&str] = &["emergency", "human_override", "kill_switch", "circuit_breaker"];

/// Patterns that indicate resonance invariant code.
const INVARIANT_PATTERNS: &[&str] = &["safety", "invariant", "consent", "coercion"];

/// Self-modification safety invariants.
///
/// Stateless checker that validates all 7 invariants against a
/// `SelfModificationCommitment`. These invariants are non-negotiable:
/// if any fails, the commitment must be denied.
pub struct SelfModificationSafetyInvariants;

impl SelfModificationSafetyInvariants {
    /// I.REGEN-1: Non-Destruction — rollback mechanism must not be affected.
    ///
    /// Self-regeneration must not destroy rollback capability or safety
    /// systems. The rollback mechanism itself must not be in the affected
    /// file list.
    pub fn check_rollback_integrity(commitment: &SelfModificationCommitment) -> SafetyResult {
        let affected = commitment.affected_files();
        for file in &affected {
            let lower = file.to_lowercase();
            for pattern in ROLLBACK_PATTERNS {
                if lower.contains(pattern) {
                    return SafetyResult::fail(
                        "I.REGEN-1",
                        format!(
                            "File '{}' matches rollback pattern '{}' — rollback mechanism at risk",
                            file, pattern,
                        ),
                    );
                }
            }
        }
        SafetyResult::pass("I.REGEN-1", "Rollback mechanism not in affected files")
    }

    /// I.REGEN-2: Resonance Invariants — no generated code may violate
    /// any of the RA invariants.
    pub fn check_resonance_invariants(commitment: &SelfModificationCommitment) -> SafetyResult {
        let affected = commitment.affected_files();
        for file in &affected {
            let lower = file.to_lowercase();
            for pattern in INVARIANT_PATTERNS {
                if lower.contains(pattern) {
                    return SafetyResult::fail(
                        "I.REGEN-2",
                        format!(
                            "File '{}' matches invariant pattern '{}' — resonance invariant at risk",
                            file, pattern,
                        ),
                    );
                }
            }
        }
        SafetyResult::pass("I.REGEN-2", "No resonance invariant files affected")
    }

    /// I.REGEN-3: Gate Integrity — the Commitment Gate must not be
    /// weakened by self-modification.
    pub fn check_gate_integrity(commitment: &SelfModificationCommitment) -> SafetyResult {
        let affected = commitment.affected_files();
        for file in &affected {
            let lower = file.to_lowercase();
            for pattern in GATE_PATTERNS {
                if lower.contains(pattern) {
                    return SafetyResult::fail(
                        "I.REGEN-3",
                        format!(
                            "File '{}' matches gate pattern '{}' — gate integrity at risk",
                            file, pattern,
                        ),
                    );
                }
            }
        }
        SafetyResult::pass("I.REGEN-3", "Gate code not in affected files")
    }

    /// I.REGEN-4: Observability — every self-modification must have
    /// full provenance from observations through meanings to intent.
    pub fn check_observability(commitment: &SelfModificationCommitment) -> SafetyResult {
        if !commitment.intent_chain.has_full_provenance() {
            return SafetyResult::fail(
                "I.REGEN-4",
                "Incomplete provenance chain — observations and meanings required",
            );
        }
        SafetyResult::pass(
            "I.REGEN-4",
            format!(
                "Full provenance chain with {} links",
                commitment.intent_chain.chain_length(),
            ),
        )
    }

    /// I.REGEN-5: Bounded Scope — each commitment must have bounded
    /// scope (limited affected components).
    pub fn check_bounded_scope(commitment: &SelfModificationCommitment) -> SafetyResult {
        let components = commitment.affected_components();
        if components.is_empty() {
            return SafetyResult::fail(
                "I.REGEN-5",
                "No affected components declared — scope is undefined",
            );
        }
        if components.len() > MAX_BOUNDED_COMPONENTS {
            return SafetyResult::fail(
                "I.REGEN-5",
                format!(
                    "Too many affected components: {} exceeds max {}",
                    components.len(),
                    MAX_BOUNDED_COMPONENTS,
                ),
            );
        }
        SafetyResult::pass(
            "I.REGEN-5",
            format!("{} components (within bound)", components.len()),
        )
    }

    /// I.REGEN-6: Rate Limiting Preserved — the rate limiter itself
    /// must not be modified by self-modification.
    pub fn check_rate_limiting_preserved(commitment: &SelfModificationCommitment) -> SafetyResult {
        let affected = commitment.affected_files();
        for file in &affected {
            let lower = file.to_lowercase();
            for pattern in RATE_LIMITER_PATTERNS {
                if lower.contains(pattern) {
                    return SafetyResult::fail(
                        "I.REGEN-6",
                        format!(
                            "File '{}' matches rate limiter pattern '{}' — rate limiting at risk",
                            file, pattern,
                        ),
                    );
                }
            }
        }
        SafetyResult::pass("I.REGEN-6", "Rate limiter not in affected files")
    }

    /// I.REGEN-7: Human Override Available — emergency stop / human
    /// override mechanism must not be modified.
    pub fn check_human_override_available(commitment: &SelfModificationCommitment) -> SafetyResult {
        let affected = commitment.affected_files();
        for file in &affected {
            let lower = file.to_lowercase();
            for pattern in EMERGENCY_PATTERNS {
                if lower.contains(pattern) {
                    return SafetyResult::fail(
                        "I.REGEN-7",
                        format!(
                            "File '{}' matches emergency pattern '{}' — human override at risk",
                            file, pattern,
                        ),
                    );
                }
            }
        }
        SafetyResult::pass("I.REGEN-7", "Human override mechanism not affected")
    }

    /// Run all 7 safety invariants and return all results.
    pub fn check_all(commitment: &SelfModificationCommitment) -> Vec<SafetyResult> {
        vec![
            Self::check_rollback_integrity(commitment),
            Self::check_resonance_invariants(commitment),
            Self::check_gate_integrity(commitment),
            Self::check_observability(commitment),
            Self::check_bounded_scope(commitment),
            Self::check_rate_limiting_preserved(commitment),
            Self::check_human_override_available(commitment),
        ]
    }

    /// Whether all safety invariants pass.
    pub fn all_pass(commitment: &SelfModificationCommitment) -> bool {
        Self::check_all(commitment).iter().all(|r| r.passed)
    }

    /// Return only the failing invariants.
    pub fn failures(commitment: &SelfModificationCommitment) -> Vec<SafetyResult> {
        Self::check_all(commitment)
            .into_iter()
            .filter(|r| !r.passed)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commitment::{IntentChain, SelfModificationCommitment};
    use crate::types::{DeploymentStrategy, SelfModTier};
    use maple_worldline_intent::intent::ImprovementEstimate;
    use maple_worldline_intent::proposal::*;
    use maple_worldline_intent::types::{CodeChangeType, IntentId, MeaningId, ProposalId};

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

        SelfModificationCommitment::new(
            RegenerationProposal {
                id: ProposalId::new(),
                summary: "Test".into(),
                rationale: "Testing".into(),
                affected_components: components.iter().map(|s| s.to_string()).collect(),
                code_changes: changes,
                required_tests: vec![TestSpec {
                    name: "t".into(),
                    description: "t".into(),
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
                    steps: vec!["revert".into()],
                    estimated_duration_secs: 60,
                },
            },
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
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

    fn make_commitment_no_provenance(files: Vec<&str>) -> SelfModificationCommitment {
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

        SelfModificationCommitment::new(
            RegenerationProposal {
                id: ProposalId::new(),
                summary: "Test".into(),
                rationale: "Testing".into(),
                affected_components: vec!["module".into()],
                code_changes: changes,
                required_tests: vec![TestSpec {
                    name: "t".into(),
                    description: "t".into(),
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
                    steps: vec!["revert".into()],
                    estimated_duration_secs: 60,
                },
            },
            SelfModTier::Tier0Configuration,
            DeploymentStrategy::Immediate,
            RollbackPlan {
                strategy: RollbackStrategy::GitRevert,
                steps: vec!["git revert HEAD".into()],
                estimated_duration_secs: 60,
            },
            IntentChain {
                observation_ids: vec![], // Empty — no provenance!
                meaning_ids: vec![],
                intent_id: IntentId::new(),
            },
        )
        .unwrap()
    }

    #[test]
    fn regen1_rollback_integrity_passes_normal_files() {
        let c = make_commitment(vec!["src/config.rs"], vec!["config"]);
        let result = SelfModificationSafetyInvariants::check_rollback_integrity(&c);
        assert!(result.passed);
        assert_eq!(result.invariant_name, "I.REGEN-1");
    }

    #[test]
    fn regen1_rollback_integrity_fails_rollback_files() {
        let c = make_commitment(vec!["src/rollback/handler.rs"], vec!["rollback"]);
        let result = SelfModificationSafetyInvariants::check_rollback_integrity(&c);
        assert!(!result.passed);
        assert!(result.details.contains("rollback"));
    }

    #[test]
    fn regen3_gate_integrity_fails_gate_files() {
        let c = make_commitment(vec!["src/adjudication.rs"], vec!["gate"]);
        let result = SelfModificationSafetyInvariants::check_gate_integrity(&c);
        assert!(!result.passed);
        assert_eq!(result.invariant_name, "I.REGEN-3");
    }

    #[test]
    fn regen4_observability_fails_no_provenance() {
        let c = make_commitment_no_provenance(vec!["src/config.rs"]);
        let result = SelfModificationSafetyInvariants::check_observability(&c);
        assert!(!result.passed);
        assert_eq!(result.invariant_name, "I.REGEN-4");
    }

    #[test]
    fn regen5_bounded_scope_passes_within_limit() {
        let c = make_commitment(vec!["src/a.rs"], vec!["module-a"]);
        let result = SelfModificationSafetyInvariants::check_bounded_scope(&c);
        assert!(result.passed);
    }

    #[test]
    fn regen6_rate_limiting_preserved_fails() {
        let c = make_commitment(vec!["src/rate_limiter.rs"], vec!["limiter"]);
        let result = SelfModificationSafetyInvariants::check_rate_limiting_preserved(&c);
        assert!(!result.passed);
        assert_eq!(result.invariant_name, "I.REGEN-6");
    }

    #[test]
    fn regen7_human_override_fails_emergency_files() {
        let c = make_commitment(vec!["src/emergency_stop.rs"], vec!["emergency"]);
        let result = SelfModificationSafetyInvariants::check_human_override_available(&c);
        assert!(!result.passed);
        assert_eq!(result.invariant_name, "I.REGEN-7");
    }

    #[test]
    fn check_all_passes_for_safe_commitment() {
        let c = make_commitment(vec!["src/config.rs"], vec!["config"]);
        let results = SelfModificationSafetyInvariants::check_all(&c);
        assert_eq!(results.len(), 7);
        assert!(results.iter().all(|r| r.passed));
        assert!(SelfModificationSafetyInvariants::all_pass(&c));
    }

    #[test]
    fn check_all_detects_multiple_violations() {
        // This file touches both gate AND safety patterns
        let c = make_commitment(vec!["src/safety_gate.rs"], vec!["safety"]);
        let failures = SelfModificationSafetyInvariants::failures(&c);
        assert!(failures.len() >= 2); // At least I.REGEN-2 (safety) and I.REGEN-3 (gate)
        assert!(!SelfModificationSafetyInvariants::all_pass(&c));
    }
}
