use serde::{Deserialize, Serialize};

/// Temporal anchor — local event ordering, no global clock assumed.
/// Based on Hybrid Logical Clocks (HLC).
///
/// Per Resonance Architecture v1.1 §3.3:
/// "Time is the ordering of resonance events as perceived and remembered by a Resonator."
/// - Different Resonators may experience time differently
/// - Simultaneity is not required
/// - Causality is inferred, not enforced
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TemporalAnchor {
    /// Physical time component (milliseconds since Unix epoch)
    pub physical_ms: u64,
    /// Logical counter for events at same physical time
    pub logical: u32,
    /// Node identifier for distributed disambiguation
    pub node_id: u16,
}

impl TemporalAnchor {
    pub fn new(physical_ms: u64, logical: u32, node_id: u16) -> Self {
        Self {
            physical_ms,
            logical,
            node_id,
        }
    }

    /// Create an anchor representing "now" on this node.
    pub fn now(node_id: u16) -> Self {
        let physical_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before Unix epoch")
            .as_millis() as u64;
        Self {
            physical_ms,
            logical: 0,
            node_id,
        }
    }

    /// Zero anchor (before all events).
    pub fn genesis() -> Self {
        Self {
            physical_ms: 0,
            logical: 0,
            node_id: 0,
        }
    }

    /// Check causal precedence: does self happen-before other?
    pub fn precedes(&self, other: &TemporalAnchor) -> bool {
        self < other
    }
}

impl PartialOrd for TemporalAnchor {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TemporalAnchor {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.physical_ms
            .cmp(&other.physical_ms)
            .then(self.logical.cmp(&other.logical))
            .then(self.node_id.cmp(&other.node_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_physical_time() {
        let a = TemporalAnchor::new(100, 0, 0);
        let b = TemporalAnchor::new(101, 0, 0);
        assert!(a < b);
        assert!(a.precedes(&b));
    }

    #[test]
    fn ordering_logical_counter() {
        let a = TemporalAnchor::new(100, 0, 0);
        let b = TemporalAnchor::new(100, 1, 0);
        assert!(a < b);
        assert!(a.precedes(&b));
    }

    #[test]
    fn ordering_node_id() {
        let a = TemporalAnchor::new(100, 0, 0);
        let b = TemporalAnchor::new(100, 0, 1);
        assert!(a < b);
    }

    #[test]
    fn ordering_composite() {
        let a = TemporalAnchor::new(100, 0, 0);
        let b = TemporalAnchor::new(100, 1, 0);
        let c = TemporalAnchor::new(101, 0, 0);
        assert!(a < b);
        assert!(b < c);
        assert!(a < c);
    }

    #[test]
    fn genesis_precedes_all() {
        let genesis = TemporalAnchor::genesis();
        let any = TemporalAnchor::new(1, 0, 0);
        assert!(genesis.precedes(&any));
    }

    #[test]
    fn precedes_consistent_with_ord() {
        let a = TemporalAnchor::new(50, 3, 1);
        let b = TemporalAnchor::new(50, 3, 2);
        assert_eq!(a.precedes(&b), a < b);
        assert!(!b.precedes(&a));
    }

    #[test]
    fn equal_anchors_dont_precede() {
        let a = TemporalAnchor::new(100, 1, 5);
        let b = TemporalAnchor::new(100, 1, 5);
        assert!(!a.precedes(&b));
        assert!(!b.precedes(&a));
    }

    #[test]
    fn serialization_roundtrip() {
        let anchor = TemporalAnchor::new(1234567890, 42, 7);
        let json = serde_json::to_string(&anchor).unwrap();
        let restored: TemporalAnchor = serde_json::from_str(&json).unwrap();
        assert_eq!(anchor, restored);
    }

    #[test]
    fn now_produces_nonzero_physical_time() {
        let anchor = TemporalAnchor::now(1);
        assert!(anchor.physical_ms > 0);
        assert_eq!(anchor.node_id, 1);
    }
}
