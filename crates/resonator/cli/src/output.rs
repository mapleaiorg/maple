//! Output formatting for CLI

use clap::ValueEnum;

/// Output format for CLI commands
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum OutputFormat {
    /// Human-readable table format
    #[default]
    Table,
    /// JSON format
    Json,
    /// YAML format
    Yaml,
}
