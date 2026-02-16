//! Agent-to-Agent (A2A) Protocol Adapter for MAPLE
//!
//! This module provides bidirectional translation between Google's Agent-to-Agent
//! Protocol (A2A) and MAPLE's Resonance Architecture. A2A tasks and messages are
//! mapped to MAPLE's resonance flow while preserving A2A's agent card discovery
//! and task lifecycle semantics.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      A2A PROTOCOL ADAPTER                       │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │   ┌─────────────┐         ┌─────────────┐                      │
//! │   │ A2A Agent   │◀───────▶│   MAPLE     │                      │
//! │   │   Cards     │         │ Resonators  │                      │
//! │   └─────────────┘         └─────────────┘                      │
//! │         │                       │                               │
//! │         ▼                       ▼                               │
//! │   ┌─────────────────────────────────────────────────────────┐ │
//! │   │              Agent Card ⟷ Profile Mapper                │ │
//! │   │         (capabilities, skills → resonator profile)      │ │
//! │   └─────────────────────────────────────────────────────────┘ │
//! │                            │                                   │
//! │                            ▼                                   │
//! │   ┌─────────────────────────────────────────────────────────┐ │
//! │   │              Task ⟷ Commitment Bridge                   │ │
//! │   │    (A2A tasks flow through commitment boundary)         │ │
//! │   └─────────────────────────────────────────────────────────┘ │
//! │                            │                                   │
//! │   ┌─────────────┐         │         ┌─────────────┐          │
//! │   │ A2A Messages│◀────────┴────────▶│   MAPLE     │          │
//! │   │  (streaming)│                   │  Coupling   │          │
//! │   └─────────────┘                   └─────────────┘          │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Principles
//!
//! 1. **A2A as Transport, Not Authority**: A2A messages flow through MAPLE's
//!    commitment gateway - A2A cannot bypass safety invariants.
//!
//! 2. **Agent Card to Profile Mapping**: A2A agent cards map to MAPLE resonator
//!    profiles with explicit capabilities and constraints.
//!
//! 3. **Task to Commitment Flow**: A2A tasks become MAPLE intents and commitments
//!    with full audit trails.
//!
//! 4. **Streaming via Coupling**: A2A message streaming maps to MAPLE's
//!    resonance coupling for continuous state exchange.

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rcf_commitment::{CommitmentBuilder, CommitmentId, RcfCommitment};
use rcf_types::{EffectDomain, IdentityRef, ScopeConstraint};
use resonator_types::{ResonatorId, ResonatorProfile};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// A2A Protocol Types (matching Google A2A spec)
// ============================================================================

/// A2A Agent Card - describes an agent's capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aAgentCard {
    /// Agent name.
    pub name: String,
    /// Agent description.
    pub description: String,
    /// Agent URL (endpoint).
    pub url: String,
    /// Provider information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<A2aProvider>,
    /// Agent version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Supported capabilities.
    #[serde(default)]
    pub capabilities: A2aCapabilities,
    /// Authentication requirements.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<A2aAuthentication>,
    /// Skills the agent can perform.
    #[serde(default)]
    pub skills: Vec<A2aSkill>,
    /// Input modes supported.
    #[serde(rename = "defaultInputModes", default)]
    pub default_input_modes: Vec<String>,
    /// Output modes supported.
    #[serde(rename = "defaultOutputModes", default)]
    pub default_output_modes: Vec<String>,
}

/// A2A Provider information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aProvider {
    /// Provider organization.
    pub organization: String,
    /// Provider URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// A2A Capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct A2aCapabilities {
    /// Supports streaming.
    #[serde(default)]
    pub streaming: bool,
    /// Supports push notifications.
    #[serde(rename = "pushNotifications", default)]
    pub push_notifications: bool,
    /// Supports state transfer.
    #[serde(rename = "stateTransferModes", default)]
    pub state_transfer_modes: Vec<String>,
}

/// A2A Authentication requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aAuthentication {
    /// Authentication schemes supported.
    pub schemes: Vec<String>,
    /// OAuth credentials if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<String>,
}

/// A2A Skill definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aSkill {
    /// Skill identifier.
    pub id: String,
    /// Skill name.
    pub name: String,
    /// Skill description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Example prompts.
    #[serde(default)]
    pub examples: Vec<String>,
    /// Input schema.
    #[serde(rename = "inputSchema", skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
    /// Output schema.
    #[serde(rename = "outputSchema", skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
}

/// A2A Task - represents a unit of work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aTask {
    /// Task ID.
    pub id: String,
    /// Session ID for multi-turn.
    #[serde(rename = "sessionId", skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Task status.
    pub status: A2aTaskStatus,
    /// Task history.
    #[serde(default)]
    pub history: Vec<A2aMessage>,
    /// Task artifacts (outputs).
    #[serde(default)]
    pub artifacts: Vec<A2aArtifact>,
    /// Task metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A2A Task status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum A2aTaskStatus {
    /// Task is submitted but not started.
    Submitted,
    /// Task is currently running.
    Working,
    /// Task requires user input.
    InputRequired,
    /// Task completed successfully.
    Completed,
    /// Task failed.
    Failed,
    /// Task was canceled.
    Canceled,
}

impl A2aTaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            A2aTaskStatus::Completed | A2aTaskStatus::Failed | A2aTaskStatus::Canceled
        )
    }
}

/// A2A Message in task history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aMessage {
    /// Message role.
    pub role: A2aRole,
    /// Message parts.
    pub parts: Vec<A2aPart>,
    /// Timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
}

/// A2A Message role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum A2aRole {
    User,
    Agent,
    System,
}

/// A2A Message part.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum A2aPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "file")]
    File {
        #[serde(rename = "mimeType")]
        mime_type: String,
        uri: String,
    },
    #[serde(rename = "data")]
    Data { data: serde_json::Value },
}

/// A2A Artifact produced by task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aArtifact {
    /// Artifact name.
    pub name: String,
    /// Artifact parts.
    pub parts: Vec<A2aPart>,
    /// Artifact index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    /// Whether this is the final version.
    #[serde(rename = "lastChunk", skip_serializing_if = "Option::is_none")]
    pub last_chunk: Option<bool>,
}

/// A2A Task send request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aTaskSendRequest {
    /// Task ID (optional for new tasks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Session ID.
    #[serde(rename = "sessionId", skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Message to send.
    pub message: A2aMessage,
    /// Push notification config.
    #[serde(rename = "pushNotification", skip_serializing_if = "Option::is_none")]
    pub push_notification: Option<A2aPushConfig>,
    /// Metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A2A Push notification config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aPushConfig {
    /// Push URL.
    pub url: String,
    /// Authentication token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

// ============================================================================
// MAPLE Integration Types
// ============================================================================

/// Mapping from A2A agent to MAPLE resonator.
#[derive(Debug, Clone)]
pub struct A2aResonatorMapping {
    /// The A2A agent card.
    pub agent_card: A2aAgentCard,
    /// Mapped MAPLE resonator ID.
    pub resonator_id: ResonatorId,
    /// MAPLE effect domain.
    pub domain: EffectDomain,
    /// MAPLE scope constraint.
    pub scope: ScopeConstraint,
    /// Whether tasks require commitments.
    pub requires_commitment: bool,
    /// Risk level (0.0-1.0).
    pub risk_level: f64,
}

impl A2aResonatorMapping {
    /// Create a basic mapping.
    pub fn new(agent_card: A2aAgentCard, resonator_id: ResonatorId) -> Self {
        Self {
            agent_card,
            resonator_id,
            domain: EffectDomain::Computation,
            scope: ScopeConstraint::default(),
            requires_commitment: false,
            risk_level: 0.3,
        }
    }

    /// Create a mapping for consequential operations.
    pub fn consequential(
        agent_card: A2aAgentCard,
        resonator_id: ResonatorId,
        domain: EffectDomain,
    ) -> Self {
        Self {
            agent_card,
            resonator_id,
            domain,
            scope: ScopeConstraint::default(),
            requires_commitment: true,
            risk_level: 0.6,
        }
    }

    /// Set scope.
    pub fn with_scope(mut self, scope: ScopeConstraint) -> Self {
        self.scope = scope;
        self
    }

    /// Set risk level.
    pub fn with_risk(mut self, risk: f64) -> Self {
        self.risk_level = risk.clamp(0.0, 1.0);
        self
    }
}

/// A2A task record for audit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aTaskRecord {
    /// Task ID.
    pub task_id: String,
    /// Session ID.
    pub session_id: Option<String>,
    /// Source agent (remote).
    pub source_agent: String,
    /// Target resonator (local).
    pub target_resonator: String,
    /// Associated commitment ID.
    pub commitment_id: Option<CommitmentId>,
    /// Task status.
    pub status: A2aTaskStatus,
    /// Created at.
    pub created_at: DateTime<Utc>,
    /// Updated at.
    pub updated_at: DateTime<Utc>,
    /// Message count.
    pub message_count: usize,
}

/// A2A task handler trait.
#[async_trait]
pub trait A2aTaskHandler: Send + Sync {
    /// Handle a task send request.
    async fn handle_task(&self, request: A2aTaskSendRequest) -> Result<A2aTask, A2aAdapterError>;

    /// Get task status.
    async fn get_task(&self, task_id: &str) -> Result<Option<A2aTask>, A2aAdapterError>;

    /// Cancel a task.
    async fn cancel_task(&self, task_id: &str) -> Result<A2aTask, A2aAdapterError>;
}

/// Registry of A2A agents and their mappings.
#[derive(Default)]
pub struct A2aAgentRegistry {
    agents: HashMap<String, A2aResonatorMapping>,
    by_resonator: HashMap<ResonatorId, String>,
}

impl A2aAgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an agent mapping.
    pub fn register(&mut self, mapping: A2aResonatorMapping) {
        let name = mapping.agent_card.name.clone();
        self.by_resonator
            .insert(mapping.resonator_id.clone(), name.clone());
        self.agents.insert(name, mapping);
    }

    /// Get a mapping by agent name.
    pub fn get(&self, name: &str) -> Option<&A2aResonatorMapping> {
        self.agents.get(name)
    }

    /// Get a mapping by resonator ID.
    pub fn get_by_resonator(&self, resonator_id: &ResonatorId) -> Option<&A2aResonatorMapping> {
        self.by_resonator
            .get(resonator_id)
            .and_then(|name| self.agents.get(name))
    }

    /// List all registered agent cards.
    pub fn list_agents(&self) -> Vec<&A2aAgentCard> {
        self.agents.values().map(|m| &m.agent_card).collect()
    }

    /// Check if a task requires commitment.
    pub fn requires_commitment(&self, agent_name: &str) -> bool {
        self.agents
            .get(agent_name)
            .map(|m| m.requires_commitment)
            .unwrap_or(true)
    }
}

/// The main A2A protocol adapter.
pub struct A2aAdapter {
    /// Agent registry.
    registry: A2aAgentRegistry,
    /// Task handlers by agent name.
    handlers: HashMap<String, Box<dyn A2aTaskHandler>>,
    /// Principal identity.
    principal: IdentityRef,
    /// Active tasks.
    tasks: RwLock<HashMap<String, A2aTask>>,
    /// Task records for audit.
    task_records: RwLock<Vec<A2aTaskRecord>>,
    /// Configuration.
    config: A2aAdapterConfig,
}

/// Configuration for the A2A adapter.
#[derive(Debug, Clone)]
pub struct A2aAdapterConfig {
    /// Require commitments for all consequential tasks.
    pub require_commitments: bool,
    /// Maximum concurrent tasks.
    pub max_concurrent_tasks: usize,
    /// Task history retention.
    pub task_history_retention: usize,
    /// Enable push notifications.
    pub enable_push_notifications: bool,
}

impl Default for A2aAdapterConfig {
    fn default() -> Self {
        Self {
            require_commitments: true,
            max_concurrent_tasks: 100,
            task_history_retention: 1000,
            enable_push_notifications: false,
        }
    }
}

impl A2aAdapter {
    /// Create a new A2A adapter.
    pub fn new(principal: IdentityRef) -> Self {
        Self {
            registry: A2aAgentRegistry::new(),
            handlers: HashMap::new(),
            principal,
            tasks: RwLock::new(HashMap::new()),
            task_records: RwLock::new(Vec::new()),
            config: A2aAdapterConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(mut self, config: A2aAdapterConfig) -> Self {
        self.config = config;
        self
    }

    /// Register an agent.
    pub fn register_agent(&mut self, mapping: A2aResonatorMapping) {
        self.registry.register(mapping);
    }

    /// Register a task handler for an agent.
    pub fn register_handler(&mut self, agent_name: &str, handler: Box<dyn A2aTaskHandler>) {
        self.handlers.insert(agent_name.to_string(), handler);
    }

    /// Get agent card (A2A /.well-known/agent.json response).
    pub fn get_agent_card(&self, agent_name: &str) -> Option<A2aAgentCard> {
        self.registry.get(agent_name).map(|m| m.agent_card.clone())
    }

    /// List available agents.
    pub fn list_agents(&self) -> Vec<A2aAgentCard> {
        self.registry.list_agents().into_iter().cloned().collect()
    }

    /// Build a commitment for an A2A task.
    pub fn build_commitment(
        &self,
        agent_name: &str,
        task_request: &A2aTaskSendRequest,
    ) -> Result<RcfCommitment, A2aAdapterError> {
        let mapping = self
            .registry
            .get(agent_name)
            .ok_or_else(|| A2aAdapterError::AgentNotFound(agent_name.to_string()))?;

        let description = task_request
            .message
            .parts
            .first()
            .map(|p| match p {
                A2aPart::Text { text } => text.chars().take(100).collect::<String>(),
                _ => "A2A task".to_string(),
            })
            .unwrap_or_else(|| "A2A task".to_string());

        let commitment = CommitmentBuilder::new(self.principal.clone(), mapping.domain.clone())
            .with_scope(mapping.scope.clone())
            .with_outcome(rcf_commitment::IntendedOutcome::new(format!(
                "Execute A2A task for agent {}: {}",
                agent_name, description
            )))
            .with_policy_tag("a2a-task")
            .build()
            .map_err(|e| A2aAdapterError::CommitmentBuildError(e.to_string()))?;

        Ok(commitment)
    }

    /// Send a task to an agent.
    pub async fn send_task(
        &self,
        agent_name: &str,
        request: A2aTaskSendRequest,
        commitment: Option<RcfCommitment>,
    ) -> Result<A2aTask, A2aAdapterError> {
        let mapping = self
            .registry
            .get(agent_name)
            .ok_or_else(|| A2aAdapterError::AgentNotFound(agent_name.to_string()))?;

        // Check commitment requirement
        if mapping.requires_commitment && commitment.is_none() && self.config.require_commitments {
            return Err(A2aAdapterError::CommitmentRequired(agent_name.to_string()));
        }

        // Check concurrent task limit
        {
            let tasks = self.tasks.read().map_err(|_| A2aAdapterError::LockError)?;
            let active_count = tasks.values().filter(|t| !t.status.is_terminal()).count();
            if active_count >= self.config.max_concurrent_tasks {
                return Err(A2aAdapterError::TooManyTasks);
            }
        }

        // Get handler
        let handler = self
            .handlers
            .get(agent_name)
            .ok_or_else(|| A2aAdapterError::NoHandler(agent_name.to_string()))?;

        // Execute task
        let task = handler.handle_task(request.clone()).await?;

        // Store task
        {
            let mut tasks = self.tasks.write().map_err(|_| A2aAdapterError::LockError)?;
            tasks.insert(task.id.clone(), task.clone());
        }

        // Record for audit
        let record = A2aTaskRecord {
            task_id: task.id.clone(),
            session_id: task.session_id.clone(),
            source_agent: agent_name.to_string(),
            target_resonator: mapping.resonator_id.to_string(),
            commitment_id: commitment.map(|c| c.commitment_id),
            status: task.status.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            message_count: task.history.len(),
        };

        {
            let mut records = self
                .task_records
                .write()
                .map_err(|_| A2aAdapterError::LockError)?;
            records.push(record);
            while records.len() > self.config.task_history_retention {
                records.remove(0);
            }
        }

        Ok(task)
    }

    /// Get a task by ID.
    pub fn get_task(&self, task_id: &str) -> Result<Option<A2aTask>, A2aAdapterError> {
        let tasks = self.tasks.read().map_err(|_| A2aAdapterError::LockError)?;
        Ok(tasks.get(task_id).cloned())
    }

    /// Update task status.
    pub fn update_task_status(
        &self,
        task_id: &str,
        status: A2aTaskStatus,
    ) -> Result<(), A2aAdapterError> {
        let mut tasks = self.tasks.write().map_err(|_| A2aAdapterError::LockError)?;

        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| A2aAdapterError::TaskNotFound(task_id.to_string()))?;

        task.status = status;
        Ok(())
    }

    /// Add message to task history.
    pub fn add_message_to_task(
        &self,
        task_id: &str,
        message: A2aMessage,
    ) -> Result<(), A2aAdapterError> {
        let mut tasks = self.tasks.write().map_err(|_| A2aAdapterError::LockError)?;

        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| A2aAdapterError::TaskNotFound(task_id.to_string()))?;

        task.history.push(message);
        Ok(())
    }

    /// Add artifact to task.
    pub fn add_artifact_to_task(
        &self,
        task_id: &str,
        artifact: A2aArtifact,
    ) -> Result<(), A2aAdapterError> {
        let mut tasks = self.tasks.write().map_err(|_| A2aAdapterError::LockError)?;

        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| A2aAdapterError::TaskNotFound(task_id.to_string()))?;

        task.artifacts.push(artifact);
        Ok(())
    }

    /// Get task records for audit.
    pub fn task_records(&self, limit: usize) -> Vec<A2aTaskRecord> {
        self.task_records
            .read()
            .map(|r| r.iter().rev().take(limit).cloned().collect())
            .unwrap_or_default()
    }

    /// Convert MAPLE resonator profile to A2A agent card.
    pub fn profile_to_agent_card(
        _resonator_id: &ResonatorId,
        profile: &ResonatorProfile,
        url: &str,
    ) -> A2aAgentCard {
        let skills: Vec<A2aSkill> = profile
            .domains
            .iter()
            .map(|domain| A2aSkill {
                id: format!("{}-skill", domain),
                name: domain.to_string(),
                description: Some(format!("Operations in {} domain", domain)),
                tags: vec![domain.to_string()],
                examples: vec![],
                input_schema: None,
                output_schema: None,
            })
            .collect();

        A2aAgentCard {
            name: profile.name.clone(),
            description: profile.description.clone(),
            url: url.to_string(),
            provider: Some(A2aProvider {
                organization: "MAPLE".to_string(),
                url: None,
            }),
            version: Some("1.0.0".to_string()),
            capabilities: A2aCapabilities {
                streaming: true,
                push_notifications: false,
                state_transfer_modes: vec!["full".to_string()],
            },
            authentication: None,
            skills,
            default_input_modes: vec!["text".to_string()],
            default_output_modes: vec!["text".to_string()],
        }
    }
}

/// A2A adapter errors.
#[derive(Debug, Error)]
pub enum A2aAdapterError {
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Commitment required for agent: {0}")]
    CommitmentRequired(String),

    #[error("No handler registered for agent: {0}")]
    NoHandler(String),

    #[error("Commitment build error: {0}")]
    CommitmentBuildError(String),

    #[error("Too many concurrent tasks")]
    TooManyTasks,

    #[error("Lock error")]
    LockError,

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Simple echo task handler for testing.
pub struct EchoTaskHandler;

#[async_trait]
impl A2aTaskHandler for EchoTaskHandler {
    async fn handle_task(&self, request: A2aTaskSendRequest) -> Result<A2aTask, A2aAdapterError> {
        let task_id = request
            .id
            .unwrap_or_else(|| format!("task-{}", uuid::Uuid::new_v4()));

        let response_text = request
            .message
            .parts
            .iter()
            .filter_map(|p| match p {
                A2aPart::Text { text } => Some(format!("Echo: {}", text)),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        let response_message = A2aMessage {
            role: A2aRole::Agent,
            parts: vec![A2aPart::Text {
                text: if response_text.is_empty() {
                    "Echo: (no text content)".to_string()
                } else {
                    response_text
                },
            }],
            timestamp: Some(Utc::now()),
        };

        Ok(A2aTask {
            id: task_id,
            session_id: request.session_id,
            status: A2aTaskStatus::Completed,
            history: vec![request.message, response_message],
            artifacts: vec![],
            metadata: request.metadata,
        })
    }

    async fn get_task(&self, _task_id: &str) -> Result<Option<A2aTask>, A2aAdapterError> {
        Ok(None) // Echo handler doesn't persist tasks
    }

    async fn cancel_task(&self, task_id: &str) -> Result<A2aTask, A2aAdapterError> {
        Err(A2aAdapterError::TaskNotFound(task_id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_agent_card() -> A2aAgentCard {
        A2aAgentCard {
            name: "test-agent".to_string(),
            description: "A test agent".to_string(),
            url: "https://example.com/agent".to_string(),
            provider: None,
            version: Some("1.0".to_string()),
            capabilities: A2aCapabilities::default(),
            authentication: None,
            skills: vec![A2aSkill {
                id: "test-skill".to_string(),
                name: "Test Skill".to_string(),
                description: Some("A test skill".to_string()),
                tags: vec![],
                examples: vec![],
                input_schema: None,
                output_schema: None,
            }],
            default_input_modes: vec!["text".to_string()],
            default_output_modes: vec!["text".to_string()],
        }
    }

    #[test]
    fn test_agent_card_serialization() {
        let card = sample_agent_card();
        let json = serde_json::to_string(&card).unwrap();
        let parsed: A2aAgentCard = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test-agent");
    }

    #[test]
    fn test_task_status() {
        assert!(!A2aTaskStatus::Submitted.is_terminal());
        assert!(!A2aTaskStatus::Working.is_terminal());
        assert!(A2aTaskStatus::Completed.is_terminal());
        assert!(A2aTaskStatus::Failed.is_terminal());
        assert!(A2aTaskStatus::Canceled.is_terminal());
    }

    #[test]
    fn test_agent_registry() {
        let mut registry = A2aAgentRegistry::new();

        let card = sample_agent_card();
        let resonator_id = ResonatorId::new("res-123");
        let mapping = A2aResonatorMapping::new(card, resonator_id.clone());

        registry.register(mapping);

        assert!(registry.get("test-agent").is_some());
        assert!(registry.get_by_resonator(&resonator_id).is_some());
        assert!(!registry.requires_commitment("test-agent"));
    }

    #[test]
    fn test_adapter_registration() {
        let mut adapter = A2aAdapter::new(IdentityRef::new("test-principal"));

        let card = sample_agent_card();
        let resonator_id = ResonatorId::new("res-123");
        let mapping = A2aResonatorMapping::new(card, resonator_id);

        adapter.register_agent(mapping);

        let agents = adapter.list_agents();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].name, "test-agent");
    }

    #[test]
    fn test_commitment_building() {
        let mut adapter = A2aAdapter::new(IdentityRef::new("test-principal"));

        let card = sample_agent_card();
        let resonator_id = ResonatorId::new("res-123");
        let mapping =
            A2aResonatorMapping::consequential(card, resonator_id, EffectDomain::Computation);

        adapter.register_agent(mapping);

        let request = A2aTaskSendRequest {
            id: None,
            session_id: None,
            message: A2aMessage {
                role: A2aRole::User,
                parts: vec![A2aPart::Text {
                    text: "Hello".to_string(),
                }],
                timestamp: None,
            },
            push_notification: None,
            metadata: HashMap::new(),
        };

        let commitment = adapter.build_commitment("test-agent", &request).unwrap();
        assert_eq!(commitment.effect_domain, EffectDomain::Computation);
    }

    #[tokio::test]
    async fn test_echo_task_handler() {
        let handler = EchoTaskHandler;

        let request = A2aTaskSendRequest {
            id: Some("test-task".to_string()),
            session_id: None,
            message: A2aMessage {
                role: A2aRole::User,
                parts: vec![A2aPart::Text {
                    text: "Hello, A2A!".to_string(),
                }],
                timestamp: None,
            },
            push_notification: None,
            metadata: HashMap::new(),
        };

        let task = handler.handle_task(request).await.unwrap();

        assert_eq!(task.id, "test-task");
        assert_eq!(task.status, A2aTaskStatus::Completed);
        assert_eq!(task.history.len(), 2);

        // Check echo response
        match &task.history[1].parts[0] {
            A2aPart::Text { text } => {
                assert!(text.contains("Hello, A2A!"));
            }
            _ => panic!("Expected text part"),
        }
    }

    #[test]
    fn test_profile_to_agent_card() {
        use resonator_types::{AutonomyLevel, RiskTolerance};

        let profile = ResonatorProfile {
            name: "TestResonator".to_string(),
            description: "A test resonator".to_string(),
            domains: vec![EffectDomain::Computation, EffectDomain::Data],
            risk_tolerance: RiskTolerance::Balanced,
            autonomy_level: AutonomyLevel::GuidedAutonomy,
            constraints: vec![],
        };

        let resonator_id = ResonatorId::new("res-456");
        let card =
            A2aAdapter::profile_to_agent_card(&resonator_id, &profile, "https://example.com/agent");

        assert_eq!(card.name, "TestResonator");
        assert_eq!(card.skills.len(), 2);
        assert!(card.capabilities.streaming);
    }
}
