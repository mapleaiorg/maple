//! Handles for interacting with runtime entities

use crate::runtime_core::MapleRuntime;
use crate::types::*;

/// Handle to a registered Resonator
///
/// This handle provides access to Resonator operations while maintaining
/// safety guarantees.
#[derive(Clone)]
pub struct ResonatorHandle {
    pub id: ResonatorId,
    runtime: MapleRuntime,
}

impl ResonatorHandle {
    pub(crate) fn new(id: ResonatorId, runtime: MapleRuntime) -> Self {
        Self { id, runtime }
    }

    /// Signal presence
    pub async fn signal_presence(&self, state: PresenceState) -> Result<(), PresenceError> {
        self.runtime
            .presence_fabric()
            .signal_presence(self.id, state)
            .await
    }

    /// Enable silent mode
    pub async fn enable_silent_mode(&self) {
        self.runtime
            .presence_fabric()
            .enable_silent_presence(self.id)
            .await
    }

    /// Get current presence state
    pub fn get_presence(&self) -> Option<PresenceState> {
        self.runtime.presence_fabric().get_presence(&self.id)
    }

    /// Establish coupling with another Resonator
    pub async fn couple_with(
        &self,
        _target: ResonatorId,
        params: CouplingParams,
    ) -> Result<CouplingHandle, CouplingError> {
        let (coupling_id, attention_token) = self
            .runtime
            .coupling_fabric()
            .establish_coupling(params)
            .await?;

        Ok(CouplingHandle::new(
            coupling_id,
            attention_token,
            self.runtime.clone(),
        ))
    }

    /// Get attention budget status
    pub async fn attention_status(&self) -> Option<AttentionBudget> {
        self.runtime
            .attention_allocator()
            .get_budget(&self.id)
            .await
    }
}

/// Handle to a coupling relationship
#[derive(Clone)]
pub struct CouplingHandle {
    pub id: CouplingId,
    _attention_token: AllocationToken,
    runtime: MapleRuntime,
}

impl CouplingHandle {
    pub(crate) fn new(
        id: CouplingId,
        attention_token: AllocationToken,
        runtime: MapleRuntime,
    ) -> Self {
        Self {
            id,
            _attention_token: attention_token,
            runtime,
        }
    }

    /// Strengthen the coupling
    pub async fn strengthen(&self, delta: f64) -> Result<(), CouplingError> {
        self.runtime
            .coupling_fabric()
            .strengthen(self.id, delta)
            .await
    }

    /// Weaken the coupling
    pub async fn weaken(&self, factor: f64) -> Result<(), CouplingError> {
        self.runtime.coupling_fabric().weaken(self.id, factor).await
    }

    /// Safely sever the coupling
    pub async fn decouple(self) -> Result<DecouplingResult, CouplingError> {
        self.runtime
            .coupling_fabric()
            .decouple_safely(self.id)
            .await
    }

    /// Get coupling state
    pub fn get_coupling(&self) -> Option<Coupling> {
        self.runtime.coupling_fabric().get_coupling(&self.id)
    }
}

/// Result of decoupling operation
#[derive(Debug, Clone)]
pub enum DecouplingResult {
    Success,
    PartialSuccess { reason: String },
}

/// Handle to a scheduled resonance task
#[derive(Debug, Clone)]
pub struct ScheduleHandle {
    pub task_id: Option<TaskId>,
    pub status: ScheduleStatus,
}

impl ScheduleHandle {
    pub(crate) fn scheduled(task_id: TaskId) -> Self {
        Self {
            task_id: Some(task_id),
            status: ScheduleStatus::Scheduled,
        }
    }

    pub(crate) fn rejected(reason: RejectionReason) -> Self {
        Self {
            task_id: None,
            status: ScheduleStatus::Rejected(reason),
        }
    }

    pub(crate) fn deferred(reason: DeferralReason) -> Self {
        Self {
            task_id: None,
            status: ScheduleStatus::Deferred(reason),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ScheduleStatus {
    Scheduled,
    Rejected(RejectionReason),
    Deferred(DeferralReason),
}

#[derive(Debug, Clone)]
pub enum RejectionReason {
    CircuitOpen,
    AttentionUnavailable,
    QueueFull,
}

#[derive(Debug, Clone)]
pub enum DeferralReason {
    AttentionUnavailable,
    BackPressure,
}

/// Task identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(uuid::Uuid);

impl TaskId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "task:{}", self.0)
    }
}
