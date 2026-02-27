# Platform Packs Tutorial

This tutorial walks through creating a custom PALM platform pack.

## Prerequisites

- Rust `1.80+`
- Async Rust basics
- Familiarity with MAPLE profiles and policy boundaries

## 1. Create a Crate

```bash
cargo new --lib my-platform-pack
cd my-platform-pack
```

## 2. Add Dependencies

```toml
[dependencies]
palm-platform-pack = { path = "../contracts/platform-pack" }
palm-types = { path = "../crates/palm/types" }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
tracing = "0.1"
```

## 3. Implement `PlatformPack`

```rust
use async_trait::async_trait;
use palm_platform_pack::*;
use palm_types::PlatformProfile;

pub struct MyPlatformPack {
    metadata: PlatformMetadata,
    policy: PlatformPolicyConfig,
    health: PlatformHealthConfig,
    state: PlatformStateConfig,
    resources: PlatformResourceConfig,
    recovery: PlatformRecoveryConfig,
    capabilities: PlatformCapabilities,
}

impl Default for MyPlatformPack {
    fn default() -> Self {
        Self {
            metadata: PlatformMetadata {
                name: "my-platform".into(),
                version: "0.1.0".into(),
                description: "Custom platform profile".into(),
                ..Default::default()
            },
            policy: PlatformPolicyConfig::default(),
            health: PlatformHealthConfig::default(),
            state: PlatformStateConfig::default(),
            resources: PlatformResourceConfig::default(),
            recovery: PlatformRecoveryConfig::default(),
            capabilities: PlatformCapabilities::default(),
        }
    }
}

#[async_trait]
impl PlatformPack for MyPlatformPack {
    fn profile(&self) -> PlatformProfile {
        PlatformProfile::Custom("my-platform".into())
    }

    fn metadata(&self) -> &PlatformMetadata { &self.metadata }
    fn policy_config(&self) -> &PlatformPolicyConfig { &self.policy }
    fn health_config(&self) -> &PlatformHealthConfig { &self.health }
    fn state_config(&self) -> &PlatformStateConfig { &self.state }
    fn resource_config(&self) -> &PlatformResourceConfig { &self.resources }
    fn recovery_config(&self) -> &PlatformRecoveryConfig { &self.recovery }
    fn capabilities(&self) -> &PlatformCapabilities { &self.capabilities }

    async fn on_load(&self) -> Result<(), PackError> {
        tracing::info!("my-platform loaded");
        Ok(())
    }
}
```

## 4. Validate with Conformance

```bash
cargo test -p palm-conformance
```

## 5. Integrate with Daemon/CLI

- Register the pack in your PALM profile registry path.
- Start daemon and select the profile.
- Verify policy, health, and recovery behavior through CLI.

## Canonical Built-In Packs

- `mapleverse-pack`: throughput-first AI coordination
- `finalverse-pack`: human agency and safety-first
- `ibank-pack`: accountability-first financial operations

## Next

- [Operations Tutorial](operations.md)
- [Conformance Guide](../conformance.md)
