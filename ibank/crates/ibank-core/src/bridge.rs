use crate::error::IBankError;
use crate::ledger::AuditEvent;
use crate::protocol::{build_accountable_wire_message, OriginAuthority};
use crate::storage::PersistentLedger;
use crate::types::{
    AccountableWireMessage, CommitmentRecord, CommitmentReference, ConsequenceRecord,
    TransferPayload,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BridgeExecutionState {
    Proposed,
    Authorized,
    Executing,
    Settled,
    Failed,
    Recorded,
}

#[derive(Debug, Clone)]
struct BridgeStateMachine {
    state: BridgeExecutionState,
}

impl BridgeStateMachine {
    fn new() -> Self {
        Self {
            state: BridgeExecutionState::Proposed,
        }
    }

    fn state(&self) -> BridgeExecutionState {
        self.state
    }

    fn transition(&mut self, next: BridgeExecutionState) -> Result<(), IBankError> {
        let allowed = matches!(
            (self.state, next),
            (
                BridgeExecutionState::Proposed,
                BridgeExecutionState::Authorized
            ) | (
                BridgeExecutionState::Authorized,
                BridgeExecutionState::Executing
            ) | (
                BridgeExecutionState::Executing,
                BridgeExecutionState::Settled
            ) | (
                BridgeExecutionState::Executing,
                BridgeExecutionState::Failed
            ) | (
                BridgeExecutionState::Settled,
                BridgeExecutionState::Recorded
            ) | (BridgeExecutionState::Failed, BridgeExecutionState::Recorded)
        );

        if !allowed {
            return Err(IBankError::InvariantViolation(format!(
                "bridge state transition not allowed: {:?} -> {:?}",
                self.state, next
            )));
        }

        self.state = next;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BridgeLegType {
    Chain,
    Rail,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChainAssetKind {
    Stablecoin,
    Native,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainBridgeLeg {
    pub leg_id: String,
    pub adapter_id: String,
    pub network: String,
    pub asset: String,
    pub asset_kind: ChainAssetKind,
    pub from_address: String,
    pub to_address: String,
    pub amount_minor: u64,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RailBridgeLeg {
    pub leg_id: String,
    pub adapter_id: String,
    pub rail: String,
    pub currency: String,
    pub from_account: String,
    pub to_account: String,
    pub amount_minor: u64,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeLeg {
    Chain(ChainBridgeLeg),
    Rail(RailBridgeLeg),
}

impl BridgeLeg {
    fn leg_id(&self) -> &str {
        match self {
            Self::Chain(leg) => &leg.leg_id,
            Self::Rail(leg) => &leg.leg_id,
        }
    }

    fn leg_type(&self) -> BridgeLegType {
        match self {
            Self::Chain(_) => BridgeLegType::Chain,
            Self::Rail(_) => BridgeLegType::Rail,
        }
    }

    fn to_payload(&self) -> TransferPayload {
        match self {
            Self::Chain(leg) => TransferPayload {
                from: leg.from_address.clone(),
                to: leg.to_address.clone(),
                amount_minor: leg.amount_minor,
                currency: leg.asset.clone(),
                destination: leg.to_address.clone(),
                purpose: leg
                    .memo
                    .clone()
                    .unwrap_or_else(|| format!("bridge_leg:{}", leg.leg_id)),
            },
            Self::Rail(leg) => TransferPayload {
                from: leg.from_account.clone(),
                to: leg.to_account.clone(),
                amount_minor: leg.amount_minor,
                currency: leg.currency.clone(),
                destination: leg.to_account.clone(),
                purpose: leg
                    .memo
                    .clone()
                    .unwrap_or_else(|| format!("bridge_leg:{}", leg.leg_id)),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BridgeRouteType {
    OnChain,
    OffChain,
    Hybrid,
}

fn route_type_from_legs(legs: &[BridgeLeg]) -> BridgeRouteType {
    let has_chain = legs.iter().any(|leg| matches!(leg, BridgeLeg::Chain(_)));
    let has_rail = legs.iter().any(|leg| matches!(leg, BridgeLeg::Rail(_)));

    match (has_chain, has_rail) {
        (true, true) => BridgeRouteType::Hybrid,
        (true, false) => BridgeRouteType::OnChain,
        (false, true) => BridgeRouteType::OffChain,
        (false, false) => BridgeRouteType::OffChain,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeExecutionRequest {
    pub execution_id: String,
    pub trace_id: String,
    pub commitment_id: String,
    pub origin_actor: String,
    pub counterparty_actor: String,
    pub legs: Vec<BridgeLeg>,
}

impl BridgeExecutionRequest {
    pub fn new(
        execution_id: impl Into<String>,
        trace_id: impl Into<String>,
        commitment_id: impl Into<String>,
        origin_actor: impl Into<String>,
        counterparty_actor: impl Into<String>,
        legs: Vec<BridgeLeg>,
    ) -> Self {
        Self {
            execution_id: execution_id.into(),
            trace_id: trace_id.into(),
            commitment_id: commitment_id.into(),
            origin_actor: origin_actor.into(),
            counterparty_actor: counterparty_actor.into(),
            legs,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainLegSettlement {
    pub tx_hash: String,
    pub settled_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RailLegSettlement {
    pub rail_reference: String,
    pub settled_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompensationActionResult {
    pub action_reference: String,
    pub detail: String,
}

#[async_trait]
pub trait ChainAdapter: Send + Sync {
    fn adapter_id(&self) -> &'static str;
    fn networks(&self) -> Vec<String>;

    async fn execute_transfer(
        &self,
        leg: &ChainBridgeLeg,
        wire: &AccountableWireMessage,
    ) -> Result<ChainLegSettlement, IBankError>;

    async fn compensate_transfer(
        &self,
        leg: &ChainBridgeLeg,
        settlement: &ChainLegSettlement,
        reason: &str,
    ) -> Result<CompensationActionResult, IBankError>;
}

#[async_trait]
pub trait RailAdapter: Send + Sync {
    fn adapter_id(&self) -> &'static str;
    fn rails(&self) -> Vec<String>;

    async fn execute_transfer(
        &self,
        leg: &RailBridgeLeg,
        wire: &AccountableWireMessage,
    ) -> Result<RailLegSettlement, IBankError>;

    async fn compensate_transfer(
        &self,
        leg: &RailBridgeLeg,
        settlement: &RailLegSettlement,
        reason: &str,
    ) -> Result<CompensationActionResult, IBankError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnifiedBridgeLegReceipt {
    pub leg_id: String,
    pub leg_type: BridgeLegType,
    pub adapter_id: String,
    pub bridge_reference: String,
    pub settled_at: DateTime<Utc>,
    pub wire_message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryAction {
    pub leg_id: String,
    pub leg_type: BridgeLegType,
    pub adapter_id: String,
    pub attempted: bool,
    pub success: bool,
    pub action_reference: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnifiedBridgeStatus {
    Settled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnifiedBridgeReceipt {
    pub execution_id: String,
    pub trace_id: String,
    pub route_type: BridgeRouteType,
    pub commitment_id: String,
    pub snapshot_hash: String,
    pub status: UnifiedBridgeStatus,
    pub state: BridgeExecutionState,
    pub leg_receipts: Vec<UnifiedBridgeLegReceipt>,
    pub recovery_plan: Vec<RecoveryAction>,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct AuthorizedCommitment {
    commitment_id: String,
    commitment_hash: String,
    snapshot_hash: String,
}

#[derive(Debug, Clone)]
enum SettledLeg {
    Chain {
        leg: ChainBridgeLeg,
        settlement: ChainLegSettlement,
        adapter_id: String,
    },
    Rail {
        leg: RailBridgeLeg,
        settlement: RailLegSettlement,
        adapter_id: String,
    },
}

pub struct BridgeExecutor {
    ledger: Arc<Mutex<PersistentLedger>>,
    authority: OriginAuthority,
    origin_key_id: String,
    chain_adapters: RwLock<HashMap<String, Arc<dyn ChainAdapter>>>,
    rail_adapters: RwLock<HashMap<String, Arc<dyn RailAdapter>>>,
}

impl BridgeExecutor {
    pub fn new(
        ledger: Arc<Mutex<PersistentLedger>>,
        authority: OriginAuthority,
        origin_key_id: impl Into<String>,
    ) -> Self {
        Self {
            ledger,
            authority,
            origin_key_id: origin_key_id.into(),
            chain_adapters: RwLock::new(HashMap::new()),
            rail_adapters: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register_chain_adapter(
        &self,
        adapter: Arc<dyn ChainAdapter>,
    ) -> Result<(), IBankError> {
        self.chain_adapters
            .write()
            .await
            .insert(adapter.adapter_id().to_string(), adapter);
        Ok(())
    }

    pub async fn register_rail_adapter(
        &self,
        adapter: Arc<dyn RailAdapter>,
    ) -> Result<(), IBankError> {
        self.rail_adapters
            .write()
            .await
            .insert(adapter.adapter_id().to_string(), adapter);
        Ok(())
    }

    pub async fn execute(
        &self,
        request: BridgeExecutionRequest,
    ) -> Result<UnifiedBridgeReceipt, IBankError> {
        if request.legs.is_empty() {
            return Err(IBankError::InvariantViolation(
                "bridge execution requires at least one leg".to_string(),
            ));
        }

        self.append_bridge_audit(
            &request.trace_id,
            Some(request.commitment_id.clone()),
            "bridge_proposed",
            format!(
                "execution_id={} legs={}",
                request.execution_id,
                request.legs.len()
            ),
        )
        .await?;

        let mut state = BridgeStateMachine::new();
        let auth = self.authorize_commitment(&request).await?;
        state.transition(BridgeExecutionState::Authorized)?;

        self.append_bridge_audit(
            &request.trace_id,
            Some(auth.commitment_id.clone()),
            "bridge_authorized",
            format!(
                "commitment_hash={} snapshot_hash={}",
                auth.commitment_hash, auth.snapshot_hash
            ),
        )
        .await?;

        state.transition(BridgeExecutionState::Executing)?;
        self.append_bridge_audit(
            &request.trace_id,
            Some(auth.commitment_id.clone()),
            "bridge_executing",
            "started executing bridge legs",
        )
        .await?;

        let mut settled_legs = Vec::new();
        let mut leg_receipts = Vec::new();
        let mut failure_reason: Option<String> = None;

        for leg in &request.legs {
            match self.execute_leg(&request, leg, &auth).await {
                Ok((receipt, settled_leg)) => {
                    leg_receipts.push(receipt);
                    settled_legs.push(settled_leg);
                }
                Err(err) => {
                    let reason = format!("leg '{}' failed: {err}", leg.leg_id());
                    self.append_bridge_audit(
                        &request.trace_id,
                        Some(auth.commitment_id.clone()),
                        "bridge_leg_failed",
                        reason.clone(),
                    )
                    .await?;
                    failure_reason = Some(reason);
                    break;
                }
            }
        }

        let (status, recovery_plan) = if let Some(reason) = failure_reason {
            state.transition(BridgeExecutionState::Failed)?;
            let recovery_plan = self
                .run_compensating_actions(&request, &auth, &settled_legs, &reason)
                .await;

            let detail_payload = serde_json::json!({
                "failure_reason": reason,
                "recovery_plan": recovery_plan,
            });

            {
                let mut ledger = self.ledger.lock().await;
                let _ = ledger
                    .append_outcome(
                        &request.trace_id,
                        Some(auth.commitment_id.clone()),
                        &ConsequenceRecord {
                            success: false,
                            detail: detail_payload.to_string(),
                            route: None,
                            occurred_at: Utc::now(),
                        },
                    )
                    .await?;
            }

            (UnifiedBridgeStatus::Failed, recovery_plan)
        } else {
            state.transition(BridgeExecutionState::Settled)?;
            {
                let mut ledger = self.ledger.lock().await;
                let _ = ledger
                    .append_outcome(
                        &request.trace_id,
                        Some(auth.commitment_id.clone()),
                        &ConsequenceRecord {
                            success: true,
                            detail: "bridge execution settled".to_string(),
                            route: None,
                            occurred_at: Utc::now(),
                        },
                    )
                    .await?;
            }
            (UnifiedBridgeStatus::Settled, Vec::new())
        };

        state.transition(BridgeExecutionState::Recorded)?;
        let receipt = UnifiedBridgeReceipt {
            execution_id: request.execution_id.clone(),
            trace_id: request.trace_id.clone(),
            route_type: route_type_from_legs(&request.legs),
            commitment_id: auth.commitment_id.clone(),
            snapshot_hash: auth.snapshot_hash,
            status,
            state: state.state(),
            leg_receipts,
            recovery_plan,
            recorded_at: Utc::now(),
        };

        self.append_bridge_audit(
            &request.trace_id,
            Some(auth.commitment_id.clone()),
            "bridge_unified_receipt",
            serde_json::to_string(&receipt)
                .map_err(|e| IBankError::Serialization(e.to_string()))?,
        )
        .await?;

        self.append_bridge_audit(
            &request.trace_id,
            Some(auth.commitment_id),
            "bridge_recorded",
            serde_json::json!({
                "execution_id": receipt.execution_id,
                "status": receipt.status,
                "state": receipt.state,
                "legs": receipt.leg_receipts.len(),
                "recoveries": receipt.recovery_plan.len(),
            })
            .to_string(),
        )
        .await?;

        Ok(receipt)
    }

    async fn authorize_commitment(
        &self,
        request: &BridgeExecutionRequest,
    ) -> Result<AuthorizedCommitment, IBankError> {
        let ledger = self.ledger.lock().await;
        let commitment_entry = ledger
            .entries()
            .iter()
            .find(|entry| {
                entry
                    .commitment_id
                    .as_deref()
                    .map(|id| id == request.commitment_id)
                    .unwrap_or(false)
            })
            .ok_or_else(|| {
                IBankError::InvariantViolation(format!(
                    "bridge authorization failed: commitment '{}' not found",
                    request.commitment_id
                ))
            })?;

        let record: CommitmentRecord = serde_json::from_value(commitment_entry.payload.clone())
            .map_err(|e| {
                IBankError::Serialization(format!("commitment payload decode failed: {e}"))
            })?;

        Ok(AuthorizedCommitment {
            commitment_id: request.commitment_id.clone(),
            commitment_hash: commitment_entry.entry_hash.clone(),
            snapshot_hash: record.platform.state_snapshot_hash,
        })
    }

    async fn execute_leg(
        &self,
        request: &BridgeExecutionRequest,
        leg: &BridgeLeg,
        auth: &AuthorizedCommitment,
    ) -> Result<(UnifiedBridgeLegReceipt, SettledLeg), IBankError> {
        let wire = self.emit_leg_wire_message(request, leg, auth).await?;

        match leg {
            BridgeLeg::Chain(chain_leg) => {
                let adapter = self
                    .chain_adapters
                    .read()
                    .await
                    .get(&chain_leg.adapter_id)
                    .cloned()
                    .ok_or_else(|| {
                        IBankError::ConnectorNotFound(format!(
                            "chain adapter '{}'",
                            chain_leg.adapter_id
                        ))
                    })?;

                let settlement = adapter.execute_transfer(chain_leg, &wire).await?;
                self.append_bridge_audit(
                    &request.trace_id,
                    Some(auth.commitment_id.clone()),
                    "bridge_leg_settled",
                    format!(
                        "leg_id={} type=chain tx_hash={}",
                        chain_leg.leg_id, settlement.tx_hash
                    ),
                )
                .await?;

                Ok((
                    UnifiedBridgeLegReceipt {
                        leg_id: chain_leg.leg_id.clone(),
                        leg_type: BridgeLegType::Chain,
                        adapter_id: chain_leg.adapter_id.clone(),
                        bridge_reference: settlement.tx_hash.clone(),
                        settled_at: settlement.settled_at,
                        wire_message_id: wire.message_id,
                    },
                    SettledLeg::Chain {
                        leg: chain_leg.clone(),
                        settlement,
                        adapter_id: adapter.adapter_id().to_string(),
                    },
                ))
            }
            BridgeLeg::Rail(rail_leg) => {
                let adapter = self
                    .rail_adapters
                    .read()
                    .await
                    .get(&rail_leg.adapter_id)
                    .cloned()
                    .ok_or_else(|| {
                        IBankError::ConnectorNotFound(format!(
                            "rail adapter '{}'",
                            rail_leg.adapter_id
                        ))
                    })?;

                let settlement = adapter.execute_transfer(rail_leg, &wire).await?;
                self.append_bridge_audit(
                    &request.trace_id,
                    Some(auth.commitment_id.clone()),
                    "bridge_leg_settled",
                    format!(
                        "leg_id={} type=rail reference={}",
                        rail_leg.leg_id, settlement.rail_reference
                    ),
                )
                .await?;

                Ok((
                    UnifiedBridgeLegReceipt {
                        leg_id: rail_leg.leg_id.clone(),
                        leg_type: BridgeLegType::Rail,
                        adapter_id: rail_leg.adapter_id.clone(),
                        bridge_reference: settlement.rail_reference.clone(),
                        settled_at: settlement.settled_at,
                        wire_message_id: wire.message_id,
                    },
                    SettledLeg::Rail {
                        leg: rail_leg.clone(),
                        settlement,
                        adapter_id: adapter.adapter_id().to_string(),
                    },
                ))
            }
        }
    }

    async fn emit_leg_wire_message(
        &self,
        request: &BridgeExecutionRequest,
        leg: &BridgeLeg,
        auth: &AuthorizedCommitment,
    ) -> Result<AccountableWireMessage, IBankError> {
        let prepared_audit = {
            let mut ledger = self.ledger.lock().await;
            ledger
                .append_audit(
                    &request.trace_id,
                    Some(auth.commitment_id.clone()),
                    AuditEvent::new(
                        "bridge_leg_prepared",
                        format!("leg_id={} type={:?}", leg.leg_id(), leg.leg_type()),
                    ),
                )
                .await?
        };

        let message = build_accountable_wire_message(
            &request.trace_id,
            &request.origin_actor,
            leg.to_payload(),
            crate::types::AuditWitness {
                entry_id: prepared_audit.entry_id,
                entry_hash: prepared_audit.entry_hash,
                observed_at: Utc::now(),
            },
            Some(CommitmentReference {
                commitment_id: auth.commitment_id.clone(),
                commitment_hash: auth.commitment_hash.clone(),
            }),
            &self.authority,
            &self.origin_key_id,
        )?;

        self.append_bridge_audit(
            &request.trace_id,
            Some(auth.commitment_id.clone()),
            "bridge_leg_wire_emitted",
            serde_json::json!({
                "leg_id": leg.leg_id(),
                "wire_message_id": message.message_id,
                "origin_proof_key": message.origin_proof.key_id,
                "commitment_id": auth.commitment_id,
                // Persist full accountable wire payload for forensic replay.
                "wire": message,
            })
            .to_string(),
        )
        .await?;

        Ok(message)
    }

    async fn run_compensating_actions(
        &self,
        request: &BridgeExecutionRequest,
        auth: &AuthorizedCommitment,
        settled_legs: &[SettledLeg],
        reason: &str,
    ) -> Vec<RecoveryAction> {
        let mut plan = Vec::new();

        for settled_leg in settled_legs.iter().rev() {
            match settled_leg {
                SettledLeg::Chain {
                    leg,
                    settlement,
                    adapter_id,
                } => {
                    let adapter = self.chain_adapters.read().await.get(adapter_id).cloned();

                    let action = if let Some(adapter) = adapter {
                        match adapter.compensate_transfer(leg, settlement, reason).await {
                            Ok(result) => RecoveryAction {
                                leg_id: leg.leg_id.clone(),
                                leg_type: BridgeLegType::Chain,
                                adapter_id: adapter_id.clone(),
                                attempted: true,
                                success: true,
                                action_reference: Some(result.action_reference),
                                detail: result.detail,
                            },
                            Err(err) => RecoveryAction {
                                leg_id: leg.leg_id.clone(),
                                leg_type: BridgeLegType::Chain,
                                adapter_id: adapter_id.clone(),
                                attempted: true,
                                success: false,
                                action_reference: None,
                                detail: format!("compensation failed: {err}"),
                            },
                        }
                    } else {
                        RecoveryAction {
                            leg_id: leg.leg_id.clone(),
                            leg_type: BridgeLegType::Chain,
                            adapter_id: adapter_id.clone(),
                            attempted: false,
                            success: false,
                            action_reference: None,
                            detail: "adapter missing for compensation".to_string(),
                        }
                    };

                    let _ = self
                        .append_bridge_audit(
                            &request.trace_id,
                            Some(auth.commitment_id.clone()),
                            "bridge_compensation",
                            serde_json::to_string(&action)
                                .unwrap_or_else(|_| "compensation audit encode failed".to_string()),
                        )
                        .await;
                    plan.push(action);
                }
                SettledLeg::Rail {
                    leg,
                    settlement,
                    adapter_id,
                } => {
                    let adapter = self.rail_adapters.read().await.get(adapter_id).cloned();

                    let action = if let Some(adapter) = adapter {
                        match adapter.compensate_transfer(leg, settlement, reason).await {
                            Ok(result) => RecoveryAction {
                                leg_id: leg.leg_id.clone(),
                                leg_type: BridgeLegType::Rail,
                                adapter_id: adapter_id.clone(),
                                attempted: true,
                                success: true,
                                action_reference: Some(result.action_reference),
                                detail: result.detail,
                            },
                            Err(err) => RecoveryAction {
                                leg_id: leg.leg_id.clone(),
                                leg_type: BridgeLegType::Rail,
                                adapter_id: adapter_id.clone(),
                                attempted: true,
                                success: false,
                                action_reference: None,
                                detail: format!("compensation failed: {err}"),
                            },
                        }
                    } else {
                        RecoveryAction {
                            leg_id: leg.leg_id.clone(),
                            leg_type: BridgeLegType::Rail,
                            adapter_id: adapter_id.clone(),
                            attempted: false,
                            success: false,
                            action_reference: None,
                            detail: "adapter missing for compensation".to_string(),
                        }
                    };

                    let _ = self
                        .append_bridge_audit(
                            &request.trace_id,
                            Some(auth.commitment_id.clone()),
                            "bridge_compensation",
                            serde_json::to_string(&action)
                                .unwrap_or_else(|_| "compensation audit encode failed".to_string()),
                        )
                        .await;
                    plan.push(action);
                }
            }
        }

        plan
    }

    async fn append_bridge_audit(
        &self,
        trace_id: &str,
        commitment_id: Option<String>,
        stage: impl Into<String>,
        detail: impl Into<String>,
    ) -> Result<(), IBankError> {
        let mut ledger = self.ledger.lock().await;
        let _ = ledger
            .append_audit(
                trace_id,
                commitment_id,
                AuditEvent::new(stage.into(), detail.into()),
            )
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::ConsequenceStageMachine;
    use crate::ledger::LedgerEntryKind;
    use crate::storage::LedgerStorageConfig;
    use crate::types::{
        CommitmentParties, CommitmentScopeContext, CommitmentTemporalBounds,
        ComplianceDecisionState, ComplianceProof, ConfidenceProfile, IBankPlatformCommitmentData,
        RegulatoryComplianceData, RiskAssessmentData,
    };
    use rcf_commitment::{CommitmentBuilder, IntendedOutcome, Reversibility};
    use rcf_types::{EffectDomain, IdentityRef, ScopeConstraint};

    struct TestChainAdapter {
        fail_leg_id: Option<String>,
    }

    #[async_trait]
    impl ChainAdapter for TestChainAdapter {
        fn adapter_id(&self) -> &'static str {
            "evm-test"
        }

        fn networks(&self) -> Vec<String> {
            vec!["base-sepolia".to_string(), "ethereum".to_string()]
        }

        async fn execute_transfer(
            &self,
            leg: &ChainBridgeLeg,
            _wire: &AccountableWireMessage,
        ) -> Result<ChainLegSettlement, IBankError> {
            if self
                .fail_leg_id
                .as_ref()
                .map(|id| id == &leg.leg_id)
                .unwrap_or(false)
            {
                return Err(IBankError::ConnectorFailure {
                    connector: self.adapter_id().to_string(),
                    message: "forced chain leg failure".to_string(),
                });
            }

            Ok(ChainLegSettlement {
                tx_hash: format!("0x{}{}", leg.leg_id, leg.amount_minor),
                settled_at: Utc::now(),
            })
        }

        async fn compensate_transfer(
            &self,
            leg: &ChainBridgeLeg,
            _settlement: &ChainLegSettlement,
            _reason: &str,
        ) -> Result<CompensationActionResult, IBankError> {
            Ok(CompensationActionResult {
                action_reference: format!("comp-chain-{}", leg.leg_id),
                detail: "reverse transfer queued".to_string(),
            })
        }
    }

    struct TestRailAdapter {
        fail_leg_id: Option<String>,
    }

    #[async_trait]
    impl RailAdapter for TestRailAdapter {
        fn adapter_id(&self) -> &'static str {
            "ach-test"
        }

        fn rails(&self) -> Vec<String> {
            vec!["ach".to_string(), "pix".to_string()]
        }

        async fn execute_transfer(
            &self,
            leg: &RailBridgeLeg,
            _wire: &AccountableWireMessage,
        ) -> Result<RailLegSettlement, IBankError> {
            if self
                .fail_leg_id
                .as_ref()
                .map(|id| id == &leg.leg_id)
                .unwrap_or(false)
            {
                return Err(IBankError::ConnectorFailure {
                    connector: self.adapter_id().to_string(),
                    message: "forced rail leg failure".to_string(),
                });
            }

            Ok(RailLegSettlement {
                rail_reference: format!("rail-{}-{}", leg.rail, leg.leg_id),
                settled_at: Utc::now(),
            })
        }

        async fn compensate_transfer(
            &self,
            leg: &RailBridgeLeg,
            _settlement: &RailLegSettlement,
            _reason: &str,
        ) -> Result<CompensationActionResult, IBankError> {
            Ok(CompensationActionResult {
                action_reference: format!("comp-rail-{}", leg.leg_id),
                detail: "refund queued".to_string(),
            })
        }
    }

    async fn setup_executor(
        snapshot_hash: &str,
    ) -> (BridgeExecutor, Arc<Mutex<PersistentLedger>>, String) {
        let mut stage = ConsequenceStageMachine::new("trace-setup");
        stage.mark_presence().unwrap();
        stage.mark_coupling().unwrap();
        stage.mark_meaning().unwrap();
        stage.mark_intent().unwrap();

        let commitment =
            CommitmentBuilder::new(IdentityRef::new("issuer-a"), EffectDomain::Finance)
                .with_outcome(IntendedOutcome::new("bridge test"))
                .with_scope(ScopeConstraint::global())
                .with_reversibility(Reversibility::Reversible)
                .build()
                .unwrap();

        let record = CommitmentRecord {
            commitment,
            scope: CommitmentScopeContext {
                action: "bridge_transfer".to_string(),
                resources: vec!["ach".to_string(), "chain".to_string()],
                constraints: vec!["requires_commitment_id=true".to_string()],
            },
            parties: CommitmentParties {
                principal: "issuer-a".to_string(),
                counterparty: "merchant-b".to_string(),
            },
            temporal_bounds: CommitmentTemporalBounds {
                not_before: Utc::now(),
                not_after: Utc::now(),
            },
            reversibility: "reversible".to_string(),
            confidence_context: ConfidenceProfile {
                meaning_confidence: 1.0,
                model_confidence: 1.0,
                overall_confidence: 1.0,
                blocking_ambiguity: false,
                notes: vec![],
            },
            platform: IBankPlatformCommitmentData {
                transaction_type: "transfer".to_string(),
                value: "100000 USD".to_string(),
                risk_assessment: RiskAssessmentData {
                    score: 10,
                    fraud_score: 3,
                    reasons: vec![],
                },
                regulatory_compliance: RegulatoryComplianceData {
                    status: "ok".to_string(),
                    required_checks: vec!["kyc".to_string()],
                    proof_placeholders: vec![],
                },
                compliance_proof: ComplianceProof {
                    policy_version: "ibank-compliance-v1".to_string(),
                    decision: ComplianceDecisionState::Green,
                    reason_codes: vec!["BASELINE_CHECKS_PASSED".to_string()],
                    evidence_hashes: vec!["h1".to_string()],
                },
                state_snapshot_hash: snapshot_hash.to_string(),
            },
        };

        let mut ledger = PersistentLedger::bootstrap(LedgerStorageConfig::Memory)
            .await
            .unwrap();
        let entry = ledger
            .append_commitment_record("trace-setup", &record)
            .await
            .unwrap();
        let committed = entry
            .commitment_id
            .expect("commitment entry should carry commitment id");

        let shared_ledger = Arc::new(Mutex::new(ledger));
        let mut authority = OriginAuthority::new();
        authority.register_key("bridge-node", "bridge-secret");

        let executor = BridgeExecutor::new(shared_ledger.clone(), authority, "bridge-node");

        (executor, shared_ledger, committed)
    }

    fn hybrid_request(commitment_id: &str, trace_id: &str) -> BridgeExecutionRequest {
        BridgeExecutionRequest::new(
            "exec-1",
            trace_id,
            commitment_id,
            "issuer-a",
            "merchant-b",
            vec![
                BridgeLeg::Chain(ChainBridgeLeg {
                    leg_id: "leg-chain-1".to_string(),
                    adapter_id: "evm-test".to_string(),
                    network: "base-sepolia".to_string(),
                    asset: "USDC".to_string(),
                    asset_kind: ChainAssetKind::Stablecoin,
                    from_address: "0xaaa".to_string(),
                    to_address: "0xbbb".to_string(),
                    amount_minor: 75_000,
                    memo: Some("fiat->stablecoin".to_string()),
                }),
                BridgeLeg::Rail(RailBridgeLeg {
                    leg_id: "leg-rail-1".to_string(),
                    adapter_id: "ach-test".to_string(),
                    rail: "ach".to_string(),
                    currency: "USD".to_string(),
                    from_account: "acct-a".to_string(),
                    to_account: "acct-b".to_string(),
                    amount_minor: 75_000,
                    memo: Some("stablecoin->local rail".to_string()),
                }),
            ],
        )
    }

    #[tokio::test]
    async fn success_path_records_all_legs_and_unified_receipt() {
        let (executor, ledger, commitment_id) = setup_executor("snapshot-abc").await;
        executor
            .register_chain_adapter(Arc::new(TestChainAdapter { fail_leg_id: None }))
            .await
            .unwrap();
        executor
            .register_rail_adapter(Arc::new(TestRailAdapter { fail_leg_id: None }))
            .await
            .unwrap();

        let receipt = executor
            .execute(hybrid_request(&commitment_id, "trace-success"))
            .await
            .unwrap();

        assert_eq!(receipt.status, UnifiedBridgeStatus::Settled);
        assert_eq!(receipt.state, BridgeExecutionState::Recorded);
        assert_eq!(receipt.snapshot_hash, "snapshot-abc");
        assert_eq!(receipt.route_type, BridgeRouteType::Hybrid);
        assert_eq!(receipt.leg_receipts.len(), 2);
        assert!(receipt.recovery_plan.is_empty());
        assert!(receipt
            .leg_receipts
            .iter()
            .all(|leg| !leg.bridge_reference.is_empty()));

        let ledger = ledger.lock().await;
        let wire_emitted = ledger
            .entries()
            .iter()
            .filter(|entry| entry.kind == LedgerEntryKind::Audit)
            .filter(|entry| {
                entry
                    .payload
                    .get("stage")
                    .and_then(|v| v.as_str())
                    .map(|stage| stage == "bridge_leg_wire_emitted")
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(wire_emitted, 2);
        let wire_payload_carries_commitment = ledger
            .entries()
            .iter()
            .filter(|entry| entry.kind == LedgerEntryKind::Audit)
            .filter_map(|entry| {
                let stage = entry.payload.get("stage").and_then(|v| v.as_str());
                let detail = entry.payload.get("detail").and_then(|v| v.as_str());
                if stage == Some("bridge_leg_wire_emitted") {
                    detail
                } else {
                    None
                }
            })
            .all(|detail| detail.contains("\"wire\"") && detail.contains("\"commitment_id\""));
        assert!(wire_payload_carries_commitment);

        let unified_receipt_audit_present = ledger
            .entries()
            .iter()
            .filter(|entry| entry.kind == LedgerEntryKind::Audit)
            .any(|entry| {
                entry
                    .payload
                    .get("stage")
                    .and_then(|v| v.as_str())
                    .map(|stage| stage == "bridge_unified_receipt")
                    .unwrap_or(false)
            });
        assert!(unified_receipt_audit_present);

        let success_outcome = ledger.entries().iter().any(|entry| {
            entry.kind == LedgerEntryKind::Outcome
                && entry
                    .payload
                    .get("success")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
        });
        assert!(success_outcome);
    }

    #[tokio::test]
    async fn failure_path_triggers_compensation_and_explicit_failure_outcome() {
        let (executor, ledger, commitment_id) = setup_executor("snapshot-def").await;
        executor
            .register_chain_adapter(Arc::new(TestChainAdapter { fail_leg_id: None }))
            .await
            .unwrap();
        executor
            .register_rail_adapter(Arc::new(TestRailAdapter {
                fail_leg_id: Some("leg-rail-1".to_string()),
            }))
            .await
            .unwrap();

        let receipt = executor
            .execute(hybrid_request(&commitment_id, "trace-fail"))
            .await
            .unwrap();

        assert_eq!(receipt.status, UnifiedBridgeStatus::Failed);
        assert_eq!(receipt.state, BridgeExecutionState::Recorded);
        assert_eq!(receipt.snapshot_hash, "snapshot-def");
        assert!(!receipt.recovery_plan.is_empty());
        assert!(receipt
            .recovery_plan
            .iter()
            .any(|recovery| recovery.success && recovery.action_reference.is_some()));

        let ledger = ledger.lock().await;
        let compensation_audits = ledger
            .entries()
            .iter()
            .filter(|entry| entry.kind == LedgerEntryKind::Audit)
            .filter(|entry| {
                entry
                    .payload
                    .get("stage")
                    .and_then(|v| v.as_str())
                    .map(|stage| stage == "bridge_compensation")
                    .unwrap_or(false)
            })
            .count();
        assert!(compensation_audits >= 1);

        let failure_outcome_detail = ledger
            .entries()
            .iter()
            .find(|entry| {
                entry.kind == LedgerEntryKind::Outcome
                    && entry.payload.get("success").and_then(|v| v.as_bool()) == Some(false)
            })
            .and_then(|entry| entry.payload.get("detail").and_then(|v| v.as_str()))
            .unwrap_or_default()
            .to_string();

        assert!(failure_outcome_detail.contains("recovery_plan"));
    }
}
