//! PALM Daemon - Background orchestration service
//!
//! The PALM daemon provides:
//! - REST API for spec/deployment/instance management
//! - Reconciliation loop for maintaining desired state
//! - Health monitoring and auto-healing
//! - Event streaming for observability

use clap::Parser;
use palm_types::PlatformProfile;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod config;
mod error;
mod playground;
mod scheduler;
mod server;
mod storage;

use config::DaemonConfig;
use error::DaemonResult;
use server::Server;

/// PALM Daemon CLI
#[derive(Parser)]
#[command(name = "palmd")]
#[command(about = "PALM Daemon - Background orchestration service", long_about = None)]
#[command(version)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, env = "PALM_CONFIG")]
    config: Option<String>,

    /// Listen address
    #[arg(
        short,
        long,
        env = "PALM_LISTEN_ADDR",
        default_value = "127.0.0.1:8080"
    )]
    listen: String,

    /// Platform profile
    #[arg(short, long, env = "PALM_PLATFORM", default_value = "development")]
    platform: String,

    /// Log level
    #[arg(long, env = "PALM_LOG_LEVEL", default_value = "info")]
    log_level: String,

    /// Enable JSON logging
    #[arg(long, env = "PALM_LOG_JSON")]
    json: bool,
}

#[tokio::main]
async fn main() -> DaemonResult<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| cli.log_level.clone().into());

    if cli.json {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    // Load configuration
    let mut config = DaemonConfig::load(cli.config.as_deref())
        .map_err(|e| error::DaemonError::Config(e.to_string()))?;

    // Override with CLI args
    config.server.listen_addr = cli
        .listen
        .parse()
        .map_err(|e| error::DaemonError::Config(format!("Invalid listen address: {}", e)))?;

    config.platform = match cli.platform.to_lowercase().as_str() {
        "mapleverse" => PlatformProfile::Mapleverse,
        "finalverse" => PlatformProfile::Finalverse,
        "ibank" => PlatformProfile::IBank,
        "development" | "dev" => PlatformProfile::Development,
        other => {
            return Err(error::DaemonError::Config(format!(
                "Unknown platform profile: {}",
                other
            )));
        }
    };

    // Print startup banner
    println!(
        r#"
  ____   _    _     __  __
 |  _ \ / \  | |   |  \/  |
 | |_) / _ \ | |   | |\/| |
 |  __/ ___ \| |___| |  | |
 |_| /_/   \_\_____|_|  |_|

  MapleAI - Platform Agent Lifecycle Manager
  Version: {}
  Platform: {:?}
  Listening: {}
"#,
        env!("CARGO_PKG_VERSION"),
        config.platform,
        config.server.listen_addr
    );

    // Create and run server
    let server = Server::new(config).await?;
    server.run().await
}
