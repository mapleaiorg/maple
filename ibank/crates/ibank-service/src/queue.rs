use chrono::{DateTime, Utc};
use ibank_core::{
    AttestationDecision, EscalationCase, EscalationWorkflowState, HandleRequest, HandleResponse,
    HumanAttestation, RiskReport,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("approval queue IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("approval queue serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("workflow transition error: {0}")]
    WorkflowTransition(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApproval {
    pub trace_id: String,
    pub commitment_id: Option<String>,
    pub decision_reason: String,
    pub risk_report: Option<RiskReport>,
    pub request: HandleRequest,
    #[serde(default)]
    pub escalation_case: Option<EscalationCase>,
    #[serde(default = "default_workflow_state")]
    pub workflow_state: EscalationWorkflowState,
    #[serde(default)]
    pub human_attestation: Option<HumanAttestation>,
    #[serde(default)]
    pub attestation_history: Vec<HumanAttestation>,
    pub queued_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_workflow_state() -> EscalationWorkflowState {
    EscalationWorkflowState::Open
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkflowEvent {
    BeginReview,
    Approve,
    Modify,
    Deny,
    MarkExecuted,
    Close,
}

#[derive(Debug, Default)]
struct EscalationWorkflowEngine;

impl EscalationWorkflowEngine {
    fn transition(
        &self,
        state: EscalationWorkflowState,
        event: WorkflowEvent,
    ) -> Result<EscalationWorkflowState, QueueError> {
        let next = match (state, event) {
            (EscalationWorkflowState::Open, WorkflowEvent::BeginReview) => {
                EscalationWorkflowState::InReview
            }
            (EscalationWorkflowState::InReview, WorkflowEvent::Approve)
            | (EscalationWorkflowState::InReview, WorkflowEvent::Modify) => {
                EscalationWorkflowState::Approved
            }
            (EscalationWorkflowState::InReview, WorkflowEvent::Deny) => {
                EscalationWorkflowState::Denied
            }
            (EscalationWorkflowState::Approved, WorkflowEvent::MarkExecuted) => {
                EscalationWorkflowState::Executed
            }
            (EscalationWorkflowState::Denied, WorkflowEvent::Close)
            | (EscalationWorkflowState::Executed, WorkflowEvent::Close)
            | (EscalationWorkflowState::Open, WorkflowEvent::Close)
            | (EscalationWorkflowState::InReview, WorkflowEvent::Close)
            | (EscalationWorkflowState::Approved, WorkflowEvent::Close) => {
                EscalationWorkflowState::Closed
            }
            (current, action) => {
                return Err(QueueError::WorkflowTransition(format!(
                    "invalid transition: state={current:?} event={action:?}"
                )));
            }
        };
        Ok(next)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct QueueData {
    entries: BTreeMap<String, PendingApproval>,
}

/// File-backed approval queue used by hybrid review workflows.
///
/// The queue is persisted after every mutation so pending approvals survive service restarts.
#[derive(Debug)]
pub struct PersistedApprovalQueue {
    path: PathBuf,
    data: QueueData,
    workflow: EscalationWorkflowEngine,
}

impl PersistedApprovalQueue {
    pub fn load(path: impl Into<PathBuf>) -> Result<Self, QueueError> {
        let path = path.into();
        let data = if path.exists() {
            let bytes = fs::read(&path)?;
            if bytes.is_empty() {
                QueueData::default()
            } else {
                serde_json::from_slice(&bytes)?
            }
        } else {
            QueueData::default()
        };

        Ok(Self {
            path,
            data,
            workflow: EscalationWorkflowEngine,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn upsert_from_response(
        &mut self,
        mut request: HandleRequest,
        response: &HandleResponse,
    ) -> Result<(), QueueError> {
        request.trace_id = Some(response.trace_id.clone());
        let now = Utc::now();
        let trace_id = response.trace_id.clone();

        let previous = self.data.entries.get(&trace_id).cloned();
        let queued_at = previous
            .as_ref()
            .map(|entry| entry.queued_at)
            .unwrap_or(now);

        let case_id = previous
            .as_ref()
            .and_then(|entry| {
                entry
                    .escalation_case
                    .as_ref()
                    .map(|case_| case_.case_id.clone())
            })
            .unwrap_or_else(|| format!("case-{}", uuid::Uuid::new_v4()));
        let workflow_state = previous
            .as_ref()
            .map(|entry| match entry.workflow_state {
                EscalationWorkflowState::InReview => EscalationWorkflowState::InReview,
                _ => EscalationWorkflowState::Open,
            })
            .unwrap_or(EscalationWorkflowState::Open);
        let previous_human_attestation = previous
            .as_ref()
            .and_then(|entry| entry.human_attestation.clone());
        let previous_attestation_history = previous
            .as_ref()
            .map(|entry| entry.attestation_history.clone())
            .unwrap_or_default();

        self.data.entries.insert(
            trace_id.clone(),
            PendingApproval {
                trace_id: trace_id.clone(),
                commitment_id: response.commitment_id.clone(),
                decision_reason: response.decision_reason.clone(),
                risk_report: response.risk_report.clone(),
                request: request.clone(),
                escalation_case: Some(build_escalation_case(
                    case_id,
                    response.commitment_id.clone(),
                    response.risk_report.clone(),
                    &request,
                    &response.decision_reason,
                )),
                workflow_state,
                human_attestation: previous_human_attestation,
                attestation_history: previous_attestation_history,
                queued_at,
                updated_at: now,
            },
        );

        self.persist()
    }

    pub fn get(&self, trace_id: &str) -> Option<&PendingApproval> {
        self.data.entries.get(trace_id)
    }

    pub fn begin_review(&mut self, trace_id: &str) -> Result<(), QueueError> {
        let entry = self.data.entries.get_mut(trace_id).ok_or_else(|| {
            QueueError::WorkflowTransition(format!("case '{trace_id}' not found"))
        })?;
        entry.workflow_state = self
            .workflow
            .transition(entry.workflow_state.clone(), WorkflowEvent::BeginReview)?;
        entry.updated_at = Utc::now();
        self.persist()
    }

    pub fn record_attestation(
        &mut self,
        trace_id: &str,
        attestation: HumanAttestation,
    ) -> Result<PendingApproval, QueueError> {
        let updated = {
            let entry = self.data.entries.get_mut(trace_id).ok_or_else(|| {
                QueueError::WorkflowTransition(format!("case '{trace_id}' not found"))
            })?;

            let event = match attestation.decision {
                AttestationDecision::Approve => WorkflowEvent::Approve,
                AttestationDecision::Deny => WorkflowEvent::Deny,
                AttestationDecision::Modify => WorkflowEvent::Modify,
            };
            entry.workflow_state = self
                .workflow
                .transition(entry.workflow_state.clone(), event)?;
            entry.human_attestation = Some(attestation.clone());
            entry.attestation_history.push(attestation);
            entry.updated_at = Utc::now();
            entry.clone()
        };
        self.persist()?;
        Ok(updated)
    }

    pub fn mark_executed(&mut self, trace_id: &str) -> Result<(), QueueError> {
        if let Some(entry) = self.data.entries.get_mut(trace_id) {
            entry.workflow_state = self
                .workflow
                .transition(entry.workflow_state.clone(), WorkflowEvent::MarkExecuted)?;
            entry.updated_at = Utc::now();
            self.persist()?;
        }
        Ok(())
    }

    pub fn close_case(&mut self, trace_id: &str) -> Result<Option<PendingApproval>, QueueError> {
        if let Some(entry) = self.data.entries.get_mut(trace_id) {
            entry.workflow_state = self
                .workflow
                .transition(entry.workflow_state.clone(), WorkflowEvent::Close)?;
            entry.updated_at = Utc::now();
        }
        let removed = self.data.entries.remove(trace_id);
        self.persist()?;
        Ok(removed)
    }

    pub fn remove(&mut self, trace_id: &str) -> Result<Option<PendingApproval>, QueueError> {
        self.close_case(trace_id)
    }

    pub fn list(&self) -> Vec<PendingApproval> {
        let mut values: Vec<PendingApproval> = self.data.entries.values().cloned().collect();
        values.sort_by_key(|item| item.queued_at);
        values
    }

    fn persist(&self) -> Result<(), QueueError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let bytes = serde_json::to_vec_pretty(&self.data)?;
        let tmp_path = self.path.with_extension("tmp");
        fs::write(&tmp_path, bytes)?;
        fs::rename(tmp_path, &self.path)?;
        Ok(())
    }
}

fn build_escalation_case(
    case_id: String,
    commitment_id: Option<String>,
    risk_report: Option<RiskReport>,
    request: &HandleRequest,
    decision_reason: &str,
) -> EscalationCase {
    let mut evidence_bundle = vec![
        format!("trace_id={}", request.trace_id.clone().unwrap_or_default()),
        format!("decision_reason={decision_reason}"),
        format!("transaction_type={}", request.transaction_type),
        format!("amount_minor={}", request.amount_minor),
        format!("jurisdiction={}", request.jurisdiction),
    ];

    evidence_bundle.extend(
        request
            .metadata
            .iter()
            .filter(|(key, _)| key.starts_with("evidence_"))
            .map(|(key, value)| format!("{key}={value}")),
    );

    if !request.compliance_flags.is_empty() {
        evidence_bundle.push(format!(
            "compliance_flags={}",
            request.compliance_flags.join(",")
        ));
    }

    if let Some(report) = &risk_report {
        evidence_bundle.push(format!("risk_score={}", report.score));
        evidence_bundle.push(format!("fraud_score={}", report.fraud_score));
        if !report.reasons.is_empty() {
            evidence_bundle.push(format!("risk_reasons={}", report.reasons.join("|")));
        }
    }

    evidence_bundle.sort();
    evidence_bundle.dedup();

    let mut recommended_actions = vec![
        "obtain_signed_attestation(approve|deny|modify)".to_string(),
        "verify_commitment_scope_against_request".to_string(),
    ];

    if let Some(report) = &risk_report {
        if report.fraud_score >= 70 {
            recommended_actions.push("run_enhanced_fraud_review".to_string());
        }
        if report.blocking_ambiguity {
            recommended_actions.push("clarify_intent_before_execution".to_string());
        }
        if report.score >= 80 {
            recommended_actions.push("escalate_to_senior_approver".to_string());
        }
    }
    if !request.compliance_flags.is_empty() {
        recommended_actions.push("complete_additional_compliance_checks".to_string());
    }
    recommended_actions.sort();
    recommended_actions.dedup();

    EscalationCase {
        case_id,
        commitment_id,
        risk_report,
        evidence_bundle,
        recommended_actions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ibank_core::{AttestationDecision, EscalationWorkflowState, HumanAttestation};
    use ibank_core::{HandleStatus, MeaningField};
    use std::collections::BTreeMap;
    use uuid::Uuid;

    #[test]
    fn queue_persists_across_reload() {
        let dir = std::env::temp_dir().join(format!("ibank-queue-{}", Uuid::new_v4()));
        let path = dir.join("approvals.json");

        let mut queue = PersistedApprovalQueue::load(&path).unwrap();
        let mut request = HandleRequest::new("a", "b", 100, "USD", "ach", "acct", "pay");
        request.metadata = BTreeMap::new();

        let response = HandleResponse {
            trace_id: "trace-1".to_string(),
            commitment_id: Some("commit-1".to_string()),
            status: HandleStatus::PendingHumanApproval,
            mode: None,
            decision_reason: "hybrid required".to_string(),
            meaning: Some(MeaningField {
                summary: "s".to_string(),
                inferred_action: "transfer".to_string(),
                ambiguity_notes: vec![],
                ambiguity_score: 0.4,
                confidence: 0.6,
                formed_at: Utc::now(),
            }),
            intent: None,
            risk_report: None,
            route: None,
        };

        queue.upsert_from_response(request, &response).unwrap();

        let reloaded = PersistedApprovalQueue::load(&path).unwrap();
        assert_eq!(reloaded.list().len(), 1);
        assert!(reloaded.get("trace-1").is_some());
        assert_eq!(
            reloaded
                .get("trace-1")
                .map(|entry| entry.workflow_state.clone()),
            Some(EscalationWorkflowState::Open)
        );
    }

    #[test]
    fn workflow_engine_transitions_open_to_closed() {
        let dir = std::env::temp_dir().join(format!("ibank-workflow-{}", Uuid::new_v4()));
        let path = dir.join("approvals.json");

        let mut queue = PersistedApprovalQueue::load(&path).unwrap();
        let request = HandleRequest::new("a", "b", 1_500_000, "USD", "ach", "acct", "review");
        let response = HandleResponse {
            trace_id: "trace-2".to_string(),
            commitment_id: Some("commit-2".to_string()),
            status: HandleStatus::PendingHumanApproval,
            mode: None,
            decision_reason: "hybrid required".to_string(),
            meaning: Some(MeaningField {
                summary: "s".to_string(),
                inferred_action: "transfer".to_string(),
                ambiguity_notes: vec![],
                ambiguity_score: 0.4,
                confidence: 0.6,
                formed_at: Utc::now(),
            }),
            intent: None,
            risk_report: None,
            route: None,
        };

        queue.upsert_from_response(request, &response).unwrap();
        queue.begin_review("trace-2").unwrap();
        let case = queue
            .record_attestation(
                "trace-2",
                HumanAttestation {
                    attestation_id: "a1".to_string(),
                    decision: AttestationDecision::Approve,
                    signer_id: "human-1".to_string(),
                    signature: "sig".to_string(),
                    anchor: "anchor://1".to_string(),
                    attested_at: Utc::now(),
                    constraints: vec![],
                    note: None,
                },
            )
            .unwrap();

        assert_eq!(case.workflow_state, EscalationWorkflowState::Approved);
        queue.mark_executed("trace-2").unwrap();
        let removed = queue.close_case("trace-2").unwrap();
        assert!(removed.is_some());
        assert!(queue.get("trace-2").is_none());
    }
}
