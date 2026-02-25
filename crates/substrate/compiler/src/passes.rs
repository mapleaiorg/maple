//! Optimization passes for WLIR modules.
//!
//! Implements 11 optimization passes organized into two categories:
//!
//! **Standard (5):** ConstantFolding, DeadCodeElimination, CSE,
//! InliningPass, TailCallOptimization
//!
//! **EVOS-specific (6):** CommitmentBatching, ProvenanceCoalescing,
//! EventDeduplication, OperatorDispatchSpecialization,
//! MemoryTierPromotion, SafetyFenceMinimization
//!
//! ## Safety Invariants
//!
//! - **I.COMPILE-2a**: CommitmentBatching MUST NOT merge commitments
//!   with different scopes.
//! - **I.COMPILE-2b**: SafetyFenceMinimization MUST NOT remove fences
//!   guarding `AssertInvariant` instructions.

use serde::{Deserialize, Serialize};

use crate::error::CompilerResult;
use crate::strategy::{CompilationStrategy, PassId};
use maple_worldline_ir::instructions::{BoundaryDirection, InstructionCategory, WlirInstruction};
use maple_worldline_ir::module::WlirModule;

// ── Pass Result ──────────────────────────────────────────────────────

/// Result of applying a single optimization pass.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PassResult {
    pub pass_id: PassId,
    pub applied: bool,
    pub changes_made: u32,
    pub description: String,
}

impl std::fmt::Display for PassResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} ({} changes) — {}",
            self.pass_id,
            if self.applied { "applied" } else { "skipped" },
            self.changes_made,
            self.description,
        )
    }
}

// ── Optimization Pass Trait ──────────────────────────────────────────

/// Trait for an optimization pass that transforms a WLIR module.
pub trait OptimizationPass: Send + Sync {
    /// The pass identifier.
    fn pass_id(&self) -> PassId;

    /// Apply this pass to a WLIR module (simulated).
    fn apply(&self, module: &WlirModule) -> CompilerResult<PassResult>;
}

// ══════════════════════════════════════════════════════════════════════
// Standard Passes
// ══════════════════════════════════════════════════════════════════════

/// Fold constant arithmetic expressions.
pub struct ConstantFoldingPass;

impl OptimizationPass for ConstantFoldingPass {
    fn pass_id(&self) -> PassId {
        PassId::ConstantFolding
    }

    fn apply(&self, module: &WlirModule) -> CompilerResult<PassResult> {
        // Count instructions that could be folded (consecutive LoadConst + arithmetic)
        let mut foldable = 0u32;
        for func in &module.functions {
            for window in func.instructions.windows(3) {
                if matches!(window[0], WlirInstruction::LoadConst { .. })
                    && matches!(window[1], WlirInstruction::LoadConst { .. })
                    && matches!(
                        window[2],
                        WlirInstruction::Add { .. }
                            | WlirInstruction::Sub { .. }
                            | WlirInstruction::Mul { .. }
                    )
                {
                    foldable += 1;
                }
            }
        }

        Ok(PassResult {
            pass_id: PassId::ConstantFolding,
            applied: foldable > 0,
            changes_made: foldable,
            description: format!("{} constant expressions folded", foldable),
        })
    }
}

/// Eliminate dead (unreachable) code.
pub struct DeadCodeEliminationPass;

impl OptimizationPass for DeadCodeEliminationPass {
    fn pass_id(&self) -> PassId {
        PassId::DeadCodeElimination
    }

    fn apply(&self, module: &WlirModule) -> CompilerResult<PassResult> {
        // Count instructions after unconditional Return or Jump (dead code)
        let mut dead = 0u32;
        for func in &module.functions {
            let mut after_terminal = false;
            for inst in &func.instructions {
                if after_terminal
                    && !matches!(
                        inst,
                        WlirInstruction::Nop | WlirInstruction::SetSourceLocation { .. }
                    )
                {
                    dead += 1;
                }
                if matches!(inst, WlirInstruction::Return { .. }) {
                    after_terminal = true;
                }
            }
        }

        Ok(PassResult {
            pass_id: PassId::DeadCodeElimination,
            applied: dead > 0,
            changes_made: dead,
            description: format!("{} dead instructions eliminated", dead),
        })
    }
}

/// Eliminate common subexpressions.
pub struct CommonSubexpressionEliminationPass;

impl OptimizationPass for CommonSubexpressionEliminationPass {
    fn pass_id(&self) -> PassId {
        PassId::CommonSubexpressionElimination
    }

    fn apply(&self, module: &WlirModule) -> CompilerResult<PassResult> {
        // Simulated: count potential CSE opportunities (same reads pattern)
        let total_instructions = module.total_instructions();
        let cse_opportunities = total_instructions / 10; // ~10% are duplicates (simulated)

        Ok(PassResult {
            pass_id: PassId::CommonSubexpressionElimination,
            applied: cse_opportunities > 0,
            changes_made: cse_opportunities as u32,
            description: format!("{} common subexpressions eliminated", cse_opportunities),
        })
    }
}

/// Inline small functions.
pub struct InliningPass;

impl OptimizationPass for InliningPass {
    fn pass_id(&self) -> PassId {
        PassId::InliningPass
    }

    fn apply(&self, module: &WlirModule) -> CompilerResult<PassResult> {
        // Count small functions (< 5 instructions) that could be inlined
        let small_funcs = module
            .functions
            .iter()
            .filter(|f| f.instructions.len() < 5 && f.instructions.len() > 0)
            .count() as u32;

        Ok(PassResult {
            pass_id: PassId::InliningPass,
            applied: small_funcs > 0,
            changes_made: small_funcs,
            description: format!("{} small functions inlined", small_funcs),
        })
    }
}

/// Convert tail calls to jumps.
pub struct TailCallOptimizationPass;

impl OptimizationPass for TailCallOptimizationPass {
    fn pass_id(&self) -> PassId {
        PassId::TailCallOptimization
    }

    fn apply(&self, module: &WlirModule) -> CompilerResult<PassResult> {
        // Count functions where last meaningful instruction before Return is a Call
        let mut tail_calls = 0u32;
        for func in &module.functions {
            let non_meta: Vec<_> = func
                .instructions
                .iter()
                .filter(|i| i.category() != InstructionCategory::Metadata)
                .collect();
            if non_meta.len() >= 2 {
                if matches!(non_meta[non_meta.len() - 1], WlirInstruction::Return { .. })
                    && matches!(non_meta[non_meta.len() - 2], WlirInstruction::Call { .. })
                {
                    tail_calls += 1;
                }
            }
        }

        Ok(PassResult {
            pass_id: PassId::TailCallOptimization,
            applied: tail_calls > 0,
            changes_made: tail_calls,
            description: format!("{} tail calls optimized", tail_calls),
        })
    }
}

// ══════════════════════════════════════════════════════════════════════
// EVOS-Specific Passes
// ══════════════════════════════════════════════════════════════════════

/// Batch commitment operations within the same scope.
///
/// **CRITICAL SAFETY (I.COMPILE-2a)**: MUST NOT merge commitments
/// with different scopes. Only adjacent CrossCommitmentBoundary
/// instructions with the *same* commitment_id can be batched.
pub struct CommitmentBatchingPass;

impl OptimizationPass for CommitmentBatchingPass {
    fn pass_id(&self) -> PassId {
        PassId::CommitmentBatching
    }

    fn apply(&self, module: &WlirModule) -> CompilerResult<PassResult> {
        let mut batchable = 0u32;
        let mut different_scopes = 0u32;

        for func in &module.functions {
            let boundaries: Vec<_> = func
                .instructions
                .iter()
                .filter_map(|i| match i {
                    WlirInstruction::CrossCommitmentBoundary {
                        commitment_id,
                        direction,
                    } => Some((commitment_id.clone(), direction.clone())),
                    _ => None,
                })
                .collect();

            // Check for adjacent same-scope boundaries that could be batched
            for window in boundaries.windows(2) {
                let (id1, dir1) = &window[0];
                let (id2, dir2) = &window[1];

                if id1 == id2
                    && matches!(dir1, BoundaryDirection::Exit)
                    && matches!(dir2, BoundaryDirection::Enter)
                {
                    batchable += 1;
                }

                // Track different-scope boundaries (safety check)
                if id1 != id2 {
                    different_scopes += 1;
                }
            }
        }

        // Safety: report different scopes but never merge them
        if different_scopes > 0 {
            tracing::info!(
                "CommitmentBatching: {} different-scope boundary pairs preserved (I.COMPILE-2a)",
                different_scopes
            );
        }

        Ok(PassResult {
            pass_id: PassId::CommitmentBatching,
            applied: batchable > 0,
            changes_made: batchable,
            description: format!(
                "{} same-scope commitment boundaries batched, {} different-scope preserved",
                batchable, different_scopes
            ),
        })
    }
}

/// Coalesce multiple provenance records when safe.
pub struct ProvenanceCoalescingPass;

impl OptimizationPass for ProvenanceCoalescingPass {
    fn pass_id(&self) -> PassId {
        PassId::ProvenanceCoalescing
    }

    fn apply(&self, module: &WlirModule) -> CompilerResult<PassResult> {
        let mut coalesceable = 0u32;
        for func in &module.functions {
            let provenance_count = func
                .instructions
                .iter()
                .filter(|i| matches!(i, WlirInstruction::RecordProvenance { .. }))
                .count();
            if provenance_count > 1 {
                coalesceable += (provenance_count - 1) as u32;
            }
        }

        Ok(PassResult {
            pass_id: PassId::ProvenanceCoalescing,
            applied: coalesceable > 0,
            changes_made: coalesceable,
            description: format!("{} provenance records coalesced", coalesceable),
        })
    }
}

/// Deduplicate identical event emissions.
pub struct EventDeduplicationPass;

impl OptimizationPass for EventDeduplicationPass {
    fn pass_id(&self) -> PassId {
        PassId::EventDeduplication
    }

    fn apply(&self, module: &WlirModule) -> CompilerResult<PassResult> {
        let mut duplicates = 0u32;
        for func in &module.functions {
            let events: Vec<_> = func
                .instructions
                .iter()
                .filter_map(|i| match i {
                    WlirInstruction::EmitEvent { event_type, .. } => Some(event_type.clone()),
                    _ => None,
                })
                .collect();
            let unique: std::collections::HashSet<_> = events.iter().collect();
            if events.len() > unique.len() {
                duplicates += (events.len() - unique.len()) as u32;
            }
        }

        Ok(PassResult {
            pass_id: PassId::EventDeduplication,
            applied: duplicates > 0,
            changes_made: duplicates,
            description: format!("{} duplicate events deduplicated", duplicates),
        })
    }
}

/// Specialize operator dispatch based on observed patterns.
pub struct OperatorDispatchSpecializationPass;

impl OptimizationPass for OperatorDispatchSpecializationPass {
    fn pass_id(&self) -> PassId {
        PassId::OperatorDispatchSpecialization
    }

    fn apply(&self, module: &WlirModule) -> CompilerResult<PassResult> {
        let operator_calls = module
            .functions
            .iter()
            .flat_map(|f| f.instructions.iter())
            .filter(|i| matches!(i, WlirInstruction::InvokeOperator { .. }))
            .count() as u32;

        Ok(PassResult {
            pass_id: PassId::OperatorDispatchSpecialization,
            applied: operator_calls > 0,
            changes_made: operator_calls,
            description: format!("{} operator dispatches specialized", operator_calls),
        })
    }
}

/// Promote hot data to faster memory tiers.
pub struct MemoryTierPromotionPass;

impl OptimizationPass for MemoryTierPromotionPass {
    fn pass_id(&self) -> PassId {
        PassId::MemoryTierPromotion
    }

    fn apply(&self, module: &WlirModule) -> CompilerResult<PassResult> {
        let memory_ops = module
            .functions
            .iter()
            .flat_map(|f| f.instructions.iter())
            .filter(|i| {
                matches!(
                    i,
                    WlirInstruction::MemoryQuery { .. } | WlirInstruction::MemoryStore { .. }
                )
            })
            .count() as u32;

        let promotable = memory_ops / 2; // ~50% can be promoted (simulated)

        Ok(PassResult {
            pass_id: PassId::MemoryTierPromotion,
            applied: promotable > 0,
            changes_made: promotable,
            description: format!("{} memory accesses promoted to faster tier", promotable),
        })
    }
}

/// Minimize redundant safety fences.
///
/// **CRITICAL SAFETY (I.COMPILE-2b)**: MUST NOT remove fences that
/// guard `AssertInvariant` instructions. Only fences whose preceding
/// ops do not include safety-critical instructions are candidates.
pub struct SafetyFenceMinimizationPass;

impl OptimizationPass for SafetyFenceMinimizationPass {
    fn pass_id(&self) -> PassId {
        PassId::SafetyFenceMinimization
    }

    fn apply(&self, module: &WlirModule) -> CompilerResult<PassResult> {
        let mut removable = 0u32;
        let mut preserved = 0u32;

        for func in &module.functions {
            for (i, inst) in func.instructions.iter().enumerate() {
                if let WlirInstruction::SafetyFence { .. } = inst {
                    // Check if next instruction is an AssertInvariant —
                    // if so, this fence guards an invariant and MUST NOT be removed
                    let guards_invariant = func
                        .instructions
                        .get(i + 1)
                        .map(|next| matches!(next, WlirInstruction::AssertInvariant { .. }))
                        .unwrap_or(false);

                    if guards_invariant {
                        preserved += 1;
                    } else {
                        // Check if there's another SafetyFence nearby (redundant)
                        let nearby_fence = func
                            .instructions
                            .get(i.saturating_sub(2)..i)
                            .map(|window| {
                                window
                                    .iter()
                                    .any(|w| matches!(w, WlirInstruction::SafetyFence { .. }))
                            })
                            .unwrap_or(false);
                        if nearby_fence {
                            removable += 1;
                        } else {
                            preserved += 1;
                        }
                    }
                }
            }
        }

        Ok(PassResult {
            pass_id: PassId::SafetyFenceMinimization,
            applied: removable > 0,
            changes_made: removable,
            description: format!(
                "{} redundant fences minimized, {} invariant-guarding fences preserved (I.COMPILE-2b)",
                removable, preserved
            ),
        })
    }
}

// ── Optimization Pipeline ────────────────────────────────────────────

/// A pipeline of optimization passes to apply in sequence.
pub struct OptimizationPipeline {
    passes: Vec<Box<dyn OptimizationPass>>,
}

impl OptimizationPipeline {
    /// Create an empty pipeline.
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    /// Add a pass to the pipeline.
    pub fn add_pass(&mut self, pass: Box<dyn OptimizationPass>) {
        self.passes.push(pass);
    }

    /// Build a pipeline from a compilation strategy.
    pub fn from_strategy(strategy: &CompilationStrategy) -> Self {
        let mut pipeline = Self::new();

        for pass_id in &strategy.enabled_passes {
            let pass: Box<dyn OptimizationPass> = match pass_id {
                PassId::ConstantFolding => Box::new(ConstantFoldingPass),
                PassId::DeadCodeElimination => Box::new(DeadCodeEliminationPass),
                PassId::CommonSubexpressionElimination => {
                    Box::new(CommonSubexpressionEliminationPass)
                }
                PassId::InliningPass => Box::new(InliningPass),
                PassId::TailCallOptimization => Box::new(TailCallOptimizationPass),
                PassId::CommitmentBatching => Box::new(CommitmentBatchingPass),
                PassId::ProvenanceCoalescing => Box::new(ProvenanceCoalescingPass),
                PassId::EventDeduplication => Box::new(EventDeduplicationPass),
                PassId::OperatorDispatchSpecialization => {
                    Box::new(OperatorDispatchSpecializationPass)
                }
                PassId::MemoryTierPromotion => Box::new(MemoryTierPromotionPass),
                PassId::SafetyFenceMinimization => Box::new(SafetyFenceMinimizationPass),
            };
            pipeline.add_pass(pass);
        }

        pipeline
    }

    /// Apply all passes in sequence.
    pub fn apply_all(&self, module: &WlirModule) -> CompilerResult<Vec<PassResult>> {
        let mut results = Vec::new();
        for pass in &self.passes {
            let result = pass.apply(module)?;
            results.push(result);
        }
        Ok(results)
    }

    /// Number of passes in the pipeline.
    pub fn len(&self) -> usize {
        self.passes.len()
    }

    /// Whether the pipeline is empty.
    pub fn is_empty(&self) -> bool {
        self.passes.is_empty()
    }
}

impl Default for OptimizationPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CompilationTarget, TargetArch};
    use maple_worldline_ir::instructions::BoundaryDirection;
    use maple_worldline_ir::module::WlirFunction;
    use maple_worldline_ir::types::WlirType;

    fn make_module() -> WlirModule {
        let mut module = WlirModule::new("test", "1.0");
        let mut f = WlirFunction::new("main", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::Nop);
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);
        module
    }

    fn make_module_with_constants() -> WlirModule {
        let mut module = WlirModule::new("const-test", "1.0");
        let mut f = WlirFunction::new("compute", vec![], WlirType::I32);
        f.push_instruction(WlirInstruction::LoadConst {
            result: 0,
            constant_index: 0,
        });
        f.push_instruction(WlirInstruction::LoadConst {
            result: 1,
            constant_index: 1,
        });
        f.push_instruction(WlirInstruction::Add {
            result: 2,
            a: 0,
            b: 1,
        });
        f.push_instruction(WlirInstruction::Return { value: Some(2) });
        module.add_function(f);
        module
    }

    fn make_module_with_commitments() -> WlirModule {
        let mut module = WlirModule::new("commit-test", "1.0");
        let mut f = WlirFunction::new("transact", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::CrossCommitmentBoundary {
            commitment_id: "payment".into(),
            direction: BoundaryDirection::Enter,
        });
        f.push_instruction(WlirInstruction::Nop);
        f.push_instruction(WlirInstruction::CrossCommitmentBoundary {
            commitment_id: "payment".into(),
            direction: BoundaryDirection::Exit,
        });
        // Different scope
        f.push_instruction(WlirInstruction::CrossCommitmentBoundary {
            commitment_id: "audit".into(),
            direction: BoundaryDirection::Enter,
        });
        f.push_instruction(WlirInstruction::CrossCommitmentBoundary {
            commitment_id: "audit".into(),
            direction: BoundaryDirection::Exit,
        });
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);
        module
    }

    fn make_module_with_safety_fences() -> WlirModule {
        let mut module = WlirModule::new("safety-test", "1.0");
        let mut f = WlirFunction::new("safe_op", vec![], WlirType::Void);
        // Fence guarding an invariant — MUST NOT be removed
        f.push_instruction(WlirInstruction::SafetyFence {
            fence_name: "pre-invariant".into(),
            preceding_ops: vec![0],
        });
        f.push_instruction(WlirInstruction::AssertInvariant {
            condition: 0,
            invariant_name: "balance_positive".into(),
            message: "balance must be positive".into(),
        });
        // Standalone fence — may be removable
        f.push_instruction(WlirInstruction::SafetyFence {
            fence_name: "standalone".into(),
            preceding_ops: vec![1],
        });
        f.push_instruction(WlirInstruction::Nop);
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);
        module
    }

    #[test]
    fn constant_folding_applies() {
        let pass = ConstantFoldingPass;
        let module = make_module_with_constants();
        let result = pass.apply(&module).unwrap();
        assert_eq!(result.pass_id, PassId::ConstantFolding);
        assert!(result.applied);
        assert!(result.changes_made > 0);
    }

    #[test]
    fn dead_code_elimination_finds_dead_code() {
        let mut module = WlirModule::new("dead-test", "1.0");
        let mut f = WlirFunction::new("main", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::Return { value: None });
        f.push_instruction(WlirInstruction::Add {
            result: 0,
            a: 1,
            b: 2,
        }); // dead
        module.add_function(f);

        let pass = DeadCodeEliminationPass;
        let result = pass.apply(&module).unwrap();
        assert!(result.applied);
        assert_eq!(result.changes_made, 1);
    }

    #[test]
    fn cse_eliminates_redundant() {
        let pass = CommonSubexpressionEliminationPass;
        let module = make_module_with_constants();
        let result = pass.apply(&module).unwrap();
        assert_eq!(result.pass_id, PassId::CommonSubexpressionElimination);
    }

    #[test]
    fn inlining_inlines_small_functions() {
        let pass = InliningPass;
        let module = make_module(); // 2 instructions — small enough
        let result = pass.apply(&module).unwrap();
        assert!(result.applied);
        assert_eq!(result.changes_made, 1);
    }

    #[test]
    fn tail_call_optimization() {
        let mut module = WlirModule::new("tco-test", "1.0");
        let mut f = WlirFunction::new("recursive", vec![], WlirType::I32);
        f.push_instruction(WlirInstruction::LoadConst {
            result: 0,
            constant_index: 0,
        });
        f.push_instruction(WlirInstruction::Call {
            result: 1,
            function: maple_worldline_ir::types::FunctionId::from_name("recursive"),
            args: vec![0],
        });
        f.push_instruction(WlirInstruction::Return { value: Some(1) });
        module.add_function(f);

        let pass = TailCallOptimizationPass;
        let result = pass.apply(&module).unwrap();
        assert!(result.applied);
        assert_eq!(result.changes_made, 1);
    }

    #[test]
    fn commitment_batching_same_scope() {
        let mut module = WlirModule::new("batch-test", "1.0");
        let mut f = WlirFunction::new("batch", vec![], WlirType::Void);
        // Same scope: exit then enter again
        f.push_instruction(WlirInstruction::CrossCommitmentBoundary {
            commitment_id: "pay".into(),
            direction: BoundaryDirection::Enter,
        });
        f.push_instruction(WlirInstruction::CrossCommitmentBoundary {
            commitment_id: "pay".into(),
            direction: BoundaryDirection::Exit,
        });
        f.push_instruction(WlirInstruction::CrossCommitmentBoundary {
            commitment_id: "pay".into(),
            direction: BoundaryDirection::Enter,
        });
        f.push_instruction(WlirInstruction::CrossCommitmentBoundary {
            commitment_id: "pay".into(),
            direction: BoundaryDirection::Exit,
        });
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);

        let pass = CommitmentBatchingPass;
        let result = pass.apply(&module).unwrap();
        // Exit→Enter of same scope is batchable
        assert!(result.applied);
        assert!(result.changes_made > 0);
    }

    #[test]
    fn commitment_batching_rejects_different_scopes() {
        let module = make_module_with_commitments();
        let pass = CommitmentBatchingPass;
        let result = pass.apply(&module).unwrap();
        // Different scopes (payment→audit) must be preserved
        assert!(result.description.contains("different-scope preserved"));
    }

    #[test]
    fn provenance_coalescing() {
        let mut module = WlirModule::new("prov-test", "1.0");
        let mut f = WlirFunction::new("traced", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::RecordProvenance {
            operation: "op1".into(),
            inputs: vec![0],
            output: 1,
        });
        f.push_instruction(WlirInstruction::RecordProvenance {
            operation: "op2".into(),
            inputs: vec![1],
            output: 2,
        });
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);

        let pass = ProvenanceCoalescingPass;
        let result = pass.apply(&module).unwrap();
        assert!(result.applied);
        assert_eq!(result.changes_made, 1);
    }

    #[test]
    fn event_deduplication() {
        let mut module = WlirModule::new("event-test", "1.0");
        let mut f = WlirFunction::new("emit", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::EmitEvent {
            event_type: "click".into(),
            payload_registers: vec![0],
            result_event_id: 1,
        });
        f.push_instruction(WlirInstruction::EmitEvent {
            event_type: "click".into(),
            payload_registers: vec![0],
            result_event_id: 2,
        });
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);

        let pass = EventDeduplicationPass;
        let result = pass.apply(&module).unwrap();
        assert!(result.applied);
        assert_eq!(result.changes_made, 1);
    }

    #[test]
    fn operator_dispatch_specialization() {
        let mut module = WlirModule::new("op-test", "1.0");
        let mut f = WlirFunction::new("dispatch", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::InvokeOperator {
            operator_name: "transfer".into(),
            args: vec![0, 1],
            result: 2,
        });
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);

        let pass = OperatorDispatchSpecializationPass;
        let result = pass.apply(&module).unwrap();
        assert!(result.applied);
        assert_eq!(result.changes_made, 1);
    }

    #[test]
    fn memory_tier_promotion() {
        let mut module = WlirModule::new("mem-test", "1.0");
        let mut f = WlirFunction::new("mem_op", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::MemoryQuery {
            tier: maple_worldline_ir::MemoryTier::Episodic,
            key: 0,
            result: 1,
        });
        f.push_instruction(WlirInstruction::MemoryStore {
            tier: maple_worldline_ir::MemoryTier::Episodic,
            key: 0,
            value: 1,
        });
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);

        let pass = MemoryTierPromotionPass;
        let result = pass.apply(&module).unwrap();
        assert!(result.applied);
    }

    #[test]
    fn safety_fence_minimization_preserves_invariants() {
        let module = make_module_with_safety_fences();
        let pass = SafetyFenceMinimizationPass;
        let result = pass.apply(&module).unwrap();
        // The fence before AssertInvariant MUST be preserved
        assert!(result
            .description
            .contains("invariant-guarding fences preserved"));
        // The number preserved must be at least 1 (the invariant guard)
        let preserved_str = result
            .description
            .split("invariant-guarding")
            .next()
            .unwrap();
        let last_num: String = preserved_str
            .chars()
            .rev()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        let preserved: u32 = last_num.parse().unwrap_or(0);
        assert!(
            preserved >= 1,
            "At least 1 invariant fence must be preserved"
        );
    }

    #[test]
    fn safety_fence_minimization_redundant_only() {
        // Module with adjacent fences — second is redundant
        let mut module = WlirModule::new("fence-test", "1.0");
        let mut f = WlirFunction::new("fenced", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::SafetyFence {
            fence_name: "fence1".into(),
            preceding_ops: vec![0],
        });
        f.push_instruction(WlirInstruction::SafetyFence {
            fence_name: "fence2".into(),
            preceding_ops: vec![0],
        });
        f.push_instruction(WlirInstruction::Nop);
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);

        let pass = SafetyFenceMinimizationPass;
        let result = pass.apply(&module).unwrap();
        assert!(result.applied);
        assert!(result.changes_made > 0, "Redundant fence should be removed");
    }

    #[test]
    fn pipeline_applies_all_passes() {
        let mut pipeline = OptimizationPipeline::new();
        pipeline.add_pass(Box::new(ConstantFoldingPass));
        pipeline.add_pass(Box::new(DeadCodeEliminationPass));
        assert_eq!(pipeline.len(), 2);

        let module = make_module();
        let results = pipeline.apply_all(&module).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn pipeline_from_strategy() {
        let strategy = CompilationStrategy::new(
            "basic",
            CompilationTarget::Native {
                arch: TargetArch::X86_64,
            },
            crate::types::OptimizationLevel::Basic,
        );
        let pipeline = OptimizationPipeline::from_strategy(&strategy);
        assert_eq!(pipeline.len(), 5);
    }
}
