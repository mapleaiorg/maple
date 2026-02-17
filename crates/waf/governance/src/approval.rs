//! Approval management: trait definition and simulated implementation.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use maple_waf_context_graph::GovernanceTier;

use crate::error::GovernanceError;
use crate::types::{ApprovalDecision, ApprovalRequest, ApprovalStatus};

/// Trait for requesting and checking governance approvals.
#[async_trait]
pub trait ApprovalManager: Send + Sync {
    /// Submit an approval request and receive the initial status.
    async fn request_approval(
        &self,
        request: ApprovalRequest,
    ) -> Result<ApprovalStatus, GovernanceError>;

    /// Check the current status of a previously submitted approval request.
    async fn check_approval(&self, id: &str) -> Result<ApprovalStatus, GovernanceError>;
}

/// A simulated approval manager for testing and development.
///
/// - **Tier0, Tier1, Tier2**: auto-approved immediately.
/// - **Tier3, Tier4, Tier5**: held as pending until explicitly approved via [`approve`].
pub struct SimulatedApprovalManager {
    statuses: Mutex<HashMap<String, ApprovalStatus>>,
}

impl SimulatedApprovalManager {
    /// Create a new empty simulated approval manager.
    pub fn new() -> Self {
        Self {
            statuses: Mutex::new(HashMap::new()),
        }
    }

    /// Explicitly approve a pending request by id.
    ///
    /// Returns an error if the request does not exist.
    pub fn approve(&self, id: &str) -> Result<(), GovernanceError> {
        let mut map = self.statuses.lock().unwrap();
        let status = map
            .get_mut(id)
            .ok_or_else(|| GovernanceError::Denied(format!("no request found with id: {id}")))?;
        status.decision = Some(ApprovalDecision::Approved);
        status.decided_at_ms = Some(status.request.timestamp_ms + 1);
        Ok(())
    }
}

impl Default for SimulatedApprovalManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ApprovalManager for SimulatedApprovalManager {
    async fn request_approval(
        &self,
        request: ApprovalRequest,
    ) -> Result<ApprovalStatus, GovernanceError> {
        let auto_approve = request.governance_tier <= GovernanceTier::Tier2;

        let status = if auto_approve {
            ApprovalStatus {
                decision: Some(ApprovalDecision::Approved),
                decided_at_ms: Some(request.timestamp_ms),
                request,
            }
        } else {
            ApprovalStatus {
                decision: None,
                decided_at_ms: None,
                request,
            }
        };

        let id = status.request.id.clone();
        self.statuses.lock().unwrap().insert(id, status.clone());
        Ok(status)
    }

    async fn check_approval(&self, id: &str) -> Result<ApprovalStatus, GovernanceError> {
        let map = self.statuses.lock().unwrap();
        map.get(id)
            .cloned()
            .ok_or_else(|| GovernanceError::Denied(format!("no request found with id: {id}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(id: &str, tier: GovernanceTier) -> ApprovalRequest {
        ApprovalRequest {
            id: id.into(),
            governance_tier: tier,
            description: format!("test request {id}"),
            requested_by: "tester".into(),
            timestamp_ms: 1700000000000,
        }
    }

    #[tokio::test]
    async fn tier0_auto_approved() {
        let mgr = SimulatedApprovalManager::new();
        let req = make_request("r-0", GovernanceTier::Tier0);
        let status = mgr.request_approval(req).await.unwrap();
        assert!(status.is_approved());
    }

    #[tokio::test]
    async fn tier2_auto_approved() {
        let mgr = SimulatedApprovalManager::new();
        let req = make_request("r-2", GovernanceTier::Tier2);
        let status = mgr.request_approval(req).await.unwrap();
        assert!(status.is_approved());
    }

    #[tokio::test]
    async fn tier3_held_pending() {
        let mgr = SimulatedApprovalManager::new();
        let req = make_request("r-3", GovernanceTier::Tier3);
        let status = mgr.request_approval(req).await.unwrap();
        assert!(!status.is_decided());
        assert!(!status.is_approved());
    }

    #[tokio::test]
    async fn tier4_held_pending() {
        let mgr = SimulatedApprovalManager::new();
        let req = make_request("r-4", GovernanceTier::Tier4);
        let status = mgr.request_approval(req).await.unwrap();
        assert!(!status.is_decided());
    }

    #[tokio::test]
    async fn explicit_approve_promotes_pending() {
        let mgr = SimulatedApprovalManager::new();
        let req = make_request("r-3", GovernanceTier::Tier3);
        let status = mgr.request_approval(req).await.unwrap();
        assert!(!status.is_approved());

        mgr.approve("r-3").unwrap();

        let updated = mgr.check_approval("r-3").await.unwrap();
        assert!(updated.is_approved());
        assert!(updated.decided_at_ms.is_some());
    }

    #[tokio::test]
    async fn check_nonexistent_returns_error() {
        let mgr = SimulatedApprovalManager::new();
        let result = mgr.check_approval("does-not-exist").await;
        assert!(result.is_err());
    }
}
