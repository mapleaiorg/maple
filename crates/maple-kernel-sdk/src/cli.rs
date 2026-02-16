//! MWL CLI command definitions and handlers.
//!
//! These commands are exposed by the umbrella `maple` CLI as direct groups:
//! `worldline`, `commit`, `provenance`, `financial`, `policy`, and `kernel`.
//!
//! They can also be embedded under a wrapper command with `MwlCommands`.

use clap::{Subcommand, ValueEnum};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Top-level MWL subcommand.
///
/// Adds these command groups to the maple CLI:
/// - `maple worldline` — WorldLine lifecycle management
/// - `maple commit` — Commitment Gate operations
/// - `maple provenance` — Causal provenance queries
/// - `maple financial` — Financial operations (EVOS/ARES)
/// - `maple policy` — Governance policy management
/// - `maple kernel` — Kernel status and metrics
#[derive(Subcommand)]
pub enum MwlCommands {
    /// WorldLine lifecycle management
    Worldline {
        #[command(subcommand)]
        command: WorldlineCommands,
    },
    /// Commitment Gate operations
    Commit {
        #[command(subcommand)]
        command: CommitCommands,
    },
    /// Causal provenance queries
    Provenance {
        #[command(subcommand)]
        command: ProvenanceCommands,
    },
    /// Financial operations (EVOS/ARES)
    Financial {
        #[command(subcommand)]
        command: FinancialCommands,
    },
    /// Governance policy management
    Policy {
        #[command(subcommand)]
        command: PolicyCommands,
    },
    /// Kernel status and metrics
    Kernel {
        #[command(subcommand)]
        command: KernelCommands,
    },
}

// ──────────────────────────────────────────────
// WorldLine commands
// ──────────────────────────────────────────────

/// Profile type argument for CLI.
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum ProfileArg {
    Human,
    Agent,
    Financial,
    World,
    Coordination,
}

impl ProfileArg {
    pub fn as_str(self) -> &'static str {
        match self {
            ProfileArg::Human => "human",
            ProfileArg::Agent => "agent",
            ProfileArg::Financial => "financial",
            ProfileArg::World => "world",
            ProfileArg::Coordination => "coordination",
        }
    }
}

#[derive(Subcommand)]
pub enum WorldlineCommands {
    /// Create a new worldline
    Create {
        /// Profile type
        #[arg(long, value_enum, default_value = "agent")]
        profile: ProfileArg,

        /// Optional human-readable label
        #[arg(long)]
        label: Option<String>,
    },
    /// Get worldline status
    Status {
        /// Worldline ID
        id: String,
    },
    /// List all worldlines
    List,
}

// ──────────────────────────────────────────────
// Commitment commands
// ──────────────────────────────────────────────

#[derive(Subcommand)]
pub enum CommitCommands {
    /// Submit a commitment declaration from a JSON file
    Submit {
        /// Path to the declaration JSON file
        #[arg(long)]
        file: String,
    },
    /// Get commitment status
    Status {
        /// Commitment ID
        id: String,
    },
    /// View full audit trail for a commitment
    AuditTrail {
        /// Commitment ID
        id: String,
    },
}

// ──────────────────────────────────────────────
// Provenance commands
// ──────────────────────────────────────────────

#[derive(Subcommand)]
pub enum ProvenanceCommands {
    /// Find ancestors of an event
    Ancestors {
        /// Event ID
        event_id: String,
        /// Maximum depth to traverse
        #[arg(long, default_value_t = 10)]
        depth: u32,
    },
    /// View worldline history
    WorldlineHistory {
        /// Worldline ID
        id: String,
        /// Start time (Unix ms)
        #[arg(long)]
        from: Option<u64>,
        /// End time (Unix ms)
        #[arg(long)]
        to: Option<u64>,
    },
}

// ──────────────────────────────────────────────
// Financial commands
// ──────────────────────────────────────────────

#[derive(Subcommand)]
pub enum FinancialCommands {
    /// Project balance for a worldline+asset
    Projection {
        /// Worldline ID
        worldline_id: String,
        /// Asset code (e.g. USD, BTC)
        asset: String,
    },
    /// Submit a settlement from a JSON file
    Settle {
        /// Path to the settlement JSON file
        #[arg(long)]
        file: String,
    },
}

// ──────────────────────────────────────────────
// Policy commands
// ──────────────────────────────────────────────

#[derive(Subcommand)]
pub enum PolicyCommands {
    /// List all governance policies
    List,
    /// Simulate a policy against a commitment declaration
    Simulate {
        /// Path to the declaration JSON file
        #[arg(long)]
        file: String,
    },
}

// ──────────────────────────────────────────────
// Kernel commands
// ──────────────────────────────────────────────

#[derive(Subcommand)]
pub enum KernelCommands {
    /// Show kernel status
    Status,
    /// Show kernel metrics
    Metrics,
}

// ──────────────────────────────────────────────
// Response DTOs
// ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct WorldlineResponse {
    pub id: String,
    pub profile: String,
    pub label: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitmentResponse {
    pub id: String,
    pub declaring_identity: String,
    pub status: String,
    pub domain: String,
    pub risk_class: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditTrailResponse {
    pub commitment_id: String,
    pub events: Vec<AuditEventResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditEventResponse {
    pub event_id: String,
    pub stage: String,
    pub result: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProvenanceResponse {
    pub event_id: String,
    pub ancestors: Vec<ProvenanceNodeResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProvenanceNodeResponse {
    pub event_id: String,
    pub worldline: String,
    pub stage: String,
    pub timestamp: String,
    pub depth: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub worldline_id: String,
    pub asset: String,
    pub balance_minor: i64,
    pub trajectory_length: usize,
    pub projected_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PolicyResponse {
    pub id: String,
    pub name: String,
    pub constitutional: bool,
    pub conditions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KernelStatusResponse {
    pub version: String,
    pub worldline_count: usize,
    pub commitment_count: usize,
    pub profile_types: Vec<String>,
    pub invariants_active: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KernelMetricsResponse {
    pub total_commitments: u64,
    pub approved: u64,
    pub denied: u64,
    pub pending: u64,
    pub total_events: u64,
    pub active_worldlines: u64,
}

// ──────────────────────────────────────────────
// CLI handler dispatch
// ──────────────────────────────────────────────

/// Handle an MWL CLI command. Dispatches to the appropriate handler.
///
/// `endpoint` is the PALM daemon base URL (e.g., "http://localhost:8080").
pub async fn handle_mwl_command(
    command: MwlCommands,
    endpoint: &str,
    client: &Client,
) {
    let base = format!("{}/api/v1", endpoint.trim_end_matches('/'));

    match command {
        MwlCommands::Worldline { command } => handle_worldline(command, &base, client).await,
        MwlCommands::Commit { command } => handle_commit(command, &base, client).await,
        MwlCommands::Provenance { command } => handle_provenance(command, &base, client).await,
        MwlCommands::Financial { command } => handle_financial(command, &base, client).await,
        MwlCommands::Policy { command } => handle_policy(command, &base, client).await,
        MwlCommands::Kernel { command } => handle_kernel(command, &base, client).await,
    }
}

async fn handle_worldline(cmd: WorldlineCommands, base: &str, client: &Client) {
    match cmd {
        WorldlineCommands::Create { profile, label } => {
            let body = serde_json::json!({
                "profile": profile.as_str(),
                "label": label,
            });
            match client.post(format!("{}/worldlines", base)).json(&body).send().await {
                Ok(resp) => print_response(resp).await,
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        WorldlineCommands::Status { id } => {
            match client.get(format!("{}/worldlines/{}", base, id)).send().await {
                Ok(resp) => print_response(resp).await,
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        WorldlineCommands::List => {
            match client.get(format!("{}/worldlines", base)).send().await {
                Ok(resp) => print_response(resp).await,
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    }
}

async fn handle_commit(cmd: CommitCommands, base: &str, client: &Client) {
    match cmd {
        CommitCommands::Submit { file } => {
            match std::fs::read_to_string(&file) {
                Ok(content) => {
                    let body: serde_json::Value = match serde_json::from_str(&content) {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("Invalid JSON in {}: {}", file, e);
                            return;
                        }
                    };
                    match client.post(format!("{}/commitments", base)).json(&body).send().await {
                        Ok(resp) => print_response(resp).await,
                        Err(e) => eprintln!("Error: {}", e),
                    }
                }
                Err(e) => eprintln!("Cannot read file {}: {}", file, e),
            }
        }
        CommitCommands::Status { id } => {
            match client.get(format!("{}/commitments/{}", base, id)).send().await {
                Ok(resp) => print_response(resp).await,
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        CommitCommands::AuditTrail { id } => {
            match client
                .get(format!("{}/commitments/{}/audit-trail", base, id))
                .send()
                .await
            {
                Ok(resp) => print_response(resp).await,
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    }
}

async fn handle_provenance(cmd: ProvenanceCommands, base: &str, client: &Client) {
    match cmd {
        ProvenanceCommands::Ancestors { event_id, depth } => {
            match client
                .get(format!(
                    "{}/provenance/{}/ancestors?depth={}",
                    base, event_id, depth
                ))
                .send()
                .await
            {
                Ok(resp) => print_response(resp).await,
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        ProvenanceCommands::WorldlineHistory { id, from, to } => {
            let mut url = format!("{}/provenance/worldline/{}/history", base, id);
            let mut params = vec![];
            if let Some(f) = from {
                params.push(format!("from={}", f));
            }
            if let Some(t) = to {
                params.push(format!("to={}", t));
            }
            if !params.is_empty() {
                url = format!("{}?{}", url, params.join("&"));
            }
            match client.get(url).send().await {
                Ok(resp) => print_response(resp).await,
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    }
}

async fn handle_financial(cmd: FinancialCommands, base: &str, client: &Client) {
    match cmd {
        FinancialCommands::Projection { worldline_id, asset } => {
            match client
                .get(format!(
                    "{}/financial/{}/balance/{}",
                    base, worldline_id, asset
                ))
                .send()
                .await
            {
                Ok(resp) => print_response(resp).await,
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        FinancialCommands::Settle { file } => {
            match std::fs::read_to_string(&file) {
                Ok(content) => {
                    let body: serde_json::Value = match serde_json::from_str(&content) {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("Invalid JSON in {}: {}", file, e);
                            return;
                        }
                    };
                    match client
                        .post(format!("{}/financial/settle", base))
                        .json(&body)
                        .send()
                        .await
                    {
                        Ok(resp) => print_response(resp).await,
                        Err(e) => eprintln!("Error: {}", e),
                    }
                }
                Err(e) => eprintln!("Cannot read file {}: {}", file, e),
            }
        }
    }
}

async fn handle_policy(cmd: PolicyCommands, base: &str, client: &Client) {
    match cmd {
        PolicyCommands::List => {
            match client.get(format!("{}/governance/policies", base)).send().await {
                Ok(resp) => print_response(resp).await,
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        PolicyCommands::Simulate { file } => {
            match std::fs::read_to_string(&file) {
                Ok(content) => {
                    let body: serde_json::Value = match serde_json::from_str(&content) {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("Invalid JSON in {}: {}", file, e);
                            return;
                        }
                    };
                    match client
                        .post(format!("{}/governance/simulate", base))
                        .json(&body)
                        .send()
                        .await
                    {
                        Ok(resp) => print_response(resp).await,
                        Err(e) => eprintln!("Error: {}", e),
                    }
                }
                Err(e) => eprintln!("Cannot read file {}: {}", file, e),
            }
        }
    }
}

async fn handle_kernel(cmd: KernelCommands, base: &str, client: &Client) {
    match cmd {
        KernelCommands::Status => {
            match client.get(format!("{}/kernel/status", base)).send().await {
                Ok(resp) => print_response(resp).await,
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        KernelCommands::Metrics => {
            match client.get(format!("{}/kernel/metrics", base)).send().await {
                Ok(resp) => print_response(resp).await,
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    }
}

/// Print an HTTP response as formatted JSON.
async fn print_response(resp: reqwest::Response) {
    let status = resp.status();
    match resp.text().await {
        Ok(body) => {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                if status.is_success() {
                    println!("{}", serde_json::to_string_pretty(&json).unwrap_or(body));
                } else {
                    eprintln!(
                        "Error ({}): {}",
                        status,
                        serde_json::to_string_pretty(&json).unwrap_or(body)
                    );
                }
            } else if status.is_success() {
                println!("{}", body);
            } else {
                eprintln!("Error ({}): {}", status, body);
            }
        }
        Err(e) => eprintln!("Error reading response: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_arg_as_str() {
        assert_eq!(ProfileArg::Human.as_str(), "human");
        assert_eq!(ProfileArg::Agent.as_str(), "agent");
        assert_eq!(ProfileArg::Financial.as_str(), "financial");
        assert_eq!(ProfileArg::World.as_str(), "world");
        assert_eq!(ProfileArg::Coordination.as_str(), "coordination");
    }

    #[test]
    fn worldline_response_serialization() {
        let resp = WorldlineResponse {
            id: "wl-123".into(),
            profile: "agent".into(),
            label: Some("test-agent".into()),
            status: "active".into(),
            created_at: "2024-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let restored: WorldlineResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "wl-123");
        assert_eq!(restored.profile, "agent");
    }

    #[test]
    fn commitment_response_serialization() {
        let resp = CommitmentResponse {
            id: "cm-456".into(),
            declaring_identity: "wl-123".into(),
            status: "approved".into(),
            domain: "communication".into(),
            risk_class: "low".into(),
            created_at: "2024-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let restored: CommitmentResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "cm-456");
    }

    #[test]
    fn balance_response_serialization() {
        let resp = BalanceResponse {
            worldline_id: "wl-123".into(),
            asset: "USD".into(),
            balance_minor: 100_000,
            trajectory_length: 5,
            projected_at: "2024-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let restored: BalanceResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.balance_minor, 100_000);
    }

    #[test]
    fn kernel_status_response_serialization() {
        let resp = KernelStatusResponse {
            version: "0.1.2".into(),
            worldline_count: 10,
            commitment_count: 50,
            profile_types: vec!["human".into(), "agent".into()],
            invariants_active: 8,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let restored: KernelStatusResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.invariants_active, 8);
    }

    #[test]
    fn kernel_metrics_response_serialization() {
        let resp = KernelMetricsResponse {
            total_commitments: 100,
            approved: 80,
            denied: 15,
            pending: 5,
            total_events: 500,
            active_worldlines: 10,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let restored: KernelMetricsResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.total_commitments, 100);
        assert_eq!(restored.approved + restored.denied + restored.pending, 100);
    }
}
