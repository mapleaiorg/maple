//! Health monitoring commands

use crate::client::PalmClient;
use crate::error::CliResult;
use crate::output::{print_success, print_warning, OutputFormat};
use clap::Subcommand;
use colored::*;

/// Health subcommands
#[derive(Subcommand)]
pub enum HealthCommands {
    /// Check health of a specific instance
    Check {
        /// Instance ID
        instance_id: String,
    },

    /// List all unhealthy instances
    Unhealthy,

    /// Show overall fleet health summary
    Summary,
}

/// Execute a health command
pub async fn execute(
    command: HealthCommands,
    client: &PalmClient,
    _format: OutputFormat,
) -> CliResult<()> {
    match command {
        HealthCommands::Check { instance_id } => {
            let health = client.get_instance_health(&instance_id).await?;

            println!("Instance: {}", instance_id);
            println!("Status: {}", colorize_status(&health.status));
            println!("Last Check: {}", health.last_check);
            println!("\nProbes:");

            for probe in &health.probes {
                let status = if probe.passed {
                    "✓".green()
                } else {
                    "✗".red()
                };
                print!("  {} {}", status, probe.name);
                if let Some(details) = &probe.details {
                    print!(" - {}", details.dimmed());
                }
                println!();
            }

            Ok(())
        }

        HealthCommands::Unhealthy => {
            let instances = client.list_unhealthy().await?;

            if instances.is_empty() {
                print_success("All instances are healthy!");
            } else {
                print_warning(&format!("{} unhealthy instances", instances.len()));

                for instance in instances {
                    println!("  {} - {:?}", instance.id, instance.health);
                }
            }

            Ok(())
        }

        HealthCommands::Summary => {
            // Would aggregate from daemon
            println!("Fleet Health Summary");
            println!("--------------------");
            println!("Total instances: 0");
            println!("Healthy: 0");
            println!("Degraded: 0");
            println!("Unhealthy: 0");
            Ok(())
        }
    }
}

fn colorize_status(status: &str) -> colored::ColoredString {
    match status.to_lowercase().as_str() {
        "healthy" => status.green(),
        "degraded" => status.yellow(),
        "unhealthy" => status.red(),
        _ => status.dimmed(),
    }
}
