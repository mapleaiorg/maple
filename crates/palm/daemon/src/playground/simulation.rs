//! Heavy simulation loop for playground activity

use super::llm;
use crate::error::StorageError;
use crate::storage::Storage;
use palm_shared_state::{
    Activity, ActivityActor, CouplingSnapshot, PlaygroundConfig, PlaygroundInferenceRequest,
    PresenceSnapshot, ResonatorStatus, ResonatorStatusKind,
};
use palm_types::{
    instance::{
        AgentInstance, HealthStatus, InstanceMetrics, InstancePlacement, InstanceStatus,
        ResonatorIdRef,
    },
    AgentSpec, Deployment, DeploymentStatus, DeploymentStrategy, PlatformProfile, ReplicaConfig,
};
use rand::{seq::SliceRandom, Rng, SeedableRng};
use std::collections::{HashMap, HashSet};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::sync::{broadcast, RwLock};
use tokio::time::{sleep, Duration};

const INFERENCE_ERROR_COOLDOWN_TICKS: u64 = 12;

#[derive(Clone)]
struct AgentSimulationContext {
    id: String,
    resonator_id: String,
    attention_utilization: f64,
    active_couplings: u32,
    status: String,
    health: String,
}

/// Simulation engine for playground data
pub struct SimulationEngine {
    storage: Arc<dyn Storage>,
    activity_tx: broadcast::Sender<Activity>,
    config: Arc<RwLock<PlaygroundConfig>>,
    tick_counter: AtomicU64,
    last_inference_error_tick: AtomicU64,
}

impl SimulationEngine {
    pub fn new(
        storage: Arc<dyn Storage>,
        activity_tx: broadcast::Sender<Activity>,
        config: Arc<RwLock<PlaygroundConfig>>,
    ) -> Self {
        Self {
            storage,
            activity_tx,
            config,
            tick_counter: AtomicU64::new(0),
            last_inference_error_tick: AtomicU64::new(0),
        }
    }

    pub async fn run(self) {
        loop {
            let config = self.config.read().await.clone();
            if !config.simulation.enabled {
                sleep(Duration::from_millis(1000)).await;
                continue;
            }

            if let Err(err) = self.tick(&config).await {
                tracing::warn!(error = %err, "Playground simulation tick failed");
            }

            sleep(Duration::from_millis(config.simulation.tick_interval_ms)).await;
        }
    }

    async fn tick(&self, config: &PlaygroundConfig) -> Result<(), StorageError> {
        let tick_number = self.tick_counter.fetch_add(1, Ordering::Relaxed) + 1;
        self.ensure_seed_data(config).await?;

        let specs = self.storage.list_specs().await?;
        let mut spec_map: HashMap<String, AgentSpec> = HashMap::new();
        let mut playground_spec_ids = HashSet::new();

        for spec in specs {
            let spec_id = spec.id.to_string();
            if is_playground_spec(&spec) {
                playground_spec_ids.insert(spec_id.clone());
            }
            spec_map.insert(spec_id, spec);
        }

        let deployments = self.storage.list_deployments().await?;
        let playground_deployments: Vec<Deployment> = deployments
            .into_iter()
            .filter(|d| playground_spec_ids.contains(&d.agent_spec_id.to_string()))
            .collect();

        let mut playground_instances: Vec<AgentInstance> = Vec::new();
        for deployment in &playground_deployments {
            let mut instances = self
                .storage
                .list_instances_for_deployment(&deployment.id)
                .await?;
            playground_instances.append(&mut instances);
        }

        let playground_resonator_ids: HashSet<String> = playground_instances
            .iter()
            .map(|i| i.resonator_id.to_string())
            .collect();

        let mut resonators: Vec<ResonatorStatus> = self.storage.list_resonators().await?;
        let mut resonator_map: HashMap<String, ResonatorStatus> = resonators
            .drain(..)
            .filter(|r| playground_resonator_ids.contains(&r.id))
            .map(|r| (r.id.clone(), r))
            .collect();

        let mut rng = rand::rngs::StdRng::from_entropy();
        let now = chrono::Utc::now();

        // Ensure resonators for all playground instances
        for instance in &playground_instances {
            let resonator_id = instance.resonator_id.to_string();
            if !resonator_map.contains_key(&resonator_id) {
                let resonator = ResonatorStatus {
                    id: resonator_id.clone(),
                    name: format!("Resonator {}", short_id(&resonator_id)),
                    status: ResonatorStatusKind::Idle,
                    presence: PresenceSnapshot::default(),
                    couplings: Vec::new(),
                    attention_utilization: 0.2,
                    last_activity: now,
                    updated_at: now,
                };
                resonator_map.insert(resonator_id.clone(), resonator.clone());
                self.storage.upsert_resonator(resonator).await?;
            }
        }

        let resonator_ids: Vec<String> = resonator_map.keys().cloned().collect();

        // Update resonators
        for resonator_id in &resonator_ids {
            if let Some(mut resonator) = resonator_map.remove(resonator_id) {
                update_presence(
                    &mut resonator.presence,
                    &mut rng,
                    config.simulation.intensity,
                );
                update_couplings(
                    &mut resonator,
                    &resonator_ids,
                    &mut rng,
                    config.simulation.intensity,
                );

                resonator.attention_utilization =
                    (0.12 * resonator.couplings.len() as f64 + rng.gen_range(0.0..0.4)).min(1.0);

                resonator.status = decide_status(resonator.attention_utilization, &mut rng);
                if matches!(
                    resonator.status,
                    ResonatorStatusKind::Processing | ResonatorStatusKind::WaitingForApproval
                ) {
                    resonator.last_activity = chrono::Utc::now();
                }
                resonator.updated_at = chrono::Utc::now();

                resonator_map.insert(resonator_id.clone(), resonator.clone());
                self.storage.upsert_resonator(resonator).await?;
            }
        }

        // Update instances based on resonator state
        let mut activities_emitted = 0usize;
        let activity_budget = (config.simulation.intensity * 6.0).ceil() as usize + 1;
        let mut agent_contexts: Vec<AgentSimulationContext> =
            Vec::with_capacity(playground_instances.len());

        for mut instance in playground_instances {
            let resonator_id = instance.resonator_id.to_string();
            let resonator = resonator_map.get(&resonator_id);

            if let Some(resonator) = resonator {
                let mut metrics = instance.metrics.clone();
                metrics.active_couplings = resonator.couplings.len() as u32;
                metrics.attention_utilization = resonator.attention_utilization;
                metrics.requests_processed += rng.gen_range(20..150) as u64;
                if rng.gen_bool(0.08 * config.simulation.intensity as f64) {
                    metrics.error_count += rng.gen_range(1..4) as u64;
                }
                let target_latency = rng.gen_range(45.0..280.0);
                metrics.avg_response_time_ms =
                    (metrics.avg_response_time_ms * 0.7) + (target_latency * 0.3);

                instance.metrics = metrics;
                instance.last_heartbeat = chrono::Utc::now();

                let error_rate = if instance.metrics.requests_processed > 0 {
                    instance.metrics.error_count as f64 / instance.metrics.requests_processed as f64
                } else {
                    0.0
                };

                instance.health = if error_rate > 0.1 {
                    HealthStatus::Unhealthy {
                        reasons: vec!["error_rate_high".to_string()],
                    }
                } else if error_rate > 0.03 {
                    HealthStatus::Degraded {
                        factors: vec!["error_rate_rising".to_string()],
                    }
                } else {
                    HealthStatus::Healthy
                };

                instance.status = match instance.health {
                    HealthStatus::Unhealthy { .. } => InstanceStatus::Draining {
                        reason: palm_types::instance::DrainReason::HealthFailure,
                    },
                    _ => InstanceStatus::Running,
                };

                agent_contexts.push(AgentSimulationContext {
                    id: instance.id.to_string(),
                    resonator_id: resonator_id.clone(),
                    attention_utilization: instance.metrics.attention_utilization,
                    active_couplings: instance.metrics.active_couplings,
                    status: format!("{:?}", instance.status),
                    health: format!("{:?}", instance.health),
                });

                self.storage.upsert_instance(instance.clone()).await?;

                if activities_emitted < activity_budget && rng.gen_bool(0.3) {
                    activities_emitted += 1;
                    let activity = Activity::new(
                        ActivityActor::Agent,
                        instance.id.to_string(),
                        "agent_activity",
                        format!(
                            "Agent {} processed {} requests",
                            short_id(&instance.id.to_string()),
                            rng.gen_range(10..120)
                        ),
                        serde_json::json!({
                            "deployment_id": instance.deployment_id.to_string(),
                            "attention_utilization": instance.metrics.attention_utilization,
                            "active_couplings": instance.metrics.active_couplings,
                        }),
                    );
                    self.record_activity(activity).await?;
                }
            }
        }

        // Emit resonator activities
        if activities_emitted < activity_budget {
            if let Some(resonator) = resonator_map.values().collect::<Vec<_>>().choose(&mut rng) {
                let activity = Activity::new(
                    ActivityActor::Resonator,
                    resonator.id.clone(),
                    "resonance",
                    format!(
                        "{} synced {} couplings",
                        resonator.name,
                        resonator.couplings.len()
                    ),
                    serde_json::json!({
                        "status": format!("{:?}", resonator.status),
                        "attention_utilization": resonator.attention_utilization,
                    }),
                );
                self.record_activity(activity).await?;
            }
        }

        // Emit a human activity occasionally
        if rng.gen_bool(0.15) {
            let activity = Activity::new(
                ActivityActor::Human,
                "operator-1",
                "operator_action",
                "Operator reviewed system health",
                serde_json::json!({
                    "note": "manual review",
                }),
            );
            self.record_activity(activity).await?;
        }

        if config.simulation.auto_inference_enabled {
            self.maybe_run_auto_inference(
                tick_number,
                config,
                &agent_contexts,
                &resonator_map,
                &mut rng,
            )
            .await?;
        }

        Ok(())
    }

    async fn record_activity(&self, activity: Activity) -> Result<(), StorageError> {
        let stored = self.storage.store_activity(activity).await?;
        let _ = self.activity_tx.send(stored);
        Ok(())
    }

    async fn ensure_seed_data(&self, config: &PlaygroundConfig) -> Result<(), StorageError> {
        let existing_specs = self.storage.list_specs().await?;
        let playground_spec = existing_specs
            .iter()
            .find(|spec| is_playground_spec(spec))
            .cloned();

        let spec = match playground_spec {
            Some(spec) => spec,
            None => {
                let mut spec = AgentSpec::new("playground-sim", semver::Version::new(0, 1, 0));
                spec.platform = PlatformProfile::Development;
                spec.metadata
                    .insert("playground".to_string(), "true".to_string());
                self.storage.upsert_spec(spec.clone()).await?;
                spec
            }
        };

        let deployments = self.storage.list_deployments().await?;
        let playground_deployment = deployments
            .iter()
            .find(|d| d.agent_spec_id == spec.id)
            .cloned();

        let deployment = match playground_deployment {
            Some(deployment) => deployment,
            None => {
                let mut deployment = Deployment {
                    id: palm_types::DeploymentId::generate(),
                    agent_spec_id: spec.id.clone(),
                    version: spec.version.clone(),
                    platform: PlatformProfile::Development,
                    strategy: DeploymentStrategy::default(),
                    status: DeploymentStatus::Pending,
                    replicas: ReplicaConfig::new(3),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };
                deployment.status = DeploymentStatus::InProgress {
                    progress: 5,
                    phase: "seeding".to_string(),
                };
                self.storage.upsert_deployment(deployment.clone()).await?;
                deployment
            }
        };

        let instances = self
            .storage
            .list_instances_for_deployment(&deployment.id)
            .await?;

        if instances.is_empty() {
            let cap = config
                .simulation
                .max_agents
                .min(config.simulation.max_resonators)
                .min(4)
                .max(2);
            let target = cap as usize;
            for _ in 0..target {
                let instance = AgentInstance {
                    id: palm_types::InstanceId::generate(),
                    deployment_id: deployment.id.clone(),
                    resonator_id: ResonatorIdRef::new(format!(
                        "resonator-{}",
                        uuid::Uuid::new_v4()
                    )),
                    status: InstanceStatus::Running,
                    health: HealthStatus::Healthy,
                    placement: InstancePlacement::default(),
                    metrics: InstanceMetrics::default(),
                    started_at: chrono::Utc::now(),
                    last_heartbeat: chrono::Utc::now(),
                };
                self.storage.upsert_instance(instance).await?;
            }
        }

        Ok(())
    }

    async fn maybe_run_auto_inference(
        &self,
        tick_number: u64,
        config: &PlaygroundConfig,
        agent_contexts: &[AgentSimulationContext],
        resonator_map: &HashMap<String, ResonatorStatus>,
        rng: &mut impl Rng,
    ) -> Result<(), StorageError> {
        let interval = config.simulation.inference_interval_ticks.max(1);
        if tick_number % interval != 0 {
            return Ok(());
        }

        if agent_contexts.is_empty() {
            return Ok(());
        }

        if !config.ai_backend.is_configured() {
            let last = self.last_inference_error_tick.load(Ordering::Relaxed);
            if tick_number.saturating_sub(last) >= INFERENCE_ERROR_COOLDOWN_TICKS {
                self.last_inference_error_tick
                    .store(tick_number, Ordering::Relaxed);
                self.record_activity(Activity::new(
                    ActivityActor::System,
                    "playground-simulation",
                    "auto_inference_skipped",
                    format!(
                        "Auto-inference skipped: backend {:?} is not configured",
                        config.ai_backend.kind
                    ),
                    serde_json::json!({
                        "backend_kind": format!("{:?}", config.ai_backend.kind),
                        "backend_model": config.ai_backend.model,
                        "tick": tick_number,
                    }),
                ))
                .await?;
            }
            return Ok(());
        }

        let mut selected = agent_contexts.to_vec();
        selected.shuffle(rng);
        let count =
            usize::max(1, config.simulation.inferences_per_tick as usize).min(selected.len());

        for agent in selected.into_iter().take(count) {
            let resonator = resonator_map.get(&agent.resonator_id);
            let prompt = build_cognition_prompt(&agent, resonator, tick_number);
            let request = PlaygroundInferenceRequest {
                prompt: prompt.clone(),
                system_prompt: Some("You are a MAPLE simulation cognition engine. Keep answers concise, operational, and include one optional UAL statement when relevant.".to_string()),
                actor_id: Some(agent.id.clone()),
                temperature: config.ai_backend.temperature,
                max_tokens: config.ai_backend.max_tokens,
            };

            match llm::infer(&config.ai_backend, &request).await {
                Ok(response) => {
                    let preview = summarize_for_activity(&response.output, 160);
                    self.record_activity(Activity::new(
                        ActivityActor::Agent,
                        agent.id.clone(),
                        "agent_cognition",
                        format!("{} -> {}", short_id(&agent.id), preview),
                        serde_json::json!({
                            "tick": tick_number,
                            "resonator_id": agent.resonator_id,
                            "backend_kind": format!("{:?}", response.backend_kind),
                            "backend_model": response.backend_model,
                            "latency_ms": response.latency_ms,
                            "finish_reason": response.finish_reason,
                            "usage": response.usage,
                            "prompt": prompt,
                            "response": response.output,
                            "attention_utilization": agent.attention_utilization,
                            "active_couplings": agent.active_couplings,
                            "status": agent.status,
                            "health": agent.health,
                        }),
                    ))
                    .await?;
                }
                Err(err) => {
                    let last = self.last_inference_error_tick.load(Ordering::Relaxed);
                    if tick_number.saturating_sub(last) >= INFERENCE_ERROR_COOLDOWN_TICKS {
                        self.last_inference_error_tick
                            .store(tick_number, Ordering::Relaxed);
                        self.record_activity(Activity::new(
                            ActivityActor::System,
                            "playground-simulation",
                            "auto_inference_failed",
                            format!(
                                "Auto-inference failed on backend {:?}",
                                config.ai_backend.kind
                            ),
                            serde_json::json!({
                                "tick": tick_number,
                                "agent_id": agent.id,
                                "backend_kind": format!("{:?}", config.ai_backend.kind),
                                "backend_model": config.ai_backend.model,
                                "error": err,
                            }),
                        ))
                        .await?;
                    }
                }
            }
        }

        Ok(())
    }
}

fn build_cognition_prompt(
    agent: &AgentSimulationContext,
    resonator: Option<&ResonatorStatus>,
    tick_number: u64,
) -> String {
    let (resonator_name, resonator_status, couplings, responsiveness, readiness) = match resonator {
        Some(resonator) => (
            resonator.name.as_str(),
            format!("{:?}", resonator.status),
            resonator.couplings.len(),
            resonator.presence.responsiveness,
            resonator.presence.coupling_readiness,
        ),
        None => ("unknown", "Unknown".to_string(), 0, 0.0, 0.0),
    };

    format!(
        "Tick: {tick_number}\nAgent: {agent_id}\nResonator: {resonator_id} ({resonator_name})\nAgent status: {agent_status}\nAgent health: {agent_health}\nAttention utilization: {attention:.2}\nActive couplings: {active_couplings}\nResonator status: {resonator_status}\nResonator coupling count: {couplings}\nPresence responsiveness: {responsiveness:.2}\nCoupling readiness: {readiness:.2}\n\nProvide:\n1) Brief situational awareness.\n2) Immediate next action.\n3) Risk note.\n4) Optional UAL statement if commitment/deployment action is appropriate.",
        agent_id = agent.id,
        resonator_id = agent.resonator_id,
        agent_status = agent.status,
        agent_health = agent.health,
        attention = agent.attention_utilization,
        active_couplings = agent.active_couplings,
    )
}

fn summarize_for_activity(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let head: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}...", head)
    } else {
        head
    }
}

fn is_playground_spec(spec: &AgentSpec) -> bool {
    if let Some(flag) = spec.metadata.get("playground") {
        if flag == "true" {
            return true;
        }
    }
    spec.name.to_lowercase().contains("playground")
}

fn update_presence(presence: &mut PresenceSnapshot, rng: &mut impl Rng, intensity: f32) {
    let jitter = intensity as f64 * 0.1;
    presence.discoverability = clamp01(presence.discoverability + rng.gen_range(-jitter..jitter));
    presence.responsiveness = clamp01(presence.responsiveness + rng.gen_range(-jitter..jitter));
    presence.stability = clamp01(presence.stability + rng.gen_range(-jitter..jitter));
    presence.coupling_readiness =
        clamp01(presence.coupling_readiness + rng.gen_range(-jitter..jitter));
}

fn update_couplings(
    resonator: &mut ResonatorStatus,
    resonator_ids: &[String],
    rng: &mut impl Rng,
    intensity: f32,
) {
    let drift = intensity as f64 * 0.15;
    for coupling in &mut resonator.couplings {
        coupling.strength = clamp01(coupling.strength + rng.gen_range(-drift..drift));
        coupling.meaning_convergence =
            clamp01(coupling.meaning_convergence + rng.gen_range(-drift..drift));
        coupling.interaction_count += rng.gen_range(0..4) as u64;
    }

    resonator.couplings.retain(|c| c.strength > 0.08);

    if resonator.couplings.len() < 5 && rng.gen_bool(0.4 * intensity as f64) {
        if let Some(peer) = resonator_ids.choose(rng) {
            if peer != &resonator.id && !resonator.couplings.iter().any(|c| c.peer_id == *peer) {
                resonator.couplings.push(CouplingSnapshot {
                    peer_id: peer.clone(),
                    strength: rng.gen_range(0.2..0.9),
                    meaning_convergence: rng.gen_range(0.1..0.8),
                    interaction_count: rng.gen_range(1..10) as u64,
                    attention_allocated: rng.gen_range(10..200) as u64,
                });
            }
        }
    }
}

fn decide_status(attention: f64, rng: &mut impl Rng) -> ResonatorStatusKind {
    if attention > 0.75 {
        ResonatorStatusKind::Processing
    } else if rng.gen_bool(0.1) {
        ResonatorStatusKind::WaitingForApproval
    } else if attention < 0.2 && rng.gen_bool(0.05) {
        ResonatorStatusKind::Idle
    } else {
        ResonatorStatusKind::Idle
    }
}

fn clamp01(value: f64) -> f64 {
    value.max(0.0).min(1.0)
}

fn short_id(value: &str) -> String {
    if value.len() > 6 {
        value[value.len() - 6..].to_string()
    } else {
        value.to_string()
    }
}
