# 🍁 MAPLE Resonance Runtime

**The World's Most Advanced Multi-Agent AI Framework**

MAPLE (Multi-Agent Platform for Learning and Evolution) is the foundational AI framework powering Mapleverse, Finalverse, and iBank. Built entirely on **Resonance Architecture** principles, MAPLE represents a fundamental paradigm shift from traditional agent frameworks.

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/mapleaiorg/maple)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org)

---

## 🚀 Why MAPLE Surpasses Google A2A and Anthropic MCP

### The Fundamental Difference

Traditional frameworks treat agents as **isolated processes** that communicate via messages:

```
Traditional: Agent A --[message]--> Agent B --[message]--> Agent C
```

MAPLE treats every entity as a **Resonator** participating in continuous, stateful **resonance**:

```
MAPLE:      Resonator A <==[coupling]==> Resonator B <==[coupling]==> Resonator C
                ↑                            ↑                            ↑
           [presence]                  [presence]                   [presence]
                ↓                            ↓                            ↓
           [meaning] -----------------> [meaning] -----------------> [meaning]
                ↓                            ↓                            ↓
           [intent] ------------------> [commitment] --------------> [consequence]
```

### Comparison Matrix

| Aspect                  | Google A2A          | Anthropic MCP    | **MAPLE**                      |
|-------------------------|---------------------|------------------|--------------------------------|
| **Core Model**          | Tool invocation     | Context injection| **Resonance relationships**    |
| **Identity**            | Ephemeral session   | None             | **Persistent continuity**      |
| **Relationships**       | Point-to-point      | None             | **Dynamic coupling**           |
| **Semantics**           | Function signatures | JSON schema      | **Emergent meaning**           |
| **Accountability**      | None                | None             | **Commitment ledger**          |
| **Learning**            | None                | None             | **Federated intelligence**     |
| **Safety Model**        | Policy-based        | Policy-based     | **Architectural invariants**   |
| **Human Protection**    | Implicit trust      | Implicit trust   | **Explicit agency preservation** |
| **Scale Target**        | Thousands           | Hundreds         | **100M+ concurrent Resonators** |

---

## ✨ Key Features

### 🏗️ **Resonance-Native Architecture**

Built from the ground up on the core flow:
```
presence → coupling → meaning → intent → commitment → consequence
```

### 🛡️ **8 Architectural Invariants**

Runtime-enforced safety guarantees that cannot be violated:

1. **Presence precedes meaning**: A Resonator must be present before forming/receiving meaning
2. **Meaning precedes intent**: Intent requires sufficient meaning
3. **Intent precedes commitment**: Commitments require stabilized intent
4. **Commitment precedes consequence**: No consequence without explicit commitment
5. **Coupling bounded by attention**: Coupling strength ≤ available attention
6. **Safety overrides optimization**: Safety constraints always take precedence
7. **Human agency cannot be bypassed**: Architectural (not policy) protection
8. **Failure must be explicit**: All failures are surfaced, never hidden

### ⚡ **Attention Economics**

- **Finite attention budgets** prevent runaway resource consumption
- **Graceful degradation** under high coupling pressure
- **Coercion prevention** through attention exhaustion detection
- **Automatic rebalancing** optimizes allocation

### 📜 **Commitment Accountability**

- **Every action is attributable** through commitment ledger
- **Audit trails** for financial operations (iBank)
- **Risk assessments** for consequential actions
- **Reversibility tracking** for experiential environments

### 👤 **Human Agency Protection**

- **Architectural enforcement** (not policy-based)
- **Presence does NOT imply willingness** to interact
- **Silent mode** allows existence without participation
- **Coercion detection** prevents manipulation
- **Always disengageable** couplings

---

## 🎯 Use Cases

### 🤖 **Mapleverse** - Pure AI Agent Coordination

```rust
use maple_runtime::{MapleRuntime, config::mapleverse_runtime_config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = mapleverse_runtime_config();
    let runtime = MapleRuntime::bootstrap(config).await?;

    // Register 100M+ concurrent AI agents
    // Each with finite attention, explicit commitments
    // No humans, pure AI-to-AI coordination

    Ok(())
}
```

**Characteristics:**
- No human profiles allowed
- Strong commitment accountability
- Optimized for massive scale
- Explicit coupling and intent

### 🌍 **Finalverse** - Human-AI Coexistence

```rust
use maple_runtime::{MapleRuntime, config::finalverse_runtime_config, ResonatorProfile};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = finalverse_runtime_config();
    let runtime = MapleRuntime::bootstrap(config).await?;

    // Create meaningful experiences where humans and AI coexist
    // Architectural guarantees of human agency preservation
    // Coercion detection and emotional exploitation prevention

    Ok(())
}
```

**Characteristics:**
- Human agency protection (architectural)
- Coercion detection enabled
- Reversible consequences preferred
- Experiential focus

### 🏦 **iBank** - Autonomous AI Finance

```rust
use maple_runtime::{MapleRuntime, config::ibank_runtime_config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ibank_runtime_config();
    let runtime = MapleRuntime::bootstrap(config).await?;

    // AI-only financial system
    // Every transaction has audit trail
    // Risk-bounded autonomous decisions
    // Strict accountability

    Ok(())
}
```

**Characteristics:**
- AI-only (no humans)
- Mandatory audit trails
- Risk assessments required
- Reversibility preferred
- Strict accountability

---

## 🚀 Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
maple-runtime = "0.1.1"
tokio = { version = "1.0", features = ["full"] }
```

### Basic Usage

```rust
use maple_runtime::{MapleRuntime, ResonatorSpec, config::RuntimeConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Bootstrap runtime
    let config = RuntimeConfig::default();
    let runtime = MapleRuntime::bootstrap(config).await?;

    // Register a Resonator
    let spec = ResonatorSpec::default();
    let resonator = runtime.register_resonator(spec).await?;

    // Signal presence
    let presence = maple_runtime::PresenceState::new();
    resonator.signal_presence(presence).await?;

    // Shutdown gracefully
    runtime.shutdown().await?;

    Ok(())
}
```

---

## 📚 Examples

Explore comprehensive examples in the `examples/` directory:

- **`01_basic_resonator.rs`** - Fundamental concepts
- **`02_resonator_coupling.rs`** - Coupling dynamics and attention economics
- **`03_mapleverse_config.rs`** - Pure AI coordination
- **`04_finalverse_config.rs`** - Human-AI coexistence
- **`05_ibank_config.rs`** - Autonomous finance (coming soon)
- **`06_agent_kernel_boundary.rs`** - iBank transfer boundary demo (`ContractMissing` negative path + contract-authorized execution + durable receipt)

Run any example:
```bash
cargo run --example 01_basic_resonator
```

---

## 🏛️ Architecture

### Core Components

#### **MapleRuntime** - The Heart of MAPLE

The central orchestrator managing all Resonators, fabrics, and subsystems:

```rust
pub struct MapleRuntime {
    // Resonator Management
    resonator_registry: ResonatorRegistry,
    profile_manager: ProfileManager,

    // Resonance Infrastructure
    presence_fabric: PresenceFabric,
    coupling_fabric: CouplingFabric,
    attention_allocator: AttentionAllocator,

    // Safety and Governance
    invariant_guard: InvariantGuard,
    agency_protector: HumanAgencyProtector,

    // Temporal Coordination
    temporal_coordinator: TemporalCoordinator,

    // Scheduling
    scheduler: ResonanceScheduler,
}
```

#### **PresenceFabric** - Gradient Presence Management

Presence is **NOT binary** (online/offline). It's multidimensional:

- **Discoverability** (0.0-1.0): How findable is this Resonator?
- **Responsiveness** (0.0-1.0): How quickly does it respond?
- **Stability** (0.0-1.0): How consistently available?
- **Coupling Readiness** (0.0-1.0): Willing to form new couplings?
- **Silent Mode** (bool): Present but not actively signaling

#### **CouplingFabric** - Relationship Management

Manages all coupling relationships with:

- **Gradual strengthening** (max 0.3 initial, 0.1 per step)
- **Attention binding** (coupling ≤ available attention)
- **Meaning convergence tracking**
- **Safe decoupling** without violating commitments

#### **AttentionAllocator** - Resource Management

Finite attention budgets prevent:

- **Runaway coupling** (unlimited relationships)
- **Attention exhaustion attacks**
- **Resource depletion**

Enables:

- **Graceful degradation** under pressure
- **Automatic rebalancing**
- **Circuit breakers** for overload

#### **InvariantGuard** - Safety Enforcement

Runtime enforcement of the 8 canonical invariants. Any violation = system error.

#### **TemporalCoordinator** - Causal Ordering

**No global clocks**. Time is defined relationally through:

- **Temporal anchors**: Events that enable ordering
- **Causal dependencies**: Happened-before relationships
- **Local timelines**: Per-Resonator sequences

---

## 🎓 Core Concepts

### Resonator

A **Resonator** is any entity with:
- **Persistent identity** (survives restarts)
- **Presence gradient** (not binary)
- **Attention budget** (finite capacity)
- **Coupling affinity** (relationship preferences)
- **Profile** (Human, World, Coordination, iBank)

### Coupling

A **coupling** is NOT a message channel. It's a stateful relationship with:

- **Strength** (0.0-1.0, must strengthen gradually)
- **Persistence** (Transient, Session, Persistent)
- **Scope** (Full, StateOnly, IntentOnly, ObservationalOnly)
- **Symmetry** (Symmetric, Asymmetric)
- **Attention cost** (bounded by available attention)
- **Meaning convergence** (how well understanding aligns)

### Attention

**Attention** is the finite capacity to process resonance. It:

- **Bounds coupling** (architectural invariant #5)
- **Prevents exhaustion** through safety reserves
- **Enables degradation** when pressure is high
- **Tracks utilization** for monitoring

### Commitment

A **commitment** is an explicit promise with:

- **Content** (Action, State, Boundary, Result)
- **Audit trail** (for accountability)
- **Risk assessment** (for financial operations)
- **Status tracking** (Pending, Active, Fulfilled, Violated, Revoked)

### Temporal Anchors

**Time** is relational, not absolute:

- **No synchronized clocks** required
- **Causal ordering** through dependencies
- **Local timestamps** per-Resonator
- **Happened-before** relationships

---

## 🔬 Performance

MAPLE is designed for **extreme scale**:

| Metric                        | Target      | Notes                         |
|-------------------------------|-------------|-------------------------------|
| **Resonator Registration**    | <1ms        | Cold start                    |
| **Resonator Resume**          | <5ms        | From continuity record        |
| **Coupling Establishment**    | <5ms        | Initial coupling              |
| **Coupling Strengthening**    | <1ms        | Incremental                   |
| **Attention Allocation**      | <100μs      | Per allocation                |
| **Invariant Check**           | <10μs       | Per operation                 |
| **Presence Signal**           | <500μs      | Broadcast                     |
| **Concurrent Resonators**     | 100,000+    | Per node                      |
| **Target Scale**              | 100M+       | Across distributed deployment |

---

## 🛠️ Development

### Building

```bash
cargo build --release
```

### Testing

```bash
cargo test
```

### Documentation

```bash
cargo doc --open
```

### Benchmarking

```bash
cargo bench
```

---

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## 📜 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

---

## 🌟 Why MAPLE is the Future of AI Agents

### 1. **Resonance > Messages**

Traditional frameworks pass messages. MAPLE creates **meaningful relationships** that evolve over time.

### 2. **Architecture > Policy**

Safety through **architectural invariants**, not policies that can be bypassed.

### 3. **Attention > Unlimited**

**Finite attention** creates natural bounds, prevents abuse, enables graceful degradation.

### 4. **Commitment > Implicit**

**Every consequential action** requires explicit commitment with audit trail.

### 5. **Gradient > Binary**

Presence, coupling strength, meaning convergence - all are **gradients**, not binaries.

### 6. **Causal > Clock**

**Relational time** through causal ordering, no global clock required.

### 7. **Agency > Trust**

**Architectural protection** of human agency, not trusting policies.

### 8. **Scale > Thousands**

Designed from day one for **100M+ concurrent Resonators**.

---

## 📞 Contact

- **Website**: https://mapleai.org
- **Documentation**: https://docs.mapleai.org
- **Issues**: https://github.com/mapleaiorg/maple/issues
- **Discord**: https://discord.gg/maple-ai

---

<div align="center">

**Built with 🍁 by the MAPLE Team**

*Making AI agents that resonate, not just respond*

</div>
