//! LLM backend adapters for playground inference.

use chrono::Utc;
use palm_shared_state::{
    AiBackendConfig, AiBackendKind, InferenceTokenUsage, PlaygroundInferenceRequest,
    PlaygroundInferenceResponse,
};
use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

const DEFAULT_OPENAI_ENDPOINT: &str = "https://api.openai.com/v1/chat/completions";
const DEFAULT_ANTHROPIC_ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const DEFAULT_GROK_ENDPOINT: &str = "https://api.x.ai/v1/chat/completions";
const DEFAULT_GEMINI_ENDPOINT: &str = "https://generativelanguage.googleapis.com";
const DEFAULT_MAX_TOKENS: u32 = 1024;
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    total_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    content: Value,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
    prompt_eval_count: Option<u64>,
    eval_count: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    stop_reason: Option<String>,
    usage: Option<AnthropicUsage>,
}

pub async fn infer(
    backend: &AiBackendConfig,
    request: &PlaygroundInferenceRequest,
) -> Result<PlaygroundInferenceResponse, String> {
    request.validate()?;

    if !backend.is_configured() {
        return Err(format!(
            "Backend {:?} is not configured. Configure endpoint/api-key first.",
            backend.kind
        ));
    }

    let client = build_http_client()?;
    let started = Instant::now();

    let (output, finish_reason, usage) = match backend.kind {
        AiBackendKind::LocalLlama => infer_ollama(&client, backend, request).await?,
        AiBackendKind::OpenAI => {
            infer_openai_compatible(&client, backend, request, DEFAULT_OPENAI_ENDPOINT).await?
        }
        AiBackendKind::Anthropic => infer_anthropic(&client, backend, request).await?,
        AiBackendKind::Grok => {
            infer_openai_compatible(&client, backend, request, DEFAULT_GROK_ENDPOINT).await?
        }
        AiBackendKind::Gemini => infer_gemini(&client, backend, request).await?,
    };

    let latency_ms = started.elapsed().as_millis();
    let latency_ms = latency_ms.min(u64::MAX as u128) as u64;

    Ok(PlaygroundInferenceResponse {
        backend_kind: backend.kind,
        backend_model: backend.model.clone(),
        output,
        latency_ms,
        created_at: Utc::now(),
        finish_reason,
        usage,
    })
}

fn build_http_client() -> Result<Client, String> {
    let mut builder = Client::builder().timeout(Duration::from_secs(60));
    let allow_system_proxy = std::env::var("PALM_USE_SYSTEM_PROXY")
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes"))
        .unwrap_or(false);

    if !allow_system_proxy {
        builder = builder.no_proxy();
    }

    builder
        .build()
        .map_err(|e| format!("failed to build HTTP client: {}", e))
}

fn compose_prompt(request: &PlaygroundInferenceRequest) -> String {
    if let Some(system_prompt) = request.system_prompt.as_deref() {
        if !system_prompt.trim().is_empty() {
            return format!(
                "System:\n{}\n\nUser:\n{}",
                system_prompt.trim(),
                request.prompt.trim()
            );
        }
    }
    request.prompt.trim().to_string()
}

async fn infer_ollama(
    client: &Client,
    backend: &AiBackendConfig,
    request: &PlaygroundInferenceRequest,
) -> Result<(String, Option<String>, Option<InferenceTokenUsage>), String> {
    let endpoint = backend
        .endpoint
        .as_deref()
        .ok_or_else(|| "local_llama backend requires endpoint".to_string())?;
    let url = format!("{}/api/generate", endpoint.trim_end_matches('/'));

    let mut payload = json!({
        "model": backend.model,
        "prompt": compose_prompt(request),
        "stream": false,
    });

    let mut options = serde_json::Map::new();
    if let Some(temp) = request.temperature.or(backend.temperature) {
        options.insert("temperature".to_string(), json!(temp));
    }
    if let Some(max_tokens) = request.max_tokens.or(backend.max_tokens) {
        options.insert("num_predict".to_string(), json!(max_tokens));
    }
    if !options.is_empty() {
        payload["options"] = Value::Object(options);
    }

    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("ollama request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("ollama error {}: {}", status, truncate(&body, 320)));
    }

    let body: OllamaResponse = response
        .json()
        .await
        .map_err(|e| format!("invalid ollama response: {}", e))?;

    Ok((
        body.response.trim().to_string(),
        Some("stop".to_string()),
        Some(InferenceTokenUsage {
            input_tokens: to_u32(body.prompt_eval_count),
            output_tokens: to_u32(body.eval_count),
            total_tokens: add_tokens(body.prompt_eval_count, body.eval_count),
        }),
    ))
}

async fn infer_openai_compatible(
    client: &Client,
    backend: &AiBackendConfig,
    request: &PlaygroundInferenceRequest,
    default_endpoint: &str,
) -> Result<(String, Option<String>, Option<InferenceTokenUsage>), String> {
    let api_key = backend
        .api_key
        .as_deref()
        .ok_or_else(|| format!("{:?} backend requires api_key", backend.kind))?;
    let url = resolve_chat_endpoint(backend.endpoint.as_deref(), default_endpoint);

    let mut messages = Vec::new();
    if let Some(system_prompt) = request.system_prompt.as_deref() {
        if !system_prompt.trim().is_empty() {
            messages.push(json!({
                "role": "system",
                "content": system_prompt,
            }));
        }
    }
    messages.push(json!({
        "role": "user",
        "content": request.prompt,
    }));

    let mut payload = json!({
        "model": backend.model,
        "messages": messages,
    });
    if let Some(temp) = request.temperature.or(backend.temperature) {
        payload["temperature"] = json!(temp);
    }
    if let Some(max_tokens) = request.max_tokens.or(backend.max_tokens) {
        payload["max_tokens"] = json!(max_tokens);
    }

    let response = client
        .post(&url)
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("{:?} request failed: {}", backend.kind, e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "{:?} error {}: {}",
            backend.kind,
            status,
            truncate(&body, 320)
        ));
    }

    let body: OpenAiResponse = response
        .json()
        .await
        .map_err(|e| format!("invalid {:?} response: {}", backend.kind, e))?;

    let choice = body
        .choices
        .first()
        .ok_or_else(|| format!("{:?} response did not include choices", backend.kind))?;
    let output = extract_text(&choice.message.content);

    let usage = body.usage.map(|usage| InferenceTokenUsage {
        input_tokens: to_u32(usage.prompt_tokens),
        output_tokens: to_u32(usage.completion_tokens),
        total_tokens: to_u32(usage.total_tokens),
    });

    Ok((output, choice.finish_reason.clone(), usage))
}

async fn infer_anthropic(
    client: &Client,
    backend: &AiBackendConfig,
    request: &PlaygroundInferenceRequest,
) -> Result<(String, Option<String>, Option<InferenceTokenUsage>), String> {
    let api_key = backend
        .api_key
        .as_deref()
        .ok_or_else(|| "anthropic backend requires api_key".to_string())?;
    let url = resolve_messages_endpoint(backend.endpoint.as_deref(), DEFAULT_ANTHROPIC_ENDPOINT);

    let mut payload = json!({
        "model": backend.model,
        "max_tokens": request.max_tokens.or(backend.max_tokens).unwrap_or(DEFAULT_MAX_TOKENS),
        "messages": [
            {
                "role": "user",
                "content": request.prompt,
            }
        ],
    });

    if let Some(temp) = request.temperature.or(backend.temperature) {
        payload["temperature"] = json!(temp);
    }
    if let Some(system_prompt) = request.system_prompt.as_deref() {
        if !system_prompt.trim().is_empty() {
            payload["system"] = json!(system_prompt);
        }
    }

    let response = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("anthropic request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "anthropic error {}: {}",
            status,
            truncate(&body, 320)
        ));
    }

    let body: AnthropicResponse = response
        .json()
        .await
        .map_err(|e| format!("invalid anthropic response: {}", e))?;

    let output = body
        .content
        .iter()
        .filter(|part| part.content_type == "text")
        .filter_map(|part| part.text.as_deref())
        .collect::<Vec<_>>()
        .join("\n");

    let usage = body.usage.map(|usage| InferenceTokenUsage {
        input_tokens: to_u32(usage.input_tokens),
        output_tokens: to_u32(usage.output_tokens),
        total_tokens: add_tokens(usage.input_tokens, usage.output_tokens),
    });

    Ok((output.trim().to_string(), body.stop_reason, usage))
}

async fn infer_gemini(
    client: &Client,
    backend: &AiBackendConfig,
    request: &PlaygroundInferenceRequest,
) -> Result<(String, Option<String>, Option<InferenceTokenUsage>), String> {
    let api_key = backend
        .api_key
        .as_deref()
        .ok_or_else(|| "gemini backend requires api_key".to_string())?;
    let url = resolve_gemini_endpoint(backend.endpoint.as_deref(), &backend.model, api_key)?;

    let mut payload = json!({
        "contents": [
            {
                "parts": [
                    {
                        "text": request.prompt
                    }
                ]
            }
        ]
    });

    if let Some(system_prompt) = request.system_prompt.as_deref() {
        if !system_prompt.trim().is_empty() {
            payload["systemInstruction"] = json!({
                "parts": [
                    {
                        "text": system_prompt
                    }
                ]
            });
        }
    }

    let mut generation_config = serde_json::Map::new();
    if let Some(temp) = request.temperature.or(backend.temperature) {
        generation_config.insert("temperature".to_string(), json!(temp));
    }
    if let Some(max_tokens) = request.max_tokens.or(backend.max_tokens) {
        generation_config.insert("maxOutputTokens".to_string(), json!(max_tokens));
    }
    if !generation_config.is_empty() {
        payload["generationConfig"] = Value::Object(generation_config);
    }

    let response = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("gemini request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("gemini error {}: {}", status, truncate(&body, 320)));
    }

    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("invalid gemini response: {}", e))?;

    let output = body["candidates"]
        .as_array()
        .and_then(|candidates| candidates.first())
        .and_then(|candidate| candidate["content"]["parts"].as_array())
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    let finish_reason = body["candidates"]
        .as_array()
        .and_then(|candidates| candidates.first())
        .and_then(|candidate| candidate["finishReason"].as_str())
        .map(str::to_string);

    let usage = body["usageMetadata"]
        .as_object()
        .map(|usage| InferenceTokenUsage {
            input_tokens: usage
                .get("promptTokenCount")
                .and_then(Value::as_u64)
                .and_then(|v| v.try_into().ok()),
            output_tokens: usage
                .get("candidatesTokenCount")
                .and_then(Value::as_u64)
                .and_then(|v| v.try_into().ok()),
            total_tokens: usage
                .get("totalTokenCount")
                .and_then(Value::as_u64)
                .and_then(|v| v.try_into().ok()),
        });

    Ok((output.trim().to_string(), finish_reason, usage))
}

fn resolve_chat_endpoint(endpoint: Option<&str>, default_endpoint: &str) -> String {
    let endpoint = endpoint.unwrap_or(default_endpoint);
    if endpoint.contains("/chat/completions") {
        endpoint.to_string()
    } else {
        format!("{}/chat/completions", endpoint.trim_end_matches('/'))
    }
}

fn resolve_messages_endpoint(endpoint: Option<&str>, default_endpoint: &str) -> String {
    let endpoint = endpoint.unwrap_or(default_endpoint);
    if endpoint.ends_with("/messages") {
        endpoint.to_string()
    } else {
        format!("{}/messages", endpoint.trim_end_matches('/'))
    }
}

fn resolve_gemini_endpoint(
    endpoint: Option<&str>,
    model: &str,
    api_key: &str,
) -> Result<Url, String> {
    let endpoint = endpoint.unwrap_or(DEFAULT_GEMINI_ENDPOINT);
    let mut url = if endpoint.contains(":generateContent") {
        Url::parse(endpoint).map_err(|e| format!("invalid gemini endpoint {}: {}", endpoint, e))?
    } else {
        let base = endpoint.trim_end_matches('/');
        let generated = format!("{}/v1beta/models/{}:generateContent", base, model);
        Url::parse(&generated)
            .map_err(|e| format!("invalid gemini endpoint {}: {}", generated, e))?
    };

    if !url.query_pairs().any(|(k, _)| k == "key") {
        url.query_pairs_mut().append_pair("key", api_key);
    }

    Ok(url)
}

fn extract_text(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn to_u32(value: Option<u64>) -> Option<u32> {
    value.and_then(|v| v.try_into().ok())
}

fn add_tokens(left: Option<u64>, right: Option<u64>) -> Option<u32> {
    match (left, right) {
        (Some(l), Some(r)) => l.checked_add(r).and_then(|v| v.try_into().ok()),
        (Some(value), None) | (None, Some(value)) => value.try_into().ok(),
        (None, None) => None,
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}
