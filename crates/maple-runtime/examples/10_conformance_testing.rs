//! # Conformance Testing Example
//!
//! This example demonstrates:
//! - Running the conformance test suite
//! - Testing individual invariants
//! - Generating conformance reports
//!
//! Run with: `cargo run --example 10_conformance_testing`

use resonator_conformance::{ConformanceConfig, ConformanceSuite, Invariant, TestStatus};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🍁 MAPLE - Conformance Testing Example\n");

    println!("═══════════════════════════════════════════════════════");
    println!("📋 Resonator Conformance Invariants (8 checks)");
    println!("═══════════════════════════════════════════════════════\n");

    let invariants = [
        (
            Invariant::PresencePrecedesCoupling,
            "Presence precedes Coupling",
        ),
        (
            Invariant::CouplingPrecedesMeaning,
            "Coupling precedes Meaning",
        ),
        (Invariant::MeaningPrecedesIntent, "Meaning precedes Intent"),
        (
            Invariant::CommitmentPrecedesConsequence,
            "Commitment precedes Consequence",
        ),
        (Invariant::ReceiptsImmutable, "Receipts are Immutable"),
        (Invariant::AuditAppendOnly, "Audit trail is Append-Only"),
        (
            Invariant::CapabilitiesGateActions,
            "Capabilities gate Actions",
        ),
        (
            Invariant::TimeAnchorsMonotonic,
            "Time anchors are Monotonic",
        ),
    ];

    for (i, (_, desc)) in invariants.iter().enumerate() {
        println!("   {}. {}", i + 1, desc);
    }
    println!();

    // Create conformance suite
    println!("═══════════════════════════════════════════════════════");
    println!("🧪 Running Conformance Tests");
    println!("═══════════════════════════════════════════════════════\n");

    let config = ConformanceConfig::default();
    let suite = ConformanceSuite::new(config);

    // Run full test suite
    println!("   Running all invariant tests...\n");
    let report = suite.run_all();

    // Show per-invariant status
    for (invariant, desc) in &invariants {
        let tests = report.by_invariant.get(invariant);
        let status = match tests {
            Some(tests) if tests.iter().all(|t| t.status == TestStatus::Passed) => "✅ PASS",
            Some(_) => "❌ FAIL",
            None => "⚠️  SKIP",
        };
        println!("   {} {}", status, desc);
    }

    println!();

    // Full test suite results
    println!("═══════════════════════════════════════════════════════");
    println!("📊 Full Test Suite Results");
    println!("═══════════════════════════════════════════════════════\n");

    println!("   Total tests: {}", report.summary.total);
    println!("   Passed: {} ✅", report.summary.passed);
    println!("   Failed: {} ❌", report.summary.failed);
    println!("   Skipped: {}", report.summary.skipped);
    println!("   Duration: {}ms", report.duration_ms);
    println!();

    // Overall status
    println!("═══════════════════════════════════════════════════════");
    if report.all_passed() {
        println!("🎉 CONFORMANCE: ALL INVARIANTS VERIFIED");
        println!("   This implementation is MAPLE-compliant.");
    } else {
        println!("⚠️  CONFORMANCE: SOME INVARIANTS FAILED");
        println!("   This implementation requires fixes.");

        println!("\n   Failures:");
        for test in &report.tests {
            if test.status == TestStatus::Failed {
                println!("   • {:?}: {}", test.invariant, test.name);
                if let Some(err) = &test.error {
                    println!("     Error: {}", err);
                }
            }
        }
    }
    println!("═══════════════════════════════════════════════════════\n");

    // Print the report using Display
    println!("📄 Detailed Report");
    println!("─────────────────────────────────────────────────────────\n");
    println!("{}", report);

    // Demonstrate invariant concept
    println!("═══════════════════════════════════════════════════════");
    println!("🔍 Invariant Demonstration");
    println!("═══════════════════════════════════════════════════════\n");

    println!("   The conformance suite verifies that:");
    println!();
    println!("   ✓ Presence is required before Coupling");
    println!("     - ResonatorId type enforces presence at compile time");
    println!();
    println!("   ✓ Coupling is required before Meaning");
    println!("     - MeaningFormationEngine requires CouplingContext");
    println!();
    println!("   ✓ Meaning is required before Intent");
    println!("     - IntentStabilizationEngine requires MeaningContext");
    println!();
    println!("   ✓ Commitment is required before Consequence");
    println!("     - ConsequenceTracker validates active commitment exists");
    println!();
    println!("   ✓ Receipts are immutable");
    println!("     - ConsequenceReceipt has no mutation methods");
    println!();
    println!("   ✓ Audit trails are append-only");
    println!("     - AuditTrail only exposes append() method");
    println!();
    println!("   ✓ Capabilities gate actions");
    println!("     - CapabilityChecker validates before execution");
    println!();
    println!("   ✓ Time anchors are monotonic");
    println!("     - TemporalAnchor::new() ensures ordering");
    println!();

    println!("🎉 Conformance testing example completed!");

    Ok(())
}
