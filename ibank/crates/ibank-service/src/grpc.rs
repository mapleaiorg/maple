use crate::pb::ibank::v1::i_bank_service_server::{IBankService, IBankServiceServer};
use crate::pb::ibank::v1::{
    ApprovePendingRequest, ConfidenceProfileMessage, ExecutionModeProto, HandleRpcRequest,
    HandleRpcResponse, HandleStatusProto, HealthReply, HealthRequest, HumanApprovalMessage,
    IntentRecordMessage, ListPendingRequest, ListPendingResponse, MeaningFieldMessage,
    PendingApprovalMessage, RejectPendingRequest, RejectPendingResponse, RiskBreakdownMessage,
    RiskReportMessage, RouteResultMessage, FILE_DESCRIPTOR_SET,
};
use crate::{
    approve_pending_trace, handle_with_queue, list_pending_items, reject_pending_trace, ApiError,
    ServiceState,
};
use chrono::{DateTime, TimeZone, Utc};
use ibank_core::types::{
    ConfidenceProfile, IntentRecord, MeaningField, RiskBreakdown, RiskReport, RouteResult,
};
use ibank_core::{ExecutionMode, HandleRequest, HandleResponse, HandleStatus, HumanApproval};
use std::collections::{BTreeMap, HashMap};
use tonic::transport::Server;
use tonic::{Request, Response, Status};

#[derive(Clone)]
pub struct GrpcApi {
    state: ServiceState,
}

impl GrpcApi {
    pub fn new(state: ServiceState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl IBankService for GrpcApi {
    async fn health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthReply>, Status> {
        Ok(Response::new(HealthReply {
            status: "ok".to_string(),
            service: "ibank-service".to_string(),
        }))
    }

    async fn handle(
        &self,
        request: Request<HandleRpcRequest>,
    ) -> Result<Response<HandleRpcResponse>, Status> {
        let payload = into_core_handle_request(request.into_inner());
        let response = handle_with_queue(&self.state, payload)
            .await
            .map_err(api_error_to_status)?;
        Ok(Response::new(from_core_handle_response(response)))
    }

    async fn list_pending(
        &self,
        _request: Request<ListPendingRequest>,
    ) -> Result<Response<ListPendingResponse>, Status> {
        let items = list_pending_items(&self.state)
            .await
            .map_err(api_error_to_status)?;
        Ok(Response::new(ListPendingResponse {
            items: items
                .into_iter()
                .map(|item| PendingApprovalMessage {
                    trace_id: item.trace_id,
                    commitment_id: item.commitment_id,
                    decision_reason: item.decision_reason,
                    risk_report: item.risk_report.map(from_core_risk_report),
                    request: Some(from_core_handle_request(item.request)),
                    queued_at_unix_ms: datetime_to_unix_ms(item.queued_at),
                    updated_at_unix_ms: datetime_to_unix_ms(item.updated_at),
                })
                .collect(),
        }))
    }

    async fn approve_pending(
        &self,
        request: Request<ApprovePendingRequest>,
    ) -> Result<Response<HandleRpcResponse>, Status> {
        let cmd = request.into_inner();
        if cmd.trace_id.is_empty() || cmd.approver_id.is_empty() {
            return Err(Status::invalid_argument(
                "trace_id and approver_id are required",
            ));
        }

        let response = approve_pending_trace(&self.state, &cmd.trace_id, cmd.approver_id, cmd.note)
            .await
            .map_err(api_error_to_status)?;

        Ok(Response::new(from_core_handle_response(response)))
    }

    async fn reject_pending(
        &self,
        request: Request<RejectPendingRequest>,
    ) -> Result<Response<RejectPendingResponse>, Status> {
        let cmd = request.into_inner();
        if cmd.trace_id.is_empty() || cmd.approver_id.is_empty() {
            return Err(Status::invalid_argument(
                "trace_id and approver_id are required",
            ));
        }

        let outcome = reject_pending_trace(&self.state, &cmd.trace_id, cmd.approver_id, cmd.note)
            .await
            .map_err(api_error_to_status)?;
        Ok(Response::new(RejectPendingResponse {
            trace_id: outcome.trace_id,
            status: outcome.status.to_string(),
        }))
    }
}

fn api_error_to_status(err: ApiError) -> Status {
    match err {
        ApiError::Http { status, message } => {
            if status == axum::http::StatusCode::NOT_FOUND {
                Status::not_found(message)
            } else if status == axum::http::StatusCode::BAD_REQUEST {
                Status::invalid_argument(message)
            } else {
                Status::internal(message)
            }
        }
        ApiError::Core(err) => Status::internal(err.to_string()),
        ApiError::Queue(err) => Status::internal(err.to_string()),
    }
}

fn into_core_handle_request(req: HandleRpcRequest) -> HandleRequest {
    HandleRequest {
        trace_id: req
            .trace_id
            .and_then(|s| if s.is_empty() { None } else { Some(s) }),
        origin_actor: req.origin_actor,
        counterparty_actor: req.counterparty_actor,
        transaction_type: req.transaction_type,
        amount_minor: req.amount_minor,
        currency: req.currency,
        rail: req.rail,
        destination: req.destination,
        jurisdiction: req.jurisdiction,
        user_intent: req.user_intent,
        ambiguity_hint: req.ambiguity_hint,
        counterparty_risk: req.counterparty_risk.min(100) as u8,
        anomaly_score: req.anomaly_score.min(100) as u8,
        model_uncertainty: req.model_uncertainty,
        compliance_flags: req.compliance_flags,
        metadata: req.metadata.into_iter().collect::<BTreeMap<_, _>>(),
        approval: req.approval.map(into_core_approval),
    }
}

fn from_core_handle_request(req: HandleRequest) -> HandleRpcRequest {
    HandleRpcRequest {
        trace_id: req.trace_id,
        origin_actor: req.origin_actor,
        counterparty_actor: req.counterparty_actor,
        transaction_type: req.transaction_type,
        amount_minor: req.amount_minor,
        currency: req.currency,
        rail: req.rail,
        destination: req.destination,
        jurisdiction: req.jurisdiction,
        user_intent: req.user_intent,
        ambiguity_hint: req.ambiguity_hint,
        counterparty_risk: req.counterparty_risk as u32,
        anomaly_score: req.anomaly_score as u32,
        model_uncertainty: req.model_uncertainty,
        compliance_flags: req.compliance_flags,
        metadata: req.metadata.into_iter().collect::<HashMap<_, _>>(),
        approval: req.approval.map(from_core_approval),
    }
}

fn into_core_approval(approval: HumanApprovalMessage) -> HumanApproval {
    HumanApproval {
        approved: approval.approved,
        approver_id: approval.approver_id,
        note: approval.note,
        approved_at: unix_ms_to_datetime(approval.approved_at_unix_ms),
    }
}

fn from_core_approval(approval: HumanApproval) -> HumanApprovalMessage {
    HumanApprovalMessage {
        approved: approval.approved,
        approver_id: approval.approver_id,
        note: approval.note,
        approved_at_unix_ms: datetime_to_unix_ms(approval.approved_at),
    }
}

fn from_core_handle_response(response: HandleResponse) -> HandleRpcResponse {
    HandleRpcResponse {
        trace_id: response.trace_id,
        commitment_id: response.commitment_id,
        status: from_core_status(response.status) as i32,
        mode: response.mode.map(|m| from_core_mode(m) as i32),
        decision_reason: response.decision_reason,
        meaning: response.meaning.map(from_core_meaning),
        intent: response.intent.map(from_core_intent),
        risk_report: response.risk_report.map(from_core_risk_report),
        route: response.route.map(from_core_route),
    }
}

fn from_core_status(status: HandleStatus) -> HandleStatusProto {
    match status {
        HandleStatus::ExecutedAutonomous => HandleStatusProto::ExecutedAutonomous,
        HandleStatus::ExecutedHybrid => HandleStatusProto::ExecutedHybrid,
        HandleStatus::PendingHumanApproval => HandleStatusProto::PendingHumanApproval,
        HandleStatus::Denied => HandleStatusProto::Denied,
        HandleStatus::Failed => HandleStatusProto::Failed,
    }
}

fn from_core_mode(mode: ExecutionMode) -> ExecutionModeProto {
    match mode {
        ExecutionMode::PureAi => ExecutionModeProto::PureAi,
        ExecutionMode::Hybrid => ExecutionModeProto::Hybrid,
    }
}

fn from_core_meaning(meaning: MeaningField) -> MeaningFieldMessage {
    MeaningFieldMessage {
        summary: meaning.summary,
        inferred_action: meaning.inferred_action,
        ambiguity_notes: meaning.ambiguity_notes,
        ambiguity_score: meaning.ambiguity_score,
        confidence: meaning.confidence,
        formed_at_unix_ms: datetime_to_unix_ms(meaning.formed_at),
    }
}

fn from_core_confidence(conf: ConfidenceProfile) -> ConfidenceProfileMessage {
    ConfidenceProfileMessage {
        meaning_confidence: conf.meaning_confidence,
        model_confidence: conf.model_confidence,
        overall_confidence: conf.overall_confidence,
        blocking_ambiguity: conf.blocking_ambiguity,
        notes: conf.notes,
    }
}

fn from_core_intent(intent: IntentRecord) -> IntentRecordMessage {
    IntentRecordMessage {
        objective: intent.objective,
        rationale: intent.rationale,
        confidence: Some(from_core_confidence(intent.confidence)),
        stabilized_at_unix_ms: datetime_to_unix_ms(intent.stabilized_at),
    }
}

fn from_core_risk_breakdown(breakdown: RiskBreakdown) -> RiskBreakdownMessage {
    RiskBreakdownMessage {
        amount: breakdown.amount as u32,
        counterparty: breakdown.counterparty as u32,
        jurisdiction: breakdown.jurisdiction as u32,
        anomaly: breakdown.anomaly as u32,
        model_uncertainty: breakdown.model_uncertainty as u32,
    }
}

fn from_core_risk_report(report: RiskReport) -> RiskReportMessage {
    RiskReportMessage {
        score: report.score as u32,
        reasons: report.reasons,
        factors: Some(from_core_risk_breakdown(report.factors)),
        fraud_score: report.fraud_score as u32,
        blocking_ambiguity: report.blocking_ambiguity,
        requires_hybrid: report.requires_hybrid,
        denied: report.denied,
    }
}

fn from_core_route(route: RouteResult) -> RouteResultMessage {
    RouteResultMessage {
        connector: route.connector,
        external_reference: route.external_reference,
        settled_at_unix_ms: datetime_to_unix_ms(route.settled_at),
    }
}

fn datetime_to_unix_ms(dt: DateTime<Utc>) -> i64 {
    dt.timestamp_millis()
}

fn unix_ms_to_datetime(ms: i64) -> DateTime<Utc> {
    Utc.timestamp_millis_opt(ms)
        .single()
        .unwrap_or_else(Utc::now)
}

pub async fn serve_grpc(state: ServiceState, addr: std::net::SocketAddr) -> anyhow::Result<()> {
    let service = IBankServiceServer::new(GrpcApi::new(state));
    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build_v1()?;

    Server::builder()
        .add_service(reflection)
        .add_service(service)
        .serve(addr)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ServiceConfig;

    #[test]
    fn descriptor_set_is_embedded() {
        assert!(!FILE_DESCRIPTOR_SET.is_empty());
    }

    #[tokio::test]
    async fn grpc_handle_returns_pending_for_large_transfer() {
        let path = std::env::temp_dir()
            .join(format!("ibank-grpc-{}", uuid::Uuid::new_v4()))
            .join("approvals.json");
        let state = ServiceState::bootstrap(ServiceConfig {
            queue_path: path,
            ledger_storage: ibank_core::LedgerStorageConfig::Memory,
        })
        .await
        .unwrap();

        let api = GrpcApi::new(state);

        let response = api
            .handle(Request::new(HandleRpcRequest {
                trace_id: None,
                origin_actor: "issuer-a".to_string(),
                counterparty_actor: "merchant-b".to_string(),
                transaction_type: "transfer".to_string(),
                amount_minor: 1_500_000,
                currency: "USD".to_string(),
                rail: "ach".to_string(),
                destination: "acct-123".to_string(),
                jurisdiction: "US".to_string(),
                user_intent: "move treasury funds".to_string(),
                ambiguity_hint: Some(0.1),
                counterparty_risk: 10,
                anomaly_score: 10,
                model_uncertainty: 0.1,
                compliance_flags: vec![],
                metadata: HashMap::new(),
                approval: None,
            }))
            .await
            .unwrap();

        assert_eq!(
            response.into_inner().status,
            HandleStatusProto::PendingHumanApproval as i32
        );
    }
}
