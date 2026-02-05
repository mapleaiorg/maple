//! Instance commands

use crate::client::PalmClient;
use crate::error::CliResult;
use crate::output::{self, print_success, print_warning, OutputFormat};
use clap::Subcommand;
use palm_types::*;
use serde::Serialize;
use tabled::Tabled;

/// Instance subcommands
#[derive(Subcommand)]
pub enum InstanceCommands {
    /// Get instance details
    Get {
        /// Instance ID
        instance_id: String,
    },

    /// List instances
    List {
        /// Filter by deployment ID
        #[arg(short, long)]
        deployment: Option<String>,

        /// Show only unhealthy instances
        #[arg(short, long)]
        unhealthy: bool,
    },

    /// Restart an instance
    Restart {
        /// Instance ID
        instance_id: String,

        /// Force restart (skip graceful shutdown)
        #[arg(short, long)]
        force: bool,
    },

    /// Drain an instance (prepare for termination)
    Drain {
        /// Instance ID
        instance_id: String,
    },

    /// Migrate instance to another node
    Migrate {
        /// Instance ID
        instance_id: String,

        /// Target node ID
        #[arg(short, long)]
        to_node: String,
    },

    /// Show instance logs (placeholder)
    Logs {
        /// Instance ID
        instance_id: String,

        /// Follow logs
        #[arg(short, long)]
        follow: bool,

        /// Number of lines
        #[arg(short, long, default_value = "100")]
        lines: u32,
    },
}

/// Table row for instance display
#[derive(Debug, Serialize, Tabled)]
struct InstanceRow {
    /// Instance ID (short form)
    id: String,
    /// Deployment ID (short form)
    deployment: String,
    /// Instance status
    status: String,
    /// Health status
    health: String,
    /// Node ID (short form)
    node: String,
    /// Age
    age: String,
}

impl From<AgentInstance> for InstanceRow {
    fn from(i: AgentInstance) -> Self {
        let age = humanize_duration(chrono::Utc::now() - i.started_at);
        let node = i
            .placement
            .node_id
            .map(|n| truncate_id(&n.to_string()))
            .unwrap_or_else(|| "-".to_string());

        Self {
            id: truncate_id(&i.id.to_string()),
            deployment: truncate_id(&i.deployment_id.to_string()),
            status: format_status(&i.status),
            health: format_health(&i.health),
            node,
            age,
        }
    }
}

fn truncate_id(id: &str) -> String {
    if id.len() > 8 {
        id[..8].to_string()
    } else {
        id.to_string()
    }
}

fn format_status(status: &InstanceStatus) -> String {
    match status {
        InstanceStatus::Starting { phase } => format!("Starting ({:?})", phase),
        InstanceStatus::Running => "Running".to_string(),
        InstanceStatus::Draining { reason } => format!("Draining ({:?})", reason),
        InstanceStatus::Terminating { reason } => format!("Terminating ({:?})", reason),
        InstanceStatus::Terminated { exit_code } => match exit_code {
            Some(code) => format!("Terminated ({})", code),
            None => "Terminated".to_string(),
        },
        InstanceStatus::Error { message } => format!("Error: {}", message),
    }
}

fn format_health(health: &HealthStatus) -> String {
    match health {
        HealthStatus::Unknown => "Unknown".to_string(),
        HealthStatus::Healthy => "✓ Healthy".to_string(),
        HealthStatus::Unhealthy { reasons } => {
            let reason = reasons.first().map(|s| s.as_str()).unwrap_or("unknown");
            format!("✗ {}", reason)
        }
        HealthStatus::Degraded { factors } => {
            let factor = factors.first().map(|s| s.as_str()).unwrap_or("degraded");
            format!("⚠ {}", factor)
        }
    }
}

fn humanize_duration(duration: chrono::Duration) -> String {
    if duration.num_days() > 0 {
        format!("{}d", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{}h", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{}m", duration.num_minutes())
    } else {
        format!("{}s", duration.num_seconds())
    }
}

/// Execute an instance command
pub async fn execute(
    command: InstanceCommands,
    client: &PalmClient,
    format: OutputFormat,
) -> CliResult<()> {
    match command {
        InstanceCommands::Get { instance_id } => {
            let instance = client.get_instance(&instance_id).await?;
            output::print_single(&instance, format);
            Ok(())
        }

        InstanceCommands::List {
            deployment,
            unhealthy,
        } => {
            let instances = if unhealthy {
                client.list_unhealthy().await?
            } else {
                client.list_instances(deployment.as_deref()).await?
            };

            let rows: Vec<InstanceRow> = instances.into_iter().map(InstanceRow::from).collect();
            output::print_output(rows, format);
            Ok(())
        }

        InstanceCommands::Restart { instance_id, force } => {
            if force {
                print_warning("Force restart will not checkpoint state");
            }

            client.restart_instance(&instance_id, !force).await?;
            print_success(&format!("Restarted instance: {}", instance_id));
            Ok(())
        }

        InstanceCommands::Drain { instance_id } => {
            client.drain_instance(&instance_id).await?;
            print_success(&format!("Draining instance: {}", instance_id));
            Ok(())
        }

        InstanceCommands::Migrate {
            instance_id,
            to_node,
        } => {
            let new_id = client.migrate_instance(&instance_id, &to_node).await?;
            print_success(&format!(
                "Migrated {} -> {} (new instance: {})",
                instance_id, to_node, new_id
            ));
            Ok(())
        }

        InstanceCommands::Logs {
            instance_id,
            follow,
            lines,
        } => {
            // Placeholder - would stream logs from daemon
            println!("Logs for instance {} (last {} lines)", instance_id, lines);
            if follow {
                println!("Following... (Ctrl+C to stop)");
            }
            Ok(())
        }
    }
}
