//! Module verification for WLIR factory modules.

use crate::error::WlirError;
use crate::module::WlirFactoryModule;

/// Verifier that checks structural and semantic invariants of a
/// [`WlirFactoryModule`].
pub struct ModuleVerifier;

impl ModuleVerifier {
    /// Verify a WLIR factory module.
    ///
    /// Returns a list of non-fatal warnings on success, or a
    /// [`WlirError::ValidationFailed`] on the first hard error.
    ///
    /// # Checks performed
    ///
    /// 1. Module name must not be empty.
    /// 2. Provenance worldline_id must not be empty.
    /// 3. `max_recursion_depth` must be greater than zero.
    /// 4. No operator may have an empty name.
    pub fn verify(module: &WlirFactoryModule) -> Result<Vec<String>, WlirError> {
        let mut warnings: Vec<String> = Vec::new();

        // 1. Module name must not be empty.
        if module.name.is_empty() {
            return Err(WlirError::ValidationFailed(
                "module name must not be empty".into(),
            ));
        }

        // 2. Provenance worldline_id must not be empty.
        if module.provenance.worldline_id.is_empty() {
            return Err(WlirError::ValidationFailed(
                "provenance worldline_id must not be empty".into(),
            ));
        }

        // 3. max_recursion_depth must be > 0.
        if module.constraints.max_recursion_depth == 0 {
            return Err(WlirError::ValidationFailed(
                "max_recursion_depth must be greater than zero".into(),
            ));
        }

        // 4. No operator may have an empty name.
        for (i, op) in module.operators.iter().enumerate() {
            if op.name.is_empty() {
                return Err(WlirError::ValidationFailed(format!(
                    "operator at index {} has an empty name",
                    i
                )));
            }
        }

        // Non-fatal warnings.
        if module.provenance.content_hash.is_empty() {
            warnings.push("provenance content_hash is empty".into());
        }

        if module.version.is_empty() {
            warnings.push("module version is empty".into());
        }

        if module.operators.is_empty() {
            warnings.push("module has no operators".into());
        }

        if module.constraints.memory_limit_mb == 0 {
            warnings.push("memory_limit_mb is zero; module cannot allocate".into());
        }

        Ok(warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operators::{OperatorBody, OperatorDefinition};
    use crate::types::{AxiomaticConstraints, ProvenanceHeader};
    use maple_waf_context_graph::GovernanceTier;

    fn valid_module() -> WlirFactoryModule {
        WlirFactoryModule::new(
            "test-mod",
            ProvenanceHeader {
                worldline_id: "wl-1".into(),
                content_hash: "deadbeef".into(),
                governance_tier: GovernanceTier::Tier0,
                timestamp_ms: 1_000,
            },
            AxiomaticConstraints::default(),
            "1.0.0",
        )
        .with_operator(OperatorDefinition::new(
            "id",
            vec!["x".into()],
            "any",
            OperatorBody::Expression("x".into()),
        ))
    }

    #[test]
    fn verify_valid_module() {
        let warnings = ModuleVerifier::verify(&valid_module()).unwrap();
        assert!(warnings.is_empty(), "expected no warnings: {:?}", warnings);
    }

    #[test]
    fn verify_empty_name_fails() {
        let mut m = valid_module();
        m.name = String::new();
        let err = ModuleVerifier::verify(&m).unwrap_err();
        assert!(matches!(err, WlirError::ValidationFailed(ref msg) if msg.contains("module name")));
    }

    #[test]
    fn verify_empty_worldline_id_fails() {
        let mut m = valid_module();
        m.provenance.worldline_id = String::new();
        let err = ModuleVerifier::verify(&m).unwrap_err();
        assert!(matches!(err, WlirError::ValidationFailed(ref msg) if msg.contains("worldline_id")));
    }

    #[test]
    fn verify_zero_recursion_depth_fails() {
        let mut m = valid_module();
        m.constraints.max_recursion_depth = 0;
        let err = ModuleVerifier::verify(&m).unwrap_err();
        assert!(
            matches!(err, WlirError::ValidationFailed(ref msg) if msg.contains("max_recursion_depth"))
        );
    }

    #[test]
    fn verify_empty_operator_name_fails() {
        let m = valid_module().with_operator(OperatorDefinition::new(
            "",
            vec![],
            "unit",
            OperatorBody::Expression("()".into()),
        ));
        let err = ModuleVerifier::verify(&m).unwrap_err();
        assert!(
            matches!(err, WlirError::ValidationFailed(ref msg) if msg.contains("empty name"))
        );
    }
}
