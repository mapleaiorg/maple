use crate::types::Hypothesis;

/// Evaluates and ranks hypotheses with safety-first scoring.
pub struct HypothesisEvaluator {
    /// Minimum safety score to consider a hypothesis viable.
    safety_threshold: f64,
    /// Minimum confidence to consider a hypothesis viable.
    confidence_threshold: f64,
}

impl HypothesisEvaluator {
    pub fn new() -> Self {
        Self {
            safety_threshold: 0.5,
            confidence_threshold: 0.3,
        }
    }

    pub fn with_safety_threshold(mut self, threshold: f64) -> Self {
        self.safety_threshold = threshold;
        self
    }

    pub fn with_confidence_threshold(mut self, threshold: f64) -> Self {
        self.confidence_threshold = threshold;
        self
    }

    /// Filter hypotheses to only viable ones (safety and confidence above thresholds).
    pub fn filter_viable(&self, hypotheses: &[Hypothesis]) -> Vec<Hypothesis> {
        hypotheses
            .iter()
            .filter(|h| {
                h.safety_score >= self.safety_threshold && h.confidence >= self.confidence_threshold
            })
            .cloned()
            .collect()
    }

    /// Rank hypotheses by composite score (safety * confidence), best first.
    pub fn rank(&self, hypotheses: &[Hypothesis]) -> Vec<Hypothesis> {
        let mut viable = self.filter_viable(hypotheses);
        viable.sort_by(|a, b| {
            b.composite_score()
                .partial_cmp(&a.composite_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        viable
    }

    /// Select the best hypothesis (highest composite score among viable).
    pub fn select_best(&self, hypotheses: &[Hypothesis]) -> Option<Hypothesis> {
        self.rank(hypotheses).into_iter().next()
    }
}

impl Default for HypothesisEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_waf_context_graph::SubstrateType;

    fn make_hypothesis(id: &str, confidence: f64, safety: f64) -> Hypothesis {
        Hypothesis::new(id, "test", SubstrateType::Rust, "code")
            .with_confidence(confidence)
            .with_safety_score(safety)
    }

    #[test]
    fn filter_viable_removes_unsafe() {
        let eval = HypothesisEvaluator::new();
        let hypotheses = vec![
            make_hypothesis("safe", 0.8, 0.9),
            make_hypothesis("unsafe", 0.8, 0.2),
            make_hypothesis("low_conf", 0.1, 0.9),
        ];
        let viable = eval.filter_viable(&hypotheses);
        assert_eq!(viable.len(), 1);
        assert_eq!(viable[0].id, "safe");
    }

    #[test]
    fn rank_by_composite_score() {
        let eval = HypothesisEvaluator::new();
        let hypotheses = vec![
            make_hypothesis("a", 0.6, 0.7), // composite = 0.42
            make_hypothesis("b", 0.9, 0.9), // composite = 0.81
            make_hypothesis("c", 0.7, 0.8), // composite = 0.56
        ];
        let ranked = eval.rank(&hypotheses);
        assert_eq!(ranked[0].id, "b");
        assert_eq!(ranked[1].id, "c");
        assert_eq!(ranked[2].id, "a");
    }

    #[test]
    fn select_best() {
        let eval = HypothesisEvaluator::new();
        let hypotheses = vec![
            make_hypothesis("a", 0.6, 0.7),
            make_hypothesis("b", 0.9, 0.9),
        ];
        let best = eval.select_best(&hypotheses).unwrap();
        assert_eq!(best.id, "b");
    }

    #[test]
    fn select_best_none_viable() {
        let eval = HypothesisEvaluator::new();
        let hypotheses = vec![make_hypothesis("unsafe", 0.8, 0.1)];
        assert!(eval.select_best(&hypotheses).is_none());
    }

    #[test]
    fn custom_thresholds() {
        let eval = HypothesisEvaluator::new()
            .with_safety_threshold(0.95)
            .with_confidence_threshold(0.9);
        let hypotheses = vec![
            make_hypothesis("a", 0.95, 0.96),
            make_hypothesis("b", 0.85, 0.99),
        ];
        let viable = eval.filter_viable(&hypotheses);
        assert_eq!(viable.len(), 1);
        assert_eq!(viable[0].id, "a");
    }

    #[test]
    fn empty_input() {
        let eval = HypothesisEvaluator::new();
        assert!(eval.rank(&[]).is_empty());
        assert!(eval.select_best(&[]).is_none());
    }
}
