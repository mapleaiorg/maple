//! EVE Evaluation - Pattern analysis and learning
//!
//! Analyzes consequences to produce learning artifacts.
//! Has NO authority - only produces insights.

#![deny(unsafe_code)]

use eve_types::{
    ArtifactContent, ArtifactId, ArtifactType, ConfidenceScore, ConsequenceRecord, EveInsight,
    InsightType, LearningArtifact,
};
use rcf_types::EffectDomain;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;

/// Evaluation engine for consequence analysis
pub struct EvaluationEngine {
    artifacts: RwLock<HashMap<ArtifactId, LearningArtifact>>,
    domain_stats: RwLock<HashMap<EffectDomain, DomainStatistics>>,
}

impl EvaluationEngine {
    pub fn new() -> Self {
        Self {
            artifacts: RwLock::new(HashMap::new()),
            domain_stats: RwLock::new(HashMap::new()),
        }
    }

    /// Analyze a batch of consequence records
    pub fn analyze_batch(
        &self,
        records: &[ConsequenceRecord],
    ) -> Result<Vec<LearningArtifact>, EvaluationError> {
        let mut artifacts = vec![];

        // Update domain statistics
        for record in records {
            self.update_domain_stats(record)?;
        }

        // Look for patterns
        if let Some(pattern) = self.detect_pattern(records)? {
            artifacts.push(pattern);
        }

        // Look for anomalies
        if let Some(anomaly) = self.detect_anomaly(records)? {
            artifacts.push(anomaly);
        }

        // Store artifacts
        let mut stored = self
            .artifacts
            .write()
            .map_err(|_| EvaluationError::LockError)?;
        for artifact in &artifacts {
            stored.insert(artifact.artifact_id.clone(), artifact.clone());
        }

        Ok(artifacts)
    }

    /// Update domain statistics
    fn update_domain_stats(&self, record: &ConsequenceRecord) -> Result<(), EvaluationError> {
        let mut stats = self
            .domain_stats
            .write()
            .map_err(|_| EvaluationError::LockError)?;

        let domain_stat = stats
            .entry(record.commitment_characteristics.domain.clone())
            .or_insert_with(DomainStatistics::new);

        domain_stat.total_consequences += 1;

        // Track success/failure based on reversibility
        // Note: In EVE's model, we track reversible vs irreversible consequences
        // The "reversed_count" is a misnomer - it tracks reversible consequences
        if record.consequence.reversible {
            domain_stat.reversed_count += 1;
        } else {
            domain_stat.successful_count += 1;
        }

        Ok(())
    }

    /// Detect patterns in consequence records
    fn detect_pattern(
        &self,
        records: &[ConsequenceRecord],
    ) -> Result<Option<LearningArtifact>, EvaluationError> {
        if records.len() < 5 {
            return Ok(None);
        }

        // Simple pattern detection: look for common characteristics
        let mut domain_counts: HashMap<EffectDomain, usize> = HashMap::new();
        for record in records {
            *domain_counts
                .entry(record.commitment_characteristics.domain.clone())
                .or_insert(0) += 1;
        }

        // If one domain dominates, create a pattern artifact
        for (domain, count) in &domain_counts {
            if *count as f64 / records.len() as f64 > 0.7 {
                return Ok(Some(LearningArtifact {
                    artifact_id: ArtifactId::generate(),
                    artifact_type: ArtifactType::Pattern,
                    source_commitment_ids: records
                        .iter()
                        .map(|r| r.consequence.commitment_id.clone())
                        .collect(),
                    domain: domain.clone(),
                    content: ArtifactContent {
                        summary: format!("High concentration of {} domain consequences", domain),
                        details: format!(
                            "{} out of {} consequences are in the {} domain",
                            count,
                            records.len(),
                            domain
                        ),
                        data: HashMap::new(),
                    },
                    confidence: ConfidenceScore::new(
                        *count as f64 / records.len() as f64,
                        records.len(),
                    ),
                    created_at: chrono::Utc::now(),
                    metadata: HashMap::new(),
                }));
            }
        }

        Ok(None)
    }

    /// Detect anomalies in consequence records
    fn detect_anomaly(
        &self,
        records: &[ConsequenceRecord],
    ) -> Result<Option<LearningArtifact>, EvaluationError> {
        // Simple anomaly detection: look for unusual reversibility patterns
        let reversible_count = records.iter().filter(|r| r.consequence.reversible).count();

        if reversible_count as f64 / records.len() as f64 > 0.3 {
            return Ok(Some(LearningArtifact {
                artifact_id: ArtifactId::generate(),
                artifact_type: ArtifactType::Anomaly,
                source_commitment_ids: records
                    .iter()
                    .filter(|r| r.consequence.reversible)
                    .map(|r| r.consequence.commitment_id.clone())
                    .collect(),
                domain: records
                    .first()
                    .map(|r| r.commitment_characteristics.domain.clone())
                    .unwrap_or(EffectDomain::Computation),
                content: ArtifactContent {
                    summary: "High reversible consequence rate detected".to_string(),
                    details: format!(
                        "{} out of {} consequences are reversible",
                        reversible_count,
                        records.len()
                    ),
                    data: HashMap::new(),
                },
                confidence: ConfidenceScore::medium(records.len()),
                created_at: chrono::Utc::now(),
                metadata: HashMap::new(),
            }));
        }

        Ok(None)
    }

    /// Generate insights from artifacts
    pub fn generate_insights(&self) -> Result<Vec<EveInsight>, EvaluationError> {
        let artifacts = self
            .artifacts
            .read()
            .map_err(|_| EvaluationError::LockError)?;

        let mut insights = vec![];

        for artifact in artifacts.values() {
            match artifact.artifact_type {
                ArtifactType::Anomaly => {
                    insights.push(EveInsight {
                        insight_id: uuid::Uuid::new_v4().to_string(),
                        insight_type: InsightType::Anomaly,
                        description: artifact.content.summary.clone(),
                        supporting_artifacts: vec![artifact.artifact_id.clone()],
                        confidence: artifact.confidence.clone(),
                        created_at: chrono::Utc::now(),
                    });
                }
                ArtifactType::Pattern => {
                    insights.push(EveInsight {
                        insight_id: uuid::Uuid::new_v4().to_string(),
                        insight_type: InsightType::Trend,
                        description: artifact.content.summary.clone(),
                        supporting_artifacts: vec![artifact.artifact_id.clone()],
                        confidence: artifact.confidence.clone(),
                        created_at: chrono::Utc::now(),
                    });
                }
                _ => {}
            }
        }

        Ok(insights)
    }

    /// Get all artifacts
    pub fn get_artifacts(&self) -> Result<Vec<LearningArtifact>, EvaluationError> {
        let artifacts = self
            .artifacts
            .read()
            .map_err(|_| EvaluationError::LockError)?;
        Ok(artifacts.values().cloned().collect())
    }
}

impl Default for EvaluationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for a domain
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DomainStatistics {
    pub total_consequences: usize,
    pub successful_count: usize,
    pub reversed_count: usize,
}

impl DomainStatistics {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Evaluation errors
#[derive(Debug, Error)]
pub enum EvaluationError {
    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),

    #[error("Insufficient data")]
    InsufficientData,

    #[error("Lock error")]
    LockError,
}

#[cfg(test)]
mod tests {
    use super::*;
    use eve_types::{CommitmentCharacteristics, Consequence};
    use rcf_commitment::CommitmentId;

    fn create_test_record(domain: EffectDomain) -> ConsequenceRecord {
        ConsequenceRecord {
            record_id: uuid::Uuid::new_v4().to_string(),
            consequence: Consequence {
                consequence_id: uuid::Uuid::new_v4().to_string(),
                commitment_id: CommitmentId::generate(),
                effect_domain: domain.clone(),
                description: "Test".to_string(),
                occurred_at: chrono::Utc::now(),
                reversible: false,
            },
            commitment_characteristics: CommitmentCharacteristics {
                domain,
                risk_level: "low".to_string(),
                scope_size: "small".to_string(),
                reversibility: "irreversible".to_string(),
                agent_history_length: 10,
            },
            recorded_at: chrono::Utc::now(),
            analysis_status: eve_types::AnalysisStatus::Pending,
        }
    }

    #[test]
    fn test_pattern_detection() {
        let engine = EvaluationEngine::new();

        // Create 10 records, 8 computation, 2 data
        let mut records: Vec<_> = (0..8)
            .map(|_| create_test_record(EffectDomain::Computation))
            .collect();
        records.extend((0..2).map(|_| create_test_record(EffectDomain::Data)));

        let artifacts = engine.analyze_batch(&records).unwrap();

        // Should detect a pattern (high concentration of Computation domain)
        assert!(artifacts
            .iter()
            .any(|a| a.artifact_type == ArtifactType::Pattern));
    }
}
