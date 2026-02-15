//! Bootstrap Protocol — From External to Self-Hosted WorldLine.
//!
//! This crate manages the WorldLine system's transition from being built by
//! an external toolchain (Phase 0) to progressively self-describing its own
//! substrate (Phase 5), through 6 well-defined phases:
//!
//! - **Phase 0**: External Substrate — built by rustc/cargo
//! - **Phase 1**: Configuration Self-Tuning — system tunes its own config
//! - **Phase 2**: Operator Self-Generation — system generates operators
//! - **Phase 3**: Module Self-Regeneration — system regenerates modules
//! - **Phase 4**: Language Self-Production — system produces DSLs
//! - **Phase 5**: Substrate Self-Description — full self-reference
//!
//! # Safety Invariants
//!
//! - **I.BOOT-1**: Phase transitions advance or rollback by exactly 1.
//! - **I.BOOT-2**: Provenance chain has no gaps.
//! - **I.BOOT-3**: Phase 0 captures the external substrate fingerprint.

#![deny(unsafe_code)]

pub mod engine;
pub mod error;
pub mod fingerprint;
pub mod manager;
pub mod phases;
pub mod provenance;
pub mod readiness;
pub mod types;

// ── Re-exports ──────────────────────────────────────────────────────

pub use engine::{BootstrapEngine, BootstrapOperation, BootstrapRecord};
pub use error::{BootstrapError, BootstrapResult};
pub use fingerprint::{FingerprintCollector, SimulatedFingerprintCollector, SubstrateFingerprint};
pub use manager::BootstrapManager;
pub use phases::{PhaseManager, PhaseTransition};
pub use provenance::{
    BootstrapProvenance, ProvenanceChain, ProvenanceTracker, SimulatedProvenanceTracker,
};
pub use readiness::{
    CriterionResult, ReadinessChecker, ReadinessCriteria, ReadinessReport,
    SimulatedReadinessChecker,
};
pub use types::{
    BootstrapConfig, BootstrapId, BootstrapPhase, BootstrapSummary, PhaseStatus, ReadinessScore,
};

#[cfg(test)]
mod tests {
    use super::*;

    // ── E2E: Full Bootstrap 0 → 5 ──────────────────────────────────

    #[test]
    fn e2e_full_bootstrap_to_self_hosting() {
        let config = BootstrapConfig {
            enforce_governance: false,
            ..BootstrapConfig::default()
        };
        let mut engine = BootstrapEngine::with_config(config);

        // Capture origin
        engine.capture_origin().unwrap();

        // Advance through all 5 phases
        for i in 1..=5u8 {
            engine.advance().unwrap();
            assert_eq!(engine.current_phase().ordinal(), i);
        }

        assert!(engine.is_self_hosting());
        assert_eq!(engine.record_count(), 6); // 1 capture + 5 advances
    }

    // ── E2E: Rollback and Re-advance ────────────────────────────────

    #[test]
    fn e2e_rollback_and_readvance() {
        let config = BootstrapConfig {
            enforce_governance: false,
            ..BootstrapConfig::default()
        };
        let mut engine = BootstrapEngine::with_config(config);
        engine.capture_origin().unwrap();

        // Advance to Phase 3
        for _ in 0..3 {
            engine.advance().unwrap();
        }
        assert_eq!(engine.current_phase().ordinal(), 3);

        // Rollback to Phase 2
        engine.rollback().unwrap();
        assert_eq!(engine.current_phase().ordinal(), 2);

        // Re-advance to Phase 3
        engine.advance().unwrap();
        assert_eq!(engine.current_phase().ordinal(), 3);

        let summary = engine.summary();
        assert_eq!(summary.successful_advances, 4); // 3 + 1 re-advance
        assert_eq!(summary.rollbacks, 1);
    }

    // ── I.BOOT-1: Monotonic Phase Advancement ──────────────────────

    #[test]
    fn invariant_boot_1_no_phase_skipping() {
        let mut pm = PhaseManager::new();

        // Cannot skip from Phase 0 to Phase 2
        let result = pm.transition(
            BootstrapPhase::Phase2OperatorSelfGeneration,
            "test",
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot skip"));

        // Can advance by 1
        pm.transition(BootstrapPhase::Phase1ConfigSelfTuning, "test")
            .unwrap();

        // Cannot skip from Phase 1 to Phase 3
        let result = pm.transition(
            BootstrapPhase::Phase3ModuleSelfRegeneration,
            "test",
        );
        assert!(result.is_err());
    }

    #[test]
    fn invariant_boot_1_rollback_only_by_one() {
        let mut pm = PhaseManager::new();
        pm.transition(BootstrapPhase::Phase1ConfigSelfTuning, "test")
            .unwrap();
        pm.transition(BootstrapPhase::Phase2OperatorSelfGeneration, "test")
            .unwrap();

        // Cannot rollback from Phase 2 to Phase 0 (skip)
        let result = pm.transition(
            BootstrapPhase::Phase0ExternalSubstrate,
            "test",
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot rollback multiple"));
    }

    // ── I.BOOT-2: Provenance Chain No Gaps ──────────────────────────

    #[test]
    fn invariant_boot_2_provenance_chain_integrity() {
        let config = BootstrapConfig {
            enforce_governance: false,
            ..BootstrapConfig::default()
        };
        let mut mgr = BootstrapManager::with_config(config);
        mgr.capture_origin().unwrap();

        for _ in 0..5 {
            mgr.advance().unwrap();
        }

        let chain = mgr.provenance_chain();
        assert!(!chain.has_gaps());
        chain.verify().unwrap();

        // Chain should have 6 entries (Phase 0 origin + 5 advances)
        assert_eq!(chain.len(), 6);

        // First entry is Phase 0
        assert_eq!(
            chain.entries()[0].phase,
            BootstrapPhase::Phase0ExternalSubstrate
        );

        // Last entry is Phase 5
        assert_eq!(
            chain.latest().unwrap().phase,
            BootstrapPhase::Phase5SubstrateSelfDescription
        );
    }

    // ── I.BOOT-3: Fingerprint Accountability ────────────────────────

    #[test]
    fn invariant_boot_3_fingerprint_captured_at_origin() {
        let mut mgr = BootstrapManager::new();
        // Before capture, origin is None
        assert!(mgr.origin_fingerprint().is_none());

        mgr.capture_origin().unwrap();

        // After capture, origin is Some
        let fp = mgr.origin_fingerprint().unwrap();
        assert_eq!(fp.rustc_version, "1.75.0");
        assert_eq!(fp.target_triple, "x86_64-unknown-linux-gnu");

        // Provenance chain also records the origin fingerprint
        let chain_fp = mgr.provenance_chain().origin_fingerprint().unwrap();
        assert_eq!(chain_fp.rustc_version, fp.rustc_version);
    }

    // ── Fingerprint Drift Detection ─────────────────────────────────

    #[test]
    fn e2e_fingerprint_drift_detection() {
        let fp1 = SubstrateFingerprint {
            rustc_version: "1.75.0".into(),
            target_triple: "x86_64-unknown-linux-gnu".into(),
            os: "linux".into(),
            cpu_arch: "x86_64".into(),
            cargo_lock_hash: "abc123".into(),
            captured_at: chrono::Utc::now(),
            features: vec!["default".into()],
        };

        let fp2 = SubstrateFingerprint {
            rustc_version: "1.76.0".into(),
            target_triple: "x86_64-unknown-linux-gnu".into(),
            os: "linux".into(),
            cpu_arch: "x86_64".into(),
            cargo_lock_hash: "def456".into(),
            captured_at: chrono::Utc::now(),
            features: vec!["default".into(), "gpu".into()],
        };

        let drifts = fp1.drift_from(&fp2);
        assert_eq!(drifts.len(), 3); // rustc, cargo_lock_hash, features
        assert!(!fp1.matches(&fp2));
    }

    // ── Governance Blocks Unready Transition ─────────────────────────

    #[test]
    fn governance_blocks_unready_advance() {
        let mut mgr = BootstrapManager::new()
            .with_readiness_checker(Box::new(SimulatedReadinessChecker::failing_stability()));
        mgr.capture_origin().unwrap();

        let result = mgr.advance();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("readiness check failed"));
    }

    // ── Public Types Accessible ─────────────────────────────────────

    #[test]
    fn public_types_accessible() {
        let _id = BootstrapId::new();
        let _phase = BootstrapPhase::Phase0ExternalSubstrate;
        let _status = PhaseStatus::NotStarted;
        let _score = ReadinessScore::new(0.5);
        let _config = BootstrapConfig::default();
        let _summary = BootstrapSummary::default();
        let _criteria = ReadinessCriteria::default();
        let _engine = BootstrapEngine::new();
    }

    // ── Engine Summary Tracks Correctly ─────────────────────────────

    #[test]
    fn e2e_engine_summary() {
        let config = BootstrapConfig {
            enforce_governance: false,
            ..BootstrapConfig::default()
        };
        let mut engine = BootstrapEngine::with_config(config);
        engine.capture_origin().unwrap();

        engine.advance().unwrap();
        engine.advance().unwrap();
        engine.advance().unwrap();
        engine.rollback().unwrap();
        engine.advance().unwrap();

        let summary = engine.summary();
        assert_eq!(summary.current_phase_ordinal, 3);
        assert_eq!(summary.successful_advances, 4);
        assert_eq!(summary.rollbacks, 1);
        assert!(summary.highest_phase_reached >= 3);
    }
}
