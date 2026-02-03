//! Shared state models for the MAPLE playground and PALM services.

#![deny(unsafe_code)]

use chrono::{DateTime, Utc};
use palm_types::instance::AgentInstance;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Supported AI backend kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiBackendKind {
    LocalLlama,
    OpenAI,
    Anthropic,
    Grok,
    Gemini,
}

/// Configuration for the active AI backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiBackendConfig {
    pub kind: AiBackendKind,
    pub model: String,
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

impl AiBackendConfig {
    pub fn requires_api_key(&self) -> bool {
        !matches!(self.kind, AiBackendKind::LocalLlama)
    }

    pub fn is_configured(&self) -> bool {
        if self.requires_api_key() {
            self.api_key
                .as_ref()
                .map(|k| !k.trim().is_empty())
                .unwrap_or(false)
        } else {
            self.endpoint.is_some()
        }
    }

    pub fn to_public(&self) -> AiBackendPublic {
        AiBackendPublic {
            kind: self.kind,
            model: self.model.clone(),
            endpoint: self.endpoint.clone(),
            active: true,
            configured: self.is_configured(),
        }
    }
}

/// Public AI backend info (no secrets).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiBackendPublic {
    pub kind: AiBackendKind,
    pub model: String,
    pub endpoint: Option<String>,
    pub active: bool,
    pub configured: bool,
}

/// Simulation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    #[serde(default = "default_simulation_enabled")]
    pub enabled: bool,
    #[serde(default = "default_tick_interval_ms")]
    pub tick_interval_ms: u64,
    #[serde(default = "default_simulation_intensity")]
    pub intensity: f32,
    #[serde(default = "default_max_resonators")]
    pub max_resonators: u32,
    #[serde(default = "default_max_agents")]
    pub max_agents: u32,
    #[serde(default = "default_auto_inference_enabled")]
    pub auto_inference_enabled: bool,
    #[serde(default = "default_inference_interval_ticks")]
    pub inference_interval_ticks: u64,
    #[serde(default = "default_inferences_per_tick")]
    pub inferences_per_tick: u32,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            enabled: default_simulation_enabled(),
            tick_interval_ms: default_tick_interval_ms(),
            intensity: default_simulation_intensity(),
            max_resonators: default_max_resonators(),
            max_agents: default_max_agents(),
            auto_inference_enabled: default_auto_inference_enabled(),
            inference_interval_ticks: default_inference_interval_ticks(),
            inferences_per_tick: default_inferences_per_tick(),
        }
    }
}

const fn default_simulation_enabled() -> bool {
    true
}

const fn default_tick_interval_ms() -> u64 {
    1200
}

const fn default_simulation_intensity() -> f32 {
    0.75
}

const fn default_max_resonators() -> u32 {
    64
}

const fn default_max_agents() -> u32 {
    64
}

const fn default_auto_inference_enabled() -> bool {
    true
}

const fn default_inference_interval_ticks() -> u64 {
    5
}

const fn default_inferences_per_tick() -> u32 {
    1
}

/// Playground configuration stored in persistent state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaygroundConfig {
    pub ai_backend: AiBackendConfig,
    pub simulation: SimulationConfig,
    pub updated_at: DateTime<Utc>,
}

impl Default for PlaygroundConfig {
    fn default() -> Self {
        Self {
            ai_backend: AiBackendConfig {
                kind: AiBackendKind::LocalLlama,
                model: "llama3".to_string(),
                endpoint: Some("http://127.0.0.1:11434".to_string()),
                api_key: None,
                temperature: Some(0.7),
                max_tokens: Some(2048),
            },
            simulation: SimulationConfig::default(),
            updated_at: Utc::now(),
        }
    }
}

impl PlaygroundConfig {
    pub fn public_view(&self) -> PlaygroundConfigPublic {
        PlaygroundConfigPublic {
            ai_backend: self.ai_backend.to_public(),
            simulation: self.simulation.clone(),
            updated_at: self.updated_at,
        }
    }
}

/// Public-facing playground configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaygroundConfigPublic {
    pub ai_backend: AiBackendPublic,
    pub simulation: SimulationConfig,
    pub updated_at: DateTime<Utc>,
}

/// Partial update payload for playground config.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlaygroundConfigUpdate {
    pub ai_backend: Option<AiBackendConfigUpdate>,
    pub simulation: Option<SimulationConfig>,
}

/// Partial update payload for AI backend config.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AiBackendConfigUpdate {
    pub kind: Option<AiBackendKind>,
    pub model: Option<String>,
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

impl PlaygroundConfigUpdate {
    pub fn apply(self, mut current: PlaygroundConfig) -> PlaygroundConfig {
        if let Some(simulation) = self.simulation {
            current.simulation = simulation;
        }

        if let Some(ai_update) = self.ai_backend {
            if let Some(kind) = ai_update.kind {
                current.ai_backend.kind = kind;
            }
            if let Some(model) = ai_update.model {
                current.ai_backend.model = model;
            }
            if let Some(endpoint) = ai_update.endpoint {
                current.ai_backend.endpoint = Some(endpoint);
            }
            if let Some(api_key) = ai_update.api_key {
                current.ai_backend.api_key = Some(api_key);
            }
            if let Some(temp) = ai_update.temperature {
                current.ai_backend.temperature = Some(temp);
            }
            if let Some(max_tokens) = ai_update.max_tokens {
                current.ai_backend.max_tokens = Some(max_tokens);
            }
        }

        current.updated_at = Utc::now();
        current
    }
}

/// Inference request payload routed through the active playground backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaygroundInferenceRequest {
    pub prompt: String,
    pub system_prompt: Option<String>,
    pub actor_id: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

impl PlaygroundInferenceRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.prompt.trim().is_empty() {
            return Err("prompt cannot be empty".to_string());
        }

        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err("temperature must be between 0.0 and 2.0".to_string());
            }
        }

        if let Some(max_tokens) = self.max_tokens {
            if max_tokens == 0 {
                return Err("max_tokens must be greater than 0".to_string());
            }
        }

        Ok(())
    }
}

/// Token usage stats returned by an inference backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceTokenUsage {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

/// Inference response payload returned by playground APIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaygroundInferenceResponse {
    pub backend_kind: AiBackendKind,
    pub backend_model: String,
    pub output: String,
    pub latency_ms: u64,
    pub created_at: DateTime<Utc>,
    pub finish_reason: Option<String>,
    pub usage: Option<InferenceTokenUsage>,
}

/// Activity actor type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityActor {
    Agent,
    Human,
    Resonator,
    System,
}

/// Activity log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: Uuid,
    pub sequence: u64,
    pub timestamp: DateTime<Utc>,
    pub actor_type: ActivityActor,
    pub actor_id: String,
    pub kind: String,
    pub summary: String,
    pub details: Value,
}

impl Activity {
    pub fn new(
        actor_type: ActivityActor,
        actor_id: impl Into<String>,
        kind: impl Into<String>,
        summary: impl Into<String>,
        details: Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            sequence: 0,
            timestamp: Utc::now(),
            actor_type,
            actor_id: actor_id.into(),
            kind: kind.into(),
            summary: summary.into(),
            details,
        }
    }
}

/// Presence snapshot summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceSnapshot {
    pub discoverability: f64,
    pub responsiveness: f64,
    pub stability: f64,
    pub coupling_readiness: f64,
    pub silent_mode: bool,
}

impl Default for PresenceSnapshot {
    fn default() -> Self {
        Self {
            discoverability: 0.5,
            responsiveness: 0.8,
            stability: 0.9,
            coupling_readiness: 0.6,
            silent_mode: false,
        }
    }
}

/// Coupling snapshot summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingSnapshot {
    pub peer_id: String,
    pub strength: f64,
    pub meaning_convergence: f64,
    pub interaction_count: u64,
    pub attention_allocated: u64,
}

/// Resonator status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResonatorStatusKind {
    Idle,
    Processing,
    WaitingForApproval,
    Draining,
    Offline,
    Error,
}

/// Resonator state summary stored for the playground.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonatorStatus {
    pub id: String,
    pub name: String,
    pub status: ResonatorStatusKind,
    pub presence: PresenceSnapshot,
    pub couplings: Vec<CouplingSnapshot>,
    pub attention_utilization: f64,
    pub last_activity: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Aggregate system stats for the playground.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemStats {
    pub agents_total: usize,
    pub agents_healthy: usize,
    pub resonators_total: usize,
    pub activities_total: usize,
    pub active_couplings: usize,
    pub last_activity_at: Option<DateTime<Utc>>,
}

/// Aggregated playground state for dashboards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub generated_at: DateTime<Utc>,
    pub playground: PlaygroundConfigPublic,
    pub backends: Vec<AiBackendPublic>,
    pub stats: SystemStats,
    pub agents: Vec<AgentInstance>,
    pub resonators: Vec<ResonatorStatus>,
    pub activities: Vec<Activity>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulation_defaults_include_auto_inference_controls() {
        let simulation = SimulationConfig::default();
        assert!(simulation.enabled);
        assert!(simulation.auto_inference_enabled);
        assert_eq!(simulation.inference_interval_ticks, 5);
        assert_eq!(simulation.inferences_per_tick, 1);
    }

    #[test]
    fn inference_request_validation_rejects_invalid_inputs() {
        let empty_prompt = PlaygroundInferenceRequest {
            prompt: " ".to_string(),
            system_prompt: None,
            actor_id: None,
            temperature: Some(0.7),
            max_tokens: Some(128),
        };
        assert!(empty_prompt.validate().is_err());

        let invalid_temp = PlaygroundInferenceRequest {
            prompt: "ok".to_string(),
            system_prompt: None,
            actor_id: None,
            temperature: Some(4.0),
            max_tokens: Some(128),
        };
        assert!(invalid_temp.validate().is_err());
    }
}
