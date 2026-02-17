use crate::config::SeedConfig;
use crate::error::GenesisError;
use maple_waf_context_graph::{
    ContextGraphManager, GovernanceTier, InMemoryContextGraphManager, IntentNode, NodeContent,
};
use maple_waf_evidence::{InvariantChecker, SimulatedInvariantChecker};
use maple_waf_resonance_monitor::{DissonanceThresholds, MonitorOrchestrator};
use maple_waf_swap_gate::{RollbackManager, WafSwapGate};
use serde::{Deserialize, Serialize};
use worldline_types::{EventId, TemporalAnchor, WorldlineId};

/// Phase of the genesis protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenesisPhase {
    /// Phase 1: Substrate attestation (verify hardware, dependencies).
    SubstrateAttestation,
    /// Phase 2: Axiomatic anchoring (lock invariants).
    AxiomaticAnchoring,
    /// Phase 3: Observer activation (start resonance monitor).
    ObserverActivation,
    /// Phase 4: Reflexive awakening (first self-observation).
    ReflexiveAwakening,
    /// Genesis complete — system is live.
    Complete,
}

impl std::fmt::Display for GenesisPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SubstrateAttestation => write!(f, "Phase 1: Substrate Attestation"),
            Self::AxiomaticAnchoring => write!(f, "Phase 2: Axiomatic Anchoring"),
            Self::ObserverActivation => write!(f, "Phase 3: Observer Activation"),
            Self::ReflexiveAwakening => write!(f, "Phase 4: Reflexive Awakening"),
            Self::Complete => write!(f, "Genesis Complete"),
        }
    }
}

/// Result of the genesis boot process.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenesisResult {
    pub worldline_id: WorldlineId,
    pub phase_reached: GenesisPhase,
    pub invariants_verified: usize,
    pub initial_resonance: f64,
    pub genesis_duration_ms: u64,
}

/// WorldLine instance created by genesis.
pub struct Worldline {
    pub id: WorldlineId,
    pub config: SeedConfig,
    pub context_graph: InMemoryContextGraphManager,
    pub monitor: MonitorOrchestrator,
    pub swap_gate: WafSwapGate,
    pub rollback: RollbackManager,
    pub phase: GenesisPhase,
}

/// Execute the 4-phase genesis boot protocol.
pub async fn genesis_boot(config: SeedConfig) -> Result<GenesisResult, GenesisError> {
    let start = std::time::Instant::now();
    let worldline_id = WorldlineId::ephemeral();

    // Phase 1: Substrate Attestation.
    attest_substrate()?;

    // Phase 2: Axiomatic Anchoring — verify all 14 invariants.
    let invariants_verified = anchor_axioms().await?;

    // Phase 3: Observer Activation — start resonance monitor.
    let initial_resonance = activate_observer(&config)?;

    // Phase 4: Reflexive Awakening — first self-observation.
    reflexive_awakening(&worldline_id, initial_resonance, &config).await?;

    let genesis_duration_ms = start.elapsed().as_millis() as u64;

    Ok(GenesisResult {
        worldline_id,
        phase_reached: GenesisPhase::Complete,
        invariants_verified,
        initial_resonance,
        genesis_duration_ms,
    })
}

/// Phase 1: Verify substrate (hardware, dependencies).
fn attest_substrate() -> Result<(), GenesisError> {
    // In a real implementation, this would check hardware capabilities,
    // available memory, disk space, and required toolchains.
    // Simulated: always passes.
    Ok(())
}

/// Phase 2: Lock all 14 invariants.
async fn anchor_axioms() -> Result<usize, GenesisError> {
    let checker = SimulatedInvariantChecker::all_pass();
    let results = checker.check_all().await;
    let all_hold = results.iter().all(|r| r.holds);
    if !all_hold {
        let violations: Vec<_> = results
            .iter()
            .filter(|r| !r.holds)
            .map(|r| r.id.clone())
            .collect();
        return Err(GenesisError::AxiomaticAnchoringFailed(format!(
            "invariant violations: {:?}",
            violations
        )));
    }
    Ok(results.len())
}

/// Phase 3: Start resonance monitor and compute initial resonance.
fn activate_observer(config: &SeedConfig) -> Result<f64, GenesisError> {
    // Simulated initial resonance — in production, this would be computed
    // from actual system metrics.
    let initial_resonance = if config.demo_mode { 0.7 } else { 0.9 };

    if initial_resonance < config.resonance_min {
        return Err(GenesisError::InsufficientResonance {
            current: initial_resonance,
            minimum: config.resonance_min,
        });
    }

    Ok(initial_resonance)
}

/// Phase 4: First self-observation — record genesis event in context graph.
async fn reflexive_awakening(
    worldline_id: &WorldlineId,
    resonance: f64,
    _config: &SeedConfig,
) -> Result<(), GenesisError> {
    let graph = InMemoryContextGraphManager::new();

    // Record the genesis intent.
    let intent = IntentNode::new(
        EventId::new(),
        format!("Genesis boot: initial resonance {:.3}", resonance),
        GovernanceTier::Tier5,
    );

    graph
        .append(
            worldline_id.clone(),
            NodeContent::Intent(intent),
            vec![],
            TemporalAnchor::now(0),
            GovernanceTier::Tier5,
        )
        .await
        .map_err(|e| GenesisError::ReflexiveAwakeningFailed(format!("{}", e)))?;

    Ok(())
}

/// Create a fully initialized Worldline from genesis.
pub async fn create_worldline(config: SeedConfig) -> Result<Worldline, GenesisError> {
    let result = genesis_boot(config.clone()).await?;

    let thresholds = DissonanceThresholds {
        resonance_min: config.resonance_min,
        ..Default::default()
    };

    Ok(Worldline {
        id: result.worldline_id,
        config: config.clone(),
        context_graph: InMemoryContextGraphManager::new(),
        monitor: MonitorOrchestrator::new(thresholds)
            .with_cooldown_ms(config.evolution_cooldown_ms),
        swap_gate: WafSwapGate::new(),
        rollback: RollbackManager::new(config.max_snapshots),
        phase: GenesisPhase::Complete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn genesis_boot_default() {
        let result = genesis_boot(SeedConfig::default()).await.unwrap();
        assert_eq!(result.phase_reached, GenesisPhase::Complete);
        assert_eq!(result.invariants_verified, 14);
        assert!(result.initial_resonance >= 0.6);
    }

    #[tokio::test]
    async fn genesis_boot_demo() {
        let result = genesis_boot(SeedConfig::demo()).await.unwrap();
        assert_eq!(result.phase_reached, GenesisPhase::Complete);
    }

    #[tokio::test]
    async fn genesis_boot_production() {
        let result = genesis_boot(SeedConfig::production()).await.unwrap();
        assert_eq!(result.phase_reached, GenesisPhase::Complete);
    }

    #[tokio::test]
    async fn create_worldline_succeeds() {
        let wl = create_worldline(SeedConfig::default()).await.unwrap();
        assert_eq!(wl.phase, GenesisPhase::Complete);
    }

    #[test]
    fn genesis_phase_display() {
        assert_eq!(
            format!("{}", GenesisPhase::SubstrateAttestation),
            "Phase 1: Substrate Attestation"
        );
        assert_eq!(format!("{}", GenesisPhase::Complete), "Genesis Complete");
    }

    #[tokio::test]
    async fn genesis_result_serde() {
        let result = genesis_boot(SeedConfig::default()).await.unwrap();
        let json = serde_json::to_string(&result).unwrap();
        let restored: GenesisResult = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.phase_reached, GenesisPhase::Complete);
    }

    #[test]
    fn insufficient_resonance() {
        // Config with very high resonance requirement.
        let config = SeedConfig {
            resonance_min: 0.99,
            ..SeedConfig::default()
        };
        let result = activate_observer(&config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn anchor_axioms_pass() {
        let count = anchor_axioms().await.unwrap();
        assert_eq!(count, 14);
    }
}
