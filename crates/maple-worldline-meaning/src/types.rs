//! Core type definitions for the meaning formation engine.
//!
//! These types represent the meanings WorldLine forms about its own behavior,
//! the evidence supporting those meanings, and the categories of self-understanding.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Identifier Types ────────────────────────────────────────────────────

/// Unique identifier for a formed meaning.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MeaningId(pub String);

impl MeaningId {
    /// Generate a new unique meaning ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for MeaningId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MeaningId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "meaning:{}", self.0)
    }
}

/// Unique identifier for a hypothesis.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HypothesisId(pub String);

impl HypothesisId {
    /// Generate a new unique hypothesis ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for HypothesisId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for HypothesisId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "hyp:{}", self.0)
    }
}

// ── Evidence ────────────────────────────────────────────────────────────

/// A piece of evidence supporting or weakening a hypothesis.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evidence {
    /// What produced this evidence (e.g., "statistical-anomaly-detector").
    pub source: String,
    /// Evidence strength (0.0 = negligible, 1.0 = definitive).
    pub strength: f64,
    /// When this evidence was collected.
    pub timestamp: DateTime<Utc>,
    /// Human-readable description of what was observed.
    pub description: String,
    /// Classification of the evidence type.
    pub category: EvidenceCategory,
}

/// Classification of evidence sources.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvidenceCategory {
    /// Direct observation from metrics/monitoring.
    Observation,
    /// Anomaly detection signal.
    Anomaly,
    /// Pattern match from historical data.
    Historical,
    /// Cross-metric correlation signal.
    Correlation,
    /// Absence of expected signal (negative evidence).
    Absence,
}

impl std::fmt::Display for EvidenceCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Observation => write!(f, "observation"),
            Self::Anomaly => write!(f, "anomaly"),
            Self::Historical => write!(f, "historical"),
            Self::Correlation => write!(f, "correlation"),
            Self::Absence => write!(f, "absence"),
        }
    }
}

// ── Root Cause Hypothesis ───────────────────────────────────────────────

/// A proposed root cause for an observed issue.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RootCauseHypothesis {
    /// Human-readable description of the proposed root cause.
    pub description: String,
    /// Confidence in this root cause (0.0 to 1.0).
    pub confidence: f64,
    /// Descriptions of supporting evidence.
    pub supporting_evidence: Vec<String>,
}

// ── Self-Meaning ────────────────────────────────────────────────────────

/// A meaning WorldLine has formed about its own behavior.
///
/// Meanings may compete with each other — ambiguity is preserved until evidence
/// resolves it or action requires a decision. Premature disambiguation is a
/// failure mode per Resonance Architecture §5.4.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SelfMeaning {
    /// Unique meaning identifier.
    pub id: MeaningId,
    /// What category of self-understanding this represents.
    pub category: SelfMeaningCategory,
    /// Evidence supporting this meaning.
    pub evidence: Vec<Evidence>,
    /// Overall confidence in this meaning (0.0 to 1.0).
    pub confidence: f64,
    /// Ambiguity level (0.0 = fully resolved, 1.0 = completely ambiguous).
    pub ambiguity: f64,
    /// When this meaning was first formed.
    pub formed_at: DateTime<Utc>,
    /// Temporal stability: how long confidence has been stable (seconds).
    pub temporal_stability_secs: f64,
    /// IDs of competing interpretations.
    pub competing_with: Vec<MeaningId>,
    /// Whether this meaning has converged (stable enough for intent).
    pub converged: bool,
}

impl SelfMeaning {
    /// Whether this meaning is safety-relevant (resource exhaustion, systematic errors).
    pub fn is_safety_relevant(&self) -> bool {
        matches!(
            self.category,
            SelfMeaningCategory::CapacityForecast { .. }
                | SelfMeaningCategory::SystematicError { .. }
        )
    }

    /// Whether this meaning has sufficient evidence for intent consideration.
    pub fn has_sufficient_evidence(&self, min_count: usize) -> bool {
        self.evidence.len() >= min_count
    }
}

// ── Self-Meaning Categories ─────────────────────────────────────────────

/// Categories of meaning WorldLine can form about itself.
///
/// Each variant represents a distinct class of self-understanding,
/// from performance insights to architectural observations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SelfMeaningCategory {
    /// A specific component is consistently underperforming.
    PerformanceBottleneck {
        component: String,
        severity: f64,
        root_causes: Vec<RootCauseHypothesis>,
    },

    /// Usage pattern suggests API design could be improved.
    ApiDesignInsight {
        pattern: String,
        improvement_direction: String,
    },

    /// Memory usage suggests storage optimization opportunity.
    MemoryOptimization {
        component: String,
        optimization_type: MemoryOptimizationType,
        estimated_improvement: f64,
    },

    /// Operator execution suggests redesign.
    OperatorOptimization {
        operator_id: String,
        bottleneck_type: OperatorBottleneckType,
        improvement_hypothesis: String,
    },

    /// Dead or redundant code detected.
    CodeSimplification {
        component: String,
        dead_paths: Vec<String>,
        redundancy_type: RedundancyType,
    },

    /// Architecture-level structural insight.
    ArchitecturalInsight {
        insight_type: ArchitecturalInsightType,
        affected_components: Vec<String>,
        structural_pressure: String,
    },

    /// System approaching a resource limit.
    CapacityForecast {
        resource: String,
        current_utilization: f64,
        projected_exhaustion_hours: Option<f64>,
        growth_model: GrowthModel,
    },

    /// Error pattern suggests systematic issue.
    SystematicError {
        error_pattern: String,
        frequency: f64,
        affected_operations: Vec<String>,
        root_causes: Vec<RootCauseHypothesis>,
    },
}

impl SelfMeaningCategory {
    /// Short label for the category (for logging/display).
    pub fn label(&self) -> &str {
        match self {
            Self::PerformanceBottleneck { .. } => "performance-bottleneck",
            Self::ApiDesignInsight { .. } => "api-design-insight",
            Self::MemoryOptimization { .. } => "memory-optimization",
            Self::OperatorOptimization { .. } => "operator-optimization",
            Self::CodeSimplification { .. } => "code-simplification",
            Self::ArchitecturalInsight { .. } => "architectural-insight",
            Self::CapacityForecast { .. } => "capacity-forecast",
            Self::SystematicError { .. } => "systematic-error",
        }
    }

    /// Returns the primary component affected by this meaning, if applicable.
    pub fn primary_component(&self) -> Option<&str> {
        match self {
            Self::PerformanceBottleneck { component, .. } => Some(component),
            Self::MemoryOptimization { component, .. } => Some(component),
            Self::OperatorOptimization { operator_id, .. } => Some(operator_id),
            Self::CodeSimplification { component, .. } => Some(component),
            _ => None,
        }
    }
}

impl std::fmt::Display for SelfMeaningCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ── Supporting Enums ────────────────────────────────────────────────────

/// Types of memory optimization opportunities.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryOptimizationType {
    /// Rebalance data across memory tiers.
    TierRebalancing,
    /// Resize cache allocations.
    CacheResizing,
    /// Tune eviction policies.
    EvictionTuning,
}

impl std::fmt::Display for MemoryOptimizationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TierRebalancing => write!(f, "tier-rebalancing"),
            Self::CacheResizing => write!(f, "cache-resizing"),
            Self::EvictionTuning => write!(f, "eviction-tuning"),
        }
    }
}

/// Types of operator performance bottlenecks.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperatorBottleneckType {
    /// CPU-bound computation.
    CpuBound,
    /// I/O-bound waiting.
    IoBound,
    /// Lock/resource contention.
    ContentionBound,
    /// Algorithmic complexity issue.
    AlgorithmicComplexity,
}

impl std::fmt::Display for OperatorBottleneckType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CpuBound => write!(f, "cpu-bound"),
            Self::IoBound => write!(f, "io-bound"),
            Self::ContentionBound => write!(f, "contention-bound"),
            Self::AlgorithmicComplexity => write!(f, "algorithmic-complexity"),
        }
    }
}

/// Types of code redundancy.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RedundancyType {
    /// Duplicate logic in different paths.
    DuplicateLogic,
    /// Branch that is never taken.
    UnusedBranch,
    /// Code that is never executed.
    DeadCode,
}

impl std::fmt::Display for RedundancyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateLogic => write!(f, "duplicate-logic"),
            Self::UnusedBranch => write!(f, "unused-branch"),
            Self::DeadCode => write!(f, "dead-code"),
        }
    }
}

/// Types of architectural insights.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArchitecturalInsightType {
    /// Components are too tightly coupled.
    CouplingTooTight,
    /// Layer boundaries are being violated.
    LayerViolation,
    /// Missing abstraction layer.
    MissingAbstraction,
    /// Over-engineered for current usage.
    OverEngineered,
}

impl std::fmt::Display for ArchitecturalInsightType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CouplingTooTight => write!(f, "coupling-too-tight"),
            Self::LayerViolation => write!(f, "layer-violation"),
            Self::MissingAbstraction => write!(f, "missing-abstraction"),
            Self::OverEngineered => write!(f, "over-engineered"),
        }
    }
}

/// Growth models for capacity forecasting.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GrowthModel {
    /// Linear growth rate.
    Linear,
    /// Exponential growth rate.
    Exponential,
    /// Logarithmic (decelerating) growth.
    Logarithmic,
    /// Stable (no significant growth).
    Stable,
}

impl std::fmt::Display for GrowthModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Linear => write!(f, "linear"),
            Self::Exponential => write!(f, "exponential"),
            Self::Logarithmic => write!(f, "logarithmic"),
            Self::Stable => write!(f, "stable"),
        }
    }
}

// ── Configuration ───────────────────────────────────────────────────────

/// Configuration for the meaning formation engine.
#[derive(Clone, Debug)]
pub struct MeaningConfig {
    /// Convergence threshold: confidence variance must be below this for convergence.
    pub convergence_threshold: f64,
    /// Minimum number of evidence items before meaning is eligible for intent.
    pub min_evidence_count: usize,
    /// Minimum observation duration (seconds) before meaning is eligible for intent.
    pub min_observation_secs: u64,
    /// Ambiguity resolution threshold (below this, meaning is "resolved").
    pub resolution_threshold: f64,
    /// Safety-relevant meanings have a stricter resolution threshold.
    pub safety_resolution_threshold: f64,
    /// Maximum number of active (in-formation) meanings.
    pub max_active_meanings: usize,
    /// Maximum number of historical meanings to retain.
    pub max_history: usize,
}

impl Default for MeaningConfig {
    fn default() -> Self {
        Self {
            convergence_threshold: 0.85,
            min_evidence_count: 10,
            min_observation_secs: 3600,
            resolution_threshold: 0.2,
            safety_resolution_threshold: 0.1,
            max_active_meanings: 256,
            max_history: 1024,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meaning_id_uniqueness() {
        let a = MeaningId::new();
        let b = MeaningId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn meaning_id_display_format() {
        let id = MeaningId::new();
        let display = id.to_string();
        assert!(display.starts_with("meaning:"));
    }

    #[test]
    fn hypothesis_id_display_format() {
        let id = HypothesisId::new();
        let display = id.to_string();
        assert!(display.starts_with("hyp:"));
    }

    #[test]
    fn evidence_category_all_variants() {
        let categories = vec![
            EvidenceCategory::Observation,
            EvidenceCategory::Anomaly,
            EvidenceCategory::Historical,
            EvidenceCategory::Correlation,
            EvidenceCategory::Absence,
        ];
        let displays: std::collections::HashSet<String> =
            categories.iter().map(|c| c.to_string()).collect();
        assert_eq!(displays.len(), 5);
    }

    #[test]
    fn self_meaning_category_all_variants() {
        let categories: Vec<SelfMeaningCategory> = vec![
            SelfMeaningCategory::PerformanceBottleneck {
                component: "test".into(),
                severity: 0.5,
                root_causes: vec![],
            },
            SelfMeaningCategory::ApiDesignInsight {
                pattern: "test".into(),
                improvement_direction: "test".into(),
            },
            SelfMeaningCategory::MemoryOptimization {
                component: "test".into(),
                optimization_type: MemoryOptimizationType::CacheResizing,
                estimated_improvement: 0.3,
            },
            SelfMeaningCategory::OperatorOptimization {
                operator_id: "test".into(),
                bottleneck_type: OperatorBottleneckType::CpuBound,
                improvement_hypothesis: "test".into(),
            },
            SelfMeaningCategory::CodeSimplification {
                component: "test".into(),
                dead_paths: vec![],
                redundancy_type: RedundancyType::DeadCode,
            },
            SelfMeaningCategory::ArchitecturalInsight {
                insight_type: ArchitecturalInsightType::CouplingTooTight,
                affected_components: vec![],
                structural_pressure: "test".into(),
            },
            SelfMeaningCategory::CapacityForecast {
                resource: "memory".into(),
                current_utilization: 0.8,
                projected_exhaustion_hours: Some(24.0),
                growth_model: GrowthModel::Linear,
            },
            SelfMeaningCategory::SystematicError {
                error_pattern: "test".into(),
                frequency: 0.1,
                affected_operations: vec![],
                root_causes: vec![],
            },
        ];
        let labels: std::collections::HashSet<&str> =
            categories.iter().map(|c| c.label()).collect();
        assert_eq!(labels.len(), 8);
    }

    #[test]
    fn self_meaning_safety_relevance() {
        let safety = SelfMeaning {
            id: MeaningId::new(),
            category: SelfMeaningCategory::CapacityForecast {
                resource: "memory".into(),
                current_utilization: 0.95,
                projected_exhaustion_hours: Some(2.0),
                growth_model: GrowthModel::Exponential,
            },
            evidence: vec![],
            confidence: 0.8,
            ambiguity: 0.2,
            formed_at: Utc::now(),
            temporal_stability_secs: 3600.0,
            competing_with: vec![],
            converged: false,
        };
        assert!(safety.is_safety_relevant());

        let non_safety = SelfMeaning {
            id: MeaningId::new(),
            category: SelfMeaningCategory::ApiDesignInsight {
                pattern: "test".into(),
                improvement_direction: "test".into(),
            },
            evidence: vec![],
            confidence: 0.5,
            ambiguity: 0.5,
            formed_at: Utc::now(),
            temporal_stability_secs: 0.0,
            competing_with: vec![],
            converged: false,
        };
        assert!(!non_safety.is_safety_relevant());
    }

    #[test]
    fn self_meaning_sufficient_evidence() {
        let meaning = SelfMeaning {
            id: MeaningId::new(),
            category: SelfMeaningCategory::PerformanceBottleneck {
                component: "test".into(),
                severity: 0.5,
                root_causes: vec![],
            },
            evidence: vec![
                Evidence {
                    source: "test".into(),
                    strength: 0.8,
                    timestamp: Utc::now(),
                    description: "desc".into(),
                    category: EvidenceCategory::Anomaly,
                },
            ],
            confidence: 0.5,
            ambiguity: 0.5,
            formed_at: Utc::now(),
            temporal_stability_secs: 0.0,
            competing_with: vec![],
            converged: false,
        };
        assert!(meaning.has_sufficient_evidence(1));
        assert!(!meaning.has_sufficient_evidence(5));
    }

    #[test]
    fn meaning_category_primary_component() {
        let cat = SelfMeaningCategory::PerformanceBottleneck {
            component: "event-fabric".into(),
            severity: 0.7,
            root_causes: vec![],
        };
        assert_eq!(cat.primary_component(), Some("event-fabric"));

        let cat = SelfMeaningCategory::ArchitecturalInsight {
            insight_type: ArchitecturalInsightType::LayerViolation,
            affected_components: vec!["a".into(), "b".into()],
            structural_pressure: "test".into(),
        };
        assert_eq!(cat.primary_component(), None);
    }

    #[test]
    fn self_meaning_serialization() {
        let meaning = SelfMeaning {
            id: MeaningId::new(),
            category: SelfMeaningCategory::PerformanceBottleneck {
                component: "gate".into(),
                severity: 0.8,
                root_causes: vec![RootCauseHypothesis {
                    description: "policy set too large".into(),
                    confidence: 0.6,
                    supporting_evidence: vec!["latency correlates with policy count".into()],
                }],
            },
            evidence: vec![Evidence {
                source: "statistical-detector".into(),
                strength: 0.9,
                timestamp: Utc::now(),
                description: "z-score 5.2".into(),
                category: EvidenceCategory::Anomaly,
            }],
            confidence: 0.75,
            ambiguity: 0.25,
            formed_at: Utc::now(),
            temporal_stability_secs: 1800.0,
            competing_with: vec![],
            converged: false,
        };
        let json = serde_json::to_string(&meaning).unwrap();
        let restored: SelfMeaning = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, meaning.id);
        assert!((restored.confidence - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn meaning_config_defaults() {
        let cfg = MeaningConfig::default();
        assert!((cfg.convergence_threshold - 0.85).abs() < f64::EPSILON);
        assert_eq!(cfg.min_evidence_count, 10);
        assert_eq!(cfg.min_observation_secs, 3600);
        assert!((cfg.resolution_threshold - 0.2).abs() < f64::EPSILON);
        assert!((cfg.safety_resolution_threshold - 0.1).abs() < f64::EPSILON);
        assert_eq!(cfg.max_active_meanings, 256);
        assert_eq!(cfg.max_history, 1024);
    }
}
