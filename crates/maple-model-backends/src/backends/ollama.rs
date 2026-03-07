//! Ollama backend adapter.
//!
//! Connects to a local Ollama instance (default: `http://localhost:11434`)
//! and provides chat inference, streaming, and embedding support.

use crate::inference::*;
use async_trait::async_trait;

/// Ollama backend adapter (localhost:11434 by default).
pub struct OllamaBackend {
    client: reqwest::Client,
    base_url: String,
}

impl OllamaBackend {
    /// Create a new Ollama backend.
    ///
    /// If `base_url` is `None`, defaults to `http://localhost:11434`.
    pub fn new(base_url: Option<&str>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.unwrap_or("http://localhost:11434").to_string(),
        }
    }

    /// Returns the base URL this backend connects to.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Convert a [`MessageRole`] to its Ollama API string representation.
    fn role_str(role: &MessageRole) -> &'static str {
        match role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        }
    }
}

#[async_trait]
impl ModelBackend for OllamaBackend {
    fn backend_id(&self) -> &str {
        "ollama"
    }

    async fn health_check(&self) -> Result<BackendHealth, BackendError> {
        let resp = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?;

        if resp.status().is_success() {
            let body: serde_json::Value = resp.json().await?;
            let models = body["models"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| m["name"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            Ok(BackendHealth {
                available: true,
                latency_ms: None,
                loaded_models: models,
                gpu_utilization: None,
                memory_used_bytes: None,
            })
        } else {
            Err(BackendError::Unavailable(
                "Ollama not responding".into(),
            ))
        }
    }

    async fn list_models(&self) -> Result<Vec<String>, BackendError> {
        let health = self.health_check().await?;
        Ok(health.loaded_models)
    }

    async fn chat(
        &self,
        request: &InferenceRequest,
    ) -> Result<InferenceResponse, BackendError> {
        let ollama_messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": Self::role_str(&m.role),
                    "content": m.content,
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": ollama_messages,
            "stream": false,
        });

        // Add options
        let mut options = serde_json::Map::new();
        if let Some(temp) = request.temperature {
            options.insert("temperature".into(), serde_json::json!(temp));
        }
        if let Some(top_p) = request.top_p {
            options.insert("top_p".into(), serde_json::json!(top_p));
        }
        if let Some(max_tokens) = request.max_tokens {
            options.insert("num_predict".into(), serde_json::json!(max_tokens));
        }
        if !options.is_empty() {
            body["options"] = serde_json::Value::Object(options);
        }

        // Add tools if present
        if !request.tools.is_empty() {
            let tools: Vec<serde_json::Value> = request
                .tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        }
                    })
                })
                .collect();
            body["tools"] = serde_json::json!(tools);
        }

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(BackendError::InferenceFailed(err_text));
        }

        let ollama_resp: serde_json::Value = resp.json().await?;

        let content = ollama_resp["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let tool_calls = ollama_resp["message"]["tool_calls"]
            .as_array()
            .map(|calls| {
                calls
                    .iter()
                    .filter_map(|tc| {
                        Some(ToolCall {
                            id: tc["id"].as_str().unwrap_or("").to_string(),
                            function: FunctionCall {
                                name: tc["function"]["name"].as_str()?.to_string(),
                                arguments: tc["function"]["arguments"].to_string(),
                            },
                        })
                    })
                    .collect()
            });

        let prompt_tokens =
            ollama_resp["prompt_eval_count"].as_u64().unwrap_or(0) as u32;
        let completion_tokens =
            ollama_resp["eval_count"].as_u64().unwrap_or(0) as u32;

        let has_tool_calls = tool_calls.is_some();

        Ok(InferenceResponse {
            message: ChatMessage {
                role: MessageRole::Assistant,
                content,
                tool_calls,
                tool_call_id: None,
            },
            usage: TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
            model: request.model.clone(),
            finish_reason: if has_tool_calls {
                FinishReason::ToolCalls
            } else {
                FinishReason::Stop
            },
            backend_metadata: ollama_resp,
        })
    }

    async fn chat_stream(
        &self,
        request: &InferenceRequest,
    ) -> Result<
        tokio::sync::mpsc::Receiver<Result<StreamEvent, BackendError>>,
        BackendError,
    > {
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let model = request.model.clone();
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": Self::role_str(&m.role),
                    "content": m.content,
                })
            })
            .collect();

        tokio::spawn(async move {
            let body = serde_json::json!({
                "model": model,
                "messages": messages,
                "stream": true,
            });

            let resp = match client
                .post(format!("{}/api/chat", base_url))
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(Err(BackendError::Http(e))).await;
                    return;
                }
            };

            let mut stream = resp.bytes_stream();
            use futures_util::StreamExt;
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                            for line in text.lines() {
                                if let Ok(json) =
                                    serde_json::from_str::<serde_json::Value>(line)
                                {
                                    let delta = json["message"]["content"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string();
                                    let done =
                                        json["done"].as_bool().unwrap_or(false);
                                    let _ = tx
                                        .send(Ok(StreamEvent {
                                            delta,
                                            finish_reason: if done {
                                                Some(FinishReason::Stop)
                                            } else {
                                                None
                                            },
                                            usage: None,
                                        }))
                                        .await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Err(BackendError::StreamError(e.to_string())))
                            .await;
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn embed(
        &self,
        text: &[String],
        model: &str,
    ) -> Result<Vec<Vec<f32>>, BackendError> {
        let mut embeddings = Vec::new();
        for t in text {
            let body = serde_json::json!({
                "model": model,
                "input": t,
            });
            let resp = self
                .client
                .post(format!("{}/api/embed", self.base_url))
                .json(&body)
                .send()
                .await?;
            let json: serde_json::Value = resp.json().await?;
            if let Some(emb) = json["embeddings"].as_array() {
                if let Some(first) = emb.first() {
                    let vec: Vec<f32> = first
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_f64().map(|f| f as f32))
                                .collect()
                        })
                        .unwrap_or_default();
                    embeddings.push(vec);
                }
            }
        }
        Ok(embeddings)
    }
}
