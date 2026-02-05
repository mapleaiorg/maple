//! Commitment reconciliation with AAS after restore.
//!
//! When a Resonator is restored, its pending commitments need to be
//! reconciled with the Agent Accountability Service (AAS) to determine
//! their current status.

use std::sync::Arc;

use tracing::{debug, info, warn};

use crate::error::Result;
use crate::snapshot::CommitmentSnapshot;

/// Status of a commitment from AAS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitmentStatus {
    /// Commitment is still pending.
    Pending,
    /// Commitment was executed.
    Executed,
    /// Commitment failed.
    Failed,
    /// Commitment expired.
    Expired,
    /// Status is unknown.
    Unknown,
}

impl std::fmt::Display for CommitmentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommitmentStatus::Pending => write!(f, "pending"),
            CommitmentStatus::Executed => write!(f, "executed"),
            CommitmentStatus::Failed => write!(f, "failed"),
            CommitmentStatus::Expired => write!(f, "expired"),
            CommitmentStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Result of commitment reconciliation.
#[derive(Debug, Clone, Default)]
pub struct ReconciliationResult {
    /// Commitments that are still pending.
    pub pending: Vec<CommitmentSnapshot>,

    /// Commitments that were executed during downtime.
    pub executed: Vec<CommitmentSnapshot>,

    /// Commitments that failed during downtime.
    pub failed: Vec<CommitmentSnapshot>,

    /// Commitments that expired during downtime.
    pub expired: Vec<CommitmentSnapshot>,

    /// Commitments with unknown status.
    pub unknown: Vec<CommitmentSnapshot>,
}

impl ReconciliationResult {
    /// Get the total number of reconciled commitments.
    pub fn total(&self) -> usize {
        self.pending.len()
            + self.executed.len()
            + self.failed.len()
            + self.expired.len()
            + self.unknown.len()
    }

    /// Check if there are any commitments requiring attention.
    pub fn requires_attention(&self) -> bool {
        !self.failed.is_empty() || !self.unknown.is_empty()
    }
}

/// Client trait for interacting with AAS.
#[async_trait::async_trait]
pub trait AasClient: Send + Sync {
    /// Get the status of a commitment from AAS.
    async fn get_commitment_status(&self, commitment_id: &str) -> Result<CommitmentStatus>;

    /// Notify AAS that a commitment is being re-assumed after restore.
    async fn notify_restore(&self, commitment_id: &str) -> Result<()>;
}

/// Reconciles pending commitments with AAS after restore.
pub struct CommitmentReconciler {
    /// AAS client.
    aas_client: Arc<dyn AasClient>,
}

impl CommitmentReconciler {
    /// Create a new commitment reconciler.
    pub fn new(aas_client: Arc<dyn AasClient>) -> Self {
        Self { aas_client }
    }

    /// Reconcile pending commitments from snapshot with current AAS state.
    pub async fn reconcile(
        &self,
        commitments: &[CommitmentSnapshot],
    ) -> Result<ReconciliationResult> {
        let mut result = ReconciliationResult::default();

        info!(
            commitment_count = commitments.len(),
            "Starting commitment reconciliation"
        );

        for commitment in commitments {
            match self
                .aas_client
                .get_commitment_status(&commitment.commitment_id)
                .await
            {
                Ok(status) => {
                    match status {
                        CommitmentStatus::Pending => {
                            debug!(
                                commitment_id = %commitment.commitment_id,
                                "Commitment still pending"
                            );

                            // Notify AAS that we're re-assuming this commitment
                            if let Err(e) = self
                                .aas_client
                                .notify_restore(&commitment.commitment_id)
                                .await
                            {
                                warn!(
                                    commitment_id = %commitment.commitment_id,
                                    error = %e,
                                    "Failed to notify AAS of commitment restore"
                                );
                            }

                            result.pending.push(commitment.clone());
                        }
                        CommitmentStatus::Executed => {
                            info!(
                                commitment_id = %commitment.commitment_id,
                                "Commitment was executed during downtime"
                            );
                            result.executed.push(commitment.clone());
                        }
                        CommitmentStatus::Failed => {
                            warn!(
                                commitment_id = %commitment.commitment_id,
                                "Commitment failed during downtime"
                            );
                            result.failed.push(commitment.clone());
                        }
                        CommitmentStatus::Expired => {
                            info!(
                                commitment_id = %commitment.commitment_id,
                                "Commitment expired during downtime"
                            );
                            result.expired.push(commitment.clone());
                        }
                        CommitmentStatus::Unknown => {
                            warn!(
                                commitment_id = %commitment.commitment_id,
                                "Commitment status unknown in AAS"
                            );
                            result.unknown.push(commitment.clone());
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        commitment_id = %commitment.commitment_id,
                        error = %e,
                        "Failed to get commitment status from AAS"
                    );
                    result.unknown.push(commitment.clone());
                }
            }
        }

        info!(
            pending = result.pending.len(),
            executed = result.executed.len(),
            failed = result.failed.len(),
            expired = result.expired.len(),
            unknown = result.unknown.len(),
            "Commitment reconciliation completed"
        );

        Ok(result)
    }
}

/// Mock AAS client for testing.
pub struct MockAasClient {
    /// Predefined statuses for commitments.
    statuses: std::collections::HashMap<String, CommitmentStatus>,

    /// Whether to simulate errors.
    simulate_error: bool,
}

impl MockAasClient {
    /// Create a new mock client where all commitments are pending.
    pub fn all_pending() -> Self {
        Self {
            statuses: std::collections::HashMap::new(),
            simulate_error: false,
        }
    }

    /// Create a mock client with predefined statuses.
    pub fn with_statuses(statuses: std::collections::HashMap<String, CommitmentStatus>) -> Self {
        Self {
            statuses,
            simulate_error: false,
        }
    }

    /// Create a mock client that simulates errors.
    pub fn with_errors() -> Self {
        Self {
            statuses: std::collections::HashMap::new(),
            simulate_error: true,
        }
    }
}

#[async_trait::async_trait]
impl AasClient for MockAasClient {
    async fn get_commitment_status(&self, commitment_id: &str) -> Result<CommitmentStatus> {
        if self.simulate_error {
            return Err(crate::error::StateError::CommitmentReconciliationFailed(
                "simulated error".to_string(),
            ));
        }

        Ok(self
            .statuses
            .get(commitment_id)
            .copied()
            .unwrap_or(CommitmentStatus::Pending))
    }

    async fn notify_restore(&self, _commitment_id: &str) -> Result<()> {
        if self.simulate_error {
            return Err(crate::error::StateError::CommitmentReconciliationFailed(
                "simulated error".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_commitments(count: usize) -> Vec<CommitmentSnapshot> {
        (0..count)
            .map(|i| CommitmentSnapshot {
                commitment_id: format!("commitment-{}", i),
                scope: "test".to_string(),
                status: "pending".to_string(),
                declared_at: Utc::now(),
                parties: vec!["party-a".to_string(), "party-b".to_string()],
            })
            .collect()
    }

    #[tokio::test]
    async fn test_reconcile_all_pending() {
        let client = Arc::new(MockAasClient::all_pending());
        let reconciler = CommitmentReconciler::new(client);

        let commitments = create_test_commitments(5);
        let result = reconciler.reconcile(&commitments).await.unwrap();

        assert_eq!(result.pending.len(), 5);
        assert_eq!(result.executed.len(), 0);
        assert_eq!(result.failed.len(), 0);
        assert_eq!(result.total(), 5);
        assert!(!result.requires_attention());
    }

    #[tokio::test]
    async fn test_reconcile_mixed_statuses() {
        let mut statuses = std::collections::HashMap::new();
        statuses.insert("commitment-0".to_string(), CommitmentStatus::Pending);
        statuses.insert("commitment-1".to_string(), CommitmentStatus::Executed);
        statuses.insert("commitment-2".to_string(), CommitmentStatus::Failed);
        statuses.insert("commitment-3".to_string(), CommitmentStatus::Expired);
        statuses.insert("commitment-4".to_string(), CommitmentStatus::Unknown);

        let client = Arc::new(MockAasClient::with_statuses(statuses));
        let reconciler = CommitmentReconciler::new(client);

        let commitments = create_test_commitments(5);
        let result = reconciler.reconcile(&commitments).await.unwrap();

        assert_eq!(result.pending.len(), 1);
        assert_eq!(result.executed.len(), 1);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.expired.len(), 1);
        assert_eq!(result.unknown.len(), 1);
        assert!(result.requires_attention());
    }

    #[tokio::test]
    async fn test_reconcile_with_errors() {
        let client = Arc::new(MockAasClient::with_errors());
        let reconciler = CommitmentReconciler::new(client);

        let commitments = create_test_commitments(3);
        let result = reconciler.reconcile(&commitments).await.unwrap();

        // All should be unknown due to errors
        assert_eq!(result.unknown.len(), 3);
        assert!(result.requires_attention());
    }
}
