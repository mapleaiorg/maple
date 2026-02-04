use clap::{Parser, ValueEnum};
use ibank_core::LedgerStorageConfig;
use ibank_service::{build_router, grpc::serve_grpc, ServiceConfig, ServiceState};
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum LedgerStorageMode {
    Auto,
    Memory,
    Postgres,
}

#[derive(Debug, Parser)]
#[command(name = "ibankd", version, about = "iBank Core REST service")]
struct Cli {
    /// REST socket address to bind, e.g. 127.0.0.1:8091
    #[arg(long, default_value = "127.0.0.1:8091")]
    listen: SocketAddr,
    /// gRPC socket address to bind, e.g. 127.0.0.1:50051
    #[arg(long, default_value = "127.0.0.1:50051")]
    grpc_listen: SocketAddr,
    /// Disable gRPC server and run REST only.
    #[arg(long, default_value_t = false)]
    no_grpc: bool,
    /// File used to persist pending hybrid approvals.
    #[arg(long, default_value = "ibank/data/approvals.json")]
    approval_queue: PathBuf,
    /// Ledger persistence backend. `auto` picks postgres when database url is configured.
    #[arg(long, value_enum, default_value_t = LedgerStorageMode::Auto, env = "IBANK_LEDGER_STORAGE")]
    ledger_storage: LedgerStorageMode,
    /// PostgreSQL url for commitment/audit/outcome ledger persistence.
    #[arg(long, env = "IBANK_LEDGER_DATABASE_URL")]
    ledger_database_url: Option<String>,
    /// Max PostgreSQL pool connections for ledger persistence.
    #[arg(long, default_value_t = 5, env = "IBANK_LEDGER_PG_MAX_CONNECTIONS")]
    ledger_pg_max_connections: u32,
}

fn resolve_ledger_storage(cli: &Cli) -> anyhow::Result<LedgerStorageConfig> {
    let resolved_url = cli
        .ledger_database_url
        .clone()
        .or_else(|| std::env::var("DATABASE_URL").ok());

    let storage = match cli.ledger_storage {
        LedgerStorageMode::Memory => LedgerStorageConfig::Memory,
        LedgerStorageMode::Postgres => {
            let database_url = resolved_url.ok_or_else(|| {
                anyhow::anyhow!(
                    "ledger_storage=postgres requires --ledger-database-url or DATABASE_URL"
                )
            })?;
            LedgerStorageConfig::postgres(database_url, cli.ledger_pg_max_connections)
        }
        LedgerStorageMode::Auto => {
            if let Some(database_url) = resolved_url {
                LedgerStorageConfig::postgres(database_url, cli.ledger_pg_max_connections)
            } else {
                LedgerStorageConfig::Memory
            }
        }
    };

    Ok(storage)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "ibank_service=info,info".to_string()),
        )
        .init();

    let cli = Cli::parse();
    let ledger_storage = resolve_ledger_storage(&cli)?;
    let config = ServiceConfig {
        queue_path: cli.approval_queue,
        ledger_storage,
    };
    let state = ServiceState::bootstrap(config).await?;
    let app = build_router(state.clone());

    let listener = tokio::net::TcpListener::bind(cli.listen).await?;
    info!("ibank-service REST listening on {}", listener.local_addr()?);

    let rest_task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .map_err(anyhow::Error::from)
    });

    let grpc_task = if cli.no_grpc {
        None
    } else {
        let grpc_state = state.clone();
        let grpc_addr = cli.grpc_listen;
        info!("ibank-service gRPC listening on {}", grpc_addr);
        Some(tokio::spawn(async move {
            serve_grpc(grpc_state, grpc_addr).await
        }))
    };

    if let Some(grpc_task) = grpc_task {
        tokio::select! {
            rest = rest_task => rest??,
            grpc = grpc_task => grpc??,
        }
    } else {
        rest_task.await??;
    }

    Ok(())
}
