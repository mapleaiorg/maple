//! Core types for the bootstrap protocol.
//!
//! Defines the 6 bootstrap phases, phase status, readiness scores,
//! configuration, and summary statistics.

use serde::{Deserialize, Serialize};

// ── Identifiers ─────────────────────────────────────────────────────

/// Unique identifier for a bootstrap operation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BootstrapId(pub String);

impl BootstrapId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for BootstrapId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BootstrapId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bootstrap:{}", self.0)
    }
}

// ── Bootstrap Phase ─────────────────────────────────────────────────

/// The 6 phases of the bootstrap protocol.
///
/// Each phase represents a level of self-hosting capability:
/// - Phase 0: External toolchain builds the system
/// - Phase 1: System can self-tune configuration
/// - Phase 2: System can self-generate operators
/// - Phase 3: System can self-regenerate modules
/// - Phase 4: System can self-produce languages
/// - Phase 5: System can self-describe its substrate
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BootstrapPhase {
    /// Phase 0: Built entirely by external toolchain (rustc, cargo).
    Phase0ExternalSubstrate,
    /// Phase 1: System can tune its own configuration parameters.
    Phase1ConfigSelfTuning,
    /// Phase 2: System can generate new operator implementations.
    Phase2OperatorSelfGeneration,
    /// Phase 3: System can regenerate its own modules from specs.
    Phase3ModuleSelfRegeneration,
    /// Phase 4: System can produce domain-specific languages.
    Phase4LanguageSelfProduction,
    /// Phase 5: System can describe its own substrate (full self-reference).
    Phase5SubstrateSelfDescription,
}

impl BootstrapPhase {
    /// Ordinal value (0-5) for this phase.
    pub fn ordinal(&self) -> u8 {
        match self {
            Self::Phase0ExternalSubstrate => 0,
            Self::Phase1ConfigSelfTuning => 1,
            Self::Phase2OperatorSelfGeneration => 2,
            Self::Phase3ModuleSelfRegeneration => 3,
            Self::Phase4LanguageSelfProduction => 4,
            Self::Phase5SubstrateSelfDescription => 5,
        }
    }

    /// Create from ordinal value.
    pub fn from_ordinal(ordinal: u8) -> Option<Self> {
        match ordinal {
            0 => Some(Self::Phase0ExternalSubstrate),
            1 => Some(Self::Phase1ConfigSelfTuning),
            2 => Some(Self::Phase2OperatorSelfGeneration),
            3 => Some(Self::Phase3ModuleSelfRegeneration),
            4 => Some(Self::Phase4LanguageSelfProduction),
            5 => Some(Self::Phase5SubstrateSelfDescription),
            _ => None,
        }
    }

    /// The next phase, if any.
    pub fn next(&self) -> Option<Self> {
        Self::from_ordinal(self.ordinal() + 1)
    }

    /// The previous phase, if any.
    pub fn previous(&self) -> Option<Self> {
        if self.ordinal() == 0 {
            return None;
        }
        Self::from_ordinal(self.ordinal() - 1)
    }

    /// Whether this is the final phase (full self-description).
    pub fn is_terminal(&self) -> bool {
        *self == Self::Phase5SubstrateSelfDescription
    }
}

impl std::fmt::Display for BootstrapPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Phase0ExternalSubstrate => write!(f, "Phase0:ExternalSubstrate"),
            Self::Phase1ConfigSelfTuning => write!(f, "Phase1:ConfigSelfTuning"),
            Self::Phase2OperatorSelfGeneration => write!(f, "Phase2:OperatorSelfGeneration"),
            Self::Phase3ModuleSelfRegeneration => write!(f, "Phase3:ModuleSelfRegeneration"),
            Self::Phase4LanguageSelfProduction => write!(f, "Phase4:LanguageSelfProduction"),
            Self::Phase5SubstrateSelfDescription => {
                write!(f, "Phase5:SubstrateSelfDescription")
            }
        }
    }
}

// ── Phase Status ────────────────────────────────────────────────────

/// Status of a bootstrap phase.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseStatus {
    /// Phase has not been started.
    NotStarted,
    /// Phase is currently in progress.
    InProgress,
    /// Phase completed successfully.
    Complete,
    /// Phase failed.
    Failed(String),
}

impl std::fmt::Display for PhaseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotStarted => write!(f, "not-started"),
            Self::InProgress => write!(f, "in-progress"),
            Self::Complete => write!(f, "complete"),
            Self::Failed(reason) => write!(f, "failed: {}", reason),
        }
    }
}

// ── Readiness Score ─────────────────────────────────────────────────

/// A readiness score in the range [0.0, 1.0].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReadinessScore {
    value: f64,
}

impl ReadinessScore {
    /// Create a new readiness score, clamped to [0.0, 1.0].
    pub fn new(value: f64) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
        }
    }

    /// The score value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Whether this score meets the given threshold.
    pub fn meets_threshold(&self, threshold: f64) -> bool {
        self.value >= threshold
    }
}

impl std::fmt::Display for ReadinessScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}%", self.value * 100.0)
    }
}

// ── Configuration ───────────────────────────────────────────────────

/// Configuration for the bootstrap protocol.
#[derive(Clone, Debug)]
pub struct BootstrapConfig {
    /// Minimum readiness score to advance (default: 0.8).
    pub readiness_threshold: f64,
    /// Whether to enforce governance checks on transitions.
    pub enforce_governance: bool,
    /// Whether to require provenance for every transition.
    pub require_provenance: bool,
    /// Maximum tracked bootstrap records.
    pub max_tracked_records: usize,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            readiness_threshold: 0.8,
            enforce_governance: true,
            require_provenance: true,
            max_tracked_records: 256,
        }
    }
}

// ── Summary ─────────────────────────────────────────────────────────

/// Summary of bootstrap activity.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BootstrapSummary {
    pub total_transitions: usize,
    pub successful_advances: usize,
    pub rollbacks: usize,
    pub current_phase_ordinal: u8,
    pub highest_phase_reached: u8,
}

impl std::fmt::Display for BootstrapSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Bootstrap(transitions={}, advances={}, rollbacks={}, current={}, highest={})",
            self.total_transitions,
            self.successful_advances,
            self.rollbacks,
            self.current_phase_ordinal,
            self.highest_phase_reached,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_id_display() {
        let id = BootstrapId::new();
        assert!(id.to_string().starts_with("bootstrap:"));
    }

    #[test]
    fn bootstrap_id_unique() {
        let a = BootstrapId::new();
        let b = BootstrapId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn phase_ordinals() {
        assert_eq!(BootstrapPhase::Phase0ExternalSubstrate.ordinal(), 0);
        assert_eq!(BootstrapPhase::Phase1ConfigSelfTuning.ordinal(), 1);
        assert_eq!(BootstrapPhase::Phase2OperatorSelfGeneration.ordinal(), 2);
        assert_eq!(BootstrapPhase::Phase3ModuleSelfRegeneration.ordinal(), 3);
        assert_eq!(BootstrapPhase::Phase4LanguageSelfProduction.ordinal(), 4);
        assert_eq!(BootstrapPhase::Phase5SubstrateSelfDescription.ordinal(), 5);
    }

    #[test]
    fn phase_from_ordinal() {
        for i in 0..=5u8 {
            let phase = BootstrapPhase::from_ordinal(i).unwrap();
            assert_eq!(phase.ordinal(), i);
        }
        assert!(BootstrapPhase::from_ordinal(6).is_none());
    }

    #[test]
    fn phase_next_previous() {
        let p0 = BootstrapPhase::Phase0ExternalSubstrate;
        assert!(p0.previous().is_none());
        assert_eq!(p0.next().unwrap(), BootstrapPhase::Phase1ConfigSelfTuning);

        let p5 = BootstrapPhase::Phase5SubstrateSelfDescription;
        assert!(p5.next().is_none());
        assert_eq!(
            p5.previous().unwrap(),
            BootstrapPhase::Phase4LanguageSelfProduction
        );
    }

    #[test]
    fn phase_is_terminal() {
        assert!(!BootstrapPhase::Phase0ExternalSubstrate.is_terminal());
        assert!(BootstrapPhase::Phase5SubstrateSelfDescription.is_terminal());
    }

    #[test]
    fn phase_display() {
        assert_eq!(
            BootstrapPhase::Phase0ExternalSubstrate.to_string(),
            "Phase0:ExternalSubstrate"
        );
        assert_eq!(
            BootstrapPhase::Phase5SubstrateSelfDescription.to_string(),
            "Phase5:SubstrateSelfDescription"
        );
    }

    #[test]
    fn readiness_score_clamp() {
        let s = ReadinessScore::new(1.5);
        assert_eq!(s.value(), 1.0);
        let s = ReadinessScore::new(-0.5);
        assert_eq!(s.value(), 0.0);
        let s = ReadinessScore::new(0.75);
        assert_eq!(s.value(), 0.75);
    }

    #[test]
    fn readiness_score_meets_threshold() {
        let s = ReadinessScore::new(0.85);
        assert!(s.meets_threshold(0.8));
        assert!(!s.meets_threshold(0.9));
    }

    #[test]
    fn config_defaults() {
        let cfg = BootstrapConfig::default();
        assert_eq!(cfg.readiness_threshold, 0.8);
        assert!(cfg.enforce_governance);
        assert!(cfg.require_provenance);
        assert_eq!(cfg.max_tracked_records, 256);
    }

    #[test]
    fn summary_display() {
        let s = BootstrapSummary {
            total_transitions: 5,
            successful_advances: 4,
            rollbacks: 1,
            current_phase_ordinal: 3,
            highest_phase_reached: 4,
        };
        let display = s.to_string();
        assert!(display.contains("transitions=5"));
        assert!(display.contains("highest=4"));
    }

    #[test]
    fn phase_status_display() {
        assert_eq!(PhaseStatus::NotStarted.to_string(), "not-started");
        assert_eq!(PhaseStatus::InProgress.to_string(), "in-progress");
        assert_eq!(PhaseStatus::Complete.to_string(), "complete");
        assert!(PhaseStatus::Failed("oops".into())
            .to_string()
            .contains("oops"));
    }
}
