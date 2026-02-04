//! Gate evaluator: checks whether transition gates are satisfied
//!
//! The gate evaluator examines collected receipts and conditions
//! to determine if a workflow edge's transition gate is satisfied.
//! It does NOT produce side effects — it's a pure evaluation function.

use collective_types::ReceiptType;
use workflow_types::{NodeId, TransitionGate, WorkflowReceipt};

/// Evaluates transition gates against collected evidence
#[derive(Clone, Debug)]
pub struct GateEvaluator;

impl GateEvaluator {
    pub fn new() -> Self {
        Self
    }

    /// Evaluate whether a transition gate is satisfied given the collected receipts
    /// and runtime context.
    ///
    /// Returns `true` if the gate is satisfied and the transition can fire.
    pub fn evaluate(
        &self,
        gate: &TransitionGate,
        receipts: &[WorkflowReceipt],
        node_id: &NodeId,
        context: &EvaluationContext,
    ) -> GateResult {
        match gate {
            TransitionGate::Automatic => GateResult::Satisfied,

            TransitionGate::ReceiptEmitted { receipt_type } => {
                if self.has_receipt(receipts, node_id, receipt_type) {
                    GateResult::Satisfied
                } else {
                    GateResult::NotSatisfied {
                        reason: format!(
                            "Receipt type '{:?}' not emitted by node '{}'",
                            receipt_type, node_id
                        ),
                    }
                }
            }

            TransitionGate::AllReceiptsEmitted { receipt_types } => {
                let missing: Vec<_> = receipt_types
                    .iter()
                    .filter(|rt| !self.has_receipt(receipts, node_id, rt))
                    .collect();

                if missing.is_empty() {
                    GateResult::Satisfied
                } else {
                    GateResult::NotSatisfied {
                        reason: format!(
                            "Missing {} receipt(s) from node '{}'",
                            missing.len(),
                            node_id
                        ),
                    }
                }
            }

            TransitionGate::AnyReceiptEmitted { receipt_types } => {
                if receipt_types
                    .iter()
                    .any(|rt| self.has_receipt(receipts, node_id, rt))
                {
                    GateResult::Satisfied
                } else {
                    GateResult::NotSatisfied {
                        reason: format!("No matching receipts emitted by node '{}'", node_id),
                    }
                }
            }

            TransitionGate::Condition { expression } => {
                self.evaluate_condition(expression, context)
            }

            TransitionGate::Timeout { timeout_secs } => {
                if context.node_active_secs >= *timeout_secs as i64 {
                    GateResult::Satisfied
                } else {
                    GateResult::NotSatisfied {
                        reason: format!(
                            "Timeout not elapsed: {} of {} seconds",
                            context.node_active_secs, timeout_secs
                        ),
                    }
                }
            }

            TransitionGate::ThresholdMet {
                description,
                min_signatures,
            } => {
                if context.threshold_signatures >= *min_signatures {
                    GateResult::Satisfied
                } else {
                    GateResult::NotSatisfied {
                        reason: format!(
                            "Threshold '{}' not met: {} of {} signatures",
                            description, context.threshold_signatures, min_signatures
                        ),
                    }
                }
            }

            TransitionGate::AllOf { gates } => {
                let mut reasons = Vec::new();
                for sub_gate in gates {
                    match self.evaluate(sub_gate, receipts, node_id, context) {
                        GateResult::Satisfied => {}
                        GateResult::NotSatisfied { reason } => {
                            reasons.push(reason);
                        }
                    }
                }
                if reasons.is_empty() {
                    GateResult::Satisfied
                } else {
                    GateResult::NotSatisfied {
                        reason: format!("AllOf: {} sub-gates not satisfied", reasons.len()),
                    }
                }
            }

            TransitionGate::AnyOf { gates } => {
                if gates.iter().any(|g| {
                    matches!(
                        self.evaluate(g, receipts, node_id, context),
                        GateResult::Satisfied
                    )
                }) {
                    GateResult::Satisfied
                } else {
                    GateResult::NotSatisfied {
                        reason: "AnyOf: no sub-gates satisfied".into(),
                    }
                }
            }
        }
    }

    /// Check if a specific receipt type has been emitted by a node
    fn has_receipt(
        &self,
        receipts: &[WorkflowReceipt],
        node_id: &NodeId,
        receipt_type: &ReceiptType,
    ) -> bool {
        receipts
            .iter()
            .any(|r| r.node_id == *node_id && r.receipt_type == *receipt_type)
    }

    /// Evaluate a condition expression
    ///
    /// For now, supports simple key-value lookups in the context variables.
    /// Future: full expression language.
    fn evaluate_condition(&self, expression: &str, context: &EvaluationContext) -> GateResult {
        // Simple expression evaluation: "key == value" or "key != value"
        // or boolean variable names
        let expression = expression.trim();

        // Check for "==" comparison
        if let Some((key, value)) = expression.split_once("==") {
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            if let Some(actual) = context.variables.get(key) {
                if actual == value {
                    return GateResult::Satisfied;
                } else {
                    return GateResult::NotSatisfied {
                        reason: format!(
                            "Condition '{}' not met: '{}' != '{}'",
                            expression, actual, value
                        ),
                    };
                }
            }
            return GateResult::NotSatisfied {
                reason: format!("Variable '{}' not found in context", key),
            };
        }

        // Check for "!=" comparison
        if let Some((key, value)) = expression.split_once("!=") {
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            if let Some(actual) = context.variables.get(key) {
                if actual != value {
                    return GateResult::Satisfied;
                } else {
                    return GateResult::NotSatisfied {
                        reason: format!(
                            "Condition '{}' not met: '{}' == '{}'",
                            expression, actual, value
                        ),
                    };
                }
            }
            // Variable not found — inequality is trivially true
            return GateResult::Satisfied;
        }

        // Check for ">=" comparison (numeric)
        if let Some((key, value)) = expression.split_once(">=") {
            let key = key.trim();
            let value = value.trim();
            if let (Some(actual_str), Ok(threshold)) =
                (context.variables.get(key), value.parse::<f64>())
            {
                if let Ok(actual) = actual_str.parse::<f64>() {
                    if actual >= threshold {
                        return GateResult::Satisfied;
                    } else {
                        return GateResult::NotSatisfied {
                            reason: format!(
                                "Condition '{}' not met: {} < {}",
                                expression, actual, threshold
                            ),
                        };
                    }
                }
            }
            return GateResult::NotSatisfied {
                reason: format!("Cannot evaluate numeric condition: {}", expression),
            };
        }

        // Check for boolean variable
        if let Some(val) = context.variables.get(expression) {
            if val == "true" || val == "1" {
                return GateResult::Satisfied;
            }
            return GateResult::NotSatisfied {
                reason: format!(
                    "Boolean variable '{}' is not true (value: '{}')",
                    expression, val
                ),
            };
        }

        GateResult::NotSatisfied {
            reason: format!("Cannot evaluate expression: {}", expression),
        }
    }
}

impl Default for GateEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of evaluating a transition gate
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GateResult {
    /// The gate is satisfied — transition can fire
    Satisfied,
    /// The gate is not satisfied
    NotSatisfied { reason: String },
}

impl GateResult {
    pub fn is_satisfied(&self) -> bool {
        matches!(self, Self::Satisfied)
    }
}

/// Context for gate evaluation
#[derive(Clone, Debug, Default)]
pub struct EvaluationContext {
    /// How long the source node has been active (seconds)
    pub node_active_secs: i64,
    /// Number of threshold signatures collected
    pub threshold_signatures: u32,
    /// Runtime variables (for condition evaluation)
    pub variables: std::collections::HashMap<String, String>,
}

impl EvaluationContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_active_secs(mut self, secs: i64) -> Self {
        self.node_active_secs = secs;
        self
    }

    pub fn with_threshold_signatures(mut self, count: u32) -> Self {
        self.threshold_signatures = count;
        self
    }

    pub fn with_variable(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use collective_types::ReceiptType;
    use resonator_types::ResonatorId;

    fn make_receipt(node: &str, receipt_type: ReceiptType) -> WorkflowReceipt {
        WorkflowReceipt::new(NodeId::new(node), receipt_type, ResonatorId::new("actor-1"))
    }

    #[test]
    fn test_automatic_gate() {
        let evaluator = GateEvaluator::new();
        let result = evaluator.evaluate(
            &TransitionGate::Automatic,
            &[],
            &NodeId::new("start"),
            &EvaluationContext::new(),
        );
        assert!(result.is_satisfied());
    }

    #[test]
    fn test_receipt_emitted_gate() {
        let evaluator = GateEvaluator::new();
        let node = NodeId::new("review");

        // No receipts — not satisfied
        let result = evaluator.evaluate(
            &TransitionGate::ReceiptEmitted {
                receipt_type: ReceiptType::CommitmentFulfilled,
            },
            &[],
            &node,
            &EvaluationContext::new(),
        );
        assert!(!result.is_satisfied());

        // With matching receipt — satisfied
        let receipts = vec![make_receipt("review", ReceiptType::CommitmentFulfilled)];
        let result = evaluator.evaluate(
            &TransitionGate::ReceiptEmitted {
                receipt_type: ReceiptType::CommitmentFulfilled,
            },
            &receipts,
            &node,
            &EvaluationContext::new(),
        );
        assert!(result.is_satisfied());

        // With wrong receipt type — not satisfied
        let result = evaluator.evaluate(
            &TransitionGate::ReceiptEmitted {
                receipt_type: ReceiptType::Audit,
            },
            &receipts,
            &node,
            &EvaluationContext::new(),
        );
        assert!(!result.is_satisfied());
    }

    #[test]
    fn test_all_receipts_gate() {
        let evaluator = GateEvaluator::new();
        let node = NodeId::new("verify");

        let receipts = vec![
            make_receipt("verify", ReceiptType::CommitmentFulfilled),
            make_receipt("verify", ReceiptType::Audit),
        ];

        // All present — satisfied
        let result = evaluator.evaluate(
            &TransitionGate::AllReceiptsEmitted {
                receipt_types: vec![ReceiptType::CommitmentFulfilled, ReceiptType::Audit],
            },
            &receipts,
            &node,
            &EvaluationContext::new(),
        );
        assert!(result.is_satisfied());

        // Missing one — not satisfied
        let result = evaluator.evaluate(
            &TransitionGate::AllReceiptsEmitted {
                receipt_types: vec![
                    ReceiptType::CommitmentFulfilled,
                    ReceiptType::Audit,
                    ReceiptType::Financial,
                ],
            },
            &receipts,
            &node,
            &EvaluationContext::new(),
        );
        assert!(!result.is_satisfied());
    }

    #[test]
    fn test_any_receipt_gate() {
        let evaluator = GateEvaluator::new();
        let node = NodeId::new("check");

        let receipts = vec![make_receipt("check", ReceiptType::CommitmentBroken)];

        let result = evaluator.evaluate(
            &TransitionGate::AnyReceiptEmitted {
                receipt_types: vec![
                    ReceiptType::CommitmentFulfilled,
                    ReceiptType::CommitmentBroken,
                ],
            },
            &receipts,
            &node,
            &EvaluationContext::new(),
        );
        assert!(result.is_satisfied());

        let result = evaluator.evaluate(
            &TransitionGate::AnyReceiptEmitted {
                receipt_types: vec![ReceiptType::Financial],
            },
            &receipts,
            &node,
            &EvaluationContext::new(),
        );
        assert!(!result.is_satisfied());
    }

    #[test]
    fn test_condition_gate_equals() {
        let evaluator = GateEvaluator::new();
        let ctx = EvaluationContext::new().with_variable("status", "approved");

        let result = evaluator.evaluate(
            &TransitionGate::Condition {
                expression: "status == approved".into(),
            },
            &[],
            &NodeId::new("check"),
            &ctx,
        );
        assert!(result.is_satisfied());

        let result = evaluator.evaluate(
            &TransitionGate::Condition {
                expression: "status == rejected".into(),
            },
            &[],
            &NodeId::new("check"),
            &ctx,
        );
        assert!(!result.is_satisfied());
    }

    #[test]
    fn test_condition_gate_not_equals() {
        let evaluator = GateEvaluator::new();
        let ctx = EvaluationContext::new().with_variable("status", "pending");

        let result = evaluator.evaluate(
            &TransitionGate::Condition {
                expression: "status != approved".into(),
            },
            &[],
            &NodeId::new("check"),
            &ctx,
        );
        assert!(result.is_satisfied());
    }

    #[test]
    fn test_condition_gate_numeric() {
        let evaluator = GateEvaluator::new();
        let ctx = EvaluationContext::new().with_variable("score", "85");

        let result = evaluator.evaluate(
            &TransitionGate::Condition {
                expression: "score >= 80".into(),
            },
            &[],
            &NodeId::new("check"),
            &ctx,
        );
        assert!(result.is_satisfied());

        let result = evaluator.evaluate(
            &TransitionGate::Condition {
                expression: "score >= 90".into(),
            },
            &[],
            &NodeId::new("check"),
            &ctx,
        );
        assert!(!result.is_satisfied());
    }

    #[test]
    fn test_condition_gate_boolean() {
        let evaluator = GateEvaluator::new();
        let ctx = EvaluationContext::new().with_variable("approved", "true");

        let result = evaluator.evaluate(
            &TransitionGate::Condition {
                expression: "approved".into(),
            },
            &[],
            &NodeId::new("check"),
            &ctx,
        );
        assert!(result.is_satisfied());
    }

    #[test]
    fn test_timeout_gate() {
        let evaluator = GateEvaluator::new();

        // Not enough time — not satisfied
        let ctx = EvaluationContext::new().with_active_secs(100);
        let result = evaluator.evaluate(
            &TransitionGate::Timeout { timeout_secs: 300 },
            &[],
            &NodeId::new("wait"),
            &ctx,
        );
        assert!(!result.is_satisfied());

        // Enough time — satisfied
        let ctx = EvaluationContext::new().with_active_secs(300);
        let result = evaluator.evaluate(
            &TransitionGate::Timeout { timeout_secs: 300 },
            &[],
            &NodeId::new("wait"),
            &ctx,
        );
        assert!(result.is_satisfied());
    }

    #[test]
    fn test_threshold_met_gate() {
        let evaluator = GateEvaluator::new();

        let ctx = EvaluationContext::new().with_threshold_signatures(2);
        let result = evaluator.evaluate(
            &TransitionGate::ThresholdMet {
                description: "Board approval".into(),
                min_signatures: 3,
            },
            &[],
            &NodeId::new("approval"),
            &ctx,
        );
        assert!(!result.is_satisfied());

        let ctx = EvaluationContext::new().with_threshold_signatures(3);
        let result = evaluator.evaluate(
            &TransitionGate::ThresholdMet {
                description: "Board approval".into(),
                min_signatures: 3,
            },
            &[],
            &NodeId::new("approval"),
            &ctx,
        );
        assert!(result.is_satisfied());
    }

    #[test]
    fn test_all_of_gate() {
        let evaluator = GateEvaluator::new();
        let node = NodeId::new("multi");
        let receipts = vec![make_receipt("multi", ReceiptType::CommitmentFulfilled)];
        let ctx = EvaluationContext::new().with_variable("approved", "true");

        let result = evaluator.evaluate(
            &TransitionGate::AllOf {
                gates: vec![
                    TransitionGate::ReceiptEmitted {
                        receipt_type: ReceiptType::CommitmentFulfilled,
                    },
                    TransitionGate::Condition {
                        expression: "approved".into(),
                    },
                ],
            },
            &receipts,
            &node,
            &ctx,
        );
        assert!(result.is_satisfied());

        // One sub-gate fails
        let result = evaluator.evaluate(
            &TransitionGate::AllOf {
                gates: vec![
                    TransitionGate::ReceiptEmitted {
                        receipt_type: ReceiptType::CommitmentFulfilled,
                    },
                    TransitionGate::ReceiptEmitted {
                        receipt_type: ReceiptType::Audit,
                    },
                ],
            },
            &receipts,
            &node,
            &ctx,
        );
        assert!(!result.is_satisfied());
    }

    #[test]
    fn test_any_of_gate() {
        let evaluator = GateEvaluator::new();
        let node = NodeId::new("either");
        let receipts = vec![make_receipt("either", ReceiptType::CommitmentBroken)];

        let result = evaluator.evaluate(
            &TransitionGate::AnyOf {
                gates: vec![
                    TransitionGate::ReceiptEmitted {
                        receipt_type: ReceiptType::CommitmentFulfilled,
                    },
                    TransitionGate::ReceiptEmitted {
                        receipt_type: ReceiptType::CommitmentBroken,
                    },
                ],
            },
            &receipts,
            &node,
            &EvaluationContext::new(),
        );
        assert!(result.is_satisfied());
    }

    #[test]
    fn test_receipt_from_wrong_node_ignored() {
        let evaluator = GateEvaluator::new();
        let receipts = vec![make_receipt("other_node", ReceiptType::CommitmentFulfilled)];

        let result = evaluator.evaluate(
            &TransitionGate::ReceiptEmitted {
                receipt_type: ReceiptType::CommitmentFulfilled,
            },
            &receipts,
            &NodeId::new("this_node"),
            &EvaluationContext::new(),
        );
        assert!(!result.is_satisfied());
    }
}
