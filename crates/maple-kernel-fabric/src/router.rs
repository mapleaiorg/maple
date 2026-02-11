use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use tracing::{debug, warn};

use crate::error::FabricError;
use crate::event::{KernelEvent, ResonanceStage};
use crate::hlc::HybridLogicalClock;
use crate::types::{SubscriptionId, WorldlineId};

/// A subscription to kernel events with filter criteria.
struct Subscription {
    id: SubscriptionId,
    /// Filter by resonance stage (None = all stages)
    stages: Option<Vec<ResonanceStage>>,
    /// Filter by worldline (None = all worldlines)
    worldlines: Option<Vec<WorldlineId>>,
    /// Channel to send matched events
    sender: mpsc::Sender<KernelEvent>,
}

impl Subscription {
    /// Check if an event matches this subscription's filters.
    fn matches(&self, event: &KernelEvent) -> bool {
        let stage_match = match &self.stages {
            Some(stages) => stages.contains(&event.stage),
            None => true,
        };
        let worldline_match = match &self.worldlines {
            Some(worldlines) => worldlines.contains(&event.worldline_id),
            None => true,
        };
        stage_match && worldline_match
    }
}

/// Routes events to subscribers based on causal dependencies and resonance stage.
pub struct CausalRouter {
    subscriptions: RwLock<Vec<Subscription>>,
    _hlc: Arc<HybridLogicalClock>,
}

impl CausalRouter {
    pub fn new(hlc: Arc<HybridLogicalClock>) -> Self {
        Self {
            subscriptions: RwLock::new(Vec::new()),
            _hlc: hlc,
        }
    }

    /// Subscribe to events matching filter criteria.
    /// Returns a receiver channel and the subscription ID.
    pub async fn subscribe(
        &self,
        stages: Option<Vec<ResonanceStage>>,
        worldlines: Option<Vec<WorldlineId>>,
    ) -> (SubscriptionId, mpsc::Receiver<KernelEvent>) {
        let (sender, receiver) = mpsc::channel(1024);
        let id = SubscriptionId::new();

        let sub = Subscription {
            id: id.clone(),
            stages,
            worldlines,
            sender,
        };

        self.subscriptions.write().await.push(sub);
        debug!(subscription_id = ?id.0, "New subscription registered");

        (id, receiver)
    }

    /// Route an event to all matching subscribers.
    /// Returns the number of subscribers that received the event.
    pub async fn route(&self, event: &KernelEvent) -> Result<usize, FabricError> {
        let subs = self.subscriptions.read().await;
        let mut delivered = 0;
        let mut closed_ids = Vec::new();

        for sub in subs.iter() {
            if sub.matches(event) {
                match sub.sender.try_send(event.clone()) {
                    Ok(()) => delivered += 1,
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        warn!(
                            subscription_id = ?sub.id.0,
                            "Subscriber channel full, dropping event"
                        );
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        closed_ids.push(sub.id.clone());
                    }
                }
            }
        }

        drop(subs);

        // Clean up closed subscriptions
        if !closed_ids.is_empty() {
            let mut subs = self.subscriptions.write().await;
            subs.retain(|s| !closed_ids.contains(&s.id));
            debug!(removed = closed_ids.len(), "Cleaned up closed subscriptions");
        }

        Ok(delivered)
    }

    /// Remove a subscription by ID.
    pub async fn unsubscribe(&self, id: &SubscriptionId) {
        let mut subs = self.subscriptions.write().await;
        subs.retain(|s| s.id != *id);
        debug!(subscription_id = ?id.0, "Subscription removed");
    }

    /// Get the number of active subscriptions.
    pub async fn subscription_count(&self) -> usize {
        self.subscriptions.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventPayload, KernelEvent};
    use crate::hlc::HlcTimestamp;
    use crate::types::{EventId, NodeId};
    use maple_mwl_types::IdentityMaterial;

    fn test_hlc() -> Arc<HybridLogicalClock> {
        Arc::new(HybridLogicalClock::new(NodeId(1)))
    }

    fn test_worldline(seed: u8) -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([seed; 32]))
    }

    fn test_event(stage: ResonanceStage, worldline: WorldlineId) -> KernelEvent {
        KernelEvent::new(
            EventId::new(),
            HlcTimestamp {
                physical: 1000,
                logical: 0,
                node_id: NodeId(1),
            },
            worldline,
            stage,
            EventPayload::MeaningFormed {
                interpretation_count: 1,
                confidence: 0.5,
                ambiguity_preserved: true,
            },
            vec![],
        )
    }

    #[tokio::test]
    async fn subscribe_and_receive() {
        let router = CausalRouter::new(test_hlc());
        let (_id, mut rx) = router.subscribe(None, None).await;

        let event = test_event(ResonanceStage::Meaning, test_worldline(1));
        let delivered = router.route(&event).await.unwrap();
        assert_eq!(delivered, 1);

        let received = rx.recv().await.unwrap();
        assert_eq!(received.id, event.id);
    }

    #[tokio::test]
    async fn stage_filter() {
        let router = CausalRouter::new(test_hlc());
        let (_id, mut rx) = router
            .subscribe(Some(vec![ResonanceStage::Commitment]), None)
            .await;

        // This should NOT match
        let meaning_event = test_event(ResonanceStage::Meaning, test_worldline(1));
        let delivered = router.route(&meaning_event).await.unwrap();
        assert_eq!(delivered, 0);

        // This SHOULD match
        let commitment_event = KernelEvent::new(
            EventId::new(),
            HlcTimestamp { physical: 1001, logical: 0, node_id: NodeId(1) },
            test_worldline(1),
            ResonanceStage::Commitment,
            EventPayload::CommitmentDeclared {
                commitment_id: maple_mwl_types::CommitmentId::new(),
                scope: serde_json::json!({}),
                parties: vec![],
            },
            vec![],
        );
        let delivered = router.route(&commitment_event).await.unwrap();
        assert_eq!(delivered, 1);

        let received = rx.recv().await.unwrap();
        assert_eq!(received.stage, ResonanceStage::Commitment);
    }

    #[tokio::test]
    async fn worldline_filter() {
        let router = CausalRouter::new(test_hlc());
        let wid1 = test_worldline(1);
        let wid2 = test_worldline(2);

        let (_id, mut rx) = router.subscribe(None, Some(vec![wid1.clone()])).await;

        // Event for wid2 should NOT match
        let event2 = test_event(ResonanceStage::Meaning, wid2);
        let delivered = router.route(&event2).await.unwrap();
        assert_eq!(delivered, 0);

        // Event for wid1 SHOULD match
        let event1 = test_event(ResonanceStage::Meaning, wid1);
        let delivered = router.route(&event1).await.unwrap();
        assert_eq!(delivered, 1);

        let received = rx.recv().await.unwrap();
        assert_eq!(received.id, event1.id);
    }

    #[tokio::test]
    async fn multiple_subscribers() {
        let router = CausalRouter::new(test_hlc());

        let (_id1, mut rx1) = router.subscribe(None, None).await;
        let (_id2, mut rx2) = router
            .subscribe(Some(vec![ResonanceStage::Meaning]), None)
            .await;

        let event = test_event(ResonanceStage::Meaning, test_worldline(1));
        let delivered = router.route(&event).await.unwrap();
        assert_eq!(delivered, 2);

        rx1.recv().await.unwrap();
        rx2.recv().await.unwrap();
    }

    #[tokio::test]
    async fn unsubscribe() {
        let router = CausalRouter::new(test_hlc());
        let (id, _rx) = router.subscribe(None, None).await;

        assert_eq!(router.subscription_count().await, 1);
        router.unsubscribe(&id).await;
        assert_eq!(router.subscription_count().await, 0);
    }

    #[tokio::test]
    async fn closed_subscriber_cleaned_up() {
        let router = CausalRouter::new(test_hlc());
        let (_id, rx) = router.subscribe(None, None).await;

        // Drop receiver
        drop(rx);

        let event = test_event(ResonanceStage::Meaning, test_worldline(1));
        let _ = router.route(&event).await.unwrap();

        // Should have cleaned up the closed subscription
        assert_eq!(router.subscription_count().await, 0);
    }
}
