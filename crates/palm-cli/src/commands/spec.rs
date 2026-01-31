//! Agent specification commands

use crate::client::PalmClient;
use crate::error::CliResult;
use crate::output::{self, print_error, print_success, OutputFormat};
use clap::Subcommand;
use palm_types::*;
use serde::Serialize;
use tabled::Tabled;

/// Spec subcommands
#[derive(Subcommand)]
pub enum SpecCommands {
    /// Register a new agent specification
    Register {
        /// Path to spec file (YAML or JSON)
        #[arg(short, long)]
        file: String,
    },

    /// Get details of a spec
    Get {
        /// Spec ID
        spec_id: String,
    },

    /// List all specs
    List {
        /// Filter by name
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Deprecate a spec version
    Deprecate {
        /// Spec ID
        spec_id: String,

        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

/// Table row for spec display
#[derive(Debug, Serialize, Tabled)]
struct SpecRow {
    /// Spec ID (short form)
    id: String,
    /// Spec name
    name: String,
    /// Version
    version: String,
    /// Target platform
    platform: String,
    /// Created timestamp
    created: String,
}

impl From<AgentSpec> for SpecRow {
    fn from(spec: AgentSpec) -> Self {
        Self {
            id: spec.id.to_string(),
            name: spec.name,
            version: spec.version.to_string(),
            platform: format!("{:?}", spec.platform),
            created: spec.created_at.format("%Y-%m-%d %H:%M").to_string(),
        }
    }
}

/// Execute a spec command
pub async fn execute(
    command: SpecCommands,
    client: &PalmClient,
    format: OutputFormat,
) -> CliResult<()> {
    match command {
        SpecCommands::Register { file } => {
            let contents = std::fs::read_to_string(&file)?;
            let spec: AgentSpec = if file.ends_with(".yaml") || file.ends_with(".yml") {
                serde_yaml::from_str(&contents)?
            } else {
                serde_json::from_str(&contents)?
            };

            let spec_id = client.register_spec(&spec).await?;
            print_success(&format!("Registered spec: {}", spec_id));
            Ok(())
        }

        SpecCommands::Get { spec_id } => {
            let spec = client.get_spec(&spec_id).await?;
            output::print_single(&spec, format);
            Ok(())
        }

        SpecCommands::List { name } => {
            let specs = client.list_specs().await?;
            let filtered: Vec<SpecRow> = specs
                .into_iter()
                .filter(|s| name.as_ref().map(|n| s.name.contains(n)).unwrap_or(true))
                .map(SpecRow::from)
                .collect();
            output::print_output(filtered, format);
            Ok(())
        }

        SpecCommands::Deprecate { spec_id, yes } => {
            if !yes {
                let confirm = dialoguer::Confirm::new()
                    .with_prompt(format!("Deprecate spec {}?", spec_id))
                    .default(false)
                    .interact()
                    .unwrap_or(false);

                if !confirm {
                    print_error("Aborted");
                    return Ok(());
                }
            }

            client.deprecate_spec(&spec_id).await?;
            print_success(&format!("Deprecated spec: {}", spec_id));
            Ok(())
        }
    }
}
