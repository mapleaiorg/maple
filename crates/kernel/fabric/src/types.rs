use serde::{Deserialize, Serialize};

// Re-export types from maple-mwl-types that the fabric uses.
pub use maple_mwl_types::{CommitmentId, EventId, NodeId, WorldlineId};

/// Subscription identifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubscriptionId(pub uuid::Uuid);

impl SubscriptionId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for SubscriptionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Coupling identifier for presence/coupling events.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CouplingId(pub uuid::Uuid);

impl CouplingId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for CouplingId {
    fn default() -> Self {
        Self::new()
    }
}

/// Coupling scope for coupling events.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CouplingScope {
    pub domains: Vec<String>,
    pub constraints: Vec<String>,
}

/// BLAKE3 hash wrapper for event integrity.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for b in &self.0[..8] {
            write!(f, "{:02x}", b)?;
        }
        write!(f, "...")
    }
}

/// WAL integrity report.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub total_events: u64,
    pub verified_events: u64,
    pub corrupted_events: u64,
    pub corrupted_offsets: Vec<u64>,
    pub segments_checked: u32,
}

impl IntegrityReport {
    pub fn is_clean(&self) -> bool {
        self.corrupted_events == 0
    }
}
