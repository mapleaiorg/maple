//! Grammar synthesis for language generation.
//!
//! Synthesizes a `GrammarSpec` from a `DomainSpec`, producing production
//! rules, operator definitions, and precedence levels for each domain concept.
//! Detects and resolves keyword conflicts.

use crate::error::{LangGenError, LangGenResult};
use crate::types::{
    Associativity, DomainSpec, GrammarSpec, GrammarStyle, OperatorDef, PrecedenceLevel, Production,
    ProductionId,
};

// ── Keyword Conflict ─────────────────────────────────────────────────

/// A detected conflict between keywords in the grammar.
#[derive(Clone, Debug)]
pub struct KeywordConflict {
    pub keyword: String,
    pub conflicting_with: String,
    pub resolution: ConflictResolution,
}

/// How a keyword conflict was resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Renamed the keyword.
    Renamed(String),
    /// Prefixed with a namespace.
    Prefixed(String),
    /// Dropped the conflicting keyword.
    Dropped,
}

impl std::fmt::Display for ConflictResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Renamed(new) => write!(f, "renamed to '{}'", new),
            Self::Prefixed(prefix) => write!(f, "prefixed with '{}'", prefix),
            Self::Dropped => write!(f, "dropped"),
        }
    }
}

// ── Grammar Synthesizer Trait ────────────────────────────────────────

/// Trait for synthesizing a grammar from a domain specification.
pub trait GrammarSynthesizer: Send + Sync {
    /// Synthesize a grammar from a domain spec and style.
    fn synthesize(
        &self,
        domain: &DomainSpec,
        style: &GrammarStyle,
    ) -> LangGenResult<GrammarSpec>;

    /// Detect keyword conflicts in a set of keywords.
    fn detect_conflicts(&self, keywords: &[String]) -> Vec<KeywordConflict>;

    /// Name of this synthesizer implementation.
    fn name(&self) -> &str;
}

// ── Simulated Grammar Synthesizer ────────────────────────────────────

/// Simulated grammar synthesizer for deterministic testing.
///
/// Generates productions for each domain concept based on the grammar style.
pub struct SimulatedGrammarSynthesizer {
    should_fail: bool,
}

impl SimulatedGrammarSynthesizer {
    /// Create a successful synthesizer.
    pub fn new() -> Self {
        Self { should_fail: false }
    }

    /// Create a synthesizer that always fails.
    pub fn failing() -> Self {
        Self { should_fail: true }
    }

    /// Generate productions for a concept based on style.
    fn productions_for_concept(concept_name: &str, style: &GrammarStyle) -> Vec<Production> {
        let lower = concept_name.to_lowercase();
        match style {
            GrammarStyle::Declarative => vec![
                Production {
                    id: ProductionId::from_name(&format!("create_{}", lower)),
                    name: format!("create_{}", lower),
                    pattern: format!("CREATE {} <properties>", concept_name),
                    description: format!("Create a new {}", lower),
                    concept: concept_name.into(),
                },
                Production {
                    id: ProductionId::from_name(&format!("query_{}", lower)),
                    name: format!("query_{}", lower),
                    pattern: format!("SELECT {} WHERE <condition>", concept_name),
                    description: format!("Query {}s by condition", lower),
                    concept: concept_name.into(),
                },
            ],
            GrammarStyle::Expressive => vec![
                Production {
                    id: ProductionId::from_name(&format!("{}_pipe", lower)),
                    name: format!("{}_pipe", lower),
                    pattern: format!("{} |> <transform>", lower),
                    description: format!("Transform a {} through a pipeline", lower),
                    concept: concept_name.into(),
                },
            ],
            GrammarStyle::Configuration => vec![
                Production {
                    id: ProductionId::from_name(&format!("{}_config", lower)),
                    name: format!("{}_config", lower),
                    pattern: format!("{} {{ <key>: <value>, ... }}", lower),
                    description: format!("Configure a {}", lower),
                    concept: concept_name.into(),
                },
            ],
            GrammarStyle::Scripting => vec![
                Production {
                    id: ProductionId::from_name(&format!("{}_stmt", lower)),
                    name: format!("{}_stmt", lower),
                    pattern: format!("let {} = <expr>;", lower),
                    description: format!("Assign a {} to a variable", lower),
                    concept: concept_name.into(),
                },
                Production {
                    id: ProductionId::from_name(&format!("{}_call", lower)),
                    name: format!("{}_call", lower),
                    pattern: format!("{}.method(<args>)", lower),
                    description: format!("Call a method on a {}", lower),
                    concept: concept_name.into(),
                },
            ],
        }
    }

    /// Generate default operators.
    fn default_operators() -> Vec<OperatorDef> {
        vec![
            OperatorDef {
                symbol: "+".into(),
                name: "add".into(),
                arity: 2,
                precedence: 10,
                associativity: Associativity::Left,
            },
            OperatorDef {
                symbol: "-".into(),
                name: "subtract".into(),
                arity: 2,
                precedence: 10,
                associativity: Associativity::Left,
            },
            OperatorDef {
                symbol: "*".into(),
                name: "multiply".into(),
                arity: 2,
                precedence: 20,
                associativity: Associativity::Left,
            },
            OperatorDef {
                symbol: "==".into(),
                name: "equals".into(),
                arity: 2,
                precedence: 5,
                associativity: Associativity::None,
            },
        ]
    }

    /// Generate default precedence levels.
    fn default_precedence_levels() -> Vec<PrecedenceLevel> {
        vec![
            PrecedenceLevel {
                level: 5,
                operators: vec!["==".into()],
                associativity: Associativity::None,
            },
            PrecedenceLevel {
                level: 10,
                operators: vec!["+".into(), "-".into()],
                associativity: Associativity::Left,
            },
            PrecedenceLevel {
                level: 20,
                operators: vec!["*".into()],
                associativity: Associativity::Left,
            },
        ]
    }

    /// Generate style-specific keywords.
    fn keywords_for_style(style: &GrammarStyle) -> Vec<String> {
        match style {
            GrammarStyle::Declarative => {
                vec!["CREATE".into(), "SELECT".into(), "WHERE".into(), "SET".into(), "FROM".into()]
            }
            GrammarStyle::Expressive => {
                vec!["pipe".into(), "map".into(), "filter".into(), "reduce".into()]
            }
            GrammarStyle::Configuration => {
                vec!["config".into(), "set".into(), "default".into(), "override".into()]
            }
            GrammarStyle::Scripting => {
                vec!["let".into(), "if".into(), "else".into(), "for".into(), "fn".into()]
            }
        }
    }
}

impl Default for SimulatedGrammarSynthesizer {
    fn default() -> Self {
        Self::new()
    }
}

impl GrammarSynthesizer for SimulatedGrammarSynthesizer {
    fn synthesize(
        &self,
        domain: &DomainSpec,
        style: &GrammarStyle,
    ) -> LangGenResult<GrammarSpec> {
        if self.should_fail {
            return Err(LangGenError::GrammarSynthesisFailed(
                "simulated failure".into(),
            ));
        }

        if domain.concepts.is_empty() {
            return Err(LangGenError::GrammarSynthesisFailed(
                "domain has no concepts".into(),
            ));
        }

        let productions: Vec<Production> = domain
            .concepts
            .iter()
            .flat_map(|c| Self::productions_for_concept(&c.name, style))
            .collect();

        let keywords = Self::keywords_for_style(style);

        Ok(GrammarSpec {
            style: style.clone(),
            productions,
            operators: Self::default_operators(),
            precedence_levels: Self::default_precedence_levels(),
            keywords,
        })
    }

    fn detect_conflicts(&self, keywords: &[String]) -> Vec<KeywordConflict> {
        let mut conflicts = Vec::new();
        let reserved = ["type", "fn", "let", "struct", "impl"];

        for kw in keywords {
            let lower = kw.to_lowercase();
            for &r in &reserved {
                if lower == r && kw != r {
                    conflicts.push(KeywordConflict {
                        keyword: kw.clone(),
                        conflicting_with: r.into(),
                        resolution: ConflictResolution::Prefixed("dsl_".into()),
                    });
                }
            }
        }
        conflicts
    }

    fn name(&self) -> &str {
        "simulated-grammar-synthesizer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DomainAnalyzer, SimulatedDomainAnalyzer};
    use crate::types::UsagePattern;

    fn sample_domain() -> DomainSpec {
        let analyzer = SimulatedDomainAnalyzer::new();
        let patterns = vec![UsagePattern {
            description: "test".into(),
            frequency: 0.5,
            concepts: vec!["account".into()],
            operations: vec!["transfer".into()],
        }];
        analyzer.analyze(&patterns, None).unwrap()
    }

    #[test]
    fn synthesize_declarative_grammar() {
        let synth = SimulatedGrammarSynthesizer::new();
        let domain = sample_domain();
        let grammar = synth.synthesize(&domain, &GrammarStyle::Declarative).unwrap();
        assert_eq!(grammar.style, GrammarStyle::Declarative);
        // 3 concepts × 2 productions each = 6
        assert_eq!(grammar.productions.len(), 6);
        assert!(!grammar.keywords.is_empty());
    }

    #[test]
    fn synthesize_expressive_grammar() {
        let synth = SimulatedGrammarSynthesizer::new();
        let domain = sample_domain();
        let grammar = synth.synthesize(&domain, &GrammarStyle::Expressive).unwrap();
        assert_eq!(grammar.style, GrammarStyle::Expressive);
        // 3 concepts × 1 production each = 3
        assert_eq!(grammar.productions.len(), 3);
    }

    #[test]
    fn synthesize_scripting_grammar() {
        let synth = SimulatedGrammarSynthesizer::new();
        let domain = sample_domain();
        let grammar = synth.synthesize(&domain, &GrammarStyle::Scripting).unwrap();
        assert_eq!(grammar.style, GrammarStyle::Scripting);
        // 3 concepts × 2 productions each = 6
        assert_eq!(grammar.productions.len(), 6);
    }

    #[test]
    fn grammar_has_operators() {
        let synth = SimulatedGrammarSynthesizer::new();
        let domain = sample_domain();
        let grammar = synth.synthesize(&domain, &GrammarStyle::Declarative).unwrap();
        assert_eq!(grammar.operators.len(), 4);
        let add = grammar.operators.iter().find(|o| o.name == "add").unwrap();
        assert_eq!(add.symbol, "+");
        assert_eq!(add.arity, 2);
    }

    #[test]
    fn grammar_has_precedence_levels() {
        let synth = SimulatedGrammarSynthesizer::new();
        let domain = sample_domain();
        let grammar = synth.synthesize(&domain, &GrammarStyle::Declarative).unwrap();
        assert_eq!(grammar.precedence_levels.len(), 3);
        // Multiplication has higher precedence than addition
        let mul_level = grammar.precedence_levels.iter().find(|p| p.operators.contains(&"*".into())).unwrap();
        let add_level = grammar.precedence_levels.iter().find(|p| p.operators.contains(&"+".into())).unwrap();
        assert!(mul_level.level > add_level.level);
    }

    #[test]
    fn failing_synthesizer() {
        let synth = SimulatedGrammarSynthesizer::failing();
        let domain = sample_domain();
        let result = synth.synthesize(&domain, &GrammarStyle::Declarative);
        assert!(result.is_err());
    }

    #[test]
    fn empty_domain_fails() {
        let synth = SimulatedGrammarSynthesizer::new();
        let empty = DomainSpec {
            name: "empty".into(),
            description: "".into(),
            concepts: vec![],
            relationships: vec![],
            constraints: vec![],
            recommended_style: GrammarStyle::Declarative,
        };
        let result = synth.synthesize(&empty, &GrammarStyle::Declarative);
        assert!(result.is_err());
    }

    #[test]
    fn conflict_detection() {
        let synth = SimulatedGrammarSynthesizer::new();
        let keywords = vec!["FN".into(), "select".into(), "TYPE".into()];
        let conflicts = synth.detect_conflicts(&keywords);
        // "FN" conflicts with reserved "fn", "TYPE" with "type"
        assert_eq!(conflicts.len(), 2);
    }

    #[test]
    fn conflict_resolution_display() {
        assert!(ConflictResolution::Renamed("foo".into()).to_string().contains("foo"));
        assert!(ConflictResolution::Prefixed("dsl_".into()).to_string().contains("dsl_"));
        assert_eq!(ConflictResolution::Dropped.to_string(), "dropped");
    }

    #[test]
    fn synthesizer_name() {
        let synth = SimulatedGrammarSynthesizer::new();
        assert_eq!(synth.name(), "simulated-grammar-synthesizer");
    }
}
