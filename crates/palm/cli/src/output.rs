//! Output formatting utilities

use colored::*;
use serde::Serialize;
use tabled::{Table, Tabled};

/// Output format for CLI commands
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OutputFormat {
    /// Pretty-printed table format
    Table,
    /// JSON format
    Json,
    /// YAML format
    Yaml,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Table
    }
}

/// Print a vector of items in the specified format
pub fn print_output<T: Serialize + Tabled>(data: Vec<T>, format: OutputFormat) {
    match format {
        OutputFormat::Table => {
            if data.is_empty() {
                println!("{}", "No results".dimmed());
            } else {
                let table = Table::new(data).to_string();
                println!("{}", table);
            }
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&data).unwrap());
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&data).unwrap());
        }
    }
}

/// Print a single item in the specified format
pub fn print_single<T: Serialize>(data: &T, format: OutputFormat) {
    match format {
        OutputFormat::Table | OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(data).unwrap());
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(data).unwrap());
        }
    }
}

/// Print a success message
pub fn print_success(message: &str) {
    println!("{} {}", "✓".green(), message);
}

/// Print an error message
pub fn print_error(message: &str) {
    eprintln!("{} {}", "✗".red(), message);
}

/// Print a warning message
pub fn print_warning(message: &str) {
    println!("{} {}", "⚠".yellow(), message);
}

/// Print an info message
pub fn print_info(message: &str) {
    println!("{} {}", "ℹ".blue(), message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_default() {
        let format = OutputFormat::default();
        assert!(matches!(format, OutputFormat::Table));
    }
}
