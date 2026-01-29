//! Resonance Scheduler - attention-aware task scheduling

use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::types::*;
use crate::runtime_core::{ScheduleHandle, TaskId, RejectionReason, DeferralReason};
use crate::config::SchedulingConfig;

/// Schedules resonance processing respecting attention budgets
pub struct ResonanceScheduler {
    /// Priority queues by attention class
    queues: Arc<RwLock<HashMap<AttentionClass, PriorityQueue>>>,

    /// Circuit breakers for overload protection
    circuit_breakers: Arc<RwLock<HashMap<ResonatorId, CircuitBreaker>>>,

    /// Configuration
    config: SchedulingConfig,
}

impl ResonanceScheduler {
    pub fn new(config: &SchedulingConfig) -> Self {
        let mut queues = HashMap::new();
        queues.insert(AttentionClass::Critical, PriorityQueue::new());
        queues.insert(AttentionClass::High, PriorityQueue::new());
        queues.insert(AttentionClass::Normal, PriorityQueue::new());
        queues.insert(AttentionClass::Low, PriorityQueue::new());

        Self {
            queues: Arc::new(RwLock::new(queues)),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            config: config.clone(),
        }
    }

    /// Schedule a resonance task
    ///
    /// This respects attention budgets and circuit breakers.
    pub async fn schedule(&self, task: ResonanceTask) -> ScheduleHandle {
        // Check circuit breaker
        let circuit_breakers = self.circuit_breakers.read().await;
        if let Some(breaker) = circuit_breakers.get(&task.target) {
            if breaker.is_open() {
                tracing::warn!("Circuit breaker open for {}", task.target);
                return ScheduleHandle::rejected(RejectionReason::CircuitOpen);
            }
        }
        drop(circuit_breakers);

        // Determine attention class
        let attention_class = self.classify_attention(&task);

        // Check if attention is available (placeholder)
        if !self.has_attention_for(&task) {
            tracing::debug!("Attention unavailable for task {}", task.id);
            return ScheduleHandle::deferred(DeferralReason::AttentionUnavailable);
        }

        // Check queue capacity
        let mut queues = self.queues.write().await;
        let queue = queues.get_mut(&attention_class).unwrap();

        if queue.is_full(self.config.max_queue_size) {
            tracing::warn!("Queue full for attention class {:?}", attention_class);
            return ScheduleHandle::rejected(RejectionReason::QueueFull);
        }

        // Add to appropriate queue
        queue.push(task.clone());

        tracing::debug!(
            "Scheduled task {} for {} (class: {:?})",
            task.id,
            task.target,
            attention_class
        );

        ScheduleHandle::scheduled(task.id)
    }

    /// Classify attention requirements for a task
    fn classify_attention(&self, task: &ResonanceTask) -> AttentionClass {
        // In real implementation, would analyze task characteristics
        task.attention_class
    }

    /// Check if attention is available (placeholder)
    fn has_attention_for(&self, _task: &ResonanceTask) -> bool {
        // In real implementation, would check attention allocator
        true
    }

    /// Trip circuit breaker for a Resonator
    pub async fn trip_circuit_breaker(&self, resonator: ResonatorId) {
        let mut breakers = self.circuit_breakers.write().await;
        let breaker = breakers
            .entry(resonator)
            .or_insert_with(|| CircuitBreaker::new(self.config.circuit_breaker_threshold));

        breaker.trip();
        tracing::warn!("Circuit breaker tripped for {}", resonator);
    }

    /// Reset circuit breaker for a Resonator
    pub async fn reset_circuit_breaker(&self, resonator: &ResonatorId) {
        let mut breakers = self.circuit_breakers.write().await;
        if let Some(breaker) = breakers.get_mut(resonator) {
            breaker.reset();
            tracing::info!("Circuit breaker reset for {}", resonator);
        }
    }

    /// Get next task to process (for worker)
    pub async fn next_task(&self) -> Option<ResonanceTask> {
        let mut queues = self.queues.write().await;

        // Process in priority order: Critical > High > Normal > Low
        for class in &[
            AttentionClass::Critical,
            AttentionClass::High,
            AttentionClass::Normal,
            AttentionClass::Low,
        ] {
            if let Some(queue) = queues.get_mut(class) {
                if let Some(task) = queue.pop() {
                    return Some(task);
                }
            }
        }

        None
    }
}

/// A resonance task to be scheduled
#[derive(Debug, Clone)]
pub struct ResonanceTask {
    pub id: TaskId,
    pub target: ResonatorId,
    pub attention_class: AttentionClass,
    pub priority: u32,
    pub payload: TaskPayload,
}

impl ResonanceTask {
    pub fn new(
        target: ResonatorId,
        attention_class: AttentionClass,
        payload: TaskPayload,
    ) -> Self {
        Self {
            id: TaskId::generate(),
            target,
            attention_class,
            priority: 0,
            payload,
        }
    }
}

/// Task payload (placeholder)
#[derive(Debug, Clone)]
pub enum TaskPayload {
    ProcessCoupling(CouplingId),
    FormMeaning,
    StabilizeIntent,
    ExecuteCommitment(CommitmentId),
}

/// Priority queue for resonance tasks
struct PriorityQueue {
    heap: BinaryHeap<PrioritizedTask>,
}

impl PriorityQueue {
    fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
        }
    }

    fn push(&mut self, task: ResonanceTask) {
        self.heap.push(PrioritizedTask(task));
    }

    fn pop(&mut self) -> Option<ResonanceTask> {
        self.heap.pop().map(|pt| pt.0)
    }

    fn is_full(&self, max_size: usize) -> bool {
        self.heap.len() >= max_size
    }
}

/// Wrapper for priority ordering
struct PrioritizedTask(ResonanceTask);

impl Ord for PrioritizedTask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.priority.cmp(&other.0.priority)
    }
}

impl PartialOrd for PrioritizedTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for PrioritizedTask {}

impl PartialEq for PrioritizedTask {
    fn eq(&self, other: &Self) -> bool {
        self.0.priority == other.0.priority
    }
}

/// Circuit breaker for overload protection
struct CircuitBreaker {
    state: CircuitBreakerState,
    failure_threshold: u32,
    failure_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    fn new(failure_threshold: u32) -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            failure_threshold,
            failure_count: 0,
        }
    }

    fn is_open(&self) -> bool {
        self.state == CircuitBreakerState::Open
    }

    fn trip(&mut self) {
        self.failure_count += 1;
        if self.failure_count >= self.failure_threshold {
            self.state = CircuitBreakerState::Open;
        }
    }

    fn reset(&mut self) {
        self.state = CircuitBreakerState::Closed;
        self.failure_count = 0;
    }
}
