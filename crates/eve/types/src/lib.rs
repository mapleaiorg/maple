//! EVE Types - Epistemic Validation Engine types
//!
//! EVE learns from consequences but has NO authority.
//! It provides insights that ONLY inform policy updates through proper channels.

#![deny(unsafe_code)]

use rcf_commitment::CommitmentId;
use rcf_types::EffectDomain;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A consequence - the observable effect of a commitment execution
/// (Local definition for EVE layer - decoupled from mapleverse-types)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Consequence {
    /// Unique consequence identifier
    pub consequence_id: String,
    /// The commitment that produced this consequence
    pub commitment_id: CommitmentId,
    /// Effect domain
    pub effect_domain: EffectDomain,
    /// Description of the consequence
    pub description: String,
    /// When the consequence occurred
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    /// Whether the consequence is reversible
    pub reversible: bool,
}

/// A learning artifact from consequence analysis
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LearningArtifact {
    pub artifact_id: ArtifactId,
    pub artifact_type: ArtifactType,
    pub source_commitment_ids: Vec<CommitmentId>,
    pub domain: EffectDomain,
    pub content: ArtifactContent,
    pub confidence: ConfidenceScore,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

/// Unique identifier for a learning artifact
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactId(pub String);

impl ArtifactId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

/// Types of learning artifacts
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactType {
    /// Pattern observed across consequences
    Pattern,
    /// Correlation between commitment characteristics and outcomes
    Correlation,
    /// Anomaly detected in consequence patterns
    Anomaly,
    /// Suggested policy improvement (NO authority to implement)
    PolicySuggestion,
    /// Risk factor identified
    RiskFactor,
    /// Performance metric
    Metric,
}

/// Content of a learning artifact
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArtifactContent {
    pub summary: String,
    pub details: String,
    pub data: HashMap<String, serde_json::Value>,
}

/// Confidence score for a learning artifact
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfidenceScore {
    pub score: f64,
    pub sample_size: usize,
    pub methodology: String,
}

impl ConfidenceScore {
    pub fn new(score: f64, sample_size: usize) -> Self {
        Self {
            score: score.clamp(0.0, 1.0),
            sample_size,
            methodology: "statistical".to_string(),
        }
    }

    pub fn low(sample_size: usize) -> Self {
        Self::new(0.3, sample_size)
    }

    pub fn medium(sample_size: usize) -> Self {
        Self::new(0.6, sample_size)
    }

    pub fn high(sample_size: usize) -> Self {
        Self::new(0.9, sample_size)
    }
}

/// A consequence record for EVE analysis
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsequenceRecord {
    pub record_id: String,
    pub consequence: Consequence,
    pub commitment_characteristics: CommitmentCharacteristics,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
    pub analysis_status: AnalysisStatus,
}

/// Characteristics of the commitment that produced a consequence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitmentCharacteristics {
    pub domain: EffectDomain,
    pub risk_level: String,
    pub scope_size: String,
    pub reversibility: String,
    pub agent_history_length: usize,
}

/// Status of consequence analysis
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnalysisStatus {
    Pending,
    Analyzed,
    IncorporatedIntoArtifact,
}

/// Query for learning artifacts
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ArtifactQuery {
    pub artifact_type: Option<ArtifactType>,
    pub domain: Option<EffectDomain>,
    pub min_confidence: Option<f64>,
    pub after: Option<chrono::DateTime<chrono::Utc>>,
    pub limit: Option<usize>,
}

/// EVE insight - a read-only recommendation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EveInsight {
    pub insight_id: String,
    pub insight_type: InsightType,
    pub description: String,
    pub supporting_artifacts: Vec<ArtifactId>,
    pub confidence: ConfidenceScore,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Types of insights EVE can provide
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsightType {
    /// A pattern that suggests potential risk
    RiskPattern,
    /// A suggestion for policy improvement (NO authority)
    PolicySuggestion,
    /// An observed trend in consequences
    Trend,
    /// An anomaly that may warrant investigation
    Anomaly,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_score() {
        let score = ConfidenceScore::high(100);
        assert_eq!(score.score, 0.9);
        assert_eq!(score.sample_size, 100);
    }
}
