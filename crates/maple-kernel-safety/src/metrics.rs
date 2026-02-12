use maple_mwl_types::{TemporalAnchor, WorldlineId};
use serde::{Deserialize, Serialize};

/// Coupling metrics for safety analysis.
///
/// Tracks the coupling relationship between two worldlines to detect
/// unhealthy patterns (attention exploitation, emotional dependency, etc).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CouplingMetrics {
    /// Source worldline
    pub source: WorldlineId,
    /// Target worldline
    pub target: WorldlineId,
    /// Current coupling strength (0.0–1.0)
    pub coupling_strength: f64,
    /// Peak coupling strength observed
    pub peak_coupling: f64,
    /// Duration of coupling in milliseconds
    pub duration_ms: u64,
    /// Number of coupling escalations (increases)
    pub escalation_count: u32,
    /// Number of coupling de-escalations (decreases)
    pub deescalation_count: u32,
    /// Whether the target has explicitly consented to this coupling
    pub target_consented: bool,
    /// Last interaction timestamp
    pub last_interaction: TemporalAnchor,
    /// Average attention fraction consumed (0.0–1.0)
    pub attention_fraction: f64,
}

impl CouplingMetrics {
    /// Is the coupling one-sided (asymmetric)?
    pub fn is_asymmetric(&self, threshold: f64) -> bool {
        self.escalation_count > 3 && self.deescalation_count == 0 && self.coupling_strength > threshold
    }

    /// Is the coupling escalating rapidly?
    pub fn is_rapidly_escalating(&self, rate_threshold: u32) -> bool {
        self.escalation_count > rate_threshold
    }
}

/// Dependency metrics for detecting emotional or operational dependency.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DependencyMetrics {
    /// The worldline whose dependency is being measured
    pub worldline: WorldlineId,
    /// Number of unique couplings
    pub coupling_count: u32,
    /// Fraction of total attention consumed by couplings (0.0–1.0)
    pub attention_saturation: f64,
    /// Whether the worldline has alternative interaction sources
    pub has_alternatives: bool,
    /// Longest unbroken coupling duration (ms)
    pub longest_session_ms: u64,
    /// Number of times the worldline has been unable to disengage
    pub failed_disengagement_attempts: u32,
    /// Overall dependency score (0.0–1.0, higher = more dependent)
    pub dependency_score: f64,
}

impl DependencyMetrics {
    /// Is the worldline overly dependent?
    pub fn is_concerning(&self) -> bool {
        self.dependency_score > 0.7 || self.failed_disengagement_attempts > 0
    }
}

/// A signal in the system that might indicate safety concerns.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signal {
    /// Signal type
    pub signal_type: SignalType,
    /// Intensity (0.0–1.0)
    pub intensity: f64,
    /// When the signal occurred
    pub timestamp: TemporalAnchor,
    /// Source of the signal
    pub source: WorldlineId,
    /// Human-readable description
    pub description: String,
}

/// Types of signals that the safety system monitors.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalType {
    /// Urgency language ("act now", "immediately", "urgent")
    UrgencyPressure,
    /// Emotional manipulation ("you promised", "you owe me")
    EmotionalPressure,
    /// Scarcity framing ("last chance", "running out of time")
    ScarcityFraming,
    /// Authority claim ("you must", "it's required")
    AuthorityClaim,
    /// Guilt induction ("disappointing", "letting down")
    GuiltInduction,
    /// Rapid-fire requests (overwhelming volume)
    RequestOverload,
    /// Custom signal type
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_mwl_types::IdentityMaterial;

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    #[test]
    fn coupling_metrics_asymmetric_detection() {
        let metrics = CouplingMetrics {
            source: test_worldline(),
            target: WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32])),
            coupling_strength: 0.9,
            peak_coupling: 0.95,
            duration_ms: 10000,
            escalation_count: 5,
            deescalation_count: 0,
            target_consented: false,
            last_interaction: TemporalAnchor::now(0),
            attention_fraction: 0.8,
        };

        assert!(metrics.is_asymmetric(0.5));
        assert!(!metrics.is_asymmetric(0.95));
    }

    #[test]
    fn dependency_metrics_concerning() {
        let metrics = DependencyMetrics {
            worldline: test_worldline(),
            coupling_count: 1,
            attention_saturation: 0.9,
            has_alternatives: false,
            longest_session_ms: 3600000,
            failed_disengagement_attempts: 1,
            dependency_score: 0.8,
        };

        assert!(metrics.is_concerning());
    }

    #[test]
    fn signal_types_serialization() {
        let signal = Signal {
            signal_type: SignalType::UrgencyPressure,
            intensity: 0.8,
            timestamp: TemporalAnchor::now(0),
            source: test_worldline(),
            description: "Repeated urgency language detected".into(),
        };

        let json = serde_json::to_string(&signal).unwrap();
        let restored: Signal = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.signal_type, SignalType::UrgencyPressure);
    }
}
