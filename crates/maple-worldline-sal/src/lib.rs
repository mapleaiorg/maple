//! # maple-worldline-sal
//!
//! Substrate Abstraction Layer (SAL) for the WorldLine framework.
//! Decouples WorldLine from any specific computational substrate,
//! enabling CPU, GPU, FPGA, or future substrates without changing
//! kernel logic.
//!
//! ## Safety Invariants
//!
//! - **I.SAL-1** (Substrate Opacity): WLIR instructions execute identically
//!   regardless of substrate.
//! - **I.SAL-2** (Commitment Gate Preservation): Gates never optimized
//!   away during routing.
//! - **I.SAL-3** (Provenance Completeness): All operations recorded,
//!   no "invisible" operations.
//! - **I.SAL-4** (Resource Limits): Execution fails gracefully on
//!   resource exhaustion.
//! - **I.SAL-5** (Migration Safety): Worldline state never corrupted
//!   during migration.

#![deny(unsafe_code)]

pub mod cpu;
pub mod engine;
pub mod error;
pub mod gpu;
pub mod hybrid;
pub mod migration;
pub mod traits;
pub mod types;

// ── Re-exports ──────────────────────────────────────────────────────

pub use cpu::CpuSubstrate;
pub use engine::{ExecutionRecord, SalEngine};
pub use error::{SalError, SalResult};
pub use gpu::GpuSubstrate;
pub use hybrid::{HybridSubstrate, RoutingFeedback};
pub use migration::{
    MigrationPlan, MigrationRecord, MigrationStatus, MigrationStrategy, SimulatedMigrator,
    StateChecksum, SubstrateMigrator,
};
pub use traits::SubstrateAbstractionLayer;
pub use types::{
    ExecutionId, ExecutionResult, OperatorInput, OperatorOutput, ProvenanceId, ResourceLimits,
    SalConfig, SalSummary, SubstrateCapabilities, SubstrateId, SubstrateKind,
    SubstrateProvenanceRecord,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_operator(name: &str) -> OperatorInput {
        OperatorInput {
            operator_name: name.into(),
            arguments: vec!["100".into(), "USD".into()],
            context: HashMap::new(),
        }
    }

    // ── I.SAL-1: Substrate Opacity ──

    #[test]
    fn invariant_sal_1_substrate_opacity() {
        // Same WLIR execution on CPU and GPU should produce structurally
        // identical results (both contain module name and entry)
        let cpu = CpuSubstrate::new();
        let gpu = GpuSubstrate::new();

        let cpu_result = cpu
            .execute_wlir("mod", "entry", vec!["arg".into()])
            .unwrap();
        let gpu_result = gpu
            .execute_wlir("mod", "entry", vec!["arg".into()])
            .unwrap();

        // Both substrates produce output containing the module/entry
        assert!(cpu_result.output_values[0].contains("mod"));
        assert!(cpu_result.output_values[0].contains("entry"));
        assert!(gpu_result.output_values[0].contains("mod"));
        assert!(gpu_result.output_values[0].contains("entry"));

        // Both executed instructions
        assert!(cpu_result.instructions_executed > 0);
        assert!(gpu_result.instructions_executed > 0);
    }

    // ── I.SAL-3: Provenance Completeness ──

    #[test]
    fn invariant_sal_3_provenance_completeness() {
        // Every execution must produce a provenance record
        let mut engine = SalEngine::new(Box::new(CpuSubstrate::new()));

        let _ = engine.execute_operator(&sample_operator("op1"));
        let _ = engine.execute_wlir("mod", "main", vec![]);
        let _ = engine.execute_operator(&sample_operator("op2"));

        let summary = engine.summary();
        // All 3 operations should have provenance
        assert_eq!(summary.total_provenance_records, 3);
        assert_eq!(summary.total_executions, 3);
    }

    // ── I.SAL-4: Resource Limits ──

    #[test]
    fn invariant_sal_4_resource_limits_config() {
        let limits = ResourceLimits::default();
        assert!(limits.max_memory_bytes > 0);
        assert!(limits.max_execution_time_ms > 0);
        assert!(limits.max_instructions > 0);
    }

    // ── I.SAL-5: Migration Safety ──

    #[test]
    fn invariant_sal_5_migration_state_integrity() {
        let migrator = SimulatedMigrator::new();
        let plan = MigrationPlan {
            source: SubstrateId::new("cpu-0"),
            target: SubstrateId::new("gpu-0"),
            strategy: MigrationStrategy::Live,
            worldline_id: "wl-critical".into(),
            estimated_downtime_ms: 50,
        };

        let state = "critical-worldline-state-commitments-provenance";
        let record = migrator.migrate(&plan, state).unwrap();

        // State must not be corrupted
        assert_eq!(record.status, MigrationStatus::Complete);
        assert!(record
            .source_checksum
            .matches(record.target_checksum.as_ref().unwrap()));
    }

    // ── E2E: multi-substrate engine ──

    #[test]
    fn e2e_cpu_engine() {
        let mut engine = SalEngine::new(Box::new(CpuSubstrate::new()));
        for i in 0..5 {
            let _ = engine
                .execute_operator(&sample_operator(&format!("op-{}", i)))
                .unwrap();
        }
        assert_eq!(engine.record_count(), 5);
        let summary = engine.summary();
        assert_eq!(summary.successful_executions, 5);
    }

    #[test]
    fn e2e_gpu_engine() {
        let mut engine = SalEngine::new(Box::new(GpuSubstrate::new()));
        let result = engine
            .execute_wlir("test-mod", "compute", vec!["42".into()])
            .unwrap();
        assert!(result.output_values[0].contains("gpu:"));
        assert_eq!(engine.record_count(), 1);
    }

    #[test]
    fn e2e_hybrid_engine() {
        let mut engine = SalEngine::new(Box::new(HybridSubstrate::new()));

        // Simple operation → CPU
        let output1 = engine.execute_operator(&sample_operator("simple")).unwrap();
        assert!(output1.result.contains("cpu:"));

        // WLIR → GPU (hybrid prefers GPU when available)
        let result = engine.execute_wlir("mod", "main", vec![]).unwrap();
        assert!(result.output_values[0].contains("gpu:"));

        assert_eq!(engine.record_count(), 2);
    }

    // ── Migration strategies E2E ──

    #[test]
    fn e2e_migration_all_strategies() {
        let migrator = SimulatedMigrator::new();
        let strategies = vec![
            MigrationStrategy::Live,
            MigrationStrategy::Snapshot,
            MigrationStrategy::Parallel,
        ];

        for strategy in strategies {
            let plan = MigrationPlan {
                source: SubstrateId::new("cpu-0"),
                target: SubstrateId::new("gpu-0"),
                strategy: strategy.clone(),
                worldline_id: "wl-test".into(),
                estimated_downtime_ms: 100,
            };

            let record = migrator.migrate(&plan, "state-data").unwrap();
            assert_eq!(record.status, MigrationStatus::Complete);
            assert_eq!(record.plan.strategy, strategy);
        }
    }

    #[test]
    fn e2e_migration_rollback() {
        let migrator = SimulatedMigrator::new();
        let plan = MigrationPlan {
            source: SubstrateId::new("cpu-0"),
            target: SubstrateId::new("gpu-0"),
            strategy: MigrationStrategy::Snapshot,
            worldline_id: "wl-test".into(),
            estimated_downtime_ms: 100,
        };

        let record = migrator.migrate(&plan, "state").unwrap();
        let rolled_back = migrator.rollback(&record).unwrap();
        assert_eq!(rolled_back.status, MigrationStatus::RolledBack);
    }

    // ── Public types ──

    #[test]
    fn public_types_accessible() {
        let _id = SubstrateId::new("test");
        let _kind = SubstrateKind::Cpu;
        let _caps = SubstrateCapabilities::default();
        let _limits = ResourceLimits::default();
        let _config = SalConfig::default();
        let _summary = SalSummary::default();
        let _prov = ProvenanceId::new();
        let _exec = ExecutionId::new();
        let _strategy = MigrationStrategy::Live;
        let _status = MigrationStatus::Pending;
    }

    #[test]
    fn substrate_names() {
        assert_eq!(CpuSubstrate::new().name(), "cpu-substrate");
        assert_eq!(GpuSubstrate::new().name(), "gpu-substrate");
        assert_eq!(HybridSubstrate::new().name(), "hybrid-substrate");
    }
}
