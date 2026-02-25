//! Strategy evolution for the adaptive compiler.
//!
//! Analyzes profiling data and proposes changes to compilation strategies.
//! `StrategyEvolver` trait + `SimulatedEvolver` implementation.

use crate::strategy::{CompilationStrategy, PassId};
use crate::types::ProfilingData;

// ── Strategy Proposal ───────────────────────────────────────────────

/// A proposed modification to a compilation strategy.
#[derive(Clone, Debug)]
pub struct StrategyProposal {
    /// Human-readable reason for the proposal.
    pub reason: String,
    /// Passes to enable.
    pub enable_passes: Vec<PassId>,
    /// Passes to disable.
    pub disable_passes: Vec<PassId>,
    /// Confidence level (0.0–1.0).
    pub confidence: f64,
}

impl StrategyProposal {
    /// Whether this proposal makes any changes.
    pub fn has_changes(&self) -> bool {
        !self.enable_passes.is_empty() || !self.disable_passes.is_empty()
    }

    /// Apply this proposal to a strategy, producing a new strategy.
    pub fn apply_to(&self, strategy: &CompilationStrategy) -> CompilationStrategy {
        let mut evolved = strategy.clone();
        for pass in &self.enable_passes {
            evolved.enable_pass(pass.clone());
        }
        for pass in &self.disable_passes {
            evolved.disable_pass(pass);
        }
        evolved.description = format!("{} (evolved: {})", evolved.description, self.reason);
        evolved
    }
}

impl std::fmt::Display for StrategyProposal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Proposal(confidence={:.2}, enable={}, disable={}, reason={})",
            self.confidence,
            self.enable_passes.len(),
            self.disable_passes.len(),
            self.reason,
        )
    }
}

// ── Strategy Evolver Trait ──────────────────────────────────────────

/// Trait for evolving compilation strategies based on profiling data.
pub trait StrategyEvolver: Send + Sync {
    /// Analyze profiling data and propose strategy modifications.
    fn propose(&self, current: &CompilationStrategy, profiling: &ProfilingData)
        -> StrategyProposal;

    /// Name of this evolver implementation.
    fn name(&self) -> &str;
}

// ── Simulated Evolver ───────────────────────────────────────────────

/// Simulated strategy evolver for deterministic testing.
///
/// Evolution rules:
/// - High memory tier usage → enable MemoryTierPromotion
/// - Frequent operator calls → enable OperatorDispatchSpecialization
/// - Many commitment boundary crossings → enable CommitmentBatching
/// - Many hot functions → enable InliningPass
pub struct SimulatedEvolver;

impl SimulatedEvolver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimulatedEvolver {
    fn default() -> Self {
        Self::new()
    }
}

impl StrategyEvolver for SimulatedEvolver {
    fn propose(
        &self,
        current: &CompilationStrategy,
        profiling: &ProfilingData,
    ) -> StrategyProposal {
        let mut enable = Vec::new();
        let mut reasons = Vec::new();

        // High memory usage → MemoryTierPromotion
        let total_memory_ops: u64 = profiling.memory_tier_usage.values().sum();
        if total_memory_ops > 100 && !current.has_pass(&PassId::MemoryTierPromotion) {
            enable.push(PassId::MemoryTierPromotion);
            reasons.push("high memory usage");
        }

        // Frequent operator calls → OperatorDispatchSpecialization
        let total_operator_calls: u64 = profiling.operator_call_frequency.values().sum();
        if total_operator_calls > 50 && !current.has_pass(&PassId::OperatorDispatchSpecialization) {
            enable.push(PassId::OperatorDispatchSpecialization);
            reasons.push("frequent operator calls");
        }

        // Many commitment crossings → CommitmentBatching
        if profiling.commitment_boundary_crossings > 20
            && !current.has_pass(&PassId::CommitmentBatching)
        {
            enable.push(PassId::CommitmentBatching);
            reasons.push("many commitment crossings");
        }

        // Many hot functions → InliningPass
        if profiling.hot_functions.len() > 5 && !current.has_pass(&PassId::InliningPass) {
            enable.push(PassId::InliningPass);
            reasons.push("many hot functions");
        }

        let confidence = if enable.is_empty() {
            0.0
        } else {
            0.7 + (enable.len() as f64 * 0.05).min(0.25)
        };

        let reason = if reasons.is_empty() {
            "no changes recommended".to_string()
        } else {
            reasons.join(", ")
        };

        StrategyProposal {
            reason,
            enable_passes: enable,
            disable_passes: Vec::new(),
            confidence,
        }
    }

    fn name(&self) -> &str {
        "simulated-strategy-evolver"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CompilationTarget, OptimizationLevel, TargetArch};

    fn base_strategy() -> CompilationStrategy {
        CompilationStrategy::new(
            "base",
            CompilationTarget::Native {
                arch: TargetArch::X86_64,
            },
            OptimizationLevel::Basic,
        )
    }

    #[test]
    fn no_changes_on_empty_profiling() {
        let evolver = SimulatedEvolver::new();
        let strategy = base_strategy();
        let profiling = ProfilingData::default();
        let proposal = evolver.propose(&strategy, &profiling);
        assert!(!proposal.has_changes());
        assert_eq!(proposal.confidence, 0.0);
    }

    #[test]
    fn high_memory_enables_tier_promotion() {
        let evolver = SimulatedEvolver::new();
        let strategy = base_strategy();
        let mut profiling = ProfilingData::default();
        profiling.memory_tier_usage.insert("episodic".into(), 200);
        let proposal = evolver.propose(&strategy, &profiling);
        assert!(proposal.has_changes());
        assert!(proposal
            .enable_passes
            .contains(&PassId::MemoryTierPromotion));
    }

    #[test]
    fn frequent_operators_enables_specialization() {
        let evolver = SimulatedEvolver::new();
        let strategy = base_strategy();
        let mut profiling = ProfilingData::default();
        profiling
            .operator_call_frequency
            .insert("transfer".into(), 100);
        let proposal = evolver.propose(&strategy, &profiling);
        assert!(proposal
            .enable_passes
            .contains(&PassId::OperatorDispatchSpecialization));
    }

    #[test]
    fn commitment_crossings_enables_batching() {
        let evolver = SimulatedEvolver::new();
        let strategy = base_strategy();
        let mut profiling = ProfilingData::default();
        profiling.commitment_boundary_crossings = 50;
        let proposal = evolver.propose(&strategy, &profiling);
        assert!(proposal.enable_passes.contains(&PassId::CommitmentBatching));
    }

    #[test]
    fn proposal_apply_creates_evolved_strategy() {
        let evolver = SimulatedEvolver::new();
        let strategy = base_strategy();
        let mut profiling = ProfilingData::default();
        profiling.memory_tier_usage.insert("semantic".into(), 500);
        let proposal = evolver.propose(&strategy, &profiling);
        let evolved = proposal.apply_to(&strategy);
        assert!(evolved.has_pass(&PassId::MemoryTierPromotion));
        assert!(evolved.description.contains("evolved"));
    }

    #[test]
    fn evolver_name() {
        let evolver = SimulatedEvolver::new();
        assert_eq!(evolver.name(), "simulated-strategy-evolver");
    }
}
