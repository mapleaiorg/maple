use serde::{Deserialize, Serialize};

/// Confidence profile for intent stabilization.
/// Per Resonance Architecture v1.1 §5.6:
/// "A Resonator MUST explicitly track uncertainty in intent.
///  Binary intent (present/absent) is insufficient."
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfidenceProfile {
    /// Overall confidence 0.0-1.0
    pub overall: f64,
    /// How stable over recent signals
    pub stability: f64,
    /// Agreement among signals
    pub signal_consistency: f64,
    /// Alignment with past behavior
    pub historical_alignment: f64,
}

impl ConfidenceProfile {
    pub fn new(overall: f64, stability: f64, consistency: f64, alignment: f64) -> Self {
        Self {
            overall: overall.clamp(0.0, 1.0),
            stability: stability.clamp(0.0, 1.0),
            signal_consistency: consistency.clamp(0.0, 1.0),
            historical_alignment: alignment.clamp(0.0, 1.0),
        }
    }

    pub fn is_sufficient_for_commitment(&self, threshold: f64) -> bool {
        self.overall >= threshold
    }
}

/// Risk classification for capabilities and commitments.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RiskClass {
    Low,
    Medium,
    High,
    Critical,
}

/// Risk level with optional numeric score.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskLevel {
    pub class: RiskClass,
    pub score: Option<f64>,
    pub factors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_clamps_to_valid_range() {
        let c = ConfidenceProfile::new(1.5, -0.3, 0.5, 2.0);
        assert_eq!(c.overall, 1.0);
        assert_eq!(c.stability, 0.0);
        assert_eq!(c.signal_consistency, 0.5);
        assert_eq!(c.historical_alignment, 1.0);
    }

    #[test]
    fn is_sufficient_for_commitment_respects_threshold() {
        let high = ConfidenceProfile::new(0.9, 0.8, 0.7, 0.8);
        let low = ConfidenceProfile::new(0.3, 0.2, 0.1, 0.2);

        assert!(high.is_sufficient_for_commitment(0.8));
        assert!(!high.is_sufficient_for_commitment(0.95));
        assert!(!low.is_sufficient_for_commitment(0.5));
        assert!(low.is_sufficient_for_commitment(0.3));
    }

    #[test]
    fn risk_class_ordering() {
        assert!(RiskClass::Low < RiskClass::Medium);
        assert!(RiskClass::Medium < RiskClass::High);
        assert!(RiskClass::High < RiskClass::Critical);
    }

    #[test]
    fn confidence_serialization_roundtrip() {
        let c = ConfidenceProfile::new(0.85, 0.7, 0.9, 0.6);
        let json = serde_json::to_string(&c).unwrap();
        let restored: ConfidenceProfile = serde_json::from_str(&json).unwrap();
        assert!((c.overall - restored.overall).abs() < f64::EPSILON);
        assert!((c.stability - restored.stability).abs() < f64::EPSILON);
    }

    #[test]
    fn risk_level_serialization() {
        let r = RiskLevel {
            class: RiskClass::High,
            score: Some(0.82),
            factors: vec!["financial exposure".into(), "irreversible".into()],
        };
        let json = serde_json::to_string(&r).unwrap();
        let restored: RiskLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.class, RiskClass::High);
        assert_eq!(restored.factors.len(), 2);
    }
}
