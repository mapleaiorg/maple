use std::sync::Arc;

use async_trait::async_trait;
use maple_mwl_types::{ResonanceType, TemporalAnchor, WorldlineId};
use tracing::{debug, info, warn};

use crate::envelope::MrpEnvelope;
use crate::error::{EscalationViolation, IntegrityError, MrpError, TypeMismatchError};
use crate::routing::{EscalationRecord, RejectionReason, RouteDecision};

/// Trait for the execution layer — so MRP can verify consequence origin.
///
/// Consequence envelopes may only be accepted from registered execution layers.
#[async_trait]
pub trait ExecutionLayer: Send + Sync {
    /// Check if a WorldLine is a registered execution layer origin.
    fn is_execution_origin(&self, wid: &WorldlineId) -> bool;
}

/// MRP Router — enforces resonance-type routing constraints.
///
/// Per Whitepaper §4.1: "MRP is a constitutional protocol. Where traditional
/// protocols optimize for throughput, MRP optimizes for preserving the
/// Commitment Boundary under all conditions."
///
/// Routing rules:
/// - MEANING: freely routable within cognition, NEVER reaches execution
/// - INTENT: routable for negotiation, NON-EXECUTABLE
/// - COMMITMENT: MUST route through Gate, immutable once declared
/// - CONSEQUENCE: emitted ONLY by execution layer
pub struct MrpRouter {
    execution_layer: Option<Arc<dyn ExecutionLayer>>,
    escalation_log: Vec<EscalationRecord>,
}

impl MrpRouter {
    /// Create a new MRP Router.
    pub fn new() -> Self {
        Self {
            execution_layer: None,
            escalation_log: Vec::new(),
        }
    }

    /// Create a router with an execution layer for consequence validation.
    pub fn with_execution_layer(execution_layer: Arc<dyn ExecutionLayer>) -> Self {
        Self {
            execution_layer: Some(execution_layer),
            escalation_log: Vec::new(),
        }
    }

    /// Route an envelope. This is the main entry point.
    ///
    /// Performs all validation checks and returns a routing decision:
    /// 1. TTL check
    /// 2. Integrity verification
    /// 3. Type consistency (header matches payload)
    /// 4. Type-based routing
    pub async fn route(&mut self, envelope: &MrpEnvelope) -> Result<RouteDecision, MrpError> {
        // 1. TTL check
        if envelope.is_expired() {
            debug!(
                envelope_id = %envelope.header.envelope_id,
                "Envelope expired"
            );
            return Ok(RouteDecision::Expired);
        }

        // 2. Integrity verification
        if !envelope.verify_integrity() {
            warn!(
                envelope_id = %envelope.header.envelope_id,
                "Integrity verification failed — possible tampering"
            );
            return Ok(RouteDecision::Quarantine(
                "Integrity verification failed".into(),
            ));
        }

        // 3. Type consistency
        if !envelope.is_type_consistent() {
            let declared = envelope.header.resonance_type;
            let actual = envelope.body.resonance_type();
            warn!(
                envelope_id = %envelope.header.envelope_id,
                declared = ?declared,
                actual = ?actual,
                "Type mismatch detected"
            );
            return Ok(RouteDecision::Reject(RejectionReason::TypeMismatch {
                declared: format!("{:?}", declared),
                actual: format!("{:?}", actual),
            }));
        }

        // 4. Type-based routing
        match envelope.header.resonance_type {
            ResonanceType::Meaning => {
                debug!(
                    envelope_id = %envelope.header.envelope_id,
                    "Routing MEANING to cognition"
                );
                let destinations = self.resolve_cognition_destinations(envelope);
                Ok(RouteDecision::DeliverToCognition(destinations))
            }
            ResonanceType::Intent => {
                debug!(
                    envelope_id = %envelope.header.envelope_id,
                    "Routing INTENT to cognition"
                );
                let destinations = self.resolve_cognition_destinations(envelope);
                Ok(RouteDecision::DeliverToCognition(destinations))
            }
            ResonanceType::Commitment => {
                info!(
                    envelope_id = %envelope.header.envelope_id,
                    "Routing COMMITMENT to Gate"
                );
                Ok(RouteDecision::RouteToGate)
            }
            ResonanceType::Consequence => {
                // Verify consequence comes from execution layer
                if let Some(ref exec_layer) = self.execution_layer {
                    if !exec_layer.is_execution_origin(&envelope.header.origin) {
                        warn!(
                            envelope_id = %envelope.header.envelope_id,
                            origin = %envelope.header.origin,
                            "CONSEQUENCE envelope not from execution layer"
                        );
                        return Ok(RouteDecision::Reject(
                            RejectionReason::InvalidConsequenceOrigin,
                        ));
                    }
                }
                debug!(
                    envelope_id = %envelope.header.envelope_id,
                    "Routing CONSEQUENCE to observers"
                );
                Ok(RouteDecision::DeliverAsConsequence(
                    envelope.header.origin.clone(),
                ))
            }
        }
    }

    /// Validate non-escalation: the core invariant I.MRP-1.
    ///
    /// No envelope may be transformed into a higher-resonance type
    /// than the one it declares. MEANING cannot become INTENT implicitly.
    ///
    /// Returns Ok(()) if the transition is valid (same type or explicit
    /// forward transition via proper channels), Err otherwise.
    pub fn validate_non_escalation(
        &mut self,
        from: &ResonanceType,
        to: &ResonanceType,
        origin: &WorldlineId,
        envelope_id: uuid::Uuid,
    ) -> Result<(), EscalationViolation> {
        // Same type is always ok
        if from == to {
            return Ok(());
        }

        // Any implicit escalation is a violation
        // The ONLY valid type change is through explicit mechanisms:
        // - Meaning → Intent: only via Intent stabilization event
        // - Intent → Commitment: only via Commitment Gate submission
        // - Commitment → Consequence: only via execution layer observation
        //
        // Implicit promotion (changing the type of an envelope) is ALWAYS forbidden.
        let violation = EscalationViolation {
            from: *from,
            to: *to,
            message: format!(
                "I.MRP-1 violation: implicit escalation from {:?} to {:?}",
                from, to
            ),
        };

        // Record the violation for accountability
        self.escalation_log.push(EscalationRecord {
            envelope_id,
            origin: origin.clone(),
            declared_type: format!("{:?}", from),
            attempted_type: format!("{:?}", to),
            timestamp: TemporalAnchor::now(0),
        });

        warn!(
            from = ?from,
            to = ?to,
            origin = %origin,
            "Non-escalation violation detected and logged"
        );

        Err(violation)
    }

    /// Validate envelope integrity.
    pub fn validate_integrity(&self, envelope: &MrpEnvelope) -> Result<(), IntegrityError> {
        let expected = envelope.compute_hash();
        if envelope.integrity.hash != expected {
            return Err(IntegrityError {
                expected_hash: expected,
                actual_hash: envelope.integrity.hash,
                message: "Envelope hash does not match computed hash".into(),
            });
        }
        Ok(())
    }

    /// Validate that the payload type matches the declared resonance type.
    pub fn validate_type_consistency(
        &self,
        envelope: &MrpEnvelope,
    ) -> Result<(), TypeMismatchError> {
        if !envelope.is_type_consistent() {
            return Err(TypeMismatchError {
                declared: envelope.header.resonance_type,
                actual: envelope.body.resonance_type(),
            });
        }
        Ok(())
    }

    /// Get the escalation violation log.
    pub fn escalation_log(&self) -> &[EscalationRecord] {
        &self.escalation_log
    }

    /// Resolve cognition-layer destinations based on routing constraints.
    fn resolve_cognition_destinations(&self, envelope: &MrpEnvelope) -> Vec<WorldlineId> {
        if envelope
            .header
            .routing_constraints
            .required_destinations
            .is_empty()
        {
            // Default: return origin as sole destination
            vec![envelope.header.origin.clone()]
        } else {
            envelope
                .header
                .routing_constraints
                .required_destinations
                .clone()
        }
    }
}

impl Default for MrpRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock execution layer for testing.
pub struct MockExecutionLayer {
    registered_origins: Vec<WorldlineId>,
}

impl MockExecutionLayer {
    pub fn new() -> Self {
        Self {
            registered_origins: Vec::new(),
        }
    }

    pub fn register(mut self, origin: WorldlineId) -> Self {
        self.registered_origins.push(origin);
        self
    }
}

impl Default for MockExecutionLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExecutionLayer for MockExecutionLayer {
    fn is_execution_origin(&self, wid: &WorldlineId) -> bool {
        self.registered_origins.contains(wid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::*;
    use crate::payloads::*;
    use maple_mwl_types::{
        CommitmentId, CommitmentScope, ConfidenceProfile, EffectDomain, EventId, IdentityMaterial,
    };

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn other_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32]))
    }

    fn meaning_envelope(origin: WorldlineId) -> MrpEnvelope {
        MeaningEnvelopeBuilder::new(origin)
            .payload(MeaningPayload {
                interpretation: "test".into(),
                confidence: 0.9,
                ambiguity_preserved: true,
                evidence_refs: vec![EventId::new()],
            })
            .build()
            .unwrap()
    }

    fn intent_envelope(origin: WorldlineId) -> MrpEnvelope {
        IntentEnvelopeBuilder::new(origin)
            .payload(IntentPayload {
                direction: "send message".into(),
                confidence: ConfidenceProfile::new(0.8, 0.8, 0.8, 0.8),
                conditions: vec![],
                derived_from: None,
            })
            .build()
            .unwrap()
    }

    fn commitment_envelope(origin: WorldlineId) -> MrpEnvelope {
        CommitmentEnvelopeBuilder::new(origin.clone())
            .payload(CommitmentPayload {
                commitment_id: CommitmentId::new(),
                scope: CommitmentScope {
                    effect_domain: EffectDomain::Communication,
                    targets: vec![other_worldline()],
                    constraints: vec![],
                },
                affected_parties: vec![],
                evidence: vec![],
            })
            .build()
            .unwrap()
    }

    fn consequence_envelope(origin: WorldlineId) -> MrpEnvelope {
        ConsequenceEnvelopeBuilder::new(origin.clone())
            .payload(ConsequencePayload {
                commitment_id: CommitmentId::new(),
                outcome_description: "done".into(),
                state_changes: serde_json::json!({}),
                observed_by: origin,
            })
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn meaning_routes_to_cognition() {
        let mut router = MrpRouter::new();
        let env = meaning_envelope(test_worldline());
        let decision = router.route(&env).await.unwrap();
        assert!(matches!(decision, RouteDecision::DeliverToCognition(_)));
    }

    #[tokio::test]
    async fn intent_routes_to_cognition() {
        let mut router = MrpRouter::new();
        let env = intent_envelope(test_worldline());
        let decision = router.route(&env).await.unwrap();
        assert!(matches!(decision, RouteDecision::DeliverToCognition(_)));
    }

    #[tokio::test]
    async fn commitment_routes_to_gate() {
        let mut router = MrpRouter::new();
        let env = commitment_envelope(test_worldline());
        let decision = router.route(&env).await.unwrap();
        assert!(matches!(decision, RouteDecision::RouteToGate));
    }

    #[tokio::test]
    async fn consequence_routes_to_observer() {
        let mut router = MrpRouter::new();
        let env = consequence_envelope(test_worldline());
        let decision = router.route(&env).await.unwrap();
        assert!(matches!(decision, RouteDecision::DeliverAsConsequence(_)));
    }

    #[tokio::test]
    async fn consequence_rejected_from_non_execution_layer() {
        let wid = test_worldline();
        let exec_layer = MockExecutionLayer::new(); // no registered origins
        let mut router = MrpRouter::with_execution_layer(Arc::new(exec_layer));

        let env = consequence_envelope(wid);
        let decision = router.route(&env).await.unwrap();
        assert!(matches!(
            decision,
            RouteDecision::Reject(RejectionReason::InvalidConsequenceOrigin)
        ));
    }

    #[tokio::test]
    async fn consequence_accepted_from_execution_layer() {
        let wid = test_worldline();
        let exec_layer = MockExecutionLayer::new().register(wid.clone());
        let mut router = MrpRouter::with_execution_layer(Arc::new(exec_layer));

        let env = consequence_envelope(wid);
        let decision = router.route(&env).await.unwrap();
        assert!(matches!(decision, RouteDecision::DeliverAsConsequence(_)));
    }

    #[tokio::test]
    async fn tampered_envelope_quarantined() {
        let mut router = MrpRouter::new();
        let mut env = meaning_envelope(test_worldline());

        // Tamper with the body
        env.body = crate::envelope::TypedPayload::Meaning(MeaningPayload {
            interpretation: "TAMPERED".into(),
            confidence: 0.1,
            ambiguity_preserved: false,
            evidence_refs: vec![],
        });

        let decision = router.route(&env).await.unwrap();
        assert!(matches!(decision, RouteDecision::Quarantine(_)));
    }

    #[tokio::test]
    async fn type_mismatch_rejected() {
        let mut router = MrpRouter::new();
        let mut env = meaning_envelope(test_worldline());

        // Change header type without changing payload
        env.header.resonance_type = ResonanceType::Commitment;
        // Recompute hash to pass integrity check
        env.integrity.hash = env.compute_hash();

        let decision = router.route(&env).await.unwrap();
        assert!(matches!(
            decision,
            RouteDecision::Reject(RejectionReason::TypeMismatch { .. })
        ));
    }

    #[tokio::test]
    async fn expired_envelope_detected() {
        let mut router = MrpRouter::new();
        let mut env = meaning_envelope(test_worldline());
        env.header.timestamp = TemporalAnchor::new(1000, 0, 0);
        env.header.ttl_ms = 1;
        // Recompute hash
        env.integrity.hash = env.compute_hash();

        let decision = router.route(&env).await.unwrap();
        assert!(matches!(decision, RouteDecision::Expired));
    }

    #[test]
    fn non_escalation_same_type_ok() {
        let mut router = MrpRouter::new();
        let result = router.validate_non_escalation(
            &ResonanceType::Meaning,
            &ResonanceType::Meaning,
            &test_worldline(),
            uuid::Uuid::new_v4(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn non_escalation_implicit_promotion_rejected() {
        let mut router = MrpRouter::new();

        // Meaning → Intent: implicit escalation
        let result = router.validate_non_escalation(
            &ResonanceType::Meaning,
            &ResonanceType::Intent,
            &test_worldline(),
            uuid::Uuid::new_v4(),
        );
        assert!(result.is_err());

        // Meaning → Commitment: implicit escalation
        let result = router.validate_non_escalation(
            &ResonanceType::Meaning,
            &ResonanceType::Commitment,
            &test_worldline(),
            uuid::Uuid::new_v4(),
        );
        assert!(result.is_err());

        // Intent → Commitment: implicit escalation
        let result = router.validate_non_escalation(
            &ResonanceType::Intent,
            &ResonanceType::Commitment,
            &test_worldline(),
            uuid::Uuid::new_v4(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn non_escalation_demotion_rejected() {
        let mut router = MrpRouter::new();

        // Commitment → Meaning: also a violation (type rewriting)
        let result = router.validate_non_escalation(
            &ResonanceType::Commitment,
            &ResonanceType::Meaning,
            &test_worldline(),
            uuid::Uuid::new_v4(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn escalation_violations_logged() {
        let mut router = MrpRouter::new();
        assert!(router.escalation_log().is_empty());

        let _ = router.validate_non_escalation(
            &ResonanceType::Meaning,
            &ResonanceType::Commitment,
            &test_worldline(),
            uuid::Uuid::new_v4(),
        );

        assert_eq!(router.escalation_log().len(), 1);
        assert_eq!(router.escalation_log()[0].declared_type, "Meaning");
        assert_eq!(router.escalation_log()[0].attempted_type, "Commitment");
    }

    #[test]
    fn integrity_validation_works() {
        let router = MrpRouter::new();
        let env = meaning_envelope(test_worldline());
        assert!(router.validate_integrity(&env).is_ok());
    }

    #[test]
    fn integrity_validation_detects_tampering() {
        let router = MrpRouter::new();
        let mut env = meaning_envelope(test_worldline());
        env.integrity.hash = [0u8; 32]; // corrupt hash
        assert!(router.validate_integrity(&env).is_err());
    }

    #[test]
    fn type_consistency_validation() {
        let router = MrpRouter::new();

        let env = meaning_envelope(test_worldline());
        assert!(router.validate_type_consistency(&env).is_ok());

        let mut bad_env = meaning_envelope(test_worldline());
        bad_env.header.resonance_type = ResonanceType::Intent;
        assert!(router.validate_type_consistency(&bad_env).is_err());
    }
}
