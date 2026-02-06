//! Playground service for shared state and simulation control

use super::llm;
use super::simulation::SimulationEngine;
use crate::error::StorageError;
use crate::storage::Storage;
use palm_shared_state::{
    Activity, ActivityActor, AiBackendKind, AiBackendPublic, PlaygroundConfig,
    PlaygroundConfigPublic, PlaygroundConfigUpdate, PlaygroundInferenceRequest,
    PlaygroundInferenceResponse,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};

const DEFAULT_OPENAI_MODEL: &str = "gpt-4o-mini";
const DEFAULT_ANTHROPIC_MODEL: &str = "claude-3-5-sonnet";
const DEFAULT_GROK_MODEL: &str = "grok-2-latest";
const DEFAULT_GEMINI_MODEL: &str = "gemini-2.0-flash";

#[derive(Debug, Error)]
pub enum PlaygroundServiceError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Inference error: {0}")]
    Inference(String),
}

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
        let current_kind = current.ai_backend.kind;
        let current_model = current.ai_backend.model.clone();
        let updated = update.apply(current);
        let backend_changed =
            current_kind != updated.ai_backend.kind || current_model != updated.ai_backend.model;
        self.storage
            .upsert_playground_config(updated.clone())
            .await?;
        let mut guard = self.config.write().await;
        *guard = updated.clone();

        if backend_changed {
            let backend_kind = updated.ai_backend.kind;
            let backend_model = updated.ai_backend.model.clone();
            let backend_configured = updated.ai_backend.is_configured();
            self.record_activity(Activity::new(
                ActivityActor::System,
                "playground-service",
                "backend_changed",
                format!("Switched backend to {:?} ({})", backend_kind, backend_model),
                serde_json::json!({
                    "kind": format!("{:?}", backend_kind),
                    "model": backend_model,
                    "configured": backend_configured,
                }),
            ))
            .await?;
        }

        Ok(updated.public_view())
    }

    pub async fn infer(
        &self,
        request: PlaygroundInferenceRequest,
    ) -> Result<PlaygroundInferenceResponse, PlaygroundServiceError> {
        request
            .validate()
            .map_err(PlaygroundServiceError::Validation)?;

        let config = self.config.read().await.clone();
        if !config.ai_backend.is_configured() {
            return Err(PlaygroundServiceError::Validation(format!(
                "Active backend {:?} is not configured (missing endpoint/api key)",
                config.ai_backend.kind
            )));
        }

        let result = llm::infer(&config.ai_backend, &request)
            .await
            .map_err(PlaygroundServiceError::Inference);

        match result {
            Ok(response) => {
                let backend_kind = response.backend_kind;
                let backend_model = response.backend_model.clone();
                let usage = response.usage.clone();
                let finish_reason = response.finish_reason.clone();
                let actor_id = request
                    .actor_id
                    .clone()
                    .unwrap_or_else(|| "playground-ui".to_string());
                let actor_type = if request.actor_id.is_some() {
                    ActivityActor::Agent
                } else {
                    ActivityActor::System
                };

                self.record_activity(Activity::new(
                    actor_type,
                    actor_id,
                    "ai_inference",
                    format!(
                        "{:?} generated {} chars",
                        backend_kind,
                        response.output.chars().count()
                    ),
                    serde_json::json!({
                        "backend_kind": format!("{:?}", backend_kind),
                        "backend_model": backend_model,
                        "latency_ms": response.latency_ms,
                        "prompt_chars": request.prompt.chars().count(),
                        "has_system_prompt": request.system_prompt.is_some(),
                        "output_chars": response.output.chars().count(),
                        "usage": usage,
                        "finish_reason": finish_reason,
                    }),
                ))
                .await?;

                Ok(response)
            }
            Err(error) => {
                let _ = self
                    .record_activity(Activity::new(
                        ActivityActor::System,
                        "playground-service",
                        "ai_inference_failed",
                        format!("AI inference failed for {:?}", config.ai_backend.kind),
                        serde_json::json!({
                            "backend_kind": format!("{:?}", config.ai_backend.kind),
                            "backend_model": config.ai_backend.model,
                            "error": error.to_string(),
                        }),
                    ))
                    .await;

                Err(error)
            }
        }
    }

    async fn record_activity(&self, activity: Activity) -> Result<(), StorageError> {
        let stored = self.storage.store_activity(activity).await?;
        let _ = self.activity_tx.send(stored);
        Ok(())
    }

    pub async fn backend_catalog(&self) -> Vec<AiBackendPublic> {
        let config = self.config.read().await;
        let active_kind = config.ai_backend.kind;
        let mut list = Vec::new();

        list.push(match active_kind {
            AiBackendKind::LocalLlama => config.ai_backend.to_public(),
            _ => AiBackendPublic {
                kind: AiBackendKind::LocalLlama,
                model: "llama3.2:3b".to_string(),
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

        list.push(match active_kind {
            AiBackendKind::Grok => config.ai_backend.to_public(),
            _ => AiBackendPublic {
                kind: AiBackendKind::Grok,
                model: DEFAULT_GROK_MODEL.to_string(),
                endpoint: None,
                active: false,
                configured: false,
            },
        });

        list.push(match active_kind {
            AiBackendKind::Gemini => config.ai_backend.to_public(),
            _ => AiBackendPublic {
                kind: AiBackendKind::Gemini,
                model: DEFAULT_GEMINI_MODEL.to_string(),
                endpoint: None,
                active: false,
                configured: false,
            },
        });

        list
    }
}
