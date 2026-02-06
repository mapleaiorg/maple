# 🍁 MAPLE AI Framework

**Multi-Agent Platform for Learning and Evolution**

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/mapleaiorg/maple)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org)

MAPLE is a production-ready AI framework built on **Resonance Architecture** - a fundamentally different approach to multi-agent systems that prioritizes meaningful relationships, architectural safety, and accountability.

## 🚀 Quick Start

```bash
# Clone and build
git clone https://github.com/mapleaiorg/maple.git
cd maple
cargo build --release

# Run your first example
cargo run -p maple-runtime --example 01_basic_resonator

# Start the control plane (optional)
cargo run -p maple-cli -- daemon start
```

### Your First Resonator

```rust
use maple_runtime::{MapleRuntime, ResonatorSpec, config::RuntimeConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Bootstrap runtime
    let runtime = MapleRuntime::bootstrap(RuntimeConfig::default()).await?;

    // Register a Resonator (intelligent entity)
    let resonator = runtime.register_resonator(ResonatorSpec::default()).await?;
    println!("Resonator created: {}", resonator.id);

    runtime.shutdown().await?;
    Ok(())
}
```

---

## 🏗️ Architecture Overview

MAPLE implements the **Resonance Pipeline** - a strict ordering of cognitive stages:

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                         RESONANCE PIPELINE                                   │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   Presence ──→ Coupling ──→ Meaning ──→ Intent ──→ Commitment ──→ Consequence│
│      │            │            │           │            │              │     │
│      ▼            ▼            ▼           ▼            ▼              ▼     │
│   Identity    Relation-    Semantic    Stabilized   Auditable      Tracked  │
│   verified    ships        under-      goals        promises       outcomes │
│               formed       standing                                          │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│   Memory System │ Observability │ Conformance Testing │ Protocol Adapters   │
└──────────────────────────────────────────────────────────────────────────────┘
```

### The 8 Runtime Invariants

These are **enforced at runtime** - violations cause errors, not silent failures:

| # | Invariant | Meaning |
|---|-----------|---------|
| 1 | Presence precedes Coupling | Must establish presence before forming relationships |
| 2 | Coupling precedes Meaning | Meaning only forms within established couplings |
| 3 | Meaning precedes Intent | Intent requires sufficient meaning convergence |
| 4 | Commitment precedes Consequence | No action without explicit, auditable commitment |
| 5 | Receipts are Immutable | Commitment receipts cannot be modified |
| 6 | Audit trail is Append-Only | Audit entries can only be added, never removed |
| 7 | Capabilities gate Actions | Actions require explicit capability grants |
| 8 | Time anchors are Monotonic | Temporal anchors always increase |

---

## 📦 Project Structure

```
maple/
├── crates/
│   ├── maple-runtime/          # Core Resonance Runtime
│   ├── maple-cli/              # Umbrella CLI (maple command)
│   │
│   ├── resonator/              # Resonator Layer (cognition/lifecycle)
│   │   ├── types/              # Core identity, presence, coupling types
│   │   ├── identity/           # Persistent identity & continuity
│   │   ├── meaning/            # Meaning formation engine
│   │   ├── intent/             # Intent stabilization engine
│   │   ├── commitment/         # Contract lifecycle & commitments
│   │   ├── consequence/        # Consequence tracking & attribution
│   │   ├── memory/             # Multi-tier memory system
│   │   ├── conversation/       # Multi-turn conversation management
│   │   ├── observability/      # Metrics, tracing, alerting
│   │   ├── conformance/        # Invariant verification tests
│   │   ├── profiles/           # Profile constraints (Human, World, etc.)
│   │   └── cli/                # Resonator CLI tools
│   │
│   ├── palm/                   # PALM Control Plane
│   │   ├── daemon/             # API server & control plane
│   │   ├── cli/                # Operations CLI
│   │   └── types/              # PALM types
│   │
│   ├── rcf-*/                  # Resonance Commitment Format
│   ├── aas-*/                  # Authority & Accountability Service
│   ├── mrp-*/                  # MAPLE Routing Protocol
│   ├── eve-*/                  # Evidence & Verification Engine
│   └── mapleverse/             # Mapleverse platform components
│
└── docs/                       # Documentation
```

---

## 🧠 Core Features

### Cognitive Pipeline

The full meaning-to-commitment pipeline with semantic understanding:

```rust
use resonator_meaning::MeaningFormationEngine;
use resonator_intent::IntentStabilizationEngine;
use resonator_commitment::ContractEngine;

// Form meaning from input
let meaning_engine = MeaningFormationEngine::new();
let meaning = meaning_engine.form_meaning(&input, &coupling_context).await?;

// Stabilize intent from meaning
let intent_engine = IntentStabilizationEngine::new();
let intent = intent_engine.stabilize(&meaning).await?;

// Create auditable commitment
let contract_engine = ContractEngine::new();
let commitment = contract_engine.create_commitment(intent).await?;
```

### Memory System

Multi-tier memory for intelligent context management:

```rust
use resonator_memory::{MemorySystem, MemoryEntry, MemoryTier};

let memory = MemorySystem::new();

// Store with appropriate tier
memory.store(MemoryEntry::new("key", content, MemoryTier::Working)).await?;

// Retrieve relevant memories
let memories = memory.retrieve_relevant(&context, 10).await?;

// Consolidate working → long-term
memory.consolidate().await?;
```

**Memory Tiers:**
- **Short-term**: Quick access, auto-expiring (recent interactions)
- **Working**: Active processing context (current task)
- **Long-term**: Persistent storage (learned patterns)
- **Episodic**: Experience sequences with emotional context

### Observability

Built-in metrics, tracing, and alerting:

```rust
use resonator_observability::{MetricsCollector, SpanTracker, AlertEngine};

let metrics = MetricsCollector::new();
let spans = SpanTracker::default();
let alerts = AlertEngine::default();

// Track pipeline metrics
metrics.record_pipeline_latency(PipelineStage::Meaning, 45.0);
metrics.record_commitment_created();

// Distributed tracing
let span = spans.start_span("commitment.validate");
// ... do work ...
spans.complete_span(&span.id)?;

// Configure alerts
alerts.add_rule(AlertRule {
    name: "high_failure_rate".into(),
    metric: "commitment.failed".into(),
    threshold: 5.0,
    severity: AlertSeverity::Warning,
    ..Default::default()
})?;
```

### Conformance Testing

Verify invariant compliance:

```rust
use resonator_conformance::{ConformanceSuite, Invariant};

let suite = ConformanceSuite::new(ConformanceConfig::default());

// Run all invariant tests
let report = suite.run_all();

if report.all_passed() {
    println!("✅ All 8 invariants verified - MAPLE compliant!");
} else {
    for test in report.failures() {
        println!("❌ {:?}: {}", test.invariant, test.error);
    }
}
```

---

## 🎮 Operations

### CLI Commands

```bash
# Daemon management
maple daemon start              # Start PALM daemon
maple daemon status             # Check daemon health
maple daemon stop               # Graceful shutdown

# System health
maple doctor                    # Run diagnostic checks

# Resonator management
maple resonator list            # List active resonators
maple resonator inspect <id>    # View resonator details

# Monitoring
maple events watch              # Real-time event stream
maple playground activities     # View recent activities

# Commitment management (via resonator CLI)
resonator commitment list       # List active commitments
resonator commitment lifecycle  # Show state machine
resonator consequence list      # View tracked consequences
```

### Starting the Daemon

```bash
# With PostgreSQL (default)
maple daemon start --platform mapleverse

# With in-memory storage (development)
maple daemon start --storage memory

# Environment override
PALM_STORAGE_TYPE=memory maple daemon start
```

### PostgreSQL Setup (Docker)

```bash
docker run --name maple-postgres \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=maple \
  -p 5432:5432 \
  -v maple_pgdata:/var/lib/postgresql/data \
  -d postgres:16
```

---

## 🎯 Platform Configurations

MAPLE supports three platform profiles with different safety constraints:

### Mapleverse (Pure AI Coordination)

```rust
use maple_runtime::config::mapleverse_runtime_config;

let runtime = MapleRuntime::bootstrap(mapleverse_runtime_config()).await?;
```

- No human profiles (pure AI agents)
- Strong commitment accountability
- Optimized for 100M+ concurrent agents

### Finalverse (Human-AI Coexistence)

```rust
use maple_runtime::config::finalverse_runtime_config;

let runtime = MapleRuntime::bootstrap(finalverse_runtime_config()).await?;
```

- Architectural human agency protection
- Coercion and exploitation detection
- Reversible consequences preferred

### iBank (Autonomous AI Finance)

```rust
use maple_runtime::config::ibank_runtime_config;

let runtime = MapleRuntime::bootstrap(ibank_runtime_config()).await?;
```

- AI-only (no human participants)
- Mandatory audit trails
- Risk-bounded decisions ($1M autonomous limit)

---

## 📚 Examples

```bash
# Basic resonator lifecycle
cargo run -p maple-runtime --example 01_basic_resonator

# Coupling dynamics
cargo run -p maple-runtime --example 02_resonator_coupling

# Platform configurations
cargo run -p maple-runtime --example 03_mapleverse_config
cargo run -p maple-runtime --example 04_finalverse_config
cargo run -p maple-runtime --example 05_ibank_config

# Multi-resonator coordination
cargo run -p maple-runtime --example 06_multi_resonator

# Cognitive pipeline (meaning → commitment)
cargo run -p maple-runtime --example 07_meaning_to_commitment

# Memory and conversation
cargo run -p maple-runtime --example 08_memory_and_conversation

# Observability demo
cargo run -p maple-runtime --example 09_observability_demo

# Conformance testing
cargo run -p maple-runtime --example 10_conformance_testing
```

---

## 🌟 Why MAPLE?

### vs. Traditional Agent Frameworks

| Aspect | Traditional | MAPLE |
|--------|-------------|-------|
| **Interactions** | Stateless messages | Stateful relationships (coupling) |
| **Identity** | Ephemeral | Persistent continuity |
| **Safety** | Policy-based | Architectural invariants |
| **Accountability** | Implicit trust | Commitment ledger with audit |
| **Semantics** | Function signatures | Emergent meaning |
| **Scale** | Thousands | 100M+ Resonators |

### Key Differentiators

1. **Resonance Over Messages** - Relationships that evolve and strengthen
2. **Architecture Over Policy** - Safety through invariants, not bypassable rules
3. **Attention Economics** - Finite budgets prevent abuse
4. **Commitment Accountability** - Every action has an audit trail
5. **Gradient Representations** - Presence, coupling, meaning are gradients, not binaries

---

## 🛠️ Development

### Prerequisites

- Rust 1.75+
- PostgreSQL (optional, for persistence)
- Docker (optional, for PostgreSQL)

### Building

```bash
cargo build --release          # Full workspace
cargo build -p maple-runtime   # Specific crate
cargo test --workspace         # Run all tests
cargo doc --workspace --open   # Generate docs
```

### Project Health

```bash
# Run all checks
cargo fmt --all -- --check
cargo clippy --workspace
cargo test --workspace

# Conformance verification
cargo test -p resonator-conformance
```

---

## 📖 Documentation

- **[Getting Started Guide](docs/getting-started.md)** - First steps with MAPLE
- **[Architecture Overview](docs/architecture.md)** - Deep dive into Resonance Architecture
- **[Resonator Layer](crates/resonator/README.md)** - Cognitive pipeline documentation
- **[CLI Reference](crates/resonator/cli/README.md)** - Command-line tools
- **[Observability Guide](crates/resonator/observability/README.md)** - Metrics and tracing
- **[Conformance Testing](crates/resonator/conformance/README.md)** - Invariant verification
- **[API Reference](https://docs.mapleai.org/api)** - Complete API docs

---

## 🤝 Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

- 🐛 **Report bugs** - Open an issue
- 💡 **Suggest features** - Share your ideas
- 📝 **Improve docs** - Help others understand
- 🔧 **Submit PRs** - Fix bugs or add features
- 🧪 **Write tests** - Increase coverage

---

## 📜 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

---

<div align="center">

**Built with 🍁 by the MAPLE Team**

*Making AI agents that resonate, not just respond*

[⭐ Star us](https://github.com/mapleaiorg/maple) • [📖 Docs](https://docs.mapleai.org) • [💬 Discord](https://discord.gg/maple-ai)

</div>
