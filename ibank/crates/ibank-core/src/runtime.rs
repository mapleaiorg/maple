use crate::aggregation::{
    AggregationConnector, AggregationUser, AssetPair, TimeRange, UnifiedLedgerAssembler,
    UnifiedLedgerView,
};
use crate::bridge::{
    BridgeExecutionRequest, BridgeExecutor, ChainAdapter, RailAdapter, UnifiedBridgeReceipt,
};
use crate::connectors::{ConnectorRegistry, SettlementConnector};
use crate::error::IBankError;
use crate::flow::ConsequenceStageMachine;
use crate::ledger::{AuditEvent, LedgerEntry, LedgerEntryKind};
use crate::policy::{AutonomyMode, RiskDecision, RiskPolicyConfig, RiskPolicyEngine};
use crate::protocol::{build_accountable_wire_message, OriginAuthority};
use crate::router::IBankRouter;
use crate::storage::{LedgerStorageConfig, PersistentLedger};
use crate::types::{
    AuditWitness, CommitmentParties, CommitmentRecord, CommitmentReference, CommitmentScopeContext,
    CommitmentTemporalBounds, ComplianceDecision, ComplianceProof, ConfidenceProfile,
    ConsequenceRecord, ExecutionMode, HandleRequest, HandleResponse, HandleStatus,
    HumanAttestation, IBankPlatformCommitmentData, IntentRecord, MeaningField,
    RegulatoryComplianceData, RiskAssessmentData, RiskReport, RouteResult, TransferIntent,
    TransferPayload,
};
use chrono::{Duration, Utc};
use maple_runtime::config::ibank_runtime_config;
use maple_runtime::{
    CouplingParams, CouplingPersistence, CouplingScope, MapleRuntime, PresenceError,
    ResonatorHandle, ResonatorProfile, ResonatorSpec, SymmetryType,
};
use rcf_commitment::{CommitmentBuilder, IntendedOutcome, Reversibility, Target};
use rcf_types::{EffectDomain, IdentityRef, ResourceLimits, ScopeConstraint, TemporalValidity};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;
use uuid::Uuid;

/// iBank runtime configuration.
#[derive(Debug, Clone)]
pub struct IBankEngineConfig {
    pub coupling_attention_cost: u64,
    pub commitment_validity_secs: i64,
    pub origin_key_id: String,
    pub origin_key_secret: String,
    pub ledger_storage: LedgerStorageConfig,
}

impl Default for IBankEngineConfig {
    fn default() -> Self {
        Self {
            coupling_attention_cost: 100,
            commitment_validity_secs: 300,
            origin_key_id: "ibank-node".to_string(),
            origin_key_secret: "ibank-local-dev-secret".to_string(),
            ledger_storage: LedgerStorageConfig::Memory,
        }
    }
}

/// iBank engine layering policy/accountability routing over MAPLE runtime primitives.
pub struct IBankEngine {
    maple: MapleRuntime,
    participants: tokio::sync::RwLock<HashMap<String, ResonatorHandle>>,
    aggregation_connectors: tokio::sync::RwLock<Vec<Arc<dyn AggregationConnector>>>,
    latest_unified_snapshots: tokio::sync::RwLock<HashMap<String, LatestUnifiedSnapshot>>,
    ledger: Arc<AsyncMutex<PersistentLedger>>,
    connectors: Arc<Mutex<ConnectorRegistry>>,
    bridge: Arc<BridgeExecutor>,
    router: IBankRouter,
    policy: RiskPolicyEngine,
    origin_authority: OriginAuthority,
    config: IBankEngineConfig,
}

/// Latest cached unified ledger snapshot for dashboard/ops consumption.
#[derive(Debug, Clone)]
pub struct LatestUnifiedSnapshot {
    pub user_id: String,
    pub source_trace_id: Option<String>,
    pub captured_at: chrono::DateTime<Utc>,
    pub view: UnifiedLedgerView,
}

impl IBankEngine {
    /// Bootstrap iBank on top of MAPLE's iBank runtime profile.
    pub async fn bootstrap(
        policy_config: RiskPolicyConfig,
        config: IBankEngineConfig,
    ) -> Result<Self, IBankError> {
        let maple = MapleRuntime::bootstrap(ibank_runtime_config())
            .await
            .map_err(|e| IBankError::MapleRuntime(e.to_string()))?;

        let mut authority = OriginAuthority::new();
        authority.register_key(
            config.origin_key_id.clone(),
            config.origin_key_secret.clone(),
        );

        let connectors = Arc::new(Mutex::new(ConnectorRegistry::new()));
        let ledger = Arc::new(AsyncMutex::new(
            PersistentLedger::bootstrap(config.ledger_storage.clone()).await?,
        ));
        let policy = RiskPolicyEngine::new(policy_config);

        let router = IBankRouter::new(
            policy.clone(),
            connectors.clone(),
            ledger.clone(),
            authority.clone(),
        );
        let bridge = Arc::new(BridgeExecutor::new(
            ledger.clone(),
            authority.clone(),
            config.origin_key_id.clone(),
        ));

        Ok(Self {
            maple,
            participants: tokio::sync::RwLock::new(HashMap::new()),
            aggregation_connectors: tokio::sync::RwLock::new(Vec::new()),
            latest_unified_snapshots: tokio::sync::RwLock::new(HashMap::new()),
            ledger,
            connectors,
            bridge,
            router,
            policy,
            origin_authority: authority,
            config,
        })
    }

    pub fn register_connector(
        &self,
        connector: Arc<dyn SettlementConnector>,
    ) -> Result<(), IBankError> {
        let mut registry = self
            .connectors
            .lock()
            .map_err(|_| IBankError::Ledger("connector lock poisoned".to_string()))?;
        registry.register(connector);
        Ok(())
    }

    pub async fn register_chain_adapter(
        &self,
        adapter: Arc<dyn ChainAdapter>,
    ) -> Result<(), IBankError> {
        self.bridge.register_chain_adapter(adapter).await
    }

    pub async fn register_rail_adapter(
        &self,
        adapter: Arc<dyn RailAdapter>,
    ) -> Result<(), IBankError> {
        self.bridge.register_rail_adapter(adapter).await
    }

    /// Execute a bridge route (on-chain/off-chain/hybrid) under commitment authorization.
    pub async fn execute_bridge_route(
        &self,
        request: BridgeExecutionRequest,
    ) -> Result<UnifiedBridgeReceipt, IBankError> {
        self.bridge.execute(request).await
    }

    pub async fn register_aggregation_connector(
        &self,
        connector: Arc<dyn AggregationConnector>,
    ) -> Result<(), IBankError> {
        let mut connectors = self.aggregation_connectors.write().await;
        let connector_id = connector.connector_id();

        if let Some(existing_idx) = connectors
            .iter()
            .position(|registered| registered.connector_id() == connector_id)
        {
            connectors[existing_idx] = connector;
            return Ok(());
        }

        connectors.push(connector);
        Ok(())
    }

    pub async fn latest_unified_snapshot(&self, user_id: &str) -> Option<LatestUnifiedSnapshot> {
        self.latest_unified_snapshots
            .read()
            .await
            .get(user_id)
            .cloned()
    }

    pub async fn refresh_unified_snapshot(
        &self,
        user_id: &str,
        pair: AssetPair,
        amount_minor: u64,
        window_days: i64,
    ) -> Result<LatestUnifiedSnapshot, IBankError> {
        let user = AggregationUser::new(user_id.to_string());
        let range = TimeRange::last_days(window_days.max(1));
        let connectors = self.aggregation_connectors.read().await.clone();
        let view =
            UnifiedLedgerAssembler::build(&connectors, &user, &range, &pair, amount_minor).await?;

        let snapshot = LatestUnifiedSnapshot {
            user_id: user_id.to_string(),
            source_trace_id: None,
            captured_at: Utc::now(),
            view,
        };

        let mut cache = self.latest_unified_snapshots.write().await;
        cache.insert(user_id.to_string(), snapshot.clone());

        Ok(snapshot)
    }

    pub async fn ledger_entries(&self) -> Result<Vec<LedgerEntry>, IBankError> {
        let ledger = self.ledger.lock().await;
        Ok(ledger.entries().to_vec())
    }

    /// Return bridge unified receipts reconstructed from append-only audit records.
    ///
    /// Bridge executor persists each finalized receipt as an audit stage
    /// (`bridge_unified_receipt`) so operators can query completed executions without
    /// introducing mutable side tables.
    pub async fn bridge_receipts(&self) -> Result<Vec<UnifiedBridgeReceipt>, IBankError> {
        let ledger = self.ledger.lock().await;

        ledger
            .entries()
            .iter()
            .filter(|entry| entry.kind == LedgerEntryKind::Audit)
            .filter_map(|entry| {
                let stage = entry.payload.get("stage").and_then(|v| v.as_str());
                if stage == Some("bridge_unified_receipt") {
                    Some(entry.payload.get("detail").and_then(|v| v.as_str()))
                } else {
                    None
                }
            })
            .flatten()
            .map(|detail| {
                serde_json::from_str::<UnifiedBridgeReceipt>(detail).map_err(|e| {
                    IBankError::Serialization(format!("bridge receipt decode failed: {e}"))
                })
            })
            .collect()
    }

    pub async fn verify_ledger_chain(&self) -> Result<bool, IBankError> {
        let ledger = self.ledger.lock().await;
        Ok(ledger.verify_chain())
    }

    pub async fn ledger_backend(&self) -> String {
        let ledger = self.ledger.lock().await;
        ledger.backend_label().to_string()
    }

    /// Persist an explicit hybrid rejection outcome.
    ///
    /// This allows external approval workflows to reject queued actions while still
    /// preserving the "no silent drop" invariant in the append-only ledger.
    pub async fn record_hybrid_rejection(
        &self,
        trace_id: &str,
        commitment_id: Option<String>,
        approver_id: &str,
        note: Option<&str>,
    ) -> Result<(), IBankError> {
        let detail = match note {
            Some(reason) if !reason.is_empty() => {
                format!("hybrid request rejected by {approver_id}: {reason}")
            }
            _ => format!("hybrid request rejected by {approver_id}"),
        };
        self.record_failure(trace_id, commitment_id, detail).await
    }

    /// Persist a signed human attestation into the append-only audit trail.
    pub async fn record_human_attestation(
        &self,
        trace_id: &str,
        commitment_id: Option<String>,
        attestation: &HumanAttestation,
    ) -> Result<(), IBankError> {
        self.append_audit_stage(
            trace_id,
            commitment_id,
            "human_attestation_recorded",
            serde_json::to_string(attestation)
                .map_err(|e| IBankError::Serialization(e.to_string()))?,
        )
        .await
    }

    /// Persist an external audit stage tied to a trace/commitment.
    pub async fn record_external_audit(
        &self,
        trace_id: &str,
        commitment_id: Option<String>,
        stage: &str,
        detail: String,
    ) -> Result<(), IBankError> {
        self.append_audit_stage(trace_id, commitment_id, stage, detail)
            .await
    }

    /// Single iBank API/App entrypoint.
    ///
    /// This method intentionally returns a response object (not an error) for most
    /// business outcomes so callers can render deterministic state in UI/API surfaces.
    pub async fn handle(&self, request: HandleRequest) -> HandleResponse {
        let trace_id = request
            .trace_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        match self.handle_result(request, trace_id.clone()).await {
            Ok(response) => response,
            Err(err) => {
                let _ = self.record_failure(&trace_id, None, err.to_string()).await;
                HandleResponse {
                    trace_id,
                    commitment_id: None,
                    status: HandleStatus::Failed,
                    mode: None,
                    decision_reason: err.to_string(),
                    meaning: None,
                    intent: None,
                    risk_report: None,
                    route: None,
                }
            }
        }
    }

    async fn handle_result(
        &self,
        request: HandleRequest,
        trace_id: String,
    ) -> Result<HandleResponse, IBankError> {
        // Parse Meaning -> Intent -> Risk before deciding mode.
        let meaning = self.parse_meaning_field(&request);
        let intent_record = self.stabilize_request_intent(&request, &meaning);
        let transfer_intent = self.transfer_intent_from_request(&request, &trace_id, &meaning);
        let compliance_decision = self.policy.evaluate_compliance(&transfer_intent);
        let risk_decision = self.policy.evaluate_with_compliance(
            &transfer_intent,
            AutonomyMode::PureAi,
            &compliance_decision,
        );
        let risk_report = extract_risk_report(&risk_decision);
        let decision_reason = format!(
            "{} | {}",
            render_decision_reason(&risk_decision, &risk_report),
            render_compliance_decision(&compliance_decision)
        );

        let execution = match risk_decision {
            RiskDecision::Allow(_) => self
                .process_transfer(AutonomyMode::PureAi, transfer_intent.clone())
                .await
                .map(|route| {
                    (
                        HandleStatus::ExecutedAutonomous,
                        Some(ExecutionMode::PureAi),
                        Some(route),
                    )
                }),
            RiskDecision::RequireHybrid(_) => {
                if request
                    .approval
                    .as_ref()
                    .map(|approval| approval.approved)
                    .unwrap_or(false)
                {
                    self.process_transfer(AutonomyMode::Hybrid, transfer_intent.clone())
                        .await
                        .map(|route| {
                            (
                                HandleStatus::ExecutedHybrid,
                                Some(ExecutionMode::Hybrid),
                                Some(route),
                            )
                        })
                } else {
                    // Deliberately execute with PureAi mode so router blocks side-effects,
                    // writes explicit failure outcomes, and preserves commitment traceability.
                    match self
                        .process_transfer(AutonomyMode::PureAi, transfer_intent.clone())
                        .await
                    {
                        Err(IBankError::HybridRequired(_)) => Ok((
                            HandleStatus::PendingHumanApproval,
                            Some(ExecutionMode::Hybrid),
                            None,
                        )),
                        Err(err) => Err(err),
                        Ok(route) => Ok((
                            HandleStatus::ExecutedAutonomous,
                            Some(ExecutionMode::PureAi),
                            Some(route),
                        )),
                    }
                }
            }
            RiskDecision::Deny(_) => {
                // Execute in PureAi mode so denial is persisted as an explicit outcome.
                match self
                    .process_transfer(AutonomyMode::PureAi, transfer_intent.clone())
                    .await
                {
                    Err(IBankError::RiskDenied(_)) => Ok((HandleStatus::Denied, None, None)),
                    Err(err) => Err(err),
                    Ok(route) => Ok((
                        HandleStatus::ExecutedAutonomous,
                        Some(ExecutionMode::PureAi),
                        Some(route),
                    )),
                }
            }
        }?;

        let commitment_id = self.find_commitment_id_for_trace(&trace_id).await?;

        Ok(HandleResponse {
            trace_id,
            commitment_id,
            status: execution.0,
            mode: execution.1,
            decision_reason,
            meaning: Some(meaning),
            intent: Some(intent_record),
            risk_report: Some(risk_report),
            route: execution.2,
        })
    }

    /// Backward-compatible transfer API used by lower-level runtime integrations.
    pub async fn process_transfer(
        &self,
        mode: AutonomyMode,
        intent: TransferIntent,
    ) -> Result<RouteResult, IBankError> {
        let mut stage = ConsequenceStageMachine::new(intent.trace_id.clone());

        let (origin, counterparty) = match self.establish_presence(&intent).await {
            Ok(pair) => {
                stage.mark_presence()?;
                pair
            }
            Err(err) => {
                self.record_failure(&intent.trace_id, None, err.to_string())
                    .await?;
                return Err(err);
            }
        };

        if let Err(err) = self.establish_coupling(&origin, &counterparty).await {
            self.record_failure(&intent.trace_id, None, err.to_string())
                .await?;
            return Err(err);
        }
        stage.mark_coupling()?;

        let meaning = self.form_meaning(&intent);
        stage.mark_meaning()?;
        self.append_audit_stage(
            &intent.trace_id,
            None,
            "meaning_formed",
            format!("ambiguity={:.2}", meaning.ambiguity_score),
        )
        .await?;

        let intent_record = self.stabilize_intent(&intent, &meaning);
        stage.mark_intent()?;
        self.append_audit_stage(
            &intent.trace_id,
            None,
            "intent_stabilized",
            format!(
                "confidence={:.2}",
                intent_record.confidence.overall_confidence
            ),
        )
        .await?;

        let compliance_decision = self.policy.evaluate_compliance(&intent);
        let compliance_proof = self.policy.generate_compliance_proof(&compliance_decision);
        self.append_audit_stage(
            &intent.trace_id,
            None,
            "compliance_gated",
            format!(
                "decision={} reason_codes={} evidence_hashes={}",
                compliance_label(&compliance_decision),
                compliance_proof.reason_codes.join(","),
                compliance_proof.evidence_hashes.join(",")
            ),
        )
        .await?;

        let pre_risk = self.policy.evaluate_with_compliance(
            &intent,
            AutonomyMode::PureAi,
            &compliance_decision,
        );
        let risk_report = extract_risk_report(&pre_risk);
        self.append_audit_stage(
            &intent.trace_id,
            None,
            "risk_scored",
            format!(
                "score={} fraud={}",
                risk_report.score, risk_report.fraud_score
            ),
        )
        .await?;

        let unified_view = match self.build_unified_ledger_view(&intent).await {
            Ok(view) => view,
            Err(err) => {
                self.record_failure(
                    &intent.trace_id,
                    None,
                    format!("unified ledger aggregation failed: {err}"),
                )
                .await?;
                return Err(err);
            }
        };
        self.cache_unified_snapshot(
            intent.origin_actor.clone(),
            Some(intent.trace_id.clone()),
            unified_view.clone(),
        )
        .await;
        self.append_audit_stage(
            &intent.trace_id,
            None,
            "unified_snapshot_built",
            format!(
                "snapshot_hash={} connectors={}",
                unified_view.snapshot_hash,
                unified_view.connector_attestations.len()
            ),
        )
        .await?;

        let commitment_record = match self.declare_commitment_record(
            &intent,
            &intent_record,
            &risk_report,
            &compliance_decision,
            &compliance_proof,
            &unified_view.snapshot_hash,
        ) {
            Ok(record) => record,
            Err(err) => {
                self.record_failure(&intent.trace_id, None, err.to_string())
                    .await?;
                return Err(err);
            }
        };

        let commitment_id = commitment_record.commitment.commitment_id.to_string();
        let commitment_entry_hash = {
            let mut ledger = self.ledger.lock().await;
            let entry = ledger
                .append_commitment_record(&intent.trace_id, &commitment_record)
                .await?;
            entry.entry_hash
        };
        stage.mark_commitment()?;
        self.append_audit_stage(
            &intent.trace_id,
            Some(commitment_id.clone()),
            "state_snapshot_attested",
            format!("snapshot_hash={}", unified_view.snapshot_hash),
        )
        .await?;

        let commitment_ref = CommitmentReference {
            commitment_id: commitment_id.clone(),
            commitment_hash: commitment_entry_hash,
        };

        let message = match self
            .build_wire_message(&intent, commitment_ref.clone())
            .await
        {
            Ok(msg) => msg,
            Err(err) => {
                self.record_failure(
                    &intent.trace_id,
                    Some(commitment_id.clone()),
                    format!("wire message build failed: {err}"),
                )
                .await?;
                return Err(err);
            }
        };

        stage.mark_consequence()?;
        self.router.route(mode, &intent, &message).await
    }

    async fn establish_presence(
        &self,
        transfer: &TransferIntent,
    ) -> Result<(ResonatorHandle, ResonatorHandle), IBankError> {
        let origin = self
            .get_or_register_participant(&transfer.origin_actor)
            .await?;
        let counterparty = self
            .get_or_register_participant(&transfer.counterparty_actor)
            .await?;

        self.signal_presence(&origin).await?;
        self.signal_presence(&counterparty).await?;

        Ok((origin, counterparty))
    }

    async fn get_or_register_participant(
        &self,
        actor: &str,
    ) -> Result<ResonatorHandle, IBankError> {
        if let Some(existing) = self.participants.read().await.get(actor).cloned() {
            return Ok(existing);
        }

        let mut spec = ResonatorSpec::default();
        spec.profile = ResonatorProfile::IBank;
        spec.identity.name = Some(actor.to_string());
        spec.identity
            .metadata
            .insert("ibank_actor".to_string(), actor.to_string());

        let created = self
            .maple
            .register_resonator(spec)
            .await
            .map_err(|e| IBankError::MapleRuntime(e.to_string()))?;

        let mut participants = self.participants.write().await;
        let entry = participants
            .entry(actor.to_string())
            .or_insert_with(|| created.clone());
        Ok(entry.clone())
    }

    async fn signal_presence(&self, handle: &ResonatorHandle) -> Result<(), IBankError> {
        match handle
            .signal_presence(maple_runtime::PresenceState::new())
            .await
        {
            Ok(()) => Ok(()),
            Err(PresenceError::RateLimitExceeded) => Ok(()),
            Err(err) => Err(IBankError::MapleRuntime(err.to_string())),
        }
    }

    async fn establish_coupling(
        &self,
        origin: &ResonatorHandle,
        counterparty: &ResonatorHandle,
    ) -> Result<(), IBankError> {
        let params = CouplingParams {
            source: origin.id,
            target: counterparty.id,
            initial_strength: 0.3,
            initial_attention_cost: self.config.coupling_attention_cost,
            persistence: CouplingPersistence::Session,
            scope: CouplingScope::Full,
            symmetry: SymmetryType::Symmetric,
        };

        origin
            .couple_with(counterparty.id, params)
            .await
            .map(|_| ())
            .map_err(|e| IBankError::MapleRuntime(e.to_string()))
    }

    fn parse_meaning_field(&self, request: &HandleRequest) -> MeaningField {
        let mut notes = Vec::new();
        let lowered = request.user_intent.to_ascii_lowercase();

        if lowered.contains("maybe")
            || lowered.contains("around")
            || lowered.contains("about")
            || lowered.contains("if possible")
        {
            notes.push("hedged language detected".to_string());
        }

        if request.transaction_type.eq_ignore_ascii_case("dispute") {
            notes.push("dispute semantics increase ambiguity".to_string());
        }

        let mut ambiguity_score = request
            .ambiguity_hint
            .map(|hint| hint.clamp(0.0, 1.0))
            .unwrap_or_else(|| if notes.is_empty() { 0.08 } else { 0.42 });

        if request.model_uncertainty > 0.55 {
            ambiguity_score = ambiguity_score.max(0.5);
            notes.push("model uncertainty elevated".to_string());
        }

        MeaningField {
            summary: format!(
                "{} {} from {} to {} via {}",
                request.amount_minor,
                request.currency,
                request.origin_actor,
                request.counterparty_actor,
                request.rail
            ),
            inferred_action: request.transaction_type.clone(),
            ambiguity_notes: notes,
            ambiguity_score,
            confidence: (1.0 - ambiguity_score).clamp(0.0, 1.0),
            formed_at: Utc::now(),
        }
    }

    fn transfer_intent_from_request(
        &self,
        request: &HandleRequest,
        trace_id: &str,
        meaning: &MeaningField,
    ) -> TransferIntent {
        let dispute_flag = request.transaction_type.eq_ignore_ascii_case("dispute");

        let mut intent = TransferIntent::new(
            request.origin_actor.clone(),
            request.counterparty_actor.clone(),
            request.amount_minor,
            request.currency.clone(),
            request.rail.clone(),
            request.destination.clone(),
            request.user_intent.clone(),
        )
        .with_transaction_type(request.transaction_type.clone(), dispute_flag)
        .with_risk_inputs(
            request.jurisdiction.clone(),
            request.counterparty_risk,
            request.anomaly_score,
            request.model_uncertainty,
        );

        intent.trace_id = trace_id.to_string();
        intent.ambiguity = meaning.ambiguity_score;
        intent.compliance_flags = request.compliance_flags.clone();
        intent.metadata = request.metadata.clone();
        intent
    }

    fn form_meaning(&self, transfer: &TransferIntent) -> MeaningField {
        MeaningField {
            summary: format!(
                "Transfer {} {} from {} to {} for {}",
                transfer.amount_minor,
                transfer.currency,
                transfer.origin_actor,
                transfer.counterparty_actor,
                transfer.purpose
            ),
            inferred_action: transfer.transaction_type.clone(),
            ambiguity_notes: Vec::new(),
            ambiguity_score: transfer.ambiguity.clamp(0.0, 1.0),
            confidence: 1.0_f32 - transfer.ambiguity.clamp(0.0, 1.0),
            formed_at: Utc::now(),
        }
    }

    fn stabilize_request_intent(
        &self,
        request: &HandleRequest,
        meaning: &MeaningField,
    ) -> IntentRecord {
        let blocking_ambiguity =
            meaning.ambiguity_score > self.policy.config().ambiguity_hybrid_threshold;

        let profile = ConfidenceProfile {
            meaning_confidence: meaning.confidence,
            model_confidence: (1.0 - request.model_uncertainty).clamp(0.0, 1.0),
            overall_confidence: ((meaning.confidence * 0.6)
                + ((1.0 - request.model_uncertainty).clamp(0.0, 1.0) * 0.4))
                .clamp(0.0, 1.0),
            blocking_ambiguity,
            notes: meaning.ambiguity_notes.clone(),
        };

        IntentRecord {
            objective: format!("execute_{}", request.transaction_type),
            rationale: format!(
                "{} | jurisdiction={} | destination={}",
                meaning.summary, request.jurisdiction, request.destination
            ),
            confidence: profile,
            stabilized_at: Utc::now(),
        }
    }

    fn stabilize_intent(&self, transfer: &TransferIntent, meaning: &MeaningField) -> IntentRecord {
        let blocking_ambiguity =
            meaning.ambiguity_score > self.policy.config().ambiguity_hybrid_threshold;

        let profile = ConfidenceProfile {
            meaning_confidence: meaning.confidence,
            model_confidence: (1.0 - transfer.model_uncertainty).clamp(0.0, 1.0),
            overall_confidence: ((meaning.confidence * 0.6)
                + ((1.0 - transfer.model_uncertainty).clamp(0.0, 1.0) * 0.4))
                .clamp(0.0, 1.0),
            blocking_ambiguity,
            notes: meaning.ambiguity_notes.clone(),
        };

        IntentRecord {
            objective: format!("execute_{}", transfer.transaction_type),
            rationale: format!(
                "{} | amount={} {} | confidence={:.2}",
                meaning.summary,
                transfer.amount_minor,
                transfer.currency,
                profile.overall_confidence
            ),
            confidence: profile,
            stabilized_at: Utc::now(),
        }
    }

    async fn build_unified_ledger_view(
        &self,
        transfer: &TransferIntent,
    ) -> Result<UnifiedLedgerView, IBankError> {
        let user = AggregationUser::new(transfer.origin_actor.clone());
        let range = TimeRange::last_days(30);
        let pair = AssetPair::new(transfer.currency.clone(), transfer.currency.clone());
        let connectors = self.aggregation_connectors.read().await.clone();

        UnifiedLedgerAssembler::build(&connectors, &user, &range, &pair, transfer.amount_minor)
            .await
    }

    async fn cache_unified_snapshot(
        &self,
        user_id: String,
        source_trace_id: Option<String>,
        view: UnifiedLedgerView,
    ) {
        let snapshot = LatestUnifiedSnapshot {
            user_id: user_id.clone(),
            source_trace_id,
            captured_at: Utc::now(),
            view,
        };
        let mut cache = self.latest_unified_snapshots.write().await;
        cache.insert(user_id, snapshot);
    }

    fn declare_commitment_record(
        &self,
        transfer: &TransferIntent,
        intent: &IntentRecord,
        risk: &RiskReport,
        compliance_decision: &ComplianceDecision,
        compliance_proof: &ComplianceProof,
        state_snapshot_hash: &str,
    ) -> Result<CommitmentRecord, IBankError> {
        let limits = ResourceLimits::new()
            .with_max_value(transfer.amount_minor)
            .with_max_operations(1);

        let scope = ScopeConstraint::new(
            vec![transfer.destination.clone()],
            vec![transfer.transaction_type.clone()],
        );

        let reversibility = if transfer.rail.eq_ignore_ascii_case("chain") {
            Reversibility::PartiallyReversible("depends_on_chain_finality_window".to_string())
        } else {
            Reversibility::Reversible
        };

        let mut commitment_builder = CommitmentBuilder::new(
            IdentityRef::new(transfer.origin_actor.clone()),
            EffectDomain::Finance,
        )
        .with_outcome(
            IntendedOutcome::new(format!(
                "settle {} {} via {}",
                transfer.amount_minor, transfer.currency, transfer.rail
            ))
            .with_criteria(format!("destination={}", transfer.destination))
            .with_criteria(format!("transaction_type={}", transfer.transaction_type)),
        )
        .with_scope(scope)
        .with_target(Target::identity(transfer.counterparty_actor.clone()))
        .with_limits(limits)
        .with_validity(TemporalValidity::from_now_secs(
            self.config.commitment_validity_secs,
        ))
        .with_reversibility(reversibility.clone())
        .with_intent_ref(intent.objective.clone())
        .with_policy_tag("ibank")
        .with_policy_tag(format!("transaction_type:{}", transfer.transaction_type))
        .with_policy_tag(format!("risk_score:{}", risk.score))
        .with_policy_tag(format!("jurisdiction:{}", transfer.jurisdiction))
        .with_policy_tag(format!(
            "compliance_decision:{}",
            compliance_label(compliance_decision)
        ));

        if let Some(merchant_id) = transfer.metadata.get("merchant_id") {
            commitment_builder =
                commitment_builder.with_policy_tag(format!("merchant:{merchant_id}"));
        }
        if let Some(dispute_policy_ref) = transfer.metadata.get("dispute_policy_ref") {
            commitment_builder =
                commitment_builder.with_policy_tag(format!("dispute_policy:{dispute_policy_ref}"));
        }
        if let Some(reversibility_window_secs) = transfer.metadata.get("reversibility_window_secs")
        {
            commitment_builder = commitment_builder.with_policy_tag(format!(
                "reversibility_window_secs:{reversibility_window_secs}"
            ));
        }
        if let Some(commerce_rail) = transfer.metadata.get("commerce_rail") {
            commitment_builder =
                commitment_builder.with_policy_tag(format!("commerce_rail:{commerce_rail}"));
        }

        let commitment = commitment_builder
            .build()
            .map_err(|e| IBankError::InvariantViolation(format!("commitment build failed: {e}")))?;

        let temporal_bounds = CommitmentTemporalBounds {
            not_before: Utc::now(),
            not_after: Utc::now() + Duration::seconds(self.config.commitment_validity_secs),
        };

        let compliance = RegulatoryComplianceData {
            status: match compliance_decision.state {
                crate::types::ComplianceDecisionState::Green => "green".to_string(),
                crate::types::ComplianceDecisionState::ReviewRequired => {
                    "review_required".to_string()
                }
                crate::types::ComplianceDecisionState::Block => "blocked".to_string(),
            },
            required_checks: if compliance_decision.reasons.is_empty() {
                vec![
                    "kyc".to_string(),
                    "aml".to_string(),
                    "sanctions".to_string(),
                ]
            } else {
                compliance_decision.reasons.clone()
            },
            // Placeholders are explicit by design and are replaced by integration layers.
            proof_placeholders: vec![
                format!("proof://{}/regulatory-compliance", transfer.trace_id),
                format!("proof://{}/risk-assessment", transfer.trace_id),
            ],
        };

        let platform = IBankPlatformCommitmentData {
            transaction_type: transfer.transaction_type.clone(),
            value: format!("{} {}", transfer.amount_minor, transfer.currency),
            risk_assessment: RiskAssessmentData {
                score: risk.score,
                fraud_score: risk.fraud_score,
                reasons: risk.reasons.clone(),
            },
            regulatory_compliance: compliance,
            compliance_proof: compliance_proof.clone(),
            state_snapshot_hash: state_snapshot_hash.to_string(),
        };

        let mut constraints = vec![
            format!("max_amount_minor={}", transfer.amount_minor),
            format!("jurisdiction={}", transfer.jurisdiction),
            "requires_commitment_id=true".to_string(),
        ];
        if let Some(merchant_id) = transfer.metadata.get("merchant_id") {
            constraints.push(format!("merchant_id={merchant_id}"));
        }
        if let Some(dispute_policy_ref) = transfer.metadata.get("dispute_policy_ref") {
            constraints.push(format!("dispute_policy_ref={dispute_policy_ref}"));
        }
        if let Some(reversibility_window_secs) = transfer.metadata.get("reversibility_window_secs")
        {
            constraints.push(format!(
                "reversibility_window_secs={reversibility_window_secs}"
            ));
        }
        if let Some(commerce_rail) = transfer.metadata.get("commerce_rail") {
            constraints.push(format!("commerce_rail={commerce_rail}"));
        }

        Ok(CommitmentRecord {
            commitment,
            scope: CommitmentScopeContext {
                action: transfer.transaction_type.clone(),
                resources: vec![transfer.destination.clone(), transfer.rail.clone()],
                constraints,
            },
            parties: CommitmentParties {
                principal: transfer.origin_actor.clone(),
                counterparty: transfer.counterparty_actor.clone(),
            },
            temporal_bounds,
            reversibility: match reversibility {
                Reversibility::Reversible => "reversible".to_string(),
                Reversibility::PartiallyReversible(reason) => {
                    format!("partially_reversible:{reason}")
                }
                Reversibility::Irreversible => "irreversible".to_string(),
            },
            confidence_context: intent.confidence.clone(),
            platform,
        })
    }

    async fn build_wire_message(
        &self,
        transfer: &TransferIntent,
        commitment_ref: CommitmentReference,
    ) -> Result<crate::types::AccountableWireMessage, IBankError> {
        let prepared_audit = {
            let mut ledger = self.ledger.lock().await;
            ledger
                .append_audit(
                    &transfer.trace_id,
                    Some(commitment_ref.commitment_id.clone()),
                    AuditEvent::new("message_prepared", "wire envelope ready"),
                )
                .await?
        };

        let payload = TransferPayload {
            from: transfer.origin_actor.clone(),
            to: transfer.counterparty_actor.clone(),
            amount_minor: transfer.amount_minor,
            currency: transfer.currency.clone(),
            destination: transfer.destination.clone(),
            purpose: transfer.purpose.clone(),
        };

        build_accountable_wire_message(
            &transfer.trace_id,
            &transfer.origin_actor,
            payload,
            AuditWitness {
                entry_id: prepared_audit.entry_id,
                entry_hash: prepared_audit.entry_hash,
                observed_at: Utc::now(),
            },
            Some(commitment_ref),
            &self.origin_authority,
            &self.config.origin_key_id,
        )
    }

    async fn record_failure(
        &self,
        trace_id: &str,
        commitment_id: Option<String>,
        detail: String,
    ) -> Result<(), IBankError> {
        let mut ledger = self.ledger.lock().await;
        let _ = ledger
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

    async fn append_audit_stage(
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

    async fn find_commitment_id_for_trace(
        &self,
        trace_id: &str,
    ) -> Result<Option<String>, IBankError> {
        let ledger = self.ledger.lock().await;

        Ok(ledger
            .entries()
            .iter()
            .rev()
            .find(|entry| entry.trace_id == trace_id && entry.kind == LedgerEntryKind::Commitment)
            .and_then(|entry| entry.commitment_id.clone()))
    }
}

fn extract_risk_report(decision: &RiskDecision) -> RiskReport {
    match decision {
        RiskDecision::Allow(report)
        | RiskDecision::RequireHybrid(report)
        | RiskDecision::Deny(report) => report.clone(),
    }
}

fn render_decision_reason(decision: &RiskDecision, report: &RiskReport) -> String {
    let prefix = match decision {
        RiskDecision::Allow(_) => "pure_ai_allowed",
        RiskDecision::RequireHybrid(_) => "hybrid_required",
        RiskDecision::Deny(_) => "denied",
    };

    if report.reasons.is_empty() {
        return prefix.to_string();
    }

    format!("{prefix}: {}", report.reasons.join("; "))
}

fn compliance_label(decision: &ComplianceDecision) -> &'static str {
    match decision.state {
        crate::types::ComplianceDecisionState::Green => "green",
        crate::types::ComplianceDecisionState::ReviewRequired => "review_required",
        crate::types::ComplianceDecisionState::Block => "block",
    }
}

fn render_compliance_decision(decision: &ComplianceDecision) -> String {
    if decision.reasons.is_empty() {
        return format!("compliance={}", compliance_label(decision));
    }

    format!(
        "compliance={} reason_codes={}",
        compliance_label(decision),
        decision.reasons.join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectors::SettlementConnector;
    use crate::types::{ConnectorReceipt, HumanApproval};
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingConnector {
        calls: Arc<AtomicUsize>,
    }

    impl SettlementConnector for CountingConnector {
        fn rail(&self) -> &'static str {
            "ach"
        }

        fn execute(
            &self,
            _message: &crate::types::AccountableWireMessage,
        ) -> Result<ConnectorReceipt, IBankError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(ConnectorReceipt {
                settlement_id: "ok-1".to_string(),
                rail: "ach".to_string(),
                settled_at: Utc::now(),
                metadata: BTreeMap::new(),
            })
        }
    }

    #[tokio::test]
    async fn pure_ai_500_transfer_routes_autonomous_and_records_commitment_and_audit() {
        let engine =
            IBankEngine::bootstrap(RiskPolicyConfig::default(), IBankEngineConfig::default())
                .await
                .unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        engine
            .register_connector(Arc::new(CountingConnector {
                calls: calls.clone(),
            }))
            .unwrap();

        let mut request = HandleRequest::new(
            "issuer-a",
            "merchant-b",
            50_000, // $500.00
            "USD",
            "ach",
            "acct-123",
            "pay invoice 889",
        );
        request.jurisdiction = "US".to_string();
        request.counterparty_risk = 12;
        request.anomaly_score = 8;
        request.model_uncertainty = 0.08;

        let response = engine.handle(request).await;
        assert_eq!(response.status, HandleStatus::ExecutedAutonomous);
        assert_eq!(response.mode, Some(ExecutionMode::PureAi));
        assert!(response.route.is_some());
        assert!(response.commitment_id.is_some());

        let entries = engine.ledger_entries().await.unwrap();
        assert!(entries
            .iter()
            .any(|entry| entry.kind == LedgerEntryKind::Commitment));
        let commitment_entry = entries
            .iter()
            .find(|entry| {
                entry.kind == LedgerEntryKind::Commitment
                    && entry.commitment_id == response.commitment_id
            })
            .expect("commitment entry should exist");
        assert_eq!(
            commitment_entry
                .payload
                .get("platform")
                .and_then(|v| v.get("compliance_proof"))
                .and_then(|v| v.get("policy_version"))
                .and_then(|v| v.as_str()),
            Some("ibank-compliance-v1")
        );
        assert_eq!(
            commitment_entry
                .payload
                .get("platform")
                .and_then(|v| v.get("compliance_proof"))
                .and_then(|v| v.get("decision"))
                .and_then(|v| v.as_str()),
            Some("green")
        );
        assert!(entries.iter().any(|entry| {
            entry.kind == LedgerEntryKind::Audit
                && entry
                    .payload
                    .get("stage")
                    .and_then(|value| value.as_str())
                    .map(|stage| stage == "accountability_verified")
                    .unwrap_or(false)
        }));
        assert!(entries.iter().any(|entry| {
            entry.kind == LedgerEntryKind::Audit
                && entry
                    .payload
                    .get("stage")
                    .and_then(|value| value.as_str())
                    .map(|stage| stage == "state_snapshot_attested")
                    .unwrap_or(false)
        }));
        let snapshot_audit = entries.iter().find(|entry| {
            entry.kind == LedgerEntryKind::Audit
                && entry.commitment_id == response.commitment_id
                && entry
                    .payload
                    .get("stage")
                    .and_then(|value| value.as_str())
                    .map(|stage| stage == "state_snapshot_attested")
                    .unwrap_or(false)
        });
        assert!(snapshot_audit.is_some());
        assert!(snapshot_audit
            .and_then(|entry| entry.payload.get("detail").and_then(|v| v.as_str()))
            .map(|detail| detail.contains("snapshot_hash="))
            .unwrap_or(false));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(engine.verify_ledger_chain().await.unwrap());
    }

    #[tokio::test]
    async fn hybrid_required_for_large_amount_blocks_execution_until_approval() {
        let engine =
            IBankEngine::bootstrap(RiskPolicyConfig::default(), IBankEngineConfig::default())
                .await
                .unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        engine
            .register_connector(Arc::new(CountingConnector {
                calls: calls.clone(),
            }))
            .unwrap();

        let mut request = HandleRequest::new(
            "issuer-a",
            "merchant-b",
            1_500_000, // $15,000.00
            "USD",
            "ach",
            "acct-123",
            "move treasury funds",
        );
        request.jurisdiction = "US".to_string();

        let response = engine.handle(request).await;
        assert_eq!(response.status, HandleStatus::PendingHumanApproval);
        assert_eq!(response.mode, Some(ExecutionMode::Hybrid));
        assert!(response.route.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        let entries = engine.ledger_entries().await.unwrap();
        assert!(entries
            .iter()
            .any(|entry| entry.kind == LedgerEntryKind::Commitment));
        assert!(entries.iter().any(|entry| {
            entry.kind == LedgerEntryKind::Outcome
                && entry
                    .payload
                    .get("success")
                    .and_then(|value| value.as_bool())
                    == Some(false)
        }));
    }

    #[tokio::test]
    async fn hybrid_required_for_dispute_blocks_without_approval() {
        let engine =
            IBankEngine::bootstrap(RiskPolicyConfig::default(), IBankEngineConfig::default())
                .await
                .unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        engine
            .register_connector(Arc::new(CountingConnector {
                calls: calls.clone(),
            }))
            .unwrap();

        let mut request = HandleRequest::new(
            "issuer-a",
            "merchant-b",
            50_000,
            "USD",
            "ach",
            "acct-123",
            "investigate and pay dispute adjustment",
        );
        request.transaction_type = "dispute".to_string();
        request.jurisdiction = "US".to_string();

        let response = engine.handle(request).await;
        assert_eq!(response.status, HandleStatus::PendingHumanApproval);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn hybrid_executes_after_explicit_approval() {
        let engine =
            IBankEngine::bootstrap(RiskPolicyConfig::default(), IBankEngineConfig::default())
                .await
                .unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        engine
            .register_connector(Arc::new(CountingConnector {
                calls: calls.clone(),
            }))
            .unwrap();

        let mut request = HandleRequest::new(
            "issuer-a",
            "merchant-b",
            1_500_000,
            "USD",
            "ach",
            "acct-123",
            "move treasury funds",
        );
        request.jurisdiction = "US".to_string();
        request.approval = Some(HumanApproval::approved_by("ops-supervisor"));

        let response = engine.handle(request).await;
        assert_eq!(response.status, HandleStatus::ExecutedHybrid);
        assert_eq!(response.mode, Some(ExecutionMode::Hybrid));
        assert!(response.route.is_some());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
