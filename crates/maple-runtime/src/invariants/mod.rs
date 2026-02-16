//! Invariant enforcement - The 9 Canonical WorldLine Invariants
//!
//! These invariants MUST hold in ALL conformant implementations.
//! Violation of ANY invariant constitutes non-conformance.

use crate::config::InvariantConfig;
use crate::types::*;

/// Enforces the WorldLine architecture's 9 canonical invariants.
pub struct InvariantGuard {
    invariants: Vec<ArchitecturalInvariant>,
    config: InvariantConfig,
}

/// The 9 canonical invariants from the WorldLine architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchitecturalInvariant {
    /// 1. Presence precedes meaning
    /// A Resonator must be present before it can form or receive meaning.
    PresencePrecedesMeaning,

    /// 2. Meaning precedes intent
    /// Intent cannot be formed without sufficient meaning.
    MeaningPrecedesIntent,

    /// 3. Intent precedes commitment
    /// Commitments cannot be created without stabilized intent.
    IntentPrecedesCommitment,

    /// 4. Commitment precedes consequence
    /// No consequence may occur without an explicit commitment.
    CommitmentPrecedesConsequence,

    /// 5. Coupling is bounded by attention
    /// Coupling strength cannot exceed available attention.
    CouplingBoundedByAttention,

    /// 6. Safety overrides optimization
    /// Safety constraints take precedence over performance/efficiency.
    SafetyOverridesOptimization,

    /// 7. Human agency cannot be bypassed
    /// Human Resonators must always be able to disengage.
    HumanAgencyCannotBeBypassed,

    /// 8. Failure must be explicit, never silent
    /// All failures must be surfaced, never hidden.
    FailureMustBeExplicit,

    /// 9. Implementation provenance and constitutional evolution.
    /// Operator evolution must be replay-verified and evidence-anchored.
    ImplementationProvenanceAndEvolution,
}

impl InvariantGuard {
    pub fn new(config: &InvariantConfig) -> Self {
        let invariants = vec![
            ArchitecturalInvariant::PresencePrecedesMeaning,
            ArchitecturalInvariant::MeaningPrecedesIntent,
            ArchitecturalInvariant::IntentPrecedesCommitment,
            ArchitecturalInvariant::CommitmentPrecedesConsequence,
            ArchitecturalInvariant::CouplingBoundedByAttention,
            ArchitecturalInvariant::SafetyOverridesOptimization,
            ArchitecturalInvariant::HumanAgencyCannotBeBypassed,
            ArchitecturalInvariant::FailureMustBeExplicit,
            ArchitecturalInvariant::ImplementationProvenanceAndEvolution,
        ];

        Self {
            invariants,
            config: config.clone(),
        }
    }

    /// Check all invariants before an operation
    pub fn check(
        &self,
        operation: &Operation,
        state: &SystemState,
    ) -> Result<(), InvariantViolation> {
        if !self.config.enabled {
            return Ok(());
        }

        for invariant in &self.invariants {
            self.check_invariant(*invariant, operation, state)?;
        }

        Ok(())
    }

    fn check_invariant(
        &self,
        invariant: ArchitecturalInvariant,
        operation: &Operation,
        state: &SystemState,
    ) -> Result<(), InvariantViolation> {
        match invariant {
            ArchitecturalInvariant::PresencePrecedesMeaning => {
                if let Operation::FormMeaning { resonator, .. } = operation {
                    if !state.is_present(resonator) {
                        tracing::error!(
                            "Invariant violation: Presence required for meaning formation"
                        );
                        return Err(InvariantViolation::PresenceRequired);
                    }
                }
            }

            ArchitecturalInvariant::MeaningPrecedesIntent => {
                if let Operation::StabilizeIntent { meaning, .. } = operation {
                    if meaning.confidence < 0.1 {
                        tracing::error!(
                            "Invariant violation: Insufficient meaning for intent stabilization"
                        );
                        return Err(InvariantViolation::InsufficientMeaning);
                    }
                }
            }

            ArchitecturalInvariant::IntentPrecedesCommitment => {
                if let Operation::CreateCommitment { intent, .. } = operation {
                    if !intent.is_stabilized() {
                        tracing::error!(
                            "Invariant violation: Intent not stabilized before commitment"
                        );
                        return Err(InvariantViolation::UnstabilizedIntent);
                    }
                }
            }

            ArchitecturalInvariant::CommitmentPrecedesConsequence => {
                if let Operation::ProduceConsequence { commitment_id, .. } = operation {
                    if !state.commitment_exists(commitment_id) {
                        tracing::error!("Invariant violation: No commitment for consequence");
                        return Err(InvariantViolation::NoCommitment);
                    }
                }
                if let Operation::ProduceExternalConsequence { commitment_ref, .. } = operation {
                    if !state.external_commitment_exists(commitment_ref) {
                        tracing::error!(
                            "Invariant violation: No external commitment for consequence"
                        );
                        return Err(InvariantViolation::NoCommitment);
                    }
                }
            }

            ArchitecturalInvariant::CouplingBoundedByAttention => {
                if let Operation::EstablishCoupling {
                    source,
                    attention_cost,
                    ..
                } = operation
                {
                    if state.available_attention(source) < *attention_cost {
                        tracing::error!("Invariant violation: Attention capacity exceeded");
                        return Err(InvariantViolation::AttentionExceeded);
                    }
                }
            }

            ArchitecturalInvariant::SafetyOverridesOptimization => {
                if operation.is_optimization() && state.safety_concern_active() {
                    tracing::error!("Invariant violation: Safety priority violated");
                    return Err(InvariantViolation::SafetyPriority);
                }
            }

            ArchitecturalInvariant::HumanAgencyCannotBeBypassed => {
                if let Operation::ForceAction { target, .. } = operation {
                    if state.is_human_resonator(target) {
                        tracing::error!("Invariant violation: Human agency cannot be bypassed");
                        return Err(InvariantViolation::HumanAgencyViolation);
                    }
                }
            }

            ArchitecturalInvariant::FailureMustBeExplicit => {
                // This is enforced by result types, not runtime checks
            }

            ArchitecturalInvariant::ImplementationProvenanceAndEvolution => {
                if let Operation::ApplyEvolutionUpgrade {
                    evidence_bundle_anchored,
                    replay_verified,
                } = operation
                {
                    if !state.upgrade_provenance_verified()
                        || !*evidence_bundle_anchored
                        || !*replay_verified
                    {
                        tracing::error!(
                            "Invariant violation: Evolution upgrade provenance requirements not satisfied"
                        );
                        return Err(InvariantViolation::ImplementationProvenanceViolation);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get list of enabled invariants
    pub fn enabled_invariants(&self) -> &[ArchitecturalInvariant] {
        &self.invariants
    }
}

/// Operations that can be checked against invariants
#[derive(Debug, Clone)]
pub enum Operation {
    FormMeaning {
        resonator: ResonatorId,
    },
    StabilizeIntent {
        meaning: MeaningContext,
    },
    CreateCommitment {
        intent: IntentContext,
    },
    ProduceConsequence {
        commitment_id: CommitmentId,
    },
    ProduceExternalConsequence {
        commitment_ref: String,
    },
    EstablishCoupling {
        source: ResonatorId,
        target: ResonatorId,
        attention_cost: u64,
    },
    ForceAction {
        target: ResonatorId,
    },
    Optimization,
    ApplyEvolutionUpgrade {
        evidence_bundle_anchored: bool,
        replay_verified: bool,
    },
}

impl Operation {
    pub fn is_optimization(&self) -> bool {
        matches!(self, Operation::Optimization)
    }
}

/// System state for invariant checking
#[derive(Debug, Clone)]
pub struct SystemState {
    present_resonators: std::collections::HashSet<ResonatorId>,
    human_resonators: std::collections::HashSet<ResonatorId>,
    commitments: std::collections::HashSet<CommitmentId>,
    external_commitments: std::collections::HashSet<String>,
    attention_budgets: std::collections::HashMap<ResonatorId, u64>,
    safety_concerns: bool,
    upgrade_provenance_verified: bool,
}

impl SystemState {
    pub fn new() -> Self {
        Self {
            present_resonators: std::collections::HashSet::new(),
            human_resonators: std::collections::HashSet::new(),
            commitments: std::collections::HashSet::new(),
            external_commitments: std::collections::HashSet::new(),
            attention_budgets: std::collections::HashMap::new(),
            safety_concerns: false,
            upgrade_provenance_verified: false,
        }
    }

    pub fn is_present(&self, resonator: &ResonatorId) -> bool {
        self.present_resonators.contains(resonator)
    }

    pub fn is_human_resonator(&self, resonator: &ResonatorId) -> bool {
        self.human_resonators.contains(resonator)
    }

    pub fn commitment_exists(&self, commitment_id: &CommitmentId) -> bool {
        self.commitments.contains(commitment_id)
    }

    pub fn external_commitment_exists(&self, commitment_ref: &str) -> bool {
        self.external_commitments.contains(commitment_ref)
    }

    pub fn available_attention(&self, resonator: &ResonatorId) -> u64 {
        self.attention_budgets.get(resonator).copied().unwrap_or(0)
    }

    pub fn safety_concern_active(&self) -> bool {
        self.safety_concerns
    }

    pub fn register_present(&mut self, resonator: ResonatorId) {
        self.present_resonators.insert(resonator);
    }

    pub fn register_human(&mut self, resonator: ResonatorId) {
        self.human_resonators.insert(resonator);
    }

    pub fn register_commitment(&mut self, commitment: CommitmentId) {
        self.commitments.insert(commitment);
    }

    pub fn register_external_commitment(&mut self, commitment_ref: String) {
        self.external_commitments.insert(commitment_ref);
    }

    pub fn set_attention(&mut self, resonator: ResonatorId, attention: u64) {
        self.attention_budgets.insert(resonator, attention);
    }

    pub fn set_safety_concern(&mut self, active: bool) {
        self.safety_concerns = active;
    }

    pub fn set_upgrade_provenance_verified(&mut self, verified: bool) {
        self.upgrade_provenance_verified = verified;
    }

    pub fn upgrade_provenance_verified(&self) -> bool {
        self.upgrade_provenance_verified
    }
}

impl Default for SystemState {
    fn default() -> Self {
        Self::new()
    }
}

/// Meaning context (placeholder)
#[derive(Debug, Clone)]
pub struct MeaningContext {
    pub confidence: f64,
}

/// Intent context (placeholder)
#[derive(Debug, Clone)]
pub struct IntentContext {
    stabilized: bool,
}

impl IntentContext {
    pub fn stabilized() -> Self {
        Self { stabilized: true }
    }

    pub fn unstabilized() -> Self {
        Self { stabilized: false }
    }

    pub fn from_confidence(confidence: f64, threshold: f64) -> Self {
        if confidence >= threshold {
            Self::stabilized()
        } else {
            Self::unstabilized()
        }
    }

    pub fn is_stabilized(&self) -> bool {
        self.stabilized
    }
}

// Re-export InvariantViolation from types module so it's accessible
pub use crate::types::InvariantViolation;
