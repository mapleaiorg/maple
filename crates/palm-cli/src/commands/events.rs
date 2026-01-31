//! Event streaming commands

use crate::client::PalmClient;
use crate::error::CliResult;
use crate::output::print_info;
use clap::Subcommand;
use colored::*;
use futures_util::StreamExt;
use palm_types::*;

/// Event subcommands
#[derive(Subcommand)]
pub enum EventCommands {
    /// Stream live events
    Watch {
        /// Filter by event type
        #[arg(short, long)]
        filter: Option<String>,

        /// Filter by deployment ID
        #[arg(short, long)]
        deployment: Option<String>,
    },

    /// Show recent events
    Recent {
        /// Number of events to show
        #[arg(short, long, default_value = "20")]
        count: u32,
    },
}

/// Execute an event command
pub async fn execute(command: EventCommands, client: &PalmClient) -> CliResult<()> {
    match command {
        EventCommands::Watch {
            filter: _,
            deployment: _,
        } => {
            print_info("Watching events... (Ctrl+C to stop)");
            println!();

            let mut stream = client.stream_events().await?;

            while let Some(result) = stream.next().await {
                match result {
                    Ok(event) => {
                        print_event(&event);
                    }
                    Err(e) => {
                        eprintln!("{} Stream error: {}", "✗".red(), e);
                    }
                }
            }

            Ok(())
        }

        EventCommands::Recent { count } => {
            // Would fetch recent events from daemon
            print_info(&format!("Showing last {} events", count));
            println!("(No events available - daemon not connected)");
            Ok(())
        }
    }
}

fn print_event(envelope: &PalmEventEnvelope) {
    let severity_color = match envelope.severity {
        EventSeverity::Debug => "DEBUG".dimmed(),
        EventSeverity::Info => "INFO".blue(),
        EventSeverity::Warning => "WARN".yellow(),
        EventSeverity::Error => "ERROR".red(),
        EventSeverity::Critical => "CRIT".red().bold(),
    };

    let time = envelope.timestamp.format("%H:%M:%S");
    let source = format!("{:?}", envelope.source);

    println!(
        "{} {} [{}] {:?}",
        time.to_string().dimmed(),
        severity_color,
        source,
        envelope.event
    );
}
