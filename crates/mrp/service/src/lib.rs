//! MRP service orchestration.
//!
//! The service coordinates multi-leg routing and records explicit per-leg
//! outcomes, including failures, so execution never fails silently.

#![deny(unsafe_code)]

use mrp_transport::{InMemoryMrpTransport, MrpTransport, TransportLeg};
use rcf_commitment::CommitmentId;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Multi-leg route execution request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiLegRouteRequest {
    pub route_id: String,
    pub legs: Vec<RouteLegRequest>,
}

/// Single leg request within a multi-leg route.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteLegRequest {
    pub leg_id: String,
    pub destination: String,
    pub commitment_ref: Option<CommitmentId>,
    pub simulate_failure: bool,
}

/// Per-leg explicit outcome recorded by MRP service.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegExecutionOutcome {
    pub route_id: String,
    pub leg_id: String,
    pub destination: String,
    pub commitment_ref: Option<CommitmentId>,
    pub status: LegExecutionStatus,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegExecutionStatus {
    Delivered,
    Failed,
}

/// Aggregate route result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteExecutionResult {
    pub route_id: String,
    pub status: RouteExecutionStatus,
    pub leg_outcomes: Vec<LegExecutionOutcome>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteExecutionStatus {
    Settled,
    Failed,
}

/// MRP multi-leg execution service.
pub struct MrpService {
    transport: Arc<dyn MrpTransport>,
    outcome_log: RwLock<Vec<LegExecutionOutcome>>,
}

impl MrpService {
    pub fn new() -> Self {
        Self {
            transport: Arc::new(InMemoryMrpTransport),
            outcome_log: RwLock::new(Vec::new()),
        }
    }

    pub fn with_transport(transport: Arc<dyn MrpTransport>) -> Self {
        Self {
            transport,
            outcome_log: RwLock::new(Vec::new()),
        }
    }

    /// Execute a multi-leg route and persist explicit outcomes for every attempted leg.
    pub fn execute_multi_leg(
        &self,
        request: MultiLegRouteRequest,
    ) -> Result<RouteExecutionResult, MrpServiceError> {
        let started_at = chrono::Utc::now();
        let mut leg_outcomes = Vec::new();
        let mut route_failed = false;

        for leg in request.legs {
            let transport_leg = TransportLeg {
                leg_id: leg.leg_id.clone(),
                destination: leg.destination.clone(),
                commitment_ref: leg.commitment_ref.clone(),
                simulate_failure: leg.simulate_failure,
            };

            match self.transport.deliver_leg(&transport_leg) {
                Ok(delivered) => {
                    leg_outcomes.push(LegExecutionOutcome {
                        route_id: request.route_id.clone(),
                        leg_id: delivered.leg_id,
                        destination: delivered.destination,
                        commitment_ref: Some(delivered.commitment_ref),
                        status: LegExecutionStatus::Delivered,
                        message: "delivered".to_string(),
                        timestamp: delivered.delivered_at,
                    });
                }
                Err(err) => {
                    let failure = LegExecutionOutcome {
                        route_id: request.route_id.clone(),
                        leg_id: transport_leg.leg_id,
                        destination: transport_leg.destination,
                        commitment_ref: transport_leg.commitment_ref,
                        status: LegExecutionStatus::Failed,
                        message: err.to_string(),
                        timestamp: chrono::Utc::now(),
                    };
                    leg_outcomes.push(failure);
                    route_failed = true;
                    break;
                }
            }
        }

        self.append_outcomes(&leg_outcomes)?;

        let result = RouteExecutionResult {
            route_id: request.route_id,
            status: if route_failed {
                RouteExecutionStatus::Failed
            } else {
                RouteExecutionStatus::Settled
            },
            leg_outcomes,
            started_at,
            completed_at: chrono::Utc::now(),
        };

        if route_failed {
            return Err(MrpServiceError::ExecutionFailed(
                "multi-leg route failed; see outcome log for explicit failure".to_string(),
            ));
        }

        Ok(result)
    }

    /// Read all explicit per-leg outcomes.
    pub fn outcome_log(&self) -> Result<Vec<LegExecutionOutcome>, MrpServiceError> {
        let guard = self
            .outcome_log
            .read()
            .map_err(|_| MrpServiceError::LockError)?;
        Ok(guard.clone())
    }

    fn append_outcomes(&self, outcomes: &[LegExecutionOutcome]) -> Result<(), MrpServiceError> {
        let mut guard = self
            .outcome_log
            .write()
            .map_err(|_| MrpServiceError::LockError)?;
        guard.extend_from_slice(outcomes);
        Ok(())
    }
}

impl Default for MrpService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Error)]
pub enum MrpServiceError {
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Lock error")]
    LockError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_leg_failure_logs_explicit_outcome() {
        let service = MrpService::new();
        let request = MultiLegRouteRequest {
            route_id: "route-1".to_string(),
            legs: vec![
                RouteLegRequest {
                    leg_id: "leg-1".to_string(),
                    destination: "aas".to_string(),
                    commitment_ref: Some(CommitmentId::new("c-1")),
                    simulate_failure: false,
                },
                RouteLegRequest {
                    leg_id: "leg-2".to_string(),
                    destination: "eve".to_string(),
                    commitment_ref: Some(CommitmentId::new("c-1")),
                    simulate_failure: true,
                },
            ],
        };

        let err = service.execute_multi_leg(request).expect_err("must fail");
        assert!(matches!(err, MrpServiceError::ExecutionFailed(_)));

        let outcomes = service.outcome_log().expect("must read log");
        assert_eq!(outcomes.len(), 2);
        let failed = outcomes
            .iter()
            .find(|o| o.status == LegExecutionStatus::Failed)
            .expect("failed leg must be logged");
        assert!(failed.message.contains("Delivery failed"));
    }

    #[test]
    fn missing_commitment_is_rejected_and_logged() {
        let service = MrpService::new();
        let request = MultiLegRouteRequest {
            route_id: "route-2".to_string(),
            legs: vec![RouteLegRequest {
                leg_id: "leg-1".to_string(),
                destination: "aas".to_string(),
                commitment_ref: None,
                simulate_failure: false,
            }],
        };

        let err = service.execute_multi_leg(request).expect_err("must reject");
        assert!(matches!(err, MrpServiceError::ExecutionFailed(_)));

        let outcomes = service.outcome_log().expect("must read log");
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].status, LegExecutionStatus::Failed);
        assert!(outcomes[0]
            .message
            .contains("Missing explicit commitment reference"));
    }
}
