# Platform Packs Tutorial

This tutorial walks you through creating a custom platform pack for MAPLE.

## Prerequisites

- Rust 1.75+
- Understanding of the Resonance Architecture
- Familiarity with async Rust

## What is a Platform Pack?

A Platform Pack is a configuration bundle that customizes MAPLE's behavior for a specific operational domain. It defines:

- **Policy rules**: What operations are allowed
- **Health checks**: How agent health is monitored
- **State management**: How checkpoints and recovery work
- **Resource limits**: What resources agents can use
- **Recovery behavior**: How failures are handled

## Understanding the Three Canonical Packs

MAPLE ships with three canonical platform packs:

### Mapleverse Pack
- **Priority**: Throughput > Everything
- **Use case**: High-velocity swarm orchestration
- **Characteristics**:
  - No human approval required
  - 10,000+ max instances
  - Fast recovery with many retry attempts
  - Hot reload and live migration supported

### Finalverse Pack
- **Priority**: Safety > Throughput
- **Use case**: Human-centric world simulation
- **Characteristics**:
  - Human approval required for destructive operations
  - Safety holds enabled
  - Conservative recovery limits
  - Checkpoint-first recovery

### iBank Pack
- **Priority**: Accountability > All
- **Use case**: Autonomous financial operations
- **Characteristics**:
  - Full accountability proof required
  - Force operations blocked
  - Long retention periods (7+ years for audits)
  - No live migration

## Step 1: Create Your Crate

```bash
cargo new --lib my-platform-pack
cd my-platform-pack
```

Add dependencies to `Cargo.toml`:

```toml
[package]
name = "my-platform-pack"
version = "0.1.0"
edition = "2021"

[dependencies]
palm-platform-pack = { path = "../contracts/platform-pack" }
palm-types = { path = "../crates/palm-types" }
palm-policy = { path = "../crates/palm-policy" }

async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
tracing = "0.1"
```

## Step 2: Implement the Platform Pack Trait

Create `src/lib.rs`:

```rust
use palm_platform_pack::*;
use palm_types::PlatformProfile;
use async_trait::async_trait;

pub struct MyPlatformPack {
    metadata: PlatformMetadata,
    policy: PlatformPolicyConfig,
    health: PlatformHealthConfig,
    state: PlatformStateConfig,
    resources: PlatformResourceConfig,
    recovery: PlatformRecoveryConfig,
    capabilities: PlatformCapabilities,
}

impl MyPlatformPack {
    pub fn new() -> Self {
        Self {
            metadata: Self::build_metadata(),
            policy: PlatformPolicyConfig::default(),
            health: PlatformHealthConfig::default(),
            state: PlatformStateConfig::default(),
            resources: PlatformResourceConfig::default(),
            recovery: PlatformRecoveryConfig::default(),
            capabilities: PlatformCapabilities::default(),
        }
    }

    fn build_metadata() -> PlatformMetadata {
        PlatformMetadata {
            name: "my-platform".to_string(),
            version: "0.1.0".to_string(),
            description: "My custom platform pack".to_string(),
            ..Default::default()
        }
    }
}

impl Default for MyPlatformPack {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PlatformPack for MyPlatformPack {
    fn profile(&self) -> PlatformProfile {
        PlatformProfile::Custom("my-platform".to_string())
    }

    fn metadata(&self) -> &PlatformMetadata {
        &self.metadata
    }

    fn policy_config(&self) -> &PlatformPolicyConfig {
        &self.policy
    }

    fn health_config(&self) -> &PlatformHealthConfig {
        &self.health
    }

    fn state_config(&self) -> &PlatformStateConfig {
        &self.state
    }

    fn resource_config(&self) -> &PlatformResourceConfig {
        &self.resources
    }

    fn recovery_config(&self) -> &PlatformRecoveryConfig {
        &self.recovery
    }

    fn capabilities(&self) -> &PlatformCapabilities {
        &self.capabilities
    }

    async fn validate_agent_spec(
        &self,
        spec: &palm_types::AgentSpec,
    ) -> Result<(), PackError> {
        // Add your custom validation logic here
        if spec.name.is_empty() {
            return Err(PackError::IncompatibleSpec(
                "Agent name cannot be empty".to_string()
            ));
        }
        Ok(())
    }

    async fn on_load(&self) -> Result<(), PackError> {
        tracing::info!("My platform pack loaded");
        Ok(())
    }

    async fn on_unload(&self) -> Result<(), PackError> {
        tracing::info!("My platform pack unloaded");
        Ok(())
    }
}
```

## Step 3: Customize Policy Configuration

Adjust the policy configuration based on your requirements:

```rust
fn build_policy() -> PlatformPolicyConfig {
    PlatformPolicyConfig {
        human_approval: HumanApprovalConfig {
            // Operations that always need human approval
            always_required: vec![
                "delete_deployment".to_string(),
            ],
            ..Default::default()
        },
        limits: OperationLimits {
            max_concurrent_deployments: 100,
            max_scale_up: 50,
            max_scale_down: 20,
            rate_limit_per_minute: 60,
        },
        safety_holds: SafetyHoldsConfig {
            enabled: true,
            blocked_operations: vec![
                "force_recovery".to_string(),
            ],
            auto_release_after_secs: Some(3600),
        },
        accountability: AccountabilityConfig {
            require_proof: true,
            require_pre_audit: true,
            reconciliation_required: vec![
                "update_deployment".to_string(),
            ],
        },
    }
}
```

## Step 4: Configure Health Monitoring

```rust
fn build_health() -> PlatformHealthConfig {
    PlatformHealthConfig {
        check_interval_secs: 30,
        failure_threshold: 3,
        success_threshold: 2,
        probes: ProbeConfigs {
            liveness: ProbeConfig {
                enabled: true,
                timeout_secs: 10,
                period_secs: 15,
                ..Default::default()
            },
            readiness: ProbeConfig {
                enabled: true,
                timeout_secs: 10,
                period_secs: 15,
                ..Default::default()
            },
            startup: None,
            custom: vec![],
        },
        ..Default::default()
    }
}
```

## Step 5: Set Capabilities

```rust
fn build_capabilities() -> PlatformCapabilities {
    PlatformCapabilities {
        max_deployments: Some(1000),
        max_instances_per_deployment: Some(100),
        max_total_instances: Some(10000),
        supports_migration: true,
        supports_hot_reload: false,
        supports_canary: true,
        supports_blue_green: true,
        supports_human_approval: true,
        supports_checkpoints: true,
        supports_cross_node_migration: false,
        ..Default::default()
    }
}
```

## Step 6: Run Conformance Tests

Test your pack against the conformance suite:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use palm_conformance::{ConformanceRunner, ConformanceConfig};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_conformance() {
        let pack: Arc<dyn PlatformPack> = Arc::new(MyPlatformPack::new());

        let config = ConformanceConfig::default();
        let runner = ConformanceRunner::new(config);

        let report = runner.run(pack).await;
        println!("{}", report.to_text());

        assert!(report.is_conformant(), "Pack should be conformant");
    }
}
```

## Best Practices

### 1. Configuration First
Design your configuration before implementation. Think about:
- What operations need approval?
- What are the resource limits?
- How should recovery work?

### 2. Test Early and Often
Use conformance tests throughout development to catch issues early.

### 3. Document Decisions
Add comments explaining why specific policy choices were made.

### 4. Version Carefully
Use semantic versioning for your pack. Breaking changes should increment the major version.

### 5. Monitor Health
Implement meaningful health probes that reflect your application's actual health.

## Common Patterns

### High-Throughput Pattern (like Mapleverse)
```rust
PlatformCapabilities {
    max_total_instances: Some(100000),
    supports_migration: true,
    supports_hot_reload: true,
    supports_human_approval: false,  // No human bottleneck
    ..Default::default()
}
```

### Safety-First Pattern (like Finalverse)
```rust
PlatformPolicyConfig {
    human_approval: HumanApprovalConfig {
        always_required: vec![
            "delete_deployment".to_string(),
            "force_recovery".to_string(),
        ],
        ..Default::default()
    },
    safety_holds: SafetyHoldsConfig {
        enabled: true,
        auto_release_after_secs: None,  // Never auto-release
        ..Default::default()
    },
    ..Default::default()
}
```

### Accountability Pattern (like iBank)
```rust
PlatformPolicyConfig {
    accountability: AccountabilityConfig {
        require_proof: true,
        require_pre_audit: true,
        reconciliation_required: vec![
            "create_deployment".to_string(),
            "update_deployment".to_string(),
            "delete_deployment".to_string(),
        ],
    },
    safety_holds: SafetyHoldsConfig {
        enabled: true,
        blocked_operations: vec![
            "force_recovery".to_string(),
            "force_restart".to_string(),
        ],
        auto_release_after_secs: None,
        ..Default::default()
    },
    ..Default::default()
}
```

## Next Steps

- Read the [Architecture Guide](../architecture.md)
- Review the [Conformance Guide](../conformance.md)
- Study the canonical packs in `crates/mapleverse-pack`, `crates/finalverse-pack`, and `crates/ibank-pack`
