//! Bootstrap manager — orchestrates the full bootstrap protocol.
//!
//! Coordinates readiness checking, governance approval, fingerprint
//! collection, provenance tracking, and phase transitions.

use crate::error::{BootstrapError, BootstrapResult};
use crate::fingerprint::{
    FingerprintCollector, SimulatedFingerprintCollector, SubstrateFingerprint,
};
use crate::phases::{PhaseManager, PhaseTransition};
use crate::provenance::{ProvenanceChain, ProvenanceTracker, SimulatedProvenanceTracker};
use crate::readiness::{
    ReadinessChecker, ReadinessCriteria, ReadinessReport, SimulatedReadinessChecker,
};
use crate::types::{BootstrapConfig, BootstrapPhase};

// ── Bootstrap Manager ───────────────────────────────────────────────

/// Orchestrates the full bootstrap protocol.
///
/// Pipeline for advancing:
/// 1. Check readiness for target phase
/// 2. Collect current substrate fingerprint
/// 3. Record provenance (linking to previous phase)
/// 4. Execute phase transition
pub struct BootstrapManager {
    config: BootstrapConfig,
    phase_manager: PhaseManager,
    readiness_checker: Box<dyn ReadinessChecker>,
    fingerprint_collector: Box<dyn FingerprintCollector>,
    provenance_tracker: Box<dyn ProvenanceTracker>,
    provenance_chain: ProvenanceChain,
    origin_fingerprint: Option<SubstrateFingerprint>,
}

impl BootstrapManager {
    /// Create with default configuration and simulated components.
    pub fn new() -> Self {
        Self {
            config: BootstrapConfig::default(),
            phase_manager: PhaseManager::new(),
            readiness_checker: Box::new(SimulatedReadinessChecker::passing()),
            fingerprint_collector: Box::new(SimulatedFingerprintCollector::new()),
            provenance_tracker: Box::new(SimulatedProvenanceTracker::new()),
            provenance_chain: ProvenanceChain::new(),
            origin_fingerprint: None,
        }
    }

    /// Create with specific configuration.
    pub fn with_config(config: BootstrapConfig) -> Self {
        Self {
            config,
            phase_manager: PhaseManager::new(),
            readiness_checker: Box::new(SimulatedReadinessChecker::passing()),
            fingerprint_collector: Box::new(SimulatedFingerprintCollector::new()),
            provenance_tracker: Box::new(SimulatedProvenanceTracker::new()),
            provenance_chain: ProvenanceChain::new(),
            origin_fingerprint: None,
        }
    }

    /// Set readiness checker.
    pub fn with_readiness_checker(mut self, checker: Box<dyn ReadinessChecker>) -> Self {
        self.readiness_checker = checker;
        self
    }

    /// Set fingerprint collector.
    pub fn with_fingerprint_collector(mut self, collector: Box<dyn FingerprintCollector>) -> Self {
        self.fingerprint_collector = collector;
        self
    }

    /// Set provenance tracker.
    pub fn with_provenance_tracker(mut self, tracker: Box<dyn ProvenanceTracker>) -> Self {
        self.provenance_tracker = tracker;
        self
    }

    /// Current bootstrap phase.
    pub fn current_phase(&self) -> &BootstrapPhase {
        self.phase_manager.current_phase()
    }

    /// Transition history.
    pub fn transition_history(&self) -> &[PhaseTransition] {
        self.phase_manager.transitions()
    }

    /// The provenance chain.
    pub fn provenance_chain(&self) -> &ProvenanceChain {
        &self.provenance_chain
    }

    /// The origin fingerprint (Phase 0).
    pub fn origin_fingerprint(&self) -> Option<&SubstrateFingerprint> {
        self.origin_fingerprint.as_ref()
    }

    /// Whether the system has reached full self-hosting.
    pub fn is_self_hosting(&self) -> bool {
        self.phase_manager.is_self_hosting()
    }

    /// Capture the Phase 0 origin fingerprint.
    ///
    /// Must be called before any advancement. Records the external
    /// substrate fingerprint and creates the root provenance entry.
    pub fn capture_origin(&mut self) -> BootstrapResult<SubstrateFingerprint> {
        let fingerprint = self.fingerprint_collector.collect()?;
        self.origin_fingerprint = Some(fingerprint.clone());

        // Create root provenance (Phase 0, no parent)
        let prov = self.provenance_tracker.record(
            &BootstrapPhase::Phase0ExternalSubstrate,
            None,
            &fingerprint,
            &["external-toolchain".into()],
        )?;
        self.provenance_chain.push(prov);

        Ok(fingerprint)
    }

    /// Advance to the next phase.
    ///
    /// Pipeline:
    /// 1. Determine target phase (current + 1)
    /// 2. Check readiness
    /// 3. Collect fingerprint
    /// 4. Record provenance
    /// 5. Execute transition
    pub fn advance(&mut self) -> BootstrapResult<(PhaseTransition, ReadinessReport)> {
        let current = self.phase_manager.current_phase().clone();
        let target = current.next().ok_or_else(|| {
            BootstrapError::PhaseTransitionFailed("already at terminal phase".into())
        })?;

        // Ensure origin is captured
        if self.origin_fingerprint.is_none() {
            return Err(BootstrapError::PhaseTransitionFailed(
                "origin fingerprint not captured; call capture_origin() first".into(),
            ));
        }

        // Step 1: Readiness check
        let criteria = if self.config.enforce_governance {
            ReadinessCriteria::for_phase(&target)
        } else {
            ReadinessCriteria {
                governance_approval_required: false,
                ..ReadinessCriteria::for_phase(&target)
            }
        };

        let report = self.readiness_checker.check(&current, &target, &criteria)?;
        if !report.all_passed {
            return Err(BootstrapError::ReadinessCheckFailed(format!(
                "readiness check failed for {}: {}/{} criteria passed",
                target,
                report.passed_count(),
                report.criteria_results.len(),
            )));
        }

        // Step 2: Collect fingerprint
        let fingerprint = self.fingerprint_collector.collect()?;

        // Step 3: Record provenance
        let parent_id = self.provenance_chain.latest().map(|p| p.id.clone());
        if self.config.require_provenance && parent_id.is_none() {
            return Err(BootstrapError::ProvenanceGap(
                "no parent provenance for transition".into(),
            ));
        }

        let prov = self.provenance_tracker.record(
            &target,
            parent_id.as_deref(),
            &fingerprint,
            &[format!("phase-{}-artifacts", target.ordinal())],
        )?;
        self.provenance_chain.push(prov);

        // Step 4: Execute transition
        let transition = self.phase_manager.transition(target, "bootstrap-manager")?;

        Ok((transition, report))
    }

    /// Roll back to the previous phase.
    pub fn rollback(&mut self) -> BootstrapResult<PhaseTransition> {
        let current = self.phase_manager.current_phase().clone();
        let target = current.previous().ok_or_else(|| {
            BootstrapError::PhaseTransitionFailed("cannot rollback from Phase 0".into())
        })?;

        let transition = self
            .phase_manager
            .transition(target, "bootstrap-manager-rollback")?;
        Ok(transition)
    }
}

impl Default for BootstrapManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_manager_at_phase0() {
        let mgr = BootstrapManager::new();
        assert_eq!(
            *mgr.current_phase(),
            BootstrapPhase::Phase0ExternalSubstrate
        );
        assert!(!mgr.is_self_hosting());
    }

    #[test]
    fn capture_origin() {
        let mut mgr = BootstrapManager::new();
        let fp = mgr.capture_origin().unwrap();
        assert_eq!(fp.rustc_version, "1.75.0");
        assert!(mgr.origin_fingerprint().is_some());
        assert_eq!(mgr.provenance_chain().len(), 1);
    }

    #[test]
    fn advance_requires_origin() {
        let mut mgr = BootstrapManager::new();
        let result = mgr.advance();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("origin fingerprint not captured"));
    }

    #[test]
    fn advance_one_phase() {
        let mut mgr = BootstrapManager::new();
        mgr.capture_origin().unwrap();
        let (transition, report) = mgr.advance().unwrap();
        assert!(transition.is_advance);
        assert!(report.all_passed);
        assert_eq!(*mgr.current_phase(), BootstrapPhase::Phase1ConfigSelfTuning);
        assert_eq!(mgr.provenance_chain().len(), 2);
    }

    #[test]
    fn advance_to_phase5() {
        let config = BootstrapConfig {
            enforce_governance: false,
            ..BootstrapConfig::default()
        };
        let mut mgr = BootstrapManager::with_config(config);
        mgr.capture_origin().unwrap();
        for _ in 0..5 {
            mgr.advance().unwrap();
        }
        assert!(mgr.is_self_hosting());
        assert_eq!(mgr.provenance_chain().len(), 6); // origin + 5 transitions
        assert_eq!(mgr.transition_history().len(), 5);
    }

    #[test]
    fn advance_past_terminal_fails() {
        let config = BootstrapConfig {
            enforce_governance: false,
            ..BootstrapConfig::default()
        };
        let mut mgr = BootstrapManager::with_config(config);
        mgr.capture_origin().unwrap();
        for _ in 0..5 {
            mgr.advance().unwrap();
        }
        let result = mgr.advance();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("terminal phase"));
    }

    #[test]
    fn readiness_failure_blocks_advance() {
        let mut mgr = BootstrapManager::new()
            .with_readiness_checker(Box::new(SimulatedReadinessChecker::failing_stability()));
        mgr.capture_origin().unwrap();
        let result = mgr.advance();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("readiness check failed"));
    }

    #[test]
    fn rollback_from_phase1() {
        let mut mgr = BootstrapManager::new();
        mgr.capture_origin().unwrap();
        mgr.advance().unwrap();
        let t = mgr.rollback().unwrap();
        assert!(!t.is_advance);
        assert_eq!(
            *mgr.current_phase(),
            BootstrapPhase::Phase0ExternalSubstrate
        );
    }

    #[test]
    fn rollback_from_phase0_fails() {
        let mut mgr = BootstrapManager::new();
        let result = mgr.rollback();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot rollback from Phase 0"));
    }

    #[test]
    fn provenance_chain_valid_after_full_advance() {
        let config = BootstrapConfig {
            enforce_governance: false,
            ..BootstrapConfig::default()
        };
        let mut mgr = BootstrapManager::with_config(config);
        mgr.capture_origin().unwrap();
        for _ in 0..5 {
            mgr.advance().unwrap();
        }
        assert!(!mgr.provenance_chain().has_gaps());
        mgr.provenance_chain().verify().unwrap();
    }

    #[test]
    fn custom_readiness_checker() {
        let checker = SimulatedReadinessChecker {
            stability: 0.5,
            success_rate: 0.99,
            rollback_rate: 0.01,
            observation_hours: 100,
            governance_approved: true,
        };
        let mut mgr = BootstrapManager::new().with_readiness_checker(Box::new(checker));
        mgr.capture_origin().unwrap();
        let result = mgr.advance();
        // Stability 0.5 < threshold 0.7 for phase 1
        assert!(result.is_err());
    }
}
