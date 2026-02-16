//! Performance profiler for subsystem-level latency and throughput analysis.
//!
//! The profiler runs bounded sessions that collect latency samples and
//! compute p50/p95/p99 percentile profiles.

use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ObservationError, ObservationResult};
use crate::events::SubsystemId;

/// Maximum samples per profiling session (memory bound).
pub const MAX_SAMPLES_PER_SESSION: usize = 10_000;

/// Maximum concurrent profiling sessions.
pub const MAX_CONCURRENT_SESSIONS: usize = 16;

/// Maximum recent profiles to retain.
pub const MAX_RECENT_PROFILES: usize = 64;

/// A single profiling sample.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfilingSample {
    /// When the sample was taken.
    pub timestamp: DateTime<Utc>,
    /// Operation that was profiled.
    pub operation: String,
    /// Measured latency.
    pub latency: Duration,
    /// Optional metadata (e.g., payload size).
    pub metadata: HashMap<String, String>,
}

/// An active profiling session collecting samples.
#[derive(Clone, Debug)]
pub struct ProfilingSession {
    /// Unique session identifier.
    pub session_id: String,
    /// Which subsystem is being profiled.
    pub subsystem: SubsystemId,
    /// When the session started.
    pub started_at: DateTime<Utc>,
    /// Maximum samples to collect before auto-closing.
    pub max_samples: usize,
    /// Collected samples.
    samples: Vec<ProfilingSample>,
}

impl ProfilingSession {
    /// Create a new profiling session.
    pub fn new(subsystem: SubsystemId, max_samples: usize) -> Self {
        Self {
            session_id: Uuid::new_v4().to_string(),
            subsystem,
            started_at: Utc::now(),
            max_samples: max_samples.min(MAX_SAMPLES_PER_SESSION),
            samples: Vec::new(),
        }
    }

    /// Record a sample in this session.
    ///
    /// Returns false if the session is full.
    pub fn record(&mut self, sample: ProfilingSample) -> bool {
        if self.samples.len() >= self.max_samples {
            return false;
        }
        self.samples.push(sample);
        true
    }

    /// Number of samples collected.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Whether this session has reached its sample limit.
    pub fn is_full(&self) -> bool {
        self.samples.len() >= self.max_samples
    }

    /// Compute a profile from the collected samples.
    pub fn compute_profile(&self) -> SubsystemProfile {
        let duration = Utc::now()
            .signed_duration_since(self.started_at)
            .to_std()
            .unwrap_or(Duration::from_secs(1));

        if self.samples.is_empty() {
            return SubsystemProfile {
                subsystem: self.subsystem.clone(),
                session_id: self.session_id.clone(),
                sample_count: 0,
                duration,
                p50_latency: Duration::ZERO,
                p95_latency: Duration::ZERO,
                p99_latency: Duration::ZERO,
                min_latency: Duration::ZERO,
                max_latency: Duration::ZERO,
                avg_latency: Duration::ZERO,
                throughput_per_sec: 0.0,
                top_operations: Vec::new(),
            };
        }

        let mut latencies: Vec<Duration> = self.samples.iter().map(|s| s.latency).collect();
        latencies.sort();

        let n = latencies.len();
        let sum_ns: u128 = latencies.iter().map(|d| d.as_nanos()).sum();
        let avg_ns = sum_ns / n as u128;

        // Count operations
        let mut op_counts: HashMap<String, usize> = HashMap::new();
        for sample in &self.samples {
            *op_counts.entry(sample.operation.clone()).or_default() += 1;
        }
        let mut top_ops: Vec<(String, usize)> = op_counts.into_iter().collect();
        top_ops.sort_by(|a, b| b.1.cmp(&a.1));
        top_ops.truncate(10);

        SubsystemProfile {
            subsystem: self.subsystem.clone(),
            session_id: self.session_id.clone(),
            sample_count: n,
            duration,
            p50_latency: latencies[n * 50 / 100],
            p95_latency: latencies[n * 95 / 100],
            p99_latency: latencies[(n * 99 / 100).min(n - 1)],
            min_latency: latencies[0],
            max_latency: latencies[n - 1],
            avg_latency: Duration::from_nanos(avg_ns as u64),
            throughput_per_sec: n as f64 / duration.as_secs_f64(),
            top_operations: top_ops,
        }
    }
}

/// Computed performance profile for a subsystem.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubsystemProfile {
    /// Which subsystem was profiled.
    pub subsystem: SubsystemId,
    /// Session that produced this profile.
    pub session_id: String,
    /// Number of samples used.
    pub sample_count: usize,
    /// Duration of the profiling session.
    pub duration: Duration,
    /// 50th percentile latency.
    pub p50_latency: Duration,
    /// 95th percentile latency.
    pub p95_latency: Duration,
    /// 99th percentile latency.
    pub p99_latency: Duration,
    /// Minimum observed latency.
    pub min_latency: Duration,
    /// Maximum observed latency.
    pub max_latency: Duration,
    /// Average latency.
    pub avg_latency: Duration,
    /// Operations per second.
    pub throughput_per_sec: f64,
    /// Top operations by frequency.
    pub top_operations: Vec<(String, usize)>,
}

/// Manages profiling sessions and retains recent profiles.
pub struct PerformanceProfiler {
    /// Active sessions by session_id.
    active_sessions: HashMap<String, ProfilingSession>,
    /// Completed profiles (bounded ring).
    recent_profiles: Vec<SubsystemProfile>,
    /// Maximum concurrent sessions.
    max_sessions: usize,
}

impl PerformanceProfiler {
    /// Create a new profiler.
    pub fn new() -> Self {
        Self {
            active_sessions: HashMap::new(),
            recent_profiles: Vec::new(),
            max_sessions: MAX_CONCURRENT_SESSIONS,
        }
    }

    /// Start a profiling session for a subsystem.
    pub fn start_session(
        &mut self,
        subsystem: SubsystemId,
        max_samples: usize,
    ) -> ObservationResult<String> {
        if self.active_sessions.len() >= self.max_sessions {
            return Err(ObservationError::InvariantViolation {
                invariant: "max_concurrent_sessions".into(),
                detail: format!("already {} active sessions", self.active_sessions.len()),
            });
        }
        let session = ProfilingSession::new(subsystem, max_samples);
        let id = session.session_id.clone();
        self.active_sessions.insert(id.clone(), session);
        Ok(id)
    }

    /// Record a sample in an active session.
    pub fn record_sample(
        &mut self,
        session_id: &str,
        sample: ProfilingSample,
    ) -> ObservationResult<()> {
        let session = self
            .active_sessions
            .get_mut(session_id)
            .ok_or_else(|| ObservationError::SessionNotFound(session_id.to_string()))?;

        if !session.record(sample) {
            // Session is full — auto-end it
            let profile = session.compute_profile();
            self.store_profile(profile);
            self.active_sessions.remove(session_id);
        }
        Ok(())
    }

    /// End a profiling session and produce a profile.
    pub fn end_session(&mut self, session_id: &str) -> ObservationResult<SubsystemProfile> {
        let session = self
            .active_sessions
            .remove(session_id)
            .ok_or_else(|| ObservationError::SessionNotFound(session_id.to_string()))?;

        let profile = session.compute_profile();
        self.store_profile(profile.clone());
        Ok(profile)
    }

    /// Get a list of active session IDs.
    pub fn active_sessions(&self) -> Vec<String> {
        self.active_sessions.keys().cloned().collect()
    }

    /// Get the number of active sessions.
    pub fn active_session_count(&self) -> usize {
        self.active_sessions.len()
    }

    /// Get recent completed profiles.
    pub fn recent_profiles(&self) -> &[SubsystemProfile] {
        &self.recent_profiles
    }

    /// Store a profile, evicting oldest if at capacity.
    fn store_profile(&mut self, profile: SubsystemProfile) {
        if self.recent_profiles.len() >= MAX_RECENT_PROFILES {
            self.recent_profiles.remove(0);
        }
        self.recent_profiles.push(profile);
    }
}

impl Default for PerformanceProfiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sample(op: &str, latency_ms: u64) -> ProfilingSample {
        ProfilingSample {
            timestamp: Utc::now(),
            operation: op.to_string(),
            latency: Duration::from_millis(latency_ms),
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn session_collect_and_profile() {
        let mut session = ProfilingSession::new(SubsystemId::EventFabric, 100);

        for i in 0..50 {
            session.record(make_sample("emit", i + 1));
        }

        let profile = session.compute_profile();
        assert_eq!(profile.sample_count, 50);
        assert!(profile.p50_latency >= Duration::from_millis(25));
        assert!(profile.p99_latency >= Duration::from_millis(49));
        assert!(profile.min_latency == Duration::from_millis(1));
        assert!(profile.max_latency == Duration::from_millis(50));
    }

    #[test]
    fn session_respects_max_samples() {
        let mut session = ProfilingSession::new(SubsystemId::CommitmentGate, 5);

        for i in 0..10 {
            session.record(make_sample("submit", i));
        }

        assert_eq!(session.sample_count(), 5);
        assert!(session.is_full());
    }

    #[test]
    fn empty_session_profile() {
        let session = ProfilingSession::new(SubsystemId::MemoryEngine, 100);
        let profile = session.compute_profile();
        assert_eq!(profile.sample_count, 0);
        assert_eq!(profile.p50_latency, Duration::ZERO);
    }

    #[test]
    fn profiler_session_lifecycle() {
        let mut profiler = PerformanceProfiler::new();

        let sid = profiler
            .start_session(SubsystemId::EventFabric, 100)
            .unwrap();

        profiler
            .record_sample(&sid, make_sample("emit", 5))
            .unwrap();
        profiler
            .record_sample(&sid, make_sample("emit", 10))
            .unwrap();

        assert_eq!(profiler.active_session_count(), 1);

        let profile = profiler.end_session(&sid).unwrap();
        assert_eq!(profile.sample_count, 2);
        assert_eq!(profiler.active_session_count(), 0);
        assert_eq!(profiler.recent_profiles().len(), 1);
    }

    #[test]
    fn profiler_session_not_found() {
        let mut profiler = PerformanceProfiler::new();
        assert!(profiler.end_session("nonexistent").is_err());
    }

    #[test]
    fn profiler_max_sessions_enforced() {
        let mut profiler = PerformanceProfiler::new();

        for _ in 0..MAX_CONCURRENT_SESSIONS {
            profiler
                .start_session(SubsystemId::EventFabric, 10)
                .unwrap();
        }

        assert!(profiler
            .start_session(SubsystemId::EventFabric, 10)
            .is_err());
    }

    #[test]
    fn profiler_recent_profiles_bounded() {
        let mut profiler = PerformanceProfiler::new();

        for i in 0..(MAX_RECENT_PROFILES + 10) {
            let sid = profiler
                .start_session(SubsystemId::EventFabric, 10)
                .unwrap();
            profiler
                .record_sample(&sid, make_sample("op", i as u64))
                .unwrap();
            profiler.end_session(&sid).unwrap();
        }

        assert!(profiler.recent_profiles().len() <= MAX_RECENT_PROFILES);
    }

    #[test]
    fn profile_top_operations() {
        let mut session = ProfilingSession::new(SubsystemId::EventFabric, 100);

        for _ in 0..30 {
            session.record(make_sample("emit", 5));
        }
        for _ in 0..20 {
            session.record(make_sample("route", 3));
        }
        for _ in 0..10 {
            session.record(make_sample("subscribe", 1));
        }

        let profile = session.compute_profile();
        assert_eq!(profile.top_operations[0].0, "emit");
        assert_eq!(profile.top_operations[0].1, 30);
    }

    #[test]
    fn profile_serialization() {
        let mut session = ProfilingSession::new(SubsystemId::MrpRouter, 10);
        session.record(make_sample("route", 5));
        let profile = session.compute_profile();

        let json = serde_json::to_string(&profile).unwrap();
        let restored: SubsystemProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.sample_count, 1);
    }
}
