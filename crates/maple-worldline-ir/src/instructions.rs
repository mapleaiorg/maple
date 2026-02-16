//! WLIR instruction set.
//!
//! Instructions are organized into four categories:
//! - **Standard**: arithmetic, memory, control flow
//! - **EVOS-native**: events, commitments, provenance, operators, memory tiers
//! - **Safety**: invariants, coercion checks, safety fences
//! - **Metadata**: source locations, type annotations

use serde::{Deserialize, Serialize};

use crate::types::{FunctionId, MemoryTier, SourceLocation, WlirType};
use maple_worldline_self_mod_gate::types::SelfModTier;

// ── Comparison Operator ────────────────────────────────────────────────

/// Comparison operators for the Compare instruction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompareOp {
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
}

impl std::fmt::Display for CompareOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equal => write!(f, "=="),
            Self::NotEqual => write!(f, "!="),
            Self::LessThan => write!(f, "<"),
            Self::LessEqual => write!(f, "<="),
            Self::GreaterThan => write!(f, ">"),
            Self::GreaterEqual => write!(f, ">="),
        }
    }
}

// ── Boundary Direction ─────────────────────────────────────────────────

/// Direction when crossing a commitment boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundaryDirection {
    /// Entering a commitment scope.
    Enter,
    /// Exiting a commitment scope.
    Exit,
}

impl std::fmt::Display for BoundaryDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Enter => write!(f, "enter"),
            Self::Exit => write!(f, "exit"),
        }
    }
}

// ── Instruction Category ───────────────────────────────────────────────

/// Category of a WLIR instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstructionCategory {
    /// Standard computational instructions.
    Standard,
    /// EVOS-native instructions.
    EvosNative,
    /// Safety instructions.
    Safety,
    /// Metadata instructions.
    Metadata,
}

impl std::fmt::Display for InstructionCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Standard => write!(f, "standard"),
            Self::EvosNative => write!(f, "evos-native"),
            Self::Safety => write!(f, "safety"),
            Self::Metadata => write!(f, "metadata"),
        }
    }
}

// ── WLIR Instruction ───────────────────────────────────────────────────

/// A single WLIR instruction.
///
/// The instruction set covers four categories:
/// 1. Standard computational (arithmetic, memory, control flow)
/// 2. EVOS-native (events, commitments, provenance, operators)
/// 3. Safety (invariants, coercion checks, fences)
/// 4. Metadata (source locations, type annotations)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WlirInstruction {
    // ── Standard Computational ─────────────────────────────────────
    /// Integer addition: result = a + b
    Add { result: u32, a: u32, b: u32 },
    /// Integer subtraction: result = a - b
    Sub { result: u32, a: u32, b: u32 },
    /// Integer multiplication: result = a * b
    Mul { result: u32, a: u32, b: u32 },
    /// Integer division: result = a / b
    Div { result: u32, a: u32, b: u32 },
    /// Modulo: result = a % b
    Mod { result: u32, a: u32, b: u32 },
    /// Negate: result = -a
    Neg { result: u32, a: u32 },
    /// Comparison: result = a cmp b (result is bool)
    Compare {
        result: u32,
        a: u32,
        b: u32,
        op: CompareOp,
    },
    /// Boolean AND: result = a && b
    And { result: u32, a: u32, b: u32 },
    /// Boolean OR: result = a || b
    Or { result: u32, a: u32, b: u32 },
    /// Boolean NOT: result = !a
    Not { result: u32, a: u32 },
    /// Load a constant into a register.
    LoadConst { result: u32, constant_index: u32 },
    /// Load from local variable.
    LoadLocal { result: u32, local_index: u32 },
    /// Store to local variable.
    StoreLocal { local_index: u32, value: u32 },
    /// Unconditional jump.
    Jump { target: u32 },
    /// Conditional branch.
    Branch {
        condition: u32,
        true_target: u32,
        false_target: u32,
    },
    /// Function call.
    Call {
        result: u32,
        function: FunctionId,
        args: Vec<u32>,
    },
    /// Return from function.
    Return { value: Option<u32> },

    // ── EVOS-Native ────────────────────────────────────────────────
    /// Emit an event into the Event Fabric.
    EmitEvent {
        event_type: String,
        payload_registers: Vec<u32>,
        result_event_id: u32,
    },
    /// Cross a commitment boundary (enter or exit a commitment scope).
    CrossCommitmentBoundary {
        commitment_id: String,
        direction: BoundaryDirection,
    },
    /// Record provenance for the current operation.
    RecordProvenance {
        operation: String,
        inputs: Vec<u32>,
        output: u32,
    },
    /// Query the tier-aware memory system.
    MemoryQuery {
        tier: MemoryTier,
        key: u32,
        result: u32,
    },
    /// Store to the tier-aware memory system.
    MemoryStore {
        tier: MemoryTier,
        key: u32,
        value: u32,
    },
    /// Invoke a WorldLine operator.
    InvokeOperator {
        operator_name: String,
        args: Vec<u32>,
        result: u32,
    },
    /// Hot-swap an operator implementation at runtime.
    HotSwapOperator {
        operator_name: String,
        new_implementation: String,
        tier: SelfModTier,
    },

    // ── Safety ─────────────────────────────────────────────────────
    /// Assert an invariant (halt if violated).
    AssertInvariant {
        condition: u32,
        invariant_name: String,
        message: String,
    },
    /// Check that a coercion is valid according to type rules.
    CoercionCheck {
        value: u32,
        from_type: WlirType,
        to_type: WlirType,
        result: u32,
    },
    /// Safety fence — ensures ordering of safety-critical operations.
    SafetyFence {
        fence_name: String,
        preceding_ops: Vec<u32>,
    },

    // ── Metadata ───────────────────────────────────────────────────
    /// Associate a source location with subsequent instructions.
    SetSourceLocation { location: SourceLocation },
    /// Annotate a register with a type (for the verifier).
    TypeAnnotation {
        register: u32,
        annotated_type: WlirType,
    },
    /// No operation (placeholder/alignment).
    Nop,
}

impl WlirInstruction {
    /// Classify this instruction by category.
    pub fn category(&self) -> InstructionCategory {
        match self {
            Self::Add { .. }
            | Self::Sub { .. }
            | Self::Mul { .. }
            | Self::Div { .. }
            | Self::Mod { .. }
            | Self::Neg { .. }
            | Self::Compare { .. }
            | Self::And { .. }
            | Self::Or { .. }
            | Self::Not { .. }
            | Self::LoadConst { .. }
            | Self::LoadLocal { .. }
            | Self::StoreLocal { .. }
            | Self::Jump { .. }
            | Self::Branch { .. }
            | Self::Call { .. }
            | Self::Return { .. } => InstructionCategory::Standard,

            Self::EmitEvent { .. }
            | Self::CrossCommitmentBoundary { .. }
            | Self::RecordProvenance { .. }
            | Self::MemoryQuery { .. }
            | Self::MemoryStore { .. }
            | Self::InvokeOperator { .. }
            | Self::HotSwapOperator { .. } => InstructionCategory::EvosNative,

            Self::AssertInvariant { .. }
            | Self::CoercionCheck { .. }
            | Self::SafetyFence { .. } => InstructionCategory::Safety,

            Self::SetSourceLocation { .. } | Self::TypeAnnotation { .. } | Self::Nop => {
                InstructionCategory::Metadata
            }
        }
    }

    /// Whether this instruction has side effects (modifies external state).
    pub fn has_side_effects(&self) -> bool {
        matches!(
            self,
            Self::EmitEvent { .. }
                | Self::CrossCommitmentBoundary { .. }
                | Self::RecordProvenance { .. }
                | Self::MemoryStore { .. }
                | Self::InvokeOperator { .. }
                | Self::HotSwapOperator { .. }
                | Self::StoreLocal { .. }
        )
    }

    /// Whether this is a control-flow instruction.
    pub fn is_control_flow(&self) -> bool {
        matches!(
            self,
            Self::Jump { .. } | Self::Branch { .. } | Self::Call { .. } | Self::Return { .. }
        )
    }

    /// Whether this is a safety instruction.
    pub fn is_safety(&self) -> bool {
        matches!(
            self,
            Self::AssertInvariant { .. } | Self::CoercionCheck { .. } | Self::SafetyFence { .. }
        )
    }

    /// Registers read by this instruction.
    pub fn reads(&self) -> Vec<u32> {
        match self {
            Self::Add { a, b, .. }
            | Self::Sub { a, b, .. }
            | Self::Mul { a, b, .. }
            | Self::Div { a, b, .. }
            | Self::Mod { a, b, .. }
            | Self::Compare { a, b, .. }
            | Self::And { a, b, .. }
            | Self::Or { a, b, .. } => vec![*a, *b],

            Self::Neg { a, .. } | Self::Not { a, .. } => vec![*a],

            Self::StoreLocal { value, .. } => vec![*value],
            Self::Branch { condition, .. } => vec![*condition],
            Self::Call { args, .. } => args.clone(),
            Self::Return { value } => value.iter().copied().collect(),
            Self::EmitEvent {
                payload_registers, ..
            } => payload_registers.clone(),
            Self::RecordProvenance { inputs, .. } => inputs.clone(),
            Self::MemoryQuery { key, .. } => vec![*key],
            Self::MemoryStore { key, value, .. } => vec![*key, *value],
            Self::InvokeOperator { args, .. } => args.clone(),
            Self::AssertInvariant { condition, .. } => vec![*condition],
            Self::CoercionCheck { value, .. } => vec![*value],
            Self::SafetyFence { preceding_ops, .. } => preceding_ops.clone(),
            _ => vec![],
        }
    }

    /// Register written by this instruction (if any).
    pub fn writes(&self) -> Option<u32> {
        match self {
            Self::Add { result, .. }
            | Self::Sub { result, .. }
            | Self::Mul { result, .. }
            | Self::Div { result, .. }
            | Self::Mod { result, .. }
            | Self::Neg { result, .. }
            | Self::Compare { result, .. }
            | Self::And { result, .. }
            | Self::Or { result, .. }
            | Self::Not { result, .. }
            | Self::LoadConst { result, .. }
            | Self::LoadLocal { result, .. }
            | Self::Call { result, .. }
            | Self::MemoryQuery { result, .. }
            | Self::InvokeOperator { result, .. }
            | Self::CoercionCheck { result, .. } => Some(*result),

            Self::EmitEvent {
                result_event_id, ..
            } => Some(*result_event_id),
            Self::RecordProvenance { output, .. } => Some(*output),

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_category_is_standard() {
        let inst = WlirInstruction::Add {
            result: 0,
            a: 1,
            b: 2,
        };
        assert_eq!(inst.category(), InstructionCategory::Standard);
    }

    #[test]
    fn emit_event_category_is_evos_native() {
        let inst = WlirInstruction::EmitEvent {
            event_type: "test".into(),
            payload_registers: vec![0],
            result_event_id: 1,
        };
        assert_eq!(inst.category(), InstructionCategory::EvosNative);
    }

    #[test]
    fn assert_invariant_category_is_safety() {
        let inst = WlirInstruction::AssertInvariant {
            condition: 0,
            invariant_name: "test".into(),
            message: "fail".into(),
        };
        assert_eq!(inst.category(), InstructionCategory::Safety);
    }

    #[test]
    fn source_location_category_is_metadata() {
        let inst = WlirInstruction::SetSourceLocation {
            location: SourceLocation {
                file: "test.rs".into(),
                line: 1,
                column: 1,
            },
        };
        assert_eq!(inst.category(), InstructionCategory::Metadata);
    }

    #[test]
    fn emit_event_has_side_effects() {
        let inst = WlirInstruction::EmitEvent {
            event_type: "test".into(),
            payload_registers: vec![],
            result_event_id: 0,
        };
        assert!(inst.has_side_effects());
    }

    #[test]
    fn add_has_no_side_effects() {
        let inst = WlirInstruction::Add {
            result: 0,
            a: 1,
            b: 2,
        };
        assert!(!inst.has_side_effects());
    }

    #[test]
    fn jump_is_control_flow() {
        let inst = WlirInstruction::Jump { target: 5 };
        assert!(inst.is_control_flow());
    }

    #[test]
    fn add_is_not_control_flow() {
        let inst = WlirInstruction::Add {
            result: 0,
            a: 1,
            b: 2,
        };
        assert!(!inst.is_control_flow());
    }

    #[test]
    fn assert_invariant_is_safety() {
        let inst = WlirInstruction::AssertInvariant {
            condition: 0,
            invariant_name: "test".into(),
            message: "fail".into(),
        };
        assert!(inst.is_safety());
    }

    #[test]
    fn add_reads_and_writes() {
        let inst = WlirInstruction::Add {
            result: 0,
            a: 1,
            b: 2,
        };
        assert_eq!(inst.reads(), vec![1, 2]);
        assert_eq!(inst.writes(), Some(0));
    }

    #[test]
    fn compare_op_display() {
        assert_eq!(CompareOp::Equal.to_string(), "==");
        assert_eq!(CompareOp::NotEqual.to_string(), "!=");
        assert_eq!(CompareOp::LessThan.to_string(), "<");
        assert_eq!(CompareOp::LessEqual.to_string(), "<=");
        assert_eq!(CompareOp::GreaterThan.to_string(), ">");
        assert_eq!(CompareOp::GreaterEqual.to_string(), ">=");
    }

    #[test]
    fn boundary_direction_display() {
        assert_eq!(BoundaryDirection::Enter.to_string(), "enter");
        assert_eq!(BoundaryDirection::Exit.to_string(), "exit");
    }

    #[test]
    fn nop_has_no_reads_writes() {
        let inst = WlirInstruction::Nop;
        assert!(inst.reads().is_empty());
        assert!(inst.writes().is_none());
    }

    #[test]
    fn instruction_category_display() {
        assert_eq!(InstructionCategory::Standard.to_string(), "standard");
        assert_eq!(InstructionCategory::EvosNative.to_string(), "evos-native");
        assert_eq!(InstructionCategory::Safety.to_string(), "safety");
        assert_eq!(InstructionCategory::Metadata.to_string(), "metadata");
    }
}
