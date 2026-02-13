//! Probabilistic data structures for bounded-memory usage analytics.
//!
//! - **Count-Min Sketch**: frequency estimation (operation counts)
//! - **HyperLogLog**: cardinality estimation (unique entities)
//!
//! Both enforce I.OBS-4 (bounded memory) by construction.

use serde::{Deserialize, Serialize};

/// Default CMS width (columns per row).
pub const DEFAULT_CMS_WIDTH: usize = 1024;

/// Default CMS depth (number of hash rows).
pub const DEFAULT_CMS_DEPTH: usize = 4;

/// Default HLL precision parameter (log2 of register count).
pub const DEFAULT_HLL_PRECISION: u8 = 12;

// ── Count-Min Sketch ────────────────────────────────────────────────────

/// A Count-Min Sketch for frequency estimation.
///
/// Memory: width × depth × 8 bytes (u64 counters).
/// Default: 1024 × 4 = 32 KB.
#[derive(Clone, Debug)]
pub struct CountMinSketch {
    width: usize,
    depth: usize,
    counters: Vec<Vec<u64>>,
    seeds: Vec<u64>,
    total: u64,
}

impl CountMinSketch {
    /// Create a new CMS with the given dimensions.
    pub fn new(width: usize, depth: usize) -> Self {
        let mut seeds = Vec::with_capacity(depth);
        for i in 0..depth {
            // Deterministic seeds using golden-ratio hashing
            seeds.push(0x9E3779B97F4A7C15_u64.wrapping_mul((i as u64) + 1));
        }
        Self {
            width,
            depth,
            counters: vec![vec![0u64; width]; depth],
            seeds,
            total: 0,
        }
    }

    /// Create a CMS with default dimensions (1024 × 4).
    pub fn default_size() -> Self {
        Self::new(DEFAULT_CMS_WIDTH, DEFAULT_CMS_DEPTH)
    }

    /// Increment the count for a key.
    pub fn increment(&mut self, key: &str) {
        self.increment_by(key, 1);
    }

    /// Increment the count for a key by a specific amount.
    pub fn increment_by(&mut self, key: &str, count: u64) {
        for row in 0..self.depth {
            let idx = self.hash(key, row);
            self.counters[row][idx] = self.counters[row][idx].saturating_add(count);
        }
        self.total = self.total.saturating_add(count);
    }

    /// Estimate the count for a key (always >= true count).
    pub fn estimate(&self, key: &str) -> u64 {
        let mut min = u64::MAX;
        for row in 0..self.depth {
            let idx = self.hash(key, row);
            min = min.min(self.counters[row][idx]);
        }
        min
    }

    /// Total number of increments recorded.
    pub fn total(&self) -> u64 {
        self.total
    }

    /// Estimated memory usage in bytes.
    pub fn memory_bytes(&self) -> usize {
        self.width * self.depth * std::mem::size_of::<u64>()
            + self.depth * std::mem::size_of::<u64>() // seeds
            + std::mem::size_of::<Self>()
    }

    /// Reset all counters to zero.
    pub fn reset(&mut self) {
        for row in &mut self.counters {
            for counter in row.iter_mut() {
                *counter = 0;
            }
        }
        self.total = 0;
    }

    /// Polynomial hash with per-row seed.
    fn hash(&self, key: &str, row: usize) -> usize {
        let seed = self.seeds[row];
        let mut h = seed;
        for byte in key.as_bytes() {
            h = h.wrapping_mul(31).wrapping_add(*byte as u64);
        }
        // Mix bits
        h ^= h >> 33;
        h = h.wrapping_mul(0xFF51AFD7ED558CCD);
        h ^= h >> 33;
        (h as usize) % self.width
    }
}

// ── HyperLogLog ─────────────────────────────────────────────────────────

/// A HyperLogLog counter for cardinality estimation.
///
/// Memory: 2^precision bytes (register array).
/// Default (p=12): 4096 registers = 4 KB.
#[derive(Clone, Debug)]
pub struct HyperLogLog {
    precision: u8,
    registers: Vec<u8>,
}

impl HyperLogLog {
    /// Create a new HLL with the given precision (4 ≤ p ≤ 18).
    pub fn new(precision: u8) -> Self {
        let p = precision.clamp(4, 18);
        let m = 1usize << p;
        Self {
            precision: p,
            registers: vec![0u8; m],
        }
    }

    /// Create an HLL with default precision (p=12, 4KB).
    pub fn default_precision() -> Self {
        Self::new(DEFAULT_HLL_PRECISION)
    }

    /// Insert an item (hashed via BLAKE3).
    pub fn insert(&mut self, item: &str) {
        let hash = blake3::hash(item.as_bytes());
        let hash_bytes = hash.as_bytes();
        // Use first 8 bytes as u64
        let h = u64::from_le_bytes([
            hash_bytes[0],
            hash_bytes[1],
            hash_bytes[2],
            hash_bytes[3],
            hash_bytes[4],
            hash_bytes[5],
            hash_bytes[6],
            hash_bytes[7],
        ]);
        self.insert_hash(h);
    }

    /// Insert a pre-computed hash value.
    pub fn insert_hash(&mut self, hash: u64) {
        let m = self.registers.len();
        let idx = (hash as usize) & (m - 1);
        let remaining = hash >> self.precision;
        // Count leading zeros + 1 (rho function)
        let rho = if remaining == 0 {
            (64 - self.precision) as u8
        } else {
            (remaining.leading_zeros() as u8) + 1
        };
        self.registers[idx] = self.registers[idx].max(rho);
    }

    /// Estimate the cardinality (number of distinct items).
    pub fn estimate_cardinality(&self) -> f64 {
        let m = self.registers.len() as f64;

        // Harmonic mean of 2^(-register)
        let raw_estimate = {
            let sum: f64 = self
                .registers
                .iter()
                .map(|&r| 2.0_f64.powi(-(r as i32)))
                .sum();
            let alpha = self.alpha_m();
            alpha * m * m / sum
        };

        // Small-range correction
        if raw_estimate <= 2.5 * m {
            let zeros = self.registers.iter().filter(|&&r| r == 0).count() as f64;
            if zeros > 0.0 {
                return m * (m / zeros).ln();
            }
        }

        // Large-range correction
        let two_32 = 2.0_f64.powi(32);
        if raw_estimate > two_32 / 30.0 {
            return -two_32 * (1.0 - raw_estimate / two_32).ln();
        }

        raw_estimate
    }

    /// Number of distinct items inserted (rounded).
    pub fn count(&self) -> u64 {
        self.estimate_cardinality().round() as u64
    }

    /// Estimated memory usage in bytes.
    pub fn memory_bytes(&self) -> usize {
        self.registers.len() + std::mem::size_of::<Self>()
    }

    /// Reset all registers.
    pub fn reset(&mut self) {
        for r in &mut self.registers {
            *r = 0;
        }
    }

    /// Alpha_m constant for bias correction.
    fn alpha_m(&self) -> f64 {
        let m = self.registers.len();
        match m {
            16 => 0.673,
            32 => 0.697,
            64 => 0.709,
            _ => 0.7213 / (1.0 + 1.079 / m as f64),
        }
    }
}

// ── Usage Analytics ─────────────────────────────────────────────────────

/// High-level usage analytics aggregating CMS + HLL.
///
/// Tracks:
/// - Operation frequency via Count-Min Sketch
/// - Unique entity cardinality via HyperLogLog
#[derive(Clone, Debug)]
pub struct UsageAnalytics {
    /// Operation frequency tracker.
    pub operation_frequency: CountMinSketch,
    /// Unique worldline cardinality estimator.
    pub unique_worldlines: HyperLogLog,
    /// Unique commitment cardinality estimator.
    pub unique_commitments: HyperLogLog,
    /// Unique event-type cardinality estimator.
    pub unique_event_types: HyperLogLog,
}

impl UsageAnalytics {
    /// Create analytics with default-sized structures.
    pub fn new() -> Self {
        Self {
            operation_frequency: CountMinSketch::default_size(),
            unique_worldlines: HyperLogLog::default_precision(),
            unique_commitments: HyperLogLog::default_precision(),
            unique_event_types: HyperLogLog::default_precision(),
        }
    }

    /// Record an operation occurrence.
    pub fn record_operation(&mut self, operation: &str) {
        self.operation_frequency.increment(operation);
    }

    /// Record a worldline observation.
    pub fn record_worldline(&mut self, worldline_id: &str) {
        self.unique_worldlines.insert(worldline_id);
    }

    /// Record a commitment observation.
    pub fn record_commitment(&mut self, commitment_id: &str) {
        self.unique_commitments.insert(commitment_id);
    }

    /// Record an event-type observation.
    pub fn record_event_type(&mut self, event_type: &str) {
        self.unique_event_types.insert(event_type);
    }

    /// Get a snapshot of current analytics state.
    pub fn snapshot(&self) -> UsageAnalyticsSnapshot {
        UsageAnalyticsSnapshot {
            total_operations: self.operation_frequency.total(),
            estimated_unique_worldlines: self.unique_worldlines.count(),
            estimated_unique_commitments: self.unique_commitments.count(),
            estimated_unique_event_types: self.unique_event_types.count(),
        }
    }

    /// Estimated memory usage in bytes.
    pub fn memory_bytes(&self) -> usize {
        self.operation_frequency.memory_bytes()
            + self.unique_worldlines.memory_bytes()
            + self.unique_commitments.memory_bytes()
            + self.unique_event_types.memory_bytes()
    }

    /// Reset all analytics state.
    pub fn reset(&mut self) {
        self.operation_frequency.reset();
        self.unique_worldlines.reset();
        self.unique_commitments.reset();
        self.unique_event_types.reset();
    }
}

impl Default for UsageAnalytics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of usage analytics at a point in time.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UsageAnalyticsSnapshot {
    /// Total recorded operations.
    pub total_operations: u64,
    /// Estimated number of unique worldlines observed.
    pub estimated_unique_worldlines: u64,
    /// Estimated number of unique commitments observed.
    pub estimated_unique_commitments: u64,
    /// Estimated number of unique event types observed.
    pub estimated_unique_event_types: u64,
}

/// Top-N operation frequencies from a CMS.
pub fn top_operations(cms: &CountMinSketch, known_keys: &[String], n: usize) -> Vec<(String, u64)> {
    let mut entries: Vec<(String, u64)> = known_keys
        .iter()
        .map(|k| (k.clone(), cms.estimate(k)))
        .collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1));
    entries.truncate(n);
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Count-Min Sketch tests ──────────────────────────────────────

    #[test]
    fn cms_basic_counting() {
        let mut cms = CountMinSketch::new(256, 4);
        cms.increment("hello");
        cms.increment("hello");
        cms.increment("world");

        // CMS guarantees estimate >= true count
        assert!(cms.estimate("hello") >= 2);
        assert!(cms.estimate("world") >= 1);
        assert_eq!(cms.total(), 3);
    }

    #[test]
    fn cms_never_underestimates() {
        let mut cms = CountMinSketch::default_size();
        for _ in 0..1000 {
            cms.increment("target");
        }
        // Must be >= 1000 (the true count)
        assert!(cms.estimate("target") >= 1000);
    }

    #[test]
    fn cms_unseen_keys_have_zero_or_low_estimate() {
        let mut cms = CountMinSketch::default_size();
        // Insert some noise
        for i in 0..100 {
            cms.increment(&format!("key_{}", i));
        }
        // An unseen key should have a low estimate
        // (may not be exactly 0 due to hash collisions, but should be small)
        let estimate = cms.estimate("never_inserted");
        assert!(estimate <= 10, "unseen key estimate unexpectedly high: {}", estimate);
    }

    #[test]
    fn cms_increment_by() {
        let mut cms = CountMinSketch::new(256, 4);
        cms.increment_by("batch", 100);
        assert!(cms.estimate("batch") >= 100);
        assert_eq!(cms.total(), 100);
    }

    #[test]
    fn cms_reset() {
        let mut cms = CountMinSketch::new(64, 2);
        cms.increment("key");
        cms.reset();
        assert_eq!(cms.estimate("key"), 0);
        assert_eq!(cms.total(), 0);
    }

    #[test]
    fn cms_memory_bounded() {
        let cms = CountMinSketch::default_size();
        // 1024 * 4 * 8 = 32768 bytes + overhead
        assert!(cms.memory_bytes() < 40_000);
    }

    // ── HyperLogLog tests ───────────────────────────────────────────

    #[test]
    fn hll_empty_count_is_zero() {
        let hll = HyperLogLog::default_precision();
        assert_eq!(hll.count(), 0);
    }

    #[test]
    fn hll_single_insert() {
        let mut hll = HyperLogLog::default_precision();
        hll.insert("item1");
        assert!(hll.count() >= 1);
    }

    #[test]
    fn hll_duplicate_handling() {
        let mut hll = HyperLogLog::default_precision();
        for _ in 0..1000 {
            hll.insert("same_item");
        }
        // Duplicates should not inflate the cardinality
        assert!(hll.count() <= 5, "duplicate inflation: count={}", hll.count());
    }

    #[test]
    fn hll_cardinality_estimation() {
        let mut hll = HyperLogLog::new(14); // higher precision for better accuracy
        let n = 10_000;
        for i in 0..n {
            hll.insert(&format!("unique_item_{}", i));
        }
        let estimated = hll.count();
        // HLL with p=14 should be within ~5% for 10k items
        let lower = (n as f64 * 0.85) as u64;
        let upper = (n as f64 * 1.15) as u64;
        assert!(
            estimated >= lower && estimated <= upper,
            "HLL estimate {} out of range [{}, {}] for n={}",
            estimated, lower, upper, n
        );
    }

    #[test]
    fn hll_reset() {
        let mut hll = HyperLogLog::default_precision();
        hll.insert("item");
        hll.reset();
        assert_eq!(hll.count(), 0);
    }

    #[test]
    fn hll_memory_bounded() {
        let hll = HyperLogLog::default_precision();
        // 2^12 = 4096 registers + overhead
        assert!(hll.memory_bytes() < 5_000);
    }

    #[test]
    fn hll_precision_clamping() {
        let low = HyperLogLog::new(2);
        assert_eq!(low.registers.len(), 1 << 4); // clamped to 4

        let high = HyperLogLog::new(20);
        assert_eq!(high.registers.len(), 1 << 18); // clamped to 18
    }

    // ── Usage Analytics tests ───────────────────────────────────────

    #[test]
    fn analytics_basic_recording() {
        let mut analytics = UsageAnalytics::new();

        analytics.record_operation("fabric.emit");
        analytics.record_operation("fabric.emit");
        analytics.record_operation("gate.submit");
        analytics.record_worldline("wl-001");
        analytics.record_worldline("wl-002");
        analytics.record_commitment("c-001");
        analytics.record_event_type("FabricEventEmitted");

        let snap = analytics.snapshot();
        assert_eq!(snap.total_operations, 3);
        assert!(snap.estimated_unique_worldlines >= 1);
        assert!(snap.estimated_unique_commitments >= 1);
        assert!(snap.estimated_unique_event_types >= 1);
    }

    #[test]
    fn analytics_memory_budget() {
        let analytics = UsageAnalytics::new();
        // CMS (32KB) + 3 HLL (4KB each) = ~44KB + overhead
        assert!(analytics.memory_bytes() < 60_000);
    }

    #[test]
    fn analytics_reset() {
        let mut analytics = UsageAnalytics::new();
        analytics.record_operation("test");
        analytics.record_worldline("wl-1");
        analytics.reset();

        let snap = analytics.snapshot();
        assert_eq!(snap.total_operations, 0);
    }

    #[test]
    fn analytics_snapshot_serializes() {
        let analytics = UsageAnalytics::new();
        let snap = analytics.snapshot();
        let json = serde_json::to_string(&snap).unwrap();
        let _: UsageAnalyticsSnapshot = serde_json::from_str(&json).unwrap();
    }

    // ── Top operations ──────────────────────────────────────────────

    #[test]
    fn top_operations_ranking() {
        let mut cms = CountMinSketch::default_size();
        cms.increment_by("emit", 100);
        cms.increment_by("route", 50);
        cms.increment_by("check", 25);

        let keys = vec!["emit".into(), "route".into(), "check".into()];
        let top = top_operations(&cms, &keys, 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "emit");
        assert_eq!(top[1].0, "route");
    }
}
