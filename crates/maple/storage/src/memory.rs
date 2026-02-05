//! In-memory reference implementation for MAPLE storage traits.
//!
//! This adapter is deterministic and test-friendly. Production deployments
//! should use a transactional backend (e.g. PostgreSQL) for source-of-truth data.

use crate::model::{
    AgentCheckpoint, AuditAppend, AuditRecord, CommitmentRecord, ProjectionSnapshot, SemanticHit,
    SemanticRecord,
};
use crate::traits::{
    AgentStateStore, AuditStore, CommitmentStore, ProjectionStore, QueryWindow, SemanticMemoryStore,
};
use crate::{StorageError, StorageResult};
use aas_types::{CommitmentOutcome, Decision, LifecycleStatus, PolicyDecisionCard};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rcf_commitment::{CommitmentId, RcfCommitment};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

/// In-memory MAPLE storage adapter.
#[derive(Default)]
pub struct InMemoryMapleStorage {
    commitments: RwLock<HashMap<CommitmentId, CommitmentRecord>>,
    audits: RwLock<Vec<AuditRecord>>,
    checkpoints: RwLock<HashMap<String, AgentCheckpoint>>,
    projections: RwLock<HashMap<(String, String), ProjectionSnapshot>>,
    semantic: RwLock<HashMap<(String, String), SemanticRecord>>,
}

impl InMemoryMapleStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl CommitmentStore for InMemoryMapleStorage {
    async fn create_commitment(
        &self,
        commitment: RcfCommitment,
        decision: PolicyDecisionCard,
        declared_at: DateTime<Utc>,
    ) -> StorageResult<()> {
        let mut guard = self
            .commitments
            .write()
            .map_err(|_| StorageError::Backend("commitments lock poisoned".to_string()))?;

        if guard.contains_key(&commitment.commitment_id) {
            return Err(StorageError::Conflict(format!(
                "commitment {} already exists",
                commitment.commitment_id
            )));
        }

        let lifecycle_status = decision_to_lifecycle(decision.decision);
        let record = CommitmentRecord {
            commitment_id: commitment.commitment_id.clone(),
            commitment,
            decision,
            lifecycle_status,
            outcome: None,
            created_at: declared_at,
            updated_at: declared_at,
        };
        guard.insert(record.commitment_id.clone(), record);
        Ok(())
    }

    async fn transition_lifecycle(
        &self,
        commitment_id: &CommitmentId,
        expected_from: LifecycleStatus,
        to: LifecycleStatus,
        updated_at: DateTime<Utc>,
    ) -> StorageResult<()> {
        let mut guard = self
            .commitments
            .write()
            .map_err(|_| StorageError::Backend("commitments lock poisoned".to_string()))?;
        let record = guard.get_mut(commitment_id).ok_or_else(|| {
            StorageError::NotFound(format!("commitment {} not found", commitment_id))
        })?;

        if record.lifecycle_status != expected_from {
            return Err(StorageError::InvariantViolation(format!(
                "invalid lifecycle transition: expected {:?}, found {:?}",
                expected_from, record.lifecycle_status
            )));
        }

        record.lifecycle_status = to;
        record.updated_at = updated_at;
        Ok(())
    }

    async fn set_outcome(
        &self,
        commitment_id: &CommitmentId,
        outcome: CommitmentOutcome,
        final_status: LifecycleStatus,
    ) -> StorageResult<()> {
        let mut guard = self
            .commitments
            .write()
            .map_err(|_| StorageError::Backend("commitments lock poisoned".to_string()))?;
        let record = guard.get_mut(commitment_id).ok_or_else(|| {
            StorageError::NotFound(format!("commitment {} not found", commitment_id))
        })?;
        record.outcome = Some(outcome);
        record.lifecycle_status = final_status;
        record.updated_at = Utc::now();
        Ok(())
    }

    async fn get_commitment(
        &self,
        commitment_id: &CommitmentId,
    ) -> StorageResult<Option<CommitmentRecord>> {
        let guard = self
            .commitments
            .read()
            .map_err(|_| StorageError::Backend("commitments lock poisoned".to_string()))?;
        Ok(guard.get(commitment_id).cloned())
    }

    async fn list_commitments(&self, window: QueryWindow) -> StorageResult<Vec<CommitmentRecord>> {
        let guard = self
            .commitments
            .read()
            .map_err(|_| StorageError::Backend("commitments lock poisoned".to_string()))?;
        let mut values = guard.values().cloned().collect::<Vec<_>>();
        values.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(apply_window(values, window))
    }
}

fn decision_to_lifecycle(decision: Decision) -> LifecycleStatus {
    match decision {
        Decision::Approved => LifecycleStatus::Approved,
        Decision::Denied => LifecycleStatus::Denied,
        Decision::PendingHumanReview | Decision::PendingAdditionalInfo => LifecycleStatus::Pending,
    }
}

#[async_trait]
impl AuditStore for InMemoryMapleStorage {
    async fn append_audit(&self, event: AuditAppend) -> StorageResult<AuditRecord> {
        let mut guard = self
            .audits
            .write()
            .map_err(|_| StorageError::Backend("audit lock poisoned".to_string()))?;

        let previous_hash = guard.last().map(|e| e.hash.clone());
        let sequence = guard.len() as u64 + 1;
        let hash = compute_audit_hash(&event, previous_hash.as_deref(), sequence)?;

        let record = AuditRecord {
            event_id: format!("audit-{}", Uuid::new_v4()),
            sequence,
            timestamp: event.timestamp,
            actor: event.actor,
            stage: event.stage,
            success: event.success,
            message: event.message,
            commitment_id: event.commitment_id,
            payload: event.payload,
            previous_hash,
            hash,
        };

        guard.push(record.clone());
        Ok(record)
    }

    async fn list_audit(&self, window: QueryWindow) -> StorageResult<Vec<AuditRecord>> {
        let guard = self
            .audits
            .read()
            .map_err(|_| StorageError::Backend("audit lock poisoned".to_string()))?;
        let mut values = guard.clone();
        values.sort_by(|a, b| b.sequence.cmp(&a.sequence));
        Ok(apply_window(values, window))
    }

    async fn latest_audit_hash(&self) -> StorageResult<Option<String>> {
        let guard = self
            .audits
            .read()
            .map_err(|_| StorageError::Backend("audit lock poisoned".to_string()))?;
        Ok(guard.last().map(|e| e.hash.clone()))
    }
}

#[async_trait]
impl AgentStateStore for InMemoryMapleStorage {
    async fn upsert_checkpoint(&self, checkpoint: AgentCheckpoint) -> StorageResult<()> {
        let mut guard = self
            .checkpoints
            .write()
            .map_err(|_| StorageError::Backend("checkpoint lock poisoned".to_string()))?;
        guard.insert(checkpoint.resonator_id.clone(), checkpoint);
        Ok(())
    }

    async fn get_checkpoint(&self, resonator_id: &str) -> StorageResult<Option<AgentCheckpoint>> {
        let guard = self
            .checkpoints
            .read()
            .map_err(|_| StorageError::Backend("checkpoint lock poisoned".to_string()))?;
        Ok(guard.get(resonator_id).cloned())
    }

    async fn list_checkpoints(&self, window: QueryWindow) -> StorageResult<Vec<AgentCheckpoint>> {
        let guard = self
            .checkpoints
            .read()
            .map_err(|_| StorageError::Backend("checkpoint lock poisoned".to_string()))?;
        let mut values = guard.values().cloned().collect::<Vec<_>>();
        values.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(apply_window(values, window))
    }
}

#[async_trait]
impl ProjectionStore for InMemoryMapleStorage {
    async fn upsert_projection(&self, snapshot: ProjectionSnapshot) -> StorageResult<()> {
        let mut guard = self
            .projections
            .write()
            .map_err(|_| StorageError::Backend("projection lock poisoned".to_string()))?;
        guard.insert((snapshot.namespace.clone(), snapshot.key.clone()), snapshot);
        Ok(())
    }

    async fn get_projection(
        &self,
        namespace: &str,
        key: &str,
    ) -> StorageResult<Option<ProjectionSnapshot>> {
        let guard = self
            .projections
            .read()
            .map_err(|_| StorageError::Backend("projection lock poisoned".to_string()))?;
        Ok(guard
            .get(&(namespace.to_string(), key.to_string()))
            .cloned())
    }

    async fn list_projections(
        &self,
        namespace: &str,
        window: QueryWindow,
    ) -> StorageResult<Vec<ProjectionSnapshot>> {
        let guard = self
            .projections
            .read()
            .map_err(|_| StorageError::Backend("projection lock poisoned".to_string()))?;
        let mut values = guard
            .values()
            .filter(|item| item.namespace == namespace)
            .cloned()
            .collect::<Vec<_>>();
        values.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(apply_window(values, window))
    }
}

#[async_trait]
impl SemanticMemoryStore for InMemoryMapleStorage {
    async fn upsert_semantic(&self, record: SemanticRecord) -> StorageResult<()> {
        let mut guard = self
            .semantic
            .write()
            .map_err(|_| StorageError::Backend("semantic lock poisoned".to_string()))?;
        guard.insert((record.namespace.clone(), record.record_id.clone()), record);
        Ok(())
    }

    async fn search_semantic(
        &self,
        namespace: &str,
        query_embedding: &[f32],
        limit: usize,
    ) -> StorageResult<Vec<SemanticHit>> {
        if query_embedding.is_empty() {
            return Err(StorageError::InvalidInput(
                "query embedding must not be empty".to_string(),
            ));
        }

        let guard = self
            .semantic
            .read()
            .map_err(|_| StorageError::Backend("semantic lock poisoned".to_string()))?;

        let mut hits = guard
            .values()
            .filter(|record| record.namespace == namespace)
            .filter_map(|record| {
                cosine_similarity(query_embedding, &record.embedding).map(|score| SemanticHit {
                    record: record.clone(),
                    score,
                })
            })
            .collect::<Vec<_>>();

        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        if limit > 0 {
            hits.truncate(limit);
        }
        Ok(hits)
    }
}

fn compute_audit_hash(
    event: &AuditAppend,
    previous_hash: Option<&str>,
    sequence: u64,
) -> StorageResult<String> {
    let serializable = serde_json::json!({
        "previous_hash": previous_hash,
        "sequence": sequence,
        "timestamp": event.timestamp,
        "actor": event.actor,
        "stage": event.stage,
        "success": event.success,
        "message": event.message,
        "commitment_id": event.commitment_id.as_ref().map(|id| id.0.clone()),
        "payload": event.payload,
    });
    let serialized = serde_json::to_vec(&serializable)
        .map_err(|e| StorageError::Serialization(e.to_string()))?;
    Ok(blake3::hash(&serialized).to_hex().to_string())
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> Option<f32> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }

    let (mut dot, mut norm_a, mut norm_b) = (0.0_f32, 0.0_f32, 0.0_f32);
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        return None;
    }
    Some(dot / (norm_a.sqrt() * norm_b.sqrt()))
}

fn apply_window<T>(items: Vec<T>, window: QueryWindow) -> Vec<T> {
    let iter = items.into_iter().skip(window.offset);
    if window.limit == 0 {
        iter.collect()
    } else {
        iter.take(window.limit).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aas_types::{
        AdjudicatorInfo, AdjudicatorType, Decision, DecisionId, PolicyDecisionCard, Rationale,
        RiskAssessment, RiskLevel,
    };
    use chrono::Duration;
    use rcf_commitment::CommitmentBuilder;
    use rcf_types::{EffectDomain, IdentityRef, ScopeConstraint};

    #[tokio::test]
    async fn audit_chain_hashes_are_linked() {
        let storage = InMemoryMapleStorage::new();
        let first = storage
            .append_audit(AuditAppend {
                timestamp: Utc::now(),
                actor: "agent-a".to_string(),
                stage: "meaning".to_string(),
                success: true,
                message: "formed meaning".to_string(),
                commitment_id: None,
                payload: serde_json::json!({"x": 1}),
            })
            .await
            .unwrap();
        let second = storage
            .append_audit(AuditAppend {
                timestamp: Utc::now() + Duration::seconds(1),
                actor: "agent-a".to_string(),
                stage: "intent".to_string(),
                success: true,
                message: "stabilized intent".to_string(),
                commitment_id: None,
                payload: serde_json::json!({"y": 2}),
            })
            .await
            .unwrap();

        assert_eq!(second.previous_hash, Some(first.hash));
    }

    #[tokio::test]
    async fn lifecycle_transition_checks_expected_state() {
        let storage = InMemoryMapleStorage::new();
        let commitment =
            CommitmentBuilder::new(IdentityRef::new("agent-a"), EffectDomain::Computation)
                .with_scope(ScopeConstraint::global())
                .build()
                .unwrap();
        let id = commitment.commitment_id.clone();

        storage
            .create_commitment(commitment, sample_decision(id.clone()), Utc::now())
            .await
            .unwrap();

        let result = storage
            .transition_lifecycle(
                &id,
                LifecycleStatus::Pending,
                LifecycleStatus::Executing,
                Utc::now(),
            )
            .await;
        assert!(matches!(result, Err(StorageError::InvariantViolation(_))));
    }

    #[tokio::test]
    async fn semantic_search_is_deterministic() {
        let storage = InMemoryMapleStorage::new();
        storage
            .upsert_semantic(SemanticRecord {
                namespace: "ibank".to_string(),
                record_id: "a".to_string(),
                embedding: vec![1.0, 0.0, 0.0],
                content: "transfer policy".to_string(),
                metadata: serde_json::json!({}),
                created_at: Utc::now(),
            })
            .await
            .unwrap();
        storage
            .upsert_semantic(SemanticRecord {
                namespace: "ibank".to_string(),
                record_id: "b".to_string(),
                embedding: vec![0.1, 0.9, 0.0],
                content: "shipping event".to_string(),
                metadata: serde_json::json!({}),
                created_at: Utc::now(),
            })
            .await
            .unwrap();

        let hits = storage
            .search_semantic("ibank", &[0.9, 0.1, 0.0], 1)
            .await
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].record.record_id, "a");
    }

    fn sample_decision(commitment_id: CommitmentId) -> PolicyDecisionCard {
        PolicyDecisionCard {
            decision_id: DecisionId::generate(),
            commitment_id,
            decision: Decision::Approved,
            rationale: Rationale {
                summary: "approved".to_string(),
                rule_references: vec![],
            },
            risk_assessment: RiskAssessment {
                overall_risk: RiskLevel::Low,
                risk_factors: vec![],
                mitigations: vec![],
            },
            conditions: vec![],
            approval_expiration: None,
            decided_at: Utc::now(),
            adjudicator: AdjudicatorInfo {
                adjudicator_type: AdjudicatorType::Automated,
                adjudicator_id: "system".to_string(),
            },
        }
    }
}
