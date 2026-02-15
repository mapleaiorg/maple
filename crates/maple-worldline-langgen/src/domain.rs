//! Domain analysis for language generation.
//!
//! Analyzes observed usage patterns and self-meanings to produce a
//! `DomainSpec` — the foundation for grammar synthesis and type system design.
//!
//! The `SimulatedDomainAnalyzer` generates a financial-settlement domain
//! for deterministic testing.

use crate::error::{LangGenError, LangGenResult};
use crate::types::{
    ConceptProperty, ConceptRelationship, ConstraintType, DomainConcept, DomainConstraint,
    DomainSpec, GrammarStyle, PropertyType, RelationshipType, UsagePattern,
};

// ── Domain Analyzer Trait ────────────────────────────────────────────

/// Trait for analyzing a domain from observed usage patterns.
pub trait DomainAnalyzer: Send + Sync {
    /// Analyze usage patterns and produce a domain specification.
    fn analyze(
        &self,
        patterns: &[UsagePattern],
        domain_hint: Option<&str>,
    ) -> LangGenResult<DomainSpec>;

    /// Name of this analyzer implementation.
    fn name(&self) -> &str;
}

// ── Simulated Domain Analyzer ────────────────────────────────────────

/// Simulated domain analyzer for deterministic testing.
///
/// Generates a financial settlement domain regardless of input patterns.
pub struct SimulatedDomainAnalyzer {
    should_fail: bool,
}

impl SimulatedDomainAnalyzer {
    /// Create a successful analyzer.
    pub fn new() -> Self {
        Self { should_fail: false }
    }

    /// Create an analyzer that always fails.
    pub fn failing() -> Self {
        Self { should_fail: true }
    }

    /// Generate the financial settlement domain spec.
    fn financial_settlement_domain() -> DomainSpec {
        DomainSpec {
            name: "financial-settlement".into(),
            description: "Domain for financial transaction settlement and reconciliation".into(),
            concepts: vec![
                DomainConcept {
                    name: "Account".into(),
                    description: "A financial account holding a balance".into(),
                    properties: vec![
                        ConceptProperty {
                            name: "id".into(),
                            property_type: PropertyType::Text,
                            required: true,
                        },
                        ConceptProperty {
                            name: "balance".into(),
                            property_type: PropertyType::Amount,
                            required: true,
                        },
                        ConceptProperty {
                            name: "currency".into(),
                            property_type: PropertyType::Text,
                            required: true,
                        },
                    ],
                    is_primary: true,
                },
                DomainConcept {
                    name: "Transfer".into(),
                    description: "A transfer of funds between accounts".into(),
                    properties: vec![
                        ConceptProperty {
                            name: "from".into(),
                            property_type: PropertyType::Reference("Account".into()),
                            required: true,
                        },
                        ConceptProperty {
                            name: "to".into(),
                            property_type: PropertyType::Reference("Account".into()),
                            required: true,
                        },
                        ConceptProperty {
                            name: "amount".into(),
                            property_type: PropertyType::Amount,
                            required: true,
                        },
                    ],
                    is_primary: true,
                },
                DomainConcept {
                    name: "Settlement".into(),
                    description: "Settlement of pending transfers".into(),
                    properties: vec![
                        ConceptProperty {
                            name: "transfers".into(),
                            property_type: PropertyType::Reference("Transfer".into()),
                            required: true,
                        },
                        ConceptProperty {
                            name: "settled_at".into(),
                            property_type: PropertyType::Date,
                            required: false,
                        },
                    ],
                    is_primary: false,
                },
            ],
            relationships: vec![
                ConceptRelationship {
                    from: "Transfer".into(),
                    to: "Account".into(),
                    relationship_type: RelationshipType::ManyToMany,
                    description: "Transfers reference source and destination accounts".into(),
                },
                ConceptRelationship {
                    from: "Settlement".into(),
                    to: "Transfer".into(),
                    relationship_type: RelationshipType::OneToMany,
                    description: "A settlement batches multiple transfers".into(),
                },
            ],
            constraints: vec![
                DomainConstraint {
                    name: "non_negative_balance".into(),
                    constraint_type: ConstraintType::NonNegative,
                    applies_to: "Account.balance".into(),
                    description: "Account balance must never be negative".into(),
                },
                DomainConstraint {
                    name: "positive_transfer_amount".into(),
                    constraint_type: ConstraintType::NonNegative,
                    applies_to: "Transfer.amount".into(),
                    description: "Transfer amount must be positive".into(),
                },
                DomainConstraint {
                    name: "same_currency".into(),
                    constraint_type: ConstraintType::Invariant("same_currency".into()),
                    applies_to: "Transfer".into(),
                    description: "Source and destination accounts must share the same currency".into(),
                },
            ],
            recommended_style: GrammarStyle::Declarative,
        }
    }
}

impl Default for SimulatedDomainAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl DomainAnalyzer for SimulatedDomainAnalyzer {
    fn analyze(
        &self,
        patterns: &[UsagePattern],
        _domain_hint: Option<&str>,
    ) -> LangGenResult<DomainSpec> {
        if self.should_fail {
            return Err(LangGenError::DomainAnalysisFailed(
                "simulated failure".into(),
            ));
        }

        if patterns.is_empty() {
            return Err(LangGenError::DomainAnalysisFailed(
                "no usage patterns provided".into(),
            ));
        }

        Ok(Self::financial_settlement_domain())
    }

    fn name(&self) -> &str {
        "simulated-domain-analyzer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_patterns() -> Vec<UsagePattern> {
        vec![
            UsagePattern {
                description: "Transfer between accounts".into(),
                frequency: 0.8,
                concepts: vec!["account".into(), "transfer".into()],
                operations: vec!["debit".into(), "credit".into()],
            },
            UsagePattern {
                description: "Batch settlement".into(),
                frequency: 0.3,
                concepts: vec!["settlement".into()],
                operations: vec!["settle".into()],
            },
        ]
    }

    #[test]
    fn successful_analysis() {
        let analyzer = SimulatedDomainAnalyzer::new();
        let domain = analyzer.analyze(&sample_patterns(), None).unwrap();
        assert_eq!(domain.name, "financial-settlement");
        assert_eq!(domain.concepts.len(), 3);
        assert_eq!(domain.relationships.len(), 2);
        assert_eq!(domain.constraints.len(), 3);
        assert_eq!(domain.recommended_style, GrammarStyle::Declarative);
    }

    #[test]
    fn analysis_with_domain_hint() {
        let analyzer = SimulatedDomainAnalyzer::new();
        let domain = analyzer.analyze(&sample_patterns(), Some("finance")).unwrap();
        assert_eq!(domain.name, "financial-settlement");
    }

    #[test]
    fn analysis_fails_on_empty_patterns() {
        let analyzer = SimulatedDomainAnalyzer::new();
        let result = analyzer.analyze(&[], None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no usage patterns"));
    }

    #[test]
    fn failing_analyzer() {
        let analyzer = SimulatedDomainAnalyzer::failing();
        let result = analyzer.analyze(&sample_patterns(), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("simulated failure"));
    }

    #[test]
    fn domain_has_primary_concepts() {
        let analyzer = SimulatedDomainAnalyzer::new();
        let domain = analyzer.analyze(&sample_patterns(), None).unwrap();
        let primaries: Vec<_> = domain.concepts.iter().filter(|c| c.is_primary).collect();
        assert_eq!(primaries.len(), 2);
    }

    #[test]
    fn account_has_amount_property() {
        let analyzer = SimulatedDomainAnalyzer::new();
        let domain = analyzer.analyze(&sample_patterns(), None).unwrap();
        let account = domain.concepts.iter().find(|c| c.name == "Account").unwrap();
        let balance = account.properties.iter().find(|p| p.name == "balance").unwrap();
        assert_eq!(balance.property_type, PropertyType::Amount);
        assert!(balance.required);
    }

    #[test]
    fn non_negative_balance_constraint() {
        let analyzer = SimulatedDomainAnalyzer::new();
        let domain = analyzer.analyze(&sample_patterns(), None).unwrap();
        let constraint = domain
            .constraints
            .iter()
            .find(|c| c.name == "non_negative_balance")
            .unwrap();
        assert_eq!(constraint.constraint_type, ConstraintType::NonNegative);
    }

    #[test]
    fn analyzer_name() {
        let analyzer = SimulatedDomainAnalyzer::new();
        assert_eq!(analyzer.name(), "simulated-domain-analyzer");
    }
}
