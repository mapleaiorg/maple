//! Coupling restoration - gradual re-establishment after restore.
//!
//! After a Resonator is restored from a snapshot, couplings need to be
//! re-established gradually to avoid overwhelming the system and to
//! verify that peers are still present.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::error::Result;
use crate::snapshot::CouplingSnapshot;

/// Handle to cancel coupling restoration.
pub struct CouplingRestorationHandle {
    /// Channel to signal cancellation.
    cancel_tx: mpsc::Sender<()>,

    /// The background task handle.
    handle: JoinHandle<CouplingRestorationResult>,
}

impl CouplingRestorationHandle {
    /// Cancel the restoration process.
    pub async fn cancel(self) -> CouplingRestorationResult {
        let _ = self.cancel_tx.send(()).await;
        self.handle.await.unwrap_or_else(|_| CouplingRestorationResult {
            total_couplings: 0,
            restored: 0,
            failed: 0,
            skipped: 0,
            cancelled: true,
        })
    }

    /// Wait for the restoration to complete.
    pub async fn wait(self) -> CouplingRestorationResult {
        self.handle.await.unwrap_or_else(|_| CouplingRestorationResult {
            total_couplings: 0,
            restored: 0,
            failed: 0,
            skipped: 0,
            cancelled: false,
        })
    }

    /// Check if the restoration is still running.
    pub fn is_running(&self) -> bool {
        !self.handle.is_finished()
    }
}

/// Result of coupling restoration.
#[derive(Debug, Clone)]
pub struct CouplingRestorationResult {
    /// Total number of couplings to restore.
    pub total_couplings: usize,

    /// Number of couplings successfully restored.
    pub restored: usize,

    /// Number of couplings that failed to restore.
    pub failed: usize,

    /// Number of couplings skipped (peer not present).
    pub skipped: usize,

    /// Whether the restoration was cancelled.
    pub cancelled: bool,
}

/// Trait for checking peer presence and establishing couplings.
#[async_trait::async_trait]
pub trait CouplingRuntime: Send + Sync {
    /// Check if a peer is present in the resonance field.
    async fn is_present(&self, peer_id: &crate::snapshot::ResonatorId) -> Result<bool>;

    /// Establish a coupling with a peer.
    async fn establish_coupling(
        &self,
        peer_id: &crate::snapshot::ResonatorId,
        intensity: f64,
        scope: &str,
    ) -> Result<()>;
}

/// Manages gradual coupling restoration.
pub struct CouplingRestorationManager {
    /// Runtime for coupling operations.
    runtime: Arc<dyn CouplingRuntime>,

    /// Batch size for restoration.
    batch_size: usize,

    /// Delay between batches.
    delay_between_batches: Duration,
}

impl CouplingRestorationManager {
    /// Create a new coupling restoration manager.
    pub fn new(
        runtime: Arc<dyn CouplingRuntime>,
        batch_size: usize,
        delay_between_batches: Duration,
    ) -> Self {
        Self {
            runtime,
            batch_size,
            delay_between_batches,
        }
    }

    /// Schedule gradual restoration of couplings.
    ///
    /// Returns a handle that can be used to cancel or wait for completion.
    pub fn schedule_restoration(
        &self,
        instance_id: palm_types::InstanceId,
        mut couplings: Vec<CouplingSnapshot>,
    ) -> CouplingRestorationHandle {
        let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);
        let runtime = self.runtime.clone();
        let batch_size = self.batch_size;
        let delay = self.delay_between_batches;
        let total_couplings = couplings.len();

        // Sort by importance (intensity * recency)
        couplings.sort_by(|a, b| {
            let now = chrono::Utc::now();
            let age_a = (now - a.created_at).num_seconds().max(1) as f64;
            let age_b = (now - b.created_at).num_seconds().max(1) as f64;

            let score_a = a.intensity * (1.0 / age_a);
            let score_b = b.intensity * (1.0 / age_b);

            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let handle = tokio::spawn(async move {
            info!(
                instance_id = %instance_id,
                coupling_count = total_couplings,
                "Starting gradual coupling restoration"
            );

            let mut restored = 0;
            let mut failed = 0;
            let mut skipped = 0;

            for (batch_num, batch) in couplings.chunks(batch_size).enumerate() {
                // Check for cancellation
                if cancel_rx.try_recv().is_ok() {
                    info!(instance_id = %instance_id, "Coupling restoration cancelled");
                    return CouplingRestorationResult {
                        total_couplings,
                        restored,
                        failed,
                        skipped,
                        cancelled: true,
                    };
                }

                for coupling in batch {
                    // Check if peer is present
                    match runtime.is_present(&coupling.peer_id).await {
                        Ok(true) => {
                            // Attempt to re-establish coupling at reduced intensity
                            let initial_intensity = coupling.intensity * 0.5;

                            match runtime
                                .establish_coupling(
                                    &coupling.peer_id,
                                    initial_intensity,
                                    &coupling.scope,
                                )
                                .await
                            {
                                Ok(_) => {
                                    debug!(
                                        instance_id = %instance_id,
                                        peer_id = %coupling.peer_id,
                                        intensity = initial_intensity,
                                        "Coupling restored"
                                    );
                                    restored += 1;
                                }
                                Err(e) => {
                                    warn!(
                                        instance_id = %instance_id,
                                        peer_id = %coupling.peer_id,
                                        error = %e,
                                        "Failed to restore coupling"
                                    );
                                    failed += 1;
                                }
                            }
                        }
                        Ok(false) => {
                            debug!(
                                instance_id = %instance_id,
                                peer_id = %coupling.peer_id,
                                "Peer not present, skipping coupling restoration"
                            );
                            skipped += 1;
                        }
                        Err(e) => {
                            warn!(
                                instance_id = %instance_id,
                                peer_id = %coupling.peer_id,
                                error = %e,
                                "Failed to check peer presence"
                            );
                            failed += 1;
                        }
                    }
                }

                // Delay between batches to avoid overwhelming the system
                let total_batches = (total_couplings + batch_size - 1) / batch_size;
                if batch_num < total_batches - 1 {
                    tokio::time::sleep(delay).await;
                }
            }

            info!(
                instance_id = %instance_id,
                restored = restored,
                failed = failed,
                skipped = skipped,
                "Coupling restoration completed"
            );

            CouplingRestorationResult {
                total_couplings,
                restored,
                failed,
                skipped,
                cancelled: false,
            }
        });

        CouplingRestorationHandle { cancel_tx, handle }
    }
}

/// Mock coupling runtime for testing.
pub struct MockCouplingRuntime {
    /// Peers that are present.
    present_peers: std::collections::HashSet<String>,

    /// Whether to simulate errors.
    simulate_error: bool,
}

impl MockCouplingRuntime {
    /// Create a new mock runtime where all peers are present.
    pub fn all_present() -> Self {
        Self {
            present_peers: std::collections::HashSet::new(),
            simulate_error: false,
        }
    }

    /// Create a mock runtime with specific present peers.
    pub fn with_peers(peers: Vec<String>) -> Self {
        Self {
            present_peers: peers.into_iter().collect(),
            simulate_error: false,
        }
    }

    /// Create a mock runtime that simulates errors.
    pub fn with_errors() -> Self {
        Self {
            present_peers: std::collections::HashSet::new(),
            simulate_error: true,
        }
    }
}

#[async_trait::async_trait]
impl CouplingRuntime for MockCouplingRuntime {
    async fn is_present(&self, peer_id: &crate::snapshot::ResonatorId) -> Result<bool> {
        if self.simulate_error {
            return Err(crate::error::StateError::Runtime("simulated error".to_string()));
        }

        // If present_peers is empty, assume all are present
        if self.present_peers.is_empty() {
            Ok(true)
        } else {
            Ok(self.present_peers.contains(peer_id.as_str()))
        }
    }

    async fn establish_coupling(
        &self,
        _peer_id: &crate::snapshot::ResonatorId,
        _intensity: f64,
        _scope: &str,
    ) -> Result<()> {
        if self.simulate_error {
            return Err(crate::error::StateError::CouplingRestorationFailed(
                "simulated error".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::{CouplingDirection, CouplingSnapshot, ResonatorId};
    use chrono::Utc;

    fn create_test_couplings(count: usize) -> Vec<CouplingSnapshot> {
        (0..count)
            .map(|i| CouplingSnapshot {
                peer_id: ResonatorId::new(format!("peer-{}", i)),
                direction: CouplingDirection::Bidirectional,
                intensity: 0.8 - (i as f64 * 0.1),
                scope: "test".to_string(),
                persistence: Duration::from_secs(3600),
                accumulated_meaning: None,
                created_at: Utc::now() - chrono::Duration::hours(i as i64),
            })
            .collect()
    }

    #[tokio::test]
    async fn test_coupling_restoration() {
        let runtime = Arc::new(MockCouplingRuntime::all_present());
        let manager = CouplingRestorationManager::new(runtime, 2, Duration::from_millis(10));

        let instance_id = palm_types::InstanceId::generate();
        let couplings = create_test_couplings(5);

        let handle = manager.schedule_restoration(instance_id, couplings);
        let result = handle.wait().await;

        assert_eq!(result.total_couplings, 5);
        assert_eq!(result.restored, 5);
        assert_eq!(result.failed, 0);
        assert_eq!(result.skipped, 0);
        assert!(!result.cancelled);
    }

    #[tokio::test]
    async fn test_coupling_restoration_with_missing_peers() {
        let runtime = Arc::new(MockCouplingRuntime::with_peers(vec![
            "peer-0".to_string(),
            "peer-2".to_string(),
        ]));
        let manager = CouplingRestorationManager::new(runtime, 10, Duration::from_millis(10));

        let instance_id = palm_types::InstanceId::generate();
        let couplings = create_test_couplings(5);

        let handle = manager.schedule_restoration(instance_id, couplings);
        let result = handle.wait().await;

        assert_eq!(result.total_couplings, 5);
        assert_eq!(result.restored, 2);
        assert_eq!(result.skipped, 3);
    }

    #[tokio::test]
    async fn test_coupling_restoration_cancellation() {
        let runtime = Arc::new(MockCouplingRuntime::all_present());
        // Use small batch size and longer delay to allow cancellation
        let manager = CouplingRestorationManager::new(runtime, 1, Duration::from_millis(100));

        let instance_id = palm_types::InstanceId::generate();
        let couplings = create_test_couplings(10);

        let handle = manager.schedule_restoration(instance_id, couplings);

        // Cancel after a short delay
        tokio::time::sleep(Duration::from_millis(50)).await;
        let result = handle.cancel().await;

        assert!(result.cancelled || result.restored < 10);
    }
}
