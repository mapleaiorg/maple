//! Server setup and lifecycle management

use crate::api::create_router;
use crate::api::rest::state::AppState;
use crate::config::{DaemonConfig, StorageConfig};
use crate::error::{DaemonError, DaemonResult};
use crate::playground::PlaygroundService;
use crate::scheduler::Scheduler;
use crate::storage::{InMemoryStorage, PostgresStorage, Storage};
use palm_types::PalmEventEnvelope;
use palm_shared_state::Activity;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc};

/// PALM Daemon Server
pub struct Server {
    config: DaemonConfig,
    storage: Arc<dyn Storage>,
    scheduler: Arc<Scheduler>,
    event_tx: broadcast::Sender<PalmEventEnvelope>,
    activity_tx: broadcast::Sender<Activity>,
    reconcile_rx: Option<mpsc::Receiver<()>>,
    playground: Arc<PlaygroundService>,
}

impl Server {
    /// Create a new server with the given configuration
    pub async fn new(config: DaemonConfig) -> DaemonResult<Self> {
        // Create storage
        let storage: Arc<dyn Storage> = match &config.storage {
            StorageConfig::Memory => Arc::new(InMemoryStorage::new()),
            StorageConfig::Postgres { url, max_connections, connect_timeout_secs } => {
                let pg = PostgresStorage::new(url, *max_connections, *connect_timeout_secs)
                    .await
                    .map_err(DaemonError::Storage)?;
                Arc::new(pg)
            }
        };

        // Create event + activity channels
        let (event_tx, _) = broadcast::channel(1000);
        let (activity_tx, _) = broadcast::channel(1000);

        // Create scheduler
        let (scheduler, reconcile_rx) = Scheduler::with_platform(
            config.scheduler.clone(),
            storage.clone(),
            event_tx.clone(),
            activity_tx.clone(),
            config.platform.clone(),
        );

        // Create playground service
        let playground = PlaygroundService::new(storage.clone(), activity_tx.clone()).await?;

        Ok(Self {
            config,
            storage,
            scheduler,
            event_tx,
            activity_tx,
            reconcile_rx: Some(reconcile_rx),
            playground,
        })
    }

    /// Run the server
    pub async fn run(self) -> DaemonResult<()> {
        let addr = self.config.server.listen_addr;

        // Create app state
        let state = AppState::new(
            self.storage.clone(),
            self.scheduler.clone(),
            self.event_tx.clone(),
            self.activity_tx.clone(),
            self.playground.clone(),
        );

        // Create router
        let app = create_router(state);

        // Create listener
        let listener = TcpListener::bind(addr).await?;

        tracing::info!("PALM daemon listening on {}", addr);
        tracing::info!("Platform profile: {:?}", self.config.platform);

        // Start scheduler in background
        let scheduler = self.scheduler.clone();
        if let Some(reconcile_rx) = self.reconcile_rx {
            tokio::spawn(async move {
                scheduler.start(reconcile_rx).await;
            });
        } else {
            tracing::warn!("Scheduler already running or reconcile channel missing");
        }

        // Start playground simulation
        self.playground.start().await;

        // Run server with graceful shutdown
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(|e| crate::error::DaemonError::Server(e.to_string()))?;

        tracing::info!("PALM daemon shutting down");

        // Stop scheduler
        self.scheduler.stop().await;

        Ok(())
    }
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown");
        }
        _ = terminate => {
            tracing::info!("Received terminate signal, initiating graceful shutdown");
        }
    }
}
