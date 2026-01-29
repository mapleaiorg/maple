//! EVE Artifacts - Learning artifact storage and retrieval
//!
//! Stores and manages learning artifacts produced by EVE.

#![deny(unsafe_code)]

use eve_types::{ArtifactId, ArtifactQuery, ArtifactType, LearningArtifact};
use rcl_types::EffectDomain;
use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;

/// Artifact store for learning artifacts
pub struct ArtifactStore {
    artifacts: RwLock<HashMap<ArtifactId, LearningArtifact>>,
    domain_index: RwLock<HashMap<EffectDomain, Vec<ArtifactId>>>,
    type_index: RwLock<HashMap<ArtifactType, Vec<ArtifactId>>>,
}

impl ArtifactStore {
    pub fn new() -> Self {
        Self {
            artifacts: RwLock::new(HashMap::new()),
            domain_index: RwLock::new(HashMap::new()),
            type_index: RwLock::new(HashMap::new()),
        }
    }

    /// Store a learning artifact
    pub fn store(&self, artifact: LearningArtifact) -> Result<ArtifactId, ArtifactStoreError> {
        let id = artifact.artifact_id.clone();

        // Store artifact
        let mut artifacts = self.artifacts.write().map_err(|_| ArtifactStoreError::LockError)?;
        artifacts.insert(id.clone(), artifact.clone());

        // Update domain index
        let mut domain_index = self.domain_index.write().map_err(|_| ArtifactStoreError::LockError)?;
        domain_index
            .entry(artifact.domain.clone())
            .or_default()
            .push(id.clone());

        // Update type index
        let mut type_index = self.type_index.write().map_err(|_| ArtifactStoreError::LockError)?;
        type_index
            .entry(artifact.artifact_type.clone())
            .or_default()
            .push(id.clone());

        Ok(id)
    }

    /// Get an artifact by ID
    pub fn get(&self, id: &ArtifactId) -> Result<Option<LearningArtifact>, ArtifactStoreError> {
        let artifacts = self.artifacts.read().map_err(|_| ArtifactStoreError::LockError)?;
        Ok(artifacts.get(id).cloned())
    }

    /// Query artifacts
    pub fn query(&self, query: ArtifactQuery) -> Result<Vec<LearningArtifact>, ArtifactStoreError> {
        let artifacts = self.artifacts.read().map_err(|_| ArtifactStoreError::LockError)?;

        let mut results: Vec<_> = artifacts
            .values()
            .filter(|a| {
                // Filter by type
                if let Some(ref artifact_type) = query.artifact_type {
                    if &a.artifact_type != artifact_type {
                        return false;
                    }
                }

                // Filter by domain
                if let Some(ref domain) = query.domain {
                    if &a.domain != domain {
                        return false;
                    }
                }

                // Filter by confidence
                if let Some(min_confidence) = query.min_confidence {
                    if a.confidence.score < min_confidence {
                        return false;
                    }
                }

                // Filter by time
                if let Some(after) = query.after {
                    if a.created_at < after {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Sort by creation time (newest first)
        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Apply limit
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// Get artifacts by domain
    pub fn get_by_domain(
        &self,
        domain: &EffectDomain,
    ) -> Result<Vec<LearningArtifact>, ArtifactStoreError> {
        let domain_index = self.domain_index.read().map_err(|_| ArtifactStoreError::LockError)?;
        let artifacts = self.artifacts.read().map_err(|_| ArtifactStoreError::LockError)?;

        let ids = match domain_index.get(domain) {
            Some(ids) => ids,
            None => return Ok(vec![]),
        };

        Ok(ids.iter().filter_map(|id| artifacts.get(id).cloned()).collect())
    }

    /// Get artifacts by type
    pub fn get_by_type(
        &self,
        artifact_type: &ArtifactType,
    ) -> Result<Vec<LearningArtifact>, ArtifactStoreError> {
        let type_index = self.type_index.read().map_err(|_| ArtifactStoreError::LockError)?;
        let artifacts = self.artifacts.read().map_err(|_| ArtifactStoreError::LockError)?;

        let ids = match type_index.get(artifact_type) {
            Some(ids) => ids,
            None => return Ok(vec![]),
        };

        Ok(ids.iter().filter_map(|id| artifacts.get(id).cloned()).collect())
    }

    /// Get statistics
    pub fn statistics(&self) -> Result<ArtifactStoreStats, ArtifactStoreError> {
        let artifacts = self.artifacts.read().map_err(|_| ArtifactStoreError::LockError)?;

        let total = artifacts.len();
        let mut by_type: HashMap<String, usize> = HashMap::new();
        let mut by_domain: HashMap<String, usize> = HashMap::new();

        for artifact in artifacts.values() {
            *by_type.entry(format!("{:?}", artifact.artifact_type)).or_insert(0) += 1;
            *by_domain.entry(format!("{}", artifact.domain)).or_insert(0) += 1;
        }

        Ok(ArtifactStoreStats {
            total,
            by_type,
            by_domain,
        })
    }
}

impl Default for ArtifactStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Artifact store statistics
#[derive(Clone, Debug)]
pub struct ArtifactStoreStats {
    pub total: usize,
    pub by_type: HashMap<String, usize>,
    pub by_domain: HashMap<String, usize>,
}

/// Artifact store errors
#[derive(Debug, Error)]
pub enum ArtifactStoreError {
    #[error("Artifact not found: {0}")]
    NotFound(String),

    #[error("Lock error")]
    LockError,
}

#[cfg(test)]
mod tests {
    use super::*;
    use eve_types::{ArtifactContent, ConfidenceScore};

    #[test]
    fn test_store_and_query() {
        let store = ArtifactStore::new();

        let artifact = LearningArtifact {
            artifact_id: ArtifactId::generate(),
            artifact_type: ArtifactType::Pattern,
            source_commitment_ids: vec![],
            domain: EffectDomain::Computation,
            content: ArtifactContent {
                summary: "Test pattern".to_string(),
                details: "Details".to_string(),
                data: HashMap::new(),
            },
            confidence: ConfidenceScore::high(100),
            created_at: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        store.store(artifact.clone()).unwrap();

        let query = ArtifactQuery {
            artifact_type: Some(ArtifactType::Pattern),
            ..Default::default()
        };

        let results = store.query(query).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].artifact_id, artifact.artifact_id);
    }
}
