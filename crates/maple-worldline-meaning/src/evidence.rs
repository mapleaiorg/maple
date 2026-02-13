//! Evidence evaluator — Bayesian updating of hypothesis confidence.
//!
//! The evidence evaluator scores hypotheses against available evidence using
//! simplified Bayesian updating. Evidence quality is assessed based on recency,
//! directness, and consistency.

use chrono::Utc;

use crate::types::{Evidence, EvidenceCategory};

// ── Confidence Update ───────────────────────────────────────────────────

/// Direction of a confidence update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateDirection {
    /// Evidence strengthened the hypothesis.
    Strengthened,
    /// Evidence weakened the hypothesis.
    Weakened,
    /// No significant change.
    Unchanged,
}

/// Result of a single Bayesian confidence update.
#[derive(Clone, Debug)]
pub struct ConfidenceUpdate {
    /// Confidence before the update.
    pub prior: f64,
    /// Confidence after the update.
    pub posterior: f64,
    /// Quality of the evidence used (0.0 to 1.0).
    pub quality: f64,
    /// Direction of the update.
    pub direction: UpdateDirection,
}

// ── Bayesian Updater ────────────────────────────────────────────────────

/// Simple Bayesian updater for hypothesis confidence.
///
/// Uses the simplified form:
/// `P(H|E) = P(E|H) * P(H) / (P(E|H) * P(H) + P(E|¬H) * P(¬H))`
///
/// Where likelihood is derived from evidence strength and quality.
pub struct BayesianUpdater;

impl BayesianUpdater {
    /// Update confidence given a likelihood value.
    ///
    /// The likelihood represents how likely we'd see this evidence if the
    /// hypothesis were true (0.0 to 1.0).
    ///
    /// Returns the posterior probability clamped to [0.01, 0.99].
    pub fn update(&self, prior: f64, likelihood: f64) -> f64 {
        let prior = prior.clamp(0.01, 0.99);
        let likelihood = likelihood.clamp(0.01, 0.99);

        let numerator = likelihood * prior;
        let denominator = numerator + (1.0 - likelihood) * (1.0 - prior);

        if denominator.abs() < f64::EPSILON {
            return prior;
        }

        (numerator / denominator).clamp(0.01, 0.99)
    }
}

// ── Evidence Quality Assessor ───────────────────────────────────────────

/// Assesses the quality of a piece of evidence.
///
/// Quality is a composite score considering recency, category weight,
/// and evidence strength.
pub struct EvidenceQualityAssessor {
    /// Weight given to evidence recency (0.0 to 1.0).
    pub recency_weight: f64,
    /// Weight given to evidence category (0.0 to 1.0).
    pub category_weight: f64,
    /// Weight given to evidence strength (0.0 to 1.0).
    pub strength_weight: f64,
}

impl Default for EvidenceQualityAssessor {
    fn default() -> Self {
        Self {
            recency_weight: 0.3,
            category_weight: 0.3,
            strength_weight: 0.4,
        }
    }
}

impl EvidenceQualityAssessor {
    /// Assess the quality of a piece of evidence.
    ///
    /// Returns a quality score from 0.0 (poor) to 1.0 (excellent).
    pub fn assess(&self, evidence: &Evidence) -> f64 {
        let recency_score = self.compute_recency(evidence);
        let category_score = self.compute_category_weight(evidence);
        let strength_score = evidence.strength.clamp(0.0, 1.0);

        let total_weight =
            self.recency_weight + self.category_weight + self.strength_weight;
        if total_weight.abs() < f64::EPSILON {
            return 0.5;
        }

        let quality = (recency_score * self.recency_weight
            + category_score * self.category_weight
            + strength_score * self.strength_weight)
            / total_weight;

        quality.clamp(0.0, 1.0)
    }

    /// Compute recency score: recent evidence is weighted higher.
    fn compute_recency(&self, evidence: &Evidence) -> f64 {
        let age_secs = (Utc::now() - evidence.timestamp)
            .num_seconds()
            .max(0) as f64;

        // Exponential decay: half-life of 1 hour (3600 seconds)
        let half_life = 3600.0;
        (-age_secs * (2.0_f64.ln()) / half_life).exp()
    }

    /// Category-based quality weight.
    fn compute_category_weight(&self, evidence: &Evidence) -> f64 {
        match evidence.category {
            EvidenceCategory::Anomaly => 0.9,       // Direct detection signal
            EvidenceCategory::Observation => 0.8,    // Direct measurement
            EvidenceCategory::Correlation => 0.7,    // Cross-signal
            EvidenceCategory::Historical => 0.6,     // Past pattern match
            EvidenceCategory::Absence => 0.4,        // Negative evidence (weakest)
        }
    }
}

// ── Evidence Evaluator ──────────────────────────────────────────────────

/// Main evidence evaluator that combines Bayesian updating with quality assessment.
pub struct EvidenceEvaluator {
    /// Bayesian updater for confidence computation.
    pub bayesian: BayesianUpdater,
    /// Evidence quality assessor.
    pub quality_assessor: EvidenceQualityAssessor,
}

impl Default for EvidenceEvaluator {
    fn default() -> Self {
        Self {
            bayesian: BayesianUpdater,
            quality_assessor: EvidenceQualityAssessor::default(),
        }
    }
}

impl EvidenceEvaluator {
    /// Update hypothesis confidence given new evidence.
    ///
    /// Computes the likelihood from evidence strength and quality,
    /// then applies Bayesian update.
    pub fn update_hypothesis(
        &self,
        prior: f64,
        evidence: &Evidence,
    ) -> ConfidenceUpdate {
        let quality = self.quality_assessor.assess(evidence);

        // Likelihood: combine evidence strength with quality
        // Strong + high-quality evidence → high likelihood
        let likelihood = (evidence.strength * quality).clamp(0.01, 0.99);

        let posterior = self.bayesian.update(prior, likelihood);

        let direction = if (posterior - prior).abs() < 0.01 {
            UpdateDirection::Unchanged
        } else if posterior > prior {
            UpdateDirection::Strengthened
        } else {
            UpdateDirection::Weakened
        };

        ConfidenceUpdate {
            prior,
            posterior,
            quality,
            direction,
        }
    }

    /// Evaluate a complete set of evidence against a hypothesis.
    ///
    /// Applies sequential Bayesian updates, starting from the given prior.
    /// Returns the final posterior confidence.
    pub fn evaluate_evidence_set(
        &self,
        prior: f64,
        evidence: &[Evidence],
    ) -> f64 {
        let mut confidence = prior;
        for e in evidence {
            let update = self.update_hypothesis(confidence, e);
            confidence = update.posterior;
        }
        confidence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recent_evidence(strength: f64, category: EvidenceCategory) -> Evidence {
        Evidence {
            source: "test".into(),
            strength,
            timestamp: Utc::now(),
            description: "test evidence".into(),
            category,
        }
    }

    #[test]
    fn bayesian_update_strong_evidence_increases_confidence() {
        let updater = BayesianUpdater;
        let prior = 0.5;
        let posterior = updater.update(prior, 0.9);
        assert!(posterior > prior, "Strong evidence should increase confidence");
    }

    #[test]
    fn bayesian_update_weak_evidence_decreases_confidence() {
        let updater = BayesianUpdater;
        let prior = 0.5;
        let posterior = updater.update(prior, 0.1);
        assert!(posterior < prior, "Weak evidence should decrease confidence");
    }

    #[test]
    fn bayesian_update_neutral_evidence_minimal_change() {
        let updater = BayesianUpdater;
        let prior = 0.5;
        let posterior = updater.update(prior, 0.5);
        assert!(
            (posterior - prior).abs() < 0.01,
            "Neutral evidence should barely change confidence"
        );
    }

    #[test]
    fn bayesian_update_clamped_bounds() {
        let updater = BayesianUpdater;
        // Very strong evidence on high prior
        let posterior = updater.update(0.99, 0.99);
        assert!(posterior <= 0.99);
        // Very weak evidence on low prior
        let posterior = updater.update(0.01, 0.01);
        assert!(posterior >= 0.01);
    }

    #[test]
    fn evidence_quality_assessor_category_weights() {
        let assessor = EvidenceQualityAssessor::default();

        let anomaly = recent_evidence(0.8, EvidenceCategory::Anomaly);
        let absence = recent_evidence(0.8, EvidenceCategory::Absence);

        let anomaly_quality = assessor.assess(&anomaly);
        let absence_quality = assessor.assess(&absence);

        assert!(
            anomaly_quality > absence_quality,
            "Anomaly evidence should be higher quality than absence"
        );
    }

    #[test]
    fn evidence_evaluator_strengthens_with_good_evidence() {
        let evaluator = EvidenceEvaluator::default();
        let evidence = recent_evidence(0.9, EvidenceCategory::Anomaly);
        let update = evaluator.update_hypothesis(0.5, &evidence);
        assert_eq!(update.direction, UpdateDirection::Strengthened);
        assert!(update.posterior > update.prior);
    }

    #[test]
    fn evidence_evaluator_sequential_updates() {
        let evaluator = EvidenceEvaluator::default();
        let evidence = vec![
            recent_evidence(0.8, EvidenceCategory::Anomaly),
            recent_evidence(0.7, EvidenceCategory::Observation),
            recent_evidence(0.6, EvidenceCategory::Correlation),
        ];
        let final_confidence = evaluator.evaluate_evidence_set(0.3, &evidence);
        assert!(
            final_confidence > 0.3,
            "Multiple supporting evidence should increase confidence"
        );
    }

    #[test]
    fn evidence_quality_recency_decay() {
        let assessor = EvidenceQualityAssessor::default();

        let recent = recent_evidence(0.8, EvidenceCategory::Observation);
        let old = Evidence {
            source: "test".into(),
            strength: 0.8,
            timestamp: Utc::now() - chrono::Duration::hours(24),
            description: "old evidence".into(),
            category: EvidenceCategory::Observation,
        };

        let recent_quality = assessor.assess(&recent);
        let old_quality = assessor.assess(&old);

        assert!(
            recent_quality > old_quality,
            "Recent evidence should be higher quality than old"
        );
    }
}
