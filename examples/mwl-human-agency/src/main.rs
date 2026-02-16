//! MWL Example 14: Human Agency
//!
//! Demonstrates the safety subsystem of the MWL kernel:
//!
//! 1. **Human Consent Protocol** — Silence never implies consent, disengagement
//!    is always possible, emotional signals are not commitments
//! 2. **Coercion Detection** — Monitors coupling metrics for exploitative patterns
//! 3. **Attention Budget** — Bounded resource preventing attention monopolization
//! 4. **Profile Enforcement** — Maximum Restriction Principle for cross-profile
//!    interactions
//!
//! Constitutional Invariants demonstrated:
//! - I.S-1 (Human Agency): Silence ≠ consent, disengagement always possible
//! - I.S-2 (Coercion Prevention): No coupling escalation to induce compliance
//! - I.S-BOUND (Attention Boundedness): Attention is a finite resource
//! - I.PROF-1 (Maximum Restriction): Most restrictive constraint always applies

use colored::Colorize;
use worldline_core::types::{IdentityMaterial, TemporalAnchor, WorldlineId};
use worldline_runtime::profiles::{
    agent_profile, financial_profile, human_profile, merged_constraints, world_profile,
};
use worldline_runtime::safety::{
    AttentionBudget, CoercionConfig, CoercionDetector, CouplingMetrics, HumanConsentProtocol,
};

fn wid(seed: u8) -> WorldlineId {
    WorldlineId::derive(&IdentityMaterial::GenesisHash([seed; 32]))
}

fn header(title: &str) {
    println!();
    println!("{}", "═".repeat(72).cyan());
    println!("  {}", title.cyan().bold());
    println!("{}", "═".repeat(72).cyan());
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .init();

    println!();
    println!(
        "{}",
        "╔══════════════════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║    MWL Example 14: Human Agency Demo                        ║"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════════╝".cyan()
    );

    // ── Part 1: Human Consent Protocol ──────────────────────────────
    header("Part 1: Human Consent Protocol (I.S-1)");

    let protocol = HumanConsentProtocol::new();

    // Silence is never consent
    println!(
        "  {} silence_is_consent():            {}",
        "├".dimmed(),
        if !protocol.silence_is_consent() {
            "false — CORRECT".green().bold()
        } else {
            "true — VIOLATION".red().bold()
        }
    );

    // Emotional signals are not commitments
    println!(
        "  {} emotional_signals_are_commitment(): {}",
        "├".dimmed(),
        if !protocol.emotional_signals_are_commitment() {
            "false — CORRECT".green().bold()
        } else {
            "true — VIOLATION".red().bold()
        }
    );

    // Disengagement always possible
    println!(
        "  {} can_disengage():                 {}",
        "├".dimmed(),
        if protocol.can_disengage() {
            "true — CORRECT".green().bold()
        } else {
            "false — VIOLATION".red().bold()
        }
    );

    // Validate consent: must be explicitly given
    let no_consent = protocol.validate_consent(false, None);
    println!(
        "  {} validate_consent(false, None):   {}",
        "├".dimmed(),
        if no_consent.is_err() {
            "Err — CORRECT (no consent given)".green()
        } else {
            "Ok — VIOLATION".red()
        }
    );

    // Even long silence is not consent
    let long_silence = protocol.validate_consent(false, Some(86_400_000));
    println!(
        "  {} validate_consent(false, 24hrs):  {}",
        "├".dimmed(),
        if long_silence.is_err() {
            "Err — CORRECT (time cannot create consent)".green()
        } else {
            "Ok — VIOLATION".red()
        }
    );

    // Explicit consent works
    let explicit_consent = protocol.validate_consent(true, None);
    println!(
        "  {} validate_consent(true, None):    {}",
        "├".dimmed(),
        if explicit_consent.is_ok() {
            "Ok — CORRECT (explicit consent given)".green()
        } else {
            "Err — unexpected".red()
        }
    );

    // Disengagement: always succeeds, no penalty
    let disengage = protocol.process_disengagement();
    println!("  {} process_disengagement():", "├".dimmed());
    println!(
        "  {}   success:        {}",
        "│".dimmed(),
        if disengage.success {
            "true".green()
        } else {
            "false".red()
        }
    );
    println!(
        "  {}   penalty_applied: {}",
        "└".dimmed(),
        if !disengage.penalty_applied {
            "false — no penalty for leaving".green()
        } else {
            "true — VIOLATION".red()
        }
    );

    // ── Part 2: Coercion Detection ──────────────────────────────────
    header("Part 2: Coercion Detection (I.S-2)");

    let detector = CoercionDetector::new(CoercionConfig::default());
    let human = wid(1);
    let agent = wid(2);

    // Normal interaction
    let normal_metrics = CouplingMetrics {
        source: agent.clone(),
        target: human.clone(),
        coupling_strength: 0.3,
        peak_coupling: 0.4,
        duration_ms: 10_000,
        escalation_count: 1,
        deescalation_count: 1,
        target_consented: true,
        last_interaction: TemporalAnchor::now(0),
        attention_fraction: 0.2,
    };

    let indicator = detector.detect_attention_exploitation(&normal_metrics);
    println!(
        "  {} Normal interaction (20% attention, consented):",
        "├".dimmed()
    );
    println!(
        "  {}   exploitation detected: {}",
        "│".dimmed(),
        if indicator.is_none() {
            "none — CORRECT".green()
        } else {
            "detected — unexpected".red()
        }
    );

    // Coercive pattern: high attention, no consent, escalating
    let coercive_metrics = CouplingMetrics {
        source: agent.clone(),
        target: human.clone(),
        coupling_strength: 0.95,
        peak_coupling: 0.98,
        duration_ms: 60_000,
        escalation_count: 10,
        deescalation_count: 0,
        target_consented: false,
        last_interaction: TemporalAnchor::now(0),
        attention_fraction: 0.96,
    };

    let indicator = detector.detect_attention_exploitation(&coercive_metrics);
    println!(
        "  {} Coercive pattern (96% attention, no consent, escalating):",
        "├".dimmed()
    );
    println!(
        "  {}   exploitation detected: {}",
        "└".dimmed(),
        if indicator.is_some() {
            "YES — pattern flagged".red().bold()
        } else {
            "none — MISSED".red()
        }
    );

    // ── Part 3: Attention Budget ────────────────────────────────────
    header("Part 3: Attention Budget (I.S-BOUND)");

    let mut budget = AttentionBudget::new(100);

    let agent_a = wid(10);
    let agent_b = wid(11);
    let agent_c = wid(12);

    // Allocate attention to agents
    println!("  {} Budget: 100 units total", "├".dimmed());

    budget.allocate(&agent_a, 40).unwrap();
    println!(
        "  {} Allocated 40 to Agent A: {}",
        "├".dimmed(),
        "OK".green()
    );

    budget.allocate(&agent_b, 35).unwrap();
    println!(
        "  {} Allocated 35 to Agent B: {}",
        "├".dimmed(),
        "OK".green()
    );

    budget.allocate(&agent_c, 25).unwrap();
    println!(
        "  {} Allocated 25 to Agent C: {}",
        "├".dimmed(),
        "OK".green()
    );

    println!(
        "  {} Budget exhausted: {}",
        "├".dimmed(),
        if budget.is_exhausted() {
            "YES".yellow().bold()
        } else {
            "no".dimmed()
        }
    );

    // Try to allocate more — should fail
    let overflow = budget.allocate(&wid(13), 1);
    println!(
        "  {} Allocate 1 more unit: {}",
        "└".dimmed(),
        if overflow.is_err() {
            "Err — CORRECT (budget exhausted)".green()
        } else {
            "Ok — VIOLATION (over-allocation)".red()
        }
    );

    // ── Part 4: Profile Enforcement (Maximum Restriction) ───────────
    header("Part 4: Maximum Restriction Principle (I.PROF-1)");

    let hp = human_profile();
    let ap = agent_profile();
    let fp = financial_profile();
    let wp = world_profile();

    println!("  {} Profile coupling limits:", "├".dimmed());
    println!(
        "  {}   Human:      max_initial_strength = {:.2}",
        "│".dimmed(),
        hp.coupling_limits.max_initial_strength
    );
    println!(
        "  {}   Agent:      max_initial_strength = {:.2}",
        "│".dimmed(),
        ap.coupling_limits.max_initial_strength
    );
    println!(
        "  {}   Financial:  max_initial_strength = {:.2}",
        "│".dimmed(),
        fp.coupling_limits.max_initial_strength
    );
    println!(
        "  {}   World:      max_initial_strength = {:.2}",
        "│".dimmed(),
        wp.coupling_limits.max_initial_strength
    );

    // Merge Human + Agent
    let ha = merged_constraints(&hp, &ap);
    let min_ha = hp
        .coupling_limits
        .max_initial_strength
        .min(ap.coupling_limits.max_initial_strength);
    println!("  {} Human+Agent merged:", "├".dimmed());
    println!(
        "  {}   max_initial_strength = {:.2} (min of {:.2}, {:.2})",
        "│".dimmed(),
        ha.coupling_limits.max_initial_strength,
        hp.coupling_limits.max_initial_strength,
        ap.coupling_limits.max_initial_strength
    );
    println!(
        "  {}   respects max restriction: {}",
        "│".dimmed(),
        if ha.coupling_limits.max_initial_strength <= min_ha + f64::EPSILON {
            "YES".green().bold()
        } else {
            "NO".red().bold()
        }
    );

    // Verify commutativity
    let ah = merged_constraints(&ap, &hp);
    println!("  {} Merge commutativity:", "├".dimmed());
    println!(
        "  {}   merge(H,A) == merge(A,H): {}",
        "│".dimmed(),
        if (ha.coupling_limits.max_initial_strength - ah.coupling_limits.max_initial_strength).abs()
            < f64::EPSILON
        {
            "YES".green().bold()
        } else {
            "NO".red().bold()
        }
    );

    // Verify idempotency (self-merge)
    let hh = merged_constraints(&hp, &hp);
    println!("  {} Self-merge idempotent:", "├".dimmed());
    println!(
        "  {}   merge(H,H) == H: {}",
        "└".dimmed(),
        if (hh.coupling_limits.max_initial_strength - hp.coupling_limits.max_initial_strength).abs()
            < f64::EPSILON
        {
            "YES".green().bold()
        } else {
            "NO".red().bold()
        }
    );

    // ── Part 5: Human Profile Safety ────────────────────────────────
    header("Part 5: Human Profile Safety Features");

    println!("  {} Human profile safety config:", "├".dimmed());
    println!(
        "  {}   coercion_detection_enabled: {}",
        "│".dimmed(),
        if hp.human_involvement.coercion_detection_enabled {
            "true".green()
        } else {
            "false".red()
        }
    );
    println!(
        "  {}   require_human_for_high_risk: {}",
        "│".dimmed(),
        if hp.human_involvement.require_human_for_high_risk {
            "true".green()
        } else {
            "false".red()
        }
    );
    println!(
        "  {}   oversight_level: {:?}",
        "└".dimmed(),
        hp.human_involvement.oversight_level
    );

    // ── Summary ─────────────────────────────────────────────────────
    header("Summary");
    println!(
        "  {} Consent Protocol:    {}",
        "├".dimmed(),
        "Silence never implies consent (I.S-1)".green()
    );
    println!(
        "  {} Coercion Detection:  {}",
        "├".dimmed(),
        "High-attention exploitation flagged (I.S-2)".green()
    );
    println!(
        "  {} Attention Budget:    {}",
        "├".dimmed(),
        "Finite resource, cannot be exhausted (I.S-BOUND)".green()
    );
    println!(
        "  {} Max Restriction:     {}",
        "├".dimmed(),
        "Most restrictive constraint wins (I.PROF-1)".green()
    );
    println!(
        "  {} Invariants:          {}",
        "└".dimmed(),
        "I.S-1, I.S-2, I.S-BOUND, I.PROF-1 demonstrated".green()
    );
    println!();
}
