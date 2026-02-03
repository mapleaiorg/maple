//! Playground commands

use crate::client::PalmClient;
use crate::error::CliResult;
use crate::output::{self, OutputFormat, print_info, print_success};
use clap::Subcommand;
use palm_shared_state::{Activity, PlaygroundConfigUpdate, AiBackendConfigUpdate};
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

    /// Set the active AI backend
    SetBackend {
        /// Backend kind: local_llama, open_ai, anthropic
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
        PlaygroundCommands::SetBackend { kind, model, endpoint, api_key } => {
            print_info("Updating backend configuration...");
            let update = PlaygroundConfigUpdate {
                ai_backend: Some(AiBackendConfigUpdate {
                    kind: Some(match kind.as_str() {
                        "local_llama" => palm_shared_state::AiBackendKind::LocalLlama,
                        "open_ai" => palm_shared_state::AiBackendKind::OpenAI,
                        "anthropic" => palm_shared_state::AiBackendKind::Anthropic,
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
    }
}
