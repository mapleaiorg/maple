//! Semantic rule mapping for generated languages.
//!
//! Maps grammar productions to semantic rules defining evaluation
//! strategy and side effects. Stateful rules always include
//! `RecordProvenance` side effect for full auditability.

use crate::error::{LangGenError, LangGenResult};
use crate::types::{EvaluationStrategy, GrammarSpec, SemanticRule, SideEffect};

// ── Semantics Engine Trait ───────────────────────────────────────────

/// Trait for mapping grammar productions to semantic rules.
pub trait SemanticsEngine: Send + Sync {
    /// Map grammar productions to semantic rules.
    fn map_semantics(&self, grammar: &GrammarSpec) -> LangGenResult<Vec<SemanticRule>>;

    /// Name of this engine implementation.
    fn name(&self) -> &str;
}

// ── Simulated Semantics Engine ───────────────────────────────────────

/// Simulated semantics engine for deterministic testing.
///
/// Maps productions to rules based on naming conventions:
/// - "create_*" and "*_stmt" → Stateful (EmitEvent + RecordProvenance)
/// - "query_*" and "*_pipe" → Pure
/// - "*_call" → OperatorInvocation (InvokeOperator + RecordProvenance)
/// - Default → Pure
///
/// **Invariant**: All Stateful rules include RecordProvenance side effect.
pub struct SimulatedSemanticsEngine {
    should_fail: bool,
}

impl SimulatedSemanticsEngine {
    /// Create a successful engine.
    pub fn new() -> Self {
        Self { should_fail: false }
    }

    /// Create an engine that always fails.
    pub fn failing() -> Self {
        Self { should_fail: true }
    }

    /// Determine evaluation strategy from production name.
    fn classify_production(name: &str) -> (EvaluationStrategy, Vec<SideEffect>) {
        if name.starts_with("create_") || name.ends_with("_stmt") || name.ends_with("_config") {
            // Stateful: modifies state, emits events, records provenance
            (
                EvaluationStrategy::Stateful,
                vec![
                    SideEffect::EmitEvent(format!("{}_executed", name)),
                    SideEffect::RecordProvenance,
                ],
            )
        } else if name.ends_with("_call") {
            // Operator invocation: delegates to WorldLine operator
            (
                EvaluationStrategy::OperatorInvocation,
                vec![
                    SideEffect::InvokeOperator(name.trim_end_matches("_call").into()),
                    SideEffect::RecordProvenance,
                ],
            )
        } else {
            // Pure: query, pipe, etc.
            (EvaluationStrategy::Pure, vec![])
        }
    }
}

impl Default for SimulatedSemanticsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticsEngine for SimulatedSemanticsEngine {
    fn map_semantics(&self, grammar: &GrammarSpec) -> LangGenResult<Vec<SemanticRule>> {
        if self.should_fail {
            return Err(LangGenError::SemanticMappingFailed(
                "simulated failure".into(),
            ));
        }

        if grammar.productions.is_empty() {
            return Err(LangGenError::SemanticMappingFailed(
                "no productions to map".into(),
            ));
        }

        let rules: Vec<SemanticRule> = grammar
            .productions
            .iter()
            .map(|p| {
                let (evaluation, side_effects) = Self::classify_production(&p.name);
                SemanticRule {
                    production_id: p.id.clone(),
                    evaluation,
                    side_effects,
                    description: format!("Semantic rule for {}", p.name),
                }
            })
            .collect();

        Ok(rules)
    }

    fn name(&self) -> &str {
        "simulated-semantics-engine"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DomainAnalyzer, SimulatedDomainAnalyzer};
    use crate::grammar::{GrammarSynthesizer, SimulatedGrammarSynthesizer};
    use crate::types::{GrammarStyle, UsagePattern};

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

    #[test]
    fn map_declarative_semantics() {
        let engine = SimulatedSemanticsEngine::new();
        let grammar = sample_grammar();
        let rules = engine.map_semantics(&grammar).unwrap();
        assert_eq!(rules.len(), grammar.productions.len());
    }

    #[test]
    fn create_productions_are_stateful() {
        let engine = SimulatedSemanticsEngine::new();
        let grammar = sample_grammar();
        let rules = engine.map_semantics(&grammar).unwrap();
        let create_rules: Vec<_> = rules
            .iter()
            .filter(|r| r.evaluation == EvaluationStrategy::Stateful)
            .collect();
        assert!(!create_rules.is_empty());
    }

    #[test]
    fn stateful_rules_have_provenance() {
        let engine = SimulatedSemanticsEngine::new();
        let grammar = sample_grammar();
        let rules = engine.map_semantics(&grammar).unwrap();
        for rule in &rules {
            if rule.evaluation == EvaluationStrategy::Stateful
                || rule.evaluation == EvaluationStrategy::OperatorInvocation
            {
                assert!(
                    rule.side_effects.contains(&SideEffect::RecordProvenance),
                    "Stateful rule {} must have RecordProvenance",
                    rule.description,
                );
            }
        }
    }

    #[test]
    fn query_productions_are_pure() {
        let engine = SimulatedSemanticsEngine::new();
        let grammar = sample_grammar();
        let rules = engine.map_semantics(&grammar).unwrap();
        let query_rules: Vec<_> = rules
            .iter()
            .filter(|r| r.evaluation == EvaluationStrategy::Pure)
            .collect();
        assert!(!query_rules.is_empty());
    }

    #[test]
    fn failing_engine() {
        let engine = SimulatedSemanticsEngine::failing();
        let grammar = sample_grammar();
        let result = engine.map_semantics(&grammar);
        assert!(result.is_err());
    }

    #[test]
    fn engine_name() {
        let engine = SimulatedSemanticsEngine::new();
        assert_eq!(engine.name(), "simulated-semantics-engine");
    }
}
