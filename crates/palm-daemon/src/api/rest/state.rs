//! Application state for API handlers

use crate::playground::PlaygroundService;
use crate::scheduler::Scheduler;
use crate::storage::Storage;
use palm_shared_state::Activity;
use palm_types::PalmEventEnvelope;
use std::sync::Arc;
use tokio::sync::{broadcast, watch};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Storage backend
    pub storage: Arc<dyn Storage>,

    /// Scheduler handle
    pub scheduler: Arc<Scheduler>,

    /// Event broadcast channel
    pub event_tx: broadcast::Sender<PalmEventEnvelope>,

    /// Activity broadcast channel
    pub activity_tx: broadcast::Sender<Activity>,

    /// Playground service
    pub playground: Arc<PlaygroundService>,

    /// Daemon version
    pub version: String,

    /// Daemon start time
    pub started_at: chrono::DateTime<chrono::Utc>,

    /// Graceful shutdown signal sender
    pub shutdown_tx: watch::Sender<bool>,
}

impl AppState {
    /// Create new application state
    pub fn new(
        storage: Arc<dyn Storage>,
        scheduler: Arc<Scheduler>,
        event_tx: broadcast::Sender<PalmEventEnvelope>,
        activity_tx: broadcast::Sender<Activity>,
        playground: Arc<PlaygroundService>,
        shutdown_tx: watch::Sender<bool>,
    ) -> Self {
        Self {
            storage,
            scheduler,
            event_tx,
            activity_tx,
            playground,
            version: env!("CARGO_PKG_VERSION").to_string(),
            started_at: chrono::Utc::now(),
            shutdown_tx,
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
