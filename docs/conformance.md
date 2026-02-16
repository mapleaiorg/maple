# Platform Pack Conformance Guide

This guide explains how to validate that your platform pack correctly implements the Platform Contract.

## What is Conformance?

Conformance testing verifies that a platform pack:
1. Implements all required trait methods
2. Provides valid configuration
3. Exhibits correct runtime behavior
4. Meets platform-specific requirements

## Running Conformance Tests

### Basic Usage

```rust
use palm_conformance::{ConformanceRunner, ConformanceConfig};
use palm_platform_pack::PlatformPack;
use std::sync::Arc;

#[tokio::test]
async fn test_my_pack() {
    let pack: Arc<dyn PlatformPack> = Arc::new(MyPlatformPack::new());

    let config = ConformanceConfig::default();
    let runner = ConformanceRunner::new(config);

    let report = runner.run(pack).await;

    println!("{}", report.to_text());
    assert!(report.is_conformant());
}
```

### Custom Configuration

```rust
let config = ConformanceConfig {
    run_core: true,
    run_behavioral: true,
    run_platform_specific: true,
    test_timeout: Duration::from_secs(60),
    continue_on_failure: true,
    verbose: true,
};

let runner = ConformanceRunner::new(config);
```

## Test Categories

### Core Tests

| Test | Description |
|------|-------------|
| `metadata_completeness` | Verifies all required metadata fields |
| `config_validity` | Validates configuration against schema |
| `capability_consistency` | Checks capabilities match config |
| `lifecycle_callbacks` | Tests on_load/on_unload |
| `profile_matches` | Verifies profile consistency |

### Behavioral Tests

| Test | Description |
|------|-------------|
| `agent_spec_validation` | Spec validation works correctly |
| `resource_limits` | Default values don't exceed limits |

### Platform-Specific Tests

#### Mapleverse Tests
- `high_throughput`: Supports >= 100k instances
- `no_human_approval_required`: Human approval disabled
- `fast_recovery`: >= 5 recovery attempts allowed

#### Finalverse Tests
- `human_approval_required`: Human approval enabled
- `safety_holds_enabled`: Safety holds active
- `conservative_limits`: Recovery attempts <= 5

#### iBank Tests
- `accountability_required`: Proof and pre-audit required
- `no_force_operations`: Force operations blocked
- `long_retention`: >= 180 days audit retention

## Understanding Reports

### Text Report

```
+============================================================+
|  PALM Platform Pack Conformance Report                     |
+============================================================+
|  Platform: my-platform                                     |
|  Timestamp: 2026-02-01 10:00:00 UTC                        |
|  Duration: 1.234s                                          |
+============================================================+
|  Core Tests:
+------------------------------------------------------------+
|  [PASS] metadata_completeness                         10ms
|  [PASS] config_validity                               15ms
|  [PASS] capability_consistency                         5ms
|  [PASS] lifecycle_callbacks                           50ms
|  [PASS] profile_matches                                1ms
+============================================================+
|  Summary:                                                  |
|    Total: 5    Passed: 5    Failed: 0    Skipped: 0        |
|                                                            |
|  Result: CONFORMANT                                        |
+============================================================+
```

### JSON Report

```json
{
  "platform_name": "my-platform",
  "timestamp": "2026-02-01T10:00:00Z",
  "duration_ms": 1234,
  "results": {
    "core": [
      {
        "name": "metadata_completeness",
        "status": "passed",
        "duration_ms": 10
      }
    ]
  },
  "summary": {
    "total": 5,
    "passed": 5,
    "failed": 0,
    "skipped": 0,
    "conformant": true
  }
}
```

## Common Issues and Solutions

### 1. Metadata Incomplete

```
[FAIL] metadata_completeness
  Error: name is required; version is required
```

**Solution**: Ensure all required metadata fields are populated:
```rust
PlatformMetadata {
    name: "my-platform".to_string(),
    version: "0.1.0".to_string(),
    ..Default::default()
}
```

### 2. Configuration Invalid

```
[FAIL] config_validity
  Error: default CPU exceeds limit
```

**Solution**: Check that default values don't exceed limits:
```rust
// Ensure defaults.cpu_millicores <= limits.max_cpu_millicores
```

### 3. Capabilities Inconsistent

```
[FAIL] capability_consistency
  Error: Migration enabled in config but not in capabilities
```

**Solution**: Align capabilities with configuration:
```rust
// If state_config.migration.enable_live_migration is true,
// then capabilities.supports_migration must also be true
```

## CI/CD Integration

### GitHub Actions

```yaml
name: Conformance Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - name: Run conformance tests
        run: cargo test -p my-platform-pack --test conformance
```

### Pre-commit Hook

```bash
#!/bin/bash
cargo test -p my-platform-pack --test conformance
```

## Best Practices

1. **Run Early, Run Often**: Include conformance tests in CI
2. **Fix Warnings**: Warnings often become errors in future versions
3. **Document Deviations**: If you intentionally deviate, document why
4. **Version Pin**: Pin to specific MAPLE versions for stability
5. **Test All Profiles**: If supporting multiple profiles, test each

## WorldLine Conformance Suites

In addition to PALM platform-pack conformance, run WorldLine suites:

```bash
# Constitutional + cross-profile + lifecycle integration
cargo test -p maple-mwl-conformance -p maple-mwl-integration

# Prompt 17-28 self-producing substrate invariants
cargo test -p maple-worldline-conformance
```

For deeper subsystem validation, run:

```bash
cargo test -p maple-worldline-observation -p maple-worldline-meaning -p maple-worldline-intent -p maple-worldline-commitment -p maple-worldline-consequence -p maple-worldline-self-mod-gate -p maple-worldline-codegen -p maple-worldline-deployment -p maple-worldline-langgen -p maple-worldline-ir -p maple-worldline-compiler -p maple-worldline-sal -p maple-worldline-hardware -p maple-worldline-bootstrap -p maple-worldline-evos
```

## Next Steps

- [WorldLine Framework Guide](worldline-framework.md)
- [WorldLine Quickstart](tutorials/worldline-quickstart.md)
- [Platform Packs Tutorial](tutorials/platform-packs.md)
- [Architecture Guide](architecture.md)
- [API Reference](api/README.md)
