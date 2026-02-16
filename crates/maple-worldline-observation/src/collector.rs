//! Observation collector — the core aggregation engine.
//!
//! Provides:
//! - **RingBuffer**: bounded circular buffer for raw events
//! - **TimeWindow / WindowAggregate**: time-based aggregation
//! - **ObservationCollector**: the main engine that ties everything together

use std::collections::HashMap;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::error::ObservationResult;
use crate::events::{ObservationMetadata, SelfObservationEvent};
use crate::invariants::{
    InvariantChecker, DEFAULT_RING_BUFFER_CAPACITY, MAX_OBSERVATION_MEMORY_BYTES,
    MAX_WINDOWS_PER_SIZE, MIN_SAMPLING_RATE,
};
use crate::snapshot::{ObservationSnapshot, SubsystemSummary};
use crate::usage::UsageAnalytics;

// ── Ring Buffer ─────────────────────────────────────────────────────────

/// A bounded circular buffer.
///
/// When full, the oldest items are silently overwritten.
/// This is the primary storage for raw observation events.
#[derive(Clone, Debug)]
pub struct RingBuffer<T> {
    buffer: Vec<Option<T>>,
    head: usize,
    len: usize,
}

impl<T: Clone> RingBuffer<T> {
    /// Create a ring buffer with the given capacity.
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(1);
        Self {
            buffer: vec![None; cap],
            head: 0,
            len: 0,
        }
    }

    /// Push an item, overwriting the oldest if full.
    pub fn push(&mut self, item: T) {
        self.buffer[self.head] = Some(item);
        self.head = (self.head + 1) % self.buffer.len();
        if self.len < self.buffer.len() {
            self.len += 1;
        }
    }

    /// Iterate over items in insertion order (oldest first).
    pub fn iter(&self) -> RingBufferIter<'_, T> {
        let start = if self.len < self.buffer.len() {
            0
        } else {
            self.head
        };
        RingBufferIter {
            buffer: &self.buffer,
            pos: start,
            remaining: self.len,
        }
    }

    /// Number of items currently in the buffer.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Maximum capacity.
    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }

    /// Utilization as a fraction (0.0 to 1.0).
    pub fn utilization(&self) -> f64 {
        self.len as f64 / self.buffer.len() as f64
    }

    /// Clear all items.
    pub fn clear(&mut self) {
        for slot in &mut self.buffer {
            *slot = None;
        }
        self.head = 0;
        self.len = 0;
    }
}

/// Iterator over a RingBuffer.
pub struct RingBufferIter<'a, T> {
    buffer: &'a [Option<T>],
    pos: usize,
    remaining: usize,
}

impl<'a, T: Clone> Iterator for RingBufferIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let item = self.buffer[self.pos].as_ref();
        self.pos = (self.pos + 1) % self.buffer.len();
        self.remaining -= 1;
        item
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

// ── Time Windows ────────────────────────────────────────────────────────

/// Aggregation time window sizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeWindow {
    OneSecond,
    OneMinute,
    FiveMinutes,
    OneHour,
    TwentyFourHours,
}

impl TimeWindow {
    /// Duration of this window.
    pub fn duration(&self) -> Duration {
        match self {
            Self::OneSecond => Duration::from_secs(1),
            Self::OneMinute => Duration::from_secs(60),
            Self::FiveMinutes => Duration::from_secs(300),
            Self::OneHour => Duration::from_secs(3600),
            Self::TwentyFourHours => Duration::from_secs(86400),
        }
    }
}

/// Aggregated statistics for a time window.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowAggregate {
    /// Which window size.
    pub window: TimeWindow,
    /// Window start time.
    pub start: DateTime<Utc>,
    /// Window end time.
    pub end: DateTime<Utc>,
    /// Total events in this window.
    pub event_count: u64,
    /// Weighted event count (accounting for sampling).
    pub weighted_event_count: f64,
    /// Latency statistics.
    pub latency_sum_ns: u128,
    pub latency_count: u64,
    pub latency_max_ns: u64,
    /// Per-subsystem event counts.
    pub subsystem_counts: HashMap<String, u64>,
    /// Error count.
    pub error_count: u64,
}

impl WindowAggregate {
    /// Create a new empty aggregate.
    pub fn new(window: TimeWindow, start: DateTime<Utc>) -> Self {
        let end = start + chrono::Duration::from_std(window.duration()).unwrap_or_default();
        Self {
            window,
            start,
            end,
            event_count: 0,
            weighted_event_count: 0.0,
            latency_sum_ns: 0,
            latency_count: 0,
            latency_max_ns: 0,
            subsystem_counts: HashMap::new(),
            error_count: 0,
        }
    }

    /// Average latency, if any events had latency.
    pub fn avg_latency(&self) -> Option<Duration> {
        if self.latency_count == 0 {
            None
        } else {
            Some(Duration::from_nanos(
                (self.latency_sum_ns / self.latency_count as u128) as u64,
            ))
        }
    }
}

// ── Sampling Configuration ──────────────────────────────────────────────

/// Adaptive sampling configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SamplingConfig {
    /// Base sampling rate (1.0 = sample everything).
    pub base_rate: f64,
    /// Minimum allowed sampling rate (per I.OBS-5).
    pub min_rate: f64,
    /// Whether adaptive sampling is enabled.
    pub adaptive: bool,
    /// Target overhead fraction for adaptive adjustment.
    pub overhead_target: f64,
}

impl Default for SamplingConfig {
    fn default() -> Self {
        Self {
            base_rate: 1.0,
            min_rate: MIN_SAMPLING_RATE,
            adaptive: true,
            overhead_target: 0.005, // target 0.5%, leave headroom before 1% limit
        }
    }
}

/// Full configuration for the ObservationCollector.
#[derive(Clone, Debug)]
pub struct CollectorConfig {
    /// Ring buffer capacity for raw events.
    pub ring_buffer_capacity: usize,
    /// Maximum memory budget in bytes.
    pub memory_budget_bytes: usize,
    /// Sampling configuration.
    pub sampling: SamplingConfig,
    /// How many aggregated windows to retain per window size.
    pub window_retention: usize,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            ring_buffer_capacity: DEFAULT_RING_BUFFER_CAPACITY,
            memory_budget_bytes: MAX_OBSERVATION_MEMORY_BYTES,
            sampling: SamplingConfig::default(),
            window_retention: MAX_WINDOWS_PER_SIZE,
        }
    }
}

// ── Observation Collector ───────────────────────────────────────────────

/// Observed event paired with its metadata.
#[derive(Clone, Debug)]
pub struct ObservedEvent {
    pub metadata: ObservationMetadata,
    pub event: SelfObservationEvent,
}

/// The main observation aggregation engine.
///
/// Hot path: `record()` — sampling check + conditional ring buffer push.
/// Designed for < 1% overhead (I.OBS-1).
pub struct ObservationCollector {
    /// Configuration.
    config: CollectorConfig,
    /// Ring buffer for raw events.
    ring_buffer: RingBuffer<ObservedEvent>,
    /// Per-subsystem summaries (running).
    subsystem_summaries: HashMap<String, SubsystemSummary>,
    /// Usage analytics (CMS + HLL).
    usage: UsageAnalytics,
    /// Aggregated time windows.
    windows: HashMap<TimeWindow, Vec<WindowAggregate>>,
    /// Current effective sampling rate.
    current_sampling_rate: f64,
    /// Total events observed (including those not sampled into the ring buffer).
    total_events: u64,
    /// Total events stored in ring buffer (after sampling).
    stored_events: u64,
    /// Cumulative observation overhead in nanoseconds.
    overhead_ns: u64,
    /// Start time for throughput calculations.
    started_at: Instant,
}

impl ObservationCollector {
    /// Create a new collector with the given configuration.
    pub fn new(config: CollectorConfig) -> Self {
        let capacity = config.ring_buffer_capacity;
        let rate = config.sampling.base_rate;
        Self {
            config,
            ring_buffer: RingBuffer::new(capacity),
            subsystem_summaries: HashMap::new(),
            usage: UsageAnalytics::new(),
            windows: HashMap::new(),
            current_sampling_rate: rate,
            total_events: 0,
            stored_events: 0,
            overhead_ns: 0,
            started_at: Instant::now(),
        }
    }

    /// Create a collector with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(CollectorConfig::default())
    }

    /// Record an observation event (hot path).
    ///
    /// This is the primary entry point. It:
    /// 1. Increments the total counter (always)
    /// 2. Applies sampling decision
    /// 3. If sampled: stores in ring buffer + updates analytics
    pub fn record(
        &mut self,
        event: SelfObservationEvent,
        metadata: ObservationMetadata,
    ) -> ObservationResult<()> {
        let start = Instant::now();

        self.total_events += 1;

        // Update usage analytics (lightweight, always runs)
        let subsystem_key = metadata.subsystem.to_string();
        self.usage.record_operation(&subsystem_key);
        self.usage.record_event_type(&format!("{:?}", event));
        if let Some(ref wid) = metadata.worldline_id {
            self.usage.record_worldline(&format!("{:?}", wid));
        }

        // Sampling decision
        if !self.should_sample() {
            self.overhead_ns += start.elapsed().as_nanos() as u64;
            return Ok(());
        }

        // Update subsystem summary
        let summary = self
            .subsystem_summaries
            .entry(subsystem_key)
            .or_insert_with(SubsystemSummary::empty);
        summary.record(event.latency(), event.is_error());

        // Store in ring buffer
        self.ring_buffer.push(ObservedEvent { metadata, event });
        self.stored_events += 1;

        // Track overhead
        self.overhead_ns += start.elapsed().as_nanos() as u64;

        // Periodic memory check (every 1000 events)
        if self.stored_events % 1000 == 0 {
            self.maybe_adjust_sampling();
        }

        Ok(())
    }

    /// Take a snapshot of the current observation state.
    pub fn snapshot(&self) -> ObservationSnapshot {
        ObservationSnapshot {
            timestamp: Utc::now(),
            total_events_observed: self.total_events,
            current_sampling_rate: self.current_sampling_rate,
            memory_usage_bytes: self.estimated_memory_bytes(),
            subsystem_summaries: self.subsystem_summaries.clone(),
            usage: self.usage.snapshot(),
        }
    }

    /// Estimate current memory usage in bytes.
    pub fn estimated_memory_bytes(&self) -> usize {
        let ring_buffer_bytes =
            self.ring_buffer.capacity() * std::mem::size_of::<Option<ObservedEvent>>();
        let usage_bytes = self.usage.memory_bytes();
        let window_bytes = self
            .windows
            .values()
            .map(|v| v.len() * std::mem::size_of::<WindowAggregate>())
            .sum::<usize>();
        let summary_bytes =
            self.subsystem_summaries.len() * std::mem::size_of::<SubsystemSummary>();

        ring_buffer_bytes + usage_bytes + window_bytes + summary_bytes
    }

    /// Get current effective sampling rate.
    pub fn current_sampling_rate(&self) -> f64 {
        self.current_sampling_rate
    }

    /// Get total events observed.
    pub fn total_events(&self) -> u64 {
        self.total_events
    }

    /// Get total events stored in ring buffer.
    pub fn stored_events(&self) -> u64 {
        self.stored_events
    }

    /// Get the ring buffer utilization.
    pub fn ring_buffer_utilization(&self) -> f64 {
        self.ring_buffer.utilization()
    }

    /// Current overhead fraction.
    pub fn overhead_fraction(&self) -> f64 {
        let elapsed = self.started_at.elapsed().as_nanos() as f64;
        if elapsed == 0.0 {
            return 0.0;
        }
        self.overhead_ns as f64 / elapsed
    }

    /// Access the underlying usage analytics.
    pub fn usage_analytics(&self) -> &UsageAnalytics {
        &self.usage
    }

    /// Aggregate a time window from the ring buffer.
    pub fn aggregate_window(&mut self, window: TimeWindow) {
        let now = Utc::now();
        let window_duration = chrono::Duration::from_std(window.duration()).unwrap_or_default();
        let window_start = now - window_duration;

        let mut agg = WindowAggregate::new(window, window_start);

        for observed in self.ring_buffer.iter() {
            if observed.metadata.timestamp >= window_start && observed.metadata.timestamp <= now {
                agg.event_count += 1;
                agg.weighted_event_count += observed.metadata.sampling_weight;

                let subsystem_key = observed.metadata.subsystem.to_string();
                *agg.subsystem_counts.entry(subsystem_key).or_default() += 1;

                if observed.event.is_error() {
                    agg.error_count += 1;
                }

                if let Some(latency) = observed.event.latency() {
                    agg.latency_sum_ns += latency.as_nanos();
                    agg.latency_count += 1;
                    agg.latency_max_ns = agg.latency_max_ns.max(latency.as_nanos() as u64);
                }
            }
        }

        let windows = self.windows.entry(window).or_default();
        windows.push(agg);

        // Evict old windows
        if windows.len() > self.config.window_retention {
            let excess = windows.len() - self.config.window_retention;
            windows.drain(0..excess);
        }
    }

    /// Get aggregated windows for a given size.
    pub fn get_windows(&self, window: &TimeWindow) -> &[WindowAggregate] {
        self.windows
            .get(window)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Manually adjust the sampling rate.
    pub fn set_sampling_rate(&mut self, rate: f64) -> ObservationResult<()> {
        InvariantChecker::validate_sampling_rate(rate)?;
        self.current_sampling_rate = rate;
        Ok(())
    }

    /// Whether an event should be sampled (probabilistic).
    fn should_sample(&self) -> bool {
        if self.current_sampling_rate >= 1.0 {
            return true;
        }
        let mut rng = rand::thread_rng();
        rng.gen::<f64>() < self.current_sampling_rate
    }

    /// Adaptively adjust sampling rate based on overhead.
    fn maybe_adjust_sampling(&mut self) {
        if !self.config.sampling.adaptive {
            return;
        }

        let overhead = self.overhead_fraction();
        if overhead > self.config.sampling.overhead_target {
            // Reduce sampling rate
            let new_rate = (self.current_sampling_rate * 0.9).max(self.config.sampling.min_rate);
            self.current_sampling_rate = new_rate;
            tracing::debug!(
                overhead = %format!("{:.4}%", overhead * 100.0),
                new_rate = %format!("{:.4}", new_rate),
                "adaptive sampling: reducing rate"
            );
        } else if overhead < self.config.sampling.overhead_target * 0.5 {
            // Can increase sampling rate
            let new_rate = (self.current_sampling_rate * 1.05).min(self.config.sampling.base_rate);
            self.current_sampling_rate = new_rate;
        }
    }
}

impl Default for ObservationCollector {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{MemoryOperationType, SubsystemId};
    use worldline_core::types::EventId;
    use worldline_runtime::fabric::ResonanceStage;

    // ── RingBuffer tests ────────────────────────────────────────────

    #[test]
    fn ring_buffer_basic() {
        let mut rb: RingBuffer<i32> = RingBuffer::new(4);
        assert!(rb.is_empty());
        assert_eq!(rb.capacity(), 4);

        rb.push(1);
        rb.push(2);
        rb.push(3);

        assert_eq!(rb.len(), 3);
        assert!(!rb.is_empty());

        let items: Vec<&i32> = rb.iter().collect();
        assert_eq!(items, vec![&1, &2, &3]);
    }

    #[test]
    fn ring_buffer_wrap_around() {
        let mut rb: RingBuffer<i32> = RingBuffer::new(3);
        rb.push(1);
        rb.push(2);
        rb.push(3);
        rb.push(4); // overwrites 1

        assert_eq!(rb.len(), 3);
        let items: Vec<&i32> = rb.iter().collect();
        assert_eq!(items, vec![&2, &3, &4]);
    }

    #[test]
    fn ring_buffer_utilization() {
        let mut rb: RingBuffer<i32> = RingBuffer::new(10);
        assert!((rb.utilization() - 0.0).abs() < f64::EPSILON);

        for i in 0..5 {
            rb.push(i);
        }
        assert!((rb.utilization() - 0.5).abs() < f64::EPSILON);

        for i in 5..10 {
            rb.push(i);
        }
        assert!((rb.utilization() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ring_buffer_clear() {
        let mut rb: RingBuffer<i32> = RingBuffer::new(5);
        rb.push(1);
        rb.push(2);
        rb.clear();
        assert!(rb.is_empty());
        assert_eq!(rb.len(), 0);
    }

    #[test]
    fn ring_buffer_single_capacity() {
        let mut rb: RingBuffer<i32> = RingBuffer::new(1);
        rb.push(1);
        assert_eq!(rb.len(), 1);
        rb.push(2);
        assert_eq!(rb.len(), 1);
        let items: Vec<&i32> = rb.iter().collect();
        assert_eq!(items, vec![&2]);
    }

    // ── TimeWindow tests ────────────────────────────────────────────

    #[test]
    fn time_window_durations() {
        assert_eq!(TimeWindow::OneSecond.duration(), Duration::from_secs(1));
        assert_eq!(TimeWindow::OneMinute.duration(), Duration::from_secs(60));
        assert_eq!(TimeWindow::FiveMinutes.duration(), Duration::from_secs(300));
        assert_eq!(TimeWindow::OneHour.duration(), Duration::from_secs(3600));
        assert_eq!(
            TimeWindow::TwentyFourHours.duration(),
            Duration::from_secs(86400)
        );
    }

    // ── WindowAggregate tests ───────────────────────────────────────

    #[test]
    fn window_aggregate_avg_latency() {
        let mut agg = WindowAggregate::new(TimeWindow::OneMinute, Utc::now());
        assert!(agg.avg_latency().is_none());

        agg.latency_sum_ns = 30_000_000; // 30ms total
        agg.latency_count = 3;
        let avg = agg.avg_latency().unwrap();
        assert_eq!(avg, Duration::from_millis(10));
    }

    // ── ObservationCollector tests ──────────────────────────────────

    fn make_event(subsystem: SubsystemId) -> (SelfObservationEvent, ObservationMetadata) {
        let event = SelfObservationEvent::FabricEventEmitted {
            event_id: EventId::new(),
            stage: ResonanceStage::Meaning,
            latency: Duration::from_millis(5),
            payload_bytes: 128,
        };
        let metadata = ObservationMetadata::now(subsystem);
        (event, metadata)
    }

    #[test]
    fn collector_record_basic() {
        let mut collector = ObservationCollector::with_defaults();

        let (event, metadata) = make_event(SubsystemId::EventFabric);
        collector.record(event, metadata).unwrap();

        assert_eq!(collector.total_events(), 1);
        assert!(collector.stored_events() >= 1); // may be 0 if sampling dropped it, but rate=1.0
    }

    #[test]
    fn collector_snapshot() {
        let mut collector = ObservationCollector::with_defaults();

        for _ in 0..10 {
            let (event, metadata) = make_event(SubsystemId::EventFabric);
            collector.record(event, metadata).unwrap();
        }

        let snap = collector.snapshot();
        assert_eq!(snap.total_events_observed, 10);
        assert!(snap.is_healthy());
    }

    #[test]
    fn collector_subsystem_tracking() {
        let mut collector = ObservationCollector::with_defaults();

        for _ in 0..5 {
            let (event, metadata) = make_event(SubsystemId::EventFabric);
            collector.record(event, metadata).unwrap();
        }

        let event = SelfObservationEvent::GateSubmission {
            commitment_id: worldline_core::types::CommitmentId::new(),
            stages_evaluated: 7,
            total_latency: Duration::from_millis(50),
            approved: false,
        };
        let metadata = ObservationMetadata::now(SubsystemId::CommitmentGate);
        collector.record(event, metadata).unwrap();

        let snap = collector.snapshot();
        assert!(snap.subsystem_summaries.contains_key("event-fabric"));
        assert!(snap.subsystem_summaries.contains_key("commitment-gate"));
    }

    #[test]
    fn collector_memory_estimation() {
        let collector = ObservationCollector::with_defaults();
        let mem = collector.estimated_memory_bytes();
        // Should be well within the 64MB budget
        assert!(mem < MAX_OBSERVATION_MEMORY_BYTES);
    }

    #[test]
    fn collector_sampling_rate_manual() {
        let mut collector = ObservationCollector::with_defaults();
        collector.set_sampling_rate(0.5).unwrap();
        assert!((collector.current_sampling_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn collector_sampling_rate_validation() {
        let mut collector = ObservationCollector::with_defaults();
        assert!(collector.set_sampling_rate(0.0).is_err());
        assert!(collector.set_sampling_rate(1.5).is_err());
    }

    #[test]
    fn collector_window_aggregation() {
        let mut collector = ObservationCollector::with_defaults();

        for _ in 0..20 {
            let (event, metadata) = make_event(SubsystemId::EventFabric);
            collector.record(event, metadata).unwrap();
        }

        collector.aggregate_window(TimeWindow::OneSecond);
        let windows = collector.get_windows(&TimeWindow::OneSecond);
        assert_eq!(windows.len(), 1);
        // All events happened within the last second
        assert!(windows[0].event_count >= 1);
    }

    #[test]
    fn collector_with_memory_events() {
        let mut collector = ObservationCollector::with_defaults();

        let event = SelfObservationEvent::MemoryOperation {
            operation: MemoryOperationType::Store,
            plane: "working".into(),
            latency: Duration::from_micros(500),
            entries_affected: 1,
        };
        let metadata = ObservationMetadata::now(SubsystemId::MemoryEngine);
        collector.record(event, metadata).unwrap();

        let snap = collector.snapshot();
        assert!(snap.subsystem_summaries.contains_key("memory-engine"));
    }

    #[test]
    fn collector_overhead_tracking() {
        let collector = ObservationCollector::with_defaults();
        // Initially zero overhead
        assert!(collector.overhead_fraction() < 0.01);
    }

    #[test]
    fn collector_config_custom() {
        let config = CollectorConfig {
            ring_buffer_capacity: 1024,
            memory_budget_bytes: 1024 * 1024,
            sampling: SamplingConfig {
                base_rate: 0.5,
                min_rate: 0.01,
                adaptive: false,
                overhead_target: 0.01,
            },
            window_retention: 100,
        };
        let collector = ObservationCollector::new(config);
        assert!((collector.current_sampling_rate() - 0.5).abs() < f64::EPSILON);
    }
}
