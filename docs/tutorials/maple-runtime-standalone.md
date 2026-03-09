# maple-runtime Standalone

This tutorial shows how to use `maple-runtime` as a smaller runtime dependency when you do not need the full daemon and worldline operations surface.

## 1. Core-only dependency

```toml
[dependencies]
maple-runtime = { version = "0.1.3", default-features = false }
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

## 2. Validate the minimal build matrix

```bash
cargo check -p maple-runtime --no-default-features
cargo test -p maple-runtime --no-default-features --lib
```

## 3. Opt into richer features when needed

```toml
[dependencies]
maple-runtime = { version = "0.1.3", default-features = false, features = ["cognition", "agent-kernel", "profile-validation"] }
tokio = { version = "1", features = ["full"] }
```

Useful example programs:

```bash
cargo run -p maple-runtime --example 06_agent_kernel_boundary --features agent-kernel
cargo run -p maple-runtime --example 08_memory_and_conversation --features memory-conversation
cargo run -p maple-runtime --example 09_observability_demo --features observability-examples
```

## 4. When to choose this path

Use the standalone runtime path when you want:

- embedded runtime experiments
- smaller dependency surfaces
- direct control over cognition and lifecycle integration

Use the PALM and worldline path when you need daemon operations, REST APIs, provenance queries, and richer control-plane workflows.

## Next

- [WorldLine Quickstart](worldline-quickstart.md)
- [Architecture Overview](../architecture/overview.md)
