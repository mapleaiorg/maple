//! Playground service for shared state and simulation control

use super::simulation::SimulationEngine;
use crate::storage::Storage;
use crate::error::StorageError;
use palm_shared_state::{
    AiBackendKind, AiBackendPublic, PlaygroundConfig, PlaygroundConfigPublic,
    PlaygroundConfigUpdate,
};
use palm_shared_state::Activity;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tokio::sync::{broadcast, RwLock};

const DEFAULT_OPENAI_MODEL: &str = "gpt-4o-mini";
const DEFAULT_ANTHROPIC_MODEL: &str = "claude-3-5-sonnet";

/// Playground service
pub struct PlaygroundService {
    storage: Arc<dyn Storage>,
    activity_tx: broadcast::Sender<Activity>,
    config: Arc<RwLock<PlaygroundConfig>>,
    simulation_started: AtomicBool,
}

impl PlaygroundService {
    pub async fn new(
        storage: Arc<dyn Storage>,
        activity_tx: broadcast::Sender<Activity>,
    ) -> Result<Arc<Self>, StorageError> {
        let config = match storage.get_playground_config().await? {
            Some(config) => config,
            None => {
                let config = PlaygroundConfig::default();
                storage.upsert_playground_config(config.clone()).await?;
                config
            }
        };

        Ok(Arc::new(Self {
            storage,
            activity_tx,
            config: Arc::new(RwLock::new(config)),
            simulation_started: AtomicBool::new(false),
        }))
    }

    pub async fn start(&self) {
        if self
            .simulation_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }

        let engine = SimulationEngine::new(
            self.storage.clone(),
            self.activity_tx.clone(),
            self.config.clone(),
        );

        tokio::spawn(async move {
            engine.run().await;
        });
    }

    pub async fn config_public(&self) -> PlaygroundConfigPublic {
        let config = self.config.read().await.clone();
        config.public_view()
    }

    pub async fn update_config(
        &self,
        update: PlaygroundConfigUpdate,
    ) -> Result<PlaygroundConfigPublic, StorageError> {
        let current = self.config.read().await.clone();
        let updated = update.apply(current);
        self.storage.upsert_playground_config(updated.clone()).await?;
        let mut guard = self.config.write().await;
        *guard = updated.clone();
        Ok(updated.public_view())
    }

    pub async fn backend_catalog(&self) -> Vec<AiBackendPublic> {
        let config = self.config.read().await;
        let active_kind = config.ai_backend.kind;
        let mut list = Vec::new();

        list.push(match active_kind {
            AiBackendKind::LocalLlama => config.ai_backend.to_public(),
            _ => AiBackendPublic {
                kind: AiBackendKind::LocalLlama,
                model: "llama3".to_string(),
                endpoint: Some("http://127.0.0.1:11434".to_string()),
                active: false,
                configured: false,
            },
        });

        list.push(match active_kind {
            AiBackendKind::OpenAI => config.ai_backend.to_public(),
            _ => AiBackendPublic {
                kind: AiBackendKind::OpenAI,
                model: DEFAULT_OPENAI_MODEL.to_string(),
                endpoint: None,
                active: false,
                configured: false,
            },
        });

        list.push(match active_kind {
            AiBackendKind::Anthropic => config.ai_backend.to_public(),
            _ => AiBackendPublic {
                kind: AiBackendKind::Anthropic,
                model: DEFAULT_ANTHROPIC_MODEL.to_string(),
                endpoint: None,
                active: false,
                configured: false,
            },
        });

        list
    }
}
