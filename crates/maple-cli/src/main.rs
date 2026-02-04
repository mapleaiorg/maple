use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Parser)]
#[command(name = "maple", about = "MAPLE AI Framework CLI")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show version information
    Version,

    /// Validate a local file (developer utility)
    Validate {
        #[arg(short, long)]
        file: String,
    },

    /// UAL parsing and compilation
    Ual {
        #[command(subcommand)]
        command: UalCommands,
    },

    /// Run local environment diagnostics for MAPLE/PALM
    Doctor(DoctorArgs),

    /// Manage PALM daemon lifecycle
    Daemon {
        #[command(subcommand)]
        command: DaemonCommands,
    },

    /// PALM operations (forwarded to palm)
    #[command(alias = "ops")]
    Palm {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },

    /// Direct PALM operations shortcut (e.g. `maple spec list`)
    #[command(external_subcommand)]
    PalmShortcut(Vec<OsString>),
}

#[derive(Subcommand)]
enum UalCommands {
    /// Parse UAL into an AST
    Parse {
        #[arg(short, long)]
        file: String,
    },
    /// Compile UAL into RCF and PALM operations
    Compile {
        #[arg(short, long)]
        file: String,
    },
    /// Validate UAL commitments (RCF validation)
    Validate {
        #[arg(short, long)]
        file: String,
    },
}

#[derive(Args)]
struct DoctorArgs {
    /// PALM daemon endpoint
    #[arg(long, env = "PALM_ENDPOINT", default_value = "http://localhost:8080")]
    endpoint: String,

    /// Local Ollama endpoint
    #[arg(long, env = "OLLAMA_HOST", default_value = "http://127.0.0.1:11434")]
    ollama_endpoint: String,

    /// Expected local model name (defaults to active playground model if available)
    #[arg(long)]
    model: Option<String>,
}

#[derive(Subcommand)]
enum DaemonCommands {
    /// Start PALM daemon
    Start {
        /// Target platform profile
        #[arg(long, default_value = "development")]
        platform: String,

        /// Storage backend override
        #[arg(long)]
        storage: Option<StorageKindArg>,

        /// Start in foreground (blocks current terminal)
        #[arg(long)]
        foreground: bool,

        /// Optional explicit daemon binary path
        #[arg(long)]
        daemon_bin: Option<String>,
    },

    /// Stop PALM daemon (graceful API shutdown, then PID fallback)
    Stop {
        /// PALM daemon endpoint
        #[arg(long, env = "PALM_ENDPOINT", default_value = "http://localhost:8080")]
        endpoint: String,
    },

    /// Alias for `stop`
    Shutdown {
        /// PALM daemon endpoint
        #[arg(long, env = "PALM_ENDPOINT", default_value = "http://localhost:8080")]
        endpoint: String,
    },

    /// Show PALM daemon status
    Status {
        /// PALM daemon endpoint
        #[arg(long, env = "PALM_ENDPOINT", default_value = "http://localhost:8080")]
        endpoint: String,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum StorageKindArg {
    Memory,
    Postgres,
}

impl StorageKindArg {
    fn as_env(self) -> &'static str {
        match self {
            StorageKindArg::Memory => "memory",
            StorageKindArg::Postgres => "postgres",
        }
    }
}

#[derive(Debug, Deserialize)]
struct DoctorHealthResponse {
    status: String,
    version: String,
    uptime: String,
}

#[derive(Debug, Deserialize)]
struct PlaygroundConfigResponse {
    ai_backend: PlaygroundBackendResponse,
}

#[derive(Debug, Deserialize)]
struct PlaygroundBackendResponse {
    kind: String,
    model: String,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaTagModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaTagModel {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DaemonPidFile {
    pid: u32,
    command: String,
    started_at_epoch_secs: u64,
    log_file: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Version => {
            println!("Maple AI Framework v{}", env!("CARGO_PKG_VERSION"));
            println!("\nResonance Architecture - Intelligence free to reason, action bound by obligation.");
        }
        Commands::Validate { file } => println!("Validating: {}", file),
        Commands::Ual { command } => {
            if let Err(err) = handle_ual(command) {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        Commands::Doctor(args) => {
            if let Err(err) = handle_doctor(args).await {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        Commands::Daemon { command } => {
            if let Err(err) = handle_daemon(command).await {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        Commands::Palm { args } => {
            let mut forwarded = Vec::with_capacity(args.len() + 1);
            forwarded.push(OsString::from("palm"));
            forwarded.extend(args);

            if let Err(err) = palm::run_with_args(forwarded).await {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        Commands::PalmShortcut(args) => {
            let mut forwarded = Vec::with_capacity(args.len() + 1);
            forwarded.push(OsString::from("palm"));
            forwarded.extend(args);

            if let Err(err) = palm::run_with_args(forwarded).await {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
    }
}

async fn handle_doctor(args: DoctorArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut failed = 0usize;
    let client = build_http_client()?;

    println!("MAPLE Doctor");
    println!("Endpoint: {}", args.endpoint);
    println!("Ollama:   {}", args.ollama_endpoint);
    println!();

    match check_daemon_health(&client, &args.endpoint).await {
        Ok(health) => {
            print_ok(&format!(
                "PALM daemon reachable (status={}, version={}, uptime={})",
                health.status, health.version, health.uptime
            ));
        }
        Err(err) => {
            failed += 1;
            print_fail(&format!("PALM daemon check failed: {}", err));
        }
    }

    let storage_type = std::env::var("PALM_STORAGE_TYPE").unwrap_or_else(|_| "postgres".to_string());
    match storage_type.as_str() {
        "memory" => {
            print_warn("Storage mode is memory (non-persistent)");
        }
        "postgres" => {
            let pg_url = std::env::var("PALM_STORAGE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/maple".to_string());
            match check_postgres_reachable(&pg_url).await {
                Ok((host, port)) => {
                    print_ok(&format!("PostgreSQL reachable at {}:{}", host, port));
                }
                Err(err) => {
                    failed += 1;
                    print_fail(&format!("PostgreSQL check failed: {}", err));
                }
            }
        }
        other => {
            print_warn(&format!(
                "Unknown PALM_STORAGE_TYPE='{}' (expected memory|postgres)",
                other
            ));
        }
    }

    let (ollama_models, ollama_error) = match fetch_ollama_models(&client, &args.ollama_endpoint).await
    {
        Ok(models) => {
            print_ok(&format!(
                "Ollama reachable with {} local model(s)",
                models.len()
            ));
            (models, None)
        }
        Err(err) => {
            failed += 1;
            print_fail(&format!("Ollama check failed: {}", err));
            (Vec::new(), Some(err))
        }
    };

    if ollama_error.is_none() {
        let expected_model = match args.model {
            Some(model) => Some(model),
            None => match fetch_playground_backend(&client, &args.endpoint).await {
                Ok(Some((kind, model))) => {
                    print_ok(&format!(
                        "Playground backend configured: kind={}, model={}",
                        kind, model
                    ));
                    Some(model)
                }
                Ok(None) => {
                    print_warn("Playground backend not available (daemon unreachable or endpoint missing)");
                    None
                }
                Err(err) => {
                    print_warn(&format!("Could not read playground backend config: {}", err));
                    None
                }
            },
        };

        let expected_model = expected_model.unwrap_or_else(|| "llama3".to_string());
        if ollama_models
            .iter()
            .any(|name| model_matches(name, &expected_model))
        {
            print_ok(&format!(
                "Required model '{}' is available in Ollama",
                expected_model
            ));
        } else {
            failed += 1;
            print_fail(&format!(
                "Model '{}' not found in Ollama. Run: `ollama pull {}`",
                expected_model, expected_model
            ));
        }
    }

    println!();
    if failed == 0 {
        print_ok("All checks passed");
        Ok(())
    } else {
        Err(format!("Doctor found {} failing check(s)", failed).into())
    }
}

async fn handle_daemon(command: DaemonCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        DaemonCommands::Start {
            platform,
            storage,
            foreground,
            daemon_bin,
        } => daemon_start(&platform, storage, foreground, daemon_bin).await,
        DaemonCommands::Stop { endpoint } | DaemonCommands::Shutdown { endpoint } => {
            daemon_stop(&endpoint).await
        }
        DaemonCommands::Status { endpoint } => daemon_status(&endpoint).await,
    }
}

fn build_http_client() -> Result<Client, Box<dyn std::error::Error>> {
    let allow_system_proxy = std::env::var("PALM_USE_SYSTEM_PROXY")
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes"))
        .unwrap_or(false);
    let mut builder = Client::builder().timeout(Duration::from_secs(4));
    if !allow_system_proxy {
        builder = builder.no_proxy();
    }
    Ok(builder.build()?)
}

async fn check_daemon_health(
    client: &Client,
    endpoint: &str,
) -> Result<DoctorHealthResponse, Box<dyn std::error::Error>> {
    let url = format!("{}/health", endpoint.trim_end_matches('/'));
    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        return Err(format!("health endpoint returned {}", resp.status()).into());
    }
    Ok(resp.json::<DoctorHealthResponse>().await?)
}

async fn check_postgres_reachable(
    pg_url: &str,
) -> Result<(String, u16), Box<dyn std::error::Error>> {
    let url = reqwest::Url::parse(pg_url)
        .map_err(|e| format!("invalid PALM_STORAGE_URL '{}': {}", pg_url, e))?;
    let host = url
        .host_str()
        .ok_or_else(|| format!("no host found in PALM_STORAGE_URL '{}'", pg_url))?
        .to_string();
    let port = url.port().unwrap_or(5432);

    let connect_future = tokio::net::TcpStream::connect((host.as_str(), port));
    tokio::time::timeout(Duration::from_secs(3), connect_future)
        .await
        .map_err(|_| "connection timeout".to_string())?
        .map_err(|e| format!("cannot connect to {}:{} ({})", host, port, e))?;

    Ok((host, port))
}

async fn fetch_ollama_models(
    client: &Client,
    ollama_endpoint: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let url = format!("{}/api/tags", ollama_endpoint.trim_end_matches('/'));
    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        return Err(format!("tags endpoint returned {}", resp.status()).into());
    }

    let tags = resp.json::<OllamaTagsResponse>().await?;
    Ok(tags.models.into_iter().map(|m| m.name).collect())
}

async fn fetch_playground_backend(
    client: &Client,
    endpoint: &str,
) -> Result<Option<(String, String)>, Box<dyn std::error::Error>> {
    let url = format!(
        "{}/api/v1/playground/config",
        endpoint.trim_end_matches('/')
    );
    let resp = client.get(url).send().await?;
    if resp.status().is_success() {
        let payload = resp.json::<PlaygroundConfigResponse>().await?;
        return Ok(Some((payload.ai_backend.kind, payload.ai_backend.model)));
    }

    if resp.status().as_u16() == 404 {
        return Ok(None);
    }

    Err(format!("playground config endpoint returned {}", resp.status()).into())
}

fn model_matches(available: &str, expected: &str) -> bool {
    available == expected
        || available
            .split(':')
            .next()
            .map(|name| name == expected)
            .unwrap_or(false)
}

fn print_ok(message: &str) {
    println!("  [OK] {}", message);
}

fn print_warn(message: &str) {
    println!("  [WARN] {}", message);
}

fn print_fail(message: &str) {
    println!("  [FAIL] {}", message);
}

async fn daemon_start(
    platform: &str,
    storage: Option<StorageKindArg>,
    foreground: bool,
    daemon_bin: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = std::env::var("PALM_ENDPOINT").unwrap_or_else(|_| "http://localhost:8080".to_string());
    if check_daemon_health(&build_http_client()?, &endpoint).await.is_ok() {
        print_warn("PALM daemon already appears to be running");
        return Ok(());
    }

    let resolved = resolve_daemon_command(daemon_bin.as_deref());
    let (program, mut args, command_label) = match resolved {
        DaemonProgram::Palmd(bin) => (
            bin.clone(),
            vec!["--platform".to_string(), platform.to_string()],
            bin,
        ),
        DaemonProgram::Cargo => (
            "cargo".to_string(),
            vec![
                "run".to_string(),
                "-p".to_string(),
                "palm-daemon".to_string(),
                "--".to_string(),
                "--platform".to_string(),
                platform.to_string(),
            ],
            "cargo run -p palm-daemon".to_string(),
        ),
    };

    let mut cmd = Command::new(&program);
    cmd.args(&args);
    if let Some(storage_kind) = storage {
        cmd.env("PALM_STORAGE_TYPE", storage_kind.as_env());
    }

    if foreground {
        print_ok(&format!("Starting daemon in foreground with {}", command_label));
        let status = cmd.status()?;
        if status.success() {
            return Ok(());
        }
        return Err(format!("daemon exited with status {}", status).into());
    }

    let state_dir = ensure_state_dir()?;
    let log_path = state_dir.join("palmd.log");
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let stderr = stdout.try_clone()?;
    cmd.stdout(Stdio::from(stdout));
    cmd.stderr(Stdio::from(stderr));
    cmd.stdin(Stdio::null());
    args.shrink_to_fit();

    let child = cmd.spawn()?;
    let pid = child.id();
    let pid_file = DaemonPidFile {
        pid,
        command: format!("{} {}", program, args.join(" ")),
        started_at_epoch_secs: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        log_file: log_path.display().to_string(),
    };
    write_pid_file(&state_dir.join("palmd.pid"), &pid_file)?;

    print_ok(&format!("PALM daemon started in background (pid={})", pid));
    print_ok(&format!("Log file: {}", log_path.display()));
    print_ok("Use `maple daemon stop` to stop it gracefully.");
    Ok(())
}

async fn daemon_stop(endpoint: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = build_http_client()?;
    if request_api_shutdown(&client, endpoint).await {
        print_ok("Shutdown request sent to PALM daemon");
        return Ok(());
    }

    if let Some(pid_path) = state_dir_path().map(|p| p.join("palmd.pid")) {
        if let Some(pid_file) = read_pid_file(&pid_path)? {
            if try_terminate_process(pid_file.pid)? {
                print_ok(&format!(
                    "Sent termination signal to daemon process {} (PID file fallback)",
                    pid_file.pid
                ));
                let _ = fs::remove_file(pid_path);
                return Ok(());
            }
        }
    }

    Err("Unable to stop daemon: API shutdown failed and no managed process found".into())
}

async fn daemon_status(endpoint: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = build_http_client()?;
    match check_daemon_health(&client, endpoint).await {
        Ok(health) => {
            print_ok(&format!(
                "PALM daemon is healthy (version={}, uptime={})",
                health.version, health.uptime
            ));
        }
        Err(err) => {
            print_fail(&format!("PALM daemon health check failed: {}", err));
        }
    }

    match state_dir_path().map(|p| p.join("palmd.pid")) {
        Some(pid_path) => match read_pid_file(&pid_path)? {
            Some(pid_file) => {
                let running = is_process_alive(pid_file.pid)?;
                if running {
                    print_ok(&format!(
                        "Managed daemon process is running (pid={}, log={})",
                        pid_file.pid, pid_file.log_file
                    ));
                } else {
                    print_warn(&format!(
                        "PID file exists but process {} is not running",
                        pid_file.pid
                    ));
                }
            }
            None => {
                print_warn("No managed PID file found (~/.maple/palmd.pid)");
            }
        },
        None => print_warn("HOME is not set; cannot resolve managed PID file path"),
    }

    Ok(())
}

async fn request_api_shutdown(client: &Client, endpoint: &str) -> bool {
    let url = format!(
        "{}/api/v1/system/shutdown",
        endpoint.trim_end_matches('/')
    );
    match client.post(url).json(&serde_json::json!({})).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

enum DaemonProgram {
    Palmd(String),
    Cargo,
}

fn resolve_daemon_command(explicit: Option<&str>) -> DaemonProgram {
    if let Some(bin) = explicit {
        return DaemonProgram::Palmd(bin.to_string());
    }

    if let Ok(path_env) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_env) {
            let candidate = dir.join("palmd");
            if candidate.exists() {
                return DaemonProgram::Palmd(candidate.display().to_string());
            }
        }
    }

    DaemonProgram::Cargo
}

fn state_dir_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(Path::new(&home).join(".maple"))
}

fn ensure_state_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = state_dir_path().ok_or("HOME environment variable is not set")?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn write_pid_file(path: &Path, payload: &DaemonPidFile) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(payload)?;
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

fn read_pid_file(path: &Path) -> Result<Option<DaemonPidFile>, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)?;
    let payload = serde_json::from_str::<DaemonPidFile>(&content)?;
    Ok(Some(payload))
}

fn try_terminate_process(pid: u32) -> Result<bool, Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        let status = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status()?;
        return Ok(status.success());
    }

    #[cfg(not(unix))]
    {
        let _ = pid;
        Ok(false)
    }
}

fn is_process_alive(pid: u32) -> Result<bool, Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        let status = Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .status()?;
        return Ok(status.success());
    }

    #[cfg(not(unix))]
    {
        let _ = pid;
        Ok(false)
    }
}

fn handle_ual(command: UalCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        UalCommands::Parse { file } => {
            let input = fs::read_to_string(file)?;
            let ast = ual_parser::parse(&input)?;
            println!("{}", serde_json::to_string_pretty(&ast)?);
            Ok(())
        }
        UalCommands::Compile { file } => {
            let input = fs::read_to_string(file)?;
            let ast = ual_parser::parse(&input)?;
            let compiled = ual_compiler::compile(&ast)?;
            println!("{}", serde_json::to_string_pretty(&compiled)?);
            Ok(())
        }
        UalCommands::Validate { file } => {
            let input = fs::read_to_string(file)?;
            let ast = ual_parser::parse(&input)?;
            let compiled = ual_compiler::compile(&ast)?;
            let validator = rcf_validator::RcfValidator::new();
            for item in compiled {
                if let ual_compiler::UalCompiled::Commitment(commitment) = item {
                    validator.validate_commitment(&commitment)?;
                }
            }
            println!("UAL validation succeeded.");
            Ok(())
        }
    }
}
