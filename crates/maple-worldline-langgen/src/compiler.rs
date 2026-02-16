//! Compiler generation for generated languages.
//!
//! Generates a `CompilerSpec` (skeleton compiler source code) from a
//! `GrammarSpec` and `TypeSystemSpec`. Supports three targets:
//! WlirInstructions, OperatorCalls, and RustSource.

use crate::error::{LangGenError, LangGenResult};
use crate::types::{CompilerSpec, CompilerTarget, GrammarSpec, OptimizationLevel, TypeSystemSpec};

// ── Compiler Generator Trait ─────────────────────────────────────────

/// Trait for generating a compiler from grammar and type system specifications.
pub trait CompilerGenerator: Send + Sync {
    /// Generate a compiler targeting the specified output.
    fn generate(
        &self,
        grammar: &GrammarSpec,
        type_system: &TypeSystemSpec,
        target: &CompilerTarget,
        optimization: &OptimizationLevel,
    ) -> LangGenResult<CompilerSpec>;

    /// Name of this generator implementation.
    fn name(&self) -> &str;
}

// ── Simulated Compiler Generator ─────────────────────────────────────

/// Simulated compiler generator for deterministic testing.
///
/// Generates skeleton compiler source code for the target backend.
/// Pass count depends on optimization level.
pub struct SimulatedCompilerGenerator {
    should_fail: bool,
}

impl SimulatedCompilerGenerator {
    /// Create a successful generator.
    pub fn new() -> Self {
        Self { should_fail: false }
    }

    /// Create a generator that always fails.
    pub fn failing() -> Self {
        Self { should_fail: true }
    }

    /// Determine the number of compilation passes based on optimization level.
    fn passes_for_optimization(opt: &OptimizationLevel) -> usize {
        match opt {
            OptimizationLevel::None => 1,
            OptimizationLevel::Basic => 3,
            OptimizationLevel::Aggressive => 5,
        }
    }

    /// Generate skeleton compiler source code.
    fn skeleton_source(
        grammar: &GrammarSpec,
        type_system: &TypeSystemSpec,
        target: &CompilerTarget,
        passes: usize,
    ) -> String {
        let mut source = String::new();
        source.push_str(&format!(
            "// Auto-generated compiler targeting {}\n",
            target
        ));
        source.push_str(&format!(
            "// Productions: {}, Types: {}, Passes: {}\n\n",
            grammar.productions.len(),
            type_system.types.len(),
            passes,
        ));

        source.push_str("fn compile(ast: &AST) -> Result<Output, CompileError> {\n");

        match target {
            CompilerTarget::WlirInstructions => {
                source.push_str("    // Emit WLIR instructions\n");
                source.push_str("    let mut instructions = Vec::new();\n");
            }
            CompilerTarget::OperatorCalls => {
                source.push_str("    // Emit WorldLine operator calls\n");
                source.push_str("    let mut calls = Vec::new();\n");
            }
            CompilerTarget::RustSource => {
                source.push_str("    // Generate Rust source code\n");
                source.push_str("    let mut rust_code = String::new();\n");
            }
        }

        for (i, prod) in grammar.productions.iter().enumerate() {
            source.push_str(&format!("    // Rule {}: {}\n", i, prod.name));
        }

        source.push_str("    todo!(\"compiler implementation\")\n");
        source.push_str("}\n");

        source
    }
}

impl Default for SimulatedCompilerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl CompilerGenerator for SimulatedCompilerGenerator {
    fn generate(
        &self,
        grammar: &GrammarSpec,
        type_system: &TypeSystemSpec,
        target: &CompilerTarget,
        optimization: &OptimizationLevel,
    ) -> LangGenResult<CompilerSpec> {
        if self.should_fail {
            return Err(LangGenError::CompilerGenerationFailed(
                "simulated failure".into(),
            ));
        }

        if grammar.productions.is_empty() {
            return Err(LangGenError::CompilerGenerationFailed(
                "no productions to compile".into(),
            ));
        }

        let total_passes = Self::passes_for_optimization(optimization);
        let source_skeleton = Self::skeleton_source(grammar, type_system, target, total_passes);

        Ok(CompilerSpec {
            target: target.clone(),
            optimization: optimization.clone(),
            source_skeleton,
            total_passes,
        })
    }

    fn name(&self) -> &str {
        "simulated-compiler-generator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DomainAnalyzer, SimulatedDomainAnalyzer};
    use crate::grammar::{GrammarSynthesizer, SimulatedGrammarSynthesizer};
    use crate::types::{GrammarStyle, UsagePattern};
    use crate::typesys::{SimulatedTypeSystemDesigner, TypeSystemDesigner};

    fn sample_grammar() -> GrammarSpec {
        let analyzer = SimulatedDomainAnalyzer::new();
        let patterns = vec![UsagePattern {
            description: "test".into(),
            frequency: 0.5,
            concepts: vec!["account".into()],
            operations: vec!["transfer".into()],
        }];
        let domain = analyzer.analyze(&patterns, None).unwrap();
        let synth = SimulatedGrammarSynthesizer::new();
        synth
            .synthesize(&domain, &GrammarStyle::Declarative)
            .unwrap()
    }

    fn sample_type_system() -> TypeSystemSpec {
        let analyzer = SimulatedDomainAnalyzer::new();
        let patterns = vec![UsagePattern {
            description: "test".into(),
            frequency: 0.5,
            concepts: vec!["account".into()],
            operations: vec!["transfer".into()],
        }];
        let domain = analyzer.analyze(&patterns, None).unwrap();
        let designer = SimulatedTypeSystemDesigner::new();
        designer.design(&domain).unwrap()
    }

    #[test]
    fn generate_wlir_compiler() {
        let gen = SimulatedCompilerGenerator::new();
        let grammar = sample_grammar();
        let ts = sample_type_system();
        let compiler = gen
            .generate(
                &grammar,
                &ts,
                &CompilerTarget::WlirInstructions,
                &OptimizationLevel::Basic,
            )
            .unwrap();
        assert_eq!(compiler.target, CompilerTarget::WlirInstructions);
        assert_eq!(compiler.total_passes, 3);
    }

    #[test]
    fn generate_operator_calls_compiler() {
        let gen = SimulatedCompilerGenerator::new();
        let grammar = sample_grammar();
        let ts = sample_type_system();
        let compiler = gen
            .generate(
                &grammar,
                &ts,
                &CompilerTarget::OperatorCalls,
                &OptimizationLevel::None,
            )
            .unwrap();
        assert_eq!(compiler.target, CompilerTarget::OperatorCalls);
        assert_eq!(compiler.total_passes, 1);
    }

    #[test]
    fn aggressive_optimization_more_passes() {
        let gen = SimulatedCompilerGenerator::new();
        let grammar = sample_grammar();
        let ts = sample_type_system();
        let compiler = gen
            .generate(
                &grammar,
                &ts,
                &CompilerTarget::WlirInstructions,
                &OptimizationLevel::Aggressive,
            )
            .unwrap();
        assert_eq!(compiler.total_passes, 5);
    }

    #[test]
    fn skeleton_contains_target_info() {
        let gen = SimulatedCompilerGenerator::new();
        let grammar = sample_grammar();
        let ts = sample_type_system();
        let compiler = gen
            .generate(
                &grammar,
                &ts,
                &CompilerTarget::RustSource,
                &OptimizationLevel::Basic,
            )
            .unwrap();
        assert!(compiler.source_skeleton.contains("Rust"));
    }

    #[test]
    fn failing_generator() {
        let gen = SimulatedCompilerGenerator::failing();
        let grammar = sample_grammar();
        let ts = sample_type_system();
        let result = gen.generate(
            &grammar,
            &ts,
            &CompilerTarget::WlirInstructions,
            &OptimizationLevel::None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn generator_name() {
        let gen = SimulatedCompilerGenerator::new();
        assert_eq!(gen.name(), "simulated-compiler-generator");
    }
}
