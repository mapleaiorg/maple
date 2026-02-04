use crate::connectors::ConnectorRegistry;
use crate::error::IBankError;
use crate::ledger::AuditEvent;
use crate::policy::{AutonomyMode, RiskDecision, RiskPolicyEngine};
use crate::protocol::{verify_accountable_wire_message, OriginAuthority};
use crate::storage::PersistentLedger;
use crate::types::{AccountableWireMessage, ConsequenceRecord, RouteResult, TransferIntent};
use chrono::Utc;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

/// Router responsible for invariant-preserving side-effect execution.
pub struct IBankRouter {
    policy: RiskPolicyEngine,
    connectors: Arc<Mutex<ConnectorRegistry>>,
    ledger: Arc<AsyncMutex<PersistentLedger>>,
    authority: OriginAuthority,
}

impl IBankRouter {
    pub fn new(
        policy: RiskPolicyEngine,
        connectors: Arc<Mutex<ConnectorRegistry>>,
        ledger: Arc<AsyncMutex<PersistentLedger>>,
        authority: OriginAuthority,
    ) -> Self {
        Self {
            policy,
            connectors,
            ledger,
            authority,
        }
    }

    /// Route with mandatory pipeline:
    /// 1) Accountability verification
    /// 2) Risk bounds check
    /// 3) Route with audit
    pub async fn route(
        &self,
        mode: AutonomyMode,
        intent: &TransferIntent,
        message: &AccountableWireMessage,
    ) -> Result<RouteResult, IBankError> {
        let trace_id = &intent.trace_id;
        let commitment_id = message
            .commitment_ref
            .as_ref()
            .map(|ref_| ref_.commitment_id.clone());

        {
            let mut ledger = self.ledger.lock().await;

            // Invariant 2: consequential side effects MUST be linked to explicit commitment.
            let commitment_ref = message.commitment_ref.as_ref().ok_or_else(|| {
                IBankError::InvariantViolation("missing commitment reference".to_string())
            })?;

            if !ledger.commitment_exists(&commitment_ref.commitment_id) {
                return Err(IBankError::InvariantViolation(format!(
                    "unknown commitment '{}'",
                    commitment_ref.commitment_id
                )));
            }

            // Step 1: Accountability verification before any policy or routing.
            if let Err(err) =
                verify_accountable_wire_message(message, &self.authority, ledger.as_append_only())
            {
                ledger
                    .append_audit(
                        trace_id,
                        commitment_id.clone(),
                        AuditEvent::new("accountability_rejected", err.to_string()),
                    )
                    .await?;
                ledger
                    .append_outcome(
                        trace_id,
                        commitment_id.clone(),
                        &ConsequenceRecord {
                            success: false,
                            detail: format!("accountability failure: {err}"),
                            route: None,
                            occurred_at: Utc::now(),
                        },
                    )
                    .await?;
                return Err(err);
            }

            ledger
                .append_audit(
                    trace_id,
                    commitment_id.clone(),
                    AuditEvent::new("accountability_verified", "origin+audit witness verified"),
                )
                .await?;
        }

        // Step 2: deterministic risk bounds check.
        let risk_decision = self.policy.evaluate(intent, mode);
        {
            let mut ledger = self.ledger.lock().await;
            let detail = match &risk_decision {
                RiskDecision::Allow(report)
                | RiskDecision::RequireHybrid(report)
                | RiskDecision::Deny(report) => {
                    format!(
                        "score={}, reasons={}",
                        report.score,
                        report.reasons.join(";")
                    )
                }
            };
            ledger
                .append_audit(
                    trace_id,
                    commitment_id.clone(),
                    AuditEvent::new("risk_checked", detail),
                )
                .await?;
        }

        match risk_decision {
            RiskDecision::Deny(report) => {
                self.record_failed_outcome(
                    trace_id,
                    commitment_id,
                    format!("risk denied: {}", report.reasons.join(";")),
                )
                .await?;
                return Err(IBankError::RiskDenied(report.reasons.join(";")));
            }
            RiskDecision::RequireHybrid(report) => {
                self.record_failed_outcome(
                    trace_id,
                    commitment_id,
                    format!("hybrid required: {}", report.reasons.join(";")),
                )
                .await?;
                return Err(IBankError::HybridRequired(report.reasons.join(";")));
            }
            RiskDecision::Allow(_) => {}
        }

        // Step 3: route with audit.
        let connector = {
            let connectors = self
                .connectors
                .lock()
                .map_err(|_| IBankError::Ledger("connector lock poisoned".to_string()))?;
            connectors
                .get(&intent.rail)
                .ok_or_else(|| IBankError::ConnectorNotFound(intent.rail.clone()))?
        };

        {
            let mut ledger = self.ledger.lock().await;
            ledger
                .append_audit(
                    trace_id,
                    commitment_id.clone(),
                    AuditEvent::new("routing_started", format!("connector={}", connector.rail())),
                )
                .await?;
        }

        match connector.execute(message) {
            Ok(receipt) => {
                let route = RouteResult {
                    connector: connector.rail().to_string(),
                    external_reference: receipt.settlement_id,
                    settled_at: receipt.settled_at,
                };

                let mut ledger = self.ledger.lock().await;
                ledger
                    .append_audit(
                        trace_id,
                        commitment_id.clone(),
                        AuditEvent::new("routing_succeeded", route.external_reference.clone()),
                    )
                    .await?;
                ledger
                    .append_outcome(
                        trace_id,
                        commitment_id,
                        &ConsequenceRecord {
                            success: true,
                            detail: "transfer settled".to_string(),
                            route: Some(route.clone()),
                            occurred_at: Utc::now(),
                        },
                    )
                    .await?;
                Ok(route)
            }
            Err(err) => {
                self.record_failed_outcome(
                    trace_id,
                    commitment_id,
                    format!("connector failure: {err}"),
                )
                .await?;
                Err(err)
            }
        }
    }

    async fn record_failed_outcome(
        &self,
        trace_id: &str,
        commitment_id: Option<String>,
        detail: String,
    ) -> Result<(), IBankError> {
        let mut ledger = self.ledger.lock().await;
        ledger
            .append_audit(
                trace_id,
                commitment_id.clone(),
                AuditEvent::new("routing_failed", detail.clone()),
            )
            .await?;
        ledger
            .append_outcome(
                trace_id,
                commitment_id,
                &ConsequenceRecord {
                    success: false,
                    detail,
                    route: None,
                    occurred_at: Utc::now(),
                },
            )
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectors::SettlementConnector;
    use crate::ledger::{AppendOnlyLedger, LedgerEntryKind};
    use crate::policy::RiskPolicyConfig;
    use crate::protocol::build_accountable_wire_message;
    use crate::storage::PersistentLedger;
    use crate::types::{AuditWitness, CommitmentReference, ConnectorReceipt, TransferPayload};
    use chrono::Utc;
    use rcf_commitment::{CommitmentBuilder, IntendedOutcome};
    use rcf_types::{EffectDomain, IdentityRef, ScopeConstraint};
    use std::collections::BTreeMap;

    struct OkConnector;
    impl SettlementConnector for OkConnector {
        fn rail(&self) -> &'static str {
            "ach"
        }

        fn execute(
            &self,
            _message: &AccountableWireMessage,
        ) -> Result<ConnectorReceipt, IBankError> {
            Ok(ConnectorReceipt {
                settlement_id: "settle-ok".to_string(),
                rail: "ach".to_string(),
                settled_at: Utc::now(),
                metadata: BTreeMap::new(),
            })
        }
    }

    #[tokio::test]
    async fn routing_order_is_accountability_then_risk_then_route() {
        let mut authority = OriginAuthority::new();
        authority.register_key("node", "secret");

        let mut ledger = AppendOnlyLedger::new();
        let commitment = CommitmentBuilder::new(IdentityRef::new("bank-a"), EffectDomain::Finance)
            .with_outcome(IntendedOutcome::new("transfer"))
            .with_scope(ScopeConstraint::global())
            .build()
            .unwrap();
        let commitment_entry = ledger.append_commitment("trace-1", &commitment).unwrap();
        let audit_entry = ledger
            .append_audit(
                "trace-1",
                Some(commitment.commitment_id.to_string()),
                AuditEvent::new("message_prepared", "ready"),
            )
            .unwrap();

        let payload = TransferPayload {
            from: "bank-a".to_string(),
            to: "bank-b".to_string(),
            amount_minor: 50_000,
            currency: "USD".to_string(),
            destination: "acct-2".to_string(),
            purpose: "invoice".to_string(),
        };

        let message = build_accountable_wire_message(
            "trace-1",
            "bank-a",
            payload,
            AuditWitness {
                entry_id: audit_entry.entry_id,
                entry_hash: audit_entry.entry_hash,
                observed_at: Utc::now(),
            },
            Some(CommitmentReference {
                commitment_id: commitment.commitment_id.to_string(),
                commitment_hash: commitment_entry.entry_hash,
            }),
            &authority,
            "node",
        )
        .unwrap();

        let mut registry = ConnectorRegistry::new();
        registry.register(Arc::new(OkConnector));

        let persistent = PersistentLedger::from_entries(ledger.entries().to_vec()).unwrap();
        let ledger_ref = Arc::new(AsyncMutex::new(persistent));
        let router = IBankRouter::new(
            RiskPolicyEngine::new(RiskPolicyConfig::default()),
            Arc::new(Mutex::new(registry)),
            ledger_ref.clone(),
            authority,
        );

        let intent = TransferIntent::new(
            "bank-a", "bank-b", 50_000, "USD", "ach", "acct-2", "invoice",
        );

        let _ = router
            .route(AutonomyMode::PureAi, &intent, &message)
            .await
            .unwrap();

        let ledger = ledger_ref.lock().await;
        let stages: Vec<String> = ledger
            .entries()
            .iter()
            .filter(|entry| entry.kind == LedgerEntryKind::Audit)
            .filter_map(|entry| entry.payload.get("stage").and_then(|s| s.as_str()))
            .map(|s| s.to_string())
            .collect();

        let accountable_idx = stages
            .iter()
            .position(|s| s == "accountability_verified")
            .unwrap();
        let risk_idx = stages.iter().position(|s| s == "risk_checked").unwrap();
        let route_idx = stages.iter().position(|s| s == "routing_started").unwrap();

        assert!(accountable_idx < risk_idx);
        assert!(risk_idx < route_idx);
    }
}
