use crate::error::IBankError;
use crate::runtime::IBankEngine;
use crate::types::{EscalationCase, HandleRequest, HandleResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RefundPolicyPreference {
    Flexible,
    Standard,
    Strict,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommerceConstraints {
    pub max_price_minor: u64,
    pub currency: String,
    pub region: String,
    pub latest_checkout_by: Option<DateTime<Utc>>,
    pub latest_delivery_by: Option<DateTime<Utc>>,
}

impl CommerceConstraints {
    pub fn new(
        max_price_minor: u64,
        currency: impl Into<String>,
        region: impl Into<String>,
    ) -> Self {
        Self {
            max_price_minor,
            currency: currency.into(),
            region: region.into(),
            latest_checkout_by: None,
            latest_delivery_by: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommerceIntent {
    pub trace_id: Option<String>,
    pub user_id: String,
    pub item_or_service: String,
    pub constraints: CommerceConstraints,
    pub preferred_rails: Vec<String>,
    pub refund_policy_preference: RefundPolicyPreference,
    pub metadata: BTreeMap<String, String>,
}

impl CommerceIntent {
    pub fn new(
        user_id: impl Into<String>,
        item_or_service: impl Into<String>,
        constraints: CommerceConstraints,
    ) -> Self {
        Self {
            trace_id: None,
            user_id: user_id.into(),
            item_or_service: item_or_service.into(),
            constraints,
            preferred_rails: Vec::new(),
            refund_policy_preference: RefundPolicyPreference::Standard,
            metadata: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommerceOption {
    pub option_id: String,
    pub merchant_id: String,
    pub rail: String,
    pub item_or_service: String,
    pub price_minor: u64,
    pub currency: String,
    pub estimated_fee_minor: u64,
    pub estimated_total_minor: u64,
    pub estimated_risk_score: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommerceDraftPlan {
    pub trace_id: String,
    pub options: Vec<CommerceOption>,
    pub recommended_option_id: Option<String>,
    pub estimated_overall_risk: u8,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommerceDiscoveryResult {
    pub trace_id: String,
    pub intent: CommerceIntent,
    pub draft_plan: CommerceDraftPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommerceTimelineSource {
    Merchant,
    Shipping,
    Payment,
    Dispute,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommerceTimelineEvent {
    pub event_id: String,
    pub source: CommerceTimelineSource,
    pub status: String,
    pub detail: String,
    pub temporal_anchor: String,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommerceOrder {
    pub order_id: String,
    pub trace_id: String,
    pub user_id: String,
    pub merchant_id: String,
    pub item_or_service: String,
    pub amount_minor: u64,
    pub currency: String,
    pub rail: String,
    pub commitment_id: String,
    pub dispute_policy_ref: String,
    pub reversibility_window_secs: i64,
    pub timeline: Vec<CommerceTimelineEvent>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommercePaymentResult {
    pub order: CommerceOrder,
    pub payment_response: HandleResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommerceDisputeResult {
    pub order_id: String,
    pub dispute_trace_id: String,
    pub escalated: bool,
    pub escalation_case: Option<EscalationCase>,
    pub response: HandleResponse,
}

#[derive(Debug, Clone)]
pub struct CommerceAgentConfig {
    pub default_rails: Vec<String>,
    pub reversibility_window_secs: i64,
    pub low_risk_auto_refund_max_minor: Option<u64>,
    pub low_risk_auto_refund_max_fraud_score: u8,
}

impl Default for CommerceAgentConfig {
    fn default() -> Self {
        Self {
            default_rails: vec!["ach".to_string(), "chain".to_string()],
            reversibility_window_secs: 60 * 60 * 24 * 2,
            low_risk_auto_refund_max_minor: None,
            low_risk_auto_refund_max_fraud_score: 20,
        }
    }
}

/// Agentic commerce lifecycle orchestrator.
///
/// Lifecycle:
/// `Discover -> Quote -> Commit -> Pay -> Track -> After-sales/Dispute`
///
/// Invariant handling:
/// - Discovery never calls payment execution and never creates commitments.
/// - Payment uses iBank `handle` entrypoint, which enforces commitment-before-side-effect.
/// - Tracking updates require temporal anchors and are mirrored into audit trail.
/// - Disputes escalate to hybrid by default, with optional low-risk auto-refund policy.
pub struct AgenticCommerceAgent {
    engine: Arc<IBankEngine>,
    config: CommerceAgentConfig,
    orders: RwLock<HashMap<String, CommerceOrder>>,
    escalation_cases: RwLock<HashMap<String, EscalationCase>>,
}

impl AgenticCommerceAgent {
    pub fn new(engine: Arc<IBankEngine>, config: CommerceAgentConfig) -> Self {
        Self {
            engine,
            config,
            orders: RwLock::new(HashMap::new()),
            escalation_cases: RwLock::new(HashMap::new()),
        }
    }

    pub async fn discover(
        &self,
        intent: CommerceIntent,
    ) -> Result<CommerceDiscoveryResult, IBankError> {
        let trace_id = intent
            .trace_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let options = self.build_options(&intent);
        let recommended_option_id = options
            .iter()
            .min_by_key(|option| (option.estimated_total_minor, option.estimated_risk_score))
            .map(|option| option.option_id.clone());
        let estimated_overall_risk = options
            .iter()
            .map(|option| option.estimated_risk_score)
            .min()
            .unwrap_or(0);
        let notes = vec![
            "discovery_only:no_payment_execution".to_string(),
            "plan_contains_estimated_fees_and_risks".to_string(),
        ];

        Ok(CommerceDiscoveryResult {
            trace_id: trace_id.clone(),
            intent,
            draft_plan: CommerceDraftPlan {
                trace_id,
                options,
                recommended_option_id,
                estimated_overall_risk,
                notes,
            },
        })
    }

    pub async fn initiate_payment(
        &self,
        intent: CommerceIntent,
        selected_option_id: Option<&str>,
    ) -> Result<CommercePaymentResult, IBankError> {
        let discovery = self.discover(intent.clone()).await?;
        let selected = match selected_option_id {
            Some(option_id) => discovery
                .draft_plan
                .options
                .iter()
                .find(|option| option.option_id == option_id)
                .cloned()
                .ok_or_else(|| {
                    IBankError::InvariantViolation(format!(
                        "selected commerce option '{}' not found",
                        option_id
                    ))
                })?,
            None => discovery
                .draft_plan
                .recommended_option_id
                .as_ref()
                .and_then(|option_id| {
                    discovery
                        .draft_plan
                        .options
                        .iter()
                        .find(|option| option.option_id == *option_id)
                })
                .cloned()
                .ok_or_else(|| {
                    IBankError::InvariantViolation(
                        "discovery did not produce a recommended commerce option".to_string(),
                    )
                })?,
        };

        let dispute_policy_ref = dispute_policy_reference(intent.refund_policy_preference.clone());
        let mut request = HandleRequest::new(
            intent.user_id.clone(),
            selected.merchant_id.clone(),
            selected.price_minor,
            selected.currency.clone(),
            selected.rail.clone(),
            format!("merchant_settlement://{}", selected.merchant_id),
            format!("purchase {}", selected.item_or_service),
        );
        request.trace_id = Some(discovery.trace_id.clone());
        request.transaction_type = "purchase".to_string();
        request.jurisdiction = intent.constraints.region.clone();
        request.counterparty_risk = selected.estimated_risk_score.min(100);
        request.anomaly_score = selected.estimated_risk_score.saturating_sub(5).min(100);
        request.model_uncertainty = 0.05;
        request.metadata = intent.metadata.clone();
        request
            .metadata
            .insert("merchant_id".to_string(), selected.merchant_id.clone());
        request
            .metadata
            .insert("commerce_rail".to_string(), selected.rail.clone());
        request.metadata.insert(
            "reversibility_window_secs".to_string(),
            self.config.reversibility_window_secs.to_string(),
        );
        request
            .metadata
            .insert("dispute_policy_ref".to_string(), dispute_policy_ref.clone());
        request.metadata.insert(
            "commerce_item".to_string(),
            selected.item_or_service.clone(),
        );

        let payment_response = self.engine.handle(request).await;
        let commitment_id = payment_response.commitment_id.clone().ok_or_else(|| {
            IBankError::InvariantViolation(
                "payment initiation must produce commitment before side effects".to_string(),
            )
        })?;

        let now = Utc::now();
        let mut order = CommerceOrder {
            order_id: format!("order-{}", Uuid::new_v4()),
            trace_id: discovery.trace_id.clone(),
            user_id: intent.user_id,
            merchant_id: selected.merchant_id,
            item_or_service: selected.item_or_service,
            amount_minor: selected.price_minor,
            currency: selected.currency,
            rail: selected.rail,
            commitment_id,
            dispute_policy_ref,
            reversibility_window_secs: self.config.reversibility_window_secs,
            timeline: Vec::new(),
            created_at: now,
            updated_at: now,
        };
        let payment_trace_id = order.trace_id.clone();

        self.append_order_timeline_event(
            &mut order,
            CommerceTimelineSource::Payment,
            "payment_initiated",
            "payment request submitted to iBank",
            format!("payment://{}/initiated", payment_trace_id),
        )
        .await?;

        if let Some(route) = &payment_response.route {
            self.append_order_timeline_event(
                &mut order,
                CommerceTimelineSource::Payment,
                "payment_settled",
                format!(
                    "connector={} external_reference={}",
                    route.connector, route.external_reference
                ),
                route.external_reference.clone(),
            )
            .await?;
        } else {
            let pending_anchor = format!("payment://{}/pending", order.trace_id);
            self.append_order_timeline_event(
                &mut order,
                CommerceTimelineSource::Payment,
                "payment_pending",
                format!("status={:?}", payment_response.status),
                pending_anchor,
            )
            .await?;
        }

        {
            let mut orders = self.orders.write().await;
            orders.insert(order.order_id.clone(), order.clone());
        }

        Ok(CommercePaymentResult {
            order,
            payment_response,
        })
    }

    pub async fn append_tracking_update(
        &self,
        order_id: &str,
        source: CommerceTimelineSource,
        status: impl Into<String>,
        detail: impl Into<String>,
        temporal_anchor: impl Into<String>,
    ) -> Result<CommerceOrder, IBankError> {
        let status = status.into();
        let detail = detail.into();
        let temporal_anchor = temporal_anchor.into();
        if temporal_anchor.trim().is_empty() {
            return Err(IBankError::InvariantViolation(
                "tracking updates require a temporal anchor".to_string(),
            ));
        }

        let (trace_id, commitment_id, event, updated_order) = {
            let mut orders = self.orders.write().await;
            let order = orders.get_mut(order_id).ok_or_else(|| {
                IBankError::InvariantViolation(format!("unknown order '{}'", order_id))
            })?;
            let event = CommerceTimelineEvent {
                event_id: format!("evt-{}", Uuid::new_v4()),
                source,
                status,
                detail,
                temporal_anchor,
                observed_at: Utc::now(),
            };
            order.timeline.push(event.clone());
            order.updated_at = event.observed_at;
            (
                order.trace_id.clone(),
                Some(order.commitment_id.clone()),
                event,
                order.clone(),
            )
        };

        self.engine
            .record_external_audit(
                &trace_id,
                commitment_id,
                "commerce_tracking_update",
                serde_json::to_string(&event).map_err(|e| {
                    IBankError::Serialization(format!("tracking event serialization failed: {e}"))
                })?,
            )
            .await?;

        Ok(updated_order)
    }

    pub async fn open_dispute(
        &self,
        order_id: &str,
        reason: impl Into<String>,
    ) -> Result<CommerceDisputeResult, IBankError> {
        let reason = reason.into();
        let order = self.order(order_id).await.ok_or_else(|| {
            IBankError::InvariantViolation(format!("unknown order '{}'", order_id))
        })?;

        if self.auto_refund_allowed(&order) {
            let mut refund_request = HandleRequest::new(
                order.merchant_id.clone(),
                order.user_id.clone(),
                order.amount_minor,
                order.currency.clone(),
                order.rail.clone(),
                format!("refund://{}", order.user_id),
                format!("auto refund for order {} ({})", order.order_id, reason),
            );
            refund_request.trace_id = Some(Uuid::new_v4().to_string());
            refund_request.transaction_type = "refund".to_string();
            refund_request.counterparty_risk = 5;
            refund_request.anomaly_score = 5;
            refund_request.model_uncertainty = 0.02;
            refund_request
                .metadata
                .insert("merchant_id".to_string(), order.merchant_id.clone());
            refund_request
                .metadata
                .insert("commerce_rail".to_string(), order.rail.clone());
            refund_request
                .metadata
                .insert("commerce_dispute_reason".to_string(), reason.clone());
            let response = self.engine.handle(refund_request).await;

            let updated = self
                .append_tracking_update(
                    &order.order_id,
                    CommerceTimelineSource::Dispute,
                    "auto_refund_executed",
                    "low-risk dispute processed via auto-refund policy",
                    format!("dispute://{}/auto-refund", order.order_id),
                )
                .await?;

            return Ok(CommerceDisputeResult {
                order_id: updated.order_id,
                dispute_trace_id: response.trace_id.clone(),
                escalated: false,
                escalation_case: None,
                response,
            });
        }

        let dispute_trace_id = Uuid::new_v4().to_string();
        let mut dispute_request = HandleRequest::new(
            order.user_id.clone(),
            order.merchant_id.clone(),
            order.amount_minor,
            order.currency.clone(),
            order.rail.clone(),
            format!("merchant_settlement://{}", order.merchant_id),
            format!("dispute order {}: {}", order.order_id, reason),
        );
        dispute_request.trace_id = Some(dispute_trace_id.clone());
        dispute_request.transaction_type = "dispute".to_string();
        dispute_request.counterparty_risk = 40;
        dispute_request.anomaly_score = 40;
        dispute_request.model_uncertainty = 0.2;
        dispute_request
            .metadata
            .insert("commerce_order_id".to_string(), order.order_id.clone());
        dispute_request.metadata.insert(
            "dispute_policy_ref".to_string(),
            order.dispute_policy_ref.clone(),
        );
        dispute_request
            .metadata
            .insert("commerce_dispute_reason".to_string(), reason.clone());

        let response = self.engine.handle(dispute_request).await;
        let escalation = EscalationCase {
            case_id: format!("case-{}", Uuid::new_v4()),
            commitment_id: response
                .commitment_id
                .clone()
                .or_else(|| Some(order.commitment_id.clone())),
            risk_report: response.risk_report.clone(),
            evidence_bundle: vec![
                format!("order_id={}", order.order_id),
                format!("merchant_id={}", order.merchant_id),
                format!("reason={reason}"),
                format!("payment_commitment={}", order.commitment_id),
            ],
            recommended_actions: vec![
                "collect_evidence_from_merchant".to_string(),
                "verify_refund_eligibility".to_string(),
                "obtain_signed_human_attestation".to_string(),
            ],
        };
        {
            let mut escalations = self.escalation_cases.write().await;
            escalations.insert(order.order_id.clone(), escalation.clone());
        }

        self.engine
            .record_external_audit(
                &dispute_trace_id,
                escalation.commitment_id.clone(),
                "commerce_dispute_escalated",
                serde_json::to_string(&escalation).map_err(|e| {
                    IBankError::Serialization(format!("escalation serialization failed: {e}"))
                })?,
            )
            .await?;

        let _ = self
            .append_tracking_update(
                &order.order_id,
                CommerceTimelineSource::Dispute,
                "dispute_escalated",
                "dispute opened and escalated for hybrid review",
                format!("dispute://{}/escalated", order.order_id),
            )
            .await?;

        Ok(CommerceDisputeResult {
            order_id: order.order_id,
            dispute_trace_id,
            escalated: true,
            escalation_case: Some(escalation),
            response,
        })
    }

    pub async fn order(&self, order_id: &str) -> Option<CommerceOrder> {
        self.orders.read().await.get(order_id).cloned()
    }

    pub async fn escalation_case_for_order(&self, order_id: &str) -> Option<EscalationCase> {
        self.escalation_cases.read().await.get(order_id).cloned()
    }

    fn build_options(&self, intent: &CommerceIntent) -> Vec<CommerceOption> {
        let rails = if intent.preferred_rails.is_empty() {
            self.config.default_rails.clone()
        } else {
            intent.preferred_rails.clone()
        };
        rails
            .into_iter()
            .enumerate()
            .map(|(index, rail)| {
                let price_minor = if intent.constraints.max_price_minor == 0 {
                    10_000
                } else {
                    intent
                        .constraints
                        .max_price_minor
                        .saturating_sub((index as u64) * 500)
                };
                let estimated_fee_minor = estimated_fee_for_rail(&rail);
                let estimated_risk_score =
                    estimated_risk_for_option(&rail, &intent.constraints.region);
                CommerceOption {
                    option_id: format!("option-{}-{}", index + 1, rail),
                    merchant_id: format!("merchant-{}", index + 1),
                    rail,
                    item_or_service: intent.item_or_service.clone(),
                    price_minor,
                    currency: intent.constraints.currency.clone(),
                    estimated_fee_minor,
                    estimated_total_minor: price_minor.saturating_add(estimated_fee_minor),
                    estimated_risk_score,
                }
            })
            .collect()
    }

    fn auto_refund_allowed(&self, order: &CommerceOrder) -> bool {
        self.config
            .low_risk_auto_refund_max_minor
            .map(|max_minor| order.amount_minor <= max_minor)
            .unwrap_or(false)
            && self.config.low_risk_auto_refund_max_fraud_score <= 20
    }

    async fn append_order_timeline_event(
        &self,
        order: &mut CommerceOrder,
        source: CommerceTimelineSource,
        status: impl Into<String>,
        detail: impl Into<String>,
        temporal_anchor: impl Into<String>,
    ) -> Result<(), IBankError> {
        let event = CommerceTimelineEvent {
            event_id: format!("evt-{}", Uuid::new_v4()),
            source,
            status: status.into(),
            detail: detail.into(),
            temporal_anchor: temporal_anchor.into(),
            observed_at: Utc::now(),
        };
        if event.temporal_anchor.trim().is_empty() {
            return Err(IBankError::InvariantViolation(
                "tracking updates require temporal anchor".to_string(),
            ));
        }
        order.timeline.push(event.clone());
        order.updated_at = event.observed_at;

        self.engine
            .record_external_audit(
                &order.trace_id,
                Some(order.commitment_id.clone()),
                "commerce_tracking_update",
                serde_json::to_string(&event).map_err(|e| {
                    IBankError::Serialization(format!("tracking event serialization failed: {e}"))
                })?,
            )
            .await
    }
}

fn dispute_policy_reference(preference: RefundPolicyPreference) -> String {
    match preference {
        RefundPolicyPreference::Flexible => "policy://refund/flexible-v1".to_string(),
        RefundPolicyPreference::Standard => "policy://refund/standard-v1".to_string(),
        RefundPolicyPreference::Strict => "policy://refund/strict-v1".to_string(),
    }
}

fn estimated_fee_for_rail(rail: &str) -> u64 {
    match rail.to_ascii_lowercase().as_str() {
        "ach" => 30,
        "pix" => 18,
        "chain" => 55,
        "card" => 95,
        _ => 60,
    }
}

fn estimated_risk_for_option(rail: &str, region: &str) -> u8 {
    let rail_risk: u8 = match rail.to_ascii_lowercase().as_str() {
        "ach" => 20,
        "pix" => 18,
        "chain" => 30,
        "card" => 35,
        _ => 28,
    };
    let region_risk: u8 = match region.to_ascii_uppercase().as_str() {
        "US" | "CA" | "EU" | "SG" => 8,
        "UNKNOWN" => 20,
        "HIGH_RISK" => 45,
        _ => 15,
    };
    rail_risk.saturating_add(region_risk).min(100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectors::SettlementConnector;
    use crate::policy::RiskPolicyConfig;
    use crate::types::{AccountableWireMessage, ConnectorReceipt, HandleStatus};
    use std::collections::BTreeMap;

    #[derive(Debug)]
    struct TestAchConnector;

    impl SettlementConnector for TestAchConnector {
        fn rail(&self) -> &'static str {
            "ach"
        }

        fn execute(
            &self,
            message: &AccountableWireMessage,
        ) -> Result<ConnectorReceipt, IBankError> {
            Ok(ConnectorReceipt {
                settlement_id: format!("ach-{}", message.message_id),
                rail: "ach".to_string(),
                settled_at: Utc::now(),
                metadata: BTreeMap::new(),
            })
        }
    }

    async fn setup_agent() -> AgenticCommerceAgent {
        let engine = Arc::new(
            IBankEngine::bootstrap(
                RiskPolicyConfig::default(),
                crate::runtime::IBankEngineConfig::default(),
            )
            .await
            .unwrap(),
        );
        engine
            .register_connector(Arc::new(TestAchConnector))
            .unwrap();
        AgenticCommerceAgent::new(engine, CommerceAgentConfig::default())
    }

    fn base_intent() -> CommerceIntent {
        let mut intent = CommerceIntent::new(
            "buyer-1",
            "wireless headset",
            CommerceConstraints::new(50_000, "USD", "US"),
        );
        intent.preferred_rails = vec!["ach".to_string()];
        intent
    }

    #[tokio::test]
    async fn discovery_produces_plan_only() {
        let agent = setup_agent().await;
        let initial_entries = agent.engine.ledger_entries().await.unwrap().len();

        let discovery = agent.discover(base_intent()).await.unwrap();
        assert!(!discovery.draft_plan.options.is_empty());
        assert_eq!(
            discovery.draft_plan.notes[0],
            "discovery_only:no_payment_execution"
        );

        let final_entries = agent.engine.ledger_entries().await.unwrap().len();
        assert_eq!(initial_entries, final_entries);
    }

    #[tokio::test]
    async fn payment_requires_commitment() {
        let agent = setup_agent().await;
        let result = agent.initiate_payment(base_intent(), None).await.unwrap();

        assert!(!result.order.commitment_id.is_empty());
        assert!(result.payment_response.commitment_id.is_some());
        let entries = agent.engine.ledger_entries().await.unwrap();
        assert!(entries
            .iter()
            .any(|entry| { entry.commitment_id.as_deref() == Some(&result.order.commitment_id) }));
    }

    #[tokio::test]
    async fn dispute_always_creates_escalation_case_by_default() {
        let agent = setup_agent().await;
        let payment = agent.initiate_payment(base_intent(), None).await.unwrap();

        let dispute = agent
            .open_dispute(&payment.order.order_id, "item defective")
            .await
            .unwrap();

        assert!(dispute.escalated);
        assert!(dispute.escalation_case.is_some());
        assert_eq!(dispute.response.status, HandleStatus::PendingHumanApproval);
        assert!(agent
            .escalation_case_for_order(&payment.order.order_id)
            .await
            .is_some());
    }
}
