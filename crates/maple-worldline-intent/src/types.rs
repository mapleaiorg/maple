//! Core type definitions for the intent stabilization engine.
//!
//! Types for identifying intents and proposals, classifying change types,
//! governance tiers, and reversibility levels.

use serde::{Deserialize, Serialize};

// ── Identifier Types ────────────────────────────────────────────────────

/// Unique identifier for a self-regeneration intent.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntentId(pub String);

impl IntentId {
    /// Generate a new unique intent ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for IntentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for IntentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "intent:{}", self.0)
    }
}

/// Unique identifier for a regeneration proposal.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProposalId(pub String);

impl ProposalId {
    /// Generate a new unique proposal ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for ProposalId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProposalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "proposal:{}", self.0)
    }
}

// ── Change Type ─────────────────────────────────────────────────────────

/// Classification of the proposed self-modification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ChangeType {
    /// Modify an existing operator's implementation.
    OperatorModification {
        operator_id: String,
        modification_scope: String,
    },

    /// Add a new operator.
    NewOperator {
        operator_name: String,
        description: String,
    },

    /// Modify kernel module implementation.
    KernelModification {
        module: String,
        modification_scope: String,
    },

    /// Modify API surface.
    ApiModification {
        api_component: String,
        breaking: bool,
        migration_plan: Option<String>,
    },

    /// Configuration change (non-code).
    ConfigurationChange {
        parameter: String,
        current_value: String,
        proposed_value: String,
        rationale: String,
    },

    /// Generate domain-specific language.
    DslGeneration { domain: String, description: String },

    /// Modify compilation/optimization strategy.
    CompilationStrategy {
        current_strategy: String,
        proposed_changes: Vec<String>,
    },

    /// Architecture restructuring.
    ArchitecturalChange {
        description: String,
        affected_modules: Vec<String>,
        migration_plan: String,
    },
}

impl ChangeType {
    /// Short label for the change type.
    pub fn label(&self) -> &str {
        match self {
            Self::OperatorModification { .. } => "operator-modification",
            Self::NewOperator { .. } => "new-operator",
            Self::KernelModification { .. } => "kernel-modification",
            Self::ApiModification { .. } => "api-modification",
            Self::ConfigurationChange { .. } => "configuration-change",
            Self::DslGeneration { .. } => "dsl-generation",
            Self::CompilationStrategy { .. } => "compilation-strategy",
            Self::ArchitecturalChange { .. } => "architectural-change",
        }
    }
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ── Code Change Type ────────────────────────────────────────────────────

/// Types of code changes in a proposal.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CodeChangeType {
    /// Create a new source file.
    CreateFile,
    /// Modify an existing function.
    ModifyFunction { function_name: String },
    /// Modify an existing struct.
    ModifyStruct { struct_name: String },
    /// Modify an existing trait.
    ModifyTrait { trait_name: String },
    /// Add a trait implementation.
    AddImplementation {
        trait_name: String,
        struct_name: String,
    },
    /// Refactor an entire module.
    RefactorModule { module_name: String },
    /// Delete code.
    DeleteCode { target: String },
}

impl std::fmt::Display for CodeChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateFile => write!(f, "create-file"),
            Self::ModifyFunction { function_name } => write!(f, "modify-fn:{}", function_name),
            Self::ModifyStruct { struct_name } => write!(f, "modify-struct:{}", struct_name),
            Self::ModifyTrait { trait_name } => write!(f, "modify-trait:{}", trait_name),
            Self::AddImplementation {
                trait_name,
                struct_name,
            } => {
                write!(f, "impl:{}:{}", trait_name, struct_name)
            }
            Self::RefactorModule { module_name } => write!(f, "refactor:{}", module_name),
            Self::DeleteCode { target } => write!(f, "delete:{}", target),
        }
    }
}

// ── Substrate Tier ──────────────────────────────────────────────────────

/// Governance tier for self-modification proposals.
///
/// Higher tiers require more scrutiny and have longer observation periods.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SubstrateTier {
    /// Configuration-only changes (lowest friction).
    Tier0,
    /// Operator-level modifications.
    Tier1,
    /// Kernel module modifications.
    Tier2,
    /// Architecture-level restructuring (highest friction).
    Tier3,
}

impl SubstrateTier {
    /// Minimum observation period for this tier (seconds).
    pub fn min_observation_secs(&self) -> u64 {
        match self {
            Self::Tier0 => 1800,   // 30 minutes
            Self::Tier1 => 3600,   // 1 hour
            Self::Tier2 => 86400,  // 24 hours
            Self::Tier3 => 259200, // 72 hours
        }
    }

    /// Minimum confidence required for this tier.
    pub fn min_confidence(&self) -> f64 {
        match self {
            Self::Tier0 => 0.7,
            Self::Tier1 => 0.8,
            Self::Tier2 => 0.85,
            Self::Tier3 => 0.9,
        }
    }
}

impl std::fmt::Display for SubstrateTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tier0 => write!(f, "tier-0-config"),
            Self::Tier1 => write!(f, "tier-1-operator"),
            Self::Tier2 => write!(f, "tier-2-kernel"),
            Self::Tier3 => write!(f, "tier-3-architecture"),
        }
    }
}

// ── Reversibility ───────────────────────────────────────────────────────

/// Reversibility level of a proposed change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReversibilityLevel {
    /// Change can be fully reverted at any time.
    FullyReversible,
    /// Change can be reverted under certain conditions.
    ConditionallyReversible { conditions: Vec<String> },
    /// Change can be reverted within a time window (seconds).
    TimeWindowReversible { window_secs: u64 },
    /// Change cannot be reverted.
    Irreversible,
}

impl ReversibilityLevel {
    /// Whether this change is reversible in any form.
    pub fn is_reversible(&self) -> bool {
        !matches!(self, Self::Irreversible)
    }
}

impl std::fmt::Display for ReversibilityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FullyReversible => write!(f, "fully-reversible"),
            Self::ConditionallyReversible { .. } => write!(f, "conditionally-reversible"),
            Self::TimeWindowReversible { window_secs } => {
                write!(f, "time-window-{}s", window_secs)
            }
            Self::Irreversible => write!(f, "irreversible"),
        }
    }
}

// ── Configuration ───────────────────────────────────────────────────────

/// Configuration for the intent stabilization engine.
#[derive(Clone, Debug)]
pub struct IntentConfig {
    /// Minimum confidence for an intent to be validated.
    pub min_confidence: f64,
    /// Maximum acceptable risk score (0.0 to 1.0).
    pub max_risk: f64,
    /// Minimum evidence items from the source meanings.
    pub min_evidence_count: usize,
    /// Whether rollback plans are mandatory.
    pub rollback_required: bool,
    /// Maximum number of concurrent active intents.
    pub max_concurrent_intents: usize,
    /// Cool-down after a modification (seconds).
    pub post_modification_cooldown_secs: u64,
    /// Maximum system load to allow regeneration.
    pub max_system_load: f64,
    /// Maximum number of active intents stored.
    pub max_active_intents: usize,
    /// Maximum number of deferred intents stored.
    pub max_deferred_intents: usize,
    /// Prioritization weights.
    pub improvement_weight: f64,
    pub risk_weight: f64,
    pub confidence_weight: f64,
    pub urgency_weight: f64,
}

impl Default for IntentConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.8,
            max_risk: 0.3,
            min_evidence_count: 5,
            rollback_required: true,
            max_concurrent_intents: 3,
            post_modification_cooldown_secs: 3600,
            max_system_load: 0.8,
            max_active_intents: 64,
            max_deferred_intents: 128,
            improvement_weight: 0.3,
            risk_weight: 0.25,
            confidence_weight: 0.25,
            urgency_weight: 0.2,
        }
    }
}

// ── Intent Precondition ─────────────────────────────────────────────────

/// A precondition that must be satisfied before an intent proceeds.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntentPrecondition {
    /// Description of the precondition.
    pub description: String,
    /// Whether this precondition is currently satisfied.
    pub satisfied: bool,
}

// Re-export MeaningId for convenience. Used in proposal.rs
// (CodeChangeSpec.provenance) and intent.rs (SelfRegenerationIntent.derived_from).
pub use maple_worldline_meaning::MeaningId;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_id_uniqueness() {
        let a = IntentId::new();
        let b = IntentId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn intent_id_display_format() {
        let id = IntentId::new();
        assert!(id.to_string().starts_with("intent:"));
    }

    #[test]
    fn proposal_id_display_format() {
        let id = ProposalId::new();
        assert!(id.to_string().starts_with("proposal:"));
    }

    #[test]
    fn change_type_all_variants() {
        let variants: Vec<ChangeType> = vec![
            ChangeType::OperatorModification {
                operator_id: "a".into(),
                modification_scope: "b".into(),
            },
            ChangeType::NewOperator {
                operator_name: "a".into(),
                description: "b".into(),
            },
            ChangeType::KernelModification {
                module: "a".into(),
                modification_scope: "b".into(),
            },
            ChangeType::ApiModification {
                api_component: "a".into(),
                breaking: false,
                migration_plan: None,
            },
            ChangeType::ConfigurationChange {
                parameter: "a".into(),
                current_value: "1".into(),
                proposed_value: "2".into(),
                rationale: "c".into(),
            },
            ChangeType::DslGeneration {
                domain: "a".into(),
                description: "b".into(),
            },
            ChangeType::CompilationStrategy {
                current_strategy: "a".into(),
                proposed_changes: vec![],
            },
            ChangeType::ArchitecturalChange {
                description: "a".into(),
                affected_modules: vec![],
                migration_plan: "b".into(),
            },
        ];
        let labels: std::collections::HashSet<&str> = variants.iter().map(|c| c.label()).collect();
        assert_eq!(labels.len(), 8);
    }

    #[test]
    fn substrate_tier_ordering() {
        assert!(SubstrateTier::Tier0 < SubstrateTier::Tier1);
        assert!(SubstrateTier::Tier1 < SubstrateTier::Tier2);
        assert!(SubstrateTier::Tier2 < SubstrateTier::Tier3);
    }

    #[test]
    fn substrate_tier_thresholds() {
        assert!(SubstrateTier::Tier3.min_confidence() > SubstrateTier::Tier0.min_confidence());
        assert!(
            SubstrateTier::Tier3.min_observation_secs()
                > SubstrateTier::Tier0.min_observation_secs()
        );
    }

    #[test]
    fn reversibility_check() {
        assert!(ReversibilityLevel::FullyReversible.is_reversible());
        assert!(ReversibilityLevel::ConditionallyReversible { conditions: vec![] }.is_reversible());
        assert!(ReversibilityLevel::TimeWindowReversible { window_secs: 3600 }.is_reversible());
        assert!(!ReversibilityLevel::Irreversible.is_reversible());
    }

    #[test]
    fn intent_config_defaults() {
        let cfg = IntentConfig::default();
        assert!((cfg.min_confidence - 0.8).abs() < f64::EPSILON);
        assert!((cfg.max_risk - 0.3).abs() < f64::EPSILON);
        assert_eq!(cfg.min_evidence_count, 5);
        assert!(cfg.rollback_required);
        assert_eq!(cfg.max_concurrent_intents, 3);
    }

    #[test]
    fn code_change_type_display() {
        let ct = CodeChangeType::ModifyFunction {
            function_name: "process".into(),
        };
        assert_eq!(ct.to_string(), "modify-fn:process");

        let ct = CodeChangeType::CreateFile;
        assert_eq!(ct.to_string(), "create-file");
    }
}
