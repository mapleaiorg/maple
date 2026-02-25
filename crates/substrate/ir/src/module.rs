//! WLIR module structure.
//!
//! A `WlirModule` is the top-level container for WLIR programs. It holds
//! functions, type declarations, constants, and commitment declarations.
//! Functions contain sequences of WLIR instructions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::instructions::{BoundaryDirection, WlirInstruction};
use crate::types::{
    FunctionId, ModuleId, SourceLocation, VerificationStatus, WlirModuleSummary, WlirType,
    WlirValue,
};

// ── Function ───────────────────────────────────────────────────────────

/// A WLIR function definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WlirFunction {
    /// Unique function ID.
    pub id: FunctionId,
    /// Function name.
    pub name: String,
    /// Parameters: (name, type) pairs.
    pub params: Vec<(String, WlirType)>,
    /// Return type.
    pub return_type: WlirType,
    /// Instruction sequence.
    pub instructions: Vec<WlirInstruction>,
    /// Number of local variables.
    pub local_count: u32,
}

impl WlirFunction {
    /// Create a new function.
    pub fn new(
        name: impl Into<String>,
        params: Vec<(String, WlirType)>,
        return_type: WlirType,
    ) -> Self {
        let name = name.into();
        Self {
            id: FunctionId::from_name(&name),
            name,
            params,
            return_type,
            instructions: vec![],
            local_count: 0,
        }
    }

    /// Add an instruction to this function.
    pub fn push_instruction(&mut self, inst: WlirInstruction) {
        self.instructions.push(inst);
    }

    /// Total number of instructions.
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    /// Whether this function contains any safety instructions.
    pub fn has_safety_instructions(&self) -> bool {
        self.instructions.iter().any(|i| i.is_safety())
    }

    /// Whether this function crosses any commitment boundaries.
    pub fn crosses_commitment_boundary(&self) -> bool {
        self.instructions
            .iter()
            .any(|i| matches!(i, WlirInstruction::CrossCommitmentBoundary { .. }))
    }

    /// Extract all source locations from this function.
    pub fn source_locations(&self) -> Vec<&SourceLocation> {
        self.instructions
            .iter()
            .filter_map(|i| match i {
                WlirInstruction::SetSourceLocation { location } => Some(location),
                _ => None,
            })
            .collect()
    }

    /// Check commitment boundary integrity (matched Enter/Exit pairs).
    pub fn commitment_boundaries_balanced(&self) -> bool {
        let mut stack: Vec<&str> = vec![];
        for inst in &self.instructions {
            if let WlirInstruction::CrossCommitmentBoundary {
                commitment_id,
                direction,
            } = inst
            {
                match direction {
                    BoundaryDirection::Enter => stack.push(commitment_id),
                    BoundaryDirection::Exit => {
                        if stack.last().map(|s| *s) != Some(commitment_id.as_str()) {
                            return false;
                        }
                        stack.pop();
                    }
                }
            }
        }
        stack.is_empty()
    }
}

// ── Type Declaration ───────────────────────────────────────────────────

/// A WLIR type declaration within a module.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WlirTypeDecl {
    /// Type name.
    pub name: String,
    /// Underlying WLIR type.
    pub underlying: WlirType,
    /// Description.
    pub description: String,
}

// ── Constant ───────────────────────────────────────────────────────────

/// A named constant within a WLIR module.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WlirConstant {
    /// Constant name.
    pub name: String,
    /// Constant value.
    pub value: WlirValue,
    /// Description.
    pub description: String,
}

// ── Commitment Declaration ─────────────────────────────────────────────

/// A commitment scope declaration within a module.
///
/// Declares that certain functions form commitment boundaries and
/// specifies which safety fences are required.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitmentDeclaration {
    /// Commitment name.
    pub name: String,
    /// Description of this commitment scope.
    pub description: String,
    /// Safety fences required within this commitment.
    pub required_safety_fences: Vec<String>,
    /// Function that enters this commitment scope.
    pub entry_function: FunctionId,
    /// Function that exits this commitment scope.
    pub exit_function: FunctionId,
}

// ── Module ─────────────────────────────────────────────────────────────

/// A WLIR module — the top-level container for WLIR programs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WlirModule {
    /// Unique module ID.
    pub id: ModuleId,
    /// Module name.
    pub name: String,
    /// Module version.
    pub version: String,
    /// Function definitions.
    pub functions: Vec<WlirFunction>,
    /// Type declarations.
    pub types: Vec<WlirTypeDecl>,
    /// Named constants.
    pub constants: Vec<WlirConstant>,
    /// Commitment declarations.
    pub commitment_declarations: Vec<CommitmentDeclaration>,
    /// Current verification status.
    pub verification_status: VerificationStatus,
    /// When this module was created.
    pub created_at: DateTime<Utc>,
}

impl WlirModule {
    /// Create a new empty module.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: ModuleId::new(),
            name: name.into(),
            version: version.into(),
            functions: vec![],
            types: vec![],
            constants: vec![],
            commitment_declarations: vec![],
            verification_status: VerificationStatus::Unverified,
            created_at: Utc::now(),
        }
    }

    /// Add a function to the module.
    pub fn add_function(&mut self, func: WlirFunction) {
        self.functions.push(func);
    }

    /// Add a type declaration.
    pub fn add_type(&mut self, decl: WlirTypeDecl) {
        self.types.push(decl);
    }

    /// Add a constant.
    pub fn add_constant(&mut self, constant: WlirConstant) {
        self.constants.push(constant);
    }

    /// Add a commitment declaration.
    pub fn add_commitment(&mut self, decl: CommitmentDeclaration) {
        self.commitment_declarations.push(decl);
    }

    /// Find a function by name.
    pub fn find_function(&self, name: &str) -> Option<&WlirFunction> {
        self.functions.iter().find(|f| f.name == name)
    }

    /// Total instructions across all functions.
    pub fn total_instructions(&self) -> usize {
        self.functions.iter().map(|f| f.instruction_count()).sum()
    }

    /// All function IDs referenced by commitment declarations.
    pub fn commitment_functions(&self) -> Vec<&FunctionId> {
        self.commitment_declarations
            .iter()
            .flat_map(|c| vec![&c.entry_function, &c.exit_function])
            .collect()
    }

    /// Generate a summary of this module.
    pub fn summary(&self) -> WlirModuleSummary {
        WlirModuleSummary {
            total_functions: self.functions.len(),
            total_instructions: self.total_instructions(),
            total_types: self.types.len(),
            total_constants: self.constants.len(),
            total_commitment_declarations: self.commitment_declarations.len(),
            verification_status: self.verification_status.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{WlirData, WlirType};

    #[test]
    fn module_creation() {
        let module = WlirModule::new("test-module", "1.0.0");
        assert_eq!(module.name, "test-module");
        assert_eq!(module.version, "1.0.0");
        assert!(module.functions.is_empty());
        assert!(matches!(
            module.verification_status,
            VerificationStatus::Unverified
        ));
    }

    #[test]
    fn add_function() {
        let mut module = WlirModule::new("test", "1.0");
        let func = WlirFunction::new("main", vec![], WlirType::Void);
        module.add_function(func);
        assert_eq!(module.functions.len(), 1);
        assert!(module.find_function("main").is_some());
    }

    #[test]
    fn add_type_declaration() {
        let mut module = WlirModule::new("test", "1.0");
        module.add_type(WlirTypeDecl {
            name: "Amount".into(),
            underlying: WlirType::Amount {
                currency: "USD".into(),
            },
            description: "USD amount".into(),
        });
        assert_eq!(module.types.len(), 1);
    }

    #[test]
    fn add_constant() {
        let mut module = WlirModule::new("test", "1.0");
        module.add_constant(WlirConstant {
            name: "MAX_RETRIES".into(),
            value: WlirValue {
                value_type: WlirType::I32,
                data: WlirData::Integer(3),
            },
            description: "Maximum retry count".into(),
        });
        assert_eq!(module.constants.len(), 1);
    }

    #[test]
    fn add_commitment_declaration() {
        let mut module = WlirModule::new("test", "1.0");
        module.add_commitment(CommitmentDeclaration {
            name: "payment".into(),
            description: "Payment processing commitment".into(),
            required_safety_fences: vec!["validate_amount".into()],
            entry_function: FunctionId::from_name("begin_payment"),
            exit_function: FunctionId::from_name("end_payment"),
        });
        assert_eq!(module.commitment_declarations.len(), 1);
    }

    #[test]
    fn find_function_by_name() {
        let mut module = WlirModule::new("test", "1.0");
        module.add_function(WlirFunction::new("foo", vec![], WlirType::Void));
        module.add_function(WlirFunction::new("bar", vec![], WlirType::I32));

        assert!(module.find_function("foo").is_some());
        assert!(module.find_function("bar").is_some());
        assert!(module.find_function("baz").is_none());
    }

    #[test]
    fn total_instruction_count() {
        let mut module = WlirModule::new("test", "1.0");
        let mut f1 = WlirFunction::new("f1", vec![], WlirType::Void);
        f1.push_instruction(WlirInstruction::Nop);
        f1.push_instruction(WlirInstruction::Return { value: None });
        let mut f2 = WlirFunction::new("f2", vec![], WlirType::Void);
        f2.push_instruction(WlirInstruction::Nop);
        module.add_function(f1);
        module.add_function(f2);
        assert_eq!(module.total_instructions(), 3);
    }

    #[test]
    fn summary_generation() {
        let mut module = WlirModule::new("test", "1.0");
        let mut f = WlirFunction::new("main", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::Nop);
        module.add_function(f);
        module.add_type(WlirTypeDecl {
            name: "T".into(),
            underlying: WlirType::I32,
            description: "test".into(),
        });
        let summary = module.summary();
        assert_eq!(summary.total_functions, 1);
        assert_eq!(summary.total_instructions, 1);
        assert_eq!(summary.total_types, 1);
    }

    #[test]
    fn commitment_functions_returns_ids() {
        let mut module = WlirModule::new("test", "1.0");
        module.add_commitment(CommitmentDeclaration {
            name: "c1".into(),
            description: "test".into(),
            required_safety_fences: vec![],
            entry_function: FunctionId::from_name("enter_c1"),
            exit_function: FunctionId::from_name("exit_c1"),
        });
        let fns = module.commitment_functions();
        assert_eq!(fns.len(), 2);
    }

    #[test]
    fn function_has_safety_instructions() {
        let mut f = WlirFunction::new("safe", vec![], WlirType::Void);
        assert!(!f.has_safety_instructions());
        f.push_instruction(WlirInstruction::AssertInvariant {
            condition: 0,
            invariant_name: "test".into(),
            message: "fail".into(),
        });
        assert!(f.has_safety_instructions());
    }
}
