//! Application state for API handlers

use crate::scheduler::Scheduler;
use crate::storage::InMemoryStorage;
use std::sync::Arc;
use tokio::sync::broadcast;
use palm_types::PalmEventEnvelope;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Storage backend
    pub storage: Arc<InMemoryStorage>,

    /// Scheduler handle
    pub scheduler: Arc<Scheduler>,

    /// Event broadcast channel
    pub event_tx: broadcast::Sender<PalmEventEnvelope>,

    /// Daemon version
    pub version: String,

    /// Daemon start time
    pub started_at: chrono::DateTime<chrono::Utc>,
}

impl AppState {
    /// Create new application state
    pub fn new(
        storage: Arc<InMemoryStorage>,
        scheduler: Arc<Scheduler>,
        event_tx: broadcast::Sender<PalmEventEnvelope>,
    ) -> Self {
        Self {
            storage,
            scheduler,
            event_tx,
            version: env!("CARGO_PKG_VERSION").to_string(),
            started_at: chrono::Utc::now(),
        }
    }

    /// Get uptime as a human-readable string
    pub fn uptime(&self) -> String {
        let duration = chrono::Utc::now() - self.started_at;
        let secs = duration.num_seconds();

        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else if secs < 86400 {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        } else {
            format!("{}d {}h", secs / 86400, (secs % 86400) / 3600)
        }
    }
}
