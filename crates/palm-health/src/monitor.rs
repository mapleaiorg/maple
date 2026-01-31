//! Health monitor for continuous fleet monitoring.
//!
//! The HealthMonitor runs continuous health checks on all registered
//! instances and triggers recovery actions when needed.

use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;
use dashmap::DashMap;
use palm_types::InstanceId;
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, instrument, warn};

use crate::assessment::{FleetHealthSummary, HealthAssessment, OverallHealth};
use crate::config::{HealthConfig, HealthThresholds};
use crate::error::{HealthError, HealthResult};
use crate::probes::{Probe, ProbeResult, ProbeSet, ProbeType};
use crate::resilience::{RecoveryAction, RecoveryOutcome, ResilienceController};

/// Events emitted by the health monitor.
#[derive(Debug, Clone)]
pub enum HealthEvent {
    /// Instance was registered for monitoring.
    InstanceRegistered(InstanceId),

    /// Instance was unregistered.
    InstanceUnregistered(InstanceId),

    /// Probe completed.
    ProbeCompleted {
        instance_id: InstanceId,
        probe_type: ProbeType,
        result: ProbeResult,
    },

    /// Health assessment updated.
    AssessmentUpdated {
        instance_id: InstanceId,
        assessment: Box<HealthAssessment>,
    },

    /// Health status changed.
    StatusChanged {
        instance_id: InstanceId,
        old_status: OverallHealth,
        new_status: OverallHealth,
    },

    /// Recovery action triggered.
    RecoveryTriggered {
        instance_id: InstanceId,
        action: RecoveryAction,
    },

    /// Recovery completed.
    RecoveryCompleted {
        instance_id: InstanceId,
        outcome: RecoveryOutcome,
    },

    /// Instance isolated.
    InstanceIsolated {
        instance_id: InstanceId,
        reason: String,
    },

    /// Instance de-isolated.
    InstanceDeIsolated(InstanceId),
}

/// Health monitor for continuous fleet health monitoring.
pub struct HealthMonitor {
    /// Configuration.
    config: HealthConfig,

    /// Probe set for health checks.
    probes: Arc<RwLock<ProbeSet>>,

    /// Current health assessments.
    assessments: DashMap<InstanceId, HealthAssessment>,

    /// Registered instances.
    instances: DashMap<InstanceId, InstanceMonitorState>,

    /// Resilience controller.
    resilience: Arc<ResilienceController>,

    /// Event broadcaster.
    event_tx: broadcast::Sender<HealthEvent>,

    /// Monitor handles for cleanup.
    monitor_handles: DashMap<InstanceId, JoinHandle<()>>,

    /// Isolated instances.
    isolated: DashMap<InstanceId, String>,
}

/// State for a monitored instance.
#[derive(Debug, Clone)]
struct InstanceMonitorState {
    /// Whether monitoring is active.
    active: bool,

    /// Whether the instance is paused.
    paused: bool,

    /// Custom probes for this instance.
    custom_probes: Vec<String>,
}

impl HealthMonitor {
    /// Create a new health monitor.
    pub fn new(config: HealthConfig, resilience: Arc<ResilienceController>) -> Self {
        let (event_tx, _) = broadcast::channel(1024);

        Self {
            config,
            probes: Arc::new(RwLock::new(ProbeSet::default_set())),
            assessments: DashMap::new(),
            instances: DashMap::new(),
            resilience,
            event_tx,
            monitor_handles: DashMap::new(),
            isolated: DashMap::new(),
        }
    }

    /// Subscribe to health events.
    pub fn subscribe(&self) -> broadcast::Receiver<HealthEvent> {
        self.event_tx.subscribe()
    }

    /// Register an instance for monitoring.
    #[instrument(skip(self))]
    pub fn register_instance(&self, instance_id: InstanceId) -> HealthResult<()> {
        if self.instances.contains_key(&instance_id) {
            return Err(HealthError::MonitorAlreadyRunning(instance_id));
        }

        info!(instance_id = %instance_id, "Registering instance for health monitoring");

        // Create initial assessment
        self.assessments
            .insert(instance_id.clone(), HealthAssessment::new(instance_id.clone()));

        // Create monitor state
        self.instances.insert(
            instance_id.clone(),
            InstanceMonitorState {
                active: true,
                paused: false,
                custom_probes: Vec::new(),
            },
        );

        // Emit event
        let _ = self
            .event_tx
            .send(HealthEvent::InstanceRegistered(instance_id));

        Ok(())
    }

    /// Unregister an instance from monitoring.
    #[instrument(skip(self))]
    pub fn unregister_instance(&self, instance_id: &InstanceId) -> HealthResult<()> {
        if !self.instances.contains_key(instance_id) {
            return Err(HealthError::MonitorNotFound(instance_id.clone()));
        }

        info!(instance_id = %instance_id, "Unregistering instance from health monitoring");

        // Stop any running monitor
        if let Some((_, handle)) = self.monitor_handles.remove(instance_id) {
            handle.abort();
        }

        // Remove state
        self.instances.remove(instance_id);
        self.assessments.remove(instance_id);
        self.isolated.remove(instance_id);
        self.resilience.remove_instance(instance_id);

        // Emit event
        let _ = self
            .event_tx
            .send(HealthEvent::InstanceUnregistered(instance_id.clone()));

        Ok(())
    }

    /// Pause monitoring for an instance.
    pub fn pause_instance(&self, instance_id: &InstanceId) -> HealthResult<()> {
        let mut state = self
            .instances
            .get_mut(instance_id)
            .ok_or_else(|| HealthError::MonitorNotFound(instance_id.clone()))?;

        state.paused = true;
        debug!(instance_id = %instance_id, "Paused health monitoring");

        Ok(())
    }

    /// Resume monitoring for an instance.
    pub fn resume_instance(&self, instance_id: &InstanceId) -> HealthResult<()> {
        let mut state = self
            .instances
            .get_mut(instance_id)
            .ok_or_else(|| HealthError::MonitorNotFound(instance_id.clone()))?;

        state.paused = false;
        debug!(instance_id = %instance_id, "Resumed health monitoring");

        Ok(())
    }

    /// Check if an instance is being monitored.
    pub fn is_monitoring(&self, instance_id: &InstanceId) -> bool {
        self.instances
            .get(instance_id)
            .map(|s| s.active && !s.paused)
            .unwrap_or(false)
    }

    /// Get current health assessment for an instance.
    pub fn get_assessment(&self, instance_id: &InstanceId) -> Option<HealthAssessment> {
        self.assessments.get(instance_id).map(|a| a.clone())
    }

    /// Get all current assessments.
    pub fn get_all_assessments(&self) -> Vec<HealthAssessment> {
        self.assessments.iter().map(|r| r.value().clone()).collect()
    }

    /// Get fleet health summary.
    pub fn get_fleet_summary(&self) -> FleetHealthSummary {
        let assessments = self.get_all_assessments();
        FleetHealthSummary::from_assessments(&assessments)
    }

    /// Check if an instance is isolated.
    pub fn is_isolated(&self, instance_id: &InstanceId) -> bool {
        self.isolated.contains_key(instance_id)
    }

    /// Isolate an instance.
    pub fn isolate_instance(&self, instance_id: &InstanceId, reason: String) -> HealthResult<()> {
        if !self.instances.contains_key(instance_id) {
            return Err(HealthError::InstanceNotFound(instance_id.clone()));
        }

        info!(instance_id = %instance_id, reason = %reason, "Isolating instance");

        self.isolated.insert(instance_id.clone(), reason.clone());

        // Update assessment
        if let Some(mut assessment) = self.assessments.get_mut(instance_id) {
            assessment.is_isolated = true;
            assessment.add_note(format!("Isolated: {}", reason));
        }

        let _ = self.event_tx.send(HealthEvent::InstanceIsolated {
            instance_id: instance_id.clone(),
            reason,
        });

        Ok(())
    }

    /// De-isolate an instance.
    pub fn deisolate_instance(&self, instance_id: &InstanceId) -> HealthResult<()> {
        if !self.isolated.contains_key(instance_id) {
            return Ok(()); // Not isolated, nothing to do
        }

        info!(instance_id = %instance_id, "De-isolating instance");

        self.isolated.remove(instance_id);

        // Update assessment
        if let Some(mut assessment) = self.assessments.get_mut(instance_id) {
            assessment.is_isolated = false;
            assessment.add_note("De-isolated");
        }

        let _ = self
            .event_tx
            .send(HealthEvent::InstanceDeIsolated(instance_id.clone()));

        Ok(())
    }

    /// Run probes for a single instance.
    #[instrument(skip(self))]
    pub async fn probe_instance(&self, instance_id: &InstanceId) -> HealthResult<HealthAssessment> {
        // Check if instance is registered
        let state = self
            .instances
            .get(instance_id)
            .ok_or_else(|| HealthError::InstanceNotFound(instance_id.clone()))?;

        if state.paused {
            debug!(instance_id = %instance_id, "Skipping probe for paused instance");
            return self
                .get_assessment(instance_id)
                .ok_or_else(|| HealthError::InstanceNotFound(instance_id.clone()));
        }

        drop(state); // Release lock before async work

        debug!(instance_id = %instance_id, "Running probes");

        let probes = self.probes.read().await;
        let results = probes.execute_all(instance_id.clone()).await;

        // Get or create assessment
        let mut assessment = self
            .assessments
            .get(instance_id)
            .map(|a| a.clone())
            .unwrap_or_else(|| HealthAssessment::new(instance_id.clone()));

        let old_status = assessment.overall;

        // Process probe results
        for result in results {
            match result {
                Ok(probe_result) => {
                    // Emit probe event
                    let _ = self.event_tx.send(HealthEvent::ProbeCompleted {
                        instance_id: instance_id.clone(),
                        probe_type: probe_result.probe_type,
                        result: probe_result.clone(),
                    });

                    // Update dimension assessment
                    if probe_result.success {
                        if let Some(value) = probe_result.value {
                            match probe_result.probe_type {
                                ProbeType::Presence => {
                                    assessment.presence.update(
                                        value,
                                        self.config.thresholds.presence_healthy,
                                        self.config.thresholds.presence_degraded,
                                    );
                                }
                                ProbeType::Coupling => {
                                    assessment.coupling.update(
                                        value,
                                        self.config.thresholds.coupling_healthy,
                                        self.config.thresholds.coupling_degraded,
                                    );
                                }
                                ProbeType::Attention => {
                                    assessment.attention.update(
                                        value,
                                        self.config.thresholds.attention_healthy,
                                        self.config.thresholds.attention_degraded,
                                    );
                                }
                                ProbeType::Custom => {
                                    // Custom probes don't update standard dimensions
                                }
                            }
                        }
                    } else {
                        // Record failure
                        match probe_result.probe_type {
                            ProbeType::Presence => assessment.presence.record_failure(),
                            ProbeType::Coupling => assessment.coupling.record_failure(),
                            ProbeType::Attention => assessment.attention.record_failure(),
                            ProbeType::Custom => {}
                        }
                    }
                }
                Err(e) => {
                    warn!(instance_id = %instance_id, error = %e, "Probe error");
                }
            }
        }

        // Recompute overall health
        assessment.recompute_overall(&self.config.thresholds);

        // Check for status change
        if old_status != assessment.overall && old_status != OverallHealth::Unknown {
            info!(
                instance_id = %instance_id,
                old_status = %old_status,
                new_status = %assessment.overall,
                "Health status changed"
            );

            let _ = self.event_tx.send(HealthEvent::StatusChanged {
                instance_id: instance_id.clone(),
                old_status,
                new_status: assessment.overall,
            });
        }

        // Update stored assessment
        self.assessments
            .insert(instance_id.clone(), assessment.clone());

        // Emit assessment update
        let _ = self.event_tx.send(HealthEvent::AssessmentUpdated {
            instance_id: instance_id.clone(),
            assessment: Box::new(assessment.clone()),
        });

        Ok(assessment)
    }

    /// Run probes and trigger recovery if needed.
    #[instrument(skip(self))]
    pub async fn check_and_recover(&self, instance_id: &InstanceId) -> HealthResult<()> {
        let assessment = self.probe_instance(instance_id).await?;

        // Check if recovery is needed
        if assessment.needs_recovery() && !assessment.is_isolated {
            let action = self.resilience.evaluate_recovery(&assessment);

            if !matches!(action, RecoveryAction::None) {
                info!(
                    instance_id = %instance_id,
                    action = %action,
                    "Triggering recovery action"
                );

                let _ = self.event_tx.send(HealthEvent::RecoveryTriggered {
                    instance_id: instance_id.clone(),
                    action: action.clone(),
                });

                // Handle isolation specially
                if let RecoveryAction::Isolate { reason } = &action {
                    self.isolate_instance(instance_id, reason.clone())?;
                }

                // Execute recovery
                let outcome = self.resilience.execute_recovery(instance_id, action).await?;

                let _ = self.event_tx.send(HealthEvent::RecoveryCompleted {
                    instance_id: instance_id.clone(),
                    outcome,
                });
            }
        }

        Ok(())
    }

    /// Run continuous monitoring for all registered instances.
    ///
    /// This spawns background tasks for each instance.
    pub async fn start_continuous_monitoring(&self) {
        info!("Starting continuous health monitoring");

        // Get all registered instances
        let instances: Vec<InstanceId> = self.instances.iter().map(|r| r.key().clone()).collect();

        for instance_id in instances {
            self.start_instance_monitor(instance_id);
        }
    }

    /// Start monitoring for a specific instance.
    fn start_instance_monitor(&self, instance_id: InstanceId) {
        let instance_id_clone = instance_id.clone();
        let presence_interval = self.config.probes.presence_interval;

        // Note: In a real implementation, this would use Arc<Self> and spawn
        // actual tasks. For this structure, we're showing the pattern.
        debug!(
            instance_id = %instance_id_clone,
            interval_ms = presence_interval.as_millis(),
            "Would start continuous monitoring"
        );
    }

    /// Stop all monitoring.
    pub fn stop_all_monitoring(&self) {
        info!("Stopping all health monitoring");

        for item in self.monitor_handles.iter() {
            item.value().abort();
        }
        self.monitor_handles.clear();
    }

    /// Get list of registered instance IDs.
    pub fn registered_instances(&self) -> Vec<InstanceId> {
        self.instances.iter().map(|r| r.key().clone()).collect()
    }

    /// Get list of isolated instance IDs.
    pub fn isolated_instances(&self) -> Vec<(InstanceId, String)> {
        self.isolated
            .iter()
            .map(|r| (r.key().clone(), r.value().clone()))
            .collect()
    }
}

impl Drop for HealthMonitor {
    fn drop(&mut self) {
        // Abort all monitor tasks
        for item in self.monitor_handles.iter() {
            item.value().abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resilience::NoOpRecoveryExecutor;
    use palm_types::PlatformProfile;

    fn test_monitor() -> HealthMonitor {
        let config = HealthConfig::default();
        let executor = Arc::new(NoOpRecoveryExecutor);
        let resilience = Arc::new(ResilienceController::new(
            config.resilience.clone(),
            PlatformProfile::Development,
            executor,
        ));
        HealthMonitor::new(config, resilience)
    }

    #[test]
    fn test_register_unregister() {
        let monitor = test_monitor();
        let instance_id = InstanceId::generate();

        monitor.register_instance(instance_id.clone()).unwrap();
        assert!(monitor.is_monitoring(&instance_id));

        monitor.unregister_instance(&instance_id).unwrap();
        assert!(!monitor.is_monitoring(&instance_id));
    }

    #[test]
    fn test_pause_resume() {
        let monitor = test_monitor();
        let instance_id = InstanceId::generate();

        monitor.register_instance(instance_id.clone()).unwrap();
        assert!(monitor.is_monitoring(&instance_id));

        monitor.pause_instance(&instance_id).unwrap();
        assert!(!monitor.is_monitoring(&instance_id));

        monitor.resume_instance(&instance_id).unwrap();
        assert!(monitor.is_monitoring(&instance_id));
    }

    #[test]
    fn test_isolation() {
        let monitor = test_monitor();
        let instance_id = InstanceId::generate();

        monitor.register_instance(instance_id.clone()).unwrap();

        monitor
            .isolate_instance(&instance_id, "Test isolation".to_string())
            .unwrap();
        assert!(monitor.is_isolated(&instance_id));

        monitor.deisolate_instance(&instance_id).unwrap();
        assert!(!monitor.is_isolated(&instance_id));
    }

    #[tokio::test]
    async fn test_probe_instance() {
        let monitor = test_monitor();
        let instance_id = InstanceId::generate();

        monitor.register_instance(instance_id.clone()).unwrap();

        let assessment = monitor.probe_instance(&instance_id).await.unwrap();
        assert_eq!(assessment.instance_id, instance_id);
    }
}
