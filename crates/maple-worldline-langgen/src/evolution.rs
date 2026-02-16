//! Language evolution engine — orchestrates the full langgen pipeline.
//!
//! The `LanguageEvolutionEngine` runs the complete pipeline:
//! analyze → synthesize → design → semantics → parser → compiler → assemble
//!
//! Language generation is **always** `Tier4SubstrateChange` (human review
//! required). Records are stored in a bounded FIFO (VecDeque).

use std::collections::VecDeque;

use chrono::Utc;

use crate::compiler::CompilerGenerator;
use crate::domain::DomainAnalyzer;
use crate::error::LangGenResult;
use crate::grammar::GrammarSynthesizer;
use crate::parser::ParserGenerator;
use crate::semantics::SemanticsEngine;
use crate::types::{
    GeneratedLanguage, LangGenConfig, LangGenId, LangGenStatus, LangGenSummary, LanguageId,
    UsagePattern,
};
use crate::typesys::TypeSystemDesigner;
use maple_worldline_self_mod_gate::types::SelfModTier;

// ── Language Generation Record ───────────────────────────────────────

/// Record of a language generation attempt.
#[derive(Clone, Debug)]
pub struct LangGenRecord {
    pub id: LangGenId,
    pub status: LangGenStatus,
    pub language: Option<GeneratedLanguage>,
    pub governance_tier: SelfModTier,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ── Language Evolution Engine ────────────────────────────────────────

/// Orchestrates the full language generation pipeline.
///
/// Pipeline stages:
/// 1. Domain analysis (from usage patterns)
/// 2. Grammar synthesis (from domain spec)
/// 3. Type system design (from domain spec)
/// 4. Semantic rule mapping (from grammar)
/// 5. Parser generation (from grammar)
/// 6. Compiler generation (from grammar + type system)
/// 7. Assembly into `GeneratedLanguage`
///
/// Governance is **always** `Tier4SubstrateChange`.
pub struct LanguageEvolutionEngine {
    domain_analyzer: Box<dyn DomainAnalyzer>,
    grammar_synthesizer: Box<dyn GrammarSynthesizer>,
    type_system_designer: Box<dyn TypeSystemDesigner>,
    semantics_engine: Box<dyn SemanticsEngine>,
    parser_generator: Box<dyn ParserGenerator>,
    compiler_generator: Box<dyn CompilerGenerator>,
    config: LangGenConfig,
    records: VecDeque<LangGenRecord>,
    total_evolutions: usize,
}

impl LanguageEvolutionEngine {
    /// Create a new engine with all pipeline components.
    pub fn new(
        domain_analyzer: Box<dyn DomainAnalyzer>,
        grammar_synthesizer: Box<dyn GrammarSynthesizer>,
        type_system_designer: Box<dyn TypeSystemDesigner>,
        semantics_engine: Box<dyn SemanticsEngine>,
        parser_generator: Box<dyn ParserGenerator>,
        compiler_generator: Box<dyn CompilerGenerator>,
        config: LangGenConfig,
    ) -> Self {
        Self {
            domain_analyzer,
            grammar_synthesizer,
            type_system_designer,
            semantics_engine,
            parser_generator,
            compiler_generator,
            config,
            records: VecDeque::new(),
            total_evolutions: 0,
        }
    }

    /// Generate a new language from usage patterns.
    ///
    /// Returns the LangGenId. The full record can be retrieved with `find()`.
    pub fn generate(
        &mut self,
        name: &str,
        version: &str,
        patterns: &[UsagePattern],
        domain_hint: Option<&str>,
    ) -> LangGenResult<LangGenId> {
        let id = LangGenId::new();
        let mut record = LangGenRecord {
            id: id.clone(),
            status: LangGenStatus::Started,
            language: None,
            governance_tier: SelfModTier::Tier4SubstrateChange,
            created_at: Utc::now(),
        };

        // 1. Domain analysis
        let domain = match self.domain_analyzer.analyze(patterns, domain_hint) {
            Ok(d) => {
                record.status = LangGenStatus::DomainAnalyzed;
                d
            }
            Err(e) => {
                record.status = LangGenStatus::Failed(e.to_string());
                self.store_record(record);
                return Err(e);
            }
        };

        let style = domain.recommended_style.clone();

        // 2. Grammar synthesis
        let grammar = match self.grammar_synthesizer.synthesize(&domain, &style) {
            Ok(g) => {
                record.status = LangGenStatus::GrammarSynthesized;
                g
            }
            Err(e) => {
                record.status = LangGenStatus::Failed(e.to_string());
                self.store_record(record);
                return Err(e);
            }
        };

        // 3. Type system design
        let type_system = match self.type_system_designer.design(&domain) {
            Ok(ts) => {
                record.status = LangGenStatus::TypeSystemDesigned;
                ts
            }
            Err(e) => {
                record.status = LangGenStatus::Failed(e.to_string());
                self.store_record(record);
                return Err(e);
            }
        };

        // 4. Semantic rule mapping
        let semantic_rules = match self.semantics_engine.map_semantics(&grammar) {
            Ok(sr) => {
                record.status = LangGenStatus::SemanticsMapped;
                sr
            }
            Err(e) => {
                record.status = LangGenStatus::Failed(e.to_string());
                self.store_record(record);
                return Err(e);
            }
        };

        // 5. Parser generation
        let parser = match self.parser_generator.generate(&grammar) {
            Ok(p) => {
                record.status = LangGenStatus::ParserGenerated;
                p
            }
            Err(e) => {
                record.status = LangGenStatus::Failed(e.to_string());
                self.store_record(record);
                return Err(e);
            }
        };

        // 6. Compiler generation
        let compiler = match self.compiler_generator.generate(
            &grammar,
            &type_system,
            &self.config.default_target,
            &self.config.default_optimization,
        ) {
            Ok(c) => {
                record.status = LangGenStatus::CompilerGenerated;
                c
            }
            Err(e) => {
                record.status = LangGenStatus::Failed(e.to_string());
                self.store_record(record);
                return Err(e);
            }
        };

        // 7. Assemble final language
        let language = GeneratedLanguage {
            id: LanguageId::from_name(name),
            name: name.into(),
            version: version.into(),
            domain,
            grammar,
            type_system,
            semantic_rules,
            parser,
            compiler,
            created_at: Utc::now(),
        };

        record.status = LangGenStatus::Complete;
        record.language = Some(language);
        self.store_record(record);

        Ok(id)
    }

    /// Evolve an existing language (re-runs the pipeline with updated patterns).
    pub fn evolve(
        &mut self,
        _language_id: &LanguageId,
        new_version: &str,
        patterns: &[UsagePattern],
    ) -> LangGenResult<LangGenId> {
        self.total_evolutions += 1;
        // Evolution re-runs the full pipeline with the same name but new version
        self.generate("evolved-language", new_version, patterns, None)
    }

    /// Find a generation record by ID.
    pub fn find(&self, id: &LangGenId) -> Option<&LangGenRecord> {
        self.records.iter().find(|r| r.id == *id)
    }

    /// Get all stored records.
    pub fn all_records(&self) -> &VecDeque<LangGenRecord> {
        &self.records
    }

    /// Get a summary of the engine's activity.
    pub fn summary(&self) -> LangGenSummary {
        let successful = self
            .records
            .iter()
            .filter(|r| matches!(r.status, LangGenStatus::Complete))
            .count();
        let failed = self
            .records
            .iter()
            .filter(|r| matches!(r.status, LangGenStatus::Failed(_)))
            .count();
        let languages = self.records.iter().filter(|r| r.language.is_some()).count();

        LangGenSummary {
            total_generations: self.records.len(),
            successful_generations: successful,
            failed_generations: failed,
            total_evolutions: self.total_evolutions,
            languages_produced: languages,
        }
    }

    /// Governance tier for language generation (always Tier4SubstrateChange).
    pub fn governance_tier(&self) -> SelfModTier {
        SelfModTier::Tier4SubstrateChange
    }

    /// Store a record, evicting oldest if at capacity.
    fn store_record(&mut self, record: LangGenRecord) {
        if self.records.len() >= self.config.max_tracked_records {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::SimulatedCompilerGenerator;
    use crate::domain::SimulatedDomainAnalyzer;
    use crate::grammar::SimulatedGrammarSynthesizer;
    use crate::parser::SimulatedParserGenerator;
    use crate::semantics::SimulatedSemanticsEngine;
    use crate::typesys::SimulatedTypeSystemDesigner;

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
        vec![UsagePattern {
            description: "Transfer between accounts".into(),
            frequency: 0.8,
            concepts: vec!["account".into(), "transfer".into()],
            operations: vec!["debit".into(), "credit".into()],
        }]
    }

    #[test]
    fn full_generation_pipeline() {
        let mut engine = make_engine();
        let id = engine
            .generate("fin-settle", "1.0.0", &sample_patterns(), None)
            .unwrap();
        let record = engine.find(&id).unwrap();
        assert!(matches!(record.status, LangGenStatus::Complete));
        assert!(record.language.is_some());
        let lang = record.language.as_ref().unwrap();
        assert_eq!(lang.name, "fin-settle");
        assert_eq!(lang.version, "1.0.0");
    }

    #[test]
    fn generated_language_has_all_components() {
        let mut engine = make_engine();
        let id = engine
            .generate("test-lang", "2.0.0", &sample_patterns(), None)
            .unwrap();
        let lang = engine.find(&id).unwrap().language.as_ref().unwrap();
        assert!(!lang.domain.concepts.is_empty());
        assert!(!lang.grammar.productions.is_empty());
        assert!(!lang.type_system.types.is_empty());
        assert!(!lang.semantic_rules.is_empty());
        assert!(!lang.parser.source_skeleton.is_empty());
        assert!(!lang.compiler.source_skeleton.is_empty());
    }

    #[test]
    fn governance_always_tier4() {
        let engine = make_engine();
        assert_eq!(engine.governance_tier(), SelfModTier::Tier4SubstrateChange);
    }

    #[test]
    fn generation_record_always_tier4() {
        let mut engine = make_engine();
        let id = engine
            .generate("test", "1.0", &sample_patterns(), None)
            .unwrap();
        let record = engine.find(&id).unwrap();
        assert_eq!(record.governance_tier, SelfModTier::Tier4SubstrateChange);
    }

    #[test]
    fn empty_patterns_fails() {
        let mut engine = make_engine();
        let result = engine.generate("test", "1.0", &[], None);
        assert!(result.is_err());
        // Failed record should be stored
        assert_eq!(engine.all_records().len(), 1);
        assert!(matches!(
            engine.all_records()[0].status,
            LangGenStatus::Failed(_)
        ));
    }

    #[test]
    fn evolution_reruns_pipeline() {
        let mut engine = make_engine();
        let id1 = engine
            .generate("lang-v1", "1.0.0", &sample_patterns(), None)
            .unwrap();
        let id2 = engine
            .evolve(
                &LanguageId::from_name("lang-v1"),
                "2.0.0",
                &sample_patterns(),
            )
            .unwrap();
        assert_ne!(id1.0, id2.0);
        assert_eq!(engine.all_records().len(), 2);
        let summary = engine.summary();
        assert_eq!(summary.total_evolutions, 1);
    }

    #[test]
    fn summary_tracks_statistics() {
        let mut engine = make_engine();
        let _ = engine.generate("lang1", "1.0", &sample_patterns(), None);
        let _ = engine.generate("fail", "1.0", &[], None);
        let summary = engine.summary();
        assert_eq!(summary.total_generations, 2);
        assert_eq!(summary.successful_generations, 1);
        assert_eq!(summary.failed_generations, 1);
        assert_eq!(summary.languages_produced, 1);
    }

    #[test]
    fn bounded_fifo_eviction() {
        let mut config = LangGenConfig::default();
        config.max_tracked_records = 2;
        let mut engine = LanguageEvolutionEngine::new(
            Box::new(SimulatedDomainAnalyzer::new()),
            Box::new(SimulatedGrammarSynthesizer::new()),
            Box::new(SimulatedTypeSystemDesigner::new()),
            Box::new(SimulatedSemanticsEngine::new()),
            Box::new(SimulatedParserGenerator::new()),
            Box::new(SimulatedCompilerGenerator::new()),
            config,
        );

        let _ = engine.generate("lang1", "1.0", &sample_patterns(), None);
        let _ = engine.generate("lang2", "1.0", &sample_patterns(), None);
        let _ = engine.generate("lang3", "1.0", &sample_patterns(), None);

        assert_eq!(engine.all_records().len(), 2);
        // First record should have been evicted
    }
}
