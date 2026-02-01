//! PALM Platform Boundary Enforcement Demo
//!
//! This demo shows how the same operations behave differently
//! across Mapleverse, Finalverse, and iBank platform packs.

use palm_policy::{HumanApproval, PolicyDecision, PolicyEvaluationContext, PolicyEvaluator};
use palm_types::policy::PalmOperation;
use palm_types::PlatformProfile;

use colored::*;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!(
        "{}",
        "╔══════════════════════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║       PALM Platform Boundary Enforcement Demonstration           ║".cyan()
    );
    println!(
        "{}",
        "║                                                                  ║".cyan()
    );
    println!(
        "{}",
        "║  This demo shows how the same operations behave differently      ║".cyan()
    );
    println!(
        "{}",
        "║  across Mapleverse, Finalverse, and iBank platform policies.     ║".cyan()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════════════╝".cyan()
    );
    println!();

    // Initialize policy evaluators for each platform
    let mapleverse_evaluator = PolicyEvaluator::new(PlatformProfile::Mapleverse)
        .with_emit_audit_events(false);
    let finalverse_evaluator = PolicyEvaluator::new(PlatformProfile::Finalverse)
        .with_emit_audit_events(false);
    let ibank_evaluator = PolicyEvaluator::new(PlatformProfile::IBank)
        .with_emit_audit_events(false);

    // Demo scenarios
    demo_delete_deployment(&mapleverse_evaluator, &finalverse_evaluator, &ibank_evaluator).await;
    println!();

    demo_scale_operation(&mapleverse_evaluator, &finalverse_evaluator, &ibank_evaluator).await;
    println!();

    demo_force_recovery(&mapleverse_evaluator, &finalverse_evaluator, &ibank_evaluator).await;
    println!();

    demo_configuration_comparison();

    println!();
    println!("{}", "Demo complete!".green().bold());
}

async fn demo_delete_deployment(
    mapleverse: &PolicyEvaluator,
    finalverse: &PolicyEvaluator,
    ibank: &PolicyEvaluator,
) {
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .yellow()
    );
    println!(
        "{}",
        "  Scenario 1: DELETE DEPLOYMENT Operation".yellow().bold()
    );
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .yellow()
    );
    println!();

    let operation = PalmOperation::DeleteDeployment {
        deployment_id: "demo-deployment-001".to_string(),
    };

    println!("  Operation: Delete deployment 'demo-deployment-001'");
    println!();

    // Test Mapleverse (no human approval needed)
    println!("  {} (without human approval):", "Mapleverse".blue().bold());
    let ctx = PolicyEvaluationContext::new("operator-1", PlatformProfile::Mapleverse);
    let decision = mapleverse.evaluate(&operation, &ctx).await.unwrap();
    print_decision(&decision, "    ");
    println!(
        "    {} Mapleverse prioritizes throughput - allows quick deletions",
        "→".cyan()
    );

    println!();
    println!(
        "  {} (without human approval):",
        "Finalverse".green().bold()
    );
    let ctx = PolicyEvaluationContext::new("operator-1", PlatformProfile::Finalverse);
    let decision = finalverse.evaluate(&operation, &ctx).await.unwrap();
    print_decision(&decision, "    ");
    println!(
        "    {} Finalverse requires human approval for destructive operations",
        "→".cyan()
    );

    println!();
    println!("  {} (with human approval):", "Finalverse".green().bold());
    let approval = HumanApproval::new("admin@finalverse.io")
        .with_reason("Authorized deletion");
    let ctx = PolicyEvaluationContext::new("operator-1", PlatformProfile::Finalverse)
        .with_human_approval(approval);
    let decision = finalverse.evaluate(&operation, &ctx).await.unwrap();
    print_decision(&decision, "    ");
    println!(
        "    {} With approval, Finalverse allows the operation",
        "→".cyan()
    );

    println!();
    println!(
        "  {} (without accountability proof):",
        "iBank".magenta().bold()
    );
    let ctx = PolicyEvaluationContext::new("operator-1", PlatformProfile::IBank);
    let decision = ibank.evaluate(&operation, &ctx).await.unwrap();
    print_decision(&decision, "    ");
    println!(
        "    {} iBank requires full accountability trail for all operations",
        "→".cyan()
    );
}

async fn demo_scale_operation(
    mapleverse: &PolicyEvaluator,
    finalverse: &PolicyEvaluator,
    ibank: &PolicyEvaluator,
) {
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .yellow()
    );
    println!(
        "{}",
        "  Scenario 2: SCALE DEPLOYMENT Operation (scale up by 100)"
            .yellow()
            .bold()
    );
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .yellow()
    );
    println!();

    let operation = PalmOperation::ScaleDeployment {
        deployment_id: "demo-deployment-001".to_string(),
        target_replicas: 100,
    };

    println!("  Operation: Scale deployment to 100 replicas");
    println!();

    println!("  {} :", "Mapleverse".blue().bold());
    let ctx = PolicyEvaluationContext::new("operator-1", PlatformProfile::Mapleverse);
    let decision = mapleverse.evaluate(&operation, &ctx).await.unwrap();
    print_decision(&decision, "    ");
    println!(
        "    {} Mapleverse allows rapid scaling without approval",
        "→".cyan()
    );

    println!();
    println!("  {} :", "Finalverse".green().bold());
    let ctx = PolicyEvaluationContext::new("operator-1", PlatformProfile::Finalverse);
    let decision = finalverse.evaluate(&operation, &ctx).await.unwrap();
    print_decision(&decision, "    ");
    println!(
        "    {} Finalverse may require approval for large scale operations",
        "→".cyan()
    );

    println!();
    println!("  {} :", "iBank".magenta().bold());
    let ctx = PolicyEvaluationContext::new("operator-1", PlatformProfile::IBank);
    let decision = ibank.evaluate(&operation, &ctx).await.unwrap();
    print_decision(&decision, "    ");
    println!(
        "    {} iBank requires accountability proof for all operations",
        "→".cyan()
    );
}

async fn demo_force_recovery(
    mapleverse: &PolicyEvaluator,
    finalverse: &PolicyEvaluator,
    ibank: &PolicyEvaluator,
) {
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .yellow()
    );
    println!(
        "{}",
        "  Scenario 3: FORCE RECOVERY Operation".yellow().bold()
    );
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .yellow()
    );
    println!();

    let operation = PalmOperation::ForceRecovery {
        instance_id: "instance-001".to_string(),
    };

    println!("  Operation: Force recovery of failed instance");
    println!();

    println!("  {} :", "Mapleverse".blue().bold());
    let ctx = PolicyEvaluationContext::new("operator-1", PlatformProfile::Mapleverse);
    let decision = mapleverse.evaluate(&operation, &ctx).await.unwrap();
    print_decision(&decision, "    ");
    println!(
        "    {} Mapleverse allows force recovery for fast restoration",
        "→".cyan()
    );

    println!();
    println!("  {} (with human approval):", "Finalverse".green().bold());
    let approval = HumanApproval::new("admin@finalverse.io")
        .with_reason("Emergency recovery authorized");
    let ctx = PolicyEvaluationContext::new("operator-1", PlatformProfile::Finalverse)
        .with_human_approval(approval);
    let decision = finalverse.evaluate(&operation, &ctx).await.unwrap();
    print_decision(&decision, "    ");
    println!(
        "    {} Finalverse requires human approval for force operations",
        "→".cyan()
    );

    println!();
    println!("  {} (with everything):", "iBank".magenta().bold());
    let approval = HumanApproval::new("admin@ibank.com")
        .with_reason("Force recovery with full audit trail");
    let ctx = PolicyEvaluationContext::new("operator-1", PlatformProfile::IBank)
        .with_human_approval(approval);
    let decision = ibank.evaluate(&operation, &ctx).await.unwrap();
    print_decision(&decision, "    ");
    println!(
        "    {} iBank may block or require extra accountability for force operations!",
        "→".red().bold()
    );
}

fn demo_configuration_comparison() {
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .yellow()
    );
    println!(
        "{}",
        "  Configuration Comparison Table".yellow().bold()
    );
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .yellow()
    );
    println!();

    // Create platform capabilities for comparison
    let mv_caps = palm_platform_pack::PlatformCapabilities::mapleverse();
    let fv_caps = palm_platform_pack::PlatformCapabilities::finalverse();
    let ib_caps = palm_platform_pack::PlatformCapabilities::ibank();

    println!("  ┌─────────────────────────────┬────────────┬────────────┬────────────┐");
    println!(
        "  │ {}                │ {} │ {} │ {}    │",
        "Feature".bold(),
        "Mapleverse".blue(),
        "Finalverse".green(),
        "iBank".magenta()
    );
    println!("  ├─────────────────────────────┼────────────┼────────────┼────────────┤");

    // Max instances
    let mv_max = mv_caps
        .max_total_instances
        .map(|n| format!("{:>10}", format_number(n as u64)))
        .unwrap_or_else(|| "unlimited".to_string());
    let fv_max = fv_caps
        .max_total_instances
        .map(|n| format!("{:>10}", format_number(n as u64)))
        .unwrap_or_else(|| "unlimited".to_string());
    let ib_max = ib_caps
        .max_total_instances
        .map(|n| format!("{:>10}", format_number(n as u64)))
        .unwrap_or_else(|| "unlimited".to_string());
    println!(
        "  │ Max Total Instances         │ {:>10} │ {:>10} │ {:>10} │",
        mv_max, fv_max, ib_max
    );

    // Human approval
    let mv_ha = format_bool_short(mv_caps.supports_human_approval);
    let fv_ha = format_bool_short(fv_caps.supports_human_approval);
    let ib_ha = format_bool_short(ib_caps.supports_human_approval);
    println!(
        "  │ Human Approval Supported    │ {:>10} │ {:>10} │ {:>10} │",
        mv_ha, fv_ha, ib_ha
    );

    // Hot reload
    let mv_hr = format_bool_short(mv_caps.supports_hot_reload);
    let fv_hr = format_bool_short(fv_caps.supports_hot_reload);
    let ib_hr = format_bool_short(ib_caps.supports_hot_reload);
    println!(
        "  │ Hot Reload Supported        │ {:>10} │ {:>10} │ {:>10} │",
        mv_hr, fv_hr, ib_hr
    );

    // Migration
    let mv_mi = format_bool_short(mv_caps.supports_migration);
    let fv_mi = format_bool_short(fv_caps.supports_migration);
    let ib_mi = format_bool_short(ib_caps.supports_migration);
    println!(
        "  │ Live Migration Supported    │ {:>10} │ {:>10} │ {:>10} │",
        mv_mi, fv_mi, ib_mi
    );

    // Cross-node migration
    let mv_cn = format_bool_short(mv_caps.supports_cross_node_migration);
    let fv_cn = format_bool_short(fv_caps.supports_cross_node_migration);
    let ib_cn = format_bool_short(ib_caps.supports_cross_node_migration);
    println!(
        "  │ Cross-Node Migration        │ {:>10} │ {:>10} │ {:>10} │",
        mv_cn, fv_cn, ib_cn
    );

    // Checkpoints
    let mv_cp = format_bool_short(mv_caps.supports_checkpoints);
    let fv_cp = format_bool_short(fv_caps.supports_checkpoints);
    let ib_cp = format_bool_short(ib_caps.supports_checkpoints);
    println!(
        "  │ Checkpoints Supported       │ {:>10} │ {:>10} │ {:>10} │",
        mv_cp, fv_cp, ib_cp
    );

    // Canary
    let mv_ca = format_bool_short(mv_caps.supports_canary);
    let fv_ca = format_bool_short(fv_caps.supports_canary);
    let ib_ca = format_bool_short(ib_caps.supports_canary);
    println!(
        "  │ Canary Deployments          │ {:>10} │ {:>10} │ {:>10} │",
        mv_ca, fv_ca, ib_ca
    );

    println!("  └─────────────────────────────┴────────────┴────────────┴────────────┘");

    println!();
    println!("  {}", "Key Insights:".bold());
    println!(
        "    {} Mapleverse: Optimized for throughput, minimal safety constraints",
        "•".blue()
    );
    println!(
        "    {} Finalverse: Balanced for safety, human oversight required",
        "•".green()
    );
    println!(
        "    {} iBank: Maximum accountability, audit trail mandatory",
        "•".magenta()
    );

    println!();
    println!("  {}", "Policy Philosophy:".bold());
    println!();
    println!(
        "    {} {} - \"Move fast, recover faster\"",
        "Mapleverse:".blue().bold(),
        ""
    );
    println!("      Priority: Throughput > Recovery > Safety");
    println!("      Use case: High-velocity swarm orchestration");
    println!();
    println!(
        "    {} {} - \"Safety first, speed second\"",
        "Finalverse:".green().bold(),
        ""
    );
    println!("      Priority: Safety > Correctness > Throughput");
    println!("      Use case: Human-centric world simulation");
    println!();
    println!(
        "    {} {} - \"Accountability above all\"",
        "iBank:".magenta().bold(),
        ""
    );
    println!("      Priority: Accountability > Auditability > Correctness");
    println!("      Use case: Autonomous financial operations");
}

fn print_decision(decision: &PolicyDecision, indent: &str) {
    match decision {
        PolicyDecision::Allow => {
            println!(
                "{}Decision: {} - Operation permitted",
                indent,
                "ALLOW".green().bold()
            );
        }
        PolicyDecision::Deny { reason, policy_id } => {
            println!(
                "{}Decision: {} - {}",
                indent,
                "DENY".red().bold(),
                reason
            );
            println!("{}Policy: {}", indent, policy_id);
        }
        PolicyDecision::RequiresApproval {
            approvers,
            reason,
            policy_id,
        } => {
            println!(
                "{}Decision: {} - {}",
                indent,
                "REQUIRES APPROVAL".yellow().bold(),
                reason
            );
            println!("{}Approvers: {:?}", indent, approvers);
            println!("{}Policy: {}", indent, policy_id);
        }
        PolicyDecision::Hold {
            reason,
            policy_id,
            ..
        } => {
            println!(
                "{}Decision: {} - {}",
                indent,
                "HOLD".cyan().bold(),
                reason
            );
            println!("{}Policy: {}", indent, policy_id);
        }
    }
}

fn format_bool_short(b: bool) -> &'static str {
    if b {
        "Yes"
    } else {
        "No"
    }
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{}M", n / 1_000_000)
    } else if n >= 1_000 {
        format!("{}K", n / 1_000)
    } else {
        n.to_string()
    }
}
