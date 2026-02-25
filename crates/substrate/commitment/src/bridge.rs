//! IntentCommitmentBridge — trait for abstracting the intent→commitment boundary.
//!
//! Follows the `MeaningIntentBridge` pattern from the meaning crate,
//! providing a clean abstraction for querying intent readiness.

use maple_worldline_intent::SelfRegenerationIntent;

use crate::types::CommitmentRecord;

/// Abstraction for the boundary between intent stabilization and commitment.
///
/// Implementations provide access to stabilized intents that have completed
/// their observation period, intents still under observation, and
/// commitments already submitted.
pub trait IntentCommitmentBridge {
    /// Intents that have completed observation and are ready for commitment.
    fn ready_for_commitment(&self) -> Vec<&SelfRegenerationIntent>;

    /// Intents still within their observation window.
    fn pending_observation(&self) -> Vec<&SelfRegenerationIntent>;

    /// Commitments that have been submitted to the gate.
    fn committed(&self) -> Vec<&CommitmentRecord>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify the trait is object-safe (can be used as dyn)
    #[test]
    fn trait_is_object_safe() {
        fn _assert_object_safe(_: &dyn IntentCommitmentBridge) {}
    }

    // Verify Send + Sync are not required (matching MeaningIntentBridge pattern)
    #[test]
    fn trait_compiles() {
        struct MockBridge;

        impl IntentCommitmentBridge for MockBridge {
            fn ready_for_commitment(&self) -> Vec<&SelfRegenerationIntent> {
                vec![]
            }

            fn pending_observation(&self) -> Vec<&SelfRegenerationIntent> {
                vec![]
            }

            fn committed(&self) -> Vec<&CommitmentRecord> {
                vec![]
            }
        }

        let bridge = MockBridge;
        assert!(bridge.ready_for_commitment().is_empty());
        assert!(bridge.pending_observation().is_empty());
        assert!(bridge.committed().is_empty());
    }
}
