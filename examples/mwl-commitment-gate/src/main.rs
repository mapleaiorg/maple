//! MWL Example 12: Commitment Gate — 7-Stage Adjudication Pipeline
//!
//! Demonstrates the Commitment Boundary — the hard architectural line between
//! cognition and action. Nothing crosses into execution without going through
//! all 7 stages of the Commitment Gate.
//!
//! ## 7 Stages
//!
//! 1. Declaration — Structural validation
//! 2. Identity Binding — WorldlineId verification
//! 3. Capability Check — Declared capabilities sufficient
//! 4. Policy Evaluation — Governance policies applied
//! 5. Risk Assessment — Risk thresholds checked
//! 6. Co-signature Collection — Multi-party approval
//! 7. Final Decision — PolicyDecisionCard emitted
//!
//! Constitutional Invariants demonstrated:
//! - I.3 (Commitment Boundary): Only explicit commitments cross into execution
//! - I.5 (Pre-Execution Accountability): Accountability before execution
//! - I.CG-1 (Decision Immutability): PolicyDecisionCards are immutable

use std::sync::Arc;

use colored::Colorize;
use worldline_core::identity::IdentityManager;
use worldline_core::types::{
    CapabilityId, CommitmentScope, EffectDomain, EventId, IdentityMaterial, WorldlineId,
};
use worldline_runtime::fabric::{EventFabric, FabricConfig};
use worldline_runtime::gate::{
    AdjudicationResult, CapabilityCheckStage, CoSignatureStage, CommitmentDeclaration,
    CommitmentGate, CommitmentOutcome, DeclarationStage, FinalDecisionStage, GateConfig,
    IdentityBindingStage, LedgerFilter, MockCapabilityProvider, MockPolicyProvider,
    PolicyEvaluationStage, PolicyProvider, RiskAssessmentStage, RiskConfig,
};

fn header(title: &str) {
    println!();
    println!("{}", "═".repeat(72).cyan());
    println!("  {}", title.cyan().bold());
    println!("{}", "═".repeat(72).cyan());
}

/// Build a fully wired Commitment Gate with all 7 stages.
async fn build_gate(
    approve_policy: bool,
) -> (
    CommitmentGate,
    WorldlineId,
    Arc<std::sync::RwLock<IdentityManager>>,
) {
    let fabric = Arc::new(EventFabric::init(FabricConfig::default()).await.unwrap());

    let mut identity_mgr = IdentityManager::new();
    let material = IdentityMaterial::GenesisHash([1u8; 32]);
    let wid = identity_mgr.create_worldline(material).unwrap();
    let identity_mgr = Arc::new(std::sync::RwLock::new(identity_mgr));

    let cap_provider = MockCapabilityProvider::new();
    cap_provider.grant(wid.clone(), "CAP-COMM", EffectDomain::Communication);
    cap_provider.grant(wid.clone(), "CAP-FIN", EffectDomain::Financial);
    let cap_provider = Arc::new(cap_provider);

    let policy_provider: Arc<dyn PolicyProvider> = if approve_policy {
        Arc::new(MockPolicyProvider::approve_all())
    } else {
        Arc::new(MockPolicyProvider::deny_all())
    };

    let config = GateConfig {
        min_intent_confidence: 0.6,
        require_intent_reference: true,
    };

    let mut gate = CommitmentGate::new(fabric, config.clone());

    // Wire up all 7 stages
    gate.add_stage(Box::new(DeclarationStage::new(
        config.require_intent_reference,
        config.min_intent_confidence,
    )));
    gate.add_stage(Box::new(IdentityBindingStage::new(identity_mgr.clone())));
    gate.add_stage(Box::new(CapabilityCheckStage::new(cap_provider)));
    gate.add_stage(Box::new(PolicyEvaluationStage::new(policy_provider)));
    gate.add_stage(Box::new(RiskAssessmentStage::new(RiskConfig::default())));
    gate.add_stage(Box::new(CoSignatureStage::new()));
    gate.add_stage(Box::new(FinalDecisionStage::new()));

    (gate, wid, identity_mgr)
}

fn print_result(result: &AdjudicationResult) {
    match result {
        AdjudicationResult::Approved { decision } => {
            println!(
                "  {}   result:     {}",
                "│".dimmed(),
                "APPROVED".green().bold()
            );
            println!(
                "  {}   decision:   {}",
                "│".dimmed(),
                decision.decision_id.dimmed()
            );
            println!("  {}   risk:       {:?}", "│".dimmed(), decision.risk.class);
            println!(
                "  {}   rationale:  {}",
                "│".dimmed(),
                decision.rationale.green()
            );
        }
        AdjudicationResult::Denied { decision } => {
            println!("  {}   result:     {}", "│".dimmed(), "DENIED".red().bold());
            println!(
                "  {}   decision:   {}",
                "│".dimmed(),
                decision.decision_id.dimmed()
            );
            println!(
                "  {}   rationale:  {}",
                "│".dimmed(),
                decision.rationale.red()
            );
        }
        AdjudicationResult::PendingCoSign { required } => {
            println!(
                "  {}   result:     {}",
                "│".dimmed(),
                "PENDING CO-SIGN".yellow().bold()
            );
            println!(
                "  {}   required:   {} signers",
                "│".dimmed(),
                required.len()
            );
        }
        AdjudicationResult::PendingHumanApproval { approver } => {
            println!(
                "  {}   result:     {}",
                "│".dimmed(),
                "PENDING HUMAN APPROVAL".yellow().bold()
            );
            println!("  {}   approver:   {}", "│".dimmed(), approver);
        }
    }
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
        "║    MWL Example 12: Commitment Gate Demo                     ║"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════════╝".cyan()
    );

    // ── Scenario 1: Valid commitment → Approved ─────────────────────
    header("Scenario 1: Valid Commitment (all stages pass)");

    let (mut gate, wid, _identity_mgr) = build_gate(true).await;

    let target = WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32]));

    let decl = CommitmentDeclaration::builder(
        wid.clone(),
        CommitmentScope {
            effect_domain: EffectDomain::Communication,
            targets: vec![target.clone()],
            constraints: vec!["max_100_messages".into()],
        },
    )
    .derived_from_intent(EventId::new())
    .capability(CapabilityId("CAP-COMM".into()))
    .affected_party(target.clone())
    .evidence("user explicitly requested greeting")
    .build();

    let cid = decl.id.clone();
    println!("  {} Submitting: Communication commitment", "├".dimmed());
    println!(
        "  {}   domain:      {}",
        "│".dimmed(),
        "Communication".blue()
    );
    println!("  {}   capability:  {}", "│".dimmed(), "CAP-COMM".blue());

    let result = gate.submit(decl).await.unwrap();
    print_result(&result);

    // Record outcome
    gate.record_outcome(&cid, CommitmentOutcome::Fulfilled)
        .await
        .unwrap();
    println!("  {} Outcome: {}", "└".dimmed(), "FULFILLED".green().bold());

    // ── Scenario 2: Missing intent reference → Denied at Stage 1 ────
    header("Scenario 2: Missing Intent Reference (Stage 1 rejects)");

    let (mut gate, wid, _) = build_gate(true).await;

    let decl_no_intent = CommitmentDeclaration::builder(
        wid.clone(),
        CommitmentScope {
            effect_domain: EffectDomain::Communication,
            targets: vec![target.clone()],
            constraints: vec![],
        },
    )
    // NO .derived_from_intent() — violates I.3
    .build();

    println!(
        "  {} Submitting: Commitment WITHOUT intent reference",
        "├".dimmed()
    );
    println!("  {}   intent ref: {}", "│".dimmed(), "NONE".red());

    let result = gate.submit(decl_no_intent).await.unwrap();
    print_result(&result);
    println!(
        "  {} I.3 enforced: Intent required before commitment",
        "└".dimmed()
    );

    // ── Scenario 3: Unknown identity → Denied at Stage 2 ───────────
    header("Scenario 3: Unknown Identity (Stage 2 rejects)");

    let (mut gate, _, _) = build_gate(true).await;

    let unknown = WorldlineId::derive(&IdentityMaterial::GenesisHash([99u8; 32]));

    let decl_unknown = CommitmentDeclaration::builder(
        unknown.clone(),
        CommitmentScope {
            effect_domain: EffectDomain::Communication,
            targets: vec![target.clone()],
            constraints: vec![],
        },
    )
    .derived_from_intent(EventId::new())
    .build();

    println!(
        "  {} Submitting: Commitment from unknown identity",
        "├".dimmed()
    );
    println!(
        "  {}   identity: {}",
        "│".dimmed(),
        format!("{}", unknown).red()
    );

    let result = gate.submit(decl_unknown).await.unwrap();
    print_result(&result);
    println!(
        "  {} I.5 enforced: Identity must be verified before execution",
        "└".dimmed()
    );

    // ── Scenario 4: Policy denial ───────────────────────────────────
    header("Scenario 4: Policy Denial (Stage 4 rejects)");

    let (mut gate, wid, _) = build_gate(false).await; // deny_all policy

    let decl_policy_fail = CommitmentDeclaration::builder(
        wid.clone(),
        CommitmentScope {
            effect_domain: EffectDomain::Communication,
            targets: vec![target.clone()],
            constraints: vec![],
        },
    )
    .derived_from_intent(EventId::new())
    .capability(CapabilityId("CAP-COMM".into()))
    .build();

    println!(
        "  {} Submitting: Commitment against deny-all policy",
        "├".dimmed()
    );

    let result = gate.submit(decl_policy_fail).await.unwrap();
    print_result(&result);
    println!("  {} Governance policies are authoritative", "└".dimmed());

    // ── Scenario 5: Ledger audit ────────────────────────────────────
    header("Scenario 5: Commitment Ledger Audit (I.AAS-3)");

    let (mut gate, wid, _) = build_gate(true).await;

    // Submit several commitments
    for i in 0..3 {
        let decl = CommitmentDeclaration::builder(
            wid.clone(),
            CommitmentScope {
                effect_domain: EffectDomain::Communication,
                targets: vec![target.clone()],
                constraints: vec![format!("batch-{}", i)],
            },
        )
        .derived_from_intent(EventId::new())
        .capability(CapabilityId("CAP-COMM".into()))
        .build();

        gate.submit(decl).await.unwrap();
    }

    let filter = LedgerFilter::new().with_worldline(wid.clone());
    let entries = gate.query_ledger(&filter);
    println!(
        "  {} Ledger entries for Alice: {}",
        "├".dimmed(),
        format!("{}", entries.len()).yellow()
    );

    for (i, entry) in entries.iter().enumerate() {
        let prefix = if i < entries.len() - 1 {
            "│  ├"
        } else {
            "│  └"
        };
        println!(
            "  {}  {} → {:?}",
            prefix.dimmed(),
            entry.commitment_id,
            entry.decision.decision,
        );
    }

    println!("  {} Ledger is append-only (I.AAS-3)", "└".dimmed());

    // ── Summary ─────────────────────────────────────────────────────
    header("Summary");
    println!(
        "  {} 7-stage pipeline:          {}",
        "├".dimmed(),
        "Declaration → Identity → Capability → Policy → Risk → CoSign → Final".green()
    );
    println!(
        "  {} Stage 1 (Declaration):     {}",
        "├".dimmed(),
        "validates intent reference (I.3)".green()
    );
    println!(
        "  {} Stage 2 (Identity):        {}",
        "├".dimmed(),
        "verifies WorldlineId exists".green()
    );
    println!(
        "  {} Stage 3 (Capability):      {}",
        "├".dimmed(),
        "checks bounded authority grants".green()
    );
    println!(
        "  {} Stage 4 (Policy):          {}",
        "├".dimmed(),
        "governance policies are authoritative".green()
    );
    println!(
        "  {} Denied commitments:        {}",
        "├".dimmed(),
        "first-class records in the ledger".green()
    );
    println!(
        "  {} Constitutional invariants: {}",
        "└".dimmed(),
        "I.3, I.5, I.CG-1, I.AAS-3 demonstrated".green()
    );
    println!();
}
