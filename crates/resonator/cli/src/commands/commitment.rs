//! Commitment management commands

use crate::error::{CliError, CliResult};
use crate::output::OutputFormat;
use clap::Subcommand;
use colored::Colorize;
use resonator_commitment::{
    ContractEngine, ContractStatus, InMemoryContractEngine, StoredContract,
};
use serde::Serialize;

/// Commitment subcommands
#[derive(Subcommand)]
pub enum CommitmentCommands {
    /// List all commitments
    List {
        /// Filter by status
        #[arg(short, long)]
        status: Option<String>,
        /// Maximum number to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Inspect a specific commitment
    Inspect {
        /// Commitment ID
        id: String,
    },

    /// Show commitment lifecycle states
    Lifecycle,

    /// Show contract status transitions
    Transitions {
        /// Starting status
        from: String,
    },

    /// Validate a commitment (dry-run)
    Validate {
        /// Path to commitment JSON file
        file: String,
    },

    /// Show commitment statistics
    Stats,
}

/// Commitment info for display
#[derive(Serialize)]
struct CommitmentInfo {
    id: String,
    status: String,
    resonator_id: String,
    capability: String,
    created_at: String,
    updated_at: String,
}

impl From<&StoredContract> for CommitmentInfo {
    fn from(record: &StoredContract) -> Self {
        Self {
            id: record.contract.commitment_id.0.to_string(),
            status: format!("{:?}", record.status),
            resonator_id: record.contract.principal.to_string(),
            capability: format!("{:?}", record.contract.effect_domain),
            created_at: record.created_at.to_rfc3339(),
            updated_at: record.status_changed_at.to_rfc3339(),
        }
    }
}

/// Execute commitment command
pub async fn execute(command: CommitmentCommands, format: OutputFormat) -> CliResult<()> {
    match command {
        CommitmentCommands::List { status, limit } => list_commitments(status, limit, format),
        CommitmentCommands::Inspect { id } => inspect_commitment(&id, format),
        CommitmentCommands::Lifecycle => show_lifecycle(format),
        CommitmentCommands::Transitions { from } => show_transitions(&from, format),
        CommitmentCommands::Validate { file } => validate_commitment(&file, format),
        CommitmentCommands::Stats => show_stats(format),
    }
}

fn list_commitments(
    status_filter: Option<String>,
    limit: usize,
    format: OutputFormat,
) -> CliResult<()> {
    // In a real implementation, this would connect to a running service
    // For now, we show a demo with empty data
    let engine = InMemoryContractEngine::new();
    let contracts = engine.list_contracts().unwrap_or_default();

    let filtered: Vec<_> = contracts
        .iter()
        .filter(|c| {
            if let Some(ref status) = status_filter {
                format!("{:?}", c.status).to_lowercase().starts_with(&status.to_lowercase())
            } else {
                true
            }
        })
        .take(limit)
        .collect();

    match format {
        OutputFormat::Json => {
            let infos: Vec<CommitmentInfo> = filtered.iter().map(|c| (*c).into()).collect();
            println!("{}", serde_json::to_string_pretty(&infos)?);
        }
        OutputFormat::Yaml => {
            let infos: Vec<CommitmentInfo> = filtered.iter().map(|c| (*c).into()).collect();
            println!("{}", serde_yaml::to_string(&infos)?);
        }
        OutputFormat::Table => {
            if filtered.is_empty() {
                println!("{}", "No commitments found.".dimmed());
                println!();
                println!(
                    "{}: Use 'resonator commitment lifecycle' to see the commitment flow",
                    "Hint".bold()
                );
            } else {
                println!("{}", "Commitments".bold().cyan());
                println!("{}", "=".repeat(80));
                for contract in &filtered {
                    print_contract_row(contract);
                }
                println!();
                println!("Total: {} commitment(s)", filtered.len());
            }
        }
    }

    Ok(())
}

fn print_contract_row(contract: &StoredContract) {
    let status_color = match &contract.status {
        ContractStatus::Draft => "yellow",
        ContractStatus::Proposed => "blue",
        ContractStatus::Accepted => "green",
        ContractStatus::Active => "green",
        ContractStatus::Executing => "cyan",
        ContractStatus::Completed => "white",
        ContractStatus::Failed { .. } => "red",
        ContractStatus::Disputed { .. } => "magenta",
        ContractStatus::Expired => "white",
        ContractStatus::Revoked { .. } | ContractStatus::Rejected { .. } => "white",
        ContractStatus::Suspended { .. } | ContractStatus::Resolved { .. } | ContractStatus::Inactive => "white",
    };

    let id_str = &contract.contract.commitment_id.0;
    let display_id = if id_str.len() > 8 { &id_str[..8] } else { id_str };

    println!(
        "  {} {} {}",
        display_id.bold(),
        format!("[{:?}]", contract.status).color(status_color),
        format!("{:?}", contract.contract.effect_domain).dimmed()
    );
}

fn inspect_commitment(id: &str, format: OutputFormat) -> CliResult<()> {
    // In a real implementation, this would look up the commitment
    match format {
        OutputFormat::Table => {
            println!(
                "{}: Commitment '{}' not found in local store",
                "Error".red().bold(),
                id
            );
            println!();
            println!(
                "{}: Connect to a running Resonator service to inspect commitments",
                "Hint".bold()
            );
        }
        _ => {
            return Err(CliError::NotFound(format!("Commitment {} not found", id)));
        }
    }
    Ok(())
}

fn show_lifecycle(format: OutputFormat) -> CliResult<()> {
    let states = [
        ("Draft", "Initial state, commitment being formed"),
        ("Proposed", "Submitted for approval"),
        ("Accepted", "Approved by authority"),
        ("Active", "Ready for execution"),
        ("Executing", "Currently being executed"),
        ("Completed", "Successfully completed"),
        ("Failed", "Execution failed"),
        ("Disputed", "Under dispute resolution"),
        ("Expired", "Time limit exceeded"),
        ("Revoked", "Revoked by authority"),
    ];

    match format {
        OutputFormat::Json => {
            let json: Vec<_> = states
                .iter()
                .map(|(state, desc)| {
                    serde_json::json!({
                        "state": state,
                        "description": desc
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Yaml => {
            let yaml: Vec<_> = states
                .iter()
                .map(|(state, desc)| {
                    serde_json::json!({
                        "state": state,
                        "description": desc
                    })
                })
                .collect();
            println!("{}", serde_yaml::to_string(&yaml)?);
        }
        OutputFormat::Table => {
            println!("{}", "Commitment Lifecycle States".bold().cyan());
            println!("{}", "=".repeat(60));
            println!();
            println!("  {} → {} → {} → {} → {} → {}",
                "Draft".yellow(),
                "Proposed".blue(),
                "Accepted".green(),
                "Active".green().bold(),
                "Executing".cyan(),
                "Completed".white()
            );
            println!();
            println!("  Alternative endings:");
            println!("    {} (execution error)", "Failed".red());
            println!("    {} (conflict raised)", "Disputed".magenta());
            println!("    {} (time limit exceeded)", "Expired".dimmed());
            println!("    {} (revoked by authority)", "Revoked".dimmed());
            println!();
            for (state, desc) in states {
                println!("  {}: {}", state.bold(), desc.dimmed());
            }
        }
    }

    Ok(())
}

fn show_transitions(from: &str, format: OutputFormat) -> CliResult<()> {
    let transitions = match from.to_lowercase().as_str() {
        "draft" => vec!["Proposed", "Revoked"],
        "proposed" => vec!["Accepted", "Rejected"],
        "accepted" => vec!["Active", "Expired", "Revoked"],
        "active" => vec!["Executing", "Expired", "Suspended", "Revoked"],
        "executing" => vec!["Completed", "Failed", "Disputed"],
        "completed" => vec![],
        "failed" => vec!["Disputed"],
        "disputed" => vec!["Resolved"],
        "expired" => vec![],
        "revoked" | "rejected" => vec![],
        "suspended" => vec!["Active", "Revoked"],
        _ => {
            return Err(CliError::InvalidArgument(format!(
                "Unknown status: {}",
                from
            )));
        }
    };

    match format {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "from": from,
                "valid_transitions": transitions
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Yaml => {
            let yaml = serde_json::json!({
                "from": from,
                "valid_transitions": transitions
            });
            println!("{}", serde_yaml::to_string(&yaml)?);
        }
        OutputFormat::Table => {
            println!(
                "{} from {}:",
                "Valid transitions".bold().cyan(),
                from.bold().yellow()
            );
            println!();
            if transitions.is_empty() {
                println!("  {} (terminal state)", "None".dimmed());
            } else {
                for t in transitions {
                    println!("  → {}", t.green());
                }
            }
        }
    }

    Ok(())
}

fn validate_commitment(file: &str, format: OutputFormat) -> CliResult<()> {
    use std::fs;

    let content = fs::read_to_string(file).map_err(|e| {
        CliError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Cannot read file '{}': {}", file, e),
        ))
    })?;

    // Try to parse as RcfCommitment
    let result: Result<rcf_commitment::RcfCommitment, _> = serde_json::from_str(&content);

    match format {
        OutputFormat::Json => {
            let json = match result {
                Ok(_) => serde_json::json!({
                    "valid": true,
                    "file": file,
                    "errors": []
                }),
                Err(e) => serde_json::json!({
                    "valid": false,
                    "file": file,
                    "errors": [e.to_string()]
                }),
            };
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Yaml => {
            let yaml = match result {
                Ok(_) => serde_json::json!({
                    "valid": true,
                    "file": file,
                    "errors": []
                }),
                Err(e) => serde_json::json!({
                    "valid": false,
                    "file": file,
                    "errors": [e.to_string()]
                }),
            };
            println!("{}", serde_yaml::to_string(&yaml)?);
        }
        OutputFormat::Table => match result {
            Ok(commitment) => {
                println!("{} Commitment is valid", "✓".green().bold());
                println!();
                println!("  Principal: {}", commitment.principal);
                println!("  Effect Domain: {:?}", commitment.effect_domain);
                println!(
                    "  Capabilities required: {}",
                    commitment.required_capabilities.len()
                );
            }
            Err(e) => {
                println!("{} Commitment validation failed", "✗".red().bold());
                println!();
                println!("  Error: {}", e);
            }
        },
    }

    Ok(())
}

fn show_stats(format: OutputFormat) -> CliResult<()> {
    // In a real implementation, this would get stats from the service
    let stats = serde_json::json!({
        "total_commitments": 0,
        "by_status": {
            "draft": 0,
            "proposed": 0,
            "accepted": 0,
            "active": 0,
            "executing": 0,
            "completed": 0,
            "failed": 0,
            "disputed": 0
        },
        "avg_completion_time_ms": null,
        "success_rate": null
    });

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&stats)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&stats)?);
        }
        OutputFormat::Table => {
            println!("{}", "Commitment Statistics".bold().cyan());
            println!("{}", "=".repeat(40));
            println!();
            println!("  Total commitments: {}", "0".dimmed());
            println!();
            println!(
                "{}: Connect to a running Resonator service to see statistics",
                "Note".bold()
            );
        }
    }

    Ok(())
}
