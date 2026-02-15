//! # maple-worldline-langgen
//!
//! WorldLine Language Generation — synthesizes domain-specific languages
//! from observed usage patterns. The generated DSL includes grammar,
//! type system, semantic rules, parser, and compiler.
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────┐
//! │              LanguageEvolutionEngine                          │
//! │                                                               │
//! │  UsagePatterns ──→ DomainAnalyzer ──→ DomainSpec              │
//! │                         │                                     │
//! │                    ┌────┴────┐                                 │
//! │                    ▼         ▼                                 │
//! │            GrammarSynth  TypeSystemDesigner                    │
//! │                    │         │                                 │
//! │                    ▼         │                                 │
//! │            SemanticsEngine   │                                 │
//! │                    │         │                                 │
//! │               ┌────┴────┐   │                                 │
//! │               ▼         ▼   ▼                                 │
//! │        ParserGenerator  CompilerGenerator                     │
//! │               │              │                                │
//! │               └──────┬───────┘                                │
//! │                      ▼                                        │
//! │             GeneratedLanguage                                 │
//! │       (always Tier4SubstrateChange)                           │
//! └───────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Design Decisions
//!
//! - **Financial type safety**: Amount → Number coercion is `Forbidden`
//! - **Governance**: Language generation is **always** `Tier4SubstrateChange`
//! - **Provenance**: All stateful semantic rules include `RecordProvenance`
//! - **Bounded FIFO**: Records capped at `max_tracked_records`

#![deny(unsafe_code)]

pub mod compiler;
pub mod domain;
pub mod error;
pub mod evolution;
pub mod grammar;
pub mod parser;
pub mod semantics;
pub mod typesys;
pub mod types;

// ── Re-exports ───────────────────────────────────────────────────────

pub use compiler::{CompilerGenerator, SimulatedCompilerGenerator};
pub use domain::{DomainAnalyzer, SimulatedDomainAnalyzer};
pub use error::{LangGenError, LangGenResult};
pub use evolution::{LangGenRecord, LanguageEvolutionEngine};
pub use grammar::{
    ConflictResolution, GrammarSynthesizer, KeywordConflict, SimulatedGrammarSynthesizer,
};
pub use parser::{ParserGenerator, SimulatedParserGenerator};
pub use semantics::{SemanticsEngine, SimulatedSemanticsEngine};
pub use typesys::{SimulatedTypeSystemDesigner, TypeSystemDesigner};
pub use types::{
    Associativity, CoercionRule, CoercionSafety, CompilerSpec, CompilerTarget, ConceptProperty,
    ConceptRelationship, ConstraintType, DomainConcept, DomainConstraint, DomainSpec, DslType,
    EnforcementPhase, EvaluationStrategy, GeneratedLanguage, GrammarSpec, GrammarStyle,
    LangGenConfig, LangGenId, LangGenStatus, LangGenSummary, LanguageId, OperatorDef,
    OptimizationLevel, ParserSpec, ParserType, PrecedenceLevel, Production, ProductionId,
    PropertyType, RelationshipType, SemanticRule, SideEffect, TypeConstraint, TypeKind,
    TypeSystemSpec, UsagePattern,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine() -> LanguageEvolutionEngine {
        LanguageEvolutionEngine::new(
            Box::new(SimulatedDomainAnalyzer::new()),
            Box::new(SimulatedGrammarSynthesizer::new()),
            Box::new(SimulatedTypeSystemDesigner::new()),
            Box::new(SimulatedSemanticsEngine::new()),
            Box::new(SimulatedParserGenerator::new()),
            Box::new(SimulatedCompilerGenerator::new()),
            LangGenConfig::default(),
        )
    }

    fn sample_patterns() -> Vec<UsagePattern> {
        vec![
            UsagePattern {
                description: "Transfer between accounts".into(),
                frequency: 0.8,
                concepts: vec!["account".into(), "transfer".into()],
                operations: vec!["debit".into(), "credit".into()],
            },
            UsagePattern {
                description: "Batch settlement".into(),
                frequency: 0.3,
                concepts: vec!["settlement".into()],
                operations: vec!["settle".into()],
            },
        ]
    }

    #[test]
    fn simple_dsl_generation_e2e() {
        let mut engine = make_engine();
        let id = engine
            .generate("fin-settle-dsl", "1.0.0", &sample_patterns(), Some("finance"))
            .unwrap();
        let record = engine.find(&id).unwrap();
        assert!(matches!(record.status, LangGenStatus::Complete));

        let lang = record.language.as_ref().unwrap();
        assert_eq!(lang.name, "fin-settle-dsl");
        assert!(!lang.domain.concepts.is_empty());
        assert!(!lang.grammar.productions.is_empty());
        assert!(!lang.type_system.types.is_empty());
        assert!(!lang.semantic_rules.is_empty());
        assert!(!lang.parser.source_skeleton.is_empty());
        assert!(!lang.compiler.source_skeleton.is_empty());
    }

    #[test]
    fn financial_type_safety_e2e() {
        let mut engine = make_engine();
        let id = engine
            .generate("fin-dsl", "1.0.0", &sample_patterns(), None)
            .unwrap();
        let lang = engine.find(&id).unwrap().language.as_ref().unwrap();

        // Amount → Number coercion must be Forbidden
        let forbidden_coercions: Vec<_> = lang
            .type_system
            .coercion_rules
            .iter()
            .filter(|r| r.from_type == "Amount" && r.safety == CoercionSafety::Forbidden)
            .collect();
        assert!(
            forbidden_coercions.len() >= 2,
            "Amount→Decimal and Amount→Integer must both be forbidden"
        );

        // Cross-currency constraint must exist
        let cross_currency = lang
            .type_system
            .constraints
            .iter()
            .find(|c| c.name == "cross_currency_forbidden");
        assert!(cross_currency.is_some());
    }

    #[test]
    fn provenance_flows_e2e() {
        let mut engine = make_engine();
        let id = engine
            .generate("prov-dsl", "1.0.0", &sample_patterns(), None)
            .unwrap();
        let lang = engine.find(&id).unwrap().language.as_ref().unwrap();

        // All stateful semantic rules must have RecordProvenance
        for rule in &lang.semantic_rules {
            if rule.evaluation == EvaluationStrategy::Stateful
                || rule.evaluation == EvaluationStrategy::OperatorInvocation
            {
                assert!(
                    rule.side_effects.contains(&SideEffect::RecordProvenance),
                    "Rule for {:?} must have RecordProvenance",
                    rule.production_id
                );
            }
        }
    }

    #[test]
    fn evolution_produces_new_version() {
        let mut engine = make_engine();
        let id1 = engine
            .generate("evolve-lang", "1.0.0", &sample_patterns(), None)
            .unwrap();
        let id2 = engine
            .evolve(
                &LanguageId::from_name("evolve-lang"),
                "2.0.0",
                &sample_patterns(),
            )
            .unwrap();

        assert_ne!(id1.0, id2.0);
        let summary = engine.summary();
        assert_eq!(summary.total_generations, 2);
        assert_eq!(summary.successful_generations, 2);
        assert_eq!(summary.total_evolutions, 1);
    }

    #[test]
    fn governance_tier4_enforced() {
        let engine = make_engine();
        assert_eq!(
            engine.governance_tier(),
            maple_worldline_self_mod_gate::types::SelfModTier::Tier4SubstrateChange,
        );
    }

    #[test]
    fn public_types_accessible() {
        // Verify all public re-exports compile
        let _id = LangGenId::new();
        let _lid = LanguageId::from_name("test");
        let _pid = ProductionId::from_name("test");
        let _status = LangGenStatus::Started;
        let _style = GrammarStyle::Declarative;
        let _safety = CoercionSafety::Forbidden;
        let _kind = TypeKind::Financial;
        let _target = CompilerTarget::WlirInstructions;
        let _opt = OptimizationLevel::Basic;
        let _parser = ParserType::RecursiveDescent;
        let _eval = EvaluationStrategy::Pure;
        let _effect = SideEffect::RecordProvenance;
        let _phase = EnforcementPhase::TypeCheck;
        let _rel = RelationshipType::OneToMany;
        let _assoc = Associativity::Left;
        let _prop = PropertyType::Amount;
        let _constraint = ConstraintType::NonNegative;
        let _config = LangGenConfig::default();
        let _summary = LangGenSummary::default();
    }
}
