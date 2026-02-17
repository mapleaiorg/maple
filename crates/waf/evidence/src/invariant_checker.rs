use crate::types::InvariantResult;
use async_trait::async_trait;

/// All 14 invariants (8 seed + 6 WAF).
pub const ALL_INVARIANT_IDS: &[&str] = &[
    "I.1", "I.2", "I.3", "I.4", "I.5", "I.6", "I.7", "I.8",
    "I.WAF-1", "I.WAF-2", "I.WAF-3", "I.WAF-4", "I.WAF-5", "I.WAF-6",
];

/// Descriptions for the 14 invariants.
pub fn invariant_description(id: &str) -> &'static str {
    match id {
        "I.1" => "Identity Persistence: WorldLine ID unique and immutable",
        "I.2" => "Causal Provenance: Every state change signed and linked to parent",
        "I.3" => "Axiomatic Primacy: Evolution never violates core axioms",
        "I.4" => "Resonance Minimum: System halts if R < R_min",
        "I.5" => "Commitment Gating: No side effects without committed commitment",
        "I.6" => "State Isolation: Evolution logic cannot mutate persistence plane",
        "I.7" => "Evidence Requirement: Self-upgrades require valid EvidenceBundle",
        "I.8" => "Human Agency Override: System remains couplable to human intent",
        "I.WAF-1" => "Context Graph Integrity: Every WLL node content-addressed + causally linked",
        "I.WAF-2" => "Synthesis Traceability: Every code delta traceable to intent",
        "I.WAF-3" => "Swap Atomicity: Logic swap is atomic; no partial upgrades",
        "I.WAF-4" => "Rollback Guarantee: System can always revert to last stable state",
        "I.WAF-5" => "Evidence Completeness: No swap without satisfying EvidenceBundle",
        "I.WAF-6" => "Resonance Monotonicity: Evolution must not decrease resonance below threshold",
        _ => "Unknown invariant",
    }
}

/// Trait for checking invariants.
#[async_trait]
pub trait InvariantChecker: Send + Sync {
    /// Check all invariants and return results.
    async fn check_all(&self) -> Vec<InvariantResult>;

    /// Check a specific invariant by ID.
    async fn check(&self, invariant_id: &str) -> InvariantResult;
}

/// Simulated invariant checker for testing.
pub struct SimulatedInvariantChecker {
    /// Which invariants should pass (by ID). If empty, all pass.
    failing_invariants: Vec<String>,
}

impl SimulatedInvariantChecker {
    /// Create a checker where all invariants pass.
    pub fn all_pass() -> Self {
        Self {
            failing_invariants: Vec::new(),
        }
    }

    /// Create a checker where specific invariants fail.
    pub fn with_failures(failing: Vec<String>) -> Self {
        Self {
            failing_invariants: failing,
        }
    }

    fn check_invariant(&self, id: &str) -> InvariantResult {
        let holds = !self.failing_invariants.iter().any(|f| f == id);
        InvariantResult {
            id: id.to_string(),
            description: invariant_description(id).to_string(),
            holds,
            details: if holds {
                "verified".into()
            } else {
                format!("invariant {} violated in simulation", id)
            },
        }
    }
}

#[async_trait]
impl InvariantChecker for SimulatedInvariantChecker {
    async fn check_all(&self) -> Vec<InvariantResult> {
        ALL_INVARIANT_IDS
            .iter()
            .map(|id| self.check_invariant(id))
            .collect()
    }

    async fn check(&self, invariant_id: &str) -> InvariantResult {
        self.check_invariant(invariant_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_invariant_ids_count() {
        assert_eq!(ALL_INVARIANT_IDS.len(), 14);
    }

    #[test]
    fn invariant_descriptions() {
        for id in ALL_INVARIANT_IDS {
            let desc = invariant_description(id);
            assert!(!desc.contains("Unknown"), "Missing description for {}", id);
        }
    }

    #[tokio::test]
    async fn simulated_all_pass() {
        let checker = SimulatedInvariantChecker::all_pass();
        let results = checker.check_all().await;
        assert_eq!(results.len(), 14);
        assert!(results.iter().all(|r| r.holds));
    }

    #[tokio::test]
    async fn simulated_with_failures() {
        let checker =
            SimulatedInvariantChecker::with_failures(vec!["I.WAF-1".into(), "I.4".into()]);
        let results = checker.check_all().await;
        assert_eq!(results.len(), 14);
        let failures: Vec<_> = results.iter().filter(|r| !r.holds).collect();
        assert_eq!(failures.len(), 2);
        assert!(failures.iter().any(|r| r.id == "I.WAF-1"));
        assert!(failures.iter().any(|r| r.id == "I.4"));
    }

    #[tokio::test]
    async fn check_specific_invariant() {
        let checker = SimulatedInvariantChecker::all_pass();
        let result = checker.check("I.WAF-1").await;
        assert!(result.holds);
        assert!(result.description.contains("Context Graph"));
    }

    #[tokio::test]
    async fn check_specific_failing() {
        let checker = SimulatedInvariantChecker::with_failures(vec!["I.3".into()]);
        let result = checker.check("I.3").await;
        assert!(!result.holds);
        assert!(result.details.contains("violated"));
    }
}
