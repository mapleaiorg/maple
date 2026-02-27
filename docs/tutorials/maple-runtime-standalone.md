# maple-runtime Standalone Tutorial

This tutorial shows how to run `maple-runtime` as an independent crate with a minimal dependency footprint, then opt into full boundary features only when required.

## 1. Prerequisites

- Rust `1.80+`
- Cargo

## 2. Create a Core-Only App

```toml
[dependencies]
maple-runtime = { version = "0.1.2", default-features = false }
tokio = { version = "1", features = ["full"] }
```

```rust
use maple_runtime::{config::RuntimeConfig, MapleRuntime, ResonatorSpec};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MapleRuntime::bootstrap(RuntimeConfig::default()).await?;
    let resonator = runtime.register_resonator(ResonatorSpec::default()).await?;
    resonator.signal_presence(maple_runtime::PresenceState::new()).await?;
    runtime.shutdown().await?;
    Ok(())
}
```

## 3. Validate Core-Only Build in This Repo

```bash
cargo check -p maple-runtime --no-default-features
cargo test -p maple-runtime --no-default-features --lib
```

## 4. Add Cognition and Boundary Features (Optional)

When you need structured cognition drafts and commitment-gated tool execution:

```toml
[dependencies]
maple-runtime = { version = "0.1.2", default-features = false, features = ["cognition", "agent-kernel", "profile-validation"] }
tokio = { version = "1", features = ["full"] }
```

This enables:

- `maple_runtime::cognition::*`
- `maple_runtime::agent_kernel::*`
- canonical profile validation through `resonator-profiles`

## 5. Run Feature-Gated Examples

```bash
cargo run -p maple-runtime --example 06_agent_kernel_boundary --features agent-kernel
cargo run -p maple-runtime --example 08_memory_and_conversation --features memory-conversation
cargo run -p maple-runtime --example 09_observability_demo --features observability-examples
cargo run -p maple-runtime --example 10_conformance_testing --features conformance-examples
```

## 6. Build Matrix Recommended for CI

```bash
# Full compatibility matrix
cargo check -p maple-runtime
cargo test -p maple-runtime

# Independent/minimal matrix
cargo check -p maple-runtime --no-default-features
cargo test -p maple-runtime --no-default-features --lib
```

## 7. Next

- Boundary flow: [iBank Commitment Boundary](ibank-commitment-boundary.md)
- Full stack ops: [Operations Tutorial](operations.md)
- WorldLine path: [WorldLine Quickstart](worldline-quickstart.md)
