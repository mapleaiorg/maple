//! Presence state types
//!
//! Presence is NOT binary (online/offline). It's a gradient.

use super::temporal::TemporalAnchor;
use serde::{Deserialize, Serialize};

/// Presence state - gradient representation of a Resonator's availability
///
/// Key insight: Presence is multidimensional, not a simple boolean.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceState {
    /// How discoverable is this Resonator? (0.0 to 1.0)
    ///
    /// 0.0 = Completely hidden
    /// 0.5 = Discoverable only through existing couplings
    /// 1.0 = Fully discoverable to all
    pub discoverability: f64,

    /// How responsive is this Resonator? (0.0 to 1.0)
    ///
    /// 0.0 = Not responding to interactions
    /// 0.5 = Slow/delayed responses
    /// 1.0 = Immediate responses
    pub responsiveness: f64,

    /// How stable has this Resonator been? (rolling average)
    ///
    /// 0.0 = Frequently disconnecting
    /// 1.0 = Consistently available
    pub stability: f64,

    /// How ready is this Resonator to form new couplings? (0.0 to 1.0)
    ///
    /// 0.0 = Not accepting new couplings
    /// 1.0 = Actively seeking new couplings
    pub coupling_readiness: f64,

    /// Last presence signal (temporal anchor)
    pub last_signal: TemporalAnchor,

    /// Silent mode (exists but not actively signaling)
    ///
    /// Important: A Resonator may be present without actively participating.
    /// Presence does NOT imply willingness to interact.
    pub silent_mode: bool,
}

impl PresenceState {
    /// Create a new presence state with default values
    pub fn new() -> Self {
        Self {
            discoverability: 0.5,
            responsiveness: 1.0,
            stability: 1.0,
            coupling_readiness: 0.7,
            last_signal: TemporalAnchor::now(),
            silent_mode: false,
        }
    }

    /// Is this Resonator effectively "online"?
    ///
    /// Note: This is a convenience method. Presence is a gradient,
    /// not a binary state.
    pub fn is_effectively_online(&self) -> bool {
        !self.silent_mode && self.responsiveness > 0.3 && self.stability > 0.3
    }

    /// Can this Resonator be discovered?
    pub fn is_discoverable(&self) -> bool {
        self.discoverability > 0.1
    }

    /// Is this Resonator accepting new couplings?
    pub fn is_accepting_couplings(&self) -> bool {
        self.coupling_readiness > 0.3 && !self.silent_mode
    }
}

impl Default for PresenceState {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for presence behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceConfig {
    /// Initial discoverability
    pub initial_discoverability: f64,

    /// Initial responsiveness
    pub initial_responsiveness: f64,

    /// Enable silent mode by default?
    pub start_silent: bool,

    /// Maximum signal frequency (to prevent spam)
    pub max_signal_frequency_ms: u64,
}

impl Default for PresenceConfig {
    fn default() -> Self {
        Self {
            initial_discoverability: 0.5,
            initial_responsiveness: 1.0,
            start_silent: false,
            max_signal_frequency_ms: 1000,
        }
    }
}
