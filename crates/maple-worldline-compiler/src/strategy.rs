//! Compilation strategy selection.
//!
//! Defines `PassId` (the 11 optimization passes), `CompilationStrategy`
//! (a named set of enabled passes for a target), and the `StrategySelector`
//! trait for choosing strategies based on module characteristics.

use serde::{Deserialize, Serialize};

use crate::types::{CompilationTarget, CompilerConfig, OptimizationLevel, StrategyId};
use maple_worldline_ir::WlirModule;

// ── Pass Identifier ──────────────────────────────────────────────────

/// Identifier for an optimization pass.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PassId {
    // ── Standard Passes ──
    ConstantFolding,
    DeadCodeElimination,
    CommonSubexpressionElimination,
    InliningPass,
    TailCallOptimization,
    // ── EVOS-Specific Passes ──
    CommitmentBatching,
    ProvenanceCoalescing,
    EventDeduplication,
    OperatorDispatchSpecialization,
    MemoryTierPromotion,
    SafetyFenceMinimization,
}

impl PassId {
    /// Whether this is a standard (non-EVOS) pass.
    pub fn is_standard(&self) -> bool {
        matches!(
            self,
            Self::ConstantFolding
                | Self::DeadCodeElimination
                | Self::CommonSubexpressionElimination
                | Self::InliningPass
                | Self::TailCallOptimization
        )
    }

    /// Whether this is an EVOS-specific pass.
    pub fn is_evos(&self) -> bool {
        !self.is_standard()
    }

    /// All 5 standard passes.
    pub fn standard_passes() -> Vec<PassId> {
        vec![
            Self::ConstantFolding,
            Self::DeadCodeElimination,
            Self::CommonSubexpressionElimination,
            Self::InliningPass,
            Self::TailCallOptimization,
        ]
    }

    /// All 6 EVOS-specific passes.
    pub fn evos_passes() -> Vec<PassId> {
        vec![
            Self::CommitmentBatching,
            Self::ProvenanceCoalescing,
            Self::EventDeduplication,
            Self::OperatorDispatchSpecialization,
            Self::MemoryTierPromotion,
            Self::SafetyFenceMinimization,
        ]
    }

    /// All 11 passes.
    pub fn all_passes() -> Vec<PassId> {
        let mut all = Self::standard_passes();
        all.extend(Self::evos_passes());
        all
    }
}

impl std::fmt::Display for PassId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConstantFolding => write!(f, "constant-folding"),
            Self::DeadCodeElimination => write!(f, "dead-code-elimination"),
            Self::CommonSubexpressionElimination => write!(f, "cse"),
            Self::InliningPass => write!(f, "inlining"),
            Self::TailCallOptimization => write!(f, "tail-call-opt"),
            Self::CommitmentBatching => write!(f, "commitment-batching"),
            Self::ProvenanceCoalescing => write!(f, "provenance-coalescing"),
            Self::EventDeduplication => write!(f, "event-deduplication"),
            Self::OperatorDispatchSpecialization => write!(f, "operator-dispatch-specialization"),
            Self::MemoryTierPromotion => write!(f, "memory-tier-promotion"),
            Self::SafetyFenceMinimization => write!(f, "safety-fence-minimization"),
        }
    }
}

// ── Compilation Strategy ─────────────────────────────────────────────

/// A named compilation strategy with enabled passes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompilationStrategy {
    pub id: StrategyId,
    pub name: String,
    pub target: CompilationTarget,
    pub optimization_level: OptimizationLevel,
    pub enabled_passes: Vec<PassId>,
    pub description: String,
}

impl CompilationStrategy {
    /// Create a new strategy.
    pub fn new(name: &str, target: CompilationTarget, level: OptimizationLevel) -> Self {
        let enabled_passes = match &level {
            OptimizationLevel::None => vec![],
            OptimizationLevel::Basic => PassId::standard_passes(),
            OptimizationLevel::Aggressive => PassId::all_passes(),
        };

        Self {
            id: StrategyId::from_name(name),
            name: name.into(),
            target,
            optimization_level: level,
            enabled_passes,
            description: format!("Strategy '{}'", name),
        }
    }

    /// Enable an additional pass.
    pub fn enable_pass(&mut self, pass: PassId) {
        if !self.enabled_passes.contains(&pass) {
            self.enabled_passes.push(pass);
        }
    }

    /// Disable a pass.
    pub fn disable_pass(&mut self, pass: &PassId) {
        self.enabled_passes.retain(|p| p != pass);
    }

    /// Whether this strategy includes a specific pass.
    pub fn has_pass(&self, pass: &PassId) -> bool {
        self.enabled_passes.contains(pass)
    }
}

// ── Strategy Selector ────────────────────────────────────────────────

/// Trait for selecting a compilation strategy based on module and config.
pub trait StrategySelector: Send + Sync {
    /// Select a strategy for the given module and configuration.
    fn select_strategy(&self, module: &WlirModule, config: &CompilerConfig) -> CompilationStrategy;

    /// Name of this selector implementation.
    fn name(&self) -> &str;
}

/// Simulated strategy selector for deterministic testing.
pub struct SimulatedStrategySelector;

impl SimulatedStrategySelector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimulatedStrategySelector {
    fn default() -> Self {
        Self::new()
    }
}

impl StrategySelector for SimulatedStrategySelector {
    fn select_strategy(&self, module: &WlirModule, config: &CompilerConfig) -> CompilationStrategy {
        let name = format!("{}-{}", config.target, config.optimization_level);
        let mut strategy = CompilationStrategy::new(
            &name,
            config.target.clone(),
            config.optimization_level.clone(),
        );

        // If module has commitment boundaries, enable commitment batching
        // even at Basic level
        let has_commitments = !module.commitment_declarations.is_empty();
        if has_commitments && !strategy.has_pass(&PassId::CommitmentBatching) {
            strategy.enable_pass(PassId::CommitmentBatching);
        }

        strategy.description = format!(
            "Auto-selected strategy for {} ({} functions, {} instructions)",
            module.name,
            module.functions.len(),
            module.total_instructions(),
        );

        strategy
    }

    fn name(&self) -> &str {
        "simulated-strategy-selector"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TargetArch;

    #[test]
    fn strategy_creation_none() {
        let s = CompilationStrategy::new(
            "test",
            CompilationTarget::Interpreted,
            OptimizationLevel::None,
        );
        assert!(s.enabled_passes.is_empty());
    }

    #[test]
    fn strategy_creation_basic() {
        let s = CompilationStrategy::new(
            "basic",
            CompilationTarget::Native {
                arch: TargetArch::X86_64,
            },
            OptimizationLevel::Basic,
        );
        assert_eq!(s.enabled_passes.len(), 5);
        assert!(s.enabled_passes.iter().all(|p| p.is_standard()));
    }

    #[test]
    fn strategy_creation_aggressive() {
        let s = CompilationStrategy::new(
            "aggro",
            CompilationTarget::Native {
                arch: TargetArch::X86_64,
            },
            OptimizationLevel::Aggressive,
        );
        assert_eq!(s.enabled_passes.len(), 11);
    }

    #[test]
    fn strategy_enable_disable_pass() {
        let mut s = CompilationStrategy::new(
            "test",
            CompilationTarget::Interpreted,
            OptimizationLevel::None,
        );
        assert!(!s.has_pass(&PassId::ConstantFolding));
        s.enable_pass(PassId::ConstantFolding);
        assert!(s.has_pass(&PassId::ConstantFolding));
        s.disable_pass(&PassId::ConstantFolding);
        assert!(!s.has_pass(&PassId::ConstantFolding));
    }

    #[test]
    fn pass_id_display() {
        assert_eq!(PassId::ConstantFolding.to_string(), "constant-folding");
        assert_eq!(
            PassId::CommitmentBatching.to_string(),
            "commitment-batching"
        );
        assert_eq!(
            PassId::SafetyFenceMinimization.to_string(),
            "safety-fence-minimization"
        );
    }

    #[test]
    fn pass_id_standard_vs_evos() {
        assert!(PassId::ConstantFolding.is_standard());
        assert!(!PassId::ConstantFolding.is_evos());
        assert!(PassId::CommitmentBatching.is_evos());
        assert!(!PassId::CommitmentBatching.is_standard());
    }

    #[test]
    fn all_passes_covered() {
        assert_eq!(PassId::standard_passes().len(), 5);
        assert_eq!(PassId::evos_passes().len(), 6);
        assert_eq!(PassId::all_passes().len(), 11);
    }

    #[test]
    fn simulated_selector_selects_strategy() {
        use maple_worldline_ir::{WlirFunction, WlirModule, WlirType};
        let selector = SimulatedStrategySelector::new();
        let mut module = WlirModule::new("test", "1.0");
        let f = WlirFunction::new("main", vec![], WlirType::Void);
        module.add_function(f);
        let config = CompilerConfig::default();
        let strategy = selector.select_strategy(&module, &config);
        assert!(!strategy.name.is_empty());
        assert_eq!(strategy.target, config.target);
    }
}
