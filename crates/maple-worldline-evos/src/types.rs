//! Core types for the EVOS integration layer.
//!
//! Defines identifiers, subsystem enumeration, cycle phases,
//! configuration, and summary statistics.

use serde::{Deserialize, Serialize};

// ── Identifiers ─────────────────────────────────────────────────────

/// Unique identifier for an EVOS substrate instance.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvosId(pub String);

impl EvosId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for EvosId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EvosId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "evos:{}", self.0)
    }
}

/// Unique identifier for a cycle execution.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CycleId(pub String);

impl CycleId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for CycleId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CycleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cycle:{}", self.0)
    }
}

// ── Subsystem Identification ────────────────────────────────────────

/// Identifies one of the 14 WorldLine subsystems.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubsystemId {
    Observation,
    Meaning,
    Intent,
    Commitment,
    Consequence,
    SelfModGate,
    Codegen,
    Deployment,
    Ir,
    Compiler,
    LangGen,
    Sal,
    Hardware,
    Bootstrap,
}

impl SubsystemId {
    /// All 14 subsystem variants in canonical order.
    pub fn all() -> &'static [SubsystemId] {
        &[
            SubsystemId::Observation,
            SubsystemId::Meaning,
            SubsystemId::Intent,
            SubsystemId::Commitment,
            SubsystemId::Consequence,
            SubsystemId::SelfModGate,
            SubsystemId::Codegen,
            SubsystemId::Deployment,
            SubsystemId::Ir,
            SubsystemId::Compiler,
            SubsystemId::LangGen,
            SubsystemId::Sal,
            SubsystemId::Hardware,
            SubsystemId::Bootstrap,
        ]
    }

    /// Crate name for this subsystem.
    pub fn crate_name(&self) -> &'static str {
        match self {
            Self::Observation => "maple-worldline-observation",
            Self::Meaning => "maple-worldline-meaning",
            Self::Intent => "maple-worldline-intent",
            Self::Commitment => "maple-worldline-commitment",
            Self::Consequence => "maple-worldline-consequence",
            Self::SelfModGate => "maple-worldline-self-mod-gate",
            Self::Codegen => "maple-worldline-codegen",
            Self::Deployment => "maple-worldline-deployment",
            Self::Ir => "maple-worldline-ir",
            Self::Compiler => "maple-worldline-compiler",
            Self::LangGen => "maple-worldline-langgen",
            Self::Sal => "maple-worldline-sal",
            Self::Hardware => "maple-worldline-hardware",
            Self::Bootstrap => "maple-worldline-bootstrap",
        }
    }
}

impl std::fmt::Display for SubsystemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Observation => write!(f, "observation"),
            Self::Meaning => write!(f, "meaning"),
            Self::Intent => write!(f, "intent"),
            Self::Commitment => write!(f, "commitment"),
            Self::Consequence => write!(f, "consequence"),
            Self::SelfModGate => write!(f, "self-mod-gate"),
            Self::Codegen => write!(f, "codegen"),
            Self::Deployment => write!(f, "deployment"),
            Self::Ir => write!(f, "ir"),
            Self::Compiler => write!(f, "compiler"),
            Self::LangGen => write!(f, "langgen"),
            Self::Sal => write!(f, "sal"),
            Self::Hardware => write!(f, "hardware"),
            Self::Bootstrap => write!(f, "bootstrap"),
        }
    }
}

// ── Subsystem Status ────────────────────────────────────────────────

/// Health status of a subsystem.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubsystemStatus {
    /// Subsystem is operating normally.
    Healthy,
    /// Subsystem is operational but experiencing issues.
    Degraded(String),
    /// Subsystem has failed.
    Failed(String),
    /// Subsystem status is unknown.
    Unknown,
}

impl SubsystemStatus {
    /// Whether this status is healthy.
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Healthy)
    }

    /// Whether this status allows cycle progression.
    pub fn allows_progression(&self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded(_))
    }

    /// Severity ordinal (0=Healthy, 1=Unknown, 2=Degraded, 3=Failed).
    pub fn severity(&self) -> u8 {
        match self {
            Self::Healthy => 0,
            Self::Unknown => 1,
            Self::Degraded(_) => 2,
            Self::Failed(_) => 3,
        }
    }
}

impl std::fmt::Display for SubsystemStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded(msg) => write!(f, "degraded: {}", msg),
            Self::Failed(msg) => write!(f, "failed: {}", msg),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

// ── Cycle Phase ─────────────────────────────────────────────────────

/// Phases of one EVOS cycle iteration.
///
/// Each cycle passes through these phases in order:
/// 1. Observing — collecting system observations
/// 2. Analyzing — deriving meaning from anomalies
/// 3. Forming — forming self-regeneration intents
/// 4. Committing — committing to changes via governance
/// 5. Executing — executing consequences of commitments
/// 6. Generating — generating code from approved changes
/// 7. Deploying — deploying generated artifacts
/// 8. Complete — cycle finished
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CyclePhase {
    Observing,
    Analyzing,
    Forming,
    Committing,
    Executing,
    Generating,
    Deploying,
    Complete,
}

impl CyclePhase {
    /// Ordinal value (0-7).
    pub fn ordinal(&self) -> u8 {
        match self {
            Self::Observing => 0,
            Self::Analyzing => 1,
            Self::Forming => 2,
            Self::Committing => 3,
            Self::Executing => 4,
            Self::Generating => 5,
            Self::Deploying => 6,
            Self::Complete => 7,
        }
    }

    /// Create from ordinal.
    pub fn from_ordinal(ordinal: u8) -> Option<Self> {
        match ordinal {
            0 => Some(Self::Observing),
            1 => Some(Self::Analyzing),
            2 => Some(Self::Forming),
            3 => Some(Self::Committing),
            4 => Some(Self::Executing),
            5 => Some(Self::Generating),
            6 => Some(Self::Deploying),
            7 => Some(Self::Complete),
            _ => None,
        }
    }

    /// Next phase, if any.
    pub fn next(&self) -> Option<Self> {
        Self::from_ordinal(self.ordinal() + 1)
    }

    /// Whether this is the terminal phase.
    pub fn is_complete(&self) -> bool {
        *self == Self::Complete
    }

    /// Primary subsystem responsible for this phase.
    pub fn primary_subsystem(&self) -> SubsystemId {
        match self {
            Self::Observing => SubsystemId::Observation,
            Self::Analyzing => SubsystemId::Meaning,
            Self::Forming => SubsystemId::Intent,
            Self::Committing => SubsystemId::Commitment,
            Self::Executing => SubsystemId::Consequence,
            Self::Generating => SubsystemId::Codegen,
            Self::Deploying => SubsystemId::Deployment,
            Self::Complete => SubsystemId::Observation, // loops back
        }
    }
}

impl std::fmt::Display for CyclePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Observing => write!(f, "observing"),
            Self::Analyzing => write!(f, "analyzing"),
            Self::Forming => write!(f, "forming"),
            Self::Committing => write!(f, "committing"),
            Self::Executing => write!(f, "executing"),
            Self::Generating => write!(f, "generating"),
            Self::Deploying => write!(f, "deploying"),
            Self::Complete => write!(f, "complete"),
        }
    }
}

// ── Configuration ───────────────────────────────────────────────────

/// Configuration for the EVOS integration layer.
#[derive(Clone, Debug)]
pub struct EvosConfig {
    /// Whether to require all subsystems healthy before starting a cycle.
    pub require_healthy_start: bool,
    /// Whether to abort cycle on subsystem failure.
    pub abort_on_failure: bool,
    /// Maximum tracked cycle records.
    pub max_tracked_records: usize,
    /// Maximum allowed cycle duration in milliseconds.
    pub max_cycle_duration_ms: u64,
}

impl Default for EvosConfig {
    fn default() -> Self {
        Self {
            require_healthy_start: true,
            abort_on_failure: true,
            max_tracked_records: 256,
            max_cycle_duration_ms: 60_000,
        }
    }
}

// ── Summary ─────────────────────────────────────────────────────────

/// Summary of EVOS activity.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EvosSummary {
    pub total_cycles: usize,
    pub successful_cycles: usize,
    pub failed_cycles: usize,
    pub total_steps_executed: usize,
    pub current_bootstrap_phase: u8,
}

impl std::fmt::Display for EvosSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EVOS(cycles={}, success={}, failed={}, steps={}, bootstrap=phase{})",
            self.total_cycles,
            self.successful_cycles,
            self.failed_cycles,
            self.total_steps_executed,
            self.current_bootstrap_phase,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evos_id_display() {
        let id = EvosId::new();
        assert!(id.to_string().starts_with("evos:"));
    }

    #[test]
    fn cycle_id_display() {
        let id = CycleId::new();
        assert!(id.to_string().starts_with("cycle:"));
    }

    #[test]
    fn subsystem_id_all_14() {
        assert_eq!(SubsystemId::all().len(), 14);
    }

    #[test]
    fn subsystem_crate_names() {
        assert_eq!(
            SubsystemId::Observation.crate_name(),
            "maple-worldline-observation"
        );
        assert_eq!(
            SubsystemId::Bootstrap.crate_name(),
            "maple-worldline-bootstrap"
        );
    }

    #[test]
    fn subsystem_status_severity() {
        assert_eq!(SubsystemStatus::Healthy.severity(), 0);
        assert_eq!(SubsystemStatus::Unknown.severity(), 1);
        assert_eq!(SubsystemStatus::Degraded("slow".into()).severity(), 2);
        assert_eq!(SubsystemStatus::Failed("crash".into()).severity(), 3);
    }

    #[test]
    fn subsystem_status_progression() {
        assert!(SubsystemStatus::Healthy.allows_progression());
        assert!(SubsystemStatus::Degraded("slow".into()).allows_progression());
        assert!(!SubsystemStatus::Failed("crash".into()).allows_progression());
        assert!(!SubsystemStatus::Unknown.allows_progression());
    }

    #[test]
    fn cycle_phase_ordinals() {
        assert_eq!(CyclePhase::Observing.ordinal(), 0);
        assert_eq!(CyclePhase::Complete.ordinal(), 7);
        for i in 0..=7u8 {
            let phase = CyclePhase::from_ordinal(i).unwrap();
            assert_eq!(phase.ordinal(), i);
        }
        assert!(CyclePhase::from_ordinal(8).is_none());
    }

    #[test]
    fn cycle_phase_next() {
        assert_eq!(
            CyclePhase::Observing.next().unwrap(),
            CyclePhase::Analyzing
        );
        assert!(CyclePhase::Complete.next().is_none());
    }

    #[test]
    fn cycle_phase_primary_subsystem() {
        assert_eq!(
            CyclePhase::Observing.primary_subsystem(),
            SubsystemId::Observation
        );
        assert_eq!(
            CyclePhase::Generating.primary_subsystem(),
            SubsystemId::Codegen
        );
        assert_eq!(
            CyclePhase::Deploying.primary_subsystem(),
            SubsystemId::Deployment
        );
    }

    #[test]
    fn config_defaults() {
        let cfg = EvosConfig::default();
        assert!(cfg.require_healthy_start);
        assert!(cfg.abort_on_failure);
        assert_eq!(cfg.max_tracked_records, 256);
    }

    #[test]
    fn summary_display() {
        let s = EvosSummary {
            total_cycles: 10,
            successful_cycles: 8,
            failed_cycles: 2,
            total_steps_executed: 70,
            current_bootstrap_phase: 3,
        };
        let display = s.to_string();
        assert!(display.contains("cycles=10"));
        assert!(display.contains("bootstrap=phase3"));
    }

    #[test]
    fn subsystem_display() {
        assert_eq!(SubsystemId::Observation.to_string(), "observation");
        assert_eq!(SubsystemId::SelfModGate.to_string(), "self-mod-gate");
        assert_eq!(SubsystemId::LangGen.to_string(), "langgen");
    }
}
