//! Consequence tracking commands

use crate::error::{CliError, CliResult};
use crate::output::OutputFormat;
use clap::Subcommand;
use colored::Colorize;
use resonator_consequence::{ConsequenceStore, InMemoryConsequenceStore, RecordedConsequence};
use serde::Serialize;

/// Consequence subcommands
#[derive(Subcommand)]
pub enum ConsequenceCommands {
    /// List recorded consequences
    List {
        /// Filter by commitment ID
        #[arg(short, long)]
        commitment: Option<String>,
        /// Maximum number to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Inspect a specific consequence
    Inspect {
        /// Consequence ID
        id: String,
    },

    /// Verify a consequence receipt
    Verify {
        /// Receipt signature to verify
        signature: String,
    },

    /// Show consequence types
    Types,

    /// Show invariant #4 enforcement details
    Invariant4,

    /// Show consequence statistics
    Stats,
}

/// Consequence info for display
#[derive(Serialize)]
struct ConsequenceInfo {
    id: String,
    commitment_id: String,
    consequence_type: String,
    severity: String,
    description: String,
    status: String,
}

impl From<&RecordedConsequence> for ConsequenceInfo {
    fn from(record: &RecordedConsequence) -> Self {
        Self {
            id: record.id.0.clone(),
            commitment_id: record.request.commitment_id.0.clone(),
            consequence_type: format!("{:?}", record.request.consequence_type),
            severity: format!("{:?}", record.request.severity),
            description: record.request.description.clone(),
            status: format!("{:?}", record.status),
        }
    }
}

/// Execute consequence command
pub async fn execute(command: ConsequenceCommands, format: OutputFormat) -> CliResult<()> {
    match command {
        ConsequenceCommands::List { commitment, limit } => {
            list_consequences(commitment, limit, format)
        }
        ConsequenceCommands::Inspect { id } => inspect_consequence(&id, format),
        ConsequenceCommands::Verify { signature } => verify_receipt(&signature, format),
        ConsequenceCommands::Types => show_types(format),
        ConsequenceCommands::Invariant4 => show_invariant4(format),
        ConsequenceCommands::Stats => show_stats(format),
    }
}

fn list_consequences(
    commitment_filter: Option<String>,
    limit: usize,
    format: OutputFormat,
) -> CliResult<()> {
    // In a real implementation, this would connect to a running service
    let store = InMemoryConsequenceStore::new();

    let consequences: Vec<RecordedConsequence> = if let Some(commitment_id) = commitment_filter {
        let cid = rcf_commitment::CommitmentId(commitment_id);
        store.list_by_commitment(&cid).unwrap_or_default()
    } else {
        // Would need a list_all method in real implementation
        vec![]
    };

    let limited: Vec<_> = consequences.iter().take(limit).collect();

    match format {
        OutputFormat::Json => {
            let infos: Vec<ConsequenceInfo> = limited.iter().map(|c| (*c).into()).collect();
            println!("{}", serde_json::to_string_pretty(&infos)?);
        }
        OutputFormat::Yaml => {
            let infos: Vec<ConsequenceInfo> = limited.iter().map(|c| (*c).into()).collect();
            println!("{}", serde_yaml::to_string(&infos)?);
        }
        OutputFormat::Table => {
            if limited.is_empty() {
                println!("{}", "No consequences found.".dimmed());
                println!();
                println!(
                    "{}: Consequences are only recorded after commitment execution",
                    "Note".bold()
                );
            } else {
                println!("{}", "Recorded Consequences".bold().cyan());
                println!("{}", "=".repeat(80));
                for consequence in &limited {
                    print_consequence_row(consequence);
                }
                println!();
                println!("Total: {} consequence(s)", limited.len());
            }
        }
    }

    Ok(())
}

fn print_consequence_row(consequence: &RecordedConsequence) {
    let id_str = &consequence.id.0;
    let display_id = if id_str.len() > 8 {
        &id_str[..8]
    } else {
        id_str
    };

    let cid_str = &consequence.request.commitment_id.0;
    let display_cid = if cid_str.len() > 8 {
        &cid_str[..8]
    } else {
        cid_str
    };

    println!(
        "  {} {} {}",
        display_id.bold(),
        format!("{:?}", consequence.request.consequence_type).cyan(),
        format!("({})", display_cid).dimmed()
    );
}

fn inspect_consequence(id: &str, format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Table => {
            println!(
                "{}: Consequence '{}' not found in local store",
                "Error".red().bold(),
                id
            );
            println!();
            println!(
                "{}: Connect to a running Resonator service to inspect consequences",
                "Hint".bold()
            );
        }
        _ => {
            return Err(CliError::NotFound(format!("Consequence {} not found", id)));
        }
    }
    Ok(())
}

fn verify_receipt(signature: &str, format: OutputFormat) -> CliResult<()> {
    // In a real implementation, this would verify against stored receipts
    match format {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "signature": signature,
                "valid": false,
                "reason": "Receipt not found in local store"
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Yaml => {
            let yaml = serde_json::json!({
                "signature": signature,
                "valid": false,
                "reason": "Receipt not found in local store"
            });
            println!("{}", serde_yaml::to_string(&yaml)?);
        }
        OutputFormat::Table => {
            println!("{}", "Receipt Verification".bold().cyan());
            println!();
            println!("  Signature: {}...", &signature[..signature.len().min(32)]);
            println!("  Status: {}", "Not Found".yellow());
            println!();
            println!(
                "{}: Connect to a running Resonator service to verify receipts",
                "Hint".bold()
            );
        }
    }

    Ok(())
}

fn show_types(format: OutputFormat) -> CliResult<()> {
    let types = [
        (
            "Computation",
            "Computation-only effect (no external side effects)",
            true,
            "Low",
        ),
        (
            "DataMutation",
            "Modification to persistent data",
            false,
            "Medium",
        ),
        (
            "Financial",
            "Movement of financial value",
            false,
            "Critical",
        ),
        (
            "Communication",
            "Message sent to external system",
            false,
            "High",
        ),
        (
            "ExternalSystem",
            "Interaction with external system",
            false,
            "High",
        ),
    ];

    match format {
        OutputFormat::Json => {
            let json: Vec<_> = types
                .iter()
                .map(|(name, desc, reversible, severity)| {
                    serde_json::json!({
                        "type": name,
                        "description": desc,
                        "reversible": reversible,
                        "severity": severity
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Yaml => {
            let yaml: Vec<_> = types
                .iter()
                .map(|(name, desc, reversible, severity)| {
                    serde_json::json!({
                        "type": name,
                        "description": desc,
                        "reversible": reversible,
                        "severity": severity
                    })
                })
                .collect();
            println!("{}", serde_yaml::to_string(&yaml)?);
        }
        OutputFormat::Table => {
            println!("{}", "Consequence Types".bold().cyan());
            println!("{}", "=".repeat(70));
            println!();
            for (name, desc, reversible, severity) in types {
                let reversible_str = if reversible {
                    "↩ Reversible".green()
                } else {
                    "⬤ Permanent".red()
                };
                let severity_color = match severity {
                    "Critical" => "red",
                    "High" => "yellow",
                    "Medium" => "blue",
                    _ => "white",
                };
                println!(
                    "  {} {} [{}]",
                    name.bold(),
                    reversible_str,
                    severity.color(severity_color)
                );
                println!("      {}", desc.dimmed());
            }
        }
    }

    Ok(())
}

fn show_invariant4(format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "invariant": 4,
                "name": "Commitment precedes Consequence",
                "enforcement": "compile-time",
                "mechanism": "ConsequenceTracker requires valid CommitmentId",
                "violation_impossible": true,
                "details": {
                    "flow": [
                        "1. Commitment drafted via ContractEngine",
                        "2. Commitment approved and activated",
                        "3. ConsequenceTracker.record() requires commitment_id",
                        "4. Tracker validates commitment exists and is Active/Executing",
                        "5. Only then is consequence recorded with receipt"
                    ]
                }
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Yaml => {
            let yaml = serde_json::json!({
                "invariant": 4,
                "name": "Commitment precedes Consequence",
                "enforcement": "compile-time",
                "mechanism": "ConsequenceTracker requires valid CommitmentId"
            });
            println!("{}", serde_yaml::to_string(&yaml)?);
        }
        OutputFormat::Table => {
            println!(
                "{}",
                "Invariant #4: Commitment precedes Consequence"
                    .bold()
                    .cyan()
            );
            println!("{}", "=".repeat(60));
            println!();
            println!(
                "  {}",
                "Every consequence MUST reference a valid commitment.".bold()
            );
            println!();
            println!(
                "  Enforcement: {} (Rust type system)",
                "Compile-time".green()
            );
            println!();
            println!("  Flow:");
            println!(
                "    {} Commitment drafted via ContractEngine",
                "1.".yellow()
            );
            println!("    {} Commitment approved and activated", "2.".yellow());
            println!(
                "    {} ConsequenceTracker.record() requires CommitmentId",
                "3.".yellow()
            );
            println!(
                "    {} Tracker validates commitment is Active/Executing",
                "4.".yellow()
            );
            println!(
                "    {} Consequence recorded with cryptographic receipt",
                "5.".yellow()
            );
            println!();
            println!(
                "  Without step 1-2, step 3-5 {} at compile time.",
                "cannot execute".red().bold()
            );
        }
    }

    Ok(())
}

fn show_stats(format: OutputFormat) -> CliResult<()> {
    let stats = serde_json::json!({
        "total_consequences": 0,
        "by_type": {},
        "by_severity": {},
        "pending_count": 0,
        "completed_count": 0,
        "failed_count": 0
    });

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&stats)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&stats)?);
        }
        OutputFormat::Table => {
            println!("{}", "Consequence Statistics".bold().cyan());
            println!("{}", "=".repeat(40));
            println!();
            println!("  Total consequences: {}", "0".dimmed());
            println!("  Pending: {}", "0".dimmed());
            println!("  Completed: {}", "0".dimmed());
            println!("  Failed: {}", "0".dimmed());
            println!();
            println!(
                "{}: Connect to a running Resonator service to see statistics",
                "Note".bold()
            );
        }
    }

    Ok(())
}
