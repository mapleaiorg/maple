use thiserror::Error;

/// Errors from Event Fabric operations.
#[derive(Error, Debug)]
pub enum FabricError {
    #[error("WAL I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("WAL corruption at offset {offset}: {reason}")]
    Corruption { offset: u64, reason: String },

    #[error("WAL segment not found: {0}")]
    SegmentNotFound(u64),

    #[error("event integrity verification failed for event {event_id}")]
    IntegrityFailure { event_id: String },

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("HLC clock drift exceeded maximum ({drift_ms}ms > {max_ms}ms)")]
    ClockDrift { drift_ms: u64, max_ms: u64 },

    #[error("event not found: {0}")]
    EventNotFound(String),

    #[error("WAL is closed")]
    Closed,

    #[error("subscriber channel closed")]
    SubscriberClosed,

    #[error("handler error: {0}")]
    Handler(String),
}

impl From<serde_json::Error> for FabricError {
    fn from(e: serde_json::Error) -> Self {
        FabricError::Serialization(e.to_string())
    }
}
