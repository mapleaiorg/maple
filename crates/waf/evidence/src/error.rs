use maple_waf_context_graph::ContentHash;

/// Errors from the Evidence system.
#[derive(Debug, thiserror::Error)]
pub enum EvidenceError {
    #[error("test execution failed: {0}")]
    TestFailed(String),
    #[error("invariant violated: {0}")]
    InvariantViolated(String),
    #[error("reproducible build check failed: {0}")]
    ReproBuildFailed(String),
    #[error("evidence bundle not found: {0}")]
    BundleNotFound(ContentHash),
    #[error("evidence bundle tampered: expected {expected}, computed {computed}")]
    BundleTampered {
        expected: ContentHash,
        computed: ContentHash,
    },
    #[error("insufficient evidence: {0}")]
    InsufficientEvidence(String),
    #[error("serialization error: {0}")]
    Serialization(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let e = EvidenceError::TestFailed("timeout".into());
        assert!(format!("{}", e).contains("timeout"));
    }

    #[test]
    fn invariant_violated_display() {
        let e = EvidenceError::InvariantViolated("I.WAF-1".into());
        assert!(format!("{}", e).contains("I.WAF-1"));
    }

    #[test]
    fn bundle_tampered_display() {
        let e = EvidenceError::BundleTampered {
            expected: ContentHash::hash(b"a"),
            computed: ContentHash::hash(b"b"),
        };
        assert!(format!("{}", e).contains("tampered"));
    }
}
