//! Cognition adapters for MAPLE agents.
//!
//! Adapters are cognition-only and can propose intent/tool suggestions, but they
//! can never execute side effects directly. All consequential execution remains
//! gated by AgentKernel commitment/AAS boundaries.

use async_trait::async_trait;
use rcf_commitment::{CommitmentBuilder, IntendedOutcome, RcfCommitment};
use rcf_types::{CapabilityRef, EffectDomain, IdentityRef, ScopeConstraint, TemporalValidity};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub mod anthropic;
pub mod gemini;
pub mod grok;
pub mod llama;
pub mod openai;

pub use anthropic::AnthropicAdapter;
pub use gemini::GeminiAdapter;
pub use grok::GrokAdapter;
pub use llama::LlamaAdapter;
pub use openai::OpenAiAdapter;

/// Provider-agnostic state context supplied to cognition proposals.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CognitionState {
    pub resonator_id: Option<String>,
    pub profile_name: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

/// Input payload for meaning proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeaningInput {
    pub utterance: String,
    #[serde(default)]
    pub metadata: Value,
}

/// Proposed meaning draft.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeaningDraft {
    pub summary: String,
    #[serde(default)]
    pub ambiguity_notes: Vec<String>,
    pub confidence: f64,
}

/// Proposed intent draft.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentDraft {
    pub objective: String,
    #[serde(default)]
    pub steps: Vec<String>,
    pub confidence: f64,
    pub blocking_ambiguity: bool,
}

/// Contract draft shape that remains compatible with RCF commitments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractDraft {
    pub effect_domain: EffectDomain,
    pub scope: ScopeConstraint,
    pub temporal_validity: TemporalValidity,
    pub intended_outcome: String,
    #[serde(default)]
    pub required_capability_ids: Vec<String>,
    pub confidence_context: f64,
    #[serde(default)]
    pub platform_data: Value,
}

impl ContractDraft {
    /// Convert this draft into a concrete RCF commitment.
    pub fn to_rcf_commitment(
        &self,
        principal: IdentityRef,
    ) -> Result<RcfCommitment, rcf_commitment::CommitmentBuildError> {
        let mut builder = CommitmentBuilder::new(principal.clone(), self.effect_domain.clone())
            .with_scope(self.scope.clone())
            .with_validity(self.temporal_validity.clone())
            .with_outcome(IntendedOutcome::new(self.intended_outcome.clone()));

        for capability_id in &self.required_capability_ids {
            let capability = CapabilityRef::new(
                capability_id.clone(),
                self.effect_domain.clone(),
                self.scope.clone(),
                self.temporal_validity.clone(),
                principal.clone(),
            );
            builder = builder.with_capability(capability);
        }

        builder.build()
    }
}

/// Normalized capability call candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityCallCandidate {
    pub capability_id: String,
    pub params_json: Value,
    pub risk_score: f64,
    pub rationale: String,
    pub required_contract_fields: Vec<String>,
}

/// Journal slice item passed to summarization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalSliceItem {
    pub stage: String,
    pub message: String,
    #[serde(default)]
    pub payload: Value,
}

/// Episodic summary produced from journal slices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicSummary {
    pub summary: String,
    #[serde(default)]
    pub key_points: Vec<String>,
    #[serde(default)]
    pub open_questions: Vec<String>,
}

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

/// Normalized provider configuration used by all backend adapters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProviderConfig {
    pub backend: ModelBackend,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    pub timeout_ms: u64,
}

impl ModelProviderConfig {
    pub fn llama(model: impl Into<String>) -> Self {
        Self {
            backend: ModelBackend::LocalLlama,
            model: model.into(),
            endpoint: Some("http://127.0.0.1:11434".to_string()),
            api_key_env: None,
            timeout_ms: 15_000,
        }
    }

    pub fn open_ai(model: impl Into<String>) -> Self {
        Self {
            backend: ModelBackend::OpenAi,
            model: model.into(),
            endpoint: Some("https://api.openai.com/v1".to_string()),
            api_key_env: Some("OPENAI_API_KEY".to_string()),
            timeout_ms: 20_000,
        }
    }

    pub fn anthropic(model: impl Into<String>) -> Self {
        Self {
            backend: ModelBackend::Anthropic,
            model: model.into(),
            endpoint: Some("https://api.anthropic.com".to_string()),
            api_key_env: Some("ANTHROPIC_API_KEY".to_string()),
            timeout_ms: 20_000,
        }
    }

    pub fn gemini(model: impl Into<String>) -> Self {
        Self {
            backend: ModelBackend::Gemini,
            model: model.into(),
            endpoint: Some("https://generativelanguage.googleapis.com".to_string()),
            api_key_env: Some("GEMINI_API_KEY".to_string()),
            timeout_ms: 20_000,
        }
    }

    pub fn grok(model: impl Into<String>) -> Self {
        Self {
            backend: ModelBackend::Grok,
            model: model.into(),
            endpoint: Some("https://api.x.ai/v1".to_string()),
            api_key_env: Some("XAI_API_KEY".to_string()),
            timeout_ms: 20_000,
        }
    }
}

/// Normalized usage envelope for provider accounting/observability.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Normalized provider error kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelErrorKind {
    InvalidConfig,
    Transport,
    Parse,
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

/// Task kind requested from transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTask {
    ProposeMeaning,
    ProposeIntent,
    DraftContract,
    SuggestCapabilityCalls,
    Summarize,
    RepairJson,
}

/// Prompt bundle sent through pluggable transport.
#[derive(Debug, Clone)]
pub struct TransportRequest {
    pub task: ModelTask,
    pub system_prompt: String,
    pub user_prompt: String,
}

/// Pluggable text transport for cognition backends.
#[async_trait]
pub trait ModelTransport: Send + Sync {
    async fn generate(
        &self,
        config: &ModelProviderConfig,
        request: &TransportRequest,
    ) -> Result<String, ModelAdapterError>;
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
    pub provider: ModelProviderConfig,
    pub usage: ModelUsage,
    pub raw_text: String,
    pub cognition: StructuredCognition,
}

/// Trait implemented by all cognition backends.
#[async_trait]
pub trait ModelAdapter: Send + Sync {
    /// Backend kind.
    fn backend(&self) -> ModelBackend;

    /// Provider configuration.
    fn config(&self) -> &ModelProviderConfig;

    /// Generate cognition output.
    async fn infer(&self, request: &ModelRequest) -> Result<ModelResponse, ModelAdapterError>;

    /// Propose a Meaning draft from user/environment input.
    async fn propose_meaning(
        &self,
        input: &MeaningInput,
        state: &CognitionState,
    ) -> Result<MeaningDraft, ModelAdapterError> {
        let request = ModelRequest {
            system_prompt: Some(
                "Return strict JSON with fields: meaning_summary, intent, confidence.".to_string(),
            ),
            user_prompt: format!(
                "state={}; utterance={}",
                serde_json::to_string(state).unwrap_or_else(|_| "{}".to_string()),
                input.utterance
            ),
            raw_response_override: None,
        };
        let response = self.infer(&request).await?;
        if response.cognition.validation == ValidationStatus::Fallback {
            return Err(ModelAdapterError::new(
                self.backend(),
                ModelErrorKind::Parse,
                "model output was not valid JSON and cannot be used for meaning proposal",
            ));
        }
        Ok(MeaningDraft {
            summary: response.cognition.meaning_summary,
            ambiguity_notes: vec![],
            confidence: response.cognition.confidence.clamp(0.0, 1.0),
        })
    }

    /// Propose an Intent draft from Meaning.
    async fn propose_intent(
        &self,
        meaning: &MeaningDraft,
        state: &CognitionState,
    ) -> Result<IntentDraft, ModelAdapterError> {
        let request = ModelRequest {
            system_prompt: Some(
                "Return strict JSON with fields: meaning_summary, intent, confidence.".to_string(),
            ),
            user_prompt: format!(
                "state={}; meaning={}",
                serde_json::to_string(state).unwrap_or_else(|_| "{}".to_string()),
                meaning.summary
            ),
            raw_response_override: None,
        };
        let response = self.infer(&request).await?;
        if response.cognition.validation == ValidationStatus::Fallback {
            return Err(ModelAdapterError::new(
                self.backend(),
                ModelErrorKind::Parse,
                "model output was not valid JSON and cannot be used for intent proposal",
            ));
        }
        Ok(IntentDraft {
            objective: response.cognition.intent,
            steps: vec![],
            confidence: response.cognition.confidence.clamp(0.0, 1.0),
            blocking_ambiguity: response.cognition.confidence < 0.5,
        })
    }

    /// Draft an RCF-compatible contract shape from Intent.
    async fn draft_contract(
        &self,
        intent: &IntentDraft,
        _state: &CognitionState,
    ) -> Result<ContractDraft, ModelAdapterError> {
        let domain = if intent.objective.to_ascii_lowercase().contains("transfer") {
            EffectDomain::Finance
        } else {
            EffectDomain::Computation
        };
        Ok(ContractDraft {
            effect_domain: domain,
            scope: ScopeConstraint::global(),
            temporal_validity: TemporalValidity::unbounded(),
            intended_outcome: intent.objective.clone(),
            required_capability_ids: vec![],
            confidence_context: intent.confidence.clamp(0.0, 1.0),
            platform_data: serde_json::json!({
                "source": "default-model-adapter",
                "blocking_ambiguity": intent.blocking_ambiguity,
            }),
        })
    }

    /// Suggest normalized capability calls for a contract.
    async fn suggest_capability_calls(
        &self,
        contract: &ContractDraft,
        _state: &CognitionState,
    ) -> Result<Vec<CapabilityCallCandidate>, ModelAdapterError> {
        if contract.required_capability_ids.is_empty() {
            return Ok(Vec::new());
        }

        let candidates = contract
            .required_capability_ids
            .iter()
            .map(|capability_id| CapabilityCallCandidate {
                capability_id: capability_id.clone(),
                params_json: serde_json::json!({}),
                risk_score: 0.0,
                rationale: "Default adapter cannot infer params; human or policy layer must fill."
                    .to_string(),
                required_contract_fields: vec![
                    "effect_domain".to_string(),
                    "scope".to_string(),
                    "temporal_validity".to_string(),
                ],
            })
            .collect();
        Ok(candidates)
    }

    /// Produce an episodic summary from a journal slice.
    async fn summarize(
        &self,
        journal_slice: &[JournalSliceItem],
    ) -> Result<EpisodicSummary, ModelAdapterError> {
        let text = journal_slice
            .iter()
            .map(|item| format!("[{}] {}", item.stage, item.message))
            .collect::<Vec<_>>()
            .join(" | ");
        Ok(EpisodicSummary {
            summary: trim_for_summary(&text),
            key_points: journal_slice
                .iter()
                .take(3)
                .map(|item| item.message.clone())
                .collect(),
            open_questions: Vec::new(),
        })
    }
}

/// Compatibility adapter for vendor endpoints while provider-specific adapters are available.
#[derive(Debug, Clone)]
pub struct VendorAdapter {
    config: ModelProviderConfig,
}

impl VendorAdapter {
    pub fn open_ai(model: impl Into<String>) -> Self {
        Self {
            config: ModelProviderConfig::open_ai(model),
        }
    }

    pub fn anthropic(model: impl Into<String>) -> Self {
        Self {
            config: ModelProviderConfig::anthropic(model),
        }
    }

    pub fn gemini(model: impl Into<String>) -> Self {
        Self {
            config: ModelProviderConfig::gemini(model),
        }
    }

    pub fn grok(model: impl Into<String>) -> Self {
        Self {
            config: ModelProviderConfig::grok(model),
        }
    }
}

#[async_trait]
impl ModelAdapter for VendorAdapter {
    fn backend(&self) -> ModelBackend {
        self.config.backend
    }

    fn config(&self) -> &ModelProviderConfig {
        &self.config
    }

    async fn infer(&self, request: &ModelRequest) -> Result<ModelResponse, ModelAdapterError> {
        infer_with_parser(
            self.backend(),
            self.config(),
            request,
            synthesize_raw_response(request, &self.config.model),
        )
    }
}

pub(crate) fn infer_with_parser(
    backend: ModelBackend,
    config: &ModelProviderConfig,
    request: &ModelRequest,
    default_raw: String,
) -> Result<ModelResponse, ModelAdapterError> {
    if config.model.trim().is_empty() {
        return Err(ModelAdapterError::new(
            backend,
            ModelErrorKind::InvalidConfig,
            "model name must not be empty",
        ));
    }

    let raw = request.raw_response_override.clone().unwrap_or(default_raw);

    let cognition = parse_cognition(&raw)
        .or_else(|| repair_and_parse(&raw))
        .unwrap_or_else(|| deterministic_fallback(&request.user_prompt));

    Ok(ModelResponse {
        backend,
        provider: config.clone(),
        usage: estimate_usage(request, &raw),
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
    let parsed = parse_json_with_normalization::<SchemaEnvelope>(raw)?;
    Some(StructuredCognition {
        meaning_summary: parsed.meaning_summary,
        intent: parsed.intent,
        confidence: parsed.confidence.clamp(0.0, 1.0),
        suggested_tool: parsed.suggested_tool,
        validation: ValidationStatus::Repaired,
    })
}

pub(crate) fn parse_json_with_normalization<T: DeserializeOwned>(raw: &str) -> Option<T> {
    for candidate in json_candidates(raw) {
        if let Ok(parsed) = serde_json::from_str::<T>(&candidate) {
            return Some(parsed);
        }
    }
    None
}

fn json_candidates(raw: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    candidates.push(raw.trim().to_string());

    if let Some(fenced) = extract_json_code_fence(raw) {
        candidates.push(fenced);
    }

    if let Some(extracted) = extract_first_json_object(raw) {
        candidates.push(extracted.clone());
        candidates.push(extracted.replace('\'', "\""));
        candidates.push(strip_trailing_commas(&extracted));
    }

    if let Some(extracted_array) = extract_first_json_array(raw) {
        candidates.push(extracted_array.clone());
        candidates.push(strip_trailing_commas(&extracted_array));
    }

    candidates.push(raw.replace('\'', "\""));
    candidates.push(strip_trailing_commas(raw));
    dedupe_candidates(candidates)
}

fn dedupe_candidates(candidates: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for candidate in candidates {
        let trimmed = candidate.trim();
        if trimmed.is_empty() {
            continue;
        }
        if out.iter().all(|known: &String| known != trimmed) {
            out.push(trimmed.to_string());
        }
    }
    out
}

fn extract_json_code_fence(raw: &str) -> Option<String> {
    let mut sections = raw.split("```");
    let _prefix = sections.next()?;
    let body = sections.next()?.trim();
    let body = body
        .strip_prefix("json")
        .or_else(|| body.strip_prefix("JSON"))
        .unwrap_or(body)
        .trim();
    if body.starts_with('{') || body.starts_with('[') {
        return Some(body.to_string());
    }
    None
}

fn extract_first_json_object(raw: &str) -> Option<String> {
    let start = raw.find('{')?;
    extract_balanced(raw, start, '{', '}')
}

fn extract_first_json_array(raw: &str) -> Option<String> {
    let start = raw.find('[')?;
    extract_balanced(raw, start, '[', ']')
}

fn extract_balanced(raw: &str, start: usize, open: char, close: char) -> Option<String> {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in raw[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            c if c == open => depth += 1,
            c if c == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(raw[start..start + idx + 1].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn strip_trailing_commas(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    while i < chars.len() {
        let ch = chars[i];
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            i += 1;
            continue;
        }

        if ch == ',' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                i += 1;
                continue;
            }
        }

        out.push(ch);
        i += 1;
    }

    out
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

pub(crate) fn synthesize_raw_response(request: &ModelRequest, model: &str) -> String {
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

/// Deterministic transport used by default when no live backend is configured.
#[derive(Debug, Default, Clone)]
pub struct SyntheticTransport;

#[async_trait]
impl ModelTransport for SyntheticTransport {
    async fn generate(
        &self,
        config: &ModelProviderConfig,
        request: &TransportRequest,
    ) -> Result<String, ModelAdapterError> {
        Ok(synthesize_transport_response(request, &config.model))
    }
}

fn synthesize_transport_response(request: &TransportRequest, model: &str) -> String {
    match request.task {
        ModelTask::ProposeMeaning => serde_json::json!({
            "summary": trim_for_summary(&request.user_prompt),
            "ambiguity_notes": [],
            "confidence": 0.86
        })
        .to_string(),
        ModelTask::ProposeIntent => serde_json::json!({
            "objective": format!("stabilize_intent: {}", trim_for_summary(&request.user_prompt)),
            "steps": ["validate context", "prepare commitment boundary"],
            "confidence": 0.82,
            "blocking_ambiguity": false
        })
        .to_string(),
        ModelTask::DraftContract => serde_json::json!({
            "effect_domain": "computation",
            "scope": {
                "targets": ["*"],
                "operations": ["*"]
            },
            "temporal_validity": {
                "valid_from": chrono::Utc::now(),
                "valid_until": chrono::Utc::now() + chrono::Duration::minutes(30)
            },
            "intended_outcome": trim_for_summary(&request.user_prompt),
            "required_capability_ids": ["echo_log"],
            "confidence_context": 0.8,
            "platform_data": {
                "source_model": model
            }
        })
        .to_string(),
        ModelTask::SuggestCapabilityCalls => serde_json::json!([{
            "capability_id": "echo_log",
            "params_json": {
                "message": trim_for_summary(&request.user_prompt)
            },
            "risk_score": 0.1,
            "rationale": "Safe default logging capability",
            "required_contract_fields": [
                "effect_domain",
                "scope",
                "temporal_validity"
            ]
        }])
        .to_string(),
        ModelTask::Summarize => serde_json::json!({
            "summary": trim_for_summary(&request.user_prompt),
            "key_points": ["journal summarized"],
            "open_questions": []
        })
        .to_string(),
        ModelTask::RepairJson => {
            // Deterministic no-op repair baseline.
            request.user_prompt.clone()
        }
    }
}

fn estimate_usage(request: &ModelRequest, raw: &str) -> ModelUsage {
    // Deterministic approximation used in tests and offline flows.
    let prompt_tokens = request.user_prompt.split_whitespace().count() as u32;
    let completion_tokens = raw.split_whitespace().count() as u32;
    ModelUsage {
        prompt_tokens,
        completion_tokens,
        total_tokens: prompt_tokens.saturating_add(completion_tokens),
    }
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
#[error("{kind:?} error for backend {backend}: {message}")]
pub struct ModelAdapterError {
    pub backend: ModelBackend,
    pub kind: ModelErrorKind,
    pub message: String,
}

impl ModelAdapterError {
    pub fn new(backend: ModelBackend, kind: ModelErrorKind, message: impl Into<String>) -> Self {
        Self {
            backend,
            kind,
            message: message.into(),
        }
    }
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

    #[tokio::test]
    async fn malformed_output_falls_back_for_all_backends() {
        let adapters: Vec<Box<dyn ModelAdapter>> = vec![
            Box::new(LlamaAdapter::new("llama3.2")),
            Box::new(OpenAiAdapter::new("gpt-4o-mini")),
            Box::new(AnthropicAdapter::new("claude-3-5-sonnet")),
            Box::new(GeminiAdapter::new("gemini-2.0-flash")),
            Box::new(GrokAdapter::new("grok-2")),
        ];

        for adapter in adapters {
            let mut req = ModelRequest::new("transfer 500 usd");
            req.raw_response_override = Some("not-json-at-all".to_string());
            let out = adapter.infer(&req).await.unwrap();
            assert_eq!(out.cognition.validation, ValidationStatus::Fallback);
            assert!(
                out.cognition.suggested_tool.is_none(),
                "backend {} should never suggest tool on fallback",
                out.backend
            );
        }
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct ParserFixture {
        capability_id: String,
        params_json: Value,
    }

    #[test]
    fn parser_accepts_valid_json() {
        let raw = r#"{"capability_id":"echo_log","params_json":{"message":"ok"}}"#;
        let parsed: ParserFixture = parse_json_with_normalization(raw).expect("valid json");
        assert_eq!(parsed.capability_id, "echo_log");
    }

    #[test]
    fn parser_repairs_trailing_commas() {
        let raw = r#"{"capability_id":"echo_log","params_json":{"message":"ok",},}"#;
        let parsed: ParserFixture =
            parse_json_with_normalization(raw).expect("trailing commas should be repaired");
        assert_eq!(parsed.capability_id, "echo_log");
    }

    #[test]
    fn parser_extracts_json_from_prose() {
        let raw = r#"llama says: {"capability_id":"echo_log","params_json":{"message":"ok"}}"#;
        let parsed: ParserFixture =
            parse_json_with_normalization(raw).expect("json object should be extracted");
        assert_eq!(parsed.capability_id, "echo_log");
    }

    #[test]
    fn parser_extracts_json_from_code_fence() {
        let raw =
            "```json\n{\"capability_id\":\"echo_log\",\"params_json\":{\"message\":\"ok\"}}\n```";
        let parsed: ParserFixture =
            parse_json_with_normalization(raw).expect("json code fence should parse");
        assert_eq!(parsed.capability_id, "echo_log");
    }
}
