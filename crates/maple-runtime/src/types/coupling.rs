//! Coupling relationship types

use super::ids::{CouplingId, ResonatorId};
use super::temporal::TemporalAnchor;
use serde::{Deserialize, Serialize};

/// A coupling relationship between Resonators
///
/// Coupling describes the strength and character of interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coupling {
    pub id: CouplingId,
    pub source: ResonatorId,
    pub target: ResonatorId,

    // ═══════════════════════════════════════════════════════════════════
    // COUPLING DIMENSIONS (from Resonance Architecture)
    // ═══════════════════════════════════════════════════════════════════
    /// Intensity: How much influence is allowed (0.0 to 1.0)
    pub strength: f64,

    /// Persistence: How long the coupling lasts
    pub persistence: CouplingPersistence,

    /// Scope: Which aspects of state are affected
    pub scope: CouplingScope,

    /// Symmetry: Whether influence is mutual
    pub symmetry: SymmetryType,

    // ═══════════════════════════════════════════════════════════════════
    // ATTENTION BINDING
    // ═══════════════════════════════════════════════════════════════════
    /// Attention allocated to this coupling
    pub attention_allocated: u64,

    // ═══════════════════════════════════════════════════════════════════
    // COUPLING HEALTH
    // ═══════════════════════════════════════════════════════════════════
    /// How well does meaning converge between these Resonators?
    pub meaning_convergence: f64,

    /// Interaction count
    pub interaction_count: u64,

    /// Created at (temporal anchor)
    pub created_at: TemporalAnchor,

    /// Last resonance through this coupling
    pub last_resonance: TemporalAnchor,
}

impl Coupling {
    /// Is this coupling healthy?
    pub fn is_healthy(&self) -> bool {
        self.meaning_convergence > 0.3 && self.strength > 0.1
    }

    /// Should this coupling be strengthened?
    pub fn should_strengthen(&self) -> bool {
        self.meaning_convergence > 0.7 && self.strength < 0.8
    }

    /// Should this coupling be weakened?
    pub fn should_weaken(&self) -> bool {
        self.meaning_convergence < 0.2 && self.strength > 0.2
    }
}

/// How long does a coupling last?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CouplingPersistence {
    /// Coupling lasts for a single interaction
    Transient,

    /// Coupling lasts for a session
    Session,

    /// Coupling persists indefinitely
    Persistent,

    /// Coupling lasts for a specific duration (in seconds)
    Timed(u64),
}

/// Which aspects of state are affected by coupling?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CouplingScope {
    /// Full coupling: state, intent, and commitment visibility
    Full,

    /// Only state is shared
    StateOnly,

    /// Only intent is shared
    IntentOnly,

    /// Only observation, no influence
    ObservationalOnly,
}

/// Is influence mutual or one-way?
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymmetryType {
    /// Both Resonators influence each other equally
    Symmetric,

    /// Asymmetric influence (primary has more influence)
    Asymmetric { primary: ResonatorId },
}

/// Parameters for establishing a new coupling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingParams {
    pub source: ResonatorId,
    pub target: ResonatorId,

    /// Initial coupling strength (MUST be <= 0.3 to prevent abrupt escalation)
    pub initial_strength: f64,

    /// Initial attention cost
    pub initial_attention_cost: u64,

    /// Persistence
    pub persistence: CouplingPersistence,

    /// Scope
    pub scope: CouplingScope,

    /// Symmetry
    pub symmetry: SymmetryType,
}

impl CouplingParams {
    /// Validate parameters before coupling
    pub fn validate(&self) -> Result<(), CouplingValidationError> {
        // ARCHITECTURAL RULE: Initial strength cannot be too aggressive
        if self.initial_strength > 0.3 {
            return Err(CouplingValidationError::InitialStrengthTooHigh);
        }

        if self.initial_strength < 0.0 || self.initial_strength > 1.0 {
            return Err(CouplingValidationError::InvalidStrength);
        }

        if self.initial_attention_cost == 0 {
            return Err(CouplingValidationError::ZeroAttentionCost);
        }

        Ok(())
    }
}

/// Affinity specification for preferred coupling patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingAffinitySpec {
    /// Preferred coupling strength
    pub preferred_strength: f64,

    /// Preferred persistence
    pub preferred_persistence: CouplingPersistence,

    /// Preferred scope
    pub preferred_scope: CouplingScope,

    /// Maximum number of concurrent couplings
    pub max_concurrent_couplings: Option<usize>,
}

impl Default for CouplingAffinitySpec {
    fn default() -> Self {
        Self {
            preferred_strength: 0.3,
            preferred_persistence: CouplingPersistence::Session,
            preferred_scope: CouplingScope::Full,
            max_concurrent_couplings: Some(100),
        }
    }
}

/// Coupling validation errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum CouplingValidationError {
    #[error("Initial coupling strength too high (max 0.3)")]
    InitialStrengthTooHigh,

    #[error("Invalid strength value (must be 0.0-1.0)")]
    InvalidStrength,

    #[error("Attention cost cannot be zero")]
    ZeroAttentionCost,
}

/// Configuration for coupling behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingConfig {
    /// Maximum initial strength
    pub max_initial_strength: f64,

    /// Maximum strengthening per interaction
    pub max_strengthening_rate: f64,

    /// Enable automatic coupling adjustment?
    pub enable_auto_adjustment: bool,

    /// Minimum meaning convergence to maintain coupling
    pub min_meaning_convergence: f64,
}

impl Default for CouplingConfig {
    fn default() -> Self {
        Self {
            max_initial_strength: 0.3,
            max_strengthening_rate: 0.1,
            enable_auto_adjustment: true,
            min_meaning_convergence: 0.1,
        }
    }
}
