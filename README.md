# 🍁 MAPLE AI Framework

**The World's Most Advanced Multi-Agent AI Platform**

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/mapleaiorg/maple)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org)

MAPLE (Multi-Agent Platform for Learning and Evolution) is a revolutionary AI framework built entirely on **Resonance Architecture** principles. It powers three transformative platforms:

- 🤖 **Mapleverse**: Pure AI agent coordination (100M+ concurrent agents)
- 🌍 **Finalverse**: Meaningful human-AI coexistence
- 🏦 **iBank**: Autonomous AI-only financial system

---

## 🚀 Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/mapleaiorg/maple.git
cd maple

# Build the entire workspace
cargo build --release

# Run tests
cargo test

# Try an example
cargo run -p maple-runtime --example 01_basic_resonator
```

### Your First MAPLE Application

```rust
use maple_runtime::{MapleRuntime, ResonatorSpec, config::RuntimeConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Bootstrap the MAPLE Resonance Runtime
    let config = RuntimeConfig::default();
    let runtime = MapleRuntime::bootstrap(config).await?;

    // Register a Resonator
    let spec = ResonatorSpec::default();
    let resonator = runtime.register_resonator(spec).await?;

    println!("Resonator created: {}", resonator.id);

    // Graceful shutdown
    runtime.shutdown().await?;
    Ok(())
}
```

### Operations: CLI and Playground

MAPLE can run fully headless (runtime-only) or with the PALM control plane + Playground UI.

- **`maple`** is the umbrella CLI. Use `maple palm ...` for operations and `maple` for developer utilities.
- **`palm`** still exists as a direct operations CLI (backwards-compatible with `palm-cli`).
- The **Playground** is optional and provides a live, game-like view plus history replay for humans and web observers.

Examples:

```bash
# Start the PALM daemon (API + control plane)
cargo run -p palm-daemon

# Real-time monitoring in the terminal (umbrella CLI)
cargo run -p maple-cli -- palm events watch
cargo run -p maple-cli -- palm playground activities --limit 50

# Direct operations CLI (optional)
cargo run -p palm-cli -- events watch

# Open the web dashboard (optional)
open http://localhost:8080/playground
```

---

## 📦 Project Structure

This monorepo contains the complete MAPLE ecosystem:

### 🧠 Core Runtime

#### **[maple-runtime](crates/maple-runtime/)** - The Heart of MAPLE
The foundational Resonance Runtime powering all MAPLE platforms.

**Features:**
- Resonance-native architecture (presence → coupling → meaning → intent → commitment → consequence)
- 8 runtime-enforced architectural invariants
- Attention economics with finite budgets
- Gradient presence (multidimensional, not binary)
- Temporal coordination without global clocks
- Platform-specific configurations (Mapleverse, Finalverse, iBank)

**[📖 Read the maple-runtime documentation](crates/maple-runtime/README.md)**

### 🎭 Resonator Layer

**[resonator-types](crates/resonator-types/)** - Core types for Resonators
- Identity, presence, coupling definitions
- Profile types (Human, World, Coordination, IBank)

**[resonator-runtime](crates/resonator-runtime/)** - Resonator execution engine
- Resonator lifecycle management
- State management
- Event processing

**[resonator-identity](crates/resonator-identity/)** - Identity and continuity
- Persistent identity across restarts
- Continuity proofs
- Identity verification

**[resonator-meaning](crates/resonator-meaning/)** - Meaning formation
- Semantic understanding
- Context building
- Meaning convergence tracking

**[resonator-intent](crates/resonator-intent/)** - Intent stabilization
- Intent formation from meaning
- Intent validation
- Intent tracking

**[resonator-commitment](crates/resonator-commitment/)** - Commitment management
- Commitment creation and tracking
- Audit trails
- Consequence management

**[resonator-profiles](crates/resonator-profiles/)** - Profile system
- Profile validation
- Cross-profile rules
- Safety constraints

**[resonator-client](crates/resonator-client/)** - Client libraries
- High-level API for applications
- Simplified Resonator interaction

### 📜 Resonance Contract Language (RCL)

- **[rcl-types](crates/rcl-types/)** - RCL type system
- **[rcl-meaning](crates/rcl-meaning/)** - Meaning expression
- **[rcl-intent](crates/rcl-intent/)** - Intent declaration
- **[rcl-commitment](crates/rcl-commitment/)** - Commitment specification
- **[rcl-validator](crates/rcl-validator/)** - Contract validation
- **[rcl-compiler](crates/rcl-compiler/)** - RCL compiler
- **[rcl-audit](crates/rcl-audit/)** - Audit trail generation

### 🌐 MAPLE Routing Protocol (MRP)

- **[mrp-types](crates/mrp-types/)** - MRP type definitions
- **[mrp-router](crates/mrp-router/)** - Resonance routing
- **[mrp-transport](crates/mrp-transport/)** - Transport layer
- **[mrp-service](crates/mrp-service/)** - MRP service

### 🛡️ Authority & Accountability Service (AAS)

- **[aas-types](crates/aas-types/)** - AAS type system
- **[aas-identity](crates/aas-identity/)** - Identity management
- **[aas-capability](crates/aas-capability/)** - Capability system
- **[aas-policy](crates/aas-policy/)** - Policy enforcement
- **[aas-adjudication](crates/aas-adjudication/)** - Dispute resolution
- **[aas-ledger](crates/aas-ledger/)** - Accountability ledger
- **[aas-service](crates/aas-service/)** - AAS service

### 🌍 Mapleverse Platform

- **[mapleverse-types](crates/mapleverse-types/)** - Platform types
- **[mapleverse-executor](crates/mapleverse-executor/)** - Agent execution
- **[mapleverse-connectors](crates/mapleverse-connectors/)** - External integrations
- **[mapleverse-evidence](crates/mapleverse-evidence/)** - Evidence collection
- **[mapleverse-service](crates/mapleverse-service/)** - Mapleverse service

### 📚 Evidence & Verification Engine (EVE)

- **[eve-types](crates/eve-types/)** - EVE type system
- **[eve-ingestion](crates/eve-ingestion/)** - Data ingestion
- **[eve-evaluation](crates/eve-evaluation/)** - Evidence evaluation
- **[eve-artifacts](crates/eve-artifacts/)** - Artifact management
- **[eve-service](crates/eve-service/)** - EVE service

### 🔧 Integration & Tools

- **[maple-integration](crates/maple-integration/)** - Integration tests
- **[maple-cli](crates/maple-cli/)** - Umbrella CLI (`maple` + `maple palm ...`)
- **[palm-cli](crates/palm-cli/)** - Direct operations CLI (backwards compatible)
- **[palm-daemon](crates/palm-daemon/)** - Control plane + API service

---

## 🏗️ Architecture

### The Resonance Architecture

MAPLE is built on a fundamentally different paradigm than traditional agent frameworks:

```
Traditional Agent Frameworks:

  ┌─────────┐   message    ┌─────────┐   message    ┌─────────┐
  │ Agent A │ ───────────> │ Agent B │ ───────────> │ Agent C │
  └─────────┘              └─────────┘              └─────────┘
      ↓                         ↓                       ↓
   Stateless                 Stateless               Stateless
   Isolated                  Isolated                Isolated

-----------------------------------------------------------------

MAPLE Resonance Architecture:

┌─────────────┐ coupling ┌─────────────┐ coupling ┌──────────────┐
│ Resonator A │<========>│ Resonator B │<========>│ Resonator C  │
│             │          │             │          │              │
│ [presence]  │          │ [presence]  │          │ [presence]   │
│      ↓      │          │      ↓      │          │      ↓       │
│ [meaning] ──┼─────────>│ [meaning] ──┼─────────>│ [meaning]    │
│      ↓      │          │      ↓      │          │      ↓       │
│ [intent] ───┼─────────>│ [commitment]│─────────>│ [consequence]│
└─────────────┘          └─────────────┘          └──────────────┘
      ↓                         ↓                        ↓
   Stateful                  Stateful                 Stateful
   Relationship              Relationship             Relationship
```

### Core Flow

Every interaction in MAPLE follows this architectural flow:

1. **Presence** - Gradient, multidimensional (NOT binary online/offline)
2. **Coupling** - Stateful relationships that strengthen gradually
3. **Meaning** - Semantic understanding that converges over time
4. **Intent** - Stabilized goals formed from sufficient meaning
5. **Commitment** - Explicit promises with audit trails
6. **Consequence** - Attributable outcomes from commitments

### The 8 Architectural Invariants

These invariants are **enforced at runtime** and violations constitute system errors:

1. **Presence precedes meaning** - Must be present before forming/receiving meaning
2. **Meaning precedes intent** - Intent requires sufficient meaning
3. **Intent precedes commitment** - Commitments require stabilized intent
4. **Commitment precedes consequence** - No consequence without explicit commitment
5. **Coupling bounded by attention** - Coupling strength ≤ available attention
6. **Safety overrides optimization** - Safety constraints always take precedence
7. **Human agency cannot be bypassed** - Architectural (not policy) protection
8. **Failure must be explicit** - All failures surfaced, never hidden

---

## 🎯 Platform Configurations

### 🤖 Mapleverse - Pure AI Agent Coordination

```rust
use maple_runtime::{MapleRuntime, config::mapleverse_runtime_config};

let config = mapleverse_runtime_config();
let runtime = MapleRuntime::bootstrap(config).await?;
```

**Characteristics:**
- No human profiles allowed (pure AI)
- Strong commitment accountability
- Explicit coupling and intent
- Optimized for 100M+ concurrent agents
- Federated collective intelligence

**Use Cases:**
- Autonomous agent swarms
- Distributed AI coordination
- Multi-agent reinforcement learning
- Agent marketplaces

### 🌍 Finalverse - Human-AI Coexistence

```rust
use maple_runtime::{MapleRuntime, config::finalverse_runtime_config};

let config = finalverse_runtime_config();
let runtime = MapleRuntime::bootstrap(config).await?;
```

**Characteristics:**
- Architectural human agency protection
- Coercion detection enabled
- Emotional exploitation prevention
- Reversible consequences preferred
- Experiential focus

**Use Cases:**
- Virtual worlds
- AI companions
- Interactive storytelling
- Educational environments
- Therapeutic applications

### 🏦 iBank - Autonomous AI Finance

```rust
use maple_runtime::{MapleRuntime, config::ibank_runtime_config};

let config = ibank_runtime_config();
let runtime = MapleRuntime::bootstrap(config).await?;
```

**Characteristics:**
- AI-only (no humans)
- Mandatory audit trails
- Risk assessments required
- Risk-bounded decisions ($1M autonomous limit)
- Strict accountability

**Use Cases:**
- Autonomous trading systems
- AI-managed portfolios
- Decentralized finance
- Algorithmic market making
- Risk management

---

## 🌟 Why MAPLE?

### vs. Google A2A and Anthropic MCP

| Aspect | Google A2A | Anthropic MCP | **MAPLE** |
|--------|------------|---------------|-----------|
| **Core Model** | Tool invocation | Context injection | **Resonance relationships** |
| **Identity** | Ephemeral | None | **Persistent continuity** |
| **Relationships** | Point-to-point | None | **Dynamic coupling** |
| **Semantics** | Function signatures | JSON schema | **Emergent meaning** |
| **Accountability** | None | None | **Commitment ledger** |
| **Learning** | None | None | **Federated intelligence** |
| **Safety** | Policy-based | Policy-based | **Architectural invariants** |
| **Human Protection** | Implicit trust | Implicit trust | **Explicit preservation** |
| **Scale Target** | Thousands | Hundreds | **100M+ Resonators** |

### Key Differentiators

#### 1. **Resonance Over Messages**
Not ephemeral messages - stateful relationships that evolve and strengthen over time.

#### 2. **Architecture Over Policy**
Safety through architectural invariants, not policies that can be bypassed.

#### 3. **Attention Economics**
Finite attention budgets prevent abuse and enable graceful degradation.

#### 4. **Gradient Representations**
Presence, coupling, meaning - all are gradients, not binaries.

#### 5. **Commitment Accountability**
Every consequential action has an audit trail and is attributable.

#### 6. **Human Agency Guarantees**
Architectural protection - humans can always disengage.

#### 7. **Causal Time**
No global clocks - causal ordering through temporal anchors.

#### 8. **Extreme Scale**
Designed from day one for 100M+ concurrent Resonators.

---

## 📚 Documentation

### For Users

- **[Getting Started Guide](docs/getting-started.md)** - Your first MAPLE application
- **[Operations Tutorial](docs/tutorials/operations.md)** - Daemon, CLI, and Playground workflows
- **[Architecture Overview](docs/architecture.md)** - Understanding Resonance Architecture
- **[Platform Guides](docs/platforms/)** - Mapleverse, Finalverse, iBank
- **[API Reference](https://docs.maple.ai/api)** - Complete API documentation
- **[Examples](crates/maple-runtime/examples/)** - Working code examples

### For Contributors

- **[Contributing Guide](CONTRIBUTING.md)** - How to contribute
- **[Development Setup](docs/development.md)** - Setting up your environment
- **[Architecture Decision Records](docs/adr/)** - Design decisions
- **[Roadmap](ROADMAP.md)** - Future plans

### Core Concepts

- **[Resonators](docs/concepts/resonators.md)** - Persistent intelligent entities
- **[Coupling](docs/concepts/coupling.md)** - Stateful relationships
- **[Attention](docs/concepts/attention.md)** - Resource economics
- **[Commitments](docs/concepts/commitments.md)** - Accountability system
- **[Temporal Anchors](docs/concepts/temporal.md)** - Causal ordering
- **[Profiles](docs/concepts/profiles.md)** - Different modes of operation

---

## 🚀 Performance

MAPLE is designed for **extreme scale**:

| Metric | Target | Status |
|--------|--------|--------|
| Resonator Registration | <1ms | ✅ |
| Coupling Establishment | <5ms | ✅ |
| Attention Allocation | <100μs | ✅ |
| Invariant Check | <10μs | ✅ |
| Concurrent Resonators (per node) | 100,000+ | ✅ |
| Total Scale | 100M+ | 🎯 |

---

## 🛠️ Development

### Prerequisites

- Rust 1.75 or higher
- Tokio async runtime
- PostgreSQL (for persistence)

### Building from Source

```bash
# Clone repository
git clone https://github.com/mapleaiorg/maple.git
cd maple

# Build entire workspace
cargo build --release

# Build specific crate
cargo build -p maple-runtime --release

# Run tests
cargo test --workspace

# Run tests for specific crate
cargo test -p maple-runtime

# Generate documentation
cargo doc --workspace --no-deps --open
```

### Running Examples

```bash
# Basic Resonator example
cargo run -p maple-runtime --example 01_basic_resonator

# Coupling dynamics
cargo run -p maple-runtime --example 02_resonator_coupling

# Mapleverse configuration
cargo run -p maple-runtime --example 03_mapleverse_config

# Finalverse configuration
cargo run -p maple-runtime --example 04_finalverse_config

# iBank configuration
cargo run -p maple-runtime --example 05_ibank_config
```

---

## 🗺️ Roadmap

### Phase 1: Foundation (Current)
- ✅ MAPLE Resonance Runtime
- ✅ 8 Architectural Invariants
- ✅ Attention Economics
- ✅ Gradient Presence
- ✅ Coupling Dynamics
- ✅ Temporal Coordination
- ✅ Platform Configurations

### Phase 2: Cognitive Pipeline (Q2 2026)
- 🚧 Meaning Formation Engine
- 🚧 Intent Stabilization Engine
- 🚧 Commitment Manager
- 🚧 Consequence Tracker
- 🚧 Human Agency Protector
- 🚧 Safety Boundary Enforcer

### Phase 3: Distribution (Q3 2026)
- ⏳ Distributed runtime (multi-node)
- ⏳ Persistence layer
- ⏳ Cross-runtime resonance
- ⏳ Federated learning
- ⏳ Web UI dashboard

### Phase 4: Platforms (Q4 2026)
- ⏳ Mapleverse alpha
- ⏳ Finalverse alpha
- ⏳ iBank alpha
- ⏳ Platform integrations

### Phase 5: Ecosystem (2027)
- ⏳ WASM target
- ⏳ Mobile SDKs
- ⏳ Cloud deployment
- ⏳ Enterprise features

---

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Ways to Contribute

- 🐛 **Report bugs** - Open an issue
- 💡 **Suggest features** - Share your ideas
- 📝 **Improve docs** - Help others understand
- 🔧 **Submit PRs** - Fix bugs or add features
- 🧪 **Write tests** - Increase coverage
- 🎨 **Create examples** - Show what's possible

---

## 📜 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

---

## 🙏 Acknowledgments

MAPLE is built on the principles of **Resonance Architecture**, a novel approach to multi-agent AI systems that prioritizes:

- Meaningful relationships over message passing
- Architectural safety over policy enforcement
- Attention economics over unlimited resources
- Causal time over synchronized clocks
- Commitment accountability over implicit trust

---

## 📞 Contact

- **Website**: https://maple.ai
- **Documentation**: https://docs.maple.ai
- **GitHub**: https://github.com/mapleaiorg/maple
- **Discord**: https://discord.gg/maple-ai
- **Twitter**: [@MapleAI](https://twitter.com/MapleAI)
- **Email**: hello@maple.ai

---

<div align="center">

**Built with 🍁 by the MAPLE Team**

*Making AI agents that resonate, not just respond*

[⭐ Star us on GitHub](https://github.com/mapleaiorg/maple) • [📖 Read the Docs](https://docs.maple.ai) • [💬 Join Discord](https://discord.gg/maple-ai)

</div>
