//! Server setup and lifecycle management

use crate::api::create_router;
use crate::api::rest::state::AppState;
use crate::config::DaemonConfig;
use crate::error::DaemonResult;
use crate::scheduler::Scheduler;
use crate::storage::InMemoryStorage;
use palm_types::PalmEventEnvelope;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

/// PALM Daemon Server
pub struct Server {
    config: DaemonConfig,
    storage: Arc<InMemoryStorage>,
    scheduler: Arc<Scheduler>,
    event_tx: broadcast::Sender<PalmEventEnvelope>,
}

impl Server {
    /// Create a new server with the given configuration
    pub fn new(config: DaemonConfig) -> DaemonResult<Self> {
        // Create storage
        let storage = Arc::new(InMemoryStorage::new());

        // Create event channel
        let (event_tx, _) = broadcast::channel(1000);

        // Create scheduler
        let (scheduler, _reconcile_rx) = Scheduler::new(
            config.scheduler.clone(),
            storage.clone(),
            event_tx.clone(),
        );

        Ok(Self {
            config,
            storage,
            scheduler,
            event_tx,
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
        );

        // Create router
        let app = create_router(state);

        // Create listener
        let listener = TcpListener::bind(addr).await?;

        tracing::info!("PALM daemon listening on {}", addr);
        tracing::info!("Platform profile: {:?}", self.config.platform);

        // Start scheduler in background
        let scheduler = self.scheduler.clone();
        let (_, reconcile_rx) = Scheduler::new(
            self.config.scheduler.clone(),
            self.storage.clone(),
            self.event_tx.clone(),
        );

        tokio::spawn(async move {
            scheduler.start(reconcile_rx).await;
        });

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
