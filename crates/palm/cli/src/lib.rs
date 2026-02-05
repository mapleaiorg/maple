//! PALM CLI - Command-line interface for fleet management
//!
//! This CLI provides operators and developers with a terminal interface to:
//! - Manage agent specifications
//! - Create/update/scale/delete deployments
//! - Inspect instance health and state
//! - Trigger checkpoints and migrations
//! - View events and audit logs

use clap::{Parser, Subcommand};
use std::ffi::OsString;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod client;
mod commands;
mod config;
mod error;
mod output;

use commands::{deployment, events, health, instance, playground, spec, state};
use config::CliConfig;
pub use error::{CliError, CliResult};

/// PALM CLI application
#[derive(Parser)]
#[command(name = "palm")]
#[command(about = "PALM - Platform Agent Lifecycle Manager CLI", long_about = None)]
#[command(version)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, env = "PALM_CONFIG")]
    config: Option<String>,

    /// PALM daemon endpoint
    #[arg(
        short,
        long,
        env = "PALM_ENDPOINT",
        default_value = "http://localhost:8080"
    )]
    endpoint: String,

    /// Output format (table, json, yaml)
    #[arg(short, long, default_value = "table")]
    output: output::OutputFormat,

    /// Platform profile (mapleverse, finalverse, ibank)
    #[arg(short, long, env = "PALM_PLATFORM")]
    platform: Option<String>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

/// Available commands
#[derive(Subcommand)]
enum Commands {
    /// Manage agent specifications
    Spec {
        #[command(subcommand)]
        command: spec::SpecCommands,
    },

    /// Manage deployments
    #[command(alias = "deploy")]
    Deployment {
        #[command(subcommand)]
        command: deployment::DeploymentCommands,
    },

    /// Manage instances
    Instance {
        #[command(subcommand)]
        command: instance::InstanceCommands,
    },

    /// State and checkpoint management
    State {
        #[command(subcommand)]
        command: state::StateCommands,
    },

    /// Health monitoring
    Health {
        #[command(subcommand)]
        command: health::HealthCommands,
    },

    /// Event streaming
    Events {
        #[command(subcommand)]
        command: events::EventCommands,
    },

    /// Playground operations
    Playground {
        #[command(subcommand)]
        command: playground::PlaygroundCommands,
    },

    /// Show configuration
    Config,

    /// Check daemon connectivity
    Status,
}

/// Run using the current process arguments.
pub async fn run() -> CliResult<()> {
    run_with_args(std::env::args_os()).await
}

/// Run using the provided argument iterator.
pub async fn run_with_args<I, T>(args: I) -> CliResult<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse_from(args);

    // Initialize tracing
    let filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    // Load config
    let config = CliConfig::load(cli.config.as_deref())?;
    let endpoint = cli.endpoint.clone();
    let platform = cli.platform.or(config.default_platform.clone());

    // Create client
    let client = client::PalmClient::new(&endpoint, platform.clone())?;

    // Execute command
    match cli.command {
        Commands::Spec { command } => spec::execute(command, &client, cli.output).await,
        Commands::Deployment { command } => deployment::execute(command, &client, cli.output).await,
        Commands::Instance { command } => instance::execute(command, &client, cli.output).await,
        Commands::State { command } => state::execute(command, &client, cli.output).await,
        Commands::Health { command } => health::execute(command, &client, cli.output).await,
        Commands::Events { command } => events::execute(command, &client).await,
        Commands::Playground { command } => playground::execute(command, &client, cli.output).await,
        Commands::Config => {
            println!("Endpoint: {}", endpoint);
            println!("Platform: {:?}", platform);
            println!("Config: {:?}", config);
            Ok(())
        }
        Commands::Status => match client.health_check().await {
            Ok(status) => {
                println!("✓ PALM daemon is healthy");
                println!("  Version: {}", status.version);
                println!("  Uptime: {}", status.uptime);
                if let Some(platform) = status.platform.as_deref() {
                    println!("  Platform: {}", platform);
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("✗ Cannot connect to PALM daemon: {}", e);
                std::process::exit(1);
            }
        },
    }
}
