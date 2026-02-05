use std::sync::Arc;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Deserialize;

use super::{
    infer_with_parser, parse_json_with_normalization, synthesize_raw_response,
    CapabilityCallCandidate, CognitionState, ContractDraft, EpisodicSummary, IntentDraft,
    JournalSliceItem, MeaningDraft, MeaningInput, ModelAdapter, ModelAdapterError, ModelBackend,
    ModelErrorKind, ModelProviderConfig, ModelRequest, ModelResponse, ModelTask, ModelTransport,
    SyntheticTransport, TransportRequest,
};

/// Llama-first adapter with strict JSON contract and deterministic repair.
///
/// The adapter never executes tools directly. It only returns validated proposals.
#[derive(Clone)]
pub struct LlamaAdapter {
    config: ModelProviderConfig,
    transport: Arc<dyn ModelTransport>,
}

impl std::fmt::Debug for LlamaAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlamaAdapter")
            .field("config", &self.config)
            .finish()
    }
}

impl LlamaAdapter {
    pub fn new(model: impl Into<String>) -> Self {
        Self::with_transport(model, Arc::new(SyntheticTransport))
    }

    pub fn with_transport(model: impl Into<String>, transport: Arc<dyn ModelTransport>) -> Self {
        Self {
            config: ModelProviderConfig::llama(model),
            transport,
        }
    }

    async fn run_json_task<T: DeserializeOwned>(
        &self,
        task: ModelTask,
        schema_hint: &str,
        payload: serde_json::Value,
    ) -> Result<T, ModelAdapterError> {
        let system_prompt = format!(
            "You are a cognition backend for MAPLE. Return JSON only. No prose, no markdown. Schema hint: {}",
            schema_hint
        );
        let user_prompt = serde_json::to_string(&payload).map_err(|err| {
            ModelAdapterError::new(
                self.backend(),
                ModelErrorKind::Parse,
                format!("failed to serialize task payload: {}", err),
            )
        })?;

        let request = TransportRequest {
            task,
            system_prompt,
            user_prompt,
        };
        let raw = self.transport.generate(&self.config, &request).await?;

        if let Some(parsed) = parse_json_with_normalization::<T>(&raw) {
            return Ok(parsed);
        }

        let repair_request = TransportRequest {
            task: ModelTask::RepairJson,
            system_prompt:
                "Repair ONLY the JSON. Do not add commentary. Output one strict JSON value."
                    .to_string(),
            user_prompt: format!("Schema hint: {}\nInvalid output:\n{}", schema_hint, raw),
        };
        let repaired = self
            .transport
            .generate(&self.config, &repair_request)
            .await?;

        parse_json_with_normalization::<T>(&repaired).ok_or_else(|| {
            ModelAdapterError::new(
                self.backend(),
                ModelErrorKind::Parse,
                format!(
                    "llama output for {:?} could not be repaired into strict JSON",
                    task
                ),
            )
        })
    }
}

#[async_trait]
impl ModelAdapter for LlamaAdapter {
    fn backend(&self) -> ModelBackend {
        ModelBackend::LocalLlama
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

    async fn propose_meaning(
        &self,
        input: &MeaningInput,
        state: &CognitionState,
    ) -> Result<MeaningDraft, ModelAdapterError> {
        #[derive(Debug, Deserialize)]
        struct MeaningPayload {
            summary: String,
            #[serde(default)]
            ambiguity_notes: Vec<String>,
            confidence: f64,
        }

        let payload = serde_json::json!({
            "state": state,
            "input": input,
        });
        let parsed: MeaningPayload = self
            .run_json_task(
                ModelTask::ProposeMeaning,
                "{summary:string, ambiguity_notes:string[], confidence:number}",
                payload,
            )
            .await?;

        Ok(MeaningDraft {
            summary: parsed.summary,
            ambiguity_notes: parsed.ambiguity_notes,
            confidence: parsed.confidence.clamp(0.0, 1.0),
        })
    }

    async fn propose_intent(
        &self,
        meaning: &MeaningDraft,
        state: &CognitionState,
    ) -> Result<IntentDraft, ModelAdapterError> {
        #[derive(Debug, Deserialize)]
        struct IntentPayload {
            objective: String,
            #[serde(default)]
            steps: Vec<String>,
            confidence: f64,
            blocking_ambiguity: bool,
        }

        let payload = serde_json::json!({
            "state": state,
            "meaning": meaning,
        });
        let parsed: IntentPayload = self
            .run_json_task(
                ModelTask::ProposeIntent,
                "{objective:string, steps:string[], confidence:number, blocking_ambiguity:bool}",
                payload,
            )
            .await?;

        Ok(IntentDraft {
            objective: parsed.objective,
            steps: parsed.steps,
            confidence: parsed.confidence.clamp(0.0, 1.0),
            blocking_ambiguity: parsed.blocking_ambiguity,
        })
    }

    async fn draft_contract(
        &self,
        intent: &IntentDraft,
        state: &CognitionState,
    ) -> Result<ContractDraft, ModelAdapterError> {
        let payload = serde_json::json!({
            "state": state,
            "intent": intent,
            "requirements": {
                "rcf_compatible": true,
                "must_include": [
                    "effect_domain",
                    "scope",
                    "temporal_validity",
                    "intended_outcome"
                ]
            }
        });
        self.run_json_task(
            ModelTask::DraftContract,
            "{effect_domain, scope, temporal_validity, intended_outcome, required_capability_ids, confidence_context, platform_data}",
            payload,
        )
        .await
    }

    async fn suggest_capability_calls(
        &self,
        contract: &ContractDraft,
        state: &CognitionState,
    ) -> Result<Vec<CapabilityCallCandidate>, ModelAdapterError> {
        let payload = serde_json::json!({
            "state": state,
            "contract": contract,
            "constraints": {
                "must_be_unambiguous": true,
                "must_include_required_contract_fields": true
            }
        });

        let parsed: Vec<CapabilityCallCandidate> = self
            .run_json_task(
                ModelTask::SuggestCapabilityCalls,
                "[{capability_id, params_json, risk_score, rationale, required_contract_fields}]",
                payload,
            )
            .await?;

        parsed
            .into_iter()
            .map(|candidate| {
                if candidate.capability_id.trim().is_empty() {
                    return Err(ModelAdapterError::new(
                        self.backend(),
                        ModelErrorKind::Parse,
                        "ambiguous capability suggestion: capability_id is empty",
                    ));
                }
                if !candidate.params_json.is_object() {
                    return Err(ModelAdapterError::new(
                        self.backend(),
                        ModelErrorKind::Parse,
                        format!(
                            "ambiguous capability suggestion `{}`: params_json must be an object",
                            candidate.capability_id
                        ),
                    ));
                }

                let required_contract_fields = candidate
                    .required_contract_fields
                    .iter()
                    .map(|field| field.trim().to_string())
                    .filter(|field| !field.is_empty())
                    .collect::<Vec<_>>();
                if required_contract_fields.is_empty() {
                    return Err(ModelAdapterError::new(
                        self.backend(),
                        ModelErrorKind::Parse,
                        format!(
                            "ambiguous capability suggestion `{}`: required_contract_fields is empty",
                            candidate.capability_id
                        ),
                    ));
                }

                Ok(CapabilityCallCandidate {
                    capability_id: candidate.capability_id.trim().to_string(),
                    params_json: candidate.params_json,
                    risk_score: candidate.risk_score.clamp(0.0, 1.0),
                    rationale: candidate.rationale.trim().to_string(),
                    required_contract_fields,
                })
            })
            .collect()
    }

    async fn summarize(
        &self,
        journal_slice: &[JournalSliceItem],
    ) -> Result<EpisodicSummary, ModelAdapterError> {
        let payload = serde_json::json!({
            "journal_slice": journal_slice
        });
        self.run_json_task(
            ModelTask::Summarize,
            "{summary:string, key_points:string[], open_questions:string[]}",
            payload,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use super::*;

    #[derive(Debug)]
    struct MockTransport {
        responses: Mutex<VecDeque<String>>,
        calls: std::sync::atomic::AtomicUsize,
    }

    impl MockTransport {
        fn new(responses: Vec<String>) -> Self {
            Self {
                responses: Mutex::new(VecDeque::from(responses)),
                calls: std::sync::atomic::AtomicUsize::new(0),
            }
        }

        fn call_count(&self) -> usize {
            self.calls.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl ModelTransport for MockTransport {
        async fn generate(
            &self,
            _config: &ModelProviderConfig,
            _request: &TransportRequest,
        ) -> Result<String, ModelAdapterError> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut guard = self.responses.lock().expect("lock");
            let Some(next) = guard.pop_front() else {
                return Err(ModelAdapterError::new(
                    ModelBackend::LocalLlama,
                    ModelErrorKind::Transport,
                    "mock transport exhausted",
                ));
            };
            Ok(next)
        }
    }

    #[tokio::test]
    async fn repair_path_recovers_invalid_meaning_json() {
        let transport = Arc::new(MockTransport::new(vec![
            "not-json".to_string(),
            r#"{"summary":"move funds","ambiguity_notes":[],"confidence":0.9}"#.to_string(),
        ]));
        let adapter = LlamaAdapter::with_transport("llama3.2", transport.clone());

        let meaning = adapter
            .propose_meaning(
                &MeaningInput {
                    utterance: "transfer 500".to_string(),
                    metadata: serde_json::json!({}),
                },
                &CognitionState::default(),
            )
            .await
            .expect("repair should recover");

        assert_eq!(meaning.summary, "move funds");
        assert_eq!(transport.call_count(), 2);
    }

    #[tokio::test]
    async fn repair_path_fails_explicitly_when_json_stays_invalid() {
        let transport = Arc::new(MockTransport::new(vec![
            "not-json".to_string(),
            "still-not-json".to_string(),
        ]));
        let adapter = LlamaAdapter::with_transport("llama3.2", transport.clone());

        let err = adapter
            .propose_meaning(
                &MeaningInput {
                    utterance: "transfer 500".to_string(),
                    metadata: serde_json::json!({}),
                },
                &CognitionState::default(),
            )
            .await
            .expect_err("invalid payload must fail explicitly");

        assert_eq!(err.kind, ModelErrorKind::Parse);
        assert_eq!(transport.call_count(), 2);
    }

    #[tokio::test]
    async fn ambiguous_capability_candidate_is_rejected() {
        let transport = Arc::new(MockTransport::new(vec![
            r#"[{"capability_id":"","params_json":{"amount":1},"risk_score":0.4,"rationale":"x","required_contract_fields":["scope"]}]"#
                .to_string(),
        ]));
        let adapter = LlamaAdapter::with_transport("llama3.2", transport);

        let err = adapter
            .suggest_capability_calls(
                &ContractDraft {
                    effect_domain: rcf_types::EffectDomain::Computation,
                    scope: rcf_types::ScopeConstraint::global(),
                    temporal_validity: rcf_types::TemporalValidity::unbounded(),
                    intended_outcome: "test".to_string(),
                    required_capability_ids: vec!["echo_log".to_string()],
                    confidence_context: 0.8,
                    platform_data: serde_json::json!({}),
                },
                &CognitionState::default(),
            )
            .await
            .expect_err("ambiguous capability must be rejected");

        assert_eq!(err.kind, ModelErrorKind::Parse);
    }
}
