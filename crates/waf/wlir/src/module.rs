//! WLIR factory module — the top-level compilation unit.

use serde::{Deserialize, Serialize};

use crate::operators::OperatorDefinition;
use crate::types::{AxiomaticConstraints, ProvenanceHeader};

/// A complete WLIR factory module.
///
/// A module bundles a provenance header, axiomatic constraints, and a
/// sequence of operator definitions into a single deployable unit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WlirFactoryModule {
    /// Module name (unique within a worldline).
    pub name: String,
    /// Provenance tracking.
    pub provenance: ProvenanceHeader,
    /// Axiomatic constraints bounding execution.
    pub constraints: AxiomaticConstraints,
    /// Operators defined in this module.
    pub operators: Vec<OperatorDefinition>,
    /// Semantic version string.
    pub version: String,
}

impl WlirFactoryModule {
    /// Create a new WLIR factory module.
    pub fn new(
        name: impl Into<String>,
        provenance: ProvenanceHeader,
        constraints: AxiomaticConstraints,
        version: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            provenance,
            constraints,
            operators: Vec::new(),
            version: version.into(),
        }
    }

    /// Add an operator to this module (builder pattern).
    pub fn with_operator(mut self, op: OperatorDefinition) -> Self {
        self.operators.push(op);
        self
    }

    /// Return the number of operators in this module.
    pub fn operator_count(&self) -> usize {
        self.operators.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operators::OperatorBody;
    use maple_waf_context_graph::GovernanceTier;

    fn sample_provenance() -> ProvenanceHeader {
        ProvenanceHeader {
            worldline_id: "wl-test".into(),
            content_hash: "hash123".into(),
            governance_tier: GovernanceTier::Tier1,
            timestamp_ms: 1_000_000,
        }
    }

    #[test]
    fn module_new_empty() {
        let m = WlirFactoryModule::new(
            "test-module",
            sample_provenance(),
            AxiomaticConstraints::default(),
            "0.1.0",
        );
        assert_eq!(m.name, "test-module");
        assert_eq!(m.version, "0.1.0");
        assert_eq!(m.operator_count(), 0);
    }

    #[test]
    fn module_with_operator() {
        let op = OperatorDefinition::new(
            "noop",
            vec![],
            "unit",
            OperatorBody::Expression("()".into()),
        );
        let m = WlirFactoryModule::new(
            "ops-module",
            sample_provenance(),
            AxiomaticConstraints::default(),
            "1.0.0",
        )
        .with_operator(op);

        assert_eq!(m.operator_count(), 1);
        assert_eq!(m.operators[0].name, "noop");
    }

    #[test]
    fn module_with_multiple_operators() {
        let op1 = OperatorDefinition::new("a", vec![], "i32", OperatorBody::Native("fa".into()));
        let op2 = OperatorDefinition::new("b", vec![], "i32", OperatorBody::Native("fb".into()));
        let op3 = OperatorDefinition::new("c", vec![], "i32", OperatorBody::Native("fc".into()));

        let m = WlirFactoryModule::new(
            "multi",
            sample_provenance(),
            AxiomaticConstraints::default(),
            "2.0.0",
        )
        .with_operator(op1)
        .with_operator(op2)
        .with_operator(op3);

        assert_eq!(m.operator_count(), 3);
    }

    #[test]
    fn module_serde_roundtrip() {
        let op = OperatorDefinition::new(
            "hash",
            vec!["data".into()],
            "bytes",
            OperatorBody::Native("blake3".into()),
        );
        let m = WlirFactoryModule::new(
            "serde-mod",
            sample_provenance(),
            AxiomaticConstraints::default(),
            "0.2.0",
        )
        .with_operator(op);

        let json = serde_json::to_string(&m).unwrap();
        let restored: WlirFactoryModule = serde_json::from_str(&json).unwrap();
        assert_eq!(m, restored);
    }
}
