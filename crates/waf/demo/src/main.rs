#![deny(unsafe_code)]
//! WAF demo binary showcasing worldline framework capabilities.
//!
//! Runs a self-contained demonstration of:
//! 1. 4-phase genesis boot
//! 2. Autopoietic kernel evolution (healthy + stressed workloads)
//! 3. WLIR module creation and verification
//! 4. Evidence bundle construction
//!
//! No external services required -- all components use simulated backends.

mod workload;

use maple_waf_context_graph::{ContentHash, GovernanceTier};
use maple_waf_evidence::{
    EvidenceBuilder, EvidenceBundle, ReproBuildResult, SimulatedInvariantChecker,
    SimulatedTestRunner,
};
use maple_waf_genesis::{create_worldline, genesis_boot, GenesisResult, SeedConfig};
use maple_waf_kernel::{AutopoieticKernel, EvolutionStepResult};
use maple_waf_wlir::{
    AxiomaticConstraints, ModuleVerifier, OperatorBody, OperatorDefinition, ProvenanceHeader,
    WlirFactoryModule,
};

use workload::SimulatedWorkload;

// ── Formatting Helpers ──────────────────────────────────────────────────

const BANNER: &str = r#"
 ╔═══════════════════════════════════════════════════════════════╗
 ║          Worldline Autopoietic Factory  --  Demo             ║
 ║                                                              ║
 ║   Self-evolving runtime with 14 invariants,                  ║
 ║   dissonance detection, and evidence-gated swaps.            ║
 ╚═══════════════════════════════════════════════════════════════╝
"#;

fn section(title: &str) {
    let width: usize = 60;
    let pad = width.saturating_sub(title.len() + 4);
    let left = pad / 2;
    let right = pad - left;
    println!();
    println!(
        " ┌{}┐",
        "─".repeat(width)
    );
    println!(
        " │{}  {}  {}│",
        " ".repeat(left),
        title,
        " ".repeat(right)
    );
    println!(
        " └{}┘",
        "─".repeat(width)
    );
}

fn ok(msg: &str) {
    println!("   [OK]  {}", msg);
}

fn info(msg: &str) {
    println!("   [--]  {}", msg);
}

fn warn(msg: &str) {
    println!("   [!!]  {}", msg);
}

fn step_label(n: usize, total: usize) -> String {
    format!("Step {}/{}", n, total)
}

// ── Main ────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_max_level(tracing::Level::WARN)
        .init();

    println!("{}", BANNER);

    if let Err(e) = run_demo().await {
        eprintln!();
        eprintln!("   [FATAL]  Demo failed: {}", e);
        std::process::exit(1);
    }

    println!();
    println!(" ════════════════════════════════════════════════════════════════");
    println!("  Demo complete.  All phases succeeded.");
    println!(" ════════════════════════════════════════════════════════════════");
    println!();
}

async fn run_demo() -> Result<(), Box<dyn std::error::Error>> {
    // ── Phase A: Genesis Boot ───────────────────────────────────────
    section("Phase A: Genesis Boot");

    let config = SeedConfig::demo();
    info(&format!("SeedConfig::demo()  resonance_min={:.2}  max_steps={}",
        config.resonance_min, config.max_evolution_steps));

    let result: GenesisResult = genesis_boot(config.clone()).await?;
    print_genesis_result(&result);

    // ── Phase B: Kernel Creation ────────────────────────────────────
    section("Phase B: Kernel Initialisation");

    let worldline = create_worldline(config).await?;
    ok(&format!("Worldline created  id={}", worldline.id));

    let mut kernel = AutopoieticKernel::from_worldline(worldline)?;
    ok("AutopoieticKernel online");

    // ── Phase C: Evolution Loop ─────────────────────────────────────
    section("Phase C: Evolution Loop  (5 steps)");

    let total_steps = 5;

    for i in 0..total_steps {
        let metrics = match i {
            0 | 2 | 4 => SimulatedWorkload::healthy_metrics(),
            1         => SimulatedWorkload::stressed_metrics(),
            3         => SimulatedWorkload::degrading_metrics(6),
            _         => SimulatedWorkload::healthy_metrics(),
        };

        let label = step_label(i + 1, total_steps);

        match kernel.step_evolution(&metrics).await {
            Ok(step_result) => print_step_result(&label, &step_result),
            Err(e) => warn(&format!("{}  Error: {}", label, e)),
        }
    }

    // ── Phase D: Kernel Metrics Summary ─────────────────────────────
    section("Phase D: Kernel Metrics");

    let km = kernel.metrics();
    info(&format!("Steps attempted  : {}", km.steps_attempted));
    info(&format!("Evolutions OK    : {}", km.evolutions_succeeded));
    info(&format!("Evolutions FAIL  : {}", km.evolutions_failed));
    info(&format!("Rollbacks        : {}", km.rollbacks));
    info(&format!("Current resonance: {:.4}", km.current_resonance));
    info(&format!("Average resonance: {:.4}", km.avg_resonance()));
    info(&format!("Success rate     : {:.1}%", km.success_rate() * 100.0));

    // ── Phase E: WLIR Module ────────────────────────────────────────
    section("Phase E: WLIR Module Creation + Verification");

    demonstrate_wlir()?;

    // ── Phase F: Evidence Bundle ────────────────────────────────────
    section("Phase F: Evidence Bundle");

    demonstrate_evidence().await?;

    Ok(())
}

// ── Genesis helpers ─────────────────────────────────────────────────────

fn print_genesis_result(r: &GenesisResult) {
    ok(&format!("Phase reached      : {}", r.phase_reached));
    ok(&format!("WorldLine ID       : {}", r.worldline_id));
    ok(&format!("Invariants verified: {}", r.invariants_verified));
    ok(&format!("Initial resonance  : {:.4}", r.initial_resonance));
    ok(&format!("Duration           : {} ms", r.genesis_duration_ms));
}

// ── Evolution step helpers ──────────────────────────────────────────────

fn print_step_result(label: &str, result: &EvolutionStepResult) {
    match result {
        EvolutionStepResult::Healthy { resonance } => {
            ok(&format!("{}  Healthy        R={:.4}", label, resonance));
        }
        EvolutionStepResult::Evolved { resonance, description } => {
            info(&format!("{}  Evolved        R={:.4}  {}", label, resonance, description));
        }
        EvolutionStepResult::EvidenceFailed { reason } => {
            warn(&format!("{}  EvidenceFailed  {}", label, reason));
        }
        EvolutionStepResult::Denied { reason } => {
            warn(&format!("{}  Denied          {}", label, reason));
        }
        EvolutionStepResult::RolledBack { reason } => {
            warn(&format!("{}  RolledBack      {}", label, reason));
        }
    }
}

// ── WLIR demonstration ─────────────────────────────────────────────────

fn demonstrate_wlir() -> Result<(), Box<dyn std::error::Error>> {
    let provenance = ProvenanceHeader {
        worldline_id: "wl-demo-001".into(),
        content_hash: "demo-hash-placeholder".into(),
        governance_tier: GovernanceTier::Tier1,
        timestamp_ms: now_ms(),
    };

    let constraints = AxiomaticConstraints {
        forbidden_operations: vec!["eval".into(), "exec".into()],
        max_recursion_depth: 32,
        memory_limit_mb: 128,
        allow_network: false,
        allow_filesystem: false,
    };

    let module = WlirFactoryModule::new(
        "demo-transform",
        provenance,
        constraints,
        "0.1.0",
    )
    .with_operator(OperatorDefinition::new(
        "square",
        vec!["x".into()],
        "i64",
        OperatorBody::Expression("(* x x)".into()),
    ))
    .with_operator(OperatorDefinition::new(
        "hash_data",
        vec!["data".into()],
        "bytes",
        OperatorBody::Native("blake3".into()),
    ))
    .with_operator(OperatorDefinition::new(
        "pipeline",
        vec!["input".into()],
        "bytes",
        OperatorBody::Composite(vec!["square".into(), "hash_data".into()]),
    ));

    info(&format!("Module name    : {}", module.name));
    info(&format!("Version        : {}", module.version));
    info(&format!("Operators      : {}", module.operator_count()));

    // Verify the module.
    match ModuleVerifier::verify(&module) {
        Ok(warnings) => {
            ok("Module verification passed");
            for w in &warnings {
                warn(&format!("  warning: {}", w));
            }
        }
        Err(e) => {
            warn(&format!("Module verification FAILED: {}", e));
        }
    }

    // Demonstrate S-expression parsing.
    let sexpr_input = "(define (square x) (* x x))";
    match maple_waf_wlir::parse_sexpr(sexpr_input) {
        Ok(expr) => {
            ok(&format!("S-expr parsed    : {}", expr));
        }
        Err(e) => {
            warn(&format!("S-expr parse error: {}", e));
        }
    }

    // Serialise the module to JSON.
    let json = serde_json::to_string_pretty(&module)?;
    let json_size = json.len();
    ok(&format!("Module JSON size : {} bytes", json_size));

    Ok(())
}

// ── Evidence demonstration ──────────────────────────────────────────────

async fn demonstrate_evidence() -> Result<(), Box<dyn std::error::Error>> {
    // Build a passing evidence bundle using the builder pattern.
    let runner = SimulatedTestRunner::all_pass(12);
    let checker = SimulatedInvariantChecker::all_pass();
    let repro = ReproBuildResult::verified(ContentHash::hash(b"demo-build-artifact"));

    let bundle: EvidenceBundle = EvidenceBuilder::new(
        runner,
        checker,
        ContentHash::hash(b"demo-delta"),
        ContentHash::hash(b"demo-artifact"),
    )
    .with_repro_build(repro)
    .build()
    .await?;

    info(&format!("Tests            : {}/{}", bundle.tests_passed(), bundle.test_count()));
    info(&format!("Invariants       : {}/{}", bundle.invariants_holding(), bundle.invariant_count()));
    info(&format!("Repro build      : {}", if bundle.repro_build_verified() { "OK" } else { "FAIL" }));
    info(&format!("Equivalence tier : {}", bundle.equivalence_tier));
    info(&format!("Hash verified    : {}", bundle.verify_hash()));

    if bundle.is_sufficient() {
        ok("Evidence bundle is SUFFICIENT for swap");
    } else {
        warn("Evidence bundle is NOT sufficient");
    }

    ok(&format!("Summary: {}", bundle.summary()));

    Ok(())
}

// ── Utilities ───────────────────────────────────────────────────────────

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_millis() as u64
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn genesis_boot_succeeds_with_demo_config() {
        let config = SeedConfig::demo();
        let result = genesis_boot(config).await;
        assert!(result.is_ok(), "genesis_boot should succeed with demo config");

        let r = result.unwrap();
        assert_eq!(
            r.phase_reached,
            maple_waf_genesis::GenesisPhase::Complete
        );
        assert_eq!(r.invariants_verified, 14);
        assert!(r.initial_resonance > 0.0);
    }

    #[test]
    fn workload_healthy_metrics_are_valid() {
        let m = SimulatedWorkload::healthy_metrics();
        assert!(m.cpu_usage_pct >= 0.0 && m.cpu_usage_pct <= 100.0);
        assert!(m.memory_usage_mb > 0.0);
        assert!(m.latency_p50_ms > 0.0);
        assert!(m.latency_p99_ms >= m.latency_p50_ms);
        assert!(m.error_rate >= 0.0 && m.error_rate <= 1.0);
        assert!(m.throughput_rps > 0.0);
        assert!(m.resonance > 0.0 && m.resonance <= 1.0);
    }

    #[test]
    fn workload_stressed_metrics_show_elevated_load() {
        let m = SimulatedWorkload::stressed_metrics();
        assert!(m.cpu_usage_pct > 80.0);
        assert!(m.error_rate > 0.05);
        assert!(m.resonance < SimulatedWorkload::healthy_metrics().resonance);
    }

    #[test]
    fn workload_degrading_metrics_worsen() {
        let early = SimulatedWorkload::degrading_metrics(1);
        let late = SimulatedWorkload::degrading_metrics(8);
        assert!(late.cpu_usage_pct > early.cpu_usage_pct);
        assert!(late.resonance < early.resonance);
    }

    #[tokio::test]
    async fn kernel_creation_from_worldline() {
        let wl = create_worldline(SeedConfig::demo()).await.unwrap();
        let kernel = AutopoieticKernel::from_worldline(wl);
        assert!(kernel.is_ok());
        assert_eq!(kernel.unwrap().step_count(), 0);
    }

    #[tokio::test]
    async fn evidence_bundle_via_builder() {
        let runner = SimulatedTestRunner::all_pass(5);
        let checker = SimulatedInvariantChecker::all_pass();
        let repro = ReproBuildResult::verified(ContentHash::hash(b"build"));

        let bundle = EvidenceBuilder::new(
            runner,
            checker,
            ContentHash::hash(b"delta"),
            ContentHash::hash(b"artifact"),
        )
        .with_repro_build(repro)
        .build()
        .await
        .unwrap();

        assert!(bundle.is_sufficient());
        assert!(bundle.verify_hash());
    }

    #[test]
    fn wlir_module_verifies() {
        let module = WlirFactoryModule::new(
            "test-mod",
            ProvenanceHeader {
                worldline_id: "wl-test".into(),
                content_hash: "hash".into(),
                governance_tier: GovernanceTier::Tier0,
                timestamp_ms: 1_000,
            },
            AxiomaticConstraints::default(),
            "1.0.0",
        )
        .with_operator(OperatorDefinition::new(
            "id",
            vec!["x".into()],
            "any",
            OperatorBody::Expression("x".into()),
        ));

        let result = ModuleVerifier::verify(&module);
        assert!(result.is_ok());
    }
}
