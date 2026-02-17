//! Parser generation for generated languages.
//!
//! Generates a `ParserSpec` (skeleton parser source code) from a
//! `GrammarSpec`. The parser type is chosen based on grammar style.

use crate::error::{LangGenError, LangGenResult};
use crate::types::{GrammarSpec, GrammarStyle, ParserSpec, ParserType};

// ── Parser Generator Trait ───────────────────────────────────────────

/// Trait for generating a parser from a grammar specification.
pub trait ParserGenerator: Send + Sync {
    /// Generate a parser from a grammar specification.
    fn generate(&self, grammar: &GrammarSpec) -> LangGenResult<ParserSpec>;

    /// Name of this generator implementation.
    fn name(&self) -> &str;
}

// ── Simulated Parser Generator ───────────────────────────────────────

/// Simulated parser generator for deterministic testing.
///
/// Generates skeleton parser source code with the parser type chosen
/// based on grammar style:
/// - Declarative → RecursiveDescent
/// - Expressive → Pratt
/// - Configuration → RecursiveDescent
/// - Scripting → Pratt
pub struct SimulatedParserGenerator {
    should_fail: bool,
}

impl SimulatedParserGenerator {
    /// Create a successful generator.
    pub fn new() -> Self {
        Self { should_fail: false }
    }

    /// Create a generator that always fails.
    pub fn failing() -> Self {
        Self { should_fail: true }
    }

    /// Choose parser type based on grammar style.
    fn parser_type_for_style(style: &GrammarStyle) -> ParserType {
        match style {
            GrammarStyle::Declarative => ParserType::RecursiveDescent,
            GrammarStyle::Expressive => ParserType::Pratt,
            GrammarStyle::Configuration => ParserType::RecursiveDescent,
            GrammarStyle::Scripting => ParserType::Pratt,
        }
    }

    /// Generate skeleton source code for the parser.
    fn skeleton_source(grammar: &GrammarSpec, parser_type: &ParserType) -> String {
        let mut source = String::new();
        source.push_str(&format!(
            "// Auto-generated {} parser for {} grammar\n",
            parser_type, grammar.style
        ));
        source.push_str(&format!(
            "// Total productions: {}\n\n",
            grammar.productions.len()
        ));

        // Entry point
        let entry = if grammar.productions.is_empty() {
            "program".to_string()
        } else {
            grammar.productions[0].name.clone()
        };

        source.push_str(&format!(
            "fn parse_{}(input: &str) -> Result<AST, ParseError> {{\n",
            entry
        ));
        source.push_str("    // Parser skeleton — production rules:\n");

        for prod in &grammar.productions {
            source.push_str(&format!("    //   {} -> {}\n", prod.name, prod.pattern));
        }

        source.push_str(
            "    Err(ParseError::from(\"parser skeleton generated; implement production rules\"))\n",
        );
        source.push_str("}\n");

        source
    }
}

impl Default for SimulatedParserGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ParserGenerator for SimulatedParserGenerator {
    fn generate(&self, grammar: &GrammarSpec) -> LangGenResult<ParserSpec> {
        if self.should_fail {
            return Err(LangGenError::ParserGenerationFailed(
                "simulated failure".into(),
            ));
        }

        if grammar.productions.is_empty() {
            return Err(LangGenError::ParserGenerationFailed(
                "no productions to parse".into(),
            ));
        }

        let parser_type = Self::parser_type_for_style(&grammar.style);
        let source_skeleton = Self::skeleton_source(grammar, &parser_type);
        let entry_rule = grammar.productions[0].name.clone();
        let total_rules = grammar.productions.len();

        Ok(ParserSpec {
            parser_type,
            source_skeleton,
            entry_rule,
            total_rules,
        })
    }

    fn name(&self) -> &str {
        "simulated-parser-generator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DomainAnalyzer, SimulatedDomainAnalyzer};
    use crate::grammar::{GrammarSynthesizer, SimulatedGrammarSynthesizer};
    use crate::types::UsagePattern;

    fn sample_grammar(style: GrammarStyle) -> GrammarSpec {
        let analyzer = SimulatedDomainAnalyzer::new();
        let patterns = vec![UsagePattern {
            description: "test".into(),
            frequency: 0.5,
            concepts: vec!["account".into()],
            operations: vec!["transfer".into()],
        }];
        let domain = analyzer.analyze(&patterns, None).unwrap();
        let synth = SimulatedGrammarSynthesizer::new();
        synth.synthesize(&domain, &style).unwrap()
    }

    #[test]
    fn generate_recursive_descent_parser() {
        let gen = SimulatedParserGenerator::new();
        let grammar = sample_grammar(GrammarStyle::Declarative);
        let parser = gen.generate(&grammar).unwrap();
        assert_eq!(parser.parser_type, ParserType::RecursiveDescent);
        assert!(!parser.source_skeleton.is_empty());
        assert_eq!(parser.total_rules, grammar.productions.len());
    }

    #[test]
    fn generate_pratt_parser() {
        let gen = SimulatedParserGenerator::new();
        let grammar = sample_grammar(GrammarStyle::Expressive);
        let parser = gen.generate(&grammar).unwrap();
        assert_eq!(parser.parser_type, ParserType::Pratt);
    }

    #[test]
    fn parser_has_entry_rule() {
        let gen = SimulatedParserGenerator::new();
        let grammar = sample_grammar(GrammarStyle::Declarative);
        let parser = gen.generate(&grammar).unwrap();
        assert!(!parser.entry_rule.is_empty());
    }

    #[test]
    fn skeleton_contains_productions() {
        let gen = SimulatedParserGenerator::new();
        let grammar = sample_grammar(GrammarStyle::Declarative);
        let parser = gen.generate(&grammar).unwrap();
        // Skeleton should reference production names
        assert!(parser.source_skeleton.contains("create_account"));
    }

    #[test]
    fn failing_generator() {
        let gen = SimulatedParserGenerator::failing();
        let grammar = sample_grammar(GrammarStyle::Declarative);
        let result = gen.generate(&grammar);
        assert!(result.is_err());
    }

    #[test]
    fn generator_name() {
        let gen = SimulatedParserGenerator::new();
        assert_eq!(gen.name(), "simulated-parser-generator");
    }
}
