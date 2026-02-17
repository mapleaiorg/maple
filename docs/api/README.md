# MAPLE API Reference

## Crates Overview

| Crate | Description |
|-------|-------------|
| `palm-types` | Core type definitions |
| `palm-registry` | Agent specification registry |
| `palm-deployment` | Deployment management |
| `palm-health` | Health monitoring |
| `palm-state` | State and checkpoint management |
| `palm-control` | Unified control plane |
| `palm-policy` | Policy gate system |
| `maple-kernel-sdk` | WorldLine SDK (CLI + REST router + Python bindings) |
| `maple-cli` | Umbrella CLI (developer tools + `maple palm ...` operations) |
| `palm` | Direct operations CLI (backwards compatible) |
| `palm-daemon` | Background service |
| `palm-observability` | Metrics and audit |
| `palm-platform-pack` | Platform pack contract |
| `palm-conformance` | Conformance test suite |

## Core Types

### PlatformProfile

```rust
pub enum PlatformProfile {
    Mapleverse,
    Finalverse,
    IBank,
    Development,
    Custom(String),
}
```

### AgentSpec

```rust
pub struct AgentSpec {
    pub id: AgentSpecId,
    pub name: String,
    pub version: semver::Version,
    pub capabilities: Vec<String>,
    pub resources: Option<ResourceRequirements>,
    pub health: Option<HealthConfig>,
    pub metadata: HashMap<String, String>,
}
```

### PalmOperation

```rust
pub enum PalmOperation {
    // Registry Operations
    CreateSpec { spec_id: String },
    UpdateSpec { spec_id: String },
    DeprecateSpec { spec_id: String },

    // Deployment Operations
    CreateDeployment { spec_id: String },
    UpdateDeployment { deployment_id: String },
    ScaleDeployment { deployment_id: String, target_replicas: u32 },
    DeleteDeployment { deployment_id: String },
    RollbackDeployment { deployment_id: String },
    PauseDeployment { deployment_id: String },
    ResumeDeployment { deployment_id: String },

    // Instance Operations
    RestartInstance { instance_id: String },
    TerminateInstance { instance_id: String },
    MigrateInstance { instance_id: String },
    DrainInstance { instance_id: String },

    // State Operations
    CreateCheckpoint { instance_id: String },
    RestoreCheckpoint { instance_id: String },
    DeleteCheckpoint { snapshot_id: String },

    // Health Operations
    HealthCheck { instance_id: String },
    ForceRecovery { instance_id: String },

    // Administrative Operations
    ConfigurePolicy { policy_name: String },
    ViewAuditLog { filter: String },
}
```

## Playground API (PALM Daemon)

All endpoints are under `/api/v1`.

### Core Playground Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/playground/state` | Aggregated playground state (config, stats, agents, resonators, activities) |
| `GET` | `/playground/config` | Public playground configuration |
| `PUT` | `/playground/config` | Update playground configuration (backend, simulation) |
| `GET` | `/playground/backends` | Available AI backend catalog |
| `POST` | `/playground/infer` | Run one-shot inference on active backend |
| `GET` | `/playground/resonators` | Resonator list + status |
| `GET` | `/playground/agents` | Agent (instance) list + status |
| `GET` | `/playground/activities` | Activity list (supports `limit` and `after_sequence`) |
| `GET` | `/playground/activities/stream` | Activity stream (SSE) |

### System Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/system/shutdown` | Request graceful PALM daemon shutdown |

### AgentKernel Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/agent-kernel/status` | Daemon-managed AgentKernel host status |
| `POST` | `/agent-kernel/handle` | Execute one AgentKernel step with runtime gating |
| `GET` | `/agent-kernel/audit` | List recent AgentKernel audit events (`limit` query) |

### WorldLine Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/worldlines` | Create worldline (`profile`, optional `label`) |
| `GET` | `/worldlines` | List worldlines |
| `GET` | `/worldlines/:id` | Get worldline status |
| `POST` | `/commitments` | Submit commitment declaration (returns `commitment_id` + `decision_receipt_id`; financial domain requires at least one target + `cap-financial-settle`) |
| `GET` | `/commitments/:id` | Get commitment status |
| `GET` | `/commitments/:id/audit-trail` | Get gate-stage audit trail |
| `GET` | `/provenance/:event_id/ancestors` | Traverse event ancestry (`depth` query) |
| `GET` | `/provenance/worldline/:id/history` | Worldline event history (`from`, `to`) |
| `POST` | `/governance/policies` | Add governance policy |
| `GET` | `/governance/policies` | List governance policies |
| `POST` | `/governance/simulate` | Simulate policy decision for a payload |
| `POST` | `/financial/settle` | Submit settlement legs (`commitment_id` + `decision_receipt_id` required) |
| `GET` | `/financial/:worldline_id/balance/:asset` | Balance projection |
| `GET` | `/kernel/status` | WorldLine kernel status |
| `GET` | `/kernel/metrics` | WorldLine kernel metrics |

Supported backend kinds: `local_llama`, `open_ai`, `anthropic`, `grok`, `gemini`.

`simulation` config now also supports:
- `auto_inference_enabled` (bool)
- `inference_interval_ticks` (u64)
- `inferences_per_tick` (u32)

`POST /playground/infer` request body:

```json
{
  "prompt": "Summarize resonator health",
  "system_prompt": "You are a MAPLE operator assistant",
  "actor_id": "ops-console",
  "temperature": 0.4,
  "max_tokens": 512
}
```

### Playground UI

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/playground` | Multi-tab dashboard (HTML) |

## Platform Pack Contract

### PlatformPack Trait

```rust
#[async_trait]
pub trait PlatformPack: Send + Sync {
    /// Returns the platform profile this pack implements
    fn profile(&self) -> PlatformProfile;

    /// Returns platform metadata (name, version, description)
    fn metadata(&self) -> &PlatformMetadata;

    /// Returns the policy configuration for this platform
    fn policy_config(&self) -> &PlatformPolicyConfig;

    /// Returns the health monitoring configuration
    fn health_config(&self) -> &PlatformHealthConfig;

    /// Returns the state management configuration
    fn state_config(&self) -> &PlatformStateConfig;

    /// Returns the resource constraints configuration
    fn resource_config(&self) -> &PlatformResourceConfig;

    /// Returns the recovery behavior configuration
    fn recovery_config(&self) -> &PlatformRecoveryConfig;

    /// Returns the capabilities this platform supports
    fn capabilities(&self) -> &PlatformCapabilities;

    /// Validates that an agent spec is compatible with this platform
    async fn validate_agent_spec(&self, spec: &AgentSpec) -> Result<(), PackError>;

    /// Called when the platform pack is loaded
    async fn on_load(&self) -> Result<(), PackError>;

    /// Called when the platform pack is unloaded
    async fn on_unload(&self) -> Result<(), PackError>;
}
```

### PlatformCapabilities

```rust
pub struct PlatformCapabilities {
    pub max_deployments: Option<u32>,
    pub max_instances_per_deployment: Option<u32>,
    pub max_total_instances: Option<u32>,
    pub supports_migration: bool,
    pub supports_hot_reload: bool,
    pub supports_canary: bool,
    pub supports_blue_green: bool,
    pub supports_human_approval: bool,
    pub supports_checkpoints: bool,
    pub supports_cross_node_migration: bool,
    pub custom: HashMap<String, serde_json::Value>,
}
```

## Policy System

### PolicyDecision

```rust
pub enum PolicyDecision {
    /// Operation is allowed
    Allow,

    /// Operation is denied
    Deny {
        reason: String,
        policy_id: String,
    },

    /// Operation requires human approval
    RequiresApproval {
        approvers: Vec<String>,
        reason: String,
        policy_id: String,
    },

    /// Operation is held for review
    Hold {
        reason: String,
        policy_id: String,
        expires_at: Option<DateTime<Utc>>,
    },
}
```

### PolicyEvaluator

```rust
impl PolicyEvaluator {
    /// Create a new policy evaluator with default platform policy
    pub fn new(platform: PlatformProfile) -> Self;

    /// Create with custom gates
    pub fn with_gates(platform: PlatformProfile, gates: Vec<Arc<dyn PolicyGate>>) -> Self;

    /// Evaluate an operation
    pub async fn evaluate(
        &self,
        operation: &PalmOperation,
        context: &PolicyEvaluationContext,
    ) -> Result<PolicyDecision>;

    /// Check if an operation is allowed
    pub async fn is_allowed(
        &self,
        operation: &PalmOperation,
        context: &PolicyEvaluationContext,
    ) -> bool;
}
```

### PolicyEvaluationContext

```rust
pub struct PolicyEvaluationContext {
    pub actor_id: String,
    pub actor_type: ActorType,
    pub platform: PlatformProfile,
    pub environment: String,
    pub human_approval: Option<HumanApproval>,
    pub timestamp: DateTime<Utc>,
    pub request_id: String,
    pub metadata: HashMap<String, String>,
    pub quota_usage: Option<QuotaUsage>,
}

impl PolicyEvaluationContext {
    pub fn new(actor_id: impl Into<String>, platform: PlatformProfile) -> Self;
    pub fn with_human_approval(self, approval: HumanApproval) -> Self;
    pub fn with_environment(self, environment: impl Into<String>) -> Self;
    pub fn has_human_approval(&self) -> bool;
}
```

## Conformance Testing

### ConformanceRunner

```rust
impl ConformanceRunner {
    pub fn new(config: ConformanceConfig) -> Self;

    pub async fn run(&self, pack: Arc<dyn PlatformPack>) -> ConformanceReport;
}
```

### ConformanceConfig

```rust
pub struct ConformanceConfig {
    pub run_core: bool,
    pub run_behavioral: bool,
    pub run_platform_specific: bool,
    pub test_timeout: Duration,
    pub continue_on_failure: bool,
    pub verbose: bool,
}
```

### ConformanceReport

```rust
impl ConformanceReport {
    pub fn is_conformant(&self) -> bool;
    pub fn passed_count(&self) -> usize;
    pub fn failed_count(&self) -> usize;
    pub fn skipped_count(&self) -> usize;
    pub fn to_text(&self) -> String;
    pub fn to_json(&self) -> String;
}
```

## Error Types

### PackError

```rust
pub enum PackError {
    ConfigParse(String),
    Io(String),
    Validation(String),
    IncompatibleSpec(String),
    UnsupportedCapability(String),
    Platform(String),
}
```

### PolicyError

```rust
pub enum PolicyError {
    Denied { reason: String },
    MissingApproval { approvers: Vec<String> },
    QuotaExceeded { resource: String },
    PlatformConstraint { constraint: String },
    TimeRestriction { restriction: String },
    RateLimitExceeded { limit: String },
    EvaluationFailed { reason: String },
    InvalidConfiguration { reason: String },
    PolicyNotFound { policy_id: String },
}
```

## Next Steps

- [Architecture Guide](../architecture.md)
- [Platform Packs Tutorial](../tutorials/platform-packs.md)
- [Conformance Guide](../conformance.md)
