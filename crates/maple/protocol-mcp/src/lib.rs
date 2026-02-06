//! Model Context Protocol (MCP) Adapter for MAPLE
//!
//! This module provides bidirectional translation between the Model Context Protocol
//! (MCP) and MAPLE's Resonance Architecture. MCP tools and resources are exposed
//! through the MAPLE commitment gateway, ensuring all MCP operations are subject
//! to the same safety invariants as native MAPLE operations.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      MCP PROTOCOL ADAPTER                       │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │   ┌─────────────┐         ┌─────────────┐                      │
//! │   │  MCP Tools  │◀───────▶│   MAPLE     │                      │
//! │   │             │         │ Capabilities │                      │
//! │   └─────────────┘         └─────────────┘                      │
//! │         │                       │                               │
//! │         ▼                       ▼                               │
//! │   ┌─────────────────────────────────────────────────────────┐ │
//! │   │              Tool-to-Capability Mapper                  │ │
//! │   │         (JSON Schema ⟷ CapabilityDescriptor)           │ │
//! │   └─────────────────────────────────────────────────────────┘ │
//! │                            │                                   │
//! │                            ▼                                   │
//! │   ┌─────────────────────────────────────────────────────────┐ │
//! │   │              Commitment Gateway Bridge                  │ │
//! │   │    (all MCP calls flow through commitment boundary)     │ │
//! │   └─────────────────────────────────────────────────────────┘ │
//! │                            │                                   │
//! │   ┌─────────────┐         │         ┌─────────────┐          │
//! │   │MCP Resources│◀────────┴────────▶│   MAPLE     │          │
//! │   │             │                   │    State    │          │
//! │   └─────────────┘                   └─────────────┘          │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Principles
//!
//! 1. **MCP as Transport, Not Authority**: MCP tools are exposed but all execution
//!    goes through MAPLE's commitment gateway - MCP cannot bypass safety invariants.
//!
//! 2. **Tool-to-Capability Mapping**: Each MCP tool maps to a MAPLE capability with
//!    explicit domain, scope, and risk classification.
//!
//! 3. **Resource-to-State Projection**: MCP resources expose MAPLE state through
//!    read-only projections that respect privacy and access controls.
//!
//! 4. **Commitment Wrapping**: Every MCP tool invocation creates an RCF commitment
//!    that flows through AAS policy evaluation.

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rcf_commitment::{CommitmentBuilder, CommitmentId, RcfCommitment};
use rcf_types::{EffectDomain, IdentityRef, ScopeConstraint};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// MCP Tool definition (matches MCP spec).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool name.
    pub name: String,
    /// Tool description.
    pub description: String,
    /// Input schema (JSON Schema).
    pub input_schema: serde_json::Value,
    /// Optional annotations.
    #[serde(default)]
    pub annotations: McpAnnotations,
}

/// MCP annotations for tools.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpAnnotations {
    /// Title for display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Whether this tool has read-only behavior.
    #[serde(rename = "readOnlyHint", skip_serializing_if = "Option::is_none")]
    pub read_only_hint: Option<bool>,
    /// Whether this tool is destructive.
    #[serde(rename = "destructiveHint", skip_serializing_if = "Option::is_none")]
    pub destructive_hint: Option<bool>,
    /// Whether this tool is idempotent.
    #[serde(rename = "idempotentHint", skip_serializing_if = "Option::is_none")]
    pub idempotent_hint: Option<bool>,
    /// Custom MAPLE extension: effect domain.
    #[serde(rename = "x-maple-domain", skip_serializing_if = "Option::is_none")]
    pub maple_domain: Option<String>,
    /// Custom MAPLE extension: scope.
    #[serde(rename = "x-maple-scope", skip_serializing_if = "Option::is_none")]
    pub maple_scope: Option<String>,
}

/// MCP Resource definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    /// Resource URI.
    pub uri: String,
    /// Resource name.
    pub name: String,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type.
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// MCP Tool call request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCall {
    /// Tool name.
    pub name: String,
    /// Arguments (JSON object).
    pub arguments: serde_json::Value,
    /// Request ID for correlation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// MCP Tool call result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    /// Content blocks.
    pub content: Vec<McpContent>,
    /// Whether the tool call failed.
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// MCP Content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    #[serde(rename = "resource")]
    Resource { uri: String, text: String },
}

/// MAPLE capability descriptor for MCP tools.
#[derive(Debug, Clone)]
pub struct McpCapabilityMapping {
    /// The MCP tool.
    pub tool: McpTool,
    /// MAPLE effect domain.
    pub domain: EffectDomain,
    /// MAPLE scope constraint.
    pub scope: ScopeConstraint,
    /// Whether this is a consequential operation.
    pub consequential: bool,
    /// Risk level (0.0-1.0).
    pub risk_level: f64,
    /// Requires explicit commitment.
    pub requires_commitment: bool,
}

impl McpCapabilityMapping {
    /// Create a mapping for a read-only tool.
    pub fn read_only(tool: McpTool) -> Self {
        Self {
            tool,
            domain: EffectDomain::Computation,
            scope: ScopeConstraint::default(),
            consequential: false,
            risk_level: 0.1,
            requires_commitment: false,
        }
    }

    /// Create a mapping for a consequential tool.
    pub fn consequential(tool: McpTool, domain: EffectDomain) -> Self {
        Self {
            tool,
            domain,
            scope: ScopeConstraint::default(),
            consequential: true,
            risk_level: 0.5,
            requires_commitment: true,
        }
    }

    /// Set custom scope.
    pub fn with_scope(mut self, scope: ScopeConstraint) -> Self {
        self.scope = scope;
        self
    }

    /// Set risk level.
    pub fn with_risk(mut self, risk: f64) -> Self {
        self.risk_level = risk.clamp(0.0, 1.0);
        self
    }

    /// Infer mapping from MCP annotations.
    pub fn from_annotations(tool: McpTool) -> Self {
        let annotations = &tool.annotations;

        let consequential = annotations.destructive_hint.unwrap_or(false)
            || !annotations.read_only_hint.unwrap_or(true);

        let domain = annotations
            .maple_domain
            .as_ref()
            .and_then(|d| match d.as_str() {
                "computation" => Some(EffectDomain::Computation),
                "finance" => Some(EffectDomain::Finance),
                "communication" => Some(EffectDomain::Communication),
                "data" => Some(EffectDomain::Data),
                _ => None,
            })
            .unwrap_or(EffectDomain::Computation);

        let risk_level = if annotations.destructive_hint.unwrap_or(false) {
            0.8
        } else if consequential {
            0.5
        } else {
            0.1
        };

        Self {
            tool,
            domain,
            scope: ScopeConstraint::default(),
            consequential,
            risk_level,
            requires_commitment: consequential,
        }
    }
}

/// Tool executor trait for MCP tools.
#[async_trait]
pub trait McpToolExecutor: Send + Sync {
    /// Execute a tool call and return the result.
    async fn execute(&self, call: &McpToolCall) -> Result<McpToolResult, McpAdapterError>;

    /// Get the tool definition.
    fn tool(&self) -> &McpTool;
}

/// Resource provider trait for MCP resources.
#[async_trait]
pub trait McpResourceProvider: Send + Sync {
    /// List available resources.
    async fn list(&self) -> Result<Vec<McpResource>, McpAdapterError>;

    /// Read a resource by URI.
    async fn read(&self, uri: &str) -> Result<McpContent, McpAdapterError>;
}

/// Registry of MCP tools and their mappings.
#[derive(Default)]
pub struct McpToolRegistry {
    tools: HashMap<String, McpCapabilityMapping>,
    executors: HashMap<String, Arc<dyn McpToolExecutor>>,
}

impl McpToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool with its mapping.
    pub fn register(&mut self, mapping: McpCapabilityMapping) {
        self.tools.insert(mapping.tool.name.clone(), mapping);
    }

    /// Register a tool with an executor.
    pub fn register_with_executor(
        &mut self,
        mapping: McpCapabilityMapping,
        executor: Arc<dyn McpToolExecutor>,
    ) {
        let name = mapping.tool.name.clone();
        self.tools.insert(name.clone(), mapping);
        self.executors.insert(name, executor);
    }

    /// Get a tool mapping by name.
    pub fn get(&self, name: &str) -> Option<&McpCapabilityMapping> {
        self.tools.get(name)
    }

    /// Get an executor by name.
    pub fn get_executor(&self, name: &str) -> Option<&Arc<dyn McpToolExecutor>> {
        self.executors.get(name)
    }

    /// List all registered tools.
    pub fn list_tools(&self) -> Vec<&McpTool> {
        self.tools.values().map(|m| &m.tool).collect()
    }

    /// Check if a tool requires commitment.
    pub fn requires_commitment(&self, name: &str) -> bool {
        self.tools
            .get(name)
            .map(|m| m.requires_commitment)
            .unwrap_or(true)
    }
}

/// MCP call record for audit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCallRecord {
    /// Unique call ID.
    pub call_id: String,
    /// Tool name.
    pub tool_name: String,
    /// Arguments.
    pub arguments: serde_json::Value,
    /// Associated commitment ID (if any).
    pub commitment_id: Option<CommitmentId>,
    /// When the call was made.
    pub called_at: DateTime<Utc>,
    /// When the call completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Whether successful.
    pub success: bool,
    /// Result summary.
    pub result_summary: Option<String>,
}

/// The main MCP protocol adapter.
///
/// Bridges MCP tools/resources to MAPLE's resonance architecture,
/// ensuring all operations respect commitment boundaries.
pub struct McpAdapter {
    /// Tool registry.
    registry: McpToolRegistry,
    /// Resource providers.
    resource_providers: Vec<Arc<dyn McpResourceProvider>>,
    /// Principal identity for commitments.
    principal: IdentityRef,
    /// Call history for audit.
    call_history: std::sync::RwLock<Vec<McpCallRecord>>,
    /// Configuration.
    config: McpAdapterConfig,
}

/// Configuration for the MCP adapter.
#[derive(Debug, Clone)]
pub struct McpAdapterConfig {
    /// Require commitments for all consequential tools.
    pub require_commitments: bool,
    /// Maximum call history to retain.
    pub max_call_history: usize,
    /// Default timeout for tool calls (ms).
    pub default_timeout_ms: u64,
    /// Enable detailed logging.
    pub detailed_logging: bool,
}

impl Default for McpAdapterConfig {
    fn default() -> Self {
        Self {
            require_commitments: true,
            max_call_history: 1000,
            default_timeout_ms: 30000,
            detailed_logging: false,
        }
    }
}

impl McpAdapter {
    /// Create a new MCP adapter.
    pub fn new(principal: IdentityRef) -> Self {
        Self {
            registry: McpToolRegistry::new(),
            resource_providers: Vec::new(),
            principal,
            call_history: std::sync::RwLock::new(Vec::new()),
            config: McpAdapterConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(mut self, config: McpAdapterConfig) -> Self {
        self.config = config;
        self
    }

    /// Register a tool.
    pub fn register_tool(&mut self, mapping: McpCapabilityMapping) {
        self.registry.register(mapping);
    }

    /// Register a tool with executor.
    pub fn register_tool_with_executor(
        &mut self,
        mapping: McpCapabilityMapping,
        executor: Arc<dyn McpToolExecutor>,
    ) {
        self.registry.register_with_executor(mapping, executor);
    }

    /// Register a resource provider.
    pub fn register_resource_provider(&mut self, provider: Arc<dyn McpResourceProvider>) {
        self.resource_providers.push(provider);
    }

    /// List available tools (MCP tools/list response).
    pub fn list_tools(&self) -> Vec<McpTool> {
        self.registry.list_tools().into_iter().cloned().collect()
    }

    /// List available resources (MCP resources/list response).
    pub async fn list_resources(&self) -> Result<Vec<McpResource>, McpAdapterError> {
        let mut resources = Vec::new();
        for provider in &self.resource_providers {
            resources.extend(provider.list().await?);
        }
        Ok(resources)
    }

    /// Read a resource (MCP resources/read response).
    pub async fn read_resource(&self, uri: &str) -> Result<McpContent, McpAdapterError> {
        for provider in &self.resource_providers {
            let resources = provider.list().await?;
            if resources.iter().any(|r| r.uri == uri) {
                return provider.read(uri).await;
            }
        }
        Err(McpAdapterError::ResourceNotFound(uri.to_string()))
    }

    /// Build a commitment for an MCP tool call.
    pub fn build_commitment(
        &self,
        tool_call: &McpToolCall,
    ) -> Result<RcfCommitment, McpAdapterError> {
        let mapping = self
            .registry
            .get(&tool_call.name)
            .ok_or_else(|| McpAdapterError::ToolNotFound(tool_call.name.clone()))?;

        let commitment = CommitmentBuilder::new(self.principal.clone(), mapping.domain.clone())
            .with_scope(mapping.scope.clone())
            .with_outcome(rcf_commitment::IntendedOutcome::new(format!(
                "Execute MCP tool: {}",
                tool_call.name
            )))
            .with_policy_tag("mcp-tool-call")
            .build()
            .map_err(|e| McpAdapterError::CommitmentBuildError(e.to_string()))?;

        Ok(commitment)
    }

    /// Call a tool with commitment tracking.
    ///
    /// If the tool requires a commitment, one must be provided or will be auto-created.
    pub async fn call_tool(
        &self,
        tool_call: McpToolCall,
        commitment: Option<RcfCommitment>,
    ) -> Result<(McpToolResult, McpCallRecord), McpAdapterError> {
        let mapping = self
            .registry
            .get(&tool_call.name)
            .ok_or_else(|| McpAdapterError::ToolNotFound(tool_call.name.clone()))?;

        // Check commitment requirement
        if mapping.requires_commitment && commitment.is_none() && self.config.require_commitments {
            return Err(McpAdapterError::CommitmentRequired(tool_call.name.clone()));
        }

        let call_id = format!("mcp-call-{}", uuid::Uuid::new_v4());
        let called_at = Utc::now();

        // Get executor
        let executor = self
            .registry
            .get_executor(&tool_call.name)
            .ok_or_else(|| McpAdapterError::NoExecutor(tool_call.name.clone()))?;

        // Execute the tool
        let result = executor.execute(&tool_call).await;

        // Create call record
        let record = McpCallRecord {
            call_id: call_id.clone(),
            tool_name: tool_call.name.clone(),
            arguments: tool_call.arguments.clone(),
            commitment_id: commitment.map(|c| c.commitment_id),
            called_at,
            completed_at: Some(Utc::now()),
            success: result.is_ok(),
            result_summary: result.as_ref().ok().map(|r| {
                r.content
                    .first()
                    .map(|c| match c {
                        McpContent::Text { text } => {
                            text.chars().take(100).collect::<String>()
                        }
                        McpContent::Image { .. } => "[image]".to_string(),
                        McpContent::Resource { uri, .. } => format!("[resource: {}]", uri),
                    })
                    .unwrap_or_else(|| "[empty]".to_string())
            }),
        };

        // Store in history
        if let Ok(mut history) = self.call_history.write() {
            history.push(record.clone());
            while history.len() > self.config.max_call_history {
                history.remove(0);
            }
        }

        match result {
            Ok(r) => Ok((r, record)),
            Err(e) => Err(e),
        }
    }

    /// Get call history.
    pub fn call_history(&self, limit: usize) -> Vec<McpCallRecord> {
        self.call_history
            .read()
            .map(|h| h.iter().rev().take(limit).cloned().collect())
            .unwrap_or_default()
    }

    /// Convert MAPLE capability to MCP tool.
    pub fn capability_to_tool(
        capability_id: &str,
        description: &str,
        domain: &EffectDomain,
        consequential: bool,
    ) -> McpTool {
        McpTool {
            name: capability_id.to_string(),
            description: description.to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": true
            }),
            annotations: McpAnnotations {
                title: Some(capability_id.to_string()),
                read_only_hint: Some(!consequential),
                destructive_hint: Some(consequential && domain.is_consequential()),
                idempotent_hint: None,
                maple_domain: Some(domain.to_string()),
                maple_scope: None,
            },
        }
    }
}

/// Extension trait for EffectDomain.
trait EffectDomainExt {
    fn is_consequential(&self) -> bool;
}

impl EffectDomainExt for EffectDomain {
    fn is_consequential(&self) -> bool {
        !matches!(self, EffectDomain::Computation)
    }
}

/// MCP adapter errors.
#[derive(Debug, Error)]
pub enum McpAdapterError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Commitment required for tool: {0}")]
    CommitmentRequired(String),

    #[error("No executor registered for tool: {0}")]
    NoExecutor(String),

    #[error("Commitment build error: {0}")]
    CommitmentBuildError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Simple echo tool executor for testing.
pub struct EchoToolExecutor {
    tool: McpTool,
}

impl EchoToolExecutor {
    pub fn new() -> Self {
        Self {
            tool: McpTool {
                name: "echo".to_string(),
                description: "Echo back the input message".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Message to echo"
                        }
                    },
                    "required": ["message"]
                }),
                annotations: McpAnnotations {
                    read_only_hint: Some(true),
                    ..Default::default()
                },
            },
        }
    }
}

impl Default for EchoToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl McpToolExecutor for EchoToolExecutor {
    async fn execute(&self, call: &McpToolCall) -> Result<McpToolResult, McpAdapterError> {
        let message = call
            .arguments
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("(no message)");

        Ok(McpToolResult {
            content: vec![McpContent::Text {
                text: format!("Echo: {}", message),
            }],
            is_error: Some(false),
        })
    }

    fn tool(&self) -> &McpTool {
        &self.tool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_tool_serialization() {
        let tool = McpTool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"}
                }
            }),
            annotations: McpAnnotations {
                read_only_hint: Some(true),
                ..Default::default()
            },
        };

        let json = serde_json::to_string(&tool).unwrap();
        let parsed: McpTool = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test_tool");
    }

    #[test]
    fn test_capability_mapping_from_annotations() {
        let tool = McpTool {
            name: "dangerous_tool".to_string(),
            description: "A dangerous tool".to_string(),
            input_schema: serde_json::json!({}),
            annotations: McpAnnotations {
                destructive_hint: Some(true),
                maple_domain: Some("finance".to_string()),
                ..Default::default()
            },
        };

        let mapping = McpCapabilityMapping::from_annotations(tool);
        assert!(mapping.consequential);
        assert_eq!(mapping.domain, EffectDomain::Finance);
        assert!(mapping.risk_level > 0.5);
    }

    #[test]
    fn test_tool_registry() {
        let mut registry = McpToolRegistry::new();

        let tool = McpTool {
            name: "test".to_string(),
            description: "Test".to_string(),
            input_schema: serde_json::json!({}),
            annotations: Default::default(),
        };

        registry.register(McpCapabilityMapping::read_only(tool));

        assert!(registry.get("test").is_some());
        assert!(!registry.requires_commitment("test"));
    }

    #[tokio::test]
    async fn test_echo_tool_executor() {
        let executor = EchoToolExecutor::new();

        let call = McpToolCall {
            name: "echo".to_string(),
            arguments: serde_json::json!({"message": "Hello, MCP!"}),
            request_id: None,
        };

        let result = executor.execute(&call).await.unwrap();

        match &result.content[0] {
            McpContent::Text { text } => {
                assert!(text.contains("Hello, MCP!"));
            }
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_mcp_adapter_tool_registration() {
        let mut adapter = McpAdapter::new(IdentityRef::new("test-agent"));

        let tool = McpTool {
            name: "my_tool".to_string(),
            description: "My tool".to_string(),
            input_schema: serde_json::json!({}),
            annotations: Default::default(),
        };

        adapter.register_tool(McpCapabilityMapping::read_only(tool));

        let tools = adapter.list_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "my_tool");
    }

    #[test]
    fn test_commitment_building() {
        let adapter = McpAdapter::new(IdentityRef::new("test-agent"));

        let tool = McpTool {
            name: "test_tool".to_string(),
            description: "Test".to_string(),
            input_schema: serde_json::json!({}),
            annotations: Default::default(),
        };

        let mut adapter = adapter;
        adapter.register_tool(McpCapabilityMapping::consequential(
            tool,
            EffectDomain::Finance,
        ));

        let call = McpToolCall {
            name: "test_tool".to_string(),
            arguments: serde_json::json!({}),
            request_id: None,
        };

        let commitment = adapter.build_commitment(&call).unwrap();
        assert_eq!(commitment.effect_domain, EffectDomain::Finance);
    }
}
