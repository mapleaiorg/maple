//! Multi-Turn Conversation Support for MAPLE Resonators
//!
//! This module implements conversation session management for the Resonance
//! Architecture. Conversations maintain context across multiple turns while
//! integrating with the memory system for long-term context preservation.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                  CONVERSATION MANAGER                           │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │   ┌───────────────┐   ┌───────────────┐   ┌───────────────┐   │
//! │   │    Session    │──▶│     Turn      │──▶│   Response    │   │
//! │   │   Manager     │   │   Processor   │   │   Generator   │   │
//! │   └───────────────┘   └───────────────┘   └───────────────┘   │
//! │          │                   │                   │             │
//! │          ▼                   ▼                   ▼             │
//! │   ┌─────────────────────────────────────────────────────────┐ │
//! │   │              Context Window Manager                     │ │
//! │   │    (maintains relevant context within token limits)     │ │
//! │   └─────────────────────────────────────────────────────────┘ │
//! │                            │                                   │
//! │                            ▼                                   │
//! │   ┌─────────────────────────────────────────────────────────┐ │
//! │   │                 Memory Integration                      │ │
//! │   │         (short-term, working, long-term recall)         │ │
//! │   └─────────────────────────────────────────────────────────┘ │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Components
//!
//! - [`ConversationSession`]: A multi-turn conversation session
//! - [`ConversationTurn`]: A single turn (message + response) in a conversation
//! - [`ContextWindow`]: Manages context within token limits
//! - [`SessionManager`]: Manages multiple concurrent sessions

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::RwLock;

use chrono::{DateTime, Duration, Utc};
use resonator_memory::MemoryId;
use resonator_types::ResonatorId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Unique identifier for a conversation session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn generate() -> Self {
        Self(format!("session-{}", uuid::Uuid::new_v4()))
    }

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a conversation turn.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TurnId(pub String);

impl TurnId {
    pub fn generate() -> Self {
        Self(format!("turn-{}", uuid::Uuid::new_v4()))
    }
}

/// Role in a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversationRole {
    /// Human user.
    User,
    /// AI assistant/resonator.
    Assistant,
    /// System message.
    System,
    /// Tool/function result.
    Tool { tool_name: String },
}

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// Role of the message sender.
    pub role: ConversationRole,
    /// Message content.
    pub content: String,
    /// Optional structured content.
    pub structured_content: Option<serde_json::Value>,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
    /// Token count estimate.
    pub token_estimate: usize,
    /// Message metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ConversationMessage {
    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        let content = content.into();
        let token_estimate = estimate_tokens(&content);
        Self {
            role: ConversationRole::User,
            content,
            structured_content: None,
            timestamp: Utc::now(),
            token_estimate,
            metadata: HashMap::new(),
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        let content = content.into();
        let token_estimate = estimate_tokens(&content);
        Self {
            role: ConversationRole::Assistant,
            content,
            structured_content: None,
            timestamp: Utc::now(),
            token_estimate,
            metadata: HashMap::new(),
        }
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        let content = content.into();
        let token_estimate = estimate_tokens(&content);
        Self {
            role: ConversationRole::System,
            content,
            structured_content: None,
            timestamp: Utc::now(),
            token_estimate,
            metadata: HashMap::new(),
        }
    }

    /// Create a tool result message.
    pub fn tool(tool_name: impl Into<String>, content: impl Into<String>) -> Self {
        let content = content.into();
        let token_estimate = estimate_tokens(&content);
        Self {
            role: ConversationRole::Tool {
                tool_name: tool_name.into(),
            },
            content,
            structured_content: None,
            timestamp: Utc::now(),
            token_estimate,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Add structured content.
    pub fn with_structured(mut self, content: serde_json::Value) -> Self {
        self.structured_content = Some(content);
        self
    }
}

/// A single turn in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    /// Turn ID.
    pub id: TurnId,
    /// Turn number (1-indexed).
    pub turn_number: usize,
    /// Input message (usually user).
    pub input: ConversationMessage,
    /// Output message (usually assistant).
    pub output: Option<ConversationMessage>,
    /// Tool calls made during this turn.
    pub tool_calls: Vec<ToolCall>,
    /// Turn started at.
    pub started_at: DateTime<Utc>,
    /// Turn completed at.
    pub completed_at: Option<DateTime<Utc>>,
    /// Processing duration in milliseconds.
    pub processing_ms: Option<i64>,
    /// Associated memory IDs.
    pub memory_ids: Vec<MemoryId>,
}

impl ConversationTurn {
    /// Create a new turn.
    pub fn new(turn_number: usize, input: ConversationMessage) -> Self {
        Self {
            id: TurnId::generate(),
            turn_number,
            input,
            output: None,
            tool_calls: Vec::new(),
            started_at: Utc::now(),
            completed_at: None,
            processing_ms: None,
            memory_ids: Vec::new(),
        }
    }

    /// Complete the turn with a response.
    pub fn complete(&mut self, output: ConversationMessage) {
        let now = Utc::now();
        self.output = Some(output);
        self.completed_at = Some(now);
        self.processing_ms = Some((now - self.started_at).num_milliseconds());
    }

    /// Add a tool call.
    pub fn add_tool_call(&mut self, tool_call: ToolCall) {
        self.tool_calls.push(tool_call);
    }

    /// Total tokens for this turn.
    pub fn total_tokens(&self) -> usize {
        let input_tokens = self.input.token_estimate;
        let output_tokens = self.output.as_ref().map(|o| o.token_estimate).unwrap_or(0);
        let tool_tokens: usize = self.tool_calls.iter().map(|t| t.token_estimate()).sum();
        input_tokens + output_tokens + tool_tokens
    }
}

/// A tool call made during a turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool name.
    pub tool_name: String,
    /// Tool arguments.
    pub arguments: serde_json::Value,
    /// Tool result.
    pub result: Option<String>,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
}

impl ToolCall {
    pub fn new(tool_name: impl Into<String>, arguments: serde_json::Value) -> Self {
        Self {
            tool_name: tool_name.into(),
            arguments,
            result: None,
            timestamp: Utc::now(),
        }
    }

    pub fn with_result(mut self, result: impl Into<String>) -> Self {
        self.result = Some(result.into());
        self
    }

    pub fn token_estimate(&self) -> usize {
        let args_str = self.arguments.to_string();
        let result_tokens = self.result.as_ref().map(|r| estimate_tokens(r)).unwrap_or(0);
        estimate_tokens(&self.tool_name) + estimate_tokens(&args_str) + result_tokens
    }
}

/// Session status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    /// Session is active.
    Active,
    /// Session is paused.
    Paused,
    /// Session completed normally.
    Completed,
    /// Session expired.
    Expired,
    /// Session terminated by user.
    Terminated,
}

/// A conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSession {
    /// Session ID.
    pub id: SessionId,
    /// Resonator handling this session.
    pub resonator_id: ResonatorId,
    /// User identity.
    pub user_id: Option<String>,
    /// Session title/topic.
    pub title: Option<String>,
    /// System prompt for this session.
    pub system_prompt: Option<String>,
    /// Session status.
    pub status: SessionStatus,
    /// Conversation turns.
    pub turns: Vec<ConversationTurn>,
    /// Session created at.
    pub created_at: DateTime<Utc>,
    /// Session last active at.
    pub last_active_at: DateTime<Utc>,
    /// Session expires at.
    pub expires_at: Option<DateTime<Utc>>,
    /// Total tokens used.
    pub total_tokens: usize,
    /// Token limit for context window.
    pub token_limit: usize,
    /// Session metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ConversationSession {
    /// Create a new session.
    pub fn new(resonator_id: ResonatorId) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::generate(),
            resonator_id,
            user_id: None,
            title: None,
            system_prompt: None,
            status: SessionStatus::Active,
            turns: Vec::new(),
            created_at: now,
            last_active_at: now,
            expires_at: Some(now + Duration::hours(24)), // Default 24h expiry
            total_tokens: 0,
            token_limit: 128000, // Default to GPT-4 context
            metadata: HashMap::new(),
        }
    }

    /// Set user ID.
    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set token limit.
    pub fn with_token_limit(mut self, limit: usize) -> Self {
        self.token_limit = limit;
        self
    }

    /// Set expiry.
    pub fn with_expiry(mut self, duration: Duration) -> Self {
        self.expires_at = Some(Utc::now() + duration);
        self
    }

    /// Add a new turn.
    pub fn add_turn(&mut self, input: ConversationMessage) -> &mut ConversationTurn {
        let turn_number = self.turns.len() + 1;
        let turn = ConversationTurn::new(turn_number, input);
        self.turns.push(turn);
        self.last_active_at = Utc::now();
        self.turns.last_mut().unwrap()
    }

    /// Complete the current turn.
    pub fn complete_turn(&mut self, output: ConversationMessage) {
        if let Some(turn) = self.turns.last_mut() {
            turn.complete(output);
            self.total_tokens += turn.total_tokens();
            self.last_active_at = Utc::now();
        }
    }

    /// Get the current turn (if incomplete).
    pub fn current_turn(&self) -> Option<&ConversationTurn> {
        self.turns.last().filter(|t| t.output.is_none())
    }

    /// Get the current turn mutably.
    pub fn current_turn_mut(&mut self) -> Option<&mut ConversationTurn> {
        self.turns.last_mut().filter(|t| t.output.is_none())
    }

    /// Check if session is expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| Utc::now() > exp)
            .unwrap_or(false)
    }

    /// Check if session is active.
    pub fn is_active(&self) -> bool {
        matches!(self.status, SessionStatus::Active) && !self.is_expired()
    }

    /// Get available token budget.
    pub fn available_tokens(&self) -> usize {
        self.token_limit.saturating_sub(self.total_tokens)
    }

    /// Get messages for context window.
    pub fn get_context_messages(&self, max_tokens: usize) -> Vec<&ConversationMessage> {
        let mut messages = Vec::new();
        let mut token_count = 0;

        // Add system prompt if present
        if let Some(ref prompt) = self.system_prompt {
            token_count += estimate_tokens(prompt);
        }

        // Add turns in reverse order until we hit the limit
        for turn in self.turns.iter().rev() {
            let turn_tokens = turn.total_tokens();
            if token_count + turn_tokens > max_tokens {
                break;
            }
            token_count += turn_tokens;

            if let Some(ref output) = turn.output {
                messages.push(output);
            }
            messages.push(&turn.input);
        }

        messages.reverse();
        messages
    }
}

/// Context window manager.
#[derive(Debug)]
pub struct ContextWindow {
    /// Maximum tokens.
    pub max_tokens: usize,
    /// Reserved tokens for response.
    pub response_reserve: usize,
    /// Summarization threshold.
    pub summarize_threshold: f64,
}

impl ContextWindow {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            response_reserve: max_tokens / 4, // Reserve 25% for response
            summarize_threshold: 0.8, // Summarize when 80% full
        }
    }

    /// Available tokens for context.
    pub fn available_for_context(&self) -> usize {
        self.max_tokens.saturating_sub(self.response_reserve)
    }

    /// Check if summarization is needed.
    pub fn needs_summarization(&self, current_tokens: usize) -> bool {
        let threshold = (self.max_tokens as f64 * self.summarize_threshold) as usize;
        current_tokens >= threshold
    }

    /// Truncate messages to fit context.
    pub fn fit_to_context<'a>(
        &self,
        messages: &'a [ConversationMessage],
    ) -> Vec<&'a ConversationMessage> {
        let available = self.available_for_context();
        let mut result = Vec::new();
        let mut total = 0;

        // Always include the last message (current input)
        if let Some(last) = messages.last() {
            result.push(last);
            total += last.token_estimate;
        }

        // Add messages from end, skipping the last (already added)
        for msg in messages.iter().rev().skip(1) {
            if total + msg.token_estimate > available {
                break;
            }
            total += msg.token_estimate;
            result.push(msg);
        }

        result.reverse();
        result
    }
}

impl Default for ContextWindow {
    fn default() -> Self {
        Self::new(128000) // Default to GPT-4 context
    }
}

/// Configuration for the session manager.
#[derive(Debug, Clone)]
pub struct SessionManagerConfig {
    /// Maximum concurrent sessions.
    pub max_sessions: usize,
    /// Default session expiry.
    pub default_expiry_hours: i64,
    /// Token limit per session.
    pub token_limit: usize,
    /// Enable memory integration.
    pub enable_memory: bool,
}

impl Default for SessionManagerConfig {
    fn default() -> Self {
        Self {
            max_sessions: 1000,
            default_expiry_hours: 24,
            token_limit: 128000,
            enable_memory: true,
        }
    }
}

/// Session manager for handling multiple conversations.
pub struct SessionManager {
    sessions: RwLock<HashMap<SessionId, ConversationSession>>,
    by_user: RwLock<HashMap<String, Vec<SessionId>>>,
    by_resonator: RwLock<HashMap<ResonatorId, Vec<SessionId>>>,
    config: SessionManagerConfig,
}

impl SessionManager {
    /// Create a new session manager.
    pub fn new(config: SessionManagerConfig) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            by_user: RwLock::new(HashMap::new()),
            by_resonator: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Create a new session.
    pub fn create_session(
        &self,
        resonator_id: ResonatorId,
    ) -> Result<SessionId, ConversationError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| ConversationError::LockError)?;

        // Check capacity
        if sessions.len() >= self.config.max_sessions {
            // Try to cleanup expired sessions
            sessions.retain(|_, s| !s.is_expired());
            if sessions.len() >= self.config.max_sessions {
                return Err(ConversationError::TooManySessions);
            }
        }

        let session = ConversationSession::new(resonator_id.clone())
            .with_token_limit(self.config.token_limit)
            .with_expiry(Duration::hours(self.config.default_expiry_hours));

        let id = session.id.clone();
        sessions.insert(id.clone(), session);

        // Index by resonator
        let mut by_resonator = self
            .by_resonator
            .write()
            .map_err(|_| ConversationError::LockError)?;
        by_resonator
            .entry(resonator_id)
            .or_default()
            .push(id.clone());

        Ok(id)
    }

    /// Get a session.
    pub fn get_session(&self, id: &SessionId) -> Result<ConversationSession, ConversationError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| ConversationError::LockError)?;

        sessions
            .get(id)
            .cloned()
            .ok_or_else(|| ConversationError::SessionNotFound(id.0.clone()))
    }

    /// Update a session.
    pub fn update_session(
        &self,
        session: ConversationSession,
    ) -> Result<(), ConversationError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| ConversationError::LockError)?;

        if !sessions.contains_key(&session.id) {
            return Err(ConversationError::SessionNotFound(session.id.0.clone()));
        }

        sessions.insert(session.id.clone(), session);
        Ok(())
    }

    /// Add a message to a session and create a turn.
    pub fn add_message(
        &self,
        session_id: &SessionId,
        message: ConversationMessage,
    ) -> Result<TurnId, ConversationError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| ConversationError::LockError)?;

        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(session_id.0.clone()))?;

        if !session.is_active() {
            return Err(ConversationError::SessionInactive(session_id.0.clone()));
        }

        let turn = session.add_turn(message);
        Ok(turn.id.clone())
    }

    /// Complete a turn with a response.
    pub fn complete_turn(
        &self,
        session_id: &SessionId,
        response: ConversationMessage,
    ) -> Result<(), ConversationError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| ConversationError::LockError)?;

        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(session_id.0.clone()))?;

        session.complete_turn(response);
        Ok(())
    }

    /// Get sessions for a resonator.
    pub fn get_sessions_for_resonator(
        &self,
        resonator_id: &ResonatorId,
    ) -> Result<Vec<SessionId>, ConversationError> {
        let by_resonator = self
            .by_resonator
            .read()
            .map_err(|_| ConversationError::LockError)?;

        Ok(by_resonator
            .get(resonator_id)
            .cloned()
            .unwrap_or_default())
    }

    /// Get sessions for a user.
    pub fn get_sessions_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<SessionId>, ConversationError> {
        let by_user = self
            .by_user
            .read()
            .map_err(|_| ConversationError::LockError)?;

        Ok(by_user.get(user_id).cloned().unwrap_or_default())
    }

    /// End a session.
    pub fn end_session(&self, session_id: &SessionId) -> Result<(), ConversationError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| ConversationError::LockError)?;

        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(session_id.0.clone()))?;

        session.status = SessionStatus::Completed;
        Ok(())
    }

    /// Cleanup expired sessions.
    pub fn cleanup_expired(&self) -> Result<usize, ConversationError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| ConversationError::LockError)?;

        let before = sessions.len();
        sessions.retain(|_, s| !s.is_expired());
        Ok(before - sessions.len())
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(SessionManagerConfig::default())
    }
}

/// Conversation errors.
#[derive(Debug, Error)]
pub enum ConversationError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session inactive: {0}")]
    SessionInactive(String),

    #[error("Too many sessions")]
    TooManySessions,

    #[error("Token limit exceeded")]
    TokenLimitExceeded,

    #[error("Lock error")]
    LockError,

    #[error("Memory error: {0}")]
    MemoryError(String),
}

/// Estimate token count for a string (rough approximation).
fn estimate_tokens(text: &str) -> usize {
    // Rough approximation: ~4 characters per token for English
    (text.len() + 3) / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let resonator_id = ResonatorId::new("test-resonator");
        let session = ConversationSession::new(resonator_id.clone())
            .with_title("Test Session")
            .with_user("user-123");

        assert!(session.is_active());
        assert_eq!(session.turns.len(), 0);
        assert_eq!(session.title, Some("Test Session".to_string()));
    }

    #[test]
    fn test_conversation_turn() {
        let resonator_id = ResonatorId::new("test-resonator");
        let mut session = ConversationSession::new(resonator_id);

        // Add a turn
        let input = ConversationMessage::user("Hello!");
        session.add_turn(input);

        assert_eq!(session.turns.len(), 1);
        assert!(session.current_turn().is_some());

        // Complete the turn
        let output = ConversationMessage::assistant("Hi there!");
        session.complete_turn(output);

        assert!(session.current_turn().is_none());
        assert!(session.turns[0].output.is_some());
        assert!(session.turns[0].processing_ms.is_some());
    }

    #[test]
    fn test_token_estimation() {
        assert_eq!(estimate_tokens("Hello"), 2); // 5 chars ~= 2 tokens
        assert_eq!(estimate_tokens("Hello, world!"), 4); // 13 chars ~= 4 tokens
    }

    #[test]
    fn test_context_window() {
        let window = ContextWindow::new(1000);

        assert_eq!(window.available_for_context(), 750); // 75% for context
        assert!(!window.needs_summarization(500));
        assert!(window.needs_summarization(850)); // 85% > 80% threshold
    }

    #[test]
    fn test_session_manager() {
        let manager = SessionManager::default();

        let resonator_id = ResonatorId::new("test-resonator");
        let session_id = manager.create_session(resonator_id.clone()).unwrap();

        // Add a message
        let _turn_id = manager
            .add_message(&session_id, ConversationMessage::user("Hello"))
            .unwrap();

        // Complete the turn
        manager
            .complete_turn(&session_id, ConversationMessage::assistant("Hi!"))
            .unwrap();

        // Get session
        let session = manager.get_session(&session_id).unwrap();
        assert_eq!(session.turns.len(), 1);
        assert!(session.turns[0].output.is_some());
    }

    #[test]
    fn test_message_roles() {
        let user_msg = ConversationMessage::user("User message");
        let assistant_msg = ConversationMessage::assistant("Assistant message");
        let system_msg = ConversationMessage::system("System message");
        let tool_msg = ConversationMessage::tool("my_tool", "Tool result");

        assert!(matches!(user_msg.role, ConversationRole::User));
        assert!(matches!(assistant_msg.role, ConversationRole::Assistant));
        assert!(matches!(system_msg.role, ConversationRole::System));
        assert!(matches!(tool_msg.role, ConversationRole::Tool { .. }));
    }

    #[test]
    fn test_tool_call() {
        let tool_call = ToolCall::new("search", serde_json::json!({"query": "test"}))
            .with_result("Found 5 results");

        assert_eq!(tool_call.tool_name, "search");
        assert!(tool_call.result.is_some());
        assert!(tool_call.token_estimate() > 0);
    }

    #[test]
    fn test_session_expiry() {
        let resonator_id = ResonatorId::new("test-resonator");
        let session = ConversationSession::new(resonator_id)
            .with_expiry(Duration::seconds(-1)); // Already expired

        assert!(session.is_expired());
        assert!(!session.is_active());
    }

    #[test]
    fn test_context_messages() {
        let resonator_id = ResonatorId::new("test-resonator");
        let mut session = ConversationSession::new(resonator_id).with_token_limit(1000);

        // Add multiple turns with longer messages to test truncation
        for i in 0..10 {
            let long_user_msg = format!("User message {} with some extra content to increase token count and test context window truncation properly", i);
            let long_assistant_msg = format!("Assistant message {} with additional content that takes up more tokens for testing purposes", i);
            session.add_turn(ConversationMessage::user(long_user_msg));
            session.complete_turn(ConversationMessage::assistant(long_assistant_msg));
        }

        // Get context messages with a small token limit
        let context = session.get_context_messages(50);
        // With a 50 token limit, we should get fewer than all 20 messages
        assert!(context.len() < 20);
        // Should have at least 1 message (the most recent)
        assert!(!context.is_empty());
    }
}
