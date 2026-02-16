# 🍁 Getting Started with MAPLE

Welcome to MAPLE (Multi-Agent Platform for Learning and Evolution) - a production-ready multi-agent AI framework built on Resonance Architecture principles.

> If you are onboarding to the WorldLine Framework (Prompt 1-28 implementation), start with [WorldLine Quickstart](tutorials/worldline-quickstart.md).

## What You'll Learn

This guide covers:
1. **Basic Setup** - Create your first Resonator
2. **Relationships** - Establish stateful couplings
3. **Cognitive Pipeline** - Use the meaning → intent → commitment flow
4. **Memory System** - Multi-tier memory for context management
5. **Observability** - Monitor your system with metrics and tracing
6. **Operations** - Use the CLI and control plane

## Prerequisites

- **Rust 1.75+** - [Install Rust](https://www.rust-lang.org/tools/install)
- **Basic async Rust** - Understanding of async/await
- **PostgreSQL** (optional) - For persistence

## Installation

### Option 1: Add to Existing Project

```toml
# Cargo.toml
[dependencies]
maple-runtime = "0.1"
tokio = { version = "1.35", features = ["full"] }
tracing-subscriber = "0.3"

# Optional: cognitive pipeline
resonator-meaning = "0.1"
resonator-intent = "0.1"
resonator-commitment = "0.1"

# Optional: memory & observability
resonator-memory = "0.1"
resonator-observability = "0.1"
```

### Option 2: Clone the Repository

```bash
git clone https://github.com/mapleaiorg/maple.git
cd maple
cargo build --release
```

---

## Part 1: Your First Resonator

```rust
use maple_runtime::{MapleRuntime, ResonatorSpec, config::RuntimeConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🍁 Bootstrapping MAPLE Runtime...\n");

    // 1. Create runtime
    let runtime = MapleRuntime::bootstrap(RuntimeConfig::default()).await?;
    println!("✅ Runtime ready\n");

    // 2. Register a Resonator (intelligent entity)
    let resonator = runtime.register_resonator(ResonatorSpec::default()).await?;
    println!("✅ Resonator: {}\n", resonator.id);

    // 3. Signal presence (gradient, not binary)
    let presence = maple_runtime::PresenceState::new();
    resonator.signal_presence(presence).await?;
    println!("✅ Presence signaled\n");

    // 4. Shutdown
    runtime.shutdown().await?;
    println!("🎉 Done!");

    Ok(())
}
```

Run it:
```bash
cargo run
```

### What Happened?

1. **Runtime Bootstrap** - Initializes invariant guards, presence fabric, coupling fabric, attention allocator
2. **Resonator Registration** - Creates a persistent identity with attention budget
3. **Presence Signaling** - Establishes gradient presence (discoverability, responsiveness, stability)

---

## Part 2: Creating Relationships (Coupling)

Resonators form **couplings** - stateful relationships that strengthen over time:

```rust
use maple_runtime::{MapleRuntime, ResonatorSpec, CouplingParams, PresenceState, config::RuntimeConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MapleRuntime::bootstrap(RuntimeConfig::default()).await?;

    // Register two Resonators
    let alice = runtime.register_resonator(ResonatorSpec::default()).await?;
    let bob = runtime.register_resonator(ResonatorSpec::default()).await?;

    // Signal presence (required before coupling - Invariant #1)
    alice.signal_presence(PresenceState::new()).await?;
    bob.signal_presence(PresenceState::new()).await?;

    // Establish coupling
    let coupling = runtime.establish_coupling(CouplingParams {
        source: alice.id.clone(),
        target: bob.id.clone(),
        initial_strength: 0.3,  // Max allowed initially
        initial_attention_cost: 100.0,
        ..Default::default()
    }).await?;

    println!("✅ Coupling: {} (strength: 0.3)", coupling.id);

    // Strengthen gradually (architectural requirement)
    coupling.strengthen(0.1).await?;
    println!("✅ Strengthened to 0.4");

    // Check attention impact
    if let Some(budget) = alice.attention_status().await {
        println!("⚡ Alice's attention: {}/{}", budget.available, budget.total_capacity);
    }

    runtime.shutdown().await?;
    Ok(())
}
```

### Key Concepts

| Concept | Description |
|---------|-------------|
| **Gradual Strengthening** | Can only strengthen by 0.1 per step (prevents aggressive coupling) |
| **Attention Economics** | Couplings consume attention - finite resource |
| **Invariant Enforcement** | Presence required before coupling (automatic) |

---

## Part 3: Cognitive Pipeline

The full resonance pipeline: **Meaning → Intent → Commitment → Consequence**

```rust
use maple_runtime::{MapleRuntime, ResonatorSpec, config::RuntimeConfig};
use resonator_meaning::MeaningFormationEngine;
use resonator_intent::IntentStabilizationEngine;
use resonator_commitment::ContractEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MapleRuntime::bootstrap(RuntimeConfig::default()).await?;
    let resonator = runtime.register_resonator(ResonatorSpec::default()).await?;

    // Initialize cognitive engines
    let meaning_engine = MeaningFormationEngine::new();
    let intent_engine = IntentStabilizationEngine::new();
    let contract_engine = ContractEngine::new();

    // 1. Form meaning from input
    let input = "User wants to modify configuration settings";
    let meaning = meaning_engine.form_meaning(input).await?;
    println!("📖 Meaning formed: {} (confidence: {:.2})",
        meaning.action, meaning.confidence);

    // 2. Stabilize intent from meaning (Invariant #3)
    let intent = intent_engine.stabilize(&meaning).await?;
    println!("🎯 Intent stabilized: {:?} (stability: {:.2})",
        intent.intent_type, intent.stability_score);

    // 3. Create commitment with audit trail (Invariant #4)
    let commitment = contract_engine.create_commitment(
        resonator.id.clone(),
        intent,
    ).await?;
    println!("📝 Commitment: {} (effect: {:?})",
        commitment.commitment_id, commitment.effect_domain);

    // 4. Now consequences can be tracked
    // (Only possible because commitment exists)

    runtime.shutdown().await?;
    Ok(())
}
```

### Pipeline Invariants

1. **Meaning precedes Intent** - Can't stabilize intent without formed meaning
2. **Commitment precedes Consequence** - No side effects without explicit commitment
3. **Receipts are Immutable** - Once committed, cannot be modified
4. **Audit trail is Append-Only** - Full accountability

---

## Part 4: Memory System

Multi-tier memory for intelligent context management:

```rust
use resonator_memory::{MemorySystem, MemoryEntry, MemoryTier, MemoryConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create memory system
    let memory = MemorySystem::new(MemoryConfig::default());

    // Store in different tiers
    memory.store(MemoryEntry::new(
        "recent_interaction",
        "User asked about configuration",
        MemoryTier::ShortTerm,
    )).await?;
    println!("📝 Stored in short-term memory");

    memory.store(MemoryEntry::new(
        "current_task",
        "Processing configuration request",
        MemoryTier::Working,
    )).await?;
    println!("📝 Stored in working memory");

    memory.store(MemoryEntry::new(
        "learned_pattern",
        "Config changes require confirmation",
        MemoryTier::LongTerm,
    )).await?;
    println!("📝 Stored in long-term memory");

    // Store episodic memory with emotional context
    memory.store(MemoryEntry::episodic(
        "positive_outcome",
        "Successfully helped user configure system",
        0.8,  // Emotional valence (positive)
    )).await?;
    println!("📝 Stored episodic memory");

    // Retrieve relevant memories
    let context = "configuration settings";
    let relevant = memory.retrieve_relevant(context, 5).await?;
    println!("\n🔍 Found {} relevant memories", relevant.len());

    // Consolidate working → long-term periodically
    memory.consolidate().await?;
    println!("✅ Memory consolidated");

    Ok(())
}
```

### Memory Tiers

| Tier | Purpose | Retention | Capacity |
|------|---------|-----------|----------|
| **Short-term** | Recent interactions | Minutes | Limited |
| **Working** | Active processing | Session | Moderate |
| **Long-term** | Learned patterns | Persistent | Large |
| **Episodic** | Experiences with emotion | Persistent | Selective |

---

## Part 5: Observability

Built-in metrics, tracing, and alerting:

```rust
use resonator_observability::{
    MetricsCollector, SpanTracker, AlertEngine,
    AlertRule, AlertSeverity, AlertOperator, PipelineStage,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize observability
    let metrics = MetricsCollector::new();
    let spans = SpanTracker::default();
    let alerts = AlertEngine::default();

    // Configure alert rule
    alerts.add_rule(AlertRule {
        name: "high_failure_rate".into(),
        description: "Too many commitment failures".into(),
        severity: AlertSeverity::Warning,
        metric: "commitment.failed".into(),
        threshold: 5.0,
        operator: AlertOperator::GreaterThan,
        enabled: true,
    })?;
    println!("🚨 Alert rule configured");

    // Start a trace span
    let span = spans.start_span("pipeline.full_cycle");
    println!("📍 Started span: {}", span.id.0);

    // Record pipeline metrics
    metrics.record_pipeline_request(PipelineStage::Meaning);
    metrics.record_pipeline_latency(PipelineStage::Meaning, 45.0);
    println!("📊 Meaning stage: 45ms");

    metrics.record_pipeline_request(PipelineStage::Intent);
    metrics.record_pipeline_latency(PipelineStage::Intent, 120.0);
    println!("📊 Intent stage: 120ms");

    metrics.record_pipeline_request(PipelineStage::Commitment);
    metrics.record_commitment_created();
    metrics.record_pipeline_latency(PipelineStage::Commitment, 50.0);
    println!("📊 Commitment stage: 50ms");

    // Complete span
    spans.complete_span(&span.id)?;
    println!("✅ Span completed");

    Ok(())
}
```

### Observability Features

- **Metrics**: Pipeline latency, commitment counts, failure rates
- **Tracing**: Distributed spans across operations
- **Alerts**: Configurable rules with severity levels
- **Export**: JSON, Prometheus, OpenTelemetry formats

---

## Part 6: Operations (CLI)

### Start the Control Plane

```bash
# Start PALM daemon
cargo run -p maple-cli -- daemon start

# Or with specific configuration
cargo run -p maple-cli -- daemon start --platform mapleverse --storage memory

# Check status
cargo run -p maple-cli -- daemon status

# Stop daemon
cargo run -p maple-cli -- daemon stop
```

### System Health

```bash
# Run diagnostics
cargo run -p maple-cli -- doctor
```

### Monitoring

```bash
# Real-time event stream
cargo run -p maple-cli -- events watch

# View recent activities
cargo run -p maple-cli -- playground activities --limit 50
```

### Resonator CLI

```bash
# Show architectural invariants
cargo run -p resonator-cli -- invariants

# Show pipeline flow
cargo run -p resonator-cli -- pipeline

# List commitments
cargo run -p resonator-cli -- commitment list

# View commitment lifecycle
cargo run -p resonator-cli -- commitment lifecycle

# Track consequences
cargo run -p resonator-cli -- consequence list
```

---

## Part 7: Conformance Testing

Verify your implementation enforces all 9 invariants:

```rust
use resonator_conformance::{ConformanceSuite, ConformanceConfig, Invariant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let suite = ConformanceSuite::new(ConformanceConfig::default());

    println!("🧪 Running conformance tests...\n");

    // Run all invariant tests
    let report = suite.run_all();

    // Check results
    println!("Results:");
    println!("  Total: {}", report.summary.total);
    println!("  Passed: {} ✅", report.summary.passed);
    println!("  Failed: {} ❌", report.summary.failed);
    println!("  Duration: {}ms\n", report.duration_ms);

    if report.all_passed() {
        println!("🎉 MAPLE COMPLIANT - All 9 invariants verified!");
    } else {
        println!("⚠️ Some invariants failed:");
        for test in &report.tests {
            if test.status == resonator_conformance::TestStatus::Failed {
                println!("  ❌ {:?}: {}", test.invariant, test.name);
            }
        }
    }

    Ok(())
}
```

### The 9 Invariants

| # | Invariant | Test |
|---|-----------|------|
| 1 | Presence precedes Coupling | Verify presence required |
| 2 | Coupling precedes Meaning | Verify coupling context |
| 3 | Meaning precedes Intent | Verify meaning convergence |
| 4 | Commitment precedes Consequence | Verify active commitment |
| 5 | Receipts are Immutable | Attempt modification (fail) |
| 6 | Audit trail is Append-Only | Attempt deletion (fail) |
| 7 | Capabilities gate Actions | Attempt unauthorized action |
| 8 | Time anchors are Monotonic | Verify ordering |
| 9 | Implementation Provenance & Evolution | Verify replay-verified, evidence-anchored upgrades |

---

## Platform Configurations

### Mapleverse (Pure AI)

```rust
use maple_runtime::config::mapleverse_runtime_config;

let runtime = MapleRuntime::bootstrap(mapleverse_runtime_config()).await?;
// No human profiles - pure AI coordination
// Optimized for 100M+ agents
```

### Finalverse (Human-AI)

```rust
use maple_runtime::config::finalverse_runtime_config;

let runtime = MapleRuntime::bootstrap(finalverse_runtime_config()).await?;
// Human agency protection
// Coercion detection enabled
```

### iBank (AI Finance)

```rust
use maple_runtime::config::ibank_runtime_config;

let runtime = MapleRuntime::bootstrap(ibank_runtime_config()).await?;
// AI-only, mandatory audit trails
// Risk-bounded ($1M limit)
```

---

## Running Examples

```bash
# Basic resonator
cargo run -p maple-runtime --example 01_basic_resonator

# Coupling dynamics
cargo run -p maple-runtime --example 02_resonator_coupling

# Platform configs
cargo run -p maple-runtime --example 03_mapleverse_config
cargo run -p maple-runtime --example 04_finalverse_config
cargo run -p maple-runtime --example 05_ibank_config

# Multi-resonator
cargo run -p maple-runtime --example 06_multi_resonator

# Cognitive pipeline
cargo run -p maple-runtime --example 07_meaning_to_commitment

# Memory system
cargo run -p maple-runtime --example 08_memory_and_conversation

# Observability
cargo run -p maple-runtime --example 09_observability_demo

# Conformance testing
cargo run -p maple-runtime --example 10_conformance_testing
```

---

## Debugging

### Enable Logging

```bash
RUST_LOG=maple_runtime=debug cargo run
```

Or in code:

```rust
tracing_subscriber::fmt()
    .with_env_filter("maple_runtime=debug,resonator_meaning=debug")
    .init();
```

### Check Telemetry

```rust
if let Some(telemetry) = runtime.telemetry().await {
    println!("Resonators: {}", telemetry.resonator_count);
    println!("Couplings: {}", telemetry.coupling_count);
    println!("Attention: {}", telemetry.total_attention_allocated);
}
```

---

## What's Next?

1. **[Architecture Overview](architecture.md)** - Deep dive into Resonance Architecture
2. **[Core Concepts](concepts/)** - Detailed concept explanations
3. **[Resonator Layer](../crates/resonator/README.md)** - Full cognitive pipeline docs
4. **[Observability Guide](../crates/resonator/observability/README.md)** - Metrics and tracing
5. **[Conformance Tests](../crates/resonator/conformance/README.md)** - Invariant verification

---

## Need Help?

- **Docs**: [docs.mapleai.org](https://docs.mapleai.org)
- **GitHub**: [github.com/mapleaiorg/maple/issues](https://github.com/mapleaiorg/maple/issues)
- **Discord**: [discord.gg/maple-ai](https://discord.gg/maple-ai)

---

**Next**: [Architecture Overview →](architecture.md)
