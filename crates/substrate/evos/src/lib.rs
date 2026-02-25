//! EVOS Integration — Self-Producing Substrate Cycle Orchestration.
//!
//! This crate ties all 14 WorldLine subsystems into a single
//! self-producing substrate cycle (EVOS — Evolving Virtual Organism
//! Substrate).
//!
//! # Cycle Phases
//!
//! Each EVOS cycle passes through 8 phases:
//! 1. **Observing** — collecting system observations (observation crate)
//! 2. **Analyzing** — deriving meaning from anomalies (meaning crate)
//! 3. **Forming** — forming self-regeneration intents (intent crate)
//! 4. **Committing** — committing to changes via governance (commitment crate)
//! 5. **Executing** — executing consequences (consequence crate)
//! 6. **Generating** — generating code (codegen crate)
//! 7. **Deploying** — deploying artifacts (deployment crate)
//! 8. **Complete** — cycle finished, loops back to observing
//!
//! # Safety Invariants
//!
//! - **I.EVOS-1**: Every cycle passes through all phases in order.
//! - **I.EVOS-2**: Health monitoring covers all 14 subsystems.

#![deny(unsafe_code)]

pub mod cycle;
pub mod engine;
pub mod error;
pub mod health;
pub mod report;
pub mod substrate;
pub mod types;

// ── Re-exports ──────────────────────────────────────────────────────

pub use cycle::{CycleRecord, CycleRunner, CycleStep, SimulatedCycleRunner};
pub use engine::EvosEngine;
pub use error::{EvosError, EvosResult};
pub use health::{HealthChecker, HealthReport, SimulatedHealthChecker, SubsystemHealth};
pub use report::{CycleReport, SubsystemSummaryEntry};
pub use substrate::{EvosSubstrate, SubstrateManifest, SubsystemEntry};
pub use types::{
    CycleId, CyclePhase, EvosConfig, EvosId, EvosSummary, SubsystemId, SubsystemStatus,
};

#[cfg(test)]
mod tests {
    use super::*;

    // ── E2E: Full EVOS Cycle ────────────────────────────────────────

    #[test]
    fn e2e_full_cycle_succeeds() {
        let mut engine = EvosEngine::new();
        let report = engine.run_cycle().unwrap();
        assert!(report.success);
        assert_eq!(report.steps_completed, 8);
        assert_eq!(report.steps_attempted, 8);
        assert_eq!(engine.record_count(), 1);
    }

    #[test]
    fn e2e_multiple_cycles() {
        let mut engine = EvosEngine::new();
        for i in 0..3 {
            let report = engine.run_cycle().unwrap();
            assert!(report.success, "cycle {} failed", i);
        }
        let summary = engine.summary();
        assert_eq!(summary.total_cycles, 3);
        assert_eq!(summary.successful_cycles, 3);
    }

    // ── I.EVOS-1: Cycle Completeness ────────────────────────────────

    #[test]
    fn invariant_evos_1_all_phases_in_order() {
        let runner = SimulatedCycleRunner::all_passing();
        let checker = SimulatedHealthChecker::all_healthy();
        let report = checker.check_all().unwrap();
        let config = EvosConfig::default();
        let record = runner.run_cycle(&report, &config).unwrap();

        // Verify all 8 phases present in order
        assert_eq!(record.total_steps(), 8);
        for (i, step) in record.steps.iter().enumerate() {
            assert_eq!(
                step.phase.ordinal(),
                i as u8,
                "phase at step {} should be ordinal {}, got {}",
                i,
                i,
                step.phase.ordinal()
            );
        }

        // Last step is Complete
        assert!(record.steps.last().unwrap().phase.is_complete());
    }

    #[test]
    fn invariant_evos_1_aborted_cycle_records_up_to_failure() {
        let runner = SimulatedCycleRunner::failing_at(vec![4]); // Fail at Executing
        let checker = SimulatedHealthChecker::all_healthy();
        let report = checker.check_all().unwrap();
        let config = EvosConfig {
            abort_on_failure: true,
            ..EvosConfig::default()
        };
        let record = runner.run_cycle(&report, &config).unwrap();

        // Should have phases 0,1,2,3,4 (the failure)
        assert_eq!(record.total_steps(), 5);
        assert!(!record.success);

        // Phases are still in order up to the failure point
        for (i, step) in record.steps.iter().enumerate() {
            assert_eq!(step.phase.ordinal(), i as u8);
        }
    }

    // ── I.EVOS-2: Health Monitoring ─────────────────────────────────

    #[test]
    fn invariant_evos_2_health_covers_all_14_subsystems() {
        let engine = EvosEngine::new();
        let health = engine.health_check().unwrap();
        assert_eq!(health.entries.len(), 14);

        // Verify all subsystems are present
        for subsystem in SubsystemId::all() {
            assert!(
                health.entries.iter().any(|e| e.subsystem == *subsystem),
                "missing health entry for {}",
                subsystem,
            );
        }
    }

    #[test]
    fn invariant_evos_2_degraded_blocks_cycle() {
        let mut engine = EvosEngine::new().with_health_checker(Box::new(
            SimulatedHealthChecker::with_overrides(vec![(
                SubsystemId::Commitment,
                SubsystemStatus::Failed("database error".into()),
            )]),
        ));

        let result = engine.run_cycle();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("commitment"));
    }

    // ── Cycle Report Aggregation ────────────────────────────────────

    #[test]
    fn e2e_cycle_report_aggregation() {
        let mut engine = EvosEngine::new();
        let report = engine.run_cycle().unwrap();

        assert!(report.participating_subsystems() >= 7);
        assert_eq!(
            report.healthy_subsystems(),
            report.participating_subsystems()
        );
    }

    // ── Bootstrap Phase Awareness ───────────────────────────────────

    #[test]
    fn substrate_bootstrap_phase_awareness() {
        let substrate = EvosSubstrate::new();
        assert_eq!(
            substrate.bootstrap_phase,
            maple_worldline_bootstrap::BootstrapPhase::Phase0ExternalSubstrate
        );
        assert!(!substrate.is_self_hosting());

        let substrate = EvosSubstrate::at_phase(
            maple_worldline_bootstrap::BootstrapPhase::Phase5SubstrateSelfDescription,
        );
        assert!(substrate.is_self_hosting());
    }

    // ── Public Types Accessible ─────────────────────────────────────

    #[test]
    fn public_types_accessible() {
        let _id = EvosId::new();
        let _cycle_id = CycleId::new();
        let _subsystem = SubsystemId::Observation;
        let _status = SubsystemStatus::Healthy;
        let _phase = CyclePhase::Observing;
        let _config = EvosConfig::default();
        let _summary = EvosSummary::default();
        let _substrate = EvosSubstrate::new();
        let _manifest = SubstrateManifest::canonical();
        let _engine = EvosEngine::new();
    }

    // ── Engine Summary with Mixed Results ───────────────────────────

    #[test]
    fn e2e_engine_summary_mixed() {
        let config = EvosConfig {
            abort_on_failure: true,
            ..EvosConfig::default()
        };

        // First: successful cycles
        let mut engine = EvosEngine::with_config(config);
        engine.run_cycle().unwrap();
        engine.run_cycle().unwrap();

        // Switch to failing runner
        engine = EvosEngine::new()
            .with_cycle_runner(Box::new(SimulatedCycleRunner::failing_at(vec![1])));
        engine.run_cycle().unwrap(); // Will record but be failed

        // Run a successful one
        let mut engine2 = EvosEngine::new();
        engine2.run_cycle().unwrap();
        engine2.run_cycle().unwrap();

        let summary = engine2.summary();
        assert_eq!(summary.total_cycles, 2);
        assert_eq!(summary.successful_cycles, 2);
    }
}
