//! Deployment commands

use crate::client::PalmClient;
use crate::error::CliResult;
use crate::output::{self, print_error, print_info, print_success, OutputFormat};
use clap::Subcommand;
use indicatif::{ProgressBar, ProgressStyle};
use palm_types::*;
use serde::Serialize;
use tabled::Tabled;

/// Deployment subcommands
#[derive(Subcommand)]
pub enum DeploymentCommands {
    /// Create a new deployment
    Create {
        /// Spec ID to deploy
        spec_id: String,

        /// Number of replicas
        #[arg(short, long, default_value = "1")]
        replicas: u32,

        /// Rollout strategy (rolling, bluegreen, canary, recreate)
        #[arg(short, long, default_value = "rolling")]
        strategy: String,

        /// Wait for deployment to complete
        #[arg(short, long)]
        wait: bool,
    },

    /// Get deployment details
    Get {
        /// Deployment ID
        deployment_id: String,
    },

    /// List deployments
    List {
        /// Show all (including completed/failed)
        #[arg(short, long)]
        all: bool,
    },

    /// Scale a deployment
    Scale {
        /// Deployment ID
        deployment_id: String,

        /// Target replica count
        replicas: u32,
    },

    /// Update deployment to new spec version
    Update {
        /// Deployment ID
        deployment_id: String,

        /// New spec ID
        #[arg(short, long)]
        spec_id: String,

        /// Rollout strategy
        #[arg(short = 'S', long, default_value = "rolling")]
        strategy: String,

        /// Wait for update to complete
        #[arg(short, long)]
        wait: bool,
    },

    /// Rollback deployment
    Rollback {
        /// Deployment ID
        deployment_id: String,

        /// Target version (omit for previous)
        #[arg(short, long)]
        version: Option<String>,
    },

    /// Delete a deployment
    Delete {
        /// Deployment ID
        deployment_id: String,

        /// Force deletion (skip draining)
        #[arg(short, long)]
        force: bool,

        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Show deployment history
    History {
        /// Deployment ID
        deployment_id: String,
    },
}

/// Table row for deployment display
#[derive(Debug, Serialize, Tabled)]
struct DeploymentRow {
    /// Deployment ID (short form)
    id: String,
    /// Spec ID (short form)
    spec: String,
    /// Deployment status
    status: String,
    /// Ready/total replicas
    ready: String,
    /// Deployment strategy
    strategy: String,
    /// Age
    age: String,
}

impl From<Deployment> for DeploymentRow {
    fn from(d: Deployment) -> Self {
        let ready = format!("{}/{}", d.replicas.current_healthy, d.replicas.desired);
        let age = humanize_duration(chrono::Utc::now() - d.created_at);

        Self {
            id: truncate_id(&d.id.to_string()),
            spec: truncate_id(&d.agent_spec_id.to_string()),
            status: format_status(&d.status),
            ready,
            strategy: format_strategy(&d.strategy),
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

fn format_status(status: &DeploymentStatus) -> String {
    match status {
        DeploymentStatus::Pending => "Pending".to_string(),
        DeploymentStatus::InProgress { progress, phase } => format!("{}% ({})", progress, phase),
        DeploymentStatus::Paused { reason, .. } => format!("Paused: {}", reason),
        DeploymentStatus::Completed { .. } => "Completed".to_string(),
        DeploymentStatus::Failed { reason, .. } => format!("Failed: {}", reason),
        DeploymentStatus::RollingBack { target_version } => {
            format!("Rolling back to {}", target_version)
        }
    }
}

fn format_strategy(strategy: &DeploymentStrategy) -> String {
    match strategy {
        DeploymentStrategy::Rolling { .. } => "Rolling".to_string(),
        DeploymentStrategy::BlueGreen { .. } => "BlueGreen".to_string(),
        DeploymentStrategy::Canary { .. } => "Canary".to_string(),
        DeploymentStrategy::Recreate => "Recreate".to_string(),
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

fn is_terminal_status(status: &DeploymentStatus) -> bool {
    matches!(
        status,
        DeploymentStatus::Completed { .. } | DeploymentStatus::Failed { .. }
    )
}

/// Execute a deployment command
pub async fn execute(
    command: DeploymentCommands,
    client: &PalmClient,
    format: OutputFormat,
) -> CliResult<()> {
    match command {
        DeploymentCommands::Create {
            spec_id,
            replicas,
            strategy,
            wait,
        } => {
            print_info(&format!(
                "Creating deployment with {} replicas using {} strategy...",
                replicas, strategy
            ));

            let deployment = client
                .create_deployment(&spec_id, replicas, &strategy)
                .await?;
            print_success(&format!("Created deployment: {}", deployment.id));

            if wait {
                wait_for_deployment(client, &deployment.id.to_string()).await?;
            }

            Ok(())
        }

        DeploymentCommands::Get { deployment_id } => {
            let deployment = client.get_deployment(&deployment_id).await?;
            output::print_single(&deployment, format);
            Ok(())
        }

        DeploymentCommands::List { all } => {
            let deployments = client.list_deployments().await?;
            let filtered: Vec<DeploymentRow> = deployments
                .into_iter()
                .filter(|d| all || !is_terminal_status(&d.status))
                .map(DeploymentRow::from)
                .collect();
            output::print_output(filtered, format);
            Ok(())
        }

        DeploymentCommands::Scale {
            deployment_id,
            replicas,
        } => {
            client.scale_deployment(&deployment_id, replicas).await?;
            print_success(&format!(
                "Scaled deployment {} to {} replicas",
                deployment_id, replicas
            ));
            Ok(())
        }

        DeploymentCommands::Update {
            deployment_id,
            spec_id,
            strategy,
            wait,
        } => {
            print_info(&format!(
                "Updating deployment to spec {} using {} strategy...",
                spec_id, strategy
            ));

            let deployment = client
                .update_deployment(&deployment_id, &spec_id, &strategy)
                .await?;
            print_success(&format!(
                "Update initiated for deployment: {}",
                deployment.id
            ));

            if wait {
                wait_for_deployment(client, &deployment_id).await?;
            }

            Ok(())
        }

        DeploymentCommands::Rollback {
            deployment_id,
            version,
        } => {
            let deployment = client
                .rollback_deployment(&deployment_id, version.as_deref())
                .await?;
            print_success(&format!(
                "Rollback initiated for deployment: {}",
                deployment.id
            ));
            Ok(())
        }

        DeploymentCommands::Delete {
            deployment_id,
            force,
            yes,
        } => {
            if !yes {
                let msg = if force {
                    format!(
                        "Force delete deployment {}? This will terminate instances immediately.",
                        deployment_id
                    )
                } else {
                    format!(
                        "Delete deployment {}? Instances will be gracefully terminated.",
                        deployment_id
                    )
                };

                let confirm = dialoguer::Confirm::new()
                    .with_prompt(msg)
                    .default(false)
                    .interact()
                    .unwrap_or(false);

                if !confirm {
                    print_error("Aborted");
                    return Ok(());
                }
            }

            client.delete_deployment(&deployment_id, force).await?;
            print_success(&format!("Deleted deployment: {}", deployment_id));
            Ok(())
        }

        DeploymentCommands::History { deployment_id } => {
            let deployment = client.get_deployment(&deployment_id).await?;
            // Would show revision history - placeholder
            println!("Deployment history for {}", deployment_id);
            println!(
                "Current: spec={}, replicas={}",
                deployment.agent_spec_id, deployment.replicas.desired
            );
            Ok(())
        }
    }
}

async fn wait_for_deployment(client: &PalmClient, deployment_id: &str) -> CliResult<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Waiting for deployment...");

    loop {
        let deployment = client.get_deployment(deployment_id).await?;

        match &deployment.status {
            DeploymentStatus::Completed { .. } => {
                pb.finish_with_message("Deployment complete!");
                return Ok(());
            }
            DeploymentStatus::Failed { reason, .. } => {
                pb.finish_with_message(format!("Deployment failed: {}", reason));
                return Err(crate::error::CliError::Api {
                    status: 500,
                    message: format!("Deployment failed: {}", reason),
                });
            }
            DeploymentStatus::InProgress { progress, phase } => {
                pb.set_message(format!(
                    "{}% - {} ({}/{} ready)",
                    progress,
                    phase,
                    deployment.replicas.current_healthy,
                    deployment.replicas.desired
                ));
            }
            _ => {
                pb.set_message(format!("Phase: {:?}", deployment.status));
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}
