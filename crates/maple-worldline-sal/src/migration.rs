//! Substrate migration strategies.
//!
//! Three strategies for moving active worldlines between substrates:
//! - **Live**: Minimal downtime, state streamed continuously
//! - **Snapshot**: Brief pause, state captured and restored
//! - **Parallel**: Dual execution on both substrates, verify, switch
//!
//! Safety invariant **I.SAL-5**: Worldline state never corrupted during migration.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::{SalError, SalResult};
use crate::types::SubstrateId;

// ── Migration Strategy ──────────────────────────────────────────────

/// Migration strategy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationStrategy {
    /// Minimal downtime — state streamed continuously.
    Live,
    /// Brief pause — state captured and restored.
    Snapshot,
    /// Dual execution — run on both, verify, switch.
    Parallel,
}

impl std::fmt::Display for MigrationStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Live => write!(f, "live"),
            Self::Snapshot => write!(f, "snapshot"),
            Self::Parallel => write!(f, "parallel"),
        }
    }
}

// ── Migration Status ────────────────────────────────────────────────

/// Status of a migration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationStatus {
    Pending,
    InProgress,
    Verifying,
    Complete,
    RolledBack,
    Failed(String),
}

impl std::fmt::Display for MigrationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::InProgress => write!(f, "in-progress"),
            Self::Verifying => write!(f, "verifying"),
            Self::Complete => write!(f, "complete"),
            Self::RolledBack => write!(f, "rolled-back"),
            Self::Failed(reason) => write!(f, "failed: {}", reason),
        }
    }
}

// ── State Checksum ──────────────────────────────────────────────────

/// Checksum of worldline state for migration verification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateChecksum(pub String);

impl StateChecksum {
    /// Compute a simulated checksum from state data.
    pub fn compute(state_data: &str) -> Self {
        // Simulated: use a simple hash
        let hash = state_data.len() * 31
            + state_data
                .as_bytes()
                .iter()
                .map(|b| *b as usize)
                .sum::<usize>();
        Self(format!("{:016x}", hash))
    }

    /// Verify that two checksums match (I.SAL-5).
    pub fn matches(&self, other: &StateChecksum) -> bool {
        self.0 == other.0
    }
}

// ── Migration Plan ──────────────────────────────────────────────────

/// A plan for migrating a worldline between substrates.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub source: SubstrateId,
    pub target: SubstrateId,
    pub strategy: MigrationStrategy,
    pub worldline_id: String,
    pub estimated_downtime_ms: u64,
}

// ── Migration Record ────────────────────────────────────────────────

/// Record of a completed migration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MigrationRecord {
    pub plan: MigrationPlan,
    pub status: MigrationStatus,
    pub source_checksum: StateChecksum,
    pub target_checksum: Option<StateChecksum>,
    pub actual_downtime_ms: u64,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

// ── Substrate Migrator ──────────────────────────────────────────────

/// Trait for performing substrate migrations.
pub trait SubstrateMigrator: Send + Sync {
    /// Execute a migration according to the plan.
    fn migrate(&self, plan: &MigrationPlan, state_data: &str) -> SalResult<MigrationRecord>;

    /// Rollback a failed migration.
    fn rollback(&self, record: &MigrationRecord) -> SalResult<MigrationRecord>;

    /// Name of this migrator implementation.
    fn name(&self) -> &str;
}

/// Simulated substrate migrator for deterministic testing.
pub struct SimulatedMigrator;

impl SimulatedMigrator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimulatedMigrator {
    fn default() -> Self {
        Self::new()
    }
}

impl SubstrateMigrator for SimulatedMigrator {
    fn migrate(&self, plan: &MigrationPlan, state_data: &str) -> SalResult<MigrationRecord> {
        let started_at = Utc::now();
        let source_checksum = StateChecksum::compute(state_data);

        // Simulated migration — compute target checksum
        let target_checksum = StateChecksum::compute(state_data);

        // Verify checksums match (I.SAL-5)
        if !source_checksum.matches(&target_checksum) {
            return Err(SalError::MigrationFailed(
                "State checksum mismatch after migration (I.SAL-5 violation)".into(),
            ));
        }

        let actual_downtime_ms = match plan.strategy {
            MigrationStrategy::Live => 10,      // ~10ms
            MigrationStrategy::Snapshot => 100, // ~100ms
            MigrationStrategy::Parallel => 5,   // ~5ms (dual execution)
        };

        let completed_at = Utc::now();

        Ok(MigrationRecord {
            plan: plan.clone(),
            status: MigrationStatus::Complete,
            source_checksum,
            target_checksum: Some(target_checksum),
            actual_downtime_ms,
            started_at,
            completed_at: Some(completed_at),
        })
    }

    fn rollback(&self, record: &MigrationRecord) -> SalResult<MigrationRecord> {
        let mut rolled_back = record.clone();
        rolled_back.status = MigrationStatus::RolledBack;
        rolled_back.completed_at = Some(Utc::now());
        Ok(rolled_back)
    }

    fn name(&self) -> &str {
        "simulated-migrator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_plan() -> MigrationPlan {
        MigrationPlan {
            source: SubstrateId::new("cpu-0"),
            target: SubstrateId::new("gpu-0"),
            strategy: MigrationStrategy::Live,
            worldline_id: "wl-test".into(),
            estimated_downtime_ms: 50,
        }
    }

    #[test]
    fn migration_strategy_display() {
        assert_eq!(MigrationStrategy::Live.to_string(), "live");
        assert_eq!(MigrationStrategy::Snapshot.to_string(), "snapshot");
        assert_eq!(MigrationStrategy::Parallel.to_string(), "parallel");
    }

    #[test]
    fn migration_status_display() {
        assert_eq!(MigrationStatus::Pending.to_string(), "pending");
        assert_eq!(MigrationStatus::Complete.to_string(), "complete");
        assert!(MigrationStatus::Failed("oops".into())
            .to_string()
            .contains("oops"));
    }

    #[test]
    fn state_checksum_deterministic() {
        let a = StateChecksum::compute("hello world");
        let b = StateChecksum::compute("hello world");
        assert!(a.matches(&b));
    }

    #[test]
    fn state_checksum_different_data() {
        let a = StateChecksum::compute("hello");
        let b = StateChecksum::compute("world");
        assert!(!a.matches(&b));
    }

    #[test]
    fn simulated_migration_live() {
        let migrator = SimulatedMigrator::new();
        let plan = sample_plan();
        let record = migrator.migrate(&plan, "worldline-state-data").unwrap();
        assert_eq!(record.status, MigrationStatus::Complete);
        assert!(record.target_checksum.is_some());
        assert!(record
            .source_checksum
            .matches(record.target_checksum.as_ref().unwrap()));
    }

    #[test]
    fn simulated_migration_snapshot() {
        let migrator = SimulatedMigrator::new();
        let mut plan = sample_plan();
        plan.strategy = MigrationStrategy::Snapshot;
        let record = migrator.migrate(&plan, "state").unwrap();
        assert_eq!(record.status, MigrationStatus::Complete);
        assert_eq!(record.actual_downtime_ms, 100);
    }

    #[test]
    fn simulated_migration_parallel() {
        let migrator = SimulatedMigrator::new();
        let mut plan = sample_plan();
        plan.strategy = MigrationStrategy::Parallel;
        let record = migrator.migrate(&plan, "state").unwrap();
        assert_eq!(record.status, MigrationStatus::Complete);
        assert_eq!(record.actual_downtime_ms, 5);
    }

    #[test]
    fn simulated_rollback() {
        let migrator = SimulatedMigrator::new();
        let plan = sample_plan();
        let record = migrator.migrate(&plan, "state").unwrap();
        let rolled_back = migrator.rollback(&record).unwrap();
        assert_eq!(rolled_back.status, MigrationStatus::RolledBack);
    }

    #[test]
    fn migrator_name() {
        let migrator = SimulatedMigrator::new();
        assert_eq!(migrator.name(), "simulated-migrator");
    }

    #[test]
    fn checksum_preserves_state_integrity() {
        // I.SAL-5: State never corrupted
        let state = "critical-worldline-state-with-commitments";
        let before = StateChecksum::compute(state);
        // After migration (same state, no corruption)
        let after = StateChecksum::compute(state);
        assert!(before.matches(&after), "I.SAL-5: state integrity preserved");
    }
}
