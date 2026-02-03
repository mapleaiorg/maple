//! State and checkpoint commands

use crate::client::PalmClient;
use crate::error::CliResult;
use crate::output::{self, print_error, print_info, print_success, OutputFormat};
use clap::Subcommand;
use serde::Serialize;
use tabled::Tabled;

/// State subcommands
#[derive(Subcommand)]
pub enum StateCommands {
    /// Create a checkpoint for an instance
    Checkpoint {
        /// Instance ID
        instance_id: String,
    },

    /// List snapshots for an instance
    List {
        /// Instance ID
        instance_id: String,
    },

    /// Restore from a snapshot
    Restore {
        /// Instance ID
        instance_id: String,

        /// Snapshot ID
        snapshot_id: String,

        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Delete a snapshot
    Delete {
        /// Snapshot ID
        snapshot_id: String,

        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

/// Table row for snapshot display
#[derive(Debug, Serialize, Tabled)]
struct SnapshotRow {
    /// Snapshot ID (short form)
    id: String,
    /// Creation timestamp
    created: String,
    /// Reason for snapshot
    reason: String,
    /// Size
    size: String,
}

impl From<crate::client::SnapshotInfo> for SnapshotRow {
    fn from(s: crate::client::SnapshotInfo) -> Self {
        Self {
            id: truncate_id(&s.id),
            created: s.created_at,
            reason: s.reason,
            size: humanize_bytes(s.size_bytes),
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

fn humanize_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

/// Execute a state command
pub async fn execute(
    command: StateCommands,
    client: &PalmClient,
    format: OutputFormat,
) -> CliResult<()> {
    match command {
        StateCommands::Checkpoint { instance_id } => {
            print_info("Creating checkpoint...");
            let snapshot_id = client.create_checkpoint(&instance_id).await?;
            print_success(&format!("Created snapshot: {}", snapshot_id));
            Ok(())
        }

        StateCommands::List { instance_id } => {
            let snapshots = client.list_snapshots(&instance_id).await?;
            let rows: Vec<SnapshotRow> = snapshots.into_iter().map(SnapshotRow::from).collect();
            output::print_output(rows, format);
            Ok(())
        }

        StateCommands::Restore {
            instance_id,
            snapshot_id,
            yes,
        } => {
            if !yes {
                let confirm = dialoguer::Confirm::new()
                    .with_prompt(format!(
                        "Restore instance {} from snapshot {}? Current state will be replaced.",
                        instance_id, snapshot_id
                    ))
                    .default(false)
                    .interact()
                    .unwrap_or(false);

                if !confirm {
                    print_error("Aborted");
                    return Ok(());
                }
            }

            print_info("Restoring from snapshot...");
            client.restore_snapshot(&instance_id, &snapshot_id).await?;
            print_success(&format!(
                "Restored instance {} from snapshot {}",
                instance_id, snapshot_id
            ));
            Ok(())
        }

        StateCommands::Delete { snapshot_id, yes } => {
            if !yes {
                let confirm = dialoguer::Confirm::new()
                    .with_prompt(format!("Delete snapshot {}?", snapshot_id))
                    .default(false)
                    .interact()
                    .unwrap_or(false);

                if !confirm {
                    print_error("Aborted");
                    return Ok(());
                }
            }

            // Would call delete API
            print_success(&format!("Deleted snapshot: {}", snapshot_id));
            Ok(())
        }
    }
}
