//! Core types for the language generation operator.
//!
//! Defines all identifiers, specifications, and intermediate structures
//! used across the language generation pipeline: domain analysis,
//! grammar synthesis, type system design, semantic mapping,
//! parser/compiler generation, and the final `GeneratedLanguage` output.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Identifiers ──────────────────────────────────────────────────────

/// Unique identifier for a language generation run.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LangGenId(pub String);

impl LangGenId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for LangGenId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for LangGenId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "langgen:{}", self.0)
    }
}

/// Unique identifier for a generated language.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LanguageId(pub String);

impl LanguageId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_name(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl Default for LanguageId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for LanguageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "lang:{}", self.0)
    }
}

/// Unique identifier for a grammar production rule.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProductionId(pub String);

impl ProductionId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_name(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl Default for ProductionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProductionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "prod:{}", self.0)
    }
}

// ── Status ───────────────────────────────────────────────────────────

/// Status of a language generation run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LangGenStatus {
    /// Pipeline started.
    Started,
    /// Domain analysis complete.
    DomainAnalyzed,
    /// Grammar synthesized.
    GrammarSynthesized,
    /// Type system designed.
    TypeSystemDesigned,
    /// Semantic rules mapped.
    SemanticsMapped,
    /// Parser generated.
    ParserGenerated,
    /// Compiler generated.
    CompilerGenerated,
    /// Language fully assembled.
    Complete,
    /// Generation failed at some stage.
    Failed(String),
}

impl std::fmt::Display for LangGenStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Started => write!(f, "started"),
            Self::DomainAnalyzed => write!(f, "domain-analyzed"),
            Self::GrammarSynthesized => write!(f, "grammar-synthesized"),
            Self::TypeSystemDesigned => write!(f, "type-system-designed"),
            Self::SemanticsMapped => write!(f, "semantics-mapped"),
            Self::ParserGenerated => write!(f, "parser-generated"),
            Self::CompilerGenerated => write!(f, "compiler-generated"),
            Self::Complete => write!(f, "complete"),
            Self::Failed(reason) => write!(f, "failed: {}", reason),
        }
    }
}

// ── Grammar Style ────────────────────────────────────────────────────

/// The stylistic approach to the generated grammar.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrammarStyle {
    /// Declarative: focuses on "what" (SQL-like).
    Declarative,
    /// Expressive: focuses on transformations and pipes.
    Expressive,
    /// Configuration: structured key-value with validation.
    Configuration,
    /// Scripting: imperative with control flow.
    Scripting,
}

impl std::fmt::Display for GrammarStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Declarative => write!(f, "declarative"),
            Self::Expressive => write!(f, "expressive"),
            Self::Configuration => write!(f, "configuration"),
            Self::Scripting => write!(f, "scripting"),
        }
    }
}

// ── Usage Pattern ────────────────────────────────────────────────────

/// An observed usage pattern that informs language design.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UsagePattern {
    /// Description of the pattern.
    pub description: String,
    /// How frequently this pattern is observed (0.0 to 1.0).
    pub frequency: f64,
    /// Domain concepts involved.
    pub concepts: Vec<String>,
    /// Operations commonly performed.
    pub operations: Vec<String>,
}

// ── Domain Specification ─────────────────────────────────────────────

/// Type of a concept property.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyType {
    Text,
    Integer,
    Decimal,
    Boolean,
    Amount,
    Date,
    Reference(String),
}

impl std::fmt::Display for PropertyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Integer => write!(f, "integer"),
            Self::Decimal => write!(f, "decimal"),
            Self::Boolean => write!(f, "boolean"),
            Self::Amount => write!(f, "amount"),
            Self::Date => write!(f, "date"),
            Self::Reference(name) => write!(f, "ref<{}>", name),
        }
    }
}

/// A property on a domain concept.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConceptProperty {
    pub name: String,
    pub property_type: PropertyType,
    pub required: bool,
}

/// A concept within the domain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainConcept {
    pub name: String,
    pub description: String,
    pub properties: Vec<ConceptProperty>,
    pub is_primary: bool,
}

/// Type of relationship between domain concepts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    /// One-to-one.
    OneToOne,
    /// One-to-many.
    OneToMany,
    /// Many-to-many.
    ManyToMany,
    /// Inheritance/specialization.
    IsA,
    /// Composition/containment.
    HasA,
}

impl std::fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OneToOne => write!(f, "one-to-one"),
            Self::OneToMany => write!(f, "one-to-many"),
            Self::ManyToMany => write!(f, "many-to-many"),
            Self::IsA => write!(f, "is-a"),
            Self::HasA => write!(f, "has-a"),
        }
    }
}

/// A relationship between two domain concepts.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConceptRelationship {
    pub from: String,
    pub to: String,
    pub relationship_type: RelationshipType,
    pub description: String,
}

/// Type of domain constraint.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintType {
    /// Value must be non-negative.
    NonNegative,
    /// Value must be within a range.
    Range,
    /// Value must be unique.
    Unique,
    /// Custom invariant.
    Invariant(String),
}

/// A constraint within the domain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainConstraint {
    pub name: String,
    pub constraint_type: ConstraintType,
    pub applies_to: String,
    pub description: String,
}

/// Full domain specification produced by domain analysis.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainSpec {
    pub name: String,
    pub description: String,
    pub concepts: Vec<DomainConcept>,
    pub relationships: Vec<ConceptRelationship>,
    pub constraints: Vec<DomainConstraint>,
    pub recommended_style: GrammarStyle,
}

// ── Grammar Specification ────────────────────────────────────────────

/// Associativity for an operator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Associativity {
    Left,
    Right,
    None,
}

/// Precedence level for operator ordering.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrecedenceLevel {
    pub level: u32,
    pub operators: Vec<String>,
    pub associativity: Associativity,
}

/// An operator definition in the grammar.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperatorDef {
    pub symbol: String,
    pub name: String,
    pub arity: u32,
    pub precedence: u32,
    pub associativity: Associativity,
}

/// A production rule in the grammar.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Production {
    pub id: ProductionId,
    pub name: String,
    pub pattern: String,
    pub description: String,
    pub concept: String,
}

/// Full grammar specification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GrammarSpec {
    pub style: GrammarStyle,
    pub productions: Vec<Production>,
    pub operators: Vec<OperatorDef>,
    pub precedence_levels: Vec<PrecedenceLevel>,
    pub keywords: Vec<String>,
}

// ── Type System Specification ────────────────────────────────────────

/// The kind of a DSL type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeKind {
    /// Primitive scalar type.
    Primitive,
    /// Composite/record type.
    Composite,
    /// Collection type (list/set).
    Collection,
    /// Financial amount type (currency-aware).
    Financial,
    /// Reference to another entity.
    Reference,
}

impl std::fmt::Display for TypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primitive => write!(f, "primitive"),
            Self::Composite => write!(f, "composite"),
            Self::Collection => write!(f, "collection"),
            Self::Financial => write!(f, "financial"),
            Self::Reference => write!(f, "reference"),
        }
    }
}

/// A type in the DSL type system.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DslType {
    pub name: String,
    pub kind: TypeKind,
    pub description: String,
    pub properties: Vec<(String, String)>,
}

/// Safety level for a type coercion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoercionSafety {
    /// Coercion is always safe (no data loss).
    Safe,
    /// Coercion may lose precision.
    Lossy,
    /// Coercion is forbidden (e.g., Amount → Number).
    Forbidden,
}

impl std::fmt::Display for CoercionSafety {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Safe => write!(f, "safe"),
            Self::Lossy => write!(f, "lossy"),
            Self::Forbidden => write!(f, "forbidden"),
        }
    }
}

/// A coercion rule between two types.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoercionRule {
    pub from_type: String,
    pub to_type: String,
    pub safety: CoercionSafety,
    pub description: String,
}

/// When a type constraint is enforced.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnforcementPhase {
    /// At type-check time (compile).
    TypeCheck,
    /// At runtime.
    Runtime,
    /// At both compile and runtime.
    Both,
}

impl std::fmt::Display for EnforcementPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TypeCheck => write!(f, "type-check"),
            Self::Runtime => write!(f, "runtime"),
            Self::Both => write!(f, "both"),
        }
    }
}

/// A type constraint enforced by the type system.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypeConstraint {
    pub name: String,
    pub applies_to: String,
    pub enforcement: EnforcementPhase,
    pub description: String,
}

/// Full type system specification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypeSystemSpec {
    pub types: Vec<DslType>,
    pub coercion_rules: Vec<CoercionRule>,
    pub constraints: Vec<TypeConstraint>,
}

// ── Semantic Rules ───────────────────────────────────────────────────

/// Side effect generated by a semantic rule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SideEffect {
    /// Emit an event into the event fabric.
    EmitEvent(String),
    /// Record provenance for the operation.
    RecordProvenance,
    /// Store to a specific memory tier.
    MemoryStore(String),
    /// Invoke an operator.
    InvokeOperator(String),
}

/// How a semantic rule is evaluated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvaluationStrategy {
    /// Pure evaluation (no side effects).
    Pure,
    /// Stateful evaluation (with side effects).
    Stateful,
    /// Delegates to a WorldLine operator invocation.
    OperatorInvocation,
}

impl std::fmt::Display for EvaluationStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pure => write!(f, "pure"),
            Self::Stateful => write!(f, "stateful"),
            Self::OperatorInvocation => write!(f, "operator-invocation"),
        }
    }
}

/// A semantic rule mapping a production to behavior.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticRule {
    pub production_id: ProductionId,
    pub evaluation: EvaluationStrategy,
    pub side_effects: Vec<SideEffect>,
    pub description: String,
}

// ── Parser Specification ─────────────────────────────────────────────

/// Type of parser to generate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParserType {
    /// Recursive descent parser.
    RecursiveDescent,
    /// PEG (Parsing Expression Grammar) parser.
    Peg,
    /// Pratt parser (for expression-heavy grammars).
    Pratt,
}

impl std::fmt::Display for ParserType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RecursiveDescent => write!(f, "recursive-descent"),
            Self::Peg => write!(f, "peg"),
            Self::Pratt => write!(f, "pratt"),
        }
    }
}

/// Parser specification for the generated language.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParserSpec {
    pub parser_type: ParserType,
    pub source_skeleton: String,
    pub entry_rule: String,
    pub total_rules: usize,
}

// ── Compiler Specification ───────────────────────────────────────────

/// Target for the generated compiler.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompilerTarget {
    /// Compile to WLIR instructions.
    WlirInstructions,
    /// Compile to WorldLine operator calls.
    OperatorCalls,
    /// Compile to Rust source code.
    RustSource,
}

impl std::fmt::Display for CompilerTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WlirInstructions => write!(f, "wlir-instructions"),
            Self::OperatorCalls => write!(f, "operator-calls"),
            Self::RustSource => write!(f, "rust-source"),
        }
    }
}

/// Optimization level for the generated compiler.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationLevel {
    None,
    Basic,
    Aggressive,
}

impl std::fmt::Display for OptimizationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Basic => write!(f, "basic"),
            Self::Aggressive => write!(f, "aggressive"),
        }
    }
}

/// Compiler specification for the generated language.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompilerSpec {
    pub target: CompilerTarget,
    pub optimization: OptimizationLevel,
    pub source_skeleton: String,
    pub total_passes: usize,
}

// ── Generated Language (Final Output) ────────────────────────────────

/// A fully generated language — the output of the langgen pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneratedLanguage {
    pub id: LanguageId,
    pub name: String,
    pub version: String,
    pub domain: DomainSpec,
    pub grammar: GrammarSpec,
    pub type_system: TypeSystemSpec,
    pub semantic_rules: Vec<SemanticRule>,
    pub parser: ParserSpec,
    pub compiler: CompilerSpec,
    pub created_at: DateTime<Utc>,
}

// ── Configuration ────────────────────────────────────────────────────

/// Configuration for the language generation engine.
#[derive(Clone, Debug)]
pub struct LangGenConfig {
    /// Maximum number of generation records to keep.
    pub max_tracked_records: usize,
    /// Maximum concepts per domain.
    pub max_concepts: usize,
    /// Maximum productions per grammar.
    pub max_productions: usize,
    /// Maximum types per type system.
    pub max_types: usize,
    /// Default grammar style if domain analysis doesn't recommend one.
    pub default_style: GrammarStyle,
    /// Default compiler target.
    pub default_target: CompilerTarget,
    /// Default optimization level.
    pub default_optimization: OptimizationLevel,
}

impl Default for LangGenConfig {
    fn default() -> Self {
        Self {
            max_tracked_records: 256,
            max_concepts: 64,
            max_productions: 256,
            max_types: 128,
            default_style: GrammarStyle::Declarative,
            default_target: CompilerTarget::WlirInstructions,
            default_optimization: OptimizationLevel::Basic,
        }
    }
}

// ── Summary ──────────────────────────────────────────────────────────

/// Summary statistics for the language generation engine.
#[derive(Clone, Debug, Default)]
pub struct LangGenSummary {
    pub total_generations: usize,
    pub successful_generations: usize,
    pub failed_generations: usize,
    pub total_evolutions: usize,
    pub languages_produced: usize,
}

impl std::fmt::Display for LangGenSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LangGen(generations={}, success={}, failed={}, evolutions={}, languages={})",
            self.total_generations,
            self.successful_generations,
            self.failed_generations,
            self.total_evolutions,
            self.languages_produced,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn langgen_id_uniqueness() {
        let a = LangGenId::new();
        let b = LangGenId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn langgen_id_display() {
        let id = LangGenId::new();
        assert!(id.to_string().starts_with("langgen:"));
    }

    #[test]
    fn language_id_display() {
        let id = LanguageId::from_name("fin-settle");
        assert_eq!(id.to_string(), "lang:fin-settle");
    }

    #[test]
    fn production_id_display() {
        let id = ProductionId::from_name("transfer_stmt");
        assert_eq!(id.to_string(), "prod:transfer_stmt");
    }

    #[test]
    fn status_display_all_variants() {
        assert_eq!(LangGenStatus::Started.to_string(), "started");
        assert_eq!(LangGenStatus::DomainAnalyzed.to_string(), "domain-analyzed");
        assert_eq!(LangGenStatus::GrammarSynthesized.to_string(), "grammar-synthesized");
        assert_eq!(LangGenStatus::TypeSystemDesigned.to_string(), "type-system-designed");
        assert_eq!(LangGenStatus::SemanticsMapped.to_string(), "semantics-mapped");
        assert_eq!(LangGenStatus::ParserGenerated.to_string(), "parser-generated");
        assert_eq!(LangGenStatus::CompilerGenerated.to_string(), "compiler-generated");
        assert_eq!(LangGenStatus::Complete.to_string(), "complete");
        assert!(LangGenStatus::Failed("bad".into()).to_string().contains("bad"));
    }

    #[test]
    fn grammar_style_display() {
        assert_eq!(GrammarStyle::Declarative.to_string(), "declarative");
        assert_eq!(GrammarStyle::Expressive.to_string(), "expressive");
        assert_eq!(GrammarStyle::Configuration.to_string(), "configuration");
        assert_eq!(GrammarStyle::Scripting.to_string(), "scripting");
    }

    #[test]
    fn coercion_safety_display() {
        assert_eq!(CoercionSafety::Safe.to_string(), "safe");
        assert_eq!(CoercionSafety::Lossy.to_string(), "lossy");
        assert_eq!(CoercionSafety::Forbidden.to_string(), "forbidden");
    }

    #[test]
    fn type_kind_display() {
        assert_eq!(TypeKind::Primitive.to_string(), "primitive");
        assert_eq!(TypeKind::Financial.to_string(), "financial");
    }

    #[test]
    fn property_type_display() {
        assert_eq!(PropertyType::Amount.to_string(), "amount");
        assert_eq!(
            PropertyType::Reference("Account".into()).to_string(),
            "ref<Account>"
        );
    }

    #[test]
    fn config_defaults() {
        let cfg = LangGenConfig::default();
        assert_eq!(cfg.max_tracked_records, 256);
        assert_eq!(cfg.max_concepts, 64);
        assert_eq!(cfg.default_style, GrammarStyle::Declarative);
        assert_eq!(cfg.default_target, CompilerTarget::WlirInstructions);
        assert_eq!(cfg.default_optimization, OptimizationLevel::Basic);
    }

    #[test]
    fn summary_default_and_display() {
        let s = LangGenSummary::default();
        assert_eq!(s.total_generations, 0);
        let display = s.to_string();
        assert!(display.contains("generations=0"));
    }

    #[test]
    fn compiler_target_display() {
        assert_eq!(CompilerTarget::WlirInstructions.to_string(), "wlir-instructions");
        assert_eq!(CompilerTarget::OperatorCalls.to_string(), "operator-calls");
        assert_eq!(CompilerTarget::RustSource.to_string(), "rust-source");
    }
}
