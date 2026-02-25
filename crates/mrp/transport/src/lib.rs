//! MRP transport boundary primitives.
//!
//! Transport adapters are responsible only for delivery mechanics. They must
//! enforce that every consequential leg carries an explicit commitment reference.

#![deny(unsafe_code)]

use rcf_commitment::CommitmentId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A single routed leg delivered by transport.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransportLeg {
    pub leg_id: String,
    pub destination: String,
    pub commitment_ref: Option<CommitmentId>,
    pub simulate_failure: bool,
}

/// Normalized transport outcome per leg.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransportOutcome {
    pub leg_id: String,
    pub destination: String,
    pub commitment_ref: CommitmentId,
    pub delivered_at: chrono::DateTime<chrono::Utc>,
}

/// Trait for transport adapters.
pub trait MrpTransport: Send + Sync {
    fn deliver_leg(&self, leg: &TransportLeg) -> Result<TransportOutcome, TransportError>;
}

/// Deterministic in-memory transport used for tests/dev.
#[derive(Default)]
pub struct InMemoryMrpTransport;

impl MrpTransport for InMemoryMrpTransport {
    fn deliver_leg(&self, leg: &TransportLeg) -> Result<TransportOutcome, TransportError> {
        let commitment_ref = leg
            .commitment_ref
            .clone()
            .ok_or(TransportError::MissingCommitmentReference)?;

        if leg.simulate_failure {
            return Err(TransportError::DeliveryFailed(format!(
                "simulated transport failure for destination '{}'",
                leg.destination
            )));
        }

        Ok(TransportOutcome {
            leg_id: leg.leg_id.clone(),
            destination: leg.destination.clone(),
            commitment_ref,
            delivered_at: chrono::Utc::now(),
        })
    }
}

/// Transport errors.
#[derive(Debug, Error)]
pub enum TransportError {
    #[error("Missing explicit commitment reference")]
    MissingCommitmentReference,

    #[error("Delivery failed: {0}")]
    DeliveryFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_leg_without_commitment_ref() {
        let transport = InMemoryMrpTransport;
        let leg = TransportLeg {
            leg_id: "leg-1".to_string(),
            destination: "aas".to_string(),
            commitment_ref: None,
            simulate_failure: false,
        };

        let err = transport.deliver_leg(&leg).expect_err("must reject");
        assert!(matches!(err, TransportError::MissingCommitmentReference));
    }

    #[test]
    fn delivers_leg_with_commitment_ref() {
        let transport = InMemoryMrpTransport;
        let leg = TransportLeg {
            leg_id: "leg-1".to_string(),
            destination: "aas".to_string(),
            commitment_ref: Some(CommitmentId::new("c-1")),
            simulate_failure: false,
        };

        let outcome = transport.deliver_leg(&leg).expect("must deliver");
        assert_eq!(outcome.commitment_ref.0, "c-1");
    }
}
