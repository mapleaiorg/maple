//! Agent kernel for MAPLE runtime.
//!
//! This module materializes the first-class runtime primitive:
//! `Agent = Resonator + Profile + CapabilitySet + ContractSet + State`.
//! All consequential capability calls flow through this kernel so the
//! commitment boundary cannot be bypassed.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use aas_capability::GrantRequest;
use aas_identity::{AgentMetadata, AgentType, RegistrationRequest};
use aas_service::AasService;
use aas_types::{AgentId, CommitmentOutcome, Decision, PolicyDecisionCard};
use async_trait::async_trait;
use maple_storage::{AgentCheckpoint, AuditAppend, MapleStorage, QueryWindow};
use rcf_commitment::{CommitmentBuilder, CommitmentId as RcfCommitmentId, RcfCommitment};
use rcf_types::{CapabilityRef, EffectDomain, IdentityRef, ScopeConstraint, TemporalValidity};
use rcf_validator::RcfValidator;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::cognition::{
    AnthropicAdapter, GeminiAdapter, GrokAdapter, LlamaAdapter, ModelAdapter, ModelBackend,
    ModelRequest, OpenAiAdapter, StructuredCognition,
};
use crate::invariants::{IntentContext, MeaningContext, Operation, SystemState};
use crate::runtime_core::{MapleRuntime, ResonatorSpec};
use crate::types::{InvariantViolation, ResonatorId};

/// Stable profile used by the runtime kernel to apply autonomous limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionProfile {
    /// Human-readable profile name.
    pub name: String,
    /// Minimum confidence required for intent stabilization.
    pub min_intent_confidence: f64,
    /// If true, consequential operations require explicit commitment.
    pub require_commitment_for_consequence: bool,
}

impl Default for AgentExecutionProfile {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            min_intent_confidence: 0.65,
            require_commitment_for_consequence: true,
        }
    }
}

/// Capability descriptor bound to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityDescriptor {
    pub name: String,
    pub domain: EffectDomain,
    pub scope: ScopeConstraint,
    /// Marks capability as consequential (side effects beyond pure cognition).
    pub consequential: bool,
}

impl CapabilityDescriptor {
    pub fn safe(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            domain: EffectDomain::Computation,
            scope: ScopeConstraint::global(),
            consequential: false,
        }
    }

    pub fn dangerous(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            domain: EffectDomain::Computation,
            scope: ScopeConstraint::new(
                vec!["wallet:demo".to_string()],
                vec!["transfer".to_string()],
            ),
            consequential: true,
        }
    }
}

/// Immutable view of agent composition stored in the kernel.
#[derive(Debug, Clone)]
pub struct AgentHost {
    pub resonator_id: ResonatorId,
    pub profile: AgentExecutionProfile,
    pub aas_agent_id: AgentId,
    pub identity_ref: IdentityRef,
    pub capability_set: HashMap<String, CapabilityDescriptor>,
    pub contract_set: HashSet<RcfCommitmentId>,
    pub state: AgentState,
}

/// Runtime state of an agent host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Idle,
    FormingMeaning,
    StabilizingIntent,
    AwaitingCommitment,
    Executing,
    Failed,
}

/// Registration input for creating an agent host.
#[derive(Debug, Clone)]
pub struct AgentRegistration {
    pub resonator_spec: ResonatorSpec,
    pub profile: AgentExecutionProfile,
    pub capabilities: Vec<CapabilityDescriptor>,
}

impl Default for AgentRegistration {
    fn default() -> Self {
        Self {
            resonator_spec: ResonatorSpec::default(),
            profile: AgentExecutionProfile::default(),
            capabilities: vec![
                CapabilityDescriptor::safe("echo_log"),
                CapabilityDescriptor::dangerous("simulate_transfer"),
            ],
        }
    }
}

/// Input request for one kernel step.
#[derive(Debug, Clone)]
pub struct AgentHandleRequest {
    pub resonator_id: ResonatorId,
    pub backend: ModelBackend,
    pub prompt: String,
    pub override_tool: Option<String>,
    pub override_args: Option<Value>,
    /// Explicit commitment required for consequential actions.
    pub commitment: Option<RcfCommitment>,
}

impl AgentHandleRequest {
    pub fn new(
        resonator_id: ResonatorId,
        backend: ModelBackend,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            resonator_id,
            backend,
            prompt: prompt.into(),
            override_tool: None,
            override_args: None,
            commitment: None,
        }
    }
}

/// Outcome of a kernel handle call.
#[derive(Debug, Clone)]
pub struct AgentHandleResponse {
    pub resonator_id: ResonatorId,
    pub cognition: StructuredCognition,
    pub raw_model_output: String,
    pub action: Option<CapabilityExecution>,
    pub audit_event_id: String,
}

/// Capability invocation input.
#[derive(Debug, Clone)]
pub struct CapabilityInvocation {
    pub resonator_id: ResonatorId,
    pub capability_name: String,
    pub args: Value,
    pub commitment_id: Option<RcfCommitmentId>,
}

/// Capability execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityExecution {
    pub capability_name: String,
    pub summary: String,
    pub payload: Value,
}

/// Trait for executable capabilities. Implementors never bypass gating.
#[async_trait]
pub trait CapabilityExecutor: Send + Sync {
    fn descriptor(&self) -> CapabilityDescriptor;

    async fn execute(
        &self,
        invocation: &CapabilityInvocation,
    ) -> Result<CapabilityExecution, CapabilityExecutionError>;
}

/// Echo/log capability (safe, non-consequential).
#[derive(Debug, Default)]
pub struct EchoCapability;

#[async_trait]
impl CapabilityExecutor for EchoCapability {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor::safe("echo_log")
    }

    async fn execute(
        &self,
        invocation: &CapabilityInvocation,
    ) -> Result<CapabilityExecution, CapabilityExecutionError> {
        let message = invocation
            .args
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("(no message)");

        Ok(CapabilityExecution {
            capability_name: invocation.capability_name.clone(),
            summary: format!("Echoed message for {}", invocation.resonator_id),
            payload: serde_json::json!({
                "message": message,
                "resonator_id": invocation.resonator_id.to_string(),
            }),
        })
    }
}

/// Simulated money transfer capability (consequential).
#[derive(Debug, Default)]
pub struct SimulatedTransferCapability;

#[async_trait]
impl CapabilityExecutor for SimulatedTransferCapability {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor::dangerous("simulate_transfer")
    }

    async fn execute(
        &self,
        invocation: &CapabilityInvocation,
    ) -> Result<CapabilityExecution, CapabilityExecutionError> {
        let amount = invocation
            .args
            .get("amount")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let to = invocation
            .args
            .get("to")
            .and_then(Value::as_str)
            .unwrap_or("unknown");

        if amount <= 0 {
            return Err(CapabilityExecutionError::ExecutionFailed(
                "amount must be positive".to_string(),
            ));
        }

        Ok(CapabilityExecution {
            capability_name: invocation.capability_name.clone(),
            summary: format!("Simulated transfer of {} units to {}", amount, to),
            payload: serde_json::json!({
                "transfer_id": format!("tx-{}", Uuid::new_v4()),
                "amount": amount,
                "to": to,
                "commitment_id": invocation.commitment_id.as_ref().map(|id| id.0.clone()),
            }),
        })
    }
}

/// Append-only audit event emitted by the kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAuditEvent {
    pub event_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub resonator_id: String,
    pub stage: String,
    pub success: bool,
    pub message: String,
    pub commitment_id: Option<String>,
}

/// Commitment gateway wraps validator + AAS submission + lifecycle recording.
#[derive(Clone)]
pub struct CommitmentGateway {
    aas: Arc<AasService>,
    validator: Arc<RcfValidator>,
}

impl CommitmentGateway {
    pub fn new(aas: Arc<AasService>) -> Self {
        Self {
            aas,
            validator: Arc::new(RcfValidator::new()),
        }
    }

    pub async fn authorize(
        &self,
        commitment: RcfCommitment,
    ) -> Result<PolicyDecisionCard, AgentKernelError> {
        self.validator
            .validate_commitment(&commitment)
            .map_err(|e| AgentKernelError::CommitmentValidation(e.to_string()))?;

        let decision = self
            .aas
            .submit_commitment(commitment)
            .await
            .map_err(|e| AgentKernelError::Aas(e.to_string()))?;

        Ok(decision)
    }

    /// Validate that a commitment is explicitly bound to the capability being executed,
    /// then authorize it through the full RCF + AAS boundary.
    pub async fn authorize_for_capability(
        &self,
        commitment: RcfCommitment,
        principal: &IdentityRef,
        capability: &CapabilityDescriptor,
        expected_capability_id: &str,
    ) -> Result<PolicyDecisionCard, AgentKernelError> {
        self.assert_binding(&commitment, principal, capability, expected_capability_id)?;
        self.authorize(commitment).await
    }

    fn assert_binding(
        &self,
        commitment: &RcfCommitment,
        principal: &IdentityRef,
        capability: &CapabilityDescriptor,
        expected_capability_id: &str,
    ) -> Result<(), AgentKernelError> {
        if commitment.principal.id != principal.id {
            return Err(AgentKernelError::CommitmentCapabilityMismatch {
                capability: expected_capability_id.to_string(),
                commitment_id: commitment.commitment_id.0.clone(),
                reason: "commitment principal does not match agent identity".to_string(),
            });
        }

        if !commitment.is_valid_at(chrono::Utc::now()) {
            return Err(AgentKernelError::CommitmentCapabilityMismatch {
                capability: expected_capability_id.to_string(),
                commitment_id: commitment.commitment_id.0.clone(),
                reason: "commitment is outside temporal validity bounds".to_string(),
            });
        }

        if commitment.effect_domain != capability.domain {
            return Err(AgentKernelError::CommitmentCapabilityMismatch {
                capability: expected_capability_id.to_string(),
                commitment_id: commitment.commitment_id.0.clone(),
                reason: format!(
                    "effect domain mismatch: expected {}, got {}",
                    capability.domain, commitment.effect_domain
                ),
            });
        }

        if !scope_covers(&commitment.scope, &capability.scope) {
            return Err(AgentKernelError::CommitmentCapabilityMismatch {
                capability: expected_capability_id.to_string(),
                commitment_id: commitment.commitment_id.0.clone(),
                reason: "commitment scope does not cover capability scope".to_string(),
            });
        }

        // Prefer explicit capability-id binding. This ensures one capability commitment
        // cannot be replayed for a different consequential capability.
        let explicit_match = commitment
            .required_capabilities
            .iter()
            .any(|cap| cap.capability_id == expected_capability_id);

        if !explicit_match {
            return Err(AgentKernelError::CommitmentCapabilityMismatch {
                capability: expected_capability_id.to_string(),
                commitment_id: commitment.commitment_id.0.clone(),
                reason: "required_capabilities does not include expected capability binding"
                    .to_string(),
            });
        }

        Ok(())
    }

    pub async fn record_execution_started(
        &self,
        commitment_id: &RcfCommitmentId,
    ) -> Result<(), AgentKernelError> {
        self.aas
            .record_execution_started(commitment_id)
            .await
            .map_err(|e| AgentKernelError::Aas(e.to_string()))
    }

    pub async fn record_success(
        &self,
        commitment_id: &RcfCommitmentId,
        description: impl Into<String>,
    ) -> Result<(), AgentKernelError> {
        self.aas
            .record_outcome(
                commitment_id,
                CommitmentOutcome {
                    success: true,
                    description: description.into(),
                    completed_at: chrono::Utc::now(),
                },
            )
            .await
            .map_err(|e| AgentKernelError::Aas(e.to_string()))
    }

    pub async fn record_failure(
        &self,
        commitment_id: &RcfCommitmentId,
        description: impl Into<String>,
    ) -> Result<(), AgentKernelError> {
        self.aas
            .record_outcome(
                commitment_id,
                CommitmentOutcome {
                    success: false,
                    description: description.into(),
                    completed_at: chrono::Utc::now(),
                },
            )
            .await
            .map_err(|e| AgentKernelError::Aas(e.to_string()))
    }
}

/// Non-bypassable execution kernel.
#[derive(Clone)]
pub struct AgentKernel {
    runtime: MapleRuntime,
    aas: Arc<AasService>,
    gateway: CommitmentGateway,
    storage: Arc<dyn MapleStorage>,
    agents: Arc<RwLock<HashMap<ResonatorId, AgentHost>>>,
    capability_executors: Arc<RwLock<HashMap<String, Arc<dyn CapabilityExecutor>>>>,
    model_adapters: Arc<RwLock<HashMap<ModelBackend, Arc<dyn ModelAdapter>>>>,
}

impl AgentKernel {
    /// Create a kernel with default adapters and built-in safe/dangerous capabilities.
    pub fn new(runtime: MapleRuntime) -> Self {
        let storage: Arc<dyn MapleStorage> =
            Arc::new(maple_storage::memory::InMemoryMapleStorage::new());
        Self::new_with_storage(runtime, storage)
    }

    /// Create a kernel with explicit storage backend.
    pub fn new_with_storage(runtime: MapleRuntime, storage: Arc<dyn MapleStorage>) -> Self {
        let aas = Arc::new(AasService::with_storage(Arc::clone(&storage)));
        let gateway = CommitmentGateway::new(Arc::clone(&aas));

        Self {
            runtime,
            aas,
            gateway,
            storage,
            agents: Arc::new(RwLock::new(HashMap::new())),
            capability_executors: Arc::new(RwLock::new(Self::default_capability_executors())),
            model_adapters: Arc::new(RwLock::new(Self::default_model_adapters())),
        }
    }

    /// Access the configured storage backend.
    pub fn storage(&self) -> Arc<dyn MapleStorage> {
        Arc::clone(&self.storage)
    }

    /// Register a custom capability executor.
    pub async fn register_capability_executor(&self, executor: Arc<dyn CapabilityExecutor>) {
        let descriptor = executor.descriptor();
        self.capability_executors
            .write()
            .await
            .insert(descriptor.name.clone(), executor);
    }

    /// Register a custom model adapter.
    pub async fn register_model_adapter(&self, adapter: Arc<dyn ModelAdapter>) {
        self.model_adapters
            .write()
            .await
            .insert(adapter.backend(), adapter);
    }

    /// Register a new agent host in the kernel.
    pub async fn register_agent(
        &self,
        registration: AgentRegistration,
    ) -> Result<AgentHost, AgentKernelError> {
        let resonator = self
            .runtime
            .register_resonator(registration.resonator_spec.clone())
            .await
            .map_err(|e| AgentKernelError::Runtime(e.to_string()))?;

        let registered = self
            .aas
            .register_agent(RegistrationRequest {
                agent_type: AgentType::Resonator,
                metadata: AgentMetadata {
                    name: registration.resonator_spec.identity.name.clone(),
                    description: Some("Registered by maple-runtime AgentKernel".to_string()),
                    owner: Some("maple-runtime".to_string()),
                    tags: vec!["agent_kernel".to_string()],
                    custom: registration.resonator_spec.identity.metadata.clone(),
                },
            })
            .map_err(|e| AgentKernelError::Aas(e.to_string()))?;

        let mut capability_set = HashMap::new();
        for cap in &registration.capabilities {
            self.aas
                .grant_capability(GrantRequest {
                    grantee: registered.agent_id.clone(),
                    domain: cap.domain.clone(),
                    scope: cap.scope.clone(),
                    validity: TemporalValidity::unbounded(),
                    issuer: AgentId::new("maple-runtime"),
                    conditions: vec![],
                })
                .map_err(|e| AgentKernelError::Aas(e.to_string()))?;
            capability_set.insert(cap.name.clone(), cap.clone());
        }

        let host = AgentHost {
            resonator_id: resonator.id,
            profile: registration.profile,
            aas_agent_id: registered.agent_id,
            identity_ref: registered.identity_ref,
            capability_set,
            contract_set: HashSet::new(),
            state: AgentState::Idle,
        };

        self.agents.write().await.insert(resonator.id, host.clone());
        self.persist_checkpoint(&host, None).await?;

        Ok(host)
    }

    /// Draft an explicit commitment for a named capability.
    pub async fn draft_commitment(
        &self,
        resonator_id: ResonatorId,
        capability_name: &str,
        outcome_description: impl Into<String>,
    ) -> Result<RcfCommitment, AgentKernelError> {
        let host = self
            .agents
            .read()
            .await
            .get(&resonator_id)
            .cloned()
            .ok_or_else(|| AgentKernelError::AgentNotFound(resonator_id.to_string()))?;

        let capability = host
            .capability_set
            .get(capability_name)
            .cloned()
            .ok_or_else(|| AgentKernelError::UnknownCapability(capability_name.to_string()))?;

        let capability_ref = CapabilityRef::new(
            format!("cap:{}:{}", resonator_id, capability_name),
            capability.domain.clone(),
            capability.scope.clone(),
            TemporalValidity::unbounded(),
            IdentityRef::new("maple-runtime"),
        );

        CommitmentBuilder::new(host.identity_ref.clone(), capability.domain)
            .with_scope(capability.scope)
            .with_capability(capability_ref)
            .with_outcome(rcf_commitment::IntendedOutcome::new(outcome_description))
            .build()
            .map_err(|e| AgentKernelError::CommitmentValidation(e.to_string()))
    }

    /// Handle one cognitive/actuation step under full invariant + AAS gating.
    pub async fn handle(
        &self,
        request: AgentHandleRequest,
    ) -> Result<AgentHandleResponse, AgentKernelError> {
        let mut host = self
            .agents
            .read()
            .await
            .get(&request.resonator_id)
            .cloned()
            .ok_or_else(|| AgentKernelError::AgentNotFound(request.resonator_id.to_string()))?;

        host.state = AgentState::FormingMeaning;

        self.assert_presence_precedes_meaning(&host)?;

        let adapter = self
            .model_adapters
            .read()
            .await
            .get(&request.backend)
            .cloned()
            .ok_or_else(|| AgentKernelError::ModelAdapterMissing(request.backend.to_string()))?;

        let model_response = adapter
            .infer(&ModelRequest {
                system_prompt: Some(
                    "Return strict JSON with meaning_summary, intent, confidence, and optional suggested_tool"
                        .to_string(),
                ),
                user_prompt: request.prompt.clone(),
                raw_response_override: None,
            })
            .await
            .map_err(|e| AgentKernelError::Model(e.to_string()))?;

        self.assert_meaning_precedes_intent(model_response.cognition.confidence)?;
        host.state = AgentState::StabilizingIntent;

        let selected_tool = request.override_tool.clone().or_else(|| {
            model_response
                .cognition
                .suggested_tool
                .as_ref()
                .map(|s| s.name.clone())
        });

        let selected_args = request.override_args.clone().or_else(|| {
            model_response
                .cognition
                .suggested_tool
                .as_ref()
                .map(|s| s.args.clone())
        });

        // Never execute tools if cognition could not be validated/repaired.
        let action = if model_response.cognition.validation.allows_tool_execution() {
            if let Some(tool_name) = selected_tool {
                let args = selected_args.unwrap_or_else(|| serde_json::json!({}));
                match self
                    .execute_capability(
                        &mut host,
                        tool_name,
                        args,
                        request.commitment,
                        &model_response.cognition,
                    )
                    .await
                {
                    Ok(execution) => Some(execution),
                    Err(err) => {
                        self.agents
                            .write()
                            .await
                            .insert(host.resonator_id, host.clone());
                        self.persist_checkpoint(&host, None).await?;
                        return Err(err);
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        host.state = AgentState::Idle;
        self.agents
            .write()
            .await
            .insert(host.resonator_id, host.clone());
        self.persist_checkpoint(&host, None).await?;

        let audit_event_id = self
            .append_audit(AgentAuditEvent {
                event_id: format!("audit-{}", Uuid::new_v4()),
                timestamp: chrono::Utc::now(),
                resonator_id: host.resonator_id.to_string(),
                stage: "handle_complete".to_string(),
                success: true,
                message: "handle completed".to_string(),
                commitment_id: action
                    .as_ref()
                    .and_then(|a| a.payload.get("commitment_id").and_then(Value::as_str))
                    .map(ToOwned::to_owned),
            })
            .await?;

        Ok(AgentHandleResponse {
            resonator_id: host.resonator_id,
            cognition: model_response.cognition,
            raw_model_output: model_response.raw_text,
            action,
            audit_event_id,
        })
    }

    /// Get immutable audit trail snapshot.
    pub async fn audit_events(&self) -> Vec<AgentAuditEvent> {
        match self
            .storage
            .list_audit(QueryWindow {
                limit: 0,
                offset: 0,
            })
            .await
        {
            Ok(records) => records
                .into_iter()
                .map(|record| AgentAuditEvent {
                    event_id: record.event_id,
                    timestamp: record.timestamp,
                    resonator_id: record.actor,
                    stage: record.stage,
                    success: record.success,
                    message: record.message,
                    commitment_id: record.commitment_id.map(|c| c.0),
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    async fn execute_capability(
        &self,
        host: &mut AgentHost,
        capability_name: String,
        args: Value,
        commitment: Option<RcfCommitment>,
        cognition: &StructuredCognition,
    ) -> Result<CapabilityExecution, AgentKernelError> {
        let capability = host
            .capability_set
            .get(&capability_name)
            .cloned()
            .ok_or_else(|| AgentKernelError::UnknownCapability(capability_name.clone()))?;

        let cap_check = self
            .aas
            .check_capability(&host.aas_agent_id, &capability.domain, &capability.scope)
            .map_err(|e| AgentKernelError::Aas(e.to_string()))?;

        if !cap_check.authorized {
            host.state = AgentState::Failed;
            self.append_audit(AgentAuditEvent {
                event_id: format!("audit-{}", Uuid::new_v4()),
                timestamp: chrono::Utc::now(),
                resonator_id: host.resonator_id.to_string(),
                stage: "capability_check".to_string(),
                success: false,
                message: cap_check
                    .denial_reason
                    .unwrap_or_else(|| "capability denied".to_string()),
                commitment_id: None,
            })
            .await?;
            self.persist_checkpoint(host, None).await?;
            return Err(AgentKernelError::CapabilityDenied);
        }

        let commitment_id = if capability.consequential
            && host.profile.require_commitment_for_consequence
        {
            host.state = AgentState::AwaitingCommitment;
            let commitment = if let Some(commitment) = commitment {
                commitment
            } else {
                self.append_audit(AgentAuditEvent {
                    event_id: format!("audit-{}", Uuid::new_v4()),
                    timestamp: chrono::Utc::now(),
                    resonator_id: host.resonator_id.to_string(),
                    stage: "commitment_required".to_string(),
                    success: false,
                    message: format!(
                        "consequential capability `{}` blocked without commitment",
                        capability_name
                    ),
                    commitment_id: None,
                })
                .await?;
                self.persist_checkpoint(host, None).await?;
                return Err(AgentKernelError::MissingCommitment {
                    capability: capability_name.clone(),
                });
            };

            self.assert_intent_precedes_commitment(
                cognition.confidence,
                host.profile.min_intent_confidence,
            )?;

            let expected_capability_id = format!("cap:{}:{}", host.resonator_id, capability_name);
            let decision = match self
                .gateway
                .authorize_for_capability(
                    commitment.clone(),
                    &host.identity_ref,
                    &capability,
                    &expected_capability_id,
                )
                .await
            {
                Ok(decision) => decision,
                Err(err) => {
                    host.state = AgentState::Failed;
                    self.append_audit(AgentAuditEvent {
                        event_id: format!("audit-{}", Uuid::new_v4()),
                        timestamp: chrono::Utc::now(),
                        resonator_id: host.resonator_id.to_string(),
                        stage: "commitment_authorization".to_string(),
                        success: false,
                        message: err.to_string(),
                        commitment_id: Some(commitment.commitment_id.0.clone()),
                    })
                    .await?;
                    self.persist_checkpoint(host, None).await?;
                    return Err(err);
                }
            };

            if !decision.decision.allows_execution() {
                host.state = AgentState::Failed;
                self.append_audit(AgentAuditEvent {
                    event_id: format!("audit-{}", Uuid::new_v4()),
                    timestamp: chrono::Utc::now(),
                    resonator_id: host.resonator_id.to_string(),
                    stage: "commitment_authorization".to_string(),
                    success: false,
                    message: format!("authorization blocked: {:?}", decision.decision),
                    commitment_id: Some(decision.commitment_id.0.clone()),
                })
                .await?;
                self.persist_checkpoint(host, None).await?;
                return Err(AgentKernelError::ApprovalRequired(decision.decision));
            }

            host.contract_set.insert(commitment.commitment_id.clone());
            Some(commitment.commitment_id)
        } else {
            None
        };

        if let Some(ref cid) = commitment_id {
            self.assert_commitment_precedes_consequence(cid)?;
            self.gateway.record_execution_started(cid).await?;
        }

        host.state = AgentState::Executing;
        self.persist_checkpoint(host, commitment_id.as_ref().map(|c| c.0.clone()))
            .await?;

        let executor = self
            .capability_executors
            .read()
            .await
            .get(&capability_name)
            .cloned()
            .ok_or_else(|| AgentKernelError::ExecutorMissing(capability_name.clone()))?;

        let invocation = CapabilityInvocation {
            resonator_id: host.resonator_id,
            capability_name: capability_name.clone(),
            args,
            commitment_id: commitment_id.clone(),
        };

        let outcome = executor.execute(&invocation).await;

        match outcome {
            Ok(mut execution) => {
                if let Some(ref cid) = commitment_id {
                    self.gateway
                        .record_success(cid, execution.summary.clone())
                        .await?;
                    execution.payload["commitment_id"] = Value::String(cid.0.clone());
                }

                self.append_audit(AgentAuditEvent {
                    event_id: format!("audit-{}", Uuid::new_v4()),
                    timestamp: chrono::Utc::now(),
                    resonator_id: host.resonator_id.to_string(),
                    stage: "capability_execute".to_string(),
                    success: true,
                    message: execution.summary.clone(),
                    commitment_id: commitment_id.as_ref().map(|c| c.0.clone()),
                })
                .await?;
                self.persist_checkpoint(host, commitment_id.as_ref().map(|c| c.0.clone()))
                    .await?;

                Ok(execution)
            }
            Err(err) => {
                host.state = AgentState::Failed;
                if let Some(ref cid) = commitment_id {
                    self.gateway.record_failure(cid, err.to_string()).await?;
                }

                self.append_audit(AgentAuditEvent {
                    event_id: format!("audit-{}", Uuid::new_v4()),
                    timestamp: chrono::Utc::now(),
                    resonator_id: host.resonator_id.to_string(),
                    stage: "capability_execute".to_string(),
                    success: false,
                    message: err.to_string(),
                    commitment_id: commitment_id.as_ref().map(|c| c.0.clone()),
                })
                .await?;
                self.persist_checkpoint(host, commitment_id.as_ref().map(|c| c.0.clone()))
                    .await?;

                Err(AgentKernelError::CapabilityExecution(err.to_string()))
            }
        }
    }

    fn assert_presence_precedes_meaning(&self, host: &AgentHost) -> Result<(), AgentKernelError> {
        let mut state = SystemState::new();
        if self
            .runtime
            .presence_fabric()
            .get_presence(&host.resonator_id)
            .is_some()
        {
            state.register_present(host.resonator_id);
        }

        self.runtime
            .invariant_guard()
            .check(
                &Operation::FormMeaning {
                    resonator: host.resonator_id,
                },
                &state,
            )
            .map_err(Into::into)
    }

    fn assert_meaning_precedes_intent(&self, confidence: f64) -> Result<(), AgentKernelError> {
        self.runtime
            .invariant_guard()
            .check(
                &Operation::StabilizeIntent {
                    meaning: MeaningContext { confidence },
                },
                &SystemState::new(),
            )
            .map_err(Into::into)
    }

    fn assert_intent_precedes_commitment(
        &self,
        confidence: f64,
        threshold: f64,
    ) -> Result<(), AgentKernelError> {
        self.runtime
            .invariant_guard()
            .check(
                &Operation::CreateCommitment {
                    intent: IntentContext::from_confidence(confidence, threshold),
                },
                &SystemState::new(),
            )
            .map_err(Into::into)
    }

    fn assert_commitment_precedes_consequence(
        &self,
        commitment_id: &RcfCommitmentId,
    ) -> Result<(), AgentKernelError> {
        let mut state = SystemState::new();
        state.register_external_commitment(commitment_id.0.clone());

        self.runtime
            .invariant_guard()
            .check(
                &Operation::ProduceExternalConsequence {
                    commitment_ref: commitment_id.0.clone(),
                },
                &state,
            )
            .map_err(Into::into)
    }

    async fn append_audit(&self, event: AgentAuditEvent) -> Result<String, AgentKernelError> {
        let event_id = event.event_id.clone();
        self.storage
            .append_audit(AuditAppend {
                timestamp: event.timestamp,
                actor: event.resonator_id,
                stage: event.stage,
                success: event.success,
                message: event.message,
                commitment_id: event.commitment_id.map(RcfCommitmentId::new),
                payload: serde_json::json!({ "event_id": event_id }),
            })
            .await
            .map_err(|e| AgentKernelError::Storage(e.to_string()))?;
        Ok(event_id)
    }

    async fn persist_checkpoint(
        &self,
        host: &AgentHost,
        last_audit_event_id: Option<String>,
    ) -> Result<(), AgentKernelError> {
        let checkpoint = AgentCheckpoint {
            resonator_id: host.resonator_id.to_string(),
            profile_name: host.profile.name.clone(),
            state: format!("{:?}", host.state),
            active_commitments: host.contract_set.iter().map(|c| c.0.clone()).collect(),
            last_audit_event_id,
            metadata: serde_json::json!({
                "aas_agent_id": host.aas_agent_id.0,
                "capabilities": host.capability_set.keys().cloned().collect::<Vec<_>>(),
            }),
            updated_at: chrono::Utc::now(),
        };
        self.storage
            .upsert_checkpoint(checkpoint)
            .await
            .map_err(|e| AgentKernelError::Storage(e.to_string()))
    }

    fn default_capability_executors() -> HashMap<String, Arc<dyn CapabilityExecutor>> {
        let mut executors: HashMap<String, Arc<dyn CapabilityExecutor>> = HashMap::new();
        executors.insert("echo_log".to_string(), Arc::new(EchoCapability));
        executors.insert(
            "simulate_transfer".to_string(),
            Arc::new(SimulatedTransferCapability),
        );
        executors
    }

    fn default_model_adapters() -> HashMap<ModelBackend, Arc<dyn ModelAdapter>> {
        let mut adapters: HashMap<ModelBackend, Arc<dyn ModelAdapter>> = HashMap::new();
        adapters.insert(
            ModelBackend::LocalLlama,
            Arc::new(LlamaAdapter::new("llama3.2")),
        );
        adapters.insert(
            ModelBackend::OpenAi,
            Arc::new(OpenAiAdapter::new("gpt-4o-mini")),
        );
        adapters.insert(
            ModelBackend::Anthropic,
            Arc::new(AnthropicAdapter::new("claude-3-5-sonnet")),
        );
        adapters.insert(
            ModelBackend::Gemini,
            Arc::new(GeminiAdapter::new("gemini-2.0-flash")),
        );
        adapters.insert(ModelBackend::Grok, Arc::new(GrokAdapter::new("grok-2")));
        adapters
    }
}

fn scope_covers(granted_scope: &ScopeConstraint, required_scope: &ScopeConstraint) -> bool {
    if required_scope.targets.is_empty() || required_scope.operations.is_empty() {
        return true;
    }

    required_scope.targets.iter().all(|target| {
        required_scope
            .operations
            .iter()
            .all(|operation| granted_scope.matches(target, operation))
    })
}

/// Capability execution errors.
#[derive(Debug, thiserror::Error)]
pub enum CapabilityExecutionError {
    #[error("capability execution failed: {0}")]
    ExecutionFailed(String),
}

/// Agent kernel errors.
#[derive(Debug, thiserror::Error)]
pub enum AgentKernelError {
    #[error("agent not found: {0}")]
    AgentNotFound(String),

    #[error("runtime error: {0}")]
    Runtime(String),

    #[error("AAS error: {0}")]
    Aas(String),

    #[error("model adapter missing: {0}")]
    ModelAdapterMissing(String),

    #[error("model error: {0}")]
    Model(String),

    #[error("unknown capability: {0}")]
    UnknownCapability(String),

    #[error("missing executor for capability: {0}")]
    ExecutorMissing(String),

    #[error("capability denied")]
    CapabilityDenied,

    #[error("missing explicit commitment for consequential capability `{capability}`")]
    MissingCommitment { capability: String },

    #[error("commitment validation failed: {0}")]
    CommitmentValidation(String),

    #[error("commitment `{commitment_id}` is not authorized for `{capability}`: {reason}")]
    CommitmentCapabilityMismatch {
        capability: String,
        commitment_id: String,
        reason: String,
    },

    #[error("approval required before consequence: {0:?}")]
    ApprovalRequired(Decision),

    #[error("capability execution failed: {0}")]
    CapabilityExecution(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("invariant violation: {0}")]
    Invariant(#[from] InvariantViolation),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RuntimeConfig;
    use rcf_types::TemporalValidity;

    #[tokio::test]
    async fn dangerous_capability_denied_without_commitment() {
        let runtime = MapleRuntime::bootstrap(RuntimeConfig::default())
            .await
            .unwrap();
        let kernel = AgentKernel::new(runtime);

        let host = kernel
            .register_agent(AgentRegistration::default())
            .await
            .unwrap();

        let mut req = AgentHandleRequest::new(
            host.resonator_id,
            ModelBackend::LocalLlama,
            "transfer 500 usd to demo account",
        );
        req.override_tool = Some("simulate_transfer".to_string());
        req.override_args = Some(serde_json::json!({"amount": 500, "to": "demo"}));

        let err = kernel.handle(req).await.err().expect("must fail");
        assert!(matches!(
            err,
            AgentKernelError::MissingCommitment { capability } if capability == "simulate_transfer"
        ));
    }

    #[tokio::test]
    async fn dangerous_capability_allowed_with_commitment_and_audited() {
        let runtime = MapleRuntime::bootstrap(RuntimeConfig::default())
            .await
            .unwrap();
        let kernel = AgentKernel::new(runtime);

        let host = kernel
            .register_agent(AgentRegistration::default())
            .await
            .unwrap();

        let commitment = kernel
            .draft_commitment(
                host.resonator_id,
                "simulate_transfer",
                "Simulate low-risk transfer for test",
            )
            .await
            .unwrap();

        let mut req = AgentHandleRequest::new(
            host.resonator_id,
            ModelBackend::LocalLlama,
            "transfer 500 usd to demo account",
        );
        req.override_tool = Some("simulate_transfer".to_string());
        req.override_args = Some(serde_json::json!({"amount": 500, "to": "demo"}));
        req.commitment = Some(commitment.clone());

        let response = kernel.handle(req).await.unwrap();
        let action = response.action.expect("action expected");

        assert_eq!(action.capability_name, "simulate_transfer");
        assert_eq!(
            action
                .payload
                .get("commitment_id")
                .and_then(Value::as_str)
                .unwrap(),
            commitment.commitment_id.0
        );

        let entry = kernel
            .aas
            .get_commitment(&commitment.commitment_id)
            .await
            .unwrap()
            .expect("ledger entry");
        assert!(entry.outcome.is_some());
        assert!(entry.outcome.unwrap().success);
    }

    #[tokio::test]
    async fn gating_behavior_consistent_across_backends() {
        let runtime = MapleRuntime::bootstrap(RuntimeConfig::default())
            .await
            .unwrap();
        let kernel = AgentKernel::new(runtime);
        let host = kernel
            .register_agent(AgentRegistration::default())
            .await
            .unwrap();

        for backend in [
            ModelBackend::LocalLlama,
            ModelBackend::OpenAi,
            ModelBackend::Anthropic,
            ModelBackend::Gemini,
            ModelBackend::Grok,
        ] {
            let mut req = AgentHandleRequest::new(
                host.resonator_id,
                backend,
                "transfer 500 usd to demo account",
            );
            req.override_tool = Some("simulate_transfer".to_string());
            req.override_args = Some(serde_json::json!({"amount": 500, "to": "demo"}));

            let err = kernel.handle(req).await.err().expect("must fail");
            assert!(matches!(err, AgentKernelError::MissingCommitment { .. }));
        }
    }

    #[tokio::test]
    async fn dangerous_capability_rejects_commitment_for_different_capability() {
        let runtime = MapleRuntime::bootstrap(RuntimeConfig::default())
            .await
            .unwrap();
        let kernel = AgentKernel::new(runtime);

        let host = kernel
            .register_agent(AgentRegistration::default())
            .await
            .unwrap();

        // Commitment explicitly bound to the safe capability, not transfer.
        let wrong_commitment = kernel
            .draft_commitment(host.resonator_id, "echo_log", "Echo operation")
            .await
            .unwrap();

        let mut req = AgentHandleRequest::new(
            host.resonator_id,
            ModelBackend::LocalLlama,
            "transfer 500 usd to demo account",
        );
        req.override_tool = Some("simulate_transfer".to_string());
        req.override_args = Some(serde_json::json!({"amount": 500, "to": "demo"}));
        req.commitment = Some(wrong_commitment.clone());

        let err = kernel.handle(req).await.err().expect("must fail");
        assert!(matches!(
            err,
            AgentKernelError::CommitmentCapabilityMismatch { .. }
        ));
    }

    #[tokio::test]
    async fn dangerous_capability_rejects_expired_commitment() {
        let runtime = MapleRuntime::bootstrap(RuntimeConfig::default())
            .await
            .unwrap();
        let kernel = AgentKernel::new(runtime);

        let host = kernel
            .register_agent(AgentRegistration::default())
            .await
            .unwrap();

        let mut commitment = kernel
            .draft_commitment(
                host.resonator_id,
                "simulate_transfer",
                "Transfer with explicit commitment",
            )
            .await
            .unwrap();
        let now = chrono::Utc::now();
        commitment.temporal_validity = TemporalValidity::new(
            now - chrono::Duration::minutes(10),
            now - chrono::Duration::minutes(1),
        );

        let mut req = AgentHandleRequest::new(
            host.resonator_id,
            ModelBackend::LocalLlama,
            "transfer 500 usd to demo account",
        );
        req.override_tool = Some("simulate_transfer".to_string());
        req.override_args = Some(serde_json::json!({"amount": 500, "to": "demo"}));
        req.commitment = Some(commitment);

        let err = kernel.handle(req).await.err().expect("must fail");
        assert!(matches!(
            err,
            AgentKernelError::CommitmentCapabilityMismatch { .. }
        ));
    }

    #[tokio::test]
    async fn register_agent_persists_checkpoint() {
        let runtime = MapleRuntime::bootstrap(RuntimeConfig::default())
            .await
            .unwrap();
        let kernel = AgentKernel::new(runtime);

        let host = kernel
            .register_agent(AgentRegistration::default())
            .await
            .unwrap();

        let checkpoint = kernel
            .storage()
            .get_checkpoint(&host.resonator_id.to_string())
            .await
            .unwrap()
            .expect("checkpoint should exist");
        assert_eq!(checkpoint.resonator_id, host.resonator_id.to_string());
    }
}
