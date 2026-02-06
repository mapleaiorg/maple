//! # Conformance Testing Example
//!
//! This example demonstrates:
//! - Running the conformance test suite
//! - Testing individual invariants
//! - Generating conformance reports
//!
//! Run with: `cargo run --example 10_conformance_testing`

use resonator_conformance::{
    ConformanceSuite, ConformanceReport, Invariant, TestResult,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🍁 MAPLE - Conformance Testing Example\n");

    println!("═══════════════════════════════════════════════════════");
    println!("📋 The 8 MAPLE Runtime Invariants");
    println!("═══════════════════════════════════════════════════════\n");

    let invariants = [
        (Invariant::PresencePrecedesCoupling, "Presence precedes Coupling"),
        (Invariant::CouplingPrecedesMeaning, "Coupling precedes Meaning"),
        (Invariant::MeaningPrecedesIntent, "Meaning precedes Intent"),
        (Invariant::CommitmentPrecedesConsequence, "Commitment precedes Consequence"),
        (Invariant::ReceiptsImmutable, "Receipts are Immutable"),
        (Invariant::AuditAppendOnly, "Audit trail is Append-Only"),
        (Invariant::CapabilitiesGateActions, "Capabilities gate Actions"),
        (Invariant::TimeAnchorsMonotonic, "Time anchors are Monotonic"),
    ];

    for (i, (_, desc)) in invariants.iter().enumerate() {
        println!("   {}. {}", i + 1, desc);
    }
    println!();

    // Create conformance suite
    println!("═══════════════════════════════════════════════════════");
    println!("🧪 Running Conformance Tests");
    println!("═══════════════════════════════════════════════════════\n");

    let suite = ConformanceSuite::new();

    // Test each invariant individually
    for (invariant, desc) in &invariants {
        print!("   Testing: {} ... ", desc);

        let result = suite.test_invariant(*invariant).await?;

        if result.passed {
            println!("✅ PASS");
        } else {
            println!("❌ FAIL - {}", result.message);
        }
    }

    println!();

    // Run full test suite
    println!("═══════════════════════════════════════════════════════");
    println!("📊 Full Test Suite Results");
    println!("═══════════════════════════════════════════════════════\n");

    let report = suite.run_all().await?;

    println!("   Total tests: {}", report.total());
    println!("   Passed: {} ✅", report.passed_count());
    println!("   Failed: {} ❌", report.failed_count());
    println!("   Duration: {:?}", report.duration());
    println!();

    // Detailed results
    println!("   Detailed Results:");
    println!("   ─────────────────────────────────────────────────────");

    for result in report.results() {
        let status = if result.passed { "✅" } else { "❌" };
        println!("   {} {:?}", status, result.invariant);
        if !result.passed {
            println!("      └─ {}", result.message);
        }
    }
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
        for failure in report.failures() {
            println!("   • {:?}: {}", failure.invariant, failure.message);
        }
    }
    println!("═══════════════════════════════════════════════════════\n");

    // Export report
    println!("📄 Exporting Report");
    println!("─────────────────────────────────────────────────────────\n");

    let json = report.to_json()?;
    println!("   JSON report size: {} bytes", json.len());

    // Show a snippet
    println!("   Report preview:");
    let preview: serde_json::Value = serde_json::from_str(&json)?;
    println!("   {}", serde_json::to_string_pretty(&serde_json::json!({
        "total": preview["total"],
        "passed": preview["passed"],
        "failed": preview["failed"],
        "all_passed": preview["all_passed"],
    }))?);
    println!();

    // Demonstrate violation detection
    println!("═══════════════════════════════════════════════════════");
    println!("🔍 Invariant Violation Detection Demo");
    println!("═══════════════════════════════════════════════════════\n");

    println!("   Attempting to create consequence without commitment...");

    // This would demonstrate the invariant enforcement
    use resonator_consequence::{ConsequenceTracker, InMemoryConsequenceTracker, ConsequenceRequest};

    let tracker = InMemoryConsequenceTracker::new();

    let result = tracker.request_consequence(
        "non_existent_commitment".to_string(),
        ConsequenceRequest {
            action: "test_action".to_string(),
            parameters: serde_json::json!({}),
        },
    );

    match result {
        Err(e) => {
            println!("   ✅ Correctly rejected: {}", e);
            println!("   Invariant 4 (Commitment precedes Consequence) enforced!");
        }
        Ok(_) => {
            println!("   ❌ ERROR: Should have rejected consequence without commitment");
        }
    }
    println!();

    println!("🎉 Conformance testing example completed!");

    Ok(())
}
