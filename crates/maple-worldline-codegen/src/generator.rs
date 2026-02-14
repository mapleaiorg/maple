//! Code generator — trait and simulated implementation.
//!
//! The `CodeGenerator` trait abstracts LLM-backed code generation.
//! Real implementations would call an LLM API; the `SimulatedGenerator`
//! produces deterministic placeholder code for testing.

use chrono::Utc;

use maple_worldline_intent::proposal::CodeChangeSpec;
use maple_worldline_intent::types::CodeChangeType;
use maple_worldline_self_mod_gate::types::SelfModTier;

use crate::error::{CodegenError, CodegenResult};
use crate::types::GeneratedCode;

// ── Generation Context ─────────────────────────────────────────────────

/// Context provided to the code generator for each generation call.
#[derive(Clone, Debug)]
pub struct GenerationContext {
    /// The full proposal summary.
    pub proposal_summary: String,
    /// The rationale for the change.
    pub rationale: String,
    /// Self-modification tier (affects generation strategy).
    pub tier: SelfModTier,
    /// Index of this change spec within the proposal.
    pub change_index: usize,
    /// Total number of changes in the proposal.
    pub total_changes: usize,
}

// ── CodeGenerator Trait ────────────────────────────────────────────────

/// Trait for generating code from a CodeChangeSpec.
///
/// Real implementations would call an LLM. The simulated implementation
/// produces deterministic placeholder code for testing.
pub trait CodeGenerator: Send + Sync {
    /// Generate code for a single change specification.
    fn generate(
        &self,
        change_spec: &CodeChangeSpec,
        context: &GenerationContext,
    ) -> CodegenResult<GeneratedCode>;

    /// Name of this generator for logging.
    fn name(&self) -> &str;
}

// ── Simulated Generator ────────────────────────────────────────────────

/// A simulated code generator for testing.
///
/// Generates deterministic placeholder code based on the CodeChangeSpec.
/// Configurable to succeed or fail.
pub struct SimulatedGenerator {
    should_succeed: bool,
    /// Optional failure index — fail on the Nth generation call.
    fail_on_index: Option<usize>,
}

impl SimulatedGenerator {
    /// Create a simulated generator.
    pub fn new(should_succeed: bool) -> Self {
        Self {
            should_succeed,
            fail_on_index: None,
        }
    }

    /// Create a generator that fails on a specific change index.
    pub fn failing_on(index: usize) -> Self {
        Self {
            should_succeed: true,
            fail_on_index: Some(index),
        }
    }

    /// Generate placeholder code based on the change type.
    fn generate_stub(change_spec: &CodeChangeSpec) -> String {
        match &change_spec.change_type {
            CodeChangeType::CreateFile => {
                format!(
                    "//! Auto-generated module: {}\n\n/// Generated implementation.\npub fn init() {{}}\n",
                    change_spec.file_path,
                )
            }
            CodeChangeType::ModifyFunction { function_name } => {
                format!(
                    "/// Modified function: {}\npub fn {}() -> bool {{\n    // Optimized implementation\n    true\n}}\n",
                    change_spec.description, function_name,
                )
            }
            CodeChangeType::ModifyStruct { struct_name } => {
                format!(
                    "/// Modified struct: {}\npub struct {} {{\n    pub updated: bool,\n}}\n",
                    change_spec.description, struct_name,
                )
            }
            CodeChangeType::ModifyTrait { trait_name } => {
                format!(
                    "/// Modified trait: {}\npub trait {} {{\n    fn execute(&self) -> bool;\n}}\n",
                    change_spec.description, trait_name,
                )
            }
            CodeChangeType::AddImplementation {
                trait_name,
                struct_name,
            } => {
                format!(
                    "impl {} for {} {{\n    fn execute(&self) -> bool {{\n        true\n    }}\n}}\n",
                    trait_name, struct_name,
                )
            }
            CodeChangeType::RefactorModule { module_name } => {
                format!(
                    "//! Refactored module: {}\n//! {}\n\npub mod refactored {{\n    pub fn init() {{}}\n}}\n",
                    module_name, change_spec.description,
                )
            }
            CodeChangeType::DeleteCode { target } => {
                format!("// DELETED: {}\n", target)
            }
        }
    }
}

impl CodeGenerator for SimulatedGenerator {
    fn generate(
        &self,
        change_spec: &CodeChangeSpec,
        context: &GenerationContext,
    ) -> CodegenResult<GeneratedCode> {
        // Check if we should fail on this index
        if let Some(fail_idx) = self.fail_on_index {
            if context.change_index == fail_idx {
                return Err(CodegenError::GenerationFailed(format!(
                    "Simulated failure at change index {}",
                    fail_idx,
                )));
            }
        }

        if !self.should_succeed {
            return Err(CodegenError::GenerationFailed(format!(
                "Simulated generation failure for '{}'",
                change_spec.file_path,
            )));
        }

        let content = Self::generate_stub(change_spec);
        let content_hash = GeneratedCode::compute_hash(&content);

        Ok(GeneratedCode {
            change_spec_index: context.change_index,
            file_path: change_spec.file_path.clone(),
            content,
            description: change_spec.description.clone(),
            content_hash,
            generated_at: Utc::now(),
        })
    }

    fn name(&self) -> &str {
        "simulated-generator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_worldline_intent::types::MeaningId;

    fn make_context() -> GenerationContext {
        GenerationContext {
            proposal_summary: "Test proposal".into(),
            rationale: "Testing".into(),
            tier: SelfModTier::Tier0Configuration,
            change_index: 0,
            total_changes: 1,
        }
    }

    fn make_change_spec(change_type: CodeChangeType) -> CodeChangeSpec {
        CodeChangeSpec {
            file_path: "src/test.rs".into(),
            change_type,
            description: "Test change".into(),
            affected_regions: vec![],
            provenance: vec![MeaningId::new()],
        }
    }

    #[test]
    fn simulated_generator_success_create_file() {
        let gen = SimulatedGenerator::new(true);
        let spec = make_change_spec(CodeChangeType::CreateFile);
        let result = gen.generate(&spec, &make_context()).unwrap();
        assert!(result.content.contains("Auto-generated module"));
        assert!(!result.content_hash.is_empty());
    }

    #[test]
    fn simulated_generator_success_modify_function() {
        let gen = SimulatedGenerator::new(true);
        let spec = make_change_spec(CodeChangeType::ModifyFunction {
            function_name: "process".into(),
        });
        let result = gen.generate(&spec, &make_context()).unwrap();
        assert!(result.content.contains("fn process()"));
    }

    #[test]
    fn simulated_generator_success_modify_struct() {
        let gen = SimulatedGenerator::new(true);
        let spec = make_change_spec(CodeChangeType::ModifyStruct {
            struct_name: "Config".into(),
        });
        let result = gen.generate(&spec, &make_context()).unwrap();
        assert!(result.content.contains("pub struct Config"));
    }

    #[test]
    fn simulated_generator_success_add_impl() {
        let gen = SimulatedGenerator::new(true);
        let spec = make_change_spec(CodeChangeType::AddImplementation {
            trait_name: "Execute".into(),
            struct_name: "Handler".into(),
        });
        let result = gen.generate(&spec, &make_context()).unwrap();
        assert!(result.content.contains("impl Execute for Handler"));
    }

    #[test]
    fn simulated_generator_success_delete_code() {
        let gen = SimulatedGenerator::new(true);
        let spec = make_change_spec(CodeChangeType::DeleteCode {
            target: "old_function".into(),
        });
        let result = gen.generate(&spec, &make_context()).unwrap();
        assert!(result.content.contains("DELETED"));
    }

    #[test]
    fn simulated_generator_failure() {
        let gen = SimulatedGenerator::new(false);
        let spec = make_change_spec(CodeChangeType::CreateFile);
        let result = gen.generate(&spec, &make_context());
        assert!(result.is_err());
        match result {
            Err(CodegenError::GenerationFailed(msg)) => {
                assert!(msg.contains("Simulated generation failure"));
            }
            _ => panic!("Expected GenerationFailed"),
        }
    }

    #[test]
    fn simulated_generator_fail_on_index() {
        let gen = SimulatedGenerator::failing_on(1);
        let spec = make_change_spec(CodeChangeType::CreateFile);

        // Index 0 should succeed
        let ctx0 = GenerationContext {
            change_index: 0,
            ..make_context()
        };
        assert!(gen.generate(&spec, &ctx0).is_ok());

        // Index 1 should fail
        let ctx1 = GenerationContext {
            change_index: 1,
            ..make_context()
        };
        assert!(gen.generate(&spec, &ctx1).is_err());
    }

    #[test]
    fn generation_context_fields() {
        let ctx = make_context();
        assert_eq!(ctx.proposal_summary, "Test proposal");
        assert_eq!(ctx.change_index, 0);
        assert_eq!(ctx.total_changes, 1);
    }
}
