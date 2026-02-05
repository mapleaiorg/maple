//! Cognition adapters for MAPLE agents.
//!
//! These adapters are deliberately constrained to proposal generation.
//! They cannot execute effects directly; execution is always gated by
//! MAPLE commitment and AAS checks in `agent_kernel`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Supported cognition backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelBackend {
    /// Local Ollama/Llama-compatible backend.
    LocalLlama,
    /// OpenAI-style backend.
    OpenAi,
    /// Anthropic backend.
    Anthropic,
    /// Google Gemini backend.
    Gemini,
    /// xAI Grok backend.
    Grok,
}

impl std::fmt::Display for ModelBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ModelBackend::LocalLlama => "local_llama",
            ModelBackend::OpenAi => "open_ai",
            ModelBackend::Anthropic => "anthropic",
            ModelBackend::Gemini => "gemini",
            ModelBackend::Grok => "grok",
        };
        write!(f, "{}", name)
    }
}

/// Request passed to a cognition backend.
#[derive(Debug, Clone)]
pub struct ModelRequest {
    /// Optional system instructions.
    pub system_prompt: Option<String>,
    /// User or environment prompt.
    pub user_prompt: String,
    /// Optional deterministic raw model text used for tests.
    pub raw_response_override: Option<String>,
}

impl ModelRequest {
    pub fn new(user_prompt: impl Into<String>) -> Self {
        Self {
            system_prompt: None,
            user_prompt: user_prompt.into(),
            raw_response_override: None,
        }
    }
}

/// Validated cognition envelope consumed by the runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredCognition {
    /// Meaning summary generated from input.
    pub meaning_summary: String,
    /// Stabilized intent statement.
    pub intent: String,
    /// Confidence in [0,1].
    pub confidence: f64,
    /// Optional tool suggestion.
    pub suggested_tool: Option<SuggestedTool>,
    /// Validation quality status.
    pub validation: ValidationStatus,
}

/// Suggested tool call from cognition output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedTool {
    /// Registered capability/tool name.
    pub name: String,
    /// JSON arguments for tool execution.
    pub args: Value,
    /// Whether the tool is consequential.
    pub consequential: bool,
}

/// Indicates whether model output reached executable quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationStatus {
    /// Strict schema parse succeeded.
    Validated,
    /// Parse succeeded after deterministic repair.
    Repaired,
    /// Could not parse. Runtime-safe fallback was synthesized.
    Fallback,
}

impl ValidationStatus {
    /// Only strict/repaired payloads may drive tool execution.
    pub fn allows_tool_execution(self) -> bool {
        !matches!(self, ValidationStatus::Fallback)
    }
}

/// Backend response including raw text and normalized cognition.
#[derive(Debug, Clone)]
pub struct ModelResponse {
    pub backend: ModelBackend,
    pub raw_text: String,
    pub cognition: StructuredCognition,
}

/// Trait implemented by all cognition backends.
#[async_trait]
pub trait ModelAdapter: Send + Sync {
    /// Backend kind.
    fn backend(&self) -> ModelBackend;

    /// Generate cognition output.
    async fn infer(&self, request: &ModelRequest) -> Result<ModelResponse, ModelAdapterError>;
}

/// Llama-first adapter with schema repair and deterministic fallback.
#[derive(Debug, Clone)]
pub struct LlamaAdapter {
    pub model: String,
}

impl LlamaAdapter {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
        }
    }
}

/// Vendor adapter (OpenAI/Anthropic/Gemini/Grok) sharing the same parser and guard semantics.
#[derive(Debug, Clone)]
pub struct VendorAdapter {
    backend: ModelBackend,
    pub model: String,
}

impl VendorAdapter {
    pub fn open_ai(model: impl Into<String>) -> Self {
        Self {
            backend: ModelBackend::OpenAi,
            model: model.into(),
        }
    }

    pub fn anthropic(model: impl Into<String>) -> Self {
        Self {
            backend: ModelBackend::Anthropic,
            model: model.into(),
        }
    }

    pub fn gemini(model: impl Into<String>) -> Self {
        Self {
            backend: ModelBackend::Gemini,
            model: model.into(),
        }
    }

    pub fn grok(model: impl Into<String>) -> Self {
        Self {
            backend: ModelBackend::Grok,
            model: model.into(),
        }
    }
}

#[async_trait]
impl ModelAdapter for LlamaAdapter {
    fn backend(&self) -> ModelBackend {
        ModelBackend::LocalLlama
    }

    async fn infer(&self, request: &ModelRequest) -> Result<ModelResponse, ModelAdapterError> {
        infer_with_parser(
            self.backend(),
            request,
            synthesize_raw_response(request, &self.model),
        )
    }
}

#[async_trait]
impl ModelAdapter for VendorAdapter {
    fn backend(&self) -> ModelBackend {
        self.backend
    }

    async fn infer(&self, request: &ModelRequest) -> Result<ModelResponse, ModelAdapterError> {
        infer_with_parser(
            self.backend(),
            request,
            synthesize_raw_response(request, &self.model),
        )
    }
}

fn infer_with_parser(
    backend: ModelBackend,
    request: &ModelRequest,
    default_raw: String,
) -> Result<ModelResponse, ModelAdapterError> {
    let raw = request.raw_response_override.clone().unwrap_or(default_raw);

    let cognition = parse_cognition(&raw)
        .or_else(|| repair_and_parse(&raw))
        .unwrap_or_else(|| deterministic_fallback(&request.user_prompt));

    Ok(ModelResponse {
        backend,
        raw_text: raw,
        cognition,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SchemaEnvelope {
    meaning_summary: String,
    intent: String,
    confidence: f64,
    #[serde(default)]
    suggested_tool: Option<SuggestedTool>,
}

fn parse_cognition(raw: &str) -> Option<StructuredCognition> {
    let parsed: SchemaEnvelope = serde_json::from_str(raw).ok()?;
    Some(StructuredCognition {
        meaning_summary: parsed.meaning_summary,
        intent: parsed.intent,
        confidence: parsed.confidence.clamp(0.0, 1.0),
        suggested_tool: parsed.suggested_tool,
        validation: ValidationStatus::Validated,
    })
}

fn repair_and_parse(raw: &str) -> Option<StructuredCognition> {
    let mut candidates = Vec::new();

    if let Some(extracted) = extract_first_json_object(raw) {
        candidates.push(extracted.clone());
        candidates.push(extracted.replace('\'', "\""));
    }

    // Deterministic normalization pass for common non-JSON quoting mistakes.
    candidates.push(raw.replace('\'', "\""));

    for candidate in candidates {
        if let Ok(parsed) = serde_json::from_str::<SchemaEnvelope>(&candidate) {
            return Some(StructuredCognition {
                meaning_summary: parsed.meaning_summary,
                intent: parsed.intent,
                confidence: parsed.confidence.clamp(0.0, 1.0),
                suggested_tool: parsed.suggested_tool,
                validation: ValidationStatus::Repaired,
            });
        }
    }

    None
}

fn extract_first_json_object(raw: &str) -> Option<String> {
    let start = raw.find('{')?;
    let mut depth = 0i32;
    let mut end = None;

    for (idx, ch) in raw[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(start + idx + 1);
                    break;
                }
            }
            _ => {}
        }
    }

    end.map(|end_idx| raw[start..end_idx].to_string())
}

fn deterministic_fallback(prompt: &str) -> StructuredCognition {
    StructuredCognition {
        meaning_summary: format!(
            "Need clarification before acting: {}",
            trim_for_summary(prompt)
        ),
        intent: "request_clarification".to_string(),
        confidence: 0.0,
        suggested_tool: None,
        validation: ValidationStatus::Fallback,
    }
}

fn synthesize_raw_response(request: &ModelRequest, model: &str) -> String {
    // Deterministic schema-compliant output for local/offline execution and tests.
    let prompt = request.user_prompt.to_lowercase();
    let transfer =
        prompt.contains("transfer") || prompt.contains("payment") || prompt.contains("send");

    let payload = if transfer {
        serde_json::json!({
            "meaning_summary": format!("{} suggests moving value", trim_for_summary(&request.user_prompt)),
            "intent": "execute_transfer",
            "confidence": 0.86,
            "suggested_tool": {
                "name": "simulate_transfer",
                "args": {"amount": 500, "currency": "USD", "to": "counterparty-demo", "model": model},
                "consequential": true
            }
        })
    } else {
        serde_json::json!({
            "meaning_summary": trim_for_summary(&request.user_prompt),
            "intent": "log_status",
            "confidence": 0.91,
            "suggested_tool": {
                "name": "echo_log",
                "args": {"message": request.user_prompt, "model": model},
                "consequential": false
            }
        })
    };

    payload.to_string()
}

fn trim_for_summary(input: &str) -> String {
    const MAX_LEN: usize = 120;
    let mut s = input.trim().replace('\n', " ");
    if s.len() > MAX_LEN {
        s.truncate(MAX_LEN);
    }
    s
}

#[derive(Debug, Error)]
pub enum ModelAdapterError {
    #[error("backend parsing failed")]
    ParseFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn llama_repairs_non_strict_json() {
        let adapter = LlamaAdapter::new("llama3.2");
        let mut req = ModelRequest::new("transfer 500 usd");
        req.raw_response_override = Some(
            "model output: {'meaning_summary':'move funds','intent':'execute_transfer','confidence':0.8,'suggested_tool':{'name':'simulate_transfer','args':{'amount':500},'consequential':true}}".to_string(),
        );

        let out = adapter.infer(&req).await.unwrap();
        assert_eq!(out.cognition.validation, ValidationStatus::Repaired);
        assert_eq!(
            out.cognition
                .suggested_tool
                .as_ref()
                .map(|t| t.name.as_str()),
            Some("simulate_transfer")
        );
    }

    #[tokio::test]
    async fn fallback_never_suggests_tool() {
        let adapter = LlamaAdapter::new("llama3.2");
        let mut req = ModelRequest::new("hello");
        req.raw_response_override = Some("<<<broken>>>".to_string());

        let out = adapter.infer(&req).await.unwrap();
        assert_eq!(out.cognition.validation, ValidationStatus::Fallback);
        assert!(out.cognition.suggested_tool.is_none());
    }
}
