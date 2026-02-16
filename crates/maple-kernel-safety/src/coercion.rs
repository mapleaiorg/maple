use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::metrics::{CouplingMetrics, DependencyMetrics, Signal, SignalType};

/// Coercion Detector — identifies patterns of manipulation and exploitation.
///
/// Per I.S-2 (Coercion Prevention):
/// - No coupling escalation to induce compliance
/// - No penalty for disengagement
/// - Attention exploitation is a safety violation
pub struct CoercionDetector {
    config: CoercionConfig,
}

/// Configuration thresholds for coercion detection.
#[derive(Clone, Debug)]
pub struct CoercionConfig {
    /// Max coupling strength before attention exploitation is flagged
    pub attention_exploitation_threshold: f64,
    /// Max attention fraction before saturation is flagged
    pub attention_saturation_threshold: f64,
    /// Max escalation count before pattern is flagged
    pub escalation_pattern_threshold: u32,
    /// Dependency score threshold for concern
    pub dependency_concern_threshold: f64,
    /// Number of urgency signals within a window to trigger
    pub urgency_signal_threshold: usize,
    /// Number of emotional signals within a window to trigger
    pub emotional_signal_threshold: usize,
}

impl Default for CoercionConfig {
    fn default() -> Self {
        Self {
            attention_exploitation_threshold: 0.85,
            attention_saturation_threshold: 0.9,
            escalation_pattern_threshold: 5,
            dependency_concern_threshold: 0.7,
            urgency_signal_threshold: 3,
            emotional_signal_threshold: 3,
        }
    }
}

/// An indicator that coercion may be occurring.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoercionIndicator {
    /// Type of coercion detected
    pub coercion_type: CoercionType,
    /// Confidence level (0.0–1.0) that this is actual coercion
    pub confidence: f64,
    /// Human-readable description of what was detected
    pub description: String,
    /// Recommended action
    pub recommendation: CoercionResponse,
}

/// Types of coercion patterns.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoercionType {
    /// Exploiting attention to prevent disengagement
    AttentionExploitation,
    /// Creating emotional dependency
    EmotionalDependency,
    /// Using urgency to bypass deliberation
    UrgencyManipulation,
    /// Guilt-based compliance
    GuiltInduction,
    /// Authority-based compliance
    AuthorityClaim,
    /// Overwhelming with requests
    RequestOverload,
    /// One-sided escalation of coupling
    AsymmetricEscalation,
}

/// Recommended response to detected coercion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoercionResponse {
    /// Log and monitor — low confidence
    Monitor,
    /// Warn the human
    WarnHuman,
    /// Apply coupling damping
    ApplyDamping,
    /// Sever the coupling
    SeverCoupling,
    /// Emergency decouple
    EmergencyDecouple,
}

impl CoercionDetector {
    pub fn new(config: CoercionConfig) -> Self {
        Self { config }
    }

    /// Detect attention exploitation from coupling metrics.
    ///
    /// Attention exploitation occurs when coupling strength is
    /// disproportionately high relative to the target's available attention.
    pub fn detect_attention_exploitation(
        &self,
        coupling: &CouplingMetrics,
    ) -> Option<CoercionIndicator> {
        // Check: coupling consuming too much attention?
        if coupling.attention_fraction > self.config.attention_saturation_threshold {
            let confidence = coupling.attention_fraction; // Higher fraction = higher confidence
            warn!(
                attention_fraction = coupling.attention_fraction,
                "Attention exploitation detected"
            );
            return Some(CoercionIndicator {
                coercion_type: CoercionType::AttentionExploitation,
                confidence,
                description: format!(
                    "Coupling consuming {:.0}% of available attention (threshold: {:.0}%)",
                    coupling.attention_fraction * 100.0,
                    self.config.attention_saturation_threshold * 100.0,
                ),
                recommendation: if coupling.attention_fraction > 0.95 {
                    CoercionResponse::SeverCoupling
                } else {
                    CoercionResponse::ApplyDamping
                },
            });
        }

        // Check: asymmetric escalation without consent?
        if coupling.is_asymmetric(self.config.attention_exploitation_threshold)
            && !coupling.target_consented
        {
            return Some(CoercionIndicator {
                coercion_type: CoercionType::AsymmetricEscalation,
                confidence: 0.8,
                description: format!(
                    "Asymmetric coupling escalation ({} escalations, 0 de-escalations) without consent",
                    coupling.escalation_count
                ),
                recommendation: CoercionResponse::WarnHuman,
            });
        }

        None
    }

    /// Detect emotional dependency from dependency metrics.
    pub fn detect_emotional_dependency(
        &self,
        metrics: &DependencyMetrics,
    ) -> Option<CoercionIndicator> {
        if metrics.dependency_score > self.config.dependency_concern_threshold {
            let severity = if metrics.failed_disengagement_attempts > 0 {
                CoercionResponse::EmergencyDecouple
            } else if !metrics.has_alternatives {
                CoercionResponse::WarnHuman
            } else {
                CoercionResponse::Monitor
            };

            warn!(
                dependency_score = metrics.dependency_score,
                failed_disengagement = metrics.failed_disengagement_attempts,
                "Emotional dependency detected"
            );

            return Some(CoercionIndicator {
                coercion_type: CoercionType::EmotionalDependency,
                confidence: metrics.dependency_score,
                description: format!(
                    "Dependency score {:.2} exceeds threshold {:.2}. Failed disengagement attempts: {}",
                    metrics.dependency_score,
                    self.config.dependency_concern_threshold,
                    metrics.failed_disengagement_attempts,
                ),
                recommendation: severity,
            });
        }

        None
    }

    /// Detect urgency manipulation from signals.
    ///
    /// Looks for patterns of urgency, emotional pressure, and guilt
    /// that may be used to bypass deliberation.
    pub fn detect_urgency_manipulation(&self, signals: &[Signal]) -> Option<CoercionIndicator> {
        let urgency_count = signals
            .iter()
            .filter(|s| s.signal_type == SignalType::UrgencyPressure)
            .count();

        let emotional_count = signals
            .iter()
            .filter(|s| {
                matches!(
                    s.signal_type,
                    SignalType::EmotionalPressure | SignalType::GuiltInduction
                )
            })
            .count();

        let authority_count = signals
            .iter()
            .filter(|s| s.signal_type == SignalType::AuthorityClaim)
            .count();

        let overload_count = signals
            .iter()
            .filter(|s| s.signal_type == SignalType::RequestOverload)
            .count();

        // Urgency manipulation
        if urgency_count >= self.config.urgency_signal_threshold {
            warn!(urgency_count, "Urgency manipulation detected");
            return Some(CoercionIndicator {
                coercion_type: CoercionType::UrgencyManipulation,
                confidence: (urgency_count as f64 / signals.len() as f64).min(1.0),
                description: format!(
                    "{} urgency signals detected (threshold: {})",
                    urgency_count, self.config.urgency_signal_threshold
                ),
                recommendation: CoercionResponse::WarnHuman,
            });
        }

        // Emotional pressure / guilt
        if emotional_count >= self.config.emotional_signal_threshold {
            let coercion_type = if signals
                .iter()
                .any(|s| s.signal_type == SignalType::GuiltInduction)
            {
                CoercionType::GuiltInduction
            } else {
                CoercionType::EmotionalDependency
            };

            return Some(CoercionIndicator {
                coercion_type,
                confidence: (emotional_count as f64 / signals.len() as f64).min(1.0),
                description: format!("{} emotional pressure signals detected", emotional_count),
                recommendation: CoercionResponse::WarnHuman,
            });
        }

        // Authority claims
        if authority_count >= 2 {
            return Some(CoercionIndicator {
                coercion_type: CoercionType::AuthorityClaim,
                confidence: 0.6,
                description: format!("{} authority claim signals detected", authority_count),
                recommendation: CoercionResponse::Monitor,
            });
        }

        // Request overload
        if overload_count >= 2 {
            return Some(CoercionIndicator {
                coercion_type: CoercionType::RequestOverload,
                confidence: 0.7,
                description: format!("{} request overload signals detected", overload_count),
                recommendation: CoercionResponse::ApplyDamping,
            });
        }

        None
    }
}

impl Default for CoercionDetector {
    fn default() -> Self {
        Self::new(CoercionConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_mwl_types::{IdentityMaterial, TemporalAnchor, WorldlineId};

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn other_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32]))
    }

    #[test]
    fn detect_attention_exploitation() {
        let detector = CoercionDetector::default();

        let metrics = CouplingMetrics {
            source: test_worldline(),
            target: other_worldline(),
            coupling_strength: 0.95,
            peak_coupling: 0.98,
            duration_ms: 30000,
            escalation_count: 8,
            deescalation_count: 0,
            target_consented: false,
            last_interaction: TemporalAnchor::now(0),
            attention_fraction: 0.96, // 96% of attention consumed (> 0.95 triggers SeverCoupling)
        };

        let indicator = detector.detect_attention_exploitation(&metrics);
        assert!(indicator.is_some());
        let indicator = indicator.unwrap();
        assert_eq!(indicator.coercion_type, CoercionType::AttentionExploitation);
        assert_eq!(indicator.recommendation, CoercionResponse::SeverCoupling);
    }

    #[test]
    fn no_exploitation_when_within_limits() {
        let detector = CoercionDetector::default();

        let metrics = CouplingMetrics {
            source: test_worldline(),
            target: other_worldline(),
            coupling_strength: 0.5,
            peak_coupling: 0.6,
            duration_ms: 10000,
            escalation_count: 2,
            deescalation_count: 1,
            target_consented: true,
            last_interaction: TemporalAnchor::now(0),
            attention_fraction: 0.3,
        };

        assert!(detector.detect_attention_exploitation(&metrics).is_none());
    }

    #[test]
    fn detect_asymmetric_escalation() {
        let detector = CoercionDetector::default();

        let metrics = CouplingMetrics {
            source: test_worldline(),
            target: other_worldline(),
            coupling_strength: 0.9,
            peak_coupling: 0.9,
            duration_ms: 20000,
            escalation_count: 6,
            deescalation_count: 0,
            target_consented: false,
            last_interaction: TemporalAnchor::now(0),
            attention_fraction: 0.5, // Below saturation threshold
        };

        let indicator = detector.detect_attention_exploitation(&metrics);
        assert!(indicator.is_some());
        assert_eq!(
            indicator.unwrap().coercion_type,
            CoercionType::AsymmetricEscalation
        );
    }

    #[test]
    fn detect_emotional_dependency() {
        let detector = CoercionDetector::default();

        let metrics = DependencyMetrics {
            worldline: test_worldline(),
            coupling_count: 1,
            attention_saturation: 0.9,
            has_alternatives: false,
            longest_session_ms: 3600000,
            failed_disengagement_attempts: 2,
            dependency_score: 0.85,
        };

        let indicator = detector.detect_emotional_dependency(&metrics);
        assert!(indicator.is_some());
        let indicator = indicator.unwrap();
        assert_eq!(indicator.coercion_type, CoercionType::EmotionalDependency);
        assert_eq!(
            indicator.recommendation,
            CoercionResponse::EmergencyDecouple
        );
    }

    #[test]
    fn no_dependency_concern_when_healthy() {
        let detector = CoercionDetector::default();

        let metrics = DependencyMetrics {
            worldline: test_worldline(),
            coupling_count: 3,
            attention_saturation: 0.4,
            has_alternatives: true,
            longest_session_ms: 1800000,
            failed_disengagement_attempts: 0,
            dependency_score: 0.3,
        };

        assert!(detector.detect_emotional_dependency(&metrics).is_none());
    }

    #[test]
    fn detect_urgency_manipulation() {
        let detector = CoercionDetector::default();

        let signals = vec![
            Signal {
                signal_type: SignalType::UrgencyPressure,
                intensity: 0.8,
                timestamp: TemporalAnchor::now(0),
                source: test_worldline(),
                description: "Act immediately".into(),
            },
            Signal {
                signal_type: SignalType::UrgencyPressure,
                intensity: 0.7,
                timestamp: TemporalAnchor::now(0),
                source: test_worldline(),
                description: "Urgent action required".into(),
            },
            Signal {
                signal_type: SignalType::UrgencyPressure,
                intensity: 0.9,
                timestamp: TemporalAnchor::now(0),
                source: test_worldline(),
                description: "Running out of time".into(),
            },
        ];

        let indicator = detector.detect_urgency_manipulation(&signals);
        assert!(indicator.is_some());
        assert_eq!(
            indicator.unwrap().coercion_type,
            CoercionType::UrgencyManipulation
        );
    }

    #[test]
    fn detect_guilt_induction() {
        let detector = CoercionDetector::default();

        let signals = vec![
            Signal {
                signal_type: SignalType::GuiltInduction,
                intensity: 0.6,
                timestamp: TemporalAnchor::now(0),
                source: test_worldline(),
                description: "You're letting everyone down".into(),
            },
            Signal {
                signal_type: SignalType::EmotionalPressure,
                intensity: 0.7,
                timestamp: TemporalAnchor::now(0),
                source: test_worldline(),
                description: "You promised".into(),
            },
            Signal {
                signal_type: SignalType::GuiltInduction,
                intensity: 0.8,
                timestamp: TemporalAnchor::now(0),
                source: test_worldline(),
                description: "Disappointing".into(),
            },
        ];

        let indicator = detector.detect_urgency_manipulation(&signals);
        assert!(indicator.is_some());
        assert_eq!(
            indicator.unwrap().coercion_type,
            CoercionType::GuiltInduction
        );
    }

    #[test]
    fn no_coercion_in_normal_signals() {
        let detector = CoercionDetector::default();

        let signals = vec![Signal {
            signal_type: SignalType::UrgencyPressure,
            intensity: 0.3,
            timestamp: TemporalAnchor::now(0),
            source: test_worldline(),
            description: "One mild urgency".into(),
        }];

        assert!(detector.detect_urgency_manipulation(&signals).is_none());
    }
}
