//! Type system design for generated languages.
//!
//! Designs a `TypeSystemSpec` from a `DomainSpec`, including types,
//! coercion rules, and constraints. Enforces **financial type safety**:
//! Amount types are currency-aware, Amount → Number coercion is Forbidden,
//! cross-currency operations are Forbidden, and non-negative constraints
//! are enforced at TypeCheck phase.

use crate::error::{LangGenError, LangGenResult};
use crate::types::{
    CoercionRule, CoercionSafety, DomainSpec, DslType, EnforcementPhase, PropertyType,
    TypeConstraint, TypeKind, TypeSystemSpec,
};

// ── Type System Designer Trait ───────────────────────────────────────

/// Trait for designing a type system from a domain specification.
pub trait TypeSystemDesigner: Send + Sync {
    /// Design a type system from a domain specification.
    fn design(&self, domain: &DomainSpec) -> LangGenResult<TypeSystemSpec>;

    /// Name of this designer implementation.
    fn name(&self) -> &str;
}

// ── Simulated Type System Designer ───────────────────────────────────

/// Simulated type system designer for deterministic testing.
///
/// Generates types from domain concepts with full financial safety enforcement:
/// - Amount types are `Financial` kind with currency awareness
/// - Amount → Number coercion is `Forbidden`
/// - Cross-currency operations are `Forbidden`
/// - Non-negative constraints enforced at `TypeCheck`
pub struct SimulatedTypeSystemDesigner {
    should_fail: bool,
}

impl SimulatedTypeSystemDesigner {
    /// Create a successful designer.
    pub fn new() -> Self {
        Self { should_fail: false }
    }

    /// Create a designer that always fails.
    pub fn failing() -> Self {
        Self { should_fail: true }
    }

    /// Map a PropertyType to a DslType.
    fn dsl_type_for_property(prop_type: &PropertyType) -> DslType {
        match prop_type {
            PropertyType::Text => DslType {
                name: "Text".into(),
                kind: TypeKind::Primitive,
                description: "UTF-8 string".into(),
                properties: vec![],
            },
            PropertyType::Integer => DslType {
                name: "Integer".into(),
                kind: TypeKind::Primitive,
                description: "64-bit signed integer".into(),
                properties: vec![],
            },
            PropertyType::Decimal => DslType {
                name: "Decimal".into(),
                kind: TypeKind::Primitive,
                description: "Decimal number".into(),
                properties: vec![],
            },
            PropertyType::Boolean => DslType {
                name: "Boolean".into(),
                kind: TypeKind::Primitive,
                description: "True or false".into(),
                properties: vec![],
            },
            PropertyType::Amount => DslType {
                name: "Amount".into(),
                kind: TypeKind::Financial,
                description: "Currency-aware monetary amount".into(),
                properties: vec![
                    ("currency".into(), "Text".into()),
                    ("value".into(), "Decimal".into()),
                ],
            },
            PropertyType::Date => DslType {
                name: "Date".into(),
                kind: TypeKind::Primitive,
                description: "Calendar date".into(),
                properties: vec![],
            },
            PropertyType::Reference(target) => DslType {
                name: format!("Ref<{}>", target),
                kind: TypeKind::Reference,
                description: format!("Reference to {}", target),
                properties: vec![("target".into(), target.clone())],
            },
        }
    }

    /// Generate financial safety coercion rules.
    fn financial_coercion_rules() -> Vec<CoercionRule> {
        vec![
            CoercionRule {
                from_type: "Integer".into(),
                to_type: "Decimal".into(),
                safety: CoercionSafety::Safe,
                description: "Integer to Decimal is always safe".into(),
            },
            CoercionRule {
                from_type: "Decimal".into(),
                to_type: "Integer".into(),
                safety: CoercionSafety::Lossy,
                description: "Decimal to Integer may lose precision".into(),
            },
            CoercionRule {
                from_type: "Amount".into(),
                to_type: "Decimal".into(),
                safety: CoercionSafety::Forbidden,
                description: "Amount to Number is forbidden — currency metadata would be lost"
                    .into(),
            },
            CoercionRule {
                from_type: "Amount".into(),
                to_type: "Integer".into(),
                safety: CoercionSafety::Forbidden,
                description: "Amount to Integer is forbidden — currency metadata would be lost"
                    .into(),
            },
        ]
    }

    /// Generate type constraints from domain constraints.
    fn constraints_from_domain(domain: &DomainSpec) -> Vec<TypeConstraint> {
        let mut constraints = Vec::new();

        for dc in &domain.constraints {
            let enforcement = match &dc.constraint_type {
                crate::types::ConstraintType::NonNegative => EnforcementPhase::TypeCheck,
                crate::types::ConstraintType::Range => EnforcementPhase::Both,
                crate::types::ConstraintType::Unique => EnforcementPhase::Runtime,
                crate::types::ConstraintType::Invariant(_) => EnforcementPhase::Both,
            };

            constraints.push(TypeConstraint {
                name: dc.name.clone(),
                applies_to: dc.applies_to.clone(),
                enforcement,
                description: dc.description.clone(),
            });
        }

        // Always add cross-currency constraint for financial domains
        let has_amount = domain.concepts.iter().any(|c| {
            c.properties
                .iter()
                .any(|p| p.property_type == PropertyType::Amount)
        });

        if has_amount {
            constraints.push(TypeConstraint {
                name: "cross_currency_forbidden".into(),
                applies_to: "Amount".into(),
                enforcement: EnforcementPhase::TypeCheck,
                description: "Cross-currency arithmetic operations are forbidden".into(),
            });
        }

        constraints
    }
}

impl Default for SimulatedTypeSystemDesigner {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeSystemDesigner for SimulatedTypeSystemDesigner {
    fn design(&self, domain: &DomainSpec) -> LangGenResult<TypeSystemSpec> {
        if self.should_fail {
            return Err(LangGenError::TypeSystemDesignFailed(
                "simulated failure".into(),
            ));
        }

        if domain.concepts.is_empty() {
            return Err(LangGenError::TypeSystemDesignFailed(
                "domain has no concepts".into(),
            ));
        }

        // Collect unique types from domain concept properties
        let mut type_set = std::collections::HashMap::new();
        for concept in &domain.concepts {
            for prop in &concept.properties {
                let dsl_type = Self::dsl_type_for_property(&prop.property_type);
                type_set.entry(dsl_type.name.clone()).or_insert(dsl_type);
            }
        }

        // Add concept types as composite
        for concept in &domain.concepts {
            let fields: Vec<(String, String)> = concept
                .properties
                .iter()
                .map(|p| {
                    let type_name = Self::dsl_type_for_property(&p.property_type).name;
                    (p.name.clone(), type_name)
                })
                .collect();

            type_set.insert(
                concept.name.clone(),
                DslType {
                    name: concept.name.clone(),
                    kind: TypeKind::Composite,
                    description: concept.description.clone(),
                    properties: fields,
                },
            );
        }

        let types: Vec<DslType> = type_set.into_values().collect();
        let coercion_rules = Self::financial_coercion_rules();
        let constraints = Self::constraints_from_domain(domain);

        Ok(TypeSystemSpec {
            types,
            coercion_rules,
            constraints,
        })
    }

    fn name(&self) -> &str {
        "simulated-type-system-designer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DomainAnalyzer, SimulatedDomainAnalyzer};
    use crate::types::UsagePattern;

    fn sample_domain() -> DomainSpec {
        let analyzer = SimulatedDomainAnalyzer::new();
        let patterns = vec![UsagePattern {
            description: "test".into(),
            frequency: 0.5,
            concepts: vec!["account".into()],
            operations: vec!["transfer".into()],
        }];
        analyzer.analyze(&patterns, None).unwrap()
    }

    #[test]
    fn design_type_system() {
        let designer = SimulatedTypeSystemDesigner::new();
        let domain = sample_domain();
        let ts = designer.design(&domain).unwrap();
        assert!(!ts.types.is_empty());
        assert!(!ts.coercion_rules.is_empty());
        assert!(!ts.constraints.is_empty());
    }

    #[test]
    fn amount_type_is_financial() {
        let designer = SimulatedTypeSystemDesigner::new();
        let domain = sample_domain();
        let ts = designer.design(&domain).unwrap();
        let amount_type = ts.types.iter().find(|t| t.name == "Amount").unwrap();
        assert_eq!(amount_type.kind, TypeKind::Financial);
    }

    #[test]
    fn amount_to_number_coercion_forbidden() {
        let designer = SimulatedTypeSystemDesigner::new();
        let domain = sample_domain();
        let ts = designer.design(&domain).unwrap();
        let forbidden: Vec<_> = ts
            .coercion_rules
            .iter()
            .filter(|r| r.from_type == "Amount" && r.safety == CoercionSafety::Forbidden)
            .collect();
        // Amount→Decimal and Amount→Integer are both forbidden
        assert_eq!(forbidden.len(), 2);
    }

    #[test]
    fn cross_currency_constraint_present() {
        let designer = SimulatedTypeSystemDesigner::new();
        let domain = sample_domain();
        let ts = designer.design(&domain).unwrap();
        let cross = ts
            .constraints
            .iter()
            .find(|c| c.name == "cross_currency_forbidden");
        assert!(cross.is_some());
        assert_eq!(cross.unwrap().enforcement, EnforcementPhase::TypeCheck);
    }

    #[test]
    fn non_negative_enforced_at_typecheck() {
        let designer = SimulatedTypeSystemDesigner::new();
        let domain = sample_domain();
        let ts = designer.design(&domain).unwrap();
        let non_neg = ts
            .constraints
            .iter()
            .find(|c| c.name == "non_negative_balance")
            .unwrap();
        assert_eq!(non_neg.enforcement, EnforcementPhase::TypeCheck);
    }

    #[test]
    fn integer_to_decimal_safe() {
        let designer = SimulatedTypeSystemDesigner::new();
        let domain = sample_domain();
        let ts = designer.design(&domain).unwrap();
        let int_to_dec = ts
            .coercion_rules
            .iter()
            .find(|r| r.from_type == "Integer" && r.to_type == "Decimal")
            .unwrap();
        assert_eq!(int_to_dec.safety, CoercionSafety::Safe);
    }

    #[test]
    fn decimal_to_integer_lossy() {
        let designer = SimulatedTypeSystemDesigner::new();
        let domain = sample_domain();
        let ts = designer.design(&domain).unwrap();
        let dec_to_int = ts
            .coercion_rules
            .iter()
            .find(|r| r.from_type == "Decimal" && r.to_type == "Integer")
            .unwrap();
        assert_eq!(dec_to_int.safety, CoercionSafety::Lossy);
    }

    #[test]
    fn concept_types_are_composite() {
        let designer = SimulatedTypeSystemDesigner::new();
        let domain = sample_domain();
        let ts = designer.design(&domain).unwrap();
        let account_type = ts.types.iter().find(|t| t.name == "Account").unwrap();
        assert_eq!(account_type.kind, TypeKind::Composite);
    }

    #[test]
    fn failing_designer() {
        let designer = SimulatedTypeSystemDesigner::failing();
        let domain = sample_domain();
        let result = designer.design(&domain);
        assert!(result.is_err());
    }

    #[test]
    fn empty_domain_fails() {
        let designer = SimulatedTypeSystemDesigner::new();
        let empty = DomainSpec {
            name: "empty".into(),
            description: "".into(),
            concepts: vec![],
            relationships: vec![],
            constraints: vec![],
            recommended_style: crate::types::GrammarStyle::Declarative,
        };
        let result = designer.design(&empty);
        assert!(result.is_err());
    }

    #[test]
    fn designer_name() {
        let designer = SimulatedTypeSystemDesigner::new();
        assert_eq!(designer.name(), "simulated-type-system-designer");
    }
}
