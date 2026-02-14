//! Bridge trait connecting the commitment layer to the consequence layer.
//!
//! Follows the same pattern as `MeaningIntentBridge` (Prompt 13→14) and
//! `IntentCommitmentBridge` (Prompt 14→15).

use maple_worldline_commitment::types::CommitmentRecord;

use crate::types::ConsequenceRecord;

/// Bridge between the commitment lifecycle and consequence execution.
///
/// Implemented by the `SelfConsequenceEngine` to provide a clean abstraction
/// for the commitment→consequence handoff.
pub trait CommitmentConsequenceBridge {
    /// Approved commitments ready for consequence execution.
    fn approved_for_execution(&self) -> Vec<&CommitmentRecord>;

    /// Consequences currently pending or executing.
    fn pending_execution(&self) -> Vec<&ConsequenceRecord>;

    /// Consequences that have reached a terminal state.
    fn completed(&self) -> Vec<&ConsequenceRecord>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_worldline_commitment::types::SelfCommitmentId;
    use maple_worldline_intent::types::{IntentId, SubstrateTier};

    /// A mock implementation for testing the trait interface.
    struct MockBridge {
        consequences: Vec<ConsequenceRecord>,
    }

    impl CommitmentConsequenceBridge for MockBridge {
        fn approved_for_execution(&self) -> Vec<&CommitmentRecord> {
            vec![] // No commitment records stored in mock
        }

        fn pending_execution(&self) -> Vec<&ConsequenceRecord> {
            self.consequences
                .iter()
                .filter(|c| !c.status.is_terminal())
                .collect()
        }

        fn completed(&self) -> Vec<&ConsequenceRecord> {
            self.consequences
                .iter()
                .filter(|c| c.status.is_terminal())
                .collect()
        }
    }

    #[test]
    fn mock_bridge_pending_and_completed() {
        let mut pending = ConsequenceRecord::new(
            SelfCommitmentId::new(),
            IntentId::new(),
            SubstrateTier::Tier0,
        );
        pending.mark_executing();

        let mut completed = ConsequenceRecord::new(
            SelfCommitmentId::new(),
            IntentId::new(),
            SubstrateTier::Tier1,
        );
        completed.mark_executing();
        completed.mark_failed("test fail".into(), 0, 1);

        let bridge = MockBridge {
            consequences: vec![pending, completed],
        };

        assert_eq!(bridge.pending_execution().len(), 1);
        assert_eq!(bridge.completed().len(), 1);
        assert_eq!(bridge.approved_for_execution().len(), 0);
    }

    #[test]
    fn mock_bridge_empty() {
        let bridge = MockBridge {
            consequences: vec![],
        };
        assert!(bridge.pending_execution().is_empty());
        assert!(bridge.completed().is_empty());
    }
}
