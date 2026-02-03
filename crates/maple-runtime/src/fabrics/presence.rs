//! Presence Fabric - manages gradient presence state
//!
//! Presence is NOT binary (online/offline). It's a gradient.

use crate::config::PresenceConfig as PresenceFabricConfig;
use crate::types::*;
use dashmap::DashMap;
use std::time::Instant;

/// Presence Fabric manages presence states for all Resonators
///
/// Key insight: Presence is multidimensional, not binary.
/// A Resonator can be:
/// - Discoverable but not responsive
/// - Responsive but not accepting new couplings
/// - Present but in silent mode
pub struct PresenceFabric {
    /// Presence states for all Resonators
    states: DashMap<ResonatorId, PresenceStateWithMetadata>,

    /// Configuration
    config: PresenceFabricConfig,
}

struct PresenceStateWithMetadata {
    state: PresenceState,
    last_update: Instant,
}

impl PresenceFabric {
    pub fn new(config: &PresenceFabricConfig) -> Self {
        Self {
            states: DashMap::new(),
            config: config.clone(),
        }
    }

    /// Initialize presence for a new Resonator
    pub async fn initialize_presence(
        &self,
        resonator: &ResonatorId,
        config: &PresenceConfig,
    ) -> Result<(), String> {
        let mut state = PresenceState::new();
        state.discoverability = config.initial_discoverability;
        state.responsiveness = config.initial_responsiveness;
        state.silent_mode = config.start_silent;

        let metadata = PresenceStateWithMetadata {
            state,
            last_update: Instant::now(),
        };

        self.states.insert(*resonator, metadata);

        tracing::debug!("Initialized presence for {}", resonator);
        Ok(())
    }

    /// Signal presence (MUST be low-cost and non-intrusive)
    ///
    /// ARCHITECTURAL RULE: Presence signaling must not be burdensome.
    pub async fn signal_presence(
        &self,
        resonator: ResonatorId,
        state: PresenceState,
    ) -> Result<(), PresenceError> {
        // Check rate limiting
        if let Some(existing) = self.states.get(&resonator) {
            let elapsed = existing.last_update.elapsed();
            let min_interval = std::time::Duration::from_millis(self.config.min_signal_interval_ms);

            if elapsed < min_interval {
                return Err(PresenceError::RateLimitExceeded);
            }
        }

        // Validate: presence signal must be low-cost
        // (In real implementation, would check if signal is too large, etc.)

        let metadata = PresenceStateWithMetadata {
            state,
            last_update: Instant::now(),
        };

        self.states.insert(resonator, metadata);

        Ok(())
    }

    /// Enable silent presence (existence without active signaling)
    ///
    /// This is important: a Resonator may be present without actively participating.
    /// Presence does NOT imply willingness to interact.
    pub async fn enable_silent_presence(&self, resonator: ResonatorId) {
        if let Some(mut entry) = self.states.get_mut(&resonator) {
            entry.state.silent_mode = true;
            entry.state.discoverability = 0.1; // Minimal discoverability
        }
    }

    /// Disable silent mode
    pub async fn disable_silent_presence(&self, resonator: ResonatorId) {
        if let Some(mut entry) = self.states.get_mut(&resonator) {
            entry.state.silent_mode = false;
            entry.state.discoverability = 0.5; // Return to default
        }
    }

    /// Get presence state
    pub fn get_presence(&self, resonator: &ResonatorId) -> Option<PresenceState> {
        self.states.get(resonator).map(|r| r.state.clone())
    }

    /// Check if Resonator is present
    pub fn is_present(&self, resonator: &ResonatorId) -> bool {
        self.states.contains_key(resonator)
    }

    /// Update presence gradient (called periodically to adjust presence based on behavior)
    pub async fn update_presence_gradient(
        &self,
        resonator: &ResonatorId,
        adjustment: PresenceAdjustment,
    ) {
        if let Some(mut entry) = self.states.get_mut(resonator) {
            match adjustment {
                PresenceAdjustment::IncreaseResponsiveness(delta) => {
                    entry.state.responsiveness = (entry.state.responsiveness + delta).min(1.0);
                }
                PresenceAdjustment::DecreaseResponsiveness(delta) => {
                    entry.state.responsiveness = (entry.state.responsiveness - delta).max(0.0);
                }
                PresenceAdjustment::IncreaseStability(delta) => {
                    entry.state.stability = (entry.state.stability + delta).min(1.0);
                }
                PresenceAdjustment::DecreaseStability(delta) => {
                    entry.state.stability = (entry.state.stability - delta).max(0.0);
                }
                PresenceAdjustment::SetCouplingReadiness(value) => {
                    entry.state.coupling_readiness = value.clamp(0.0, 1.0);
                }
            }
        }
    }

    /// Restore presence from continuity record
    pub async fn restore_presence(
        &self,
        resonator: &ResonatorId,
        state: &PresenceState,
    ) -> Result<(), String> {
        let metadata = PresenceStateWithMetadata {
            state: state.clone(),
            last_update: Instant::now(),
        };

        self.states.insert(*resonator, metadata);

        tracing::debug!("Restored presence for {}", resonator);
        Ok(())
    }

    /// Remove presence (for cleanup)
    pub fn remove_presence(&self, resonator: &ResonatorId) {
        self.states.remove(resonator);
    }

    /// Get all present Resonators
    pub fn get_all_present(&self) -> Vec<ResonatorId> {
        self.states.iter().map(|entry| *entry.key()).collect()
    }

    /// Count of present Resonators
    pub fn count(&self) -> usize {
        self.states.len()
    }
}

/// Adjustments to presence gradients
pub enum PresenceAdjustment {
    IncreaseResponsiveness(f64),
    DecreaseResponsiveness(f64),
    IncreaseStability(f64),
    DecreaseStability(f64),
    SetCouplingReadiness(f64),
}
