use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::error::FabricError;
use crate::types::NodeId;

/// Hybrid Logical Clock for causal ordering without global clock.
///
/// Combines wall-clock time with a logical counter to guarantee:
/// - If event A happens-before event B, then `hlc(A) < hlc(B)`
/// - HLC timestamps are monotonically increasing per node
/// - HLC stays close to wall-clock time
///
/// Constitutional Invariant I.EF-1 (Causal Ordering):
/// HLC preserves causal precedence.
pub struct HybridLogicalClock {
    /// Physical time component (milliseconds since epoch)
    physical: AtomicU64,
    /// Logical counter component
    logical: AtomicU32,
    /// Node identifier for distributed disambiguation
    node_id: NodeId,
    /// Maximum allowed drift from wall-clock (ms)
    max_drift_ms: u64,
}

/// HLC timestamp — the causal ordering primitive.
///
/// Totally ordered: physical → logical → node_id.
/// Serializes to 14 bytes: 8 (physical) + 4 (logical) + 2 (node_id).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HlcTimestamp {
    pub physical: u64,
    pub logical: u32,
    pub node_id: NodeId,
}

impl PartialOrd for HlcTimestamp {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HlcTimestamp {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.physical
            .cmp(&other.physical)
            .then(self.logical.cmp(&other.logical))
            .then(self.node_id.0.cmp(&other.node_id.0))
    }
}

impl std::fmt::Display for HlcTimestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.physical, self.logical, self.node_id.0)
    }
}

fn wall_clock_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_millis() as u64
}

impl HybridLogicalClock {
    /// Create a new HLC with the given node ID.
    /// Default max drift: 1000ms (1 second).
    pub fn new(node_id: NodeId) -> Self {
        Self::with_max_drift(node_id, 1000)
    }

    /// Create with a custom max drift threshold.
    pub fn with_max_drift(node_id: NodeId, max_drift_ms: u64) -> Self {
        let now = wall_clock_ms();
        Self {
            physical: AtomicU64::new(now),
            logical: AtomicU32::new(0),
            node_id,
            max_drift_ms,
        }
    }

    /// Generate a new timestamp. Guarantees monotonic increase.
    ///
    /// Lock-free implementation using CAS loops on atomics.
    pub fn now(&self) -> HlcTimestamp {
        loop {
            let wall = wall_clock_ms();
            let prev_physical = self.physical.load(Ordering::Acquire);
            let prev_logical = self.logical.load(Ordering::Acquire);

            if wall > prev_physical {
                // Wall clock advanced — reset logical counter
                if self
                    .physical
                    .compare_exchange(prev_physical, wall, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    self.logical.store(0, Ordering::Release);
                    return HlcTimestamp {
                        physical: wall,
                        logical: 0,
                        node_id: self.node_id.clone(),
                    };
                }
                // CAS failed — another thread updated; retry
            } else {
                // Wall clock hasn't advanced — increment logical counter
                let new_logical = prev_logical + 1;
                if self
                    .logical
                    .compare_exchange(
                        prev_logical,
                        new_logical,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_ok()
                {
                    return HlcTimestamp {
                        physical: prev_physical,
                        logical: new_logical,
                        node_id: self.node_id.clone(),
                    };
                }
                // CAS failed — retry
            }
        }
    }

    /// Update clock on receiving a remote timestamp.
    ///
    /// Ensures causal ordering: our next timestamp will be > remote.
    /// Returns the new local timestamp after merge.
    pub fn receive(&self, remote: HlcTimestamp) -> Result<HlcTimestamp, FabricError> {
        let wall = wall_clock_ms();

        // Check for excessive drift
        if remote.physical > wall + self.max_drift_ms {
            return Err(FabricError::ClockDrift {
                drift_ms: remote.physical - wall,
                max_ms: self.max_drift_ms,
            });
        }

        loop {
            let prev_physical = self.physical.load(Ordering::Acquire);
            let prev_logical = self.logical.load(Ordering::Acquire);

            let new_physical = wall.max(prev_physical).max(remote.physical);

            let new_logical = if new_physical == prev_physical && new_physical == remote.physical {
                // All three equal — take max logical + 1
                prev_logical.max(remote.logical) + 1
            } else if new_physical == prev_physical {
                // Local physical is highest — increment local logical
                prev_logical + 1
            } else if new_physical == remote.physical {
                // Remote physical is highest — increment remote logical
                remote.logical + 1
            } else {
                // Wall clock is highest — reset
                0
            };

            if self
                .physical
                .compare_exchange(
                    prev_physical,
                    new_physical,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                self.logical.store(new_logical, Ordering::Release);
                return Ok(HlcTimestamp {
                    physical: new_physical,
                    logical: new_logical,
                    node_id: self.node_id.clone(),
                });
            }
            // CAS failed — retry
        }
    }

    /// Check causal precedence: does `a` happen-before `b`?
    pub fn precedes(a: &HlcTimestamp, b: &HlcTimestamp) -> bool {
        a < b
    }

    /// Access the node ID.
    pub fn node_id(&self) -> NodeId {
        self.node_id.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monotonically_increasing() {
        let hlc = HybridLogicalClock::new(NodeId(1));
        let mut prev = hlc.now();
        for _ in 0..1000 {
            let ts = hlc.now();
            assert!(ts > prev, "HLC must be monotonically increasing: {:?} should be > {:?}", ts, prev);
            prev = ts;
        }
    }

    #[test]
    fn concurrent_monotonicity() {
        use std::sync::Arc;
        let hlc = Arc::new(HybridLogicalClock::new(NodeId(1)));
        let mut handles = vec![];

        for _ in 0..4 {
            let hlc = hlc.clone();
            handles.push(std::thread::spawn(move || {
                let mut timestamps = Vec::with_capacity(1000);
                for _ in 0..1000 {
                    timestamps.push(hlc.now());
                }
                timestamps
            }));
        }

        let mut all_timestamps: Vec<HlcTimestamp> = vec![];
        for h in handles {
            all_timestamps.extend(h.join().unwrap());
        }

        // All timestamps should be unique (since same node, they should be distinct)
        let count = all_timestamps.len();
        all_timestamps.sort();
        all_timestamps.dedup();
        assert_eq!(all_timestamps.len(), count, "All timestamps should be unique");
    }

    #[test]
    fn receive_advances_clock() {
        let hlc = HybridLogicalClock::new(NodeId(1));
        let local = hlc.now();

        // Simulate a remote timestamp far in the future (within drift)
        let remote = HlcTimestamp {
            physical: local.physical + 500,
            logical: 10,
            node_id: NodeId(2),
        };

        let after = hlc.receive(remote.clone()).unwrap();
        assert!(after > remote, "After receive, local must be > remote");
        assert!(after > local, "After receive, local must be > previous local");
    }

    #[test]
    fn receive_rejects_excessive_drift() {
        let hlc = HybridLogicalClock::with_max_drift(NodeId(1), 100);
        let remote = HlcTimestamp {
            physical: wall_clock_ms() + 5000, // 5 seconds in future, max is 100ms
            logical: 0,
            node_id: NodeId(2),
        };
        assert!(hlc.receive(remote).is_err());
    }

    #[test]
    fn causal_ordering_guarantee() {
        let hlc_a = HybridLogicalClock::new(NodeId(1));
        let hlc_b = HybridLogicalClock::new(NodeId(2));

        // A generates event
        let ts_a = hlc_a.now();

        // B receives A's event, then generates its own
        hlc_b.receive(ts_a.clone()).unwrap();
        let ts_b = hlc_b.now();

        // B's event must be causally after A's
        assert!(
            HybridLogicalClock::precedes(&ts_a, &ts_b),
            "ts_a={:?} should precede ts_b={:?}",
            ts_a,
            ts_b
        );
    }

    #[test]
    fn total_ordering() {
        let ts1 = HlcTimestamp { physical: 100, logical: 0, node_id: NodeId(1) };
        let ts2 = HlcTimestamp { physical: 100, logical: 1, node_id: NodeId(1) };
        let ts3 = HlcTimestamp { physical: 100, logical: 1, node_id: NodeId(2) };
        let ts4 = HlcTimestamp { physical: 101, logical: 0, node_id: NodeId(0) };

        assert!(ts1 < ts2);
        assert!(ts2 < ts3);
        assert!(ts3 < ts4);
    }

    #[test]
    fn serialization_roundtrip() {
        let ts = HlcTimestamp {
            physical: 1234567890,
            logical: 42,
            node_id: NodeId(7),
        };
        let json = serde_json::to_string(&ts).unwrap();
        let restored: HlcTimestamp = serde_json::from_str(&json).unwrap();
        assert_eq!(ts, restored);
    }

    #[test]
    fn display_format() {
        let ts = HlcTimestamp {
            physical: 1000,
            logical: 5,
            node_id: NodeId(3),
        };
        assert_eq!(format!("{}", ts), "1000:5:3");
    }
}
