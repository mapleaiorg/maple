use chrono::{DateTime, Utc};
use ibank_core::{HandleRequest, HandleResponse, RiskReport};
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApproval {
    pub trace_id: String,
    pub commitment_id: Option<String>,
    pub decision_reason: String,
    pub risk_report: Option<RiskReport>,
    pub request: HandleRequest,
    pub queued_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

        Ok(Self { path, data })
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

        self.data.entries.insert(
            trace_id.clone(),
            PendingApproval {
                trace_id,
                commitment_id: response.commitment_id.clone(),
                decision_reason: response.decision_reason.clone(),
                risk_report: response.risk_report.clone(),
                request,
                queued_at,
                updated_at: now,
            },
        );

        self.persist()
    }

    pub fn get(&self, trace_id: &str) -> Option<&PendingApproval> {
        self.data.entries.get(trace_id)
    }

    pub fn remove(&mut self, trace_id: &str) -> Result<Option<PendingApproval>, QueueError> {
        let removed = self.data.entries.remove(trace_id);
        self.persist()?;
        Ok(removed)
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

#[cfg(test)]
mod tests {
    use super::*;
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
    }
}
