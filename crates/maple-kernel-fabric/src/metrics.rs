use serde::{Deserialize, Serialize};

/// Metrics snapshot from the Event Fabric.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FabricMetrics {
    pub events_total: u64,
    pub wal_size_bytes: u64,
    pub wal_segments: u32,
    pub latest_sequence: u64,
    pub subscribers_active: u32,
}
