//! Bootstrap engine with bounded history.
//!
//! Wraps `BootstrapManager` with a bounded FIFO of `BootstrapRecord`s
//! for tracking bootstrap operations over time.

use std::collections::VecDeque;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::BootstrapResult;
use crate::manager::BootstrapManager;
use crate::types::{BootstrapConfig, BootstrapId, BootstrapPhase, BootstrapSummary};

// ── Bootstrap Record ────────────────────────────────────────────────

/// Record of a bootstrap operation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BootstrapRecord {
    pub id: BootstrapId,
    pub operation: BootstrapOperation,
    pub from_phase: BootstrapPhase,
    pub to_phase: BootstrapPhase,
    pub success: bool,
    pub error_message: Option<String>,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
}

/// Type of bootstrap operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BootstrapOperation {
    CaptureOrigin,
    Advance,
    Rollback,
}

impl std::fmt::Display for BootstrapOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CaptureOrigin => write!(f, "capture-origin"),
            Self::Advance => write!(f, "advance"),
            Self::Rollback => write!(f, "rollback"),
        }
    }
}

// ── Bootstrap Engine ────────────────────────────────────────────────

/// Engine wrapping the bootstrap manager with bounded record keeping.
pub struct BootstrapEngine {
    manager: BootstrapManager,
    records: VecDeque<BootstrapRecord>,
    max_records: usize,
}

impl BootstrapEngine {
    /// Create with default configuration.
    pub fn new() -> Self {
        let config = BootstrapConfig::default();
        let max = config.max_tracked_records;
        Self {
            manager: BootstrapManager::with_config(config),
            records: VecDeque::new(),
            max_records: max,
        }
    }

    /// Create with specific configuration.
    pub fn with_config(config: BootstrapConfig) -> Self {
        let max = config.max_tracked_records;
        Self {
            manager: BootstrapManager::with_config(config),
            records: VecDeque::new(),
            max_records: max,
        }
    }

    /// Create with a pre-configured manager.
    pub fn with_manager(manager: BootstrapManager, max_records: usize) -> Self {
        Self {
            manager,
            records: VecDeque::new(),
            max_records,
        }
    }

    /// Current bootstrap phase.
    pub fn current_phase(&self) -> &BootstrapPhase {
        self.manager.current_phase()
    }

    /// Whether the system is self-hosting.
    pub fn is_self_hosting(&self) -> bool {
        self.manager.is_self_hosting()
    }

    /// Capture the origin fingerprint.
    pub fn capture_origin(&mut self) -> BootstrapResult<()> {
        let from = self.manager.current_phase().clone();
        match self.manager.capture_origin() {
            Ok(_fp) => {
                self.push_record(BootstrapRecord {
                    id: BootstrapId::new(),
                    operation: BootstrapOperation::CaptureOrigin,
                    from_phase: from.clone(),
                    to_phase: from,
                    success: true,
                    error_message: None,
                    recorded_at: Utc::now(),
                });
                Ok(())
            }
            Err(e) => {
                self.push_record(BootstrapRecord {
                    id: BootstrapId::new(),
                    operation: BootstrapOperation::CaptureOrigin,
                    from_phase: from.clone(),
                    to_phase: from,
                    success: false,
                    error_message: Some(e.to_string()),
                    recorded_at: Utc::now(),
                });
                Err(e)
            }
        }
    }

    /// Advance to the next phase.
    pub fn advance(&mut self) -> BootstrapResult<()> {
        let from = self.manager.current_phase().clone();
        match self.manager.advance() {
            Ok((transition, _report)) => {
                self.push_record(BootstrapRecord {
                    id: BootstrapId::new(),
                    operation: BootstrapOperation::Advance,
                    from_phase: transition.from,
                    to_phase: transition.to,
                    success: true,
                    error_message: None,
                    recorded_at: Utc::now(),
                });
                Ok(())
            }
            Err(e) => {
                self.push_record(BootstrapRecord {
                    id: BootstrapId::new(),
                    operation: BootstrapOperation::Advance,
                    from_phase: from.clone(),
                    to_phase: from,
                    success: false,
                    error_message: Some(e.to_string()),
                    recorded_at: Utc::now(),
                });
                Err(e)
            }
        }
    }

    /// Roll back to the previous phase.
    pub fn rollback(&mut self) -> BootstrapResult<()> {
        let from = self.manager.current_phase().clone();
        match self.manager.rollback() {
            Ok(transition) => {
                self.push_record(BootstrapRecord {
                    id: BootstrapId::new(),
                    operation: BootstrapOperation::Rollback,
                    from_phase: transition.from,
                    to_phase: transition.to,
                    success: true,
                    error_message: None,
                    recorded_at: Utc::now(),
                });
                Ok(())
            }
            Err(e) => {
                self.push_record(BootstrapRecord {
                    id: BootstrapId::new(),
                    operation: BootstrapOperation::Rollback,
                    from_phase: from.clone(),
                    to_phase: from,
                    success: false,
                    error_message: Some(e.to_string()),
                    recorded_at: Utc::now(),
                });
                Err(e)
            }
        }
    }

    /// Push a record with bounded FIFO eviction.
    fn push_record(&mut self, record: BootstrapRecord) {
        if self.records.len() >= self.max_records {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }

    /// Find a record by ID.
    pub fn find(&self, id: &BootstrapId) -> Option<&BootstrapRecord> {
        self.records.iter().find(|r| r.id == *id)
    }

    /// All records.
    pub fn all_records(&self) -> &VecDeque<BootstrapRecord> {
        &self.records
    }

    /// Number of records.
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// Summary statistics.
    pub fn summary(&self) -> BootstrapSummary {
        let total = self.records.iter().filter(|r| r.success).count();
        let advances = self
            .records
            .iter()
            .filter(|r| r.success && r.operation == BootstrapOperation::Advance)
            .count();
        let rollbacks = self
            .records
            .iter()
            .filter(|r| r.success && r.operation == BootstrapOperation::Rollback)
            .count();

        BootstrapSummary {
            total_transitions: total,
            successful_advances: advances,
            rollbacks,
            current_phase_ordinal: self.manager.current_phase().ordinal(),
            highest_phase_reached: self
                .records
                .iter()
                .filter(|r| r.success)
                .map(|r| r.to_phase.ordinal())
                .max()
                .unwrap_or(0),
        }
    }
}

impl Default for BootstrapEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine() -> BootstrapEngine {
        let config = BootstrapConfig {
            enforce_governance: false,
            ..BootstrapConfig::default()
        };
        BootstrapEngine::with_config(config)
    }

    #[test]
    fn engine_starts_at_phase0() {
        let engine = make_engine();
        assert_eq!(
            *engine.current_phase(),
            BootstrapPhase::Phase0ExternalSubstrate
        );
        assert_eq!(engine.record_count(), 0);
    }

    #[test]
    fn engine_capture_and_advance() {
        let mut engine = make_engine();
        engine.capture_origin().unwrap();
        engine.advance().unwrap();
        assert_eq!(
            *engine.current_phase(),
            BootstrapPhase::Phase1ConfigSelfTuning
        );
        assert_eq!(engine.record_count(), 2); // capture + advance
    }

    #[test]
    fn engine_full_bootstrap() {
        let mut engine = make_engine();
        engine.capture_origin().unwrap();
        for _ in 0..5 {
            engine.advance().unwrap();
        }
        assert!(engine.is_self_hosting());
        assert_eq!(engine.record_count(), 6); // 1 capture + 5 advances
    }

    #[test]
    fn engine_rollback() {
        let mut engine = make_engine();
        engine.capture_origin().unwrap();
        engine.advance().unwrap();
        engine.rollback().unwrap();
        assert_eq!(
            *engine.current_phase(),
            BootstrapPhase::Phase0ExternalSubstrate
        );
        assert_eq!(engine.record_count(), 3); // capture + advance + rollback
    }

    #[test]
    fn engine_bounded_fifo() {
        let config = BootstrapConfig {
            max_tracked_records: 3,
            enforce_governance: false,
            ..BootstrapConfig::default()
        };
        let mut engine = BootstrapEngine::with_config(config);
        engine.capture_origin().unwrap();
        for _ in 0..5 {
            engine.advance().unwrap();
        }
        // 6 total records, max 3 → oldest 3 evicted
        assert_eq!(engine.record_count(), 3);
    }

    #[test]
    fn engine_summary() {
        let mut engine = make_engine();
        engine.capture_origin().unwrap();
        engine.advance().unwrap();
        engine.advance().unwrap();
        let summary = engine.summary();
        assert_eq!(summary.successful_advances, 2);
        assert_eq!(summary.current_phase_ordinal, 2);
    }

    #[test]
    fn engine_summary_with_rollback() {
        let mut engine = make_engine();
        engine.capture_origin().unwrap();
        engine.advance().unwrap();
        engine.advance().unwrap();
        engine.rollback().unwrap();
        let summary = engine.summary();
        assert_eq!(summary.successful_advances, 2);
        assert_eq!(summary.rollbacks, 1);
        assert_eq!(summary.current_phase_ordinal, 1);
    }

    #[test]
    fn engine_operation_display() {
        assert_eq!(BootstrapOperation::CaptureOrigin.to_string(), "capture-origin");
        assert_eq!(BootstrapOperation::Advance.to_string(), "advance");
        assert_eq!(BootstrapOperation::Rollback.to_string(), "rollback");
    }
}
