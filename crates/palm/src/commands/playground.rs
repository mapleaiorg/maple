//! Playground commands

use crate::client::PalmClient;
use crate::error::CliResult;
use crate::output::{self, print_info, print_success, OutputFormat};
use clap::Subcommand;
use palm_shared_state::{
    Activity, AiBackendConfigUpdate, AiBackendPublic, PlaygroundConfigUpdate,
    PlaygroundInferenceRequest,
};
use serde::Serialize;
use tabled::Tabled;

/// Playground subcommands
#[derive(Subcommand)]
pub enum PlaygroundCommands {
    /// Show playground status summary
    Status,

    /// Show playground configuration
    Config,

    /// List recent activities
    Activities {
        /// Limit number of activities
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },

    /// List resonators
    Resonators,

    /// List agents
    Agents,

    /// List available AI backends
    Backends,

    /// Set the active AI backend
    SetBackend {
        /// Backend kind: local_llama, open_ai, anthropic, grok, gemini
        #[arg(long)]
        kind: String,
        /// Model name
        #[arg(long)]
        model: Option<String>,
        /// Endpoint (for local Llama)
        #[arg(long)]
        endpoint: Option<String>,
        /// API key (for OpenAI/Anthropic)
        #[arg(long)]
        api_key: Option<String>,
    },

    /// Run a one-shot prompt on the active backend
    Infer {
        /// Prompt text to send to the active backend
        prompt: String,
        /// Optional system prompt
        #[arg(long)]
        system_prompt: Option<String>,
        /// Optional actor id for activity attribution
        #[arg(long)]
        actor_id: Option<String>,
        /// Optional temperature override
        #[arg(long)]
        temperature: Option<f32>,
        /// Optional max token override
        #[arg(long)]
        max_tokens: Option<u32>,
    },
}

#[derive(Debug, Serialize, Tabled)]
struct ActivityRow {
    sequence: u64,
    timestamp: String,
    actor: String,
    kind: String,
    summary: String,
}

impl From<Activity> for ActivityRow {
    fn from(a: Activity) -> Self {
        Self {
            sequence: a.sequence,
            timestamp: a.timestamp.to_rfc3339(),
            actor: format!("{:?}:{}", a.actor_type, a.actor_id),
            kind: a.kind,
            summary: a.summary,
        }
    }
}

#[derive(Debug, Serialize, Tabled)]
struct ResonatorRow {
    id: String,
    name: String,
    status: String,
    couplings: usize,
    attention: String,
}

#[derive(Debug, Serialize, Tabled)]
struct AgentRow {
    id: String,
    status: String,
    health: String,
    attention: String,
}

#[derive(Debug, Serialize, Tabled)]
struct BackendRow {
    kind: String,
    model: String,
    endpoint: String,
    active: bool,
    configured: bool,
}

impl From<AiBackendPublic> for BackendRow {
    fn from(b: AiBackendPublic) -> Self {
        Self {
            kind: format!("{:?}", b.kind),
            model: b.model,
            endpoint: b.endpoint.unwrap_or_else(|| "-".to_string()),
            active: b.active,
            configured: b.configured,
        }
    }
}

pub async fn execute(
    command: PlaygroundCommands,
    client: &PalmClient,
    format: OutputFormat,
) -> CliResult<()> {
    match command {
        PlaygroundCommands::Status => {
            let state = client.get_playground_state().await?;
            output::print_single(&state, format);
            Ok(())
        }
        PlaygroundCommands::Config => {
            let config = client.get_playground_config().await?;
            output::print_single(&config, format);
            Ok(())
        }
        PlaygroundCommands::Activities { limit } => {
            let activities = client.list_playground_activities(limit, None).await?;
            let rows: Vec<ActivityRow> = activities.into_iter().map(ActivityRow::from).collect();
            output::print_output(rows, format);
            Ok(())
        }
        PlaygroundCommands::Resonators => {
            let resonators = client.list_playground_resonators().await?;
            let rows: Vec<ResonatorRow> = resonators
                .into_iter()
                .map(|r| ResonatorRow {
                    id: r.id,
                    name: r.name,
                    status: format!("{:?}", r.status),
                    couplings: r.couplings.len(),
                    attention: format!("{:.2}", r.attention_utilization),
                })
                .collect();
            output::print_output(rows, format);
            Ok(())
        }
        PlaygroundCommands::Agents => {
            let agents = client.list_playground_agents().await?;
            let rows: Vec<AgentRow> = agents
                .into_iter()
                .map(|a| AgentRow {
                    id: a.id.to_string(),
                    status: format!("{:?}", a.status),
                    health: format!("{:?}", a.health),
                    attention: format!("{:.2}", a.metrics.attention_utilization),
                })
                .collect();
            output::print_output(rows, format);
            Ok(())
        }
        PlaygroundCommands::Backends => {
            let backends = client.list_playground_backends().await?;
            let rows: Vec<BackendRow> = backends.into_iter().map(BackendRow::from).collect();
            output::print_output(rows, format);
            Ok(())
        }
        PlaygroundCommands::SetBackend {
            kind,
            model,
            endpoint,
            api_key,
        } => {
            print_info("Updating backend configuration...");
            let update = PlaygroundConfigUpdate {
                ai_backend: Some(AiBackendConfigUpdate {
                    kind: Some(match kind.as_str() {
                        "local_llama" => palm_shared_state::AiBackendKind::LocalLlama,
                        "open_ai" => palm_shared_state::AiBackendKind::OpenAI,
                        "anthropic" => palm_shared_state::AiBackendKind::Anthropic,
                        "grok" => palm_shared_state::AiBackendKind::Grok,
                        "gemini" => palm_shared_state::AiBackendKind::Gemini,
                        other => {
                            return Err(crate::error::CliError::Config(format!(
                                "Unknown backend kind: {}",
                                other
                            )));
                        }
                    }),
                    model,
                    endpoint,
                    api_key,
                    temperature: None,
                    max_tokens: None,
                }),
                simulation: None,
            };

            let config = client.update_playground_config(&update).await?;
            print_success(&format!("Active backend: {:?}", config.ai_backend.kind));
            Ok(())
        }
        PlaygroundCommands::Infer {
            prompt,
            system_prompt,
            actor_id,
            temperature,
            max_tokens,
        } => {
            let request = PlaygroundInferenceRequest {
                prompt,
                system_prompt,
                actor_id,
                temperature,
                max_tokens,
            };
            let response = client.infer_playground_backend(&request).await?;
            output::print_single(&response, format);
            Ok(())
        }
    }
}
