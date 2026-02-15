//! Phase management — tracks current phase and validates transitions.
//!
//! Enforces I.BOOT-1 (Monotonic Phase Advancement): transitions can only
//! advance by exactly 1 or roll back by exactly 1. No skipping phases.

use serde::{Deserialize, Serialize};

use crate::error::{BootstrapError, BootstrapResult};
use crate::types::{BootstrapPhase, PhaseStatus};

// ── Phase Transition ────────────────────────────────────────────────

/// Record of a phase transition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhaseTransition {
    /// Phase transitioned from.
    pub from: BootstrapPhase,
    /// Phase transitioned to.
    pub to: BootstrapPhase,
    /// Whether this was an advance (+1) or rollback (-1).
    pub is_advance: bool,
    /// Who approved the transition.
    pub approved_by: String,
    /// When the transition occurred.
    pub transitioned_at: chrono::DateTime<chrono::Utc>,
}

impl std::fmt::Display for PhaseTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let direction = if self.is_advance {
            "advance"
        } else {
            "rollback"
        };
        write!(f, "Transition({}: {} → {})", direction, self.from, self.to)
    }
}

// ── Phase Manager ───────────────────────────────────────────────────

/// Manages the current bootstrap phase and transition history.
///
/// Enforces the invariant that transitions can only move by exactly
/// one phase in either direction (advance or rollback).
pub struct PhaseManager {
    current_phase: BootstrapPhase,
    phase_statuses: Vec<(BootstrapPhase, PhaseStatus)>,
    transitions: Vec<PhaseTransition>,
    highest_reached: u8,
}

impl PhaseManager {
    /// Create starting at Phase 0.
    pub fn new() -> Self {
        let mut statuses = Vec::new();
        for i in 0..=5u8 {
            let phase = BootstrapPhase::from_ordinal(i).unwrap();
            let status = if i == 0 {
                PhaseStatus::InProgress
            } else {
                PhaseStatus::NotStarted
            };
            statuses.push((phase, status));
        }
        Self {
            current_phase: BootstrapPhase::Phase0ExternalSubstrate,
            phase_statuses: statuses,
            transitions: Vec::new(),
            highest_reached: 0,
        }
    }

    /// Current phase.
    pub fn current_phase(&self) -> &BootstrapPhase {
        &self.current_phase
    }

    /// Highest phase ordinal ever reached.
    pub fn highest_reached(&self) -> u8 {
        self.highest_reached
    }

    /// Status of a specific phase.
    pub fn phase_status(&self, phase: &BootstrapPhase) -> &PhaseStatus {
        &self.phase_statuses[phase.ordinal() as usize].1
    }

    /// All transition history.
    pub fn transitions(&self) -> &[PhaseTransition] {
        &self.transitions
    }

    /// Validate that a transition from current to target is legal.
    ///
    /// Legal transitions:
    /// - Advance by exactly 1 (current.ordinal() + 1 == target.ordinal())
    /// - Rollback by exactly 1 (current.ordinal() - 1 == target.ordinal())
    fn validate_transition(&self, target: &BootstrapPhase) -> BootstrapResult<bool> {
        let current_ord = self.current_phase.ordinal();
        let target_ord = target.ordinal();

        // Advance by 1
        if target_ord == current_ord + 1 {
            return Ok(true); // is_advance = true
        }

        // Rollback by 1
        if current_ord > 0 && target_ord == current_ord - 1 {
            return Ok(false); // is_advance = false
        }

        // Illegal transition
        if target_ord == current_ord {
            return Err(BootstrapError::PhaseTransitionFailed(format!(
                "already at {}",
                self.current_phase
            )));
        }

        let direction = if target_ord > current_ord {
            "cannot skip phases"
        } else {
            "cannot rollback multiple phases"
        };
        Err(BootstrapError::PhaseTransitionFailed(format!(
            "{}: {} → {} ({})",
            direction, self.current_phase, target, direction,
        )))
    }

    /// Attempt to transition to a target phase.
    pub fn transition(
        &mut self,
        target: BootstrapPhase,
        approved_by: &str,
    ) -> BootstrapResult<PhaseTransition> {
        let is_advance = self.validate_transition(&target)?;

        // Mark current phase as complete (for advance) or failed (for rollback)
        let current_ord = self.current_phase.ordinal() as usize;
        if is_advance {
            self.phase_statuses[current_ord].1 = PhaseStatus::Complete;
        } else {
            self.phase_statuses[current_ord].1 =
                PhaseStatus::Failed("rolled back".into());
        }

        let transition = PhaseTransition {
            from: self.current_phase.clone(),
            to: target.clone(),
            is_advance,
            approved_by: approved_by.to_string(),
            transitioned_at: chrono::Utc::now(),
        };

        // Update current phase
        let target_ord = target.ordinal() as usize;
        self.phase_statuses[target_ord].1 = PhaseStatus::InProgress;
        self.current_phase = target;

        if is_advance && self.current_phase.ordinal() > self.highest_reached {
            self.highest_reached = self.current_phase.ordinal();
        }

        self.transitions.push(transition.clone());
        Ok(transition)
    }

    /// Whether the system has reached full self-hosting (Phase 5 complete).
    pub fn is_self_hosting(&self) -> bool {
        self.current_phase == BootstrapPhase::Phase5SubstrateSelfDescription
    }
}

impl Default for PhaseManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_at_phase0() {
        let pm = PhaseManager::new();
        assert_eq!(*pm.current_phase(), BootstrapPhase::Phase0ExternalSubstrate);
        assert_eq!(pm.highest_reached(), 0);
        assert!(!pm.is_self_hosting());
    }

    #[test]
    fn advance_by_one() {
        let mut pm = PhaseManager::new();
        let t = pm
            .transition(BootstrapPhase::Phase1ConfigSelfTuning, "test-approver")
            .unwrap();
        assert!(t.is_advance);
        assert_eq!(*pm.current_phase(), BootstrapPhase::Phase1ConfigSelfTuning);
        assert_eq!(pm.highest_reached(), 1);
    }

    #[test]
    fn rollback_by_one() {
        let mut pm = PhaseManager::new();
        pm.transition(BootstrapPhase::Phase1ConfigSelfTuning, "approver")
            .unwrap();
        let t = pm
            .transition(BootstrapPhase::Phase0ExternalSubstrate, "approver")
            .unwrap();
        assert!(!t.is_advance);
        assert_eq!(
            *pm.current_phase(),
            BootstrapPhase::Phase0ExternalSubstrate
        );
        // Highest reached stays at 1
        assert_eq!(pm.highest_reached(), 1);
    }

    #[test]
    fn cannot_skip_phases() {
        let mut pm = PhaseManager::new();
        let result =
            pm.transition(BootstrapPhase::Phase2OperatorSelfGeneration, "approver");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot skip"));
    }

    #[test]
    fn cannot_transition_to_same_phase() {
        let pm = PhaseManager::new();
        // We need a mutable reference
        let mut pm = pm;
        let result =
            pm.transition(BootstrapPhase::Phase0ExternalSubstrate, "approver");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already at"));
    }

    #[test]
    fn cannot_rollback_from_phase0() {
        let pm = PhaseManager::new();
        assert_eq!(pm.current_phase().ordinal(), 0);
        // Phase 0 has no previous — rollback is naturally impossible.
        assert!(pm.current_phase().previous().is_none());
    }

    #[test]
    fn full_advancement_to_phase5() {
        let mut pm = PhaseManager::new();
        for i in 1..=5u8 {
            let target = BootstrapPhase::from_ordinal(i).unwrap();
            pm.transition(target, "approver").unwrap();
        }
        assert!(pm.is_self_hosting());
        assert_eq!(pm.highest_reached(), 5);
        assert_eq!(pm.transitions().len(), 5);
    }

    #[test]
    fn phase_statuses_tracked() {
        let mut pm = PhaseManager::new();
        assert_eq!(
            *pm.phase_status(&BootstrapPhase::Phase0ExternalSubstrate),
            PhaseStatus::InProgress
        );
        assert_eq!(
            *pm.phase_status(&BootstrapPhase::Phase1ConfigSelfTuning),
            PhaseStatus::NotStarted
        );

        pm.transition(BootstrapPhase::Phase1ConfigSelfTuning, "approver")
            .unwrap();

        assert_eq!(
            *pm.phase_status(&BootstrapPhase::Phase0ExternalSubstrate),
            PhaseStatus::Complete
        );
        assert_eq!(
            *pm.phase_status(&BootstrapPhase::Phase1ConfigSelfTuning),
            PhaseStatus::InProgress
        );
    }

    #[test]
    fn transition_display() {
        let t = PhaseTransition {
            from: BootstrapPhase::Phase0ExternalSubstrate,
            to: BootstrapPhase::Phase1ConfigSelfTuning,
            is_advance: true,
            approved_by: "test".into(),
            transitioned_at: chrono::Utc::now(),
        };
        let display = t.to_string();
        assert!(display.contains("advance"));
        assert!(display.contains("Phase0"));
        assert!(display.contains("Phase1"));
    }

    #[test]
    fn advance_then_rollback_then_advance() {
        let mut pm = PhaseManager::new();
        pm.transition(BootstrapPhase::Phase1ConfigSelfTuning, "a")
            .unwrap();
        pm.transition(BootstrapPhase::Phase2OperatorSelfGeneration, "a")
            .unwrap();
        pm.transition(BootstrapPhase::Phase1ConfigSelfTuning, "a")
            .unwrap(); // rollback
        pm.transition(BootstrapPhase::Phase2OperatorSelfGeneration, "a")
            .unwrap(); // re-advance
        assert_eq!(
            *pm.current_phase(),
            BootstrapPhase::Phase2OperatorSelfGeneration
        );
        assert_eq!(pm.transitions().len(), 4);
        assert_eq!(pm.highest_reached(), 2);
    }
}
