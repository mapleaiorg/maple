//! Enhanced Memory System for MAPLE Resonators
//!
//! This module implements a multi-tier memory system for the Resonance Architecture.
//! Memory is organized into hierarchical tiers that mirror human memory systems:
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    ENHANCED MEMORY SYSTEM                       │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │   ┌───────────────┐   ┌───────────────┐   ┌───────────────┐   │
//! │   │  Short-Term   │──▶│    Working    │──▶│   Long-Term   │   │
//! │   │    Memory     │   │    Memory     │   │    Memory     │   │
//! │   │  (immediate)  │   │ (task scope)  │   │ (persistent)  │   │
//! │   └───────────────┘   └───────────────┘   └───────────────┘   │
//! │          │                   │                   │             │
//! │          ▼                   ▼                   ▼             │
//! │   ┌─────────────────────────────────────────────────────────┐ │
//! │   │              Memory Consolidation Engine                │ │
//! │   │    (decay, reinforcement, semantic indexing)            │ │
//! │   └─────────────────────────────────────────────────────────┘ │
//! │                            │                                   │
//! │                            ▼                                   │
//! │   ┌─────────────────────────────────────────────────────────┐ │
//! │   │                 Episodic Memory                         │ │
//! │   │         (interaction history, experiences)              │ │
//! │   └─────────────────────────────────────────────────────────┘ │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Memory Tiers
//!
//! - [`ShortTermMemory`]: Immediate context, limited capacity, fast decay
//! - [`WorkingMemory`]: Current task context, moderate duration
//! - [`LongTermMemory`]: Persistent storage with semantic retrieval
//! - [`EpisodicMemory`]: Specific interaction records
//!
//! # Consolidation
//!
//! The [`ConsolidationEngine`] manages memory lifecycle:
//! - Decay: Unreinforced memories fade
//! - Reinforcement: Frequently accessed memories strengthen
//! - Promotion: Important working memories move to long-term
//! - Indexing: Semantic embeddings for retrieval

#![deny(unsafe_code)]

use std::collections::{HashMap, VecDeque};
use std::sync::RwLock;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Unique identifier for a memory item.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub String);

impl MemoryId {
    pub fn generate() -> Self {
        Self(format!("mem-{}", uuid::Uuid::new_v4()))
    }

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for MemoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Memory tier classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryTier {
    /// Immediate context, ~seconds to minutes.
    ShortTerm,
    /// Current task context, ~minutes to hours.
    Working,
    /// Persistent storage, days to permanent.
    LongTerm,
    /// Specific interaction records.
    Episodic,
}

/// Type of memory content.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryType {
    /// Factual knowledge.
    Fact,
    /// Procedural knowledge (how to do something).
    Procedure,
    /// User preference or context.
    Preference,
    /// Conversation context.
    Conversation,
    /// Task-related working context.
    TaskContext,
    /// Experience from an interaction.
    Experience,
    /// Custom type.
    Custom(String),
}

/// A memory item with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    /// Unique identifier.
    pub id: MemoryId,
    /// Memory tier.
    pub tier: MemoryTier,
    /// Type of memory.
    pub memory_type: MemoryType,
    /// The content/payload.
    pub content: serde_json::Value,
    /// Human-readable summary.
    pub summary: String,
    /// Tags for categorization.
    pub tags: Vec<String>,
    /// Importance score (0.0-1.0).
    pub importance: f64,
    /// Strength/activation (0.0-1.0, decays over time).
    pub strength: f64,
    /// Number of times accessed.
    pub access_count: u64,
    /// When created.
    pub created_at: DateTime<Utc>,
    /// Last accessed time.
    pub last_accessed: DateTime<Utc>,
    /// When this memory expires (if applicable).
    pub expires_at: Option<DateTime<Utc>>,
    /// Embedding for semantic search (if available).
    pub embedding: Option<Vec<f32>>,
    /// Source of this memory.
    pub source: MemorySource,
    /// References to related memories.
    pub related_memories: Vec<MemoryId>,
}

impl MemoryItem {
    /// Create a new short-term memory item.
    pub fn short_term(
        content: serde_json::Value,
        summary: impl Into<String>,
        memory_type: MemoryType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: MemoryId::generate(),
            tier: MemoryTier::ShortTerm,
            memory_type,
            content,
            summary: summary.into(),
            tags: Vec::new(),
            importance: 0.5,
            strength: 1.0,
            access_count: 0,
            created_at: now,
            last_accessed: now,
            expires_at: Some(now + Duration::minutes(30)), // Default 30 min expiry
            embedding: None,
            source: MemorySource::System,
            related_memories: Vec::new(),
        }
    }

    /// Create a new working memory item.
    pub fn working(
        content: serde_json::Value,
        summary: impl Into<String>,
        memory_type: MemoryType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: MemoryId::generate(),
            tier: MemoryTier::Working,
            memory_type,
            content,
            summary: summary.into(),
            tags: Vec::new(),
            importance: 0.6,
            strength: 1.0,
            access_count: 0,
            created_at: now,
            last_accessed: now,
            expires_at: Some(now + Duration::hours(4)), // Default 4 hour expiry
            embedding: None,
            source: MemorySource::System,
            related_memories: Vec::new(),
        }
    }

    /// Create a new long-term memory item.
    pub fn long_term(
        content: serde_json::Value,
        summary: impl Into<String>,
        memory_type: MemoryType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: MemoryId::generate(),
            tier: MemoryTier::LongTerm,
            memory_type,
            content,
            summary: summary.into(),
            tags: Vec::new(),
            importance: 0.7,
            strength: 1.0,
            access_count: 0,
            created_at: now,
            last_accessed: now,
            expires_at: None, // Long-term doesn't expire by default
            embedding: None,
            source: MemorySource::System,
            related_memories: Vec::new(),
        }
    }

    /// Builder pattern: set importance.
    pub fn with_importance(mut self, importance: f64) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// Builder pattern: set tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Builder pattern: set source.
    pub fn with_source(mut self, source: MemorySource) -> Self {
        self.source = source;
        self
    }

    /// Builder pattern: set embedding.
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Builder pattern: set expiry.
    pub fn with_expiry(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Check if the memory has expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| Utc::now() > exp)
            .unwrap_or(false)
    }

    /// Record an access (updates last_accessed and access_count).
    pub fn record_access(&mut self) {
        self.last_accessed = Utc::now();
        self.access_count += 1;
        // Reinforce strength on access
        self.strength = (self.strength + 0.1).min(1.0);
    }

    /// Apply decay to the memory strength.
    pub fn apply_decay(&mut self, decay_rate: f64) {
        let elapsed = (Utc::now() - self.last_accessed).num_seconds() as f64;
        let decay = (-decay_rate * elapsed / 3600.0).exp(); // Exponential decay over hours
        self.strength *= decay;
    }
}

/// Source of a memory item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemorySource {
    /// Generated by the system.
    System,
    /// From user input.
    User,
    /// From a conversation.
    Conversation { conversation_id: String },
    /// From task execution.
    Task { task_id: String },
    /// Consolidated from other memories.
    Consolidated { source_ids: Vec<MemoryId> },
    /// Imported from external source.
    External { source: String },
}

/// Query for retrieving memories.
#[derive(Debug, Clone)]
pub struct MemoryQuery {
    /// Filter by tier.
    pub tier: Option<MemoryTier>,
    /// Filter by type.
    pub memory_type: Option<MemoryType>,
    /// Filter by tags (any match).
    pub tags: Vec<String>,
    /// Minimum importance.
    pub min_importance: Option<f64>,
    /// Minimum strength.
    pub min_strength: Option<f64>,
    /// Maximum results.
    pub limit: usize,
    /// Include expired memories.
    pub include_expired: bool,
    /// Semantic query embedding.
    pub embedding: Option<Vec<f32>>,
    /// Minimum semantic similarity.
    pub min_similarity: Option<f32>,
}

impl Default for MemoryQuery {
    fn default() -> Self {
        Self {
            tier: None,
            memory_type: None,
            tags: Vec::new(),
            min_importance: None,
            min_strength: None,
            limit: 10,
            include_expired: false,
            embedding: None,
            min_similarity: None,
        }
    }
}

impl MemoryQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tier(mut self, tier: MemoryTier) -> Self {
        self.tier = Some(tier);
        self
    }

    pub fn memory_type(mut self, memory_type: MemoryType) -> Self {
        self.memory_type = Some(memory_type);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn min_importance(mut self, importance: f64) -> Self {
        self.min_importance = Some(importance);
        self
    }

    pub fn min_strength(mut self, strength: f64) -> Self {
        self.min_strength = Some(strength);
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn include_expired(mut self) -> Self {
        self.include_expired = true;
        self
    }

    pub fn semantic(mut self, embedding: Vec<f32>, min_similarity: f32) -> Self {
        self.embedding = Some(embedding);
        self.min_similarity = Some(min_similarity);
        self
    }
}

/// Result of a memory query.
#[derive(Debug, Clone)]
pub struct MemoryQueryResult {
    pub item: MemoryItem,
    pub relevance_score: f64,
    pub similarity_score: Option<f32>,
}

/// Short-term memory store.
///
/// Limited capacity, fast access, automatic expiry.
#[derive(Debug)]
pub struct ShortTermMemory {
    items: RwLock<VecDeque<MemoryItem>>,
    capacity: usize,
}

impl ShortTermMemory {
    pub fn new(capacity: usize) -> Self {
        Self {
            items: RwLock::new(VecDeque::new()),
            capacity,
        }
    }

    /// Store a memory item.
    pub fn store(&self, item: MemoryItem) -> Result<MemoryId, MemoryError> {
        let mut guard = self
            .items
            .write()
            .map_err(|_| MemoryError::LockError)?;

        // Enforce capacity limit (FIFO eviction)
        while guard.len() >= self.capacity {
            guard.pop_front();
        }

        let id = item.id.clone();
        guard.push_back(item);
        Ok(id)
    }

    /// Retrieve a memory by ID.
    pub fn get(&self, id: &MemoryId) -> Result<Option<MemoryItem>, MemoryError> {
        let mut guard = self
            .items
            .write()
            .map_err(|_| MemoryError::LockError)?;

        if let Some(item) = guard.iter_mut().find(|i| &i.id == id) {
            item.record_access();
            return Ok(Some(item.clone()));
        }

        Ok(None)
    }

    /// Query memories.
    pub fn query(&self, query: &MemoryQuery) -> Result<Vec<MemoryQueryResult>, MemoryError> {
        let mut guard = self
            .items
            .write()
            .map_err(|_| MemoryError::LockError)?;

        let now = Utc::now();
        let mut results: Vec<MemoryQueryResult> = guard
            .iter_mut()
            .filter(|item| {
                // Filter expired
                if !query.include_expired && item.is_expired() {
                    return false;
                }
                // Filter by tier
                if let Some(ref tier) = query.tier {
                    if &item.tier != tier {
                        return false;
                    }
                }
                // Filter by type
                if let Some(ref memory_type) = query.memory_type {
                    if &item.memory_type != memory_type {
                        return false;
                    }
                }
                // Filter by tags
                if !query.tags.is_empty()
                    && !query.tags.iter().any(|t| item.tags.contains(t))
                {
                    return false;
                }
                // Filter by importance
                if let Some(min) = query.min_importance {
                    if item.importance < min {
                        return false;
                    }
                }
                // Filter by strength
                if let Some(min) = query.min_strength {
                    if item.strength < min {
                        return false;
                    }
                }
                true
            })
            .map(|item| {
                item.record_access();
                let relevance = calculate_relevance(item, &now);
                let similarity = query
                    .embedding
                    .as_ref()
                    .and_then(|q| item.embedding.as_ref().map(|e| cosine_similarity(q, e)));
                MemoryQueryResult {
                    item: item.clone(),
                    relevance_score: relevance,
                    similarity_score: similarity,
                }
            })
            .filter(|r| {
                if let (Some(sim), Some(min)) = (r.similarity_score, query.min_similarity) {
                    sim >= min
                } else {
                    true
                }
            })
            .collect();

        // Sort by relevance
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

        // Apply limit
        results.truncate(query.limit);

        Ok(results)
    }

    /// Clear expired memories.
    pub fn clear_expired(&self) -> Result<usize, MemoryError> {
        let mut guard = self
            .items
            .write()
            .map_err(|_| MemoryError::LockError)?;

        let before = guard.len();
        guard.retain(|item| !item.is_expired());
        Ok(before - guard.len())
    }

    /// Get all items (for consolidation).
    pub fn all(&self) -> Result<Vec<MemoryItem>, MemoryError> {
        let guard = self
            .items
            .read()
            .map_err(|_| MemoryError::LockError)?;
        Ok(guard.iter().cloned().collect())
    }

    /// Remove a specific memory.
    pub fn remove(&self, id: &MemoryId) -> Result<Option<MemoryItem>, MemoryError> {
        let mut guard = self
            .items
            .write()
            .map_err(|_| MemoryError::LockError)?;

        if let Some(pos) = guard.iter().position(|i| &i.id == id) {
            return Ok(guard.remove(pos));
        }

        Ok(None)
    }
}

impl Default for ShortTermMemory {
    fn default() -> Self {
        Self::new(100) // Default 100 items
    }
}

/// Working memory for current task context.
///
/// Longer duration than short-term, supports task association.
#[derive(Debug)]
pub struct WorkingMemory {
    items: RwLock<HashMap<MemoryId, MemoryItem>>,
    by_task: RwLock<HashMap<String, Vec<MemoryId>>>,
    capacity: usize,
}

impl WorkingMemory {
    pub fn new(capacity: usize) -> Self {
        Self {
            items: RwLock::new(HashMap::new()),
            by_task: RwLock::new(HashMap::new()),
            capacity,
        }
    }

    /// Store a memory item, optionally associated with a task.
    pub fn store(
        &self,
        item: MemoryItem,
        task_id: Option<&str>,
    ) -> Result<MemoryId, MemoryError> {
        let mut items = self
            .items
            .write()
            .map_err(|_| MemoryError::LockError)?;

        // Enforce capacity
        if items.len() >= self.capacity {
            // Evict lowest strength item
            if let Some(evict_id) = items
                .values()
                .min_by(|a, b| a.strength.partial_cmp(&b.strength).unwrap())
                .map(|i| i.id.clone())
            {
                items.remove(&evict_id);
            }
        }

        let id = item.id.clone();
        items.insert(id.clone(), item);

        // Associate with task if provided
        if let Some(task_id) = task_id {
            let mut by_task = self
                .by_task
                .write()
                .map_err(|_| MemoryError::LockError)?;
            by_task
                .entry(task_id.to_string())
                .or_default()
                .push(id.clone());
        }

        Ok(id)
    }

    /// Get memory by ID.
    pub fn get(&self, id: &MemoryId) -> Result<Option<MemoryItem>, MemoryError> {
        let mut items = self
            .items
            .write()
            .map_err(|_| MemoryError::LockError)?;

        if let Some(item) = items.get_mut(id) {
            item.record_access();
            return Ok(Some(item.clone()));
        }

        Ok(None)
    }

    /// Get all memories for a task.
    pub fn get_for_task(&self, task_id: &str) -> Result<Vec<MemoryItem>, MemoryError> {
        let by_task = self
            .by_task
            .read()
            .map_err(|_| MemoryError::LockError)?;
        let items = self
            .items
            .read()
            .map_err(|_| MemoryError::LockError)?;

        let ids = by_task.get(task_id).cloned().unwrap_or_default();
        Ok(ids
            .iter()
            .filter_map(|id| items.get(id).cloned())
            .collect())
    }

    /// Clear all memories for a task.
    pub fn clear_task(&self, task_id: &str) -> Result<usize, MemoryError> {
        let mut by_task = self
            .by_task
            .write()
            .map_err(|_| MemoryError::LockError)?;
        let mut items = self
            .items
            .write()
            .map_err(|_| MemoryError::LockError)?;

        if let Some(ids) = by_task.remove(task_id) {
            let count = ids.len();
            for id in ids {
                items.remove(&id);
            }
            return Ok(count);
        }

        Ok(0)
    }

    /// Query working memories.
    pub fn query(&self, query: &MemoryQuery) -> Result<Vec<MemoryQueryResult>, MemoryError> {
        let mut items = self
            .items
            .write()
            .map_err(|_| MemoryError::LockError)?;

        let now = Utc::now();
        let mut results: Vec<MemoryQueryResult> = items
            .values_mut()
            .filter(|item| {
                if !query.include_expired && item.is_expired() {
                    return false;
                }
                if let Some(ref tier) = query.tier {
                    if &item.tier != tier {
                        return false;
                    }
                }
                if let Some(ref memory_type) = query.memory_type {
                    if &item.memory_type != memory_type {
                        return false;
                    }
                }
                if !query.tags.is_empty()
                    && !query.tags.iter().any(|t| item.tags.contains(t))
                {
                    return false;
                }
                if let Some(min) = query.min_importance {
                    if item.importance < min {
                        return false;
                    }
                }
                if let Some(min) = query.min_strength {
                    if item.strength < min {
                        return false;
                    }
                }
                true
            })
            .map(|item| {
                item.record_access();
                let relevance = calculate_relevance(item, &now);
                let similarity = query
                    .embedding
                    .as_ref()
                    .and_then(|q| item.embedding.as_ref().map(|e| cosine_similarity(q, e)));
                MemoryQueryResult {
                    item: item.clone(),
                    relevance_score: relevance,
                    similarity_score: similarity,
                }
            })
            .filter(|r| {
                if let (Some(sim), Some(min)) = (r.similarity_score, query.min_similarity) {
                    sim >= min
                } else {
                    true
                }
            })
            .collect();

        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        results.truncate(query.limit);

        Ok(results)
    }

    /// Get all items for consolidation.
    pub fn all(&self) -> Result<Vec<MemoryItem>, MemoryError> {
        let items = self
            .items
            .read()
            .map_err(|_| MemoryError::LockError)?;
        Ok(items.values().cloned().collect())
    }

    /// Remove a specific memory.
    pub fn remove(&self, id: &MemoryId) -> Result<Option<MemoryItem>, MemoryError> {
        let mut items = self
            .items
            .write()
            .map_err(|_| MemoryError::LockError)?;
        Ok(items.remove(id))
    }
}

impl Default for WorkingMemory {
    fn default() -> Self {
        Self::new(500) // Default 500 items
    }
}

/// Long-term memory with semantic retrieval.
#[derive(Debug)]
pub struct LongTermMemory {
    items: RwLock<HashMap<MemoryId, MemoryItem>>,
    by_type: RwLock<HashMap<MemoryType, Vec<MemoryId>>>,
}

impl LongTermMemory {
    pub fn new() -> Self {
        Self {
            items: RwLock::new(HashMap::new()),
            by_type: RwLock::new(HashMap::new()),
        }
    }

    /// Store a memory item.
    pub fn store(&self, item: MemoryItem) -> Result<MemoryId, MemoryError> {
        let mut items = self
            .items
            .write()
            .map_err(|_| MemoryError::LockError)?;

        let id = item.id.clone();
        let memory_type = item.memory_type.clone();

        items.insert(id.clone(), item);

        let mut by_type = self
            .by_type
            .write()
            .map_err(|_| MemoryError::LockError)?;
        by_type.entry(memory_type).or_default().push(id.clone());

        Ok(id)
    }

    /// Get memory by ID.
    pub fn get(&self, id: &MemoryId) -> Result<Option<MemoryItem>, MemoryError> {
        let mut items = self
            .items
            .write()
            .map_err(|_| MemoryError::LockError)?;

        if let Some(item) = items.get_mut(id) {
            item.record_access();
            return Ok(Some(item.clone()));
        }

        Ok(None)
    }

    /// Semantic search.
    pub fn semantic_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
        min_similarity: f32,
    ) -> Result<Vec<MemoryQueryResult>, MemoryError> {
        let items = self
            .items
            .read()
            .map_err(|_| MemoryError::LockError)?;

        let now = Utc::now();
        let mut results: Vec<MemoryQueryResult> = items
            .values()
            .filter_map(|item| {
                let embedding = item.embedding.as_ref()?;
                let similarity = cosine_similarity(query_embedding, embedding);
                if similarity >= min_similarity {
                    Some(MemoryQueryResult {
                        item: item.clone(),
                        relevance_score: calculate_relevance(item, &now),
                        similarity_score: Some(similarity),
                    })
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.similarity_score
                .partial_cmp(&a.similarity_score)
                .unwrap()
        });
        results.truncate(limit);

        Ok(results)
    }

    /// Query long-term memories.
    pub fn query(&self, query: &MemoryQuery) -> Result<Vec<MemoryQueryResult>, MemoryError> {
        let mut items = self
            .items
            .write()
            .map_err(|_| MemoryError::LockError)?;

        let now = Utc::now();
        let mut results: Vec<MemoryQueryResult> = items
            .values_mut()
            .filter(|item| {
                if !query.include_expired && item.is_expired() {
                    return false;
                }
                if let Some(ref tier) = query.tier {
                    if &item.tier != tier {
                        return false;
                    }
                }
                if let Some(ref memory_type) = query.memory_type {
                    if &item.memory_type != memory_type {
                        return false;
                    }
                }
                if !query.tags.is_empty()
                    && !query.tags.iter().any(|t| item.tags.contains(t))
                {
                    return false;
                }
                if let Some(min) = query.min_importance {
                    if item.importance < min {
                        return false;
                    }
                }
                if let Some(min) = query.min_strength {
                    if item.strength < min {
                        return false;
                    }
                }
                true
            })
            .map(|item| {
                item.record_access();
                let relevance = calculate_relevance(item, &now);
                let similarity = query
                    .embedding
                    .as_ref()
                    .and_then(|q| item.embedding.as_ref().map(|e| cosine_similarity(q, e)));
                MemoryQueryResult {
                    item: item.clone(),
                    relevance_score: relevance,
                    similarity_score: similarity,
                }
            })
            .filter(|r| {
                if let (Some(sim), Some(min)) = (r.similarity_score, query.min_similarity) {
                    sim >= min
                } else {
                    true
                }
            })
            .collect();

        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        results.truncate(query.limit);

        Ok(results)
    }

    /// Get all items.
    pub fn all(&self) -> Result<Vec<MemoryItem>, MemoryError> {
        let items = self
            .items
            .read()
            .map_err(|_| MemoryError::LockError)?;
        Ok(items.values().cloned().collect())
    }

    /// Remove a memory.
    pub fn remove(&self, id: &MemoryId) -> Result<Option<MemoryItem>, MemoryError> {
        let mut items = self
            .items
            .write()
            .map_err(|_| MemoryError::LockError)?;
        Ok(items.remove(id))
    }
}

impl Default for LongTermMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// Episodic memory for specific interaction records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// Unique episode ID.
    pub id: String,
    /// When the episode started.
    pub started_at: DateTime<Utc>,
    /// When the episode ended.
    pub ended_at: Option<DateTime<Utc>>,
    /// Description/summary.
    pub summary: String,
    /// Participants.
    pub participants: Vec<String>,
    /// Outcome of the episode.
    pub outcome: Option<EpisodeOutcome>,
    /// Related memory IDs.
    pub memories: Vec<MemoryId>,
    /// Tags.
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EpisodeOutcome {
    Success,
    Failure { reason: String },
    Incomplete,
    Abandoned,
}

/// Episodic memory store.
#[derive(Debug, Default)]
pub struct EpisodicMemory {
    episodes: RwLock<HashMap<String, Episode>>,
}

impl EpisodicMemory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new episode.
    pub fn start_episode(
        &self,
        summary: impl Into<String>,
        participants: Vec<String>,
    ) -> Result<String, MemoryError> {
        let mut episodes = self
            .episodes
            .write()
            .map_err(|_| MemoryError::LockError)?;

        let id = format!("ep-{}", uuid::Uuid::new_v4());
        let episode = Episode {
            id: id.clone(),
            started_at: Utc::now(),
            ended_at: None,
            summary: summary.into(),
            participants,
            outcome: None,
            memories: Vec::new(),
            tags: Vec::new(),
        };

        episodes.insert(id.clone(), episode);
        Ok(id)
    }

    /// End an episode.
    pub fn end_episode(
        &self,
        episode_id: &str,
        outcome: EpisodeOutcome,
    ) -> Result<(), MemoryError> {
        let mut episodes = self
            .episodes
            .write()
            .map_err(|_| MemoryError::LockError)?;

        let episode = episodes
            .get_mut(episode_id)
            .ok_or_else(|| MemoryError::NotFound(episode_id.to_string()))?;

        episode.ended_at = Some(Utc::now());
        episode.outcome = Some(outcome);

        Ok(())
    }

    /// Add a memory to an episode.
    pub fn add_memory_to_episode(
        &self,
        episode_id: &str,
        memory_id: MemoryId,
    ) -> Result<(), MemoryError> {
        let mut episodes = self
            .episodes
            .write()
            .map_err(|_| MemoryError::LockError)?;

        let episode = episodes
            .get_mut(episode_id)
            .ok_or_else(|| MemoryError::NotFound(episode_id.to_string()))?;

        episode.memories.push(memory_id);

        Ok(())
    }

    /// Get an episode.
    pub fn get(&self, episode_id: &str) -> Result<Option<Episode>, MemoryError> {
        let episodes = self
            .episodes
            .read()
            .map_err(|_| MemoryError::LockError)?;
        Ok(episodes.get(episode_id).cloned())
    }

    /// Get recent episodes.
    pub fn recent(&self, limit: usize) -> Result<Vec<Episode>, MemoryError> {
        let episodes = self
            .episodes
            .read()
            .map_err(|_| MemoryError::LockError)?;

        let mut vec: Vec<Episode> = episodes.values().cloned().collect();
        vec.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        vec.truncate(limit);

        Ok(vec)
    }
}

/// Configuration for memory consolidation.
#[derive(Debug, Clone)]
pub struct ConsolidationConfig {
    /// Decay rate for short-term memory.
    pub short_term_decay_rate: f64,
    /// Decay rate for working memory.
    pub working_memory_decay_rate: f64,
    /// Minimum strength to promote to working memory.
    pub promote_to_working_threshold: f64,
    /// Minimum importance to promote to long-term.
    pub promote_to_long_term_threshold: f64,
    /// Minimum access count for long-term promotion.
    pub min_access_for_long_term: u64,
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self {
            short_term_decay_rate: 0.5,      // Fast decay
            working_memory_decay_rate: 0.1,  // Slower decay
            promote_to_working_threshold: 0.7,
            promote_to_long_term_threshold: 0.8,
            min_access_for_long_term: 3,
        }
    }
}

/// Memory consolidation engine.
///
/// Manages the lifecycle of memories across tiers.
pub struct ConsolidationEngine {
    config: ConsolidationConfig,
}

impl ConsolidationEngine {
    pub fn new(config: ConsolidationConfig) -> Self {
        Self { config }
    }

    /// Run consolidation on memory stores.
    pub fn consolidate(
        &self,
        short_term: &ShortTermMemory,
        working: &WorkingMemory,
        long_term: &LongTermMemory,
    ) -> Result<ConsolidationResult, MemoryError> {
        let mut promoted_to_working = 0;
        let mut promoted_to_long_term = 0;
        let mut decayed = 0;
        let mut expired_cleared = 0;

        // Clear expired short-term
        expired_cleared += short_term.clear_expired()?;

        // Process short-term memories
        for mut item in short_term.all()? {
            // Apply decay
            item.apply_decay(self.config.short_term_decay_rate);
            decayed += 1;

            // Check for promotion to working
            if item.strength >= self.config.promote_to_working_threshold
                && item.importance >= 0.5
            {
                item.tier = MemoryTier::Working;
                item.expires_at = Some(Utc::now() + Duration::hours(4));
                working.store(item.clone(), None)?;
                short_term.remove(&item.id)?;
                promoted_to_working += 1;
            }
        }

        // Process working memories
        for mut item in working.all()? {
            // Apply decay
            item.apply_decay(self.config.working_memory_decay_rate);

            // Check for promotion to long-term
            if item.importance >= self.config.promote_to_long_term_threshold
                && item.access_count >= self.config.min_access_for_long_term
            {
                item.tier = MemoryTier::LongTerm;
                item.expires_at = None;
                long_term.store(item.clone())?;
                working.remove(&item.id)?;
                promoted_to_long_term += 1;
            }
        }

        Ok(ConsolidationResult {
            promoted_to_working,
            promoted_to_long_term,
            decayed,
            expired_cleared,
        })
    }
}

impl Default for ConsolidationEngine {
    fn default() -> Self {
        Self::new(ConsolidationConfig::default())
    }
}

/// Result of a consolidation run.
#[derive(Debug, Clone)]
pub struct ConsolidationResult {
    pub promoted_to_working: usize,
    pub promoted_to_long_term: usize,
    pub decayed: usize,
    pub expired_cleared: usize,
}

/// Unified memory system facade.
pub struct MemorySystem {
    pub short_term: ShortTermMemory,
    pub working: WorkingMemory,
    pub long_term: LongTermMemory,
    pub episodic: EpisodicMemory,
    consolidation: ConsolidationEngine,
}

impl MemorySystem {
    pub fn new() -> Self {
        Self {
            short_term: ShortTermMemory::default(),
            working: WorkingMemory::default(),
            long_term: LongTermMemory::default(),
            episodic: EpisodicMemory::default(),
            consolidation: ConsolidationEngine::default(),
        }
    }

    pub fn with_config(consolidation_config: ConsolidationConfig) -> Self {
        Self {
            short_term: ShortTermMemory::default(),
            working: WorkingMemory::default(),
            long_term: LongTermMemory::default(),
            episodic: EpisodicMemory::default(),
            consolidation: ConsolidationEngine::new(consolidation_config),
        }
    }

    /// Store a memory item in the appropriate tier.
    pub fn store(&self, item: MemoryItem) -> Result<MemoryId, MemoryError> {
        match item.tier {
            MemoryTier::ShortTerm => self.short_term.store(item),
            MemoryTier::Working => self.working.store(item, None),
            MemoryTier::LongTerm => self.long_term.store(item),
            MemoryTier::Episodic => Err(MemoryError::InvalidOperation(
                "Use episodic methods for episodic memories".to_string(),
            )),
        }
    }

    /// Unified query across all tiers.
    pub fn query(&self, query: &MemoryQuery) -> Result<Vec<MemoryQueryResult>, MemoryError> {
        let mut results = Vec::new();

        // Query each tier
        if query.tier.is_none() || query.tier == Some(MemoryTier::ShortTerm) {
            results.extend(self.short_term.query(query)?);
        }
        if query.tier.is_none() || query.tier == Some(MemoryTier::Working) {
            results.extend(self.working.query(query)?);
        }
        if query.tier.is_none() || query.tier == Some(MemoryTier::LongTerm) {
            results.extend(self.long_term.query(query)?);
        }

        // Sort and limit
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        results.truncate(query.limit);

        Ok(results)
    }

    /// Run memory consolidation.
    pub fn consolidate(&self) -> Result<ConsolidationResult, MemoryError> {
        self.consolidation
            .consolidate(&self.short_term, &self.working, &self.long_term)
    }
}

impl Default for MemorySystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory system errors.
#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Memory not found: {0}")]
    NotFound(String),

    #[error("Lock error")]
    LockError,

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Capacity exceeded")]
    CapacityExceeded,

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Calculate relevance score based on recency, importance, and strength.
fn calculate_relevance(item: &MemoryItem, now: &DateTime<Utc>) -> f64 {
    let recency_hours = (*now - item.last_accessed).num_hours() as f64;
    let recency_score = (-recency_hours / 24.0).exp(); // Decay over 24 hours

    // Weighted combination
    0.3 * recency_score + 0.4 * item.importance + 0.3 * item.strength
}

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let (mut dot, mut norm_a, mut norm_b) = (0.0_f32, 0.0_f32, 0.0_f32);
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a.sqrt() * norm_b.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_term_memory_store_and_retrieve() {
        let stm = ShortTermMemory::new(10);

        let item = MemoryItem::short_term(
            serde_json::json!({"key": "value"}),
            "Test memory",
            MemoryType::Fact,
        );
        let id = item.id.clone();

        stm.store(item).unwrap();

        let retrieved = stm.get(&id).unwrap().unwrap();
        assert_eq!(retrieved.summary, "Test memory");
        assert_eq!(retrieved.access_count, 1); // Access recorded
    }

    #[test]
    fn test_short_term_capacity_eviction() {
        let stm = ShortTermMemory::new(3);

        for i in 0..5 {
            let item = MemoryItem::short_term(
                serde_json::json!({"i": i}),
                format!("Memory {}", i),
                MemoryType::Fact,
            );
            stm.store(item).unwrap();
        }

        // Only last 3 should remain
        let all = stm.all().unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_working_memory_task_association() {
        let wm = WorkingMemory::new(100);

        let item1 = MemoryItem::working(
            serde_json::json!({"task": "a"}),
            "Task A memory",
            MemoryType::TaskContext,
        );
        let item2 = MemoryItem::working(
            serde_json::json!({"task": "b"}),
            "Task B memory",
            MemoryType::TaskContext,
        );

        wm.store(item1, Some("task-a")).unwrap();
        wm.store(item2, Some("task-b")).unwrap();

        let task_a_memories = wm.get_for_task("task-a").unwrap();
        assert_eq!(task_a_memories.len(), 1);
        assert_eq!(task_a_memories[0].summary, "Task A memory");
    }

    #[test]
    fn test_long_term_semantic_search() {
        let ltm = LongTermMemory::new();

        let item1 = MemoryItem::long_term(
            serde_json::json!({"topic": "dogs"}),
            "About dogs",
            MemoryType::Fact,
        )
        .with_embedding(vec![1.0, 0.0, 0.0]);

        let item2 = MemoryItem::long_term(
            serde_json::json!({"topic": "cats"}),
            "About cats",
            MemoryType::Fact,
        )
        .with_embedding(vec![0.1, 0.9, 0.0]);

        ltm.store(item1).unwrap();
        ltm.store(item2).unwrap();

        // Search with embedding close to "dogs"
        let results = ltm
            .semantic_search(&[0.9, 0.1, 0.0], 1, 0.0)
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].item.summary, "About dogs");
    }

    #[test]
    fn test_memory_decay() {
        let mut item = MemoryItem::short_term(
            serde_json::json!({}),
            "Decaying memory",
            MemoryType::Fact,
        );

        assert_eq!(item.strength, 1.0);

        // Simulate time passing
        item.last_accessed = Utc::now() - Duration::hours(2);
        item.apply_decay(0.5);

        assert!(item.strength < 1.0);
    }

    #[test]
    fn test_episodic_memory() {
        let em = EpisodicMemory::new();

        let ep_id = em
            .start_episode("Test interaction", vec!["user".to_string(), "agent".to_string()])
            .unwrap();

        let mem_id = MemoryId::generate();
        em.add_memory_to_episode(&ep_id, mem_id).unwrap();

        em.end_episode(&ep_id, EpisodeOutcome::Success).unwrap();

        let episode = em.get(&ep_id).unwrap().unwrap();
        assert!(episode.ended_at.is_some());
        assert!(matches!(episode.outcome, Some(EpisodeOutcome::Success)));
        assert_eq!(episode.memories.len(), 1);
    }

    #[test]
    fn test_memory_system_unified_query() {
        let system = MemorySystem::new();

        // Store in different tiers
        system
            .store(MemoryItem::short_term(
                serde_json::json!({}),
                "Short term",
                MemoryType::Fact,
            ))
            .unwrap();
        system
            .store(MemoryItem::working(
                serde_json::json!({}),
                "Working",
                MemoryType::Fact,
            ))
            .unwrap();
        system
            .store(MemoryItem::long_term(
                serde_json::json!({}),
                "Long term",
                MemoryType::Fact,
            ))
            .unwrap();

        // Query all
        let query = MemoryQuery::new().memory_type(MemoryType::Fact).limit(10);
        let results = system.query(&query).unwrap();

        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_cosine_similarity() {
        assert!((cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 0.001);
        assert!((cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]) - 0.0).abs() < 0.001);
        assert!((cosine_similarity(&[1.0, 0.0], &[-1.0, 0.0]) - (-1.0)).abs() < 0.001);
    }
}
