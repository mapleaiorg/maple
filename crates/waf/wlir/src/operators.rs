//! Operator definitions for WLIR modules.

use serde::{Deserialize, Serialize};

/// The body of an operator — how it computes its result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatorBody {
    /// A single S-expression to evaluate.
    Expression(String),
    /// A reference to a native (host-provided) function by name.
    Native(String),
    /// A composite of operator names to execute in sequence.
    Composite(Vec<String>),
}

/// Definition of a single operator within a WLIR module.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorDefinition {
    /// Operator name (must be unique within a module).
    pub name: String,
    /// Parameter names.
    pub params: Vec<String>,
    /// Return type descriptor.
    pub return_type: String,
    /// The operator body.
    pub body: OperatorBody,
}

impl OperatorDefinition {
    /// Create a new operator definition.
    pub fn new(
        name: impl Into<String>,
        params: Vec<String>,
        return_type: impl Into<String>,
        body: OperatorBody,
    ) -> Self {
        Self {
            name: name.into(),
            params,
            return_type: return_type.into(),
            body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_definition_new() {
        let op = OperatorDefinition::new(
            "add",
            vec!["a".into(), "b".into()],
            "i64",
            OperatorBody::Expression("(+ a b)".into()),
        );
        assert_eq!(op.name, "add");
        assert_eq!(op.params.len(), 2);
        assert_eq!(op.return_type, "i64");
        assert!(matches!(op.body, OperatorBody::Expression(_)));
    }

    #[test]
    fn operator_body_variants() {
        let expr = OperatorBody::Expression("(* x x)".into());
        let native = OperatorBody::Native("blake3_hash".into());
        let composite = OperatorBody::Composite(vec!["step1".into(), "step2".into()]);

        // Verify variant discrimination.
        assert!(matches!(expr, OperatorBody::Expression(_)));
        assert!(matches!(native, OperatorBody::Native(_)));
        assert!(matches!(composite, OperatorBody::Composite(ref v) if v.len() == 2));
    }

    #[test]
    fn operator_definition_serde_roundtrip() {
        let op = OperatorDefinition::new(
            "transform",
            vec!["input".into()],
            "String",
            OperatorBody::Native("to_upper".into()),
        );
        let json = serde_json::to_string(&op).unwrap();
        let restored: OperatorDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(op, restored);
    }
}
