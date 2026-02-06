# Resonator Conformance (`crates/resonator/conformance`)

Conformance test suite for validating MAPLE Runtime Invariants.

## Overview

This crate provides a comprehensive test framework for verifying that Resonator implementations correctly enforce the 8 architectural invariants that define the MAPLE Resonance Architecture.

## The 8 Runtime Invariants

| # | Invariant | Description |
|---|-----------|-------------|
| 1 | Presence precedes Coupling | A Resonator must establish presence before forming couplings |
| 2 | Coupling precedes Meaning | Meaning can only form within established coupling relationships |
| 3 | Meaning precedes Intent | Intent requires sufficient meaning convergence |
| 4 | Commitment precedes Consequence | No consequence without explicit, auditable commitment |
| 5 | Receipts are Immutable | Once created, commitment receipts cannot be modified |
| 6 | Audit trail is Append-Only | Audit entries can only be added, never removed |
| 7 | Capabilities gate Actions | Actions require explicit capability grants |
| 8 | Time anchors are Monotonic | Temporal anchors always increase within a timeline |

## Quick Start

```rust
use resonator_conformance::{ConformanceSuite, ConformanceReport};

// Create test suite
let suite = ConformanceSuite::new();

// Run all invariant tests
let report = suite.run_all().await?;

// Check results
if report.all_passed() {
    println!("All invariants verified!");
} else {
    for failure in report.failures() {
        println!("Failed: {:?} - {}", failure.invariant, failure.message);
    }
}
```

## Running Specific Invariant Tests

```rust
use resonator_conformance::{ConformanceSuite, Invariant};

let suite = ConformanceSuite::new();

// Test specific invariant
let result = suite.test_invariant(Invariant::CommitmentPrecedesConsequence).await?;

if result.passed {
    println!("Invariant 4 verified: {}", result.message);
}
```

## Test Categories

### Pipeline Order Tests (Invariants 1-4)

These tests verify the resonance pipeline ordering:

```rust
// Invariant 1: Presence → Coupling
suite.test_invariant(Invariant::PresencePrecedesCoupling).await?;

// Invariant 2: Coupling → Meaning
suite.test_invariant(Invariant::CouplingPrecedesMeaning).await?;

// Invariant 3: Meaning → Intent
suite.test_invariant(Invariant::MeaningPrecedesIntent).await?;

// Invariant 4: Commitment → Consequence
suite.test_invariant(Invariant::CommitmentPrecedesConsequence).await?;
```

### Immutability Tests (Invariants 5-6)

These tests verify data integrity guarantees:

```rust
// Invariant 5: Receipts cannot be modified
suite.test_invariant(Invariant::ReceiptsImmutable).await?;

// Invariant 6: Audit trail is append-only
suite.test_invariant(Invariant::AuditAppendOnly).await?;
```

### Access Control Tests (Invariant 7)

Tests that capabilities properly gate actions:

```rust
// Invariant 7: Actions require capabilities
suite.test_invariant(Invariant::CapabilitiesGateActions).await?;
```

### Temporal Tests (Invariant 8)

Tests for temporal ordering guarantees:

```rust
// Invariant 8: Time anchors are monotonic
suite.test_invariant(Invariant::TimeAnchorsMonotonic).await?;
```

## Conformance Report

The conformance report provides detailed results:

```rust
let report = suite.run_all().await?;

// Summary
println!("Tests run: {}", report.total());
println!("Passed: {}", report.passed_count());
println!("Failed: {}", report.failed_count());
println!("Duration: {:?}", report.duration());

// Detailed results
for result in report.results() {
    println!("{:?}: {} - {}",
        result.invariant,
        if result.passed { "PASS" } else { "FAIL" },
        result.message
    );
}

// Export as JSON
let json = report.to_json()?;
```

## Writing Custom Conformance Tests

### Test Harness

```rust
use resonator_conformance::{TestHarness, InvariantTest};

struct MyCustomTest;

impl InvariantTest for MyCustomTest {
    fn invariant(&self) -> Invariant {
        Invariant::CommitmentPrecedesConsequence
    }

    async fn run(&self, harness: &TestHarness) -> TestResult {
        // Set up test scenario
        let tracker = harness.create_consequence_tracker();

        // Attempt to record consequence without commitment
        let result = tracker.request_consequence(
            "no-commitment".to_string(),
            ConsequenceRequest::default(),
        ).await;

        // Verify invariant was enforced
        match result {
            Err(ConsequenceError::NoActiveCommitment(_)) => {
                TestResult::pass("Correctly rejected consequence without commitment")
            }
            _ => {
                TestResult::fail("Should have rejected consequence without commitment")
            }
        }
    }
}
```

### Adding Custom Tests to Suite

```rust
let mut suite = ConformanceSuite::new();
suite.add_test(Box::new(MyCustomTest));

let report = suite.run_all().await?;
```

## Integration with CI/CD

### Running in CI

```bash
# Run conformance tests
cargo test -p resonator-conformance

# Run with verbose output
cargo test -p resonator-conformance -- --nocapture

# Generate JUnit report
cargo test -p resonator-conformance -- --format junit > conformance-results.xml
```

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

echo "Running conformance tests..."
cargo test -p resonator-conformance --quiet

if [ $? -ne 0 ]; then
    echo "Conformance tests failed. Commit aborted."
    exit 1
fi
```

### GitHub Actions

```yaml
- name: Run Conformance Tests
  run: |
    cargo test -p resonator-conformance --no-fail-fast

- name: Upload Conformance Report
  uses: actions/upload-artifact@v3
  with:
    name: conformance-report
    path: target/conformance-report.json
```

## Invariant Violation Handling

When an invariant is violated at runtime:

```rust
use resonator_conformance::InvariantViolation;

fn handle_violation(violation: InvariantViolation) {
    // Log the violation
    tracing::error!(
        invariant = ?violation.invariant,
        context = ?violation.context,
        "Runtime invariant violated"
    );

    // Trigger alert
    alert_engine.trigger(Alert::critical(
        format!("Invariant {:?} violated", violation.invariant),
    ));

    // In production, you might want to:
    // 1. Reject the operation
    // 2. Roll back any partial changes
    // 3. Notify operators
    // 4. Collect diagnostic information
}
```

## Test Coverage Requirements

For MAPLE compliance, all 8 invariants must pass:

| Invariant | Required Tests | Status |
|-----------|---------------|--------|
| 1. Presence precedes Coupling | Positive + Negative | Required |
| 2. Coupling precedes Meaning | Positive + Negative | Required |
| 3. Meaning precedes Intent | Positive + Negative | Required |
| 4. Commitment precedes Consequence | Positive + Negative | Required |
| 5. Receipts Immutable | Modification attempt | Required |
| 6. Audit Append-Only | Deletion attempt | Required |
| 7. Capabilities gate Actions | Unauthorized attempt | Required |
| 8. Time anchors Monotonic | Ordering verification | Required |

## See Also

- [Runtime Invariants Documentation](../../maple-runtime/README.md#invariants)
- [Resonator Architecture](../README.md)
- [Observability](../observability/README.md)
