use maple_waf_context_graph::ContentHash;

/// Errors from the Swap Gate.
#[derive(Debug, thiserror::Error)]
pub enum SwapError {
    #[error("evidence insufficient: {0}")]
    EvidenceInsufficient(String),
    #[error("shadow execution failed: {0}")]
    ShadowFailed(String),
    #[error("equivalence check failed at tier {0}: {1}")]
    EquivalenceFailed(String, String),
    #[error("swap aborted: {0}")]
    SwapAborted(String),
    #[error("rollback triggered: {0}")]
    RollbackTriggered(String),
    #[error("governance denied: {0}")]
    GovernanceDenied(String),
    #[error("snapshot not found: {0}")]
    SnapshotNotFound(ContentHash),
    #[error("atomic swap failed: {0}")]
    AtomicSwapFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let e = SwapError::EvidenceInsufficient("3/14 invariants failed".into());
        assert!(format!("{}", e).contains("3/14"));
    }

    #[test]
    fn rollback_display() {
        let e = SwapError::RollbackTriggered("resonance dropped".into());
        assert!(format!("{}", e).contains("resonance"));
    }
}
