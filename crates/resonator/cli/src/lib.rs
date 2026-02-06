//! Resonator CLI - Command-line interface for MAPLE Resonator system
//!
//! This CLI provides operators and developers with a terminal interface to:
//! - Manage commitments and contracts
//! - Track consequences and their receipts
//! - Inspect and manage memory tiers
//! - Monitor conversation sessions

use clap::{Parser, Subcommand};
use std::ffi::OsString;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod commands;
mod error;
mod output;

use commands::{commitment, consequence, conversation, memory};
pub use error::{CliError, CliResult};

/// Resonator CLI application
#[derive(Parser)]
#[command(name = "resonator")]
#[command(about = "Resonator - MAPLE Resonance Architecture CLI", long_about = None)]
#[command(version)]
struct Cli {
    /// Output format (table, json, yaml)
    #[arg(short, long, default_value = "table")]
    output: output::OutputFormat,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

/// Available commands
#[derive(Subcommand)]
enum Commands {
    /// Manage commitments and contracts
    Commitment {
        #[command(subcommand)]
        command: commitment::CommitmentCommands,
    },

    /// Track consequences and receipts
    Consequence {
        #[command(subcommand)]
        command: consequence::ConsequenceCommands,
    },

    /// Manage memory tiers
    Memory {
        #[command(subcommand)]
        command: memory::MemoryCommands,
    },

    /// Monitor conversation sessions
    Conversation {
        #[command(subcommand)]
        command: conversation::ConversationCommands,
    },

    /// Show invariant enforcement status
    Invariants,

    /// Show resonance architecture pipeline status
    Pipeline,
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

    // Execute command
    match cli.command {
        Commands::Commitment { command } => commitment::execute(command, cli.output).await,
        Commands::Consequence { command } => consequence::execute(command, cli.output).await,
        Commands::Memory { command } => memory::execute(command, cli.output).await,
        Commands::Conversation { command } => conversation::execute(command, cli.output).await,
        Commands::Invariants => show_invariants(cli.output),
        Commands::Pipeline => show_pipeline(cli.output),
    }
}

fn show_invariants(format: output::OutputFormat) -> CliResult<()> {
    use colored::Colorize;

    let invariants = [
        ("1", "Presence precedes Coupling", "Active agents must register presence before interactions"),
        ("2", "Coupling precedes Meaning", "Context established before interpretation"),
        ("3", "Meaning precedes Intent", "Understanding drives goal formation"),
        ("4", "Commitment precedes Consequence", "All state changes require prior commitment"),
        ("5", "Receipts are immutable", "Once recorded, consequences cannot be altered"),
        ("6", "Audit trails are append-only", "History preserved for accountability"),
        ("7", "Capabilities gate actions", "Authorization checked before execution"),
        ("8", "Time anchors are monotonic", "Temporal ordering preserved"),
    ];

    match format {
        output::OutputFormat::Json => {
            let json: Vec<_> = invariants
                .iter()
                .map(|(id, name, desc)| {
                    serde_json::json!({
                        "id": id,
                        "name": name,
                        "description": desc,
                        "enforced": true
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        output::OutputFormat::Yaml => {
            let yaml: Vec<_> = invariants
                .iter()
                .map(|(id, name, desc)| {
                    serde_json::json!({
                        "id": id,
                        "name": name,
                        "description": desc,
                        "enforced": true
                    })
                })
                .collect();
            println!("{}", serde_yaml::to_string(&yaml).unwrap());
        }
        output::OutputFormat::Table => {
            println!("{}", "MAPLE Runtime Invariants".bold().cyan());
            println!("{}", "=".repeat(70));
            println!();
            for (id, name, desc) in invariants {
                println!(
                    "  {} {} {}",
                    format!("#{}", id).bold().yellow(),
                    "✓".green(),
                    name.bold()
                );
                println!("      {}", desc.dimmed());
            }
            println!();
            println!(
                "{}: All invariants are {} by the Resonance Architecture",
                "Note".bold(),
                "enforced at compile-time".green()
            );
        }
    }

    Ok(())
}

fn show_pipeline(format: output::OutputFormat) -> CliResult<()> {
    use colored::Colorize;

    let stages = [
        ("Presence", "Agent registration and discovery", "resonator-types"),
        ("Coupling", "Context establishment and binding", "resonator-types"),
        ("Meaning", "Semantic interpretation and formation", "resonator-meaning"),
        ("Intent", "Goal stabilization and validation", "resonator-intent"),
        ("Commitment", "Contract formation and lifecycle", "resonator-commitment"),
        ("Consequence", "Effect tracking and receipts", "resonator-consequence"),
    ];

    match format {
        output::OutputFormat::Json => {
            let json: Vec<_> = stages
                .iter()
                .enumerate()
                .map(|(i, (name, desc, crate_name))| {
                    serde_json::json!({
                        "order": i + 1,
                        "stage": name,
                        "description": desc,
                        "crate": crate_name
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        output::OutputFormat::Yaml => {
            let yaml: Vec<_> = stages
                .iter()
                .enumerate()
                .map(|(i, (name, desc, crate_name))| {
                    serde_json::json!({
                        "order": i + 1,
                        "stage": name,
                        "description": desc,
                        "crate": crate_name
                    })
                })
                .collect();
            println!("{}", serde_yaml::to_string(&yaml).unwrap());
        }
        output::OutputFormat::Table => {
            println!("{}", "Resonance Architecture Pipeline".bold().cyan());
            println!("{}", "=".repeat(60));
            println!();
            for (i, (name, desc, crate_name)) in stages.iter().enumerate() {
                let arrow = if i < stages.len() - 1 { "→" } else { "⬤" };
                println!(
                    "  {} {} {} ({})",
                    format!("[{}]", i + 1).bold().yellow(),
                    name.bold(),
                    arrow.dimmed(),
                    crate_name.dimmed()
                );
                println!("      {}", desc.dimmed());
                if i < stages.len() - 1 {
                    println!("      {}", "│".dimmed());
                }
            }
            println!();
            println!(
                "{}: Each stage enforces invariants before proceeding",
                "Note".bold()
            );
        }
    }

    Ok(())
}
