//! Intent validator — ensures intents meet quality and safety requirements.
//!
//! Validates that intents have sufficient confidence, acceptable risk,
//! rollback plans, safety checks, and appropriate governance classification
//! before they can proceed to the commitment boundary.

use crate::intent::SelfRegenerationIntent;
use crate::types::{IntentConfig, SubstrateTier};

// ── Issue Severity ─────────────────────────────────────────────────────

/// Severity level of a validation issue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IssueSeverity {
    /// Blocks the intent from proceeding.
    Error,
    /// Advisory — intent can still proceed.
    Warning,
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "ERROR"),
            Self::Warning => write!(f, "WARNING"),
        }
    }
}

// ── Validation Issue ───────────────────────────────────────────────────

/// A specific issue found during validation.
#[derive(Clone, Debug)]
pub struct ValidationIssue {
    /// Machine-readable code (e.g., "LOW_CONFIDENCE").
    pub code: String,
    /// Human-readable description.
    pub description: String,
    /// Severity level.
    pub severity: IssueSeverity,
}

impl std::fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.severity, self.code, self.description)
    }
}

// ── Validation Result ──────────────────────────────────────────────────

/// Result of validating an intent.
#[derive(Clone, Debug)]
pub enum IntentValidationResult {
    /// Intent passed all validation checks.
    Valid,
    /// Intent has issues (errors block, warnings are advisory).
    Invalid { issues: Vec<ValidationIssue> },
}

impl IntentValidationResult {
    /// Whether validation passed (no errors, warnings are OK).
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Valid => true,
            Self::Invalid { issues } => !issues.iter().any(|i| i.severity == IssueSeverity::Error),
        }
    }

    /// Get all issues (empty if Valid).
    pub fn issues(&self) -> &[ValidationIssue] {
        match self {
            Self::Valid => &[],
            Self::Invalid { issues } => issues,
        }
    }
}

// ── Intent Validator ───────────────────────────────────────────────────

/// Validates intents against quality and safety requirements.
pub struct IntentValidator {
    /// Minimum confidence for an intent to pass validation.
    pub min_confidence: f64,
    /// Maximum acceptable risk score.
    pub max_risk: f64,
    /// Minimum number of evidence items (from source meanings).
    pub min_evidence_count: usize,
    /// Whether a rollback plan is mandatory.
    pub rollback_required: bool,
}

impl Default for IntentValidator {
    fn default() -> Self {
        Self {
            min_confidence: 0.8,
            max_risk: 0.3,
            min_evidence_count: 5,
            rollback_required: true,
        }
    }
}

impl IntentValidator {
    /// Create a validator from an intent configuration.
    pub fn from_config(config: &IntentConfig) -> Self {
        Self {
            min_confidence: config.min_confidence,
            max_risk: config.max_risk,
            min_evidence_count: config.min_evidence_count,
            rollback_required: config.rollback_required,
        }
    }

    /// Validate a single intent.
    pub fn validate(&self, intent: &SelfRegenerationIntent) -> IntentValidationResult {
        let mut issues = Vec::new();

        // Check 1: Confidence threshold
        if intent.confidence < self.min_confidence {
            issues.push(ValidationIssue {
                code: "LOW_CONFIDENCE".into(),
                description: format!(
                    "Confidence {:.2} is below minimum {:.2}",
                    intent.confidence, self.min_confidence
                ),
                severity: IssueSeverity::Error,
            });
        }

        // Check 2: Risk threshold
        if intent.impact.risk_score > self.max_risk {
            issues.push(ValidationIssue {
                code: "HIGH_RISK".into(),
                description: format!(
                    "Risk score {:.2} exceeds maximum {:.2}",
                    intent.impact.risk_score, self.max_risk
                ),
                severity: IssueSeverity::Error,
            });
        }

        // Check 3: Rollback plan
        if self.rollback_required && !intent.proposal.has_rollback() {
            issues.push(ValidationIssue {
                code: "NO_ROLLBACK".into(),
                description: "Proposal has no rollback plan".into(),
                severity: IssueSeverity::Error,
            });
        }

        // Check 4: Safety checks for high-tier changes
        if intent.governance_tier >= SubstrateTier::Tier2 && !intent.proposal.has_safety_checks() {
            issues.push(ValidationIssue {
                code: "MISSING_SAFETY_CHECKS".into(),
                description: format!(
                    "Tier {} changes require safety checks",
                    intent.governance_tier
                ),
                severity: IssueSeverity::Error,
            });
        }

        // Check 5: Governance tier matches confidence
        let tier_min_confidence = intent.governance_tier.min_confidence();
        if intent.confidence < tier_min_confidence {
            issues.push(ValidationIssue {
                code: "TIER_CONFIDENCE_MISMATCH".into(),
                description: format!(
                    "Governance tier {} requires confidence >= {:.2}, got {:.2}",
                    intent.governance_tier, tier_min_confidence, intent.confidence
                ),
                severity: IssueSeverity::Warning,
            });
        }

        // Check 6: At least one code change or it's a config-only change
        if intent.proposal.code_changes.is_empty()
            && !matches!(
                intent.change_type,
                crate::types::ChangeType::ConfigurationChange { .. }
            )
        {
            issues.push(ValidationIssue {
                code: "NO_CODE_CHANGES".into(),
                description: "Non-config proposal has no code changes".into(),
                severity: IssueSeverity::Warning,
            });
        }

        if issues.is_empty() {
            IntentValidationResult::Valid
        } else {
            IntentValidationResult::Invalid { issues }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{ImpactAssessment, ImprovementEstimate, IntentStatus};
    use crate::proposal::{RegenerationProposal, RollbackPlan, RollbackStrategy, SafetyCheck};
    use crate::types::{ChangeType, IntentId, ProposalId, ReversibilityLevel, SubstrateTier};
    use chrono::Utc;
    use maple_worldline_meaning::MeaningId;

    fn make_valid_intent() -> SelfRegenerationIntent {
        SelfRegenerationIntent {
            id: IntentId::new(),
            derived_from: vec![MeaningId::new()],
            change_type: ChangeType::ConfigurationChange {
                parameter: "batch_size".into(),
                current_value: "32".into(),
                proposed_value: "64".into(),
                rationale: "improve throughput".into(),
            },
            proposal: RegenerationProposal {
                id: ProposalId::new(),
                summary: "Increase batch size".into(),
                rationale: "Suboptimal batch size".into(),
                affected_components: vec!["scheduler".into()],
                code_changes: vec![],
                required_tests: vec![],
                performance_gates: vec![],
                safety_checks: vec![],
                estimated_improvement: ImprovementEstimate {
                    metric: "throughput".into(),
                    current_value: 100.0,
                    projected_value: 150.0,
                    confidence: 0.8,
                    unit: "ops/s".into(),
                },
                risk_score: 0.2,
                rollback_plan: RollbackPlan {
                    strategy: RollbackStrategy::ConfigRestore,
                    steps: vec!["restore config".into()],
                    estimated_duration_secs: 60,
                },
            },
            confidence: 0.9,
            reversibility: ReversibilityLevel::FullyReversible,
            impact: ImpactAssessment {
                affected_components: vec!["scheduler".into()],
                risk_score: 0.2,
                risk_factors: vec![],
                blast_radius: "scheduler only".into(),
            },
            governance_tier: SubstrateTier::Tier0,
            estimated_improvement: ImprovementEstimate {
                metric: "throughput".into(),
                current_value: 100.0,
                projected_value: 150.0,
                confidence: 0.8,
                unit: "ops/s".into(),
            },
            stabilized_at: Utc::now(),
            status: IntentStatus::Stabilized,
        }
    }

    #[test]
    fn valid_intent_passes() {
        let validator = IntentValidator::default();
        let intent = make_valid_intent();
        let result = validator.validate(&intent);
        assert!(result.is_valid());
    }

    #[test]
    fn low_confidence_rejected() {
        let validator = IntentValidator::default();
        let mut intent = make_valid_intent();
        intent.confidence = 0.5;
        let result = validator.validate(&intent);
        assert!(!result.is_valid());
        assert!(result.issues().iter().any(|i| i.code == "LOW_CONFIDENCE"));
    }

    #[test]
    fn high_risk_rejected() {
        let validator = IntentValidator::default();
        let mut intent = make_valid_intent();
        intent.impact.risk_score = 0.9;
        let result = validator.validate(&intent);
        assert!(!result.is_valid());
        assert!(result.issues().iter().any(|i| i.code == "HIGH_RISK"));
    }

    #[test]
    fn missing_rollback_rejected() {
        let validator = IntentValidator::default();
        let mut intent = make_valid_intent();
        intent.proposal.rollback_plan.steps.clear();
        let result = validator.validate(&intent);
        assert!(!result.is_valid());
        assert!(result.issues().iter().any(|i| i.code == "NO_ROLLBACK"));
    }

    #[test]
    fn tier2_requires_safety_checks() {
        let validator = IntentValidator::default();
        let mut intent = make_valid_intent();
        intent.governance_tier = SubstrateTier::Tier2;
        intent.proposal.safety_checks.clear();
        let result = validator.validate(&intent);
        assert!(!result.is_valid());
        assert!(result
            .issues()
            .iter()
            .any(|i| i.code == "MISSING_SAFETY_CHECKS"));

        // Add safety checks → should pass
        intent.proposal.safety_checks.push(SafetyCheck {
            invariant: "test".into(),
            description: "test".into(),
        });
        let result = validator.validate(&intent);
        assert!(result.is_valid());
    }

    #[test]
    fn from_config_applies_settings() {
        let config = IntentConfig {
            min_confidence: 0.95,
            max_risk: 0.1,
            min_evidence_count: 20,
            rollback_required: false,
            ..IntentConfig::default()
        };
        let validator = IntentValidator::from_config(&config);
        assert!((validator.min_confidence - 0.95).abs() < f64::EPSILON);
        assert!((validator.max_risk - 0.1).abs() < f64::EPSILON);
        assert!(!validator.rollback_required);
    }
}
