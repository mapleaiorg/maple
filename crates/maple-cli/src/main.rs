use clap::{Args, Parser, Subcommand, ValueEnum};
use maple_runtime::{
    config::RuntimeConfig, AgentHandleRequest, AgentKernel, AgentRegistration, MapleRuntime,
    ModelBackend,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

    /// Run and observe MAPLE AgentKernel operations
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
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

#[derive(Subcommand)]
enum AgentCommands {
    /// Run a local AgentKernel demo (no daemon required)
    Demo {
        /// Backend kind
        #[arg(long, value_enum, default_value = "local_llama")]
        backend: BackendKindArg,

        /// Prompt sent to the model adapter
        #[arg(long, default_value = "log current runtime status")]
        prompt: String,

        /// Use dangerous capability path (simulate transfer)
        #[arg(long)]
        dangerous: bool,

        /// Provide explicit commitment for dangerous path
        #[arg(long)]
        with_commitment: bool,

        /// Simulated amount for dangerous path
        #[arg(long, default_value_t = 500)]
        amount: i64,
    },

    /// Call daemon AgentKernel handle endpoint
    Handle(AgentHandleArgs),

    /// Read daemon AgentKernel audit events
    Audit(AgentAuditArgs),

    /// Show daemon AgentKernel status
    Status {
        /// PALM daemon endpoint
        #[arg(long, env = "PALM_ENDPOINT", default_value = "http://localhost:8080")]
        endpoint: String,
    },

    /// Inspect a contract/commitment and its receipts from the durable ledger
    #[command(alias = "commitment")]
    Contract(AgentCommitmentArgs),

    /// List recent commitment lifecycle records
    Commitments(AgentCommitmentsArgs),
}

#[derive(Args)]
struct AgentHandleArgs {
    /// PALM daemon endpoint
    #[arg(long, env = "PALM_ENDPOINT", default_value = "http://localhost:8080")]
    endpoint: String,

    /// Prompt sent to AgentKernel
    #[arg(long)]
    prompt: String,

    /// Backend kind
    #[arg(long, value_enum, default_value = "local_llama")]
    backend: BackendKindArg,

    /// Optional forced tool name (e.g. echo_log or simulate_transfer)
    #[arg(long)]
    tool: Option<String>,

    /// Optional JSON args for tool override
    #[arg(long)]
    args: Option<String>,

    /// Auto-draft commitment before execution (requires --tool)
    #[arg(long)]
    with_commitment: bool,

    /// Optional commitment outcome description
    #[arg(long)]
    commitment_outcome: Option<String>,
}

#[derive(Args)]
struct AgentAuditArgs {
    /// PALM daemon endpoint
    #[arg(long, env = "PALM_ENDPOINT", default_value = "http://localhost:8080")]
    endpoint: String,

    /// Max number of events
    #[arg(long, default_value_t = 50)]
    limit: usize,
}

#[derive(Args)]
struct AgentCommitmentArgs {
    /// PALM daemon endpoint
    #[arg(long, env = "PALM_ENDPOINT", default_value = "http://localhost:8080")]
    endpoint: String,

    /// Commitment id to inspect
    #[arg(long)]
    id: String,
}

#[derive(Args)]
struct AgentCommitmentsArgs {
    /// PALM daemon endpoint
    #[arg(long, env = "PALM_ENDPOINT", default_value = "http://localhost:8080")]
    endpoint: String,

    /// Number of commitment records to return
    #[arg(long, default_value_t = 20)]
    limit: usize,
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

#[derive(Copy, Clone, Debug, ValueEnum)]
enum BackendKindArg {
    #[value(name = "local_llama")]
    LocalLlama,
    #[value(name = "open_ai")]
    OpenAi,
    #[value(name = "anthropic")]
    Anthropic,
    #[value(name = "gemini")]
    Gemini,
    #[value(name = "grok")]
    Grok,
}

impl BackendKindArg {
    fn as_model_backend(self) -> ModelBackend {
        match self {
            BackendKindArg::LocalLlama => ModelBackend::LocalLlama,
            BackendKindArg::OpenAi => ModelBackend::OpenAi,
            BackendKindArg::Anthropic => ModelBackend::Anthropic,
            BackendKindArg::Gemini => ModelBackend::Gemini,
            BackendKindArg::Grok => ModelBackend::Grok,
        }
    }

    fn as_api_value(self) -> &'static str {
        match self {
            BackendKindArg::LocalLlama => "local_llama",
            BackendKindArg::OpenAi => "open_ai",
            BackendKindArg::Anthropic => "anthropic",
            BackendKindArg::Gemini => "gemini",
            BackendKindArg::Grok => "grok",
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

#[derive(Debug, Deserialize)]
struct AgentKernelStatusResponse {
    resonator_id: String,
    audit_events: usize,
    backends: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AgentKernelHandlePayload {
    prompt: String,
    backend: String,
    tool: Option<String>,
    args: Option<Value>,
    with_commitment: bool,
    commitment_outcome: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgentKernelHandleResponse {
    resonator_id: String,
    cognition: Value,
    action: Option<Value>,
    audit_event_id: String,
    raw_model_output: String,
}

#[derive(Debug, Deserialize)]
struct AgentAuditEvent {
    event_id: String,
    stage: String,
    success: bool,
    message: String,
    commitment_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgentKernelCommitmentResponse {
    commitment_id: String,
    lifecycle_status: String,
    principal: String,
    effect_domain: String,
    decision: String,
    declared_at: String,
    execution_started_at: Option<String>,
    execution_completed_at: Option<String>,
    updated_at: String,
    outcome: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct AgentKernelCommitmentSummaryResponse {
    commitment_id: String,
    lifecycle_status: String,
    decision: String,
    principal: String,
    effect_domain: String,
    declared_at: String,
    execution_started_at: Option<String>,
    execution_completed_at: Option<String>,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct AgentKernelReceiptResponse {
    receipt_id: String,
    tool_call_id: String,
    contract_id: String,
    capability_id: String,
    hash: String,
    timestamp: String,
    status: String,
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
        Commands::Agent { command } => {
            if let Err(err) = handle_agent(command).await {
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

    let storage_type =
        std::env::var("PALM_STORAGE_TYPE").unwrap_or_else(|_| "postgres".to_string());
    match storage_type.as_str() {
        "memory" => {
            print_warn("Storage mode is memory (non-persistent)");
        }
        "postgres" => {
            let pg_url = std::env::var("PALM_STORAGE_URL").unwrap_or_else(|_| {
                "postgres://postgres:postgres@localhost:5432/maple".to_string()
            });
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

    let (ollama_models, ollama_error) =
        match fetch_ollama_models(&client, &args.ollama_endpoint).await {
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
            None => {
                match fetch_playground_backend(&client, &args.endpoint).await {
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
                        print_warn(&format!(
                            "Could not read playground backend config: {}",
                            err
                        ));
                        None
                    }
                }
            }
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

async fn handle_agent(command: AgentCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        AgentCommands::Demo {
            backend,
            prompt,
            dangerous,
            with_commitment,
            amount,
        } => run_local_agent_demo(backend, prompt, dangerous, with_commitment, amount).await,
        AgentCommands::Status { endpoint } => daemon_agent_status(&endpoint).await,
        AgentCommands::Audit(args) => daemon_agent_audit(&args.endpoint, args.limit).await,
        AgentCommands::Handle(args) => daemon_agent_handle(args).await,
        AgentCommands::Contract(args) => daemon_agent_contract(args).await,
        AgentCommands::Commitments(args) => daemon_agent_commitments(args).await,
    }
}

async fn run_local_agent_demo(
    backend: BackendKindArg,
    prompt: String,
    dangerous: bool,
    with_commitment: bool,
    amount: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MapleRuntime::bootstrap(RuntimeConfig::default()).await?;
    let kernel = AgentKernel::new(runtime);
    let host = kernel.register_agent(AgentRegistration::default()).await?;

    print_ok(&format!(
        "Local AgentKernel ready (resonator={})",
        host.resonator_id
    ));

    let mut request =
        AgentHandleRequest::new(host.resonator_id, backend.as_model_backend(), prompt);

    if dangerous {
        request.override_tool = Some("simulate_transfer".to_string());
        request.override_args =
            Some(serde_json::json!({"amount": amount, "to": "demo-beneficiary"}));
        if with_commitment {
            request.commitment = Some(
                kernel
                    .draft_commitment(
                        host.resonator_id,
                        "simulate_transfer",
                        "CLI-authorized simulated transfer",
                    )
                    .await?,
            );
        }
    } else {
        request.override_tool = Some("echo_log".to_string());
        request.override_args = Some(serde_json::json!({"message": "local demo"}));
    }

    match kernel.handle(request).await {
        Ok(response) => {
            print_ok(&format!(
                "Agent handled request (audit={})",
                response.audit_event_id
            ));
            println!("Resonator: {}", response.resonator_id);
            println!(
                "Cognition: {}",
                serde_json::to_string_pretty(&response.cognition)?
            );
            if let Some(action) = response.action {
                println!("Action: {}", serde_json::to_string_pretty(&action)?);
            } else {
                print_warn("No action executed (likely fallback cognition or no tool selected)");
            }
        }
        Err(err) => {
            print_fail(&format!("AgentKernel denied/failed request: {}", err));
        }
    }

    let audits = kernel.audit_events().await;
    print_ok(&format!("Audit events recorded: {}", audits.len()));
    Ok(())
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
    let endpoint =
        std::env::var("PALM_ENDPOINT").unwrap_or_else(|_| "http://localhost:8080".to_string());
    if check_daemon_health(&build_http_client()?, &endpoint)
        .await
        .is_ok()
    {
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
        print_ok(&format!(
            "Starting daemon in foreground with {}",
            command_label
        ));
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

async fn daemon_agent_status(endpoint: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = build_http_client()?;
    let url = format!("{}/api/v1/agent/status", endpoint.trim_end_matches('/'));
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(format!("agent-kernel status request failed: {}", response.status()).into());
    }

    let status = response.json::<AgentKernelStatusResponse>().await?;
    print_ok("AgentKernel status retrieved");
    println!("Resonator: {}", status.resonator_id);
    println!("Audit events: {}", status.audit_events);
    println!("Backends: {}", status.backends.join(", "));
    Ok(())
}

async fn daemon_agent_audit(
    endpoint: &str,
    limit: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = build_http_client()?;
    let url = format!(
        "{}/api/v1/agent/audit?limit={}",
        endpoint.trim_end_matches('/'),
        limit
    );
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(format!("agent-kernel audit request failed: {}", response.status()).into());
    }

    let events = response.json::<Vec<AgentAuditEvent>>().await?;
    print_ok(&format!("Fetched {} audit event(s)", events.len()));
    for event in events {
        println!(
            "- {} [{}] success={} commitment={} :: {}",
            event.event_id,
            event.stage,
            event.success,
            event.commitment_id.unwrap_or_else(|| "-".to_string()),
            event.message
        );
    }
    Ok(())
}

async fn daemon_agent_handle(args: AgentHandleArgs) -> Result<(), Box<dyn std::error::Error>> {
    let client = build_http_client()?;
    let url = format!(
        "{}/api/v1/agent/handle",
        args.endpoint.trim_end_matches('/')
    );

    let parsed_args = if let Some(raw) = args.args {
        Some(
            serde_json::from_str::<Value>(&raw)
                .map_err(|err| format!("invalid --args JSON: {}", err))?,
        )
    } else {
        None
    };

    let payload = AgentKernelHandlePayload {
        prompt: args.prompt,
        backend: args.backend.as_api_value().to_string(),
        tool: args.tool,
        args: parsed_args,
        with_commitment: args.with_commitment,
        commitment_outcome: args.commitment_outcome,
    };

    let response = client.post(url).json(&payload).send().await?;
    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("agent-kernel handle failed: {}", body).into());
    }

    let handled = response.json::<AgentKernelHandleResponse>().await?;
    print_ok(&format!(
        "AgentKernel handle succeeded (resonator={}, audit={})",
        handled.resonator_id, handled.audit_event_id
    ));
    println!(
        "Cognition: {}",
        serde_json::to_string_pretty(&handled.cognition)?
    );
    if let Some(action) = handled.action {
        println!("Action: {}", serde_json::to_string_pretty(&action)?);
    } else {
        print_warn("No action executed");
    }
    println!("Raw model output: {}", handled.raw_model_output);
    Ok(())
}

async fn daemon_agent_contract(
    args: AgentCommitmentArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = build_http_client()?;
    let url = format!(
        "{}/api/v1/agent/commitments/{}",
        args.endpoint.trim_end_matches('/'),
        args.id
    );

    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("commitment lookup failed: {}", body).into());
    }

    let commitment = response.json::<AgentKernelCommitmentResponse>().await?;
    print_ok(&format!("Commitment {} loaded", commitment.commitment_id));
    println!("Lifecycle: {}", commitment.lifecycle_status);
    println!("Principal: {}", commitment.principal);
    println!("Domain: {}", commitment.effect_domain);
    println!("Decision: {}", commitment.decision);
    println!("Declared: {}", commitment.declared_at);
    println!(
        "Started: {}",
        commitment
            .execution_started_at
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "Completed: {}",
        commitment
            .execution_completed_at
            .unwrap_or_else(|| "-".to_string())
    );
    println!("Updated: {}", commitment.updated_at);
    if let Some(outcome) = commitment.outcome {
        println!("Outcome: {}", serde_json::to_string_pretty(&outcome)?);
    } else {
        println!("Outcome: -");
    }

    let receipt_url = format!(
        "{}/api/v1/agent/commitments/{}/receipts",
        args.endpoint.trim_end_matches('/'),
        commitment.commitment_id
    );
    let receipt_response = client.get(receipt_url).send().await?;
    if !receipt_response.status().is_success() {
        let body = receipt_response.text().await.unwrap_or_default();
        return Err(format!("commitment receipts lookup failed: {}", body).into());
    }
    let receipts = receipt_response
        .json::<Vec<AgentKernelReceiptResponse>>()
        .await?;

    println!();
    println!("Receipts: {}", receipts.len());
    println!(
        "{:<20} {:<16} {:<14} {:<12} {:<20}",
        "RECEIPT_ID", "CAPABILITY", "STATUS", "HASH", "TIMESTAMP"
    );
    for receipt in receipts {
        println!(
            "{:<20} {:<16} {:<14} {:<12} {:<20}",
            shorten(&receipt.receipt_id, 20),
            shorten(&receipt.capability_id, 16),
            shorten(&receipt.status, 14),
            shorten(&receipt.hash, 12),
            shorten(&receipt.timestamp, 20),
        );
        println!(
            "  contract={} tool_call={}",
            receipt.contract_id, receipt.tool_call_id
        );
    }
    Ok(())
}

async fn daemon_agent_commitments(
    args: AgentCommitmentsArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = build_http_client()?;
    let url = format!(
        "{}/api/v1/agent/commitments?limit={}",
        args.endpoint.trim_end_matches('/'),
        args.limit
    );
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("commitments lookup failed: {}", body).into());
    }

    let commitments = response
        .json::<Vec<AgentKernelCommitmentSummaryResponse>>()
        .await?;

    print_ok(&format!("Fetched {} commitment(s)", commitments.len()));
    println!(
        "{:<24} {:<10} {:<10} {:<10} {:<12} {:<17} {:<17} {:<17} {:<17}",
        "COMMITMENT_ID",
        "STATUS",
        "DECISION",
        "DOMAIN",
        "PRINCIPAL",
        "DECLARED",
        "STARTED",
        "COMPLETED",
        "UPDATED",
    );
    for item in commitments {
        println!(
            "{:<24} {:<10} {:<10} {:<10} {:<12} {:<17} {:<17} {:<17} {:<17}",
            shorten(&item.commitment_id, 24),
            shorten(&item.lifecycle_status, 10),
            shorten(&item.decision, 10),
            shorten(&item.effect_domain, 10),
            shorten(&item.principal, 12),
            shorten(&item.declared_at, 17),
            shorten(item.execution_started_at.as_deref().unwrap_or("-"), 17),
            shorten(item.execution_completed_at.as_deref().unwrap_or("-"), 17),
            shorten(&item.updated_at, 17),
        );
    }
    Ok(())
}

fn shorten(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let truncated: String = value.chars().take(max.saturating_sub(1)).collect();
    format!("{truncated}…")
}

async fn request_api_shutdown(client: &Client, endpoint: &str) -> bool {
    let url = format!("{}/api/v1/system/shutdown", endpoint.trim_end_matches('/'));
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
