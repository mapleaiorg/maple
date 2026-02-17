use maple_waf_context_graph::IntentNode;
#[cfg(test)]
use maple_waf_context_graph::GovernanceTier;

/// Builds system prompts for LLM synthesis with invariant awareness.
pub struct SystemPromptBuilder;

impl SystemPromptBuilder {
    /// Build a system prompt for synthesis based on an intent.
    pub fn build_system_prompt(intent: &IntentNode) -> String {
        let mut prompt = String::new();

        prompt.push_str("You are an evolution synthesizer for the MAPLE WorldLine Framework.\n");
        prompt.push_str("Generate code changes that satisfy the following intent:\n\n");

        // Intent description.
        prompt.push_str(&format!("## Intent\n{}\n\n", intent.description));

        // Target metrics.
        if !intent.target_metrics.is_empty() {
            prompt.push_str("## Target Metrics\n");
            for (metric, target) in &intent.target_metrics {
                prompt.push_str(&format!("- {}: {}\n", metric, target));
            }
            prompt.push('\n');
        }

        // Governance constraints.
        prompt.push_str(&format!(
            "## Governance Tier: {}\n",
            intent.governance_tier
        ));
        if intent.governance_tier.requires_human_approval() {
            prompt.push_str("⚠ This change requires human review.\n");
        }
        if intent.governance_tier.requires_formal_verification() {
            prompt.push_str("⚠ This change requires formal verification.\n");
        }
        prompt.push('\n');

        // Invariants.
        prompt.push_str("## Non-Negotiable Invariants\n");
        prompt.push_str(INVARIANT_BLOCK);
        prompt.push('\n');

        // Safety.
        prompt.push_str("## Safety Requirements\n");
        prompt.push_str("- NEVER use `unsafe` code\n");
        prompt.push_str("- All changes must be backward-compatible unless governance tier ≥ 3\n");
        prompt.push_str("- Every hypothesis must include test strategy\n");
        prompt.push_str("- Confidence and safety scores must be calibrated\n");

        prompt
    }

    /// Build a hypothesis evaluation prompt.
    pub fn build_evaluation_prompt(hypothesis_code: &str) -> String {
        format!(
            "Evaluate this code hypothesis for safety, correctness, and performance:\n\n```\n{}\n```\n\nProvide:\n1. Safety score [0.0-1.0]\n2. Confidence [0.0-1.0]\n3. Risk assessment\n4. Invariant compliance check",
            hypothesis_code
        )
    }
}

const INVARIANT_BLOCK: &str = "\
I.1  Identity Persistence: WorldLine ID unique and immutable
I.2  Causal Provenance: Every state change signed and linked
I.3  Axiomatic Primacy: Evolution never violates core axioms
I.4  Resonance Minimum: System halts if R < R_min
I.5  Commitment Gating: No side effects without commitment
I.6  State Isolation: Evolution cannot mutate persistence plane
I.7  Evidence Requirement: Self-upgrades require valid EvidenceBundle
I.8  Human Agency Override: System couplable to human intent
I.WAF-1  Context Graph Integrity
I.WAF-2  Synthesis Traceability
I.WAF-3  Swap Atomicity
I.WAF-4  Rollback Guarantee
I.WAF-5  Evidence Completeness
I.WAF-6  Resonance Monotonicity\n";

#[cfg(test)]
mod tests {
    use super::*;
    use worldline_types::EventId;

    #[test]
    fn system_prompt_contains_intent() {
        let intent = IntentNode::new(EventId::new(), "reduce latency", GovernanceTier::Tier0);
        let prompt = SystemPromptBuilder::build_system_prompt(&intent);
        assert!(prompt.contains("reduce latency"));
    }

    #[test]
    fn system_prompt_contains_metrics() {
        let intent = IntentNode::new(EventId::new(), "test", GovernanceTier::Tier0)
            .with_metric("cpu_pct", -10.0);
        let prompt = SystemPromptBuilder::build_system_prompt(&intent);
        assert!(prompt.contains("cpu_pct"));
    }

    #[test]
    fn system_prompt_contains_invariants() {
        let intent = IntentNode::new(EventId::new(), "test", GovernanceTier::Tier0);
        let prompt = SystemPromptBuilder::build_system_prompt(&intent);
        assert!(prompt.contains("I.WAF-1"));
        assert!(prompt.contains("I.WAF-6"));
    }

    #[test]
    fn high_tier_warns_human_review() {
        let intent = IntentNode::new(EventId::new(), "test", GovernanceTier::Tier4);
        let prompt = SystemPromptBuilder::build_system_prompt(&intent);
        assert!(prompt.contains("human review"));
        assert!(prompt.contains("formal verification"));
    }

    #[test]
    fn evaluation_prompt_includes_code() {
        let prompt = SystemPromptBuilder::build_evaluation_prompt("fn foo() {}");
        assert!(prompt.contains("fn foo() {}"));
        assert!(prompt.contains("Safety score"));
    }
}
