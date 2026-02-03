# 🍁 Getting Started with MAPLE

Welcome to MAPLE (Multi-Agent Platform for Learning and Evolution) - the world's most advanced multi-agent AI framework built on Resonance Architecture principles.

## What You'll Build

In this guide, you'll create your first MAPLE application that:
- Bootstraps the MAPLE Resonance Runtime
- Registers Resonators (intelligent entities)
- Establishes stateful couplings (relationships)
- Demonstrates attention economics
- Shows how architectural invariants protect your system

## Prerequisites

- **Rust 1.75 or higher** - [Install Rust](https://www.rust-lang.org/tools/install)
- **Basic Rust knowledge** - Understanding of async/await, Result types
- **PostgreSQL** (optional) - Only needed for persistence features

## Installation

### Create a New Project

```bash
cargo new my-maple-app
cd my-maple-app
```

### Add MAPLE Dependencies

Edit your `Cargo.toml`:

```toml
[dependencies]
maple-runtime = "0.1.1"
tokio = { version = "1.35", features = ["full"] }
tracing-subscriber = "0.3"
```

## Your First MAPLE Application

Create `src/main.rs`:

```rust
use maple_runtime::{MapleRuntime, ResonatorSpec, config::RuntimeConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🍁 Bootstrapping MAPLE Runtime...\n");

    // 1. Create runtime configuration
    let config = RuntimeConfig::default();

    // 2. Bootstrap the runtime
    let runtime = MapleRuntime::bootstrap(config).await?;
    println!("✅ Runtime ready\n");

    // 3. Register your first Resonator
    let spec = ResonatorSpec::default();
    let resonator = runtime.register_resonator(spec).await?;
    println!("✅ Resonator registered: {}\n", resonator.id);

    // 4. Signal presence
    let presence = maple_runtime::PresenceState::new();
    resonator.signal_presence(presence).await?;
    println!("✅ Presence signaled\n");

    // 5. Graceful shutdown
    runtime.shutdown().await?;
    println!("🎉 Completed successfully!");

    Ok(())
}
```

### Run Your Application

```bash
cargo run
```

You should see:

```
🍁 Bootstrapping MAPLE Runtime...

✅ Runtime ready

✅ Resonator registered: res_abc123...

✅ Presence signaled

🎉 Completed successfully!
```

## Optional: Operate with PALM (Daemon + CLI + Playground)

MAPLE can run headless, but if you want a control plane with persistence, monitoring, and a live UI, start the PALM daemon and use the PALM CLI:

```bash
# Start the PALM daemon (API + control plane)
cargo run -p palm-daemon

# Check connectivity
cargo run -p maple-cli -- status

# Real-time monitoring
cargo run -p maple-cli -- events watch
cargo run -p maple-cli -- playground activities --limit 50
```

The Playground UI is optional and available at:

```bash
http://127.0.0.1:8080/playground
```

## Understanding What Just Happened

### 1. Runtime Bootstrap

```rust
let runtime = MapleRuntime::bootstrap(config).await?;
```

This initializes:
- **InvariantGuard** - Enforces 8 architectural invariants
- **PresenceFabric** - Manages gradient presence
- **CouplingFabric** - Manages stateful relationships
- **AttentionAllocator** - Manages finite attention budgets
- **TemporalCoordinator** - Causal ordering without clocks
- **ResonanceScheduler** - Attention-aware task scheduling

### 2. Resonator Registration

```rust
let resonator = runtime.register_resonator(spec).await?;
```

A **Resonator** is created with:
- **Persistent identity** - Survives restarts
- **Attention budget** - Finite capacity (default: 1000.0)
- **Profile** - Determines safety constraints
- **Presence state** - Gradient, not binary

### 3. Presence Signaling

```rust
resonator.signal_presence(presence).await?;
```

**Presence is NOT binary** (online/offline). It's multidimensional:
- **Discoverability** (0.0-1.0) - How findable?
- **Responsiveness** (0.0-1.0) - How quick to respond?
- **Stability** (0.0-1.0) - How consistently available?
- **Coupling Readiness** (0.0-1.0) - Willing to form relationships?

## Next Steps: Creating Relationships

Let's establish a **coupling** (stateful relationship) between two Resonators:

```rust
use maple_runtime::{MapleRuntime, ResonatorSpec, CouplingParams};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = RuntimeConfig::default();
    let runtime = MapleRuntime::bootstrap(config).await?;

    // Register two Resonators
    let alice = runtime.register_resonator(ResonatorSpec::default()).await?;
    let bob = runtime.register_resonator(ResonatorSpec::default()).await?;

    println!("👤 Alice: {}", alice.id);
    println!("👤 Bob: {}\n", bob.id);

    // Signal presence (required before coupling)
    alice.signal_presence(PresenceState::new()).await?;
    bob.signal_presence(PresenceState::new()).await?;

    // Establish coupling
    let params = CouplingParams {
        source: alice.id.clone(),
        target: bob.id.clone(),
        initial_strength: 0.3,  // Max allowed initially
        initial_attention_cost: 100.0,
        ..Default::default()
    };

    let coupling = runtime.establish_coupling(params).await?;
    println!("✅ Coupling established: {}\n", coupling.id);

    // Strengthen coupling gradually (architectural requirement)
    coupling.strengthen(0.1).await?;
    println!("✅ Coupling strengthened to 0.4\n");

    // Check attention impact
    if let Some(budget) = alice.attention_status().await {
        println!("⚡ Alice's attention: {}/{} available\n",
            budget.available, budget.total_capacity);
    }

    runtime.shutdown().await?;
    Ok(())
}
```

### Key Concepts Demonstrated

#### Gradual Strengthening

```rust
coupling.strengthen(0.1).await?;  // Can only strengthen by 0.1 per step
```

**Why?** Prevents aggressive coupling that could:
- Consume attention too quickly
- Create unstable relationships
- Bypass safety checks

#### Attention Economics

```rust
initial_attention_cost: 100.0,  // This attention is now bound to the coupling
```

**Attention** is finite. Each coupling **consumes** attention:
- Prevents unlimited relationships
- Enables graceful degradation under pressure
- Creates natural bounds on coordination

#### Invariant Enforcement

The runtime **automatically enforces**:
- ✅ Presence precedes coupling (you must signal presence first)
- ✅ Coupling bounded by attention (can't exceed available attention)
- ✅ Gradual strengthening (can't jump from 0.3 to 1.0)

## Platform Configurations

### Mapleverse - Pure AI Coordination

For autonomous AI agents (no humans):

```rust
use maple_runtime::config::mapleverse_runtime_config;

let config = mapleverse_runtime_config();
let runtime = MapleRuntime::bootstrap(config).await?;

// Only Coordination profile Resonators allowed
let mut spec = ResonatorSpec::default();
spec.profile = ResonatorProfile::Coordination;
let agent = runtime.register_resonator(spec).await?;
```

**Characteristics:**
- No human profiles allowed
- Strong commitment accountability
- Explicit coupling and intent required
- Optimized for 100M+ concurrent agents

### Finalverse - Human-AI Coexistence

For meaningful human-AI experiences:

```rust
use maple_runtime::config::finalverse_runtime_config;

let config = finalverse_runtime_config();
let runtime = MapleRuntime::bootstrap(config).await?;

// Can register both Human and World profiles
let mut human_spec = ResonatorSpec::default();
human_spec.profile = ResonatorProfile::Human;
let human = runtime.register_resonator(human_spec).await?;

let mut ai_spec = ResonatorSpec::default();
ai_spec.profile = ResonatorProfile::World;
let ai = runtime.register_resonator(ai_spec).await?;
```

**Characteristics:**
- **Human agency protection** (architectural)
- **Coercion detection** enabled
- **Emotional exploitation prevention**
- Reversible consequences preferred

### iBank - Autonomous Finance

For AI-only financial operations:

```rust
use maple_runtime::config::ibank_runtime_config;

let config = ibank_runtime_config();
let runtime = MapleRuntime::bootstrap(config).await?;

// Only IBank profile allowed
let mut spec = ResonatorSpec::default();
spec.profile = ResonatorProfile::IBank;
let financial_agent = runtime.register_resonator(spec).await?;
```

**Characteristics:**
- AI-only (no humans)
- **Mandatory audit trails**
- **Risk assessments** required
- Risk-bounded decisions ($1M limit)
- Strict accountability

## Common Patterns

### Pattern 1: Resuming from Identity

Resonators have **persistent identity**:

```rust
// First session - register
let spec = ResonatorSpec::default();
let resonator = runtime.register_resonator(spec).await?;
let id = resonator.id.clone();

// Save identity for later
// ...

// Later session - resume
let resumed = runtime.resume_resonator(id).await?;
// Same identity, continuity preserved
```

### Pattern 2: Silent Mode

Present but not actively signaling:

```rust
let mut presence = PresenceState::new();
presence.silent_mode = true;  // Present but quiet
presence.discoverability = 0.1;  // Hard to find

resonator.signal_presence(presence).await?;
```

### Pattern 3: Attention Management

Monitor and rebalance attention:

```rust
if let Some(budget) = resonator.attention_status().await {
    println!("Total: {}", budget.total_capacity);
    println!("Available: {}", budget.available);
    println!("Allocated: {}", budget.allocated);

    if budget.available < 100.0 {
        // Low attention - decouple or wait
        println!("⚠️ Low attention available");
    }
}
```

### Pattern 4: Safe Decoupling

End relationships without violating commitments:

```rust
// Decouple when done
coupling.decouple().await?;

// Attention is released back to the Resonator
```

## Error Handling

MAPLE uses explicit error types:

```rust
match resonator.signal_presence(presence).await {
    Ok(_) => println!("✅ Presence signaled"),
    Err(PresenceError::RateLimitExceeded) => {
        println!("⚠️ Signaling too frequently, wait before retrying");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    Err(e) => return Err(e.into()),
}
```

**Invariant #8**: Failure must be explicit. No silent failures.

## Testing Your Application

Add tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_resonator() {
        let config = RuntimeConfig::default();
        let runtime = MapleRuntime::bootstrap(config).await.unwrap();

        let spec = ResonatorSpec::default();
        let resonator = runtime.register_resonator(spec).await.unwrap();

        assert!(!resonator.id.to_string().is_empty());

        runtime.shutdown().await.unwrap();
    }
}
```

Run tests:

```bash
cargo test
```

## Performance Considerations

MAPLE is designed for **extreme scale**:

| Operation | Target Performance |
|-----------|-------------------|
| Resonator Registration | <1ms |
| Coupling Establishment | <5ms |
| Attention Allocation | <100μs |
| Invariant Check | <10μs |
| Presence Signal | <500μs |

**Per-node capacity**: 100,000+ concurrent Resonators

**Target scale**: 100M+ Resonators across distributed deployment

## Debugging Tips

### Enable Detailed Logging

```rust
use tracing_subscriber::EnvFilter;

tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env()
        .add_directive("maple_runtime=debug".parse().unwrap()))
    .init();
```

Run with:

```bash
RUST_LOG=maple_runtime=debug cargo run
```

### Check Telemetry

```rust
if let Some(telemetry) = runtime.telemetry().await {
    println!("Registered Resonators: {}", telemetry.resonator_count);
    println!("Active Couplings: {}", telemetry.coupling_count);
    println!("Attention Allocated: {}", telemetry.total_attention_allocated);
}
```

## What's Next?

Now that you understand the basics, explore:

1. **[Architecture Overview](architecture.md)** - Deep dive into Resonance Architecture
2. **[Core Concepts](concepts/)** - Detailed explanations of key concepts
3. **[Platform Guides](platforms/)** - Platform-specific features
4. **[Examples](../crates/maple-runtime/examples/)** - More comprehensive examples

## Need Help?

- **Documentation**: [docs.mapleai.org](https://docs.mapleai.org)
- **GitHub Issues**: [github.com/mapleaiorg/maple/issues](https://github.com/mapleaiorg/maple/issues)
- **Discord**: [discord.gg/maple-ai](https://discord.gg/maple-ai)
- **Email**: hello@mapleai.org

---

**Next**: [Architecture Overview →](architecture.md)
