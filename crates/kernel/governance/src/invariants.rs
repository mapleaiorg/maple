use crate::error::{InvariantViolation, ViolationSeverity};
use tracing::{debug, error, info};

/// System state snapshot for invariant checking.
///
/// Each invariant inspects the relevant portions of this state.
/// The state is constructed by AAS before invariant checks.
pub struct SystemState {
    /// Are worldline trajectories treated as primary identity units?
    pub worldline_primacy_enforced: bool,
    /// Is the memory engine using two-plane architecture?
    pub two_plane_memory_active: bool,
    /// Does every memory entry have provenance binding?
    pub all_memory_has_provenance: bool,
    /// Does the commitment boundary enforce gate passage?
    pub commitment_boundary_enforced: bool,
    /// Does every non-genesis event have causal parents?
    pub all_events_have_lineage: bool,
    /// Is coupling bounded by available attention and safety limits?
    pub resonance_bounded_coupling: bool,
    /// Are evolution operators pluggable only through gated commitment flow?
    pub pluggable_evolution_gated: bool,
    /// Do safety and agency constraints override optimization pressure?
    pub safety_overrides_optimization: bool,
    /// Are semantics independent of transport/substrate?
    pub substrate_independent: bool,
    /// Are operator upgrades replay-verified against causal history?
    pub upgrade_replay_verified: bool,
    /// Are operator upgrades anchored with evidence bundles?
    pub upgrade_evidence_anchored: bool,
}

impl SystemState {
    /// Create a state representing a healthy system (all invariants hold).
    pub fn healthy() -> Self {
        Self {
            worldline_primacy_enforced: true,
            two_plane_memory_active: true,
            all_memory_has_provenance: true,
            commitment_boundary_enforced: true,
            all_events_have_lineage: true,
            resonance_bounded_coupling: true,
            pluggable_evolution_gated: true,
            safety_overrides_optimization: true,
            substrate_independent: true,
            upgrade_replay_verified: true,
            upgrade_evidence_anchored: true,
        }
    }
}

/// Invariant trait — each constitutional invariant implements this.
///
/// Per I.GCP-2 (Constitutional Immutability): Invariants I.1-I.9 cannot
/// be weakened by any policy or operator.
pub trait Invariant: Send + Sync {
    /// Unique invariant identifier (e.g., "I.1").
    fn id(&self) -> &str;

    /// Human-readable name.
    fn name(&self) -> &str;

    /// Is this a constitutional invariant?
    /// Constitutional invariants MUST always be enforced.
    fn is_constitutional(&self) -> bool;

    /// Check the invariant against system state.
    fn check(&self, state: &SystemState) -> Result<(), InvariantViolation>;
}

// =========================================================================
// THE 9 CONSTITUTIONAL INVARIANTS
// =========================================================================

/// I.1: WorldLine Primacy.
///
/// "Everything important is a trajectory object; identity is continuity of
/// commitments and provenance."
pub struct NonCollapseInvariant;

impl Invariant for NonCollapseInvariant {
    fn id(&self) -> &str {
        "I.1"
    }
    fn name(&self) -> &str {
        "WorldLine Primacy"
    }
    fn is_constitutional(&self) -> bool {
        true
    }
    fn check(&self, state: &SystemState) -> Result<(), InvariantViolation> {
        if !state.worldline_primacy_enforced {
            return Err(InvariantViolation {
                invariant_id: self.id().into(),
                message:
                    "Session-centric identity detected — worldline continuity must remain primary"
                        .into(),
                severity: ViolationSeverity::Constitutional,
            });
        }
        Ok(())
    }
}

/// I.2: Intrinsic Typed Memory — Two-plane with provenance.
///
/// "Memory is not external storage. It is an intrinsic, typed, provenance-bound
/// component of every WorldLine."
pub struct IntrinsicMemoryInvariant;

impl Invariant for IntrinsicMemoryInvariant {
    fn id(&self) -> &str {
        "I.2"
    }
    fn name(&self) -> &str {
        "Intrinsic Typed Memory"
    }
    fn is_constitutional(&self) -> bool {
        true
    }
    fn check(&self, state: &SystemState) -> Result<(), InvariantViolation> {
        if !state.two_plane_memory_active {
            return Err(InvariantViolation {
                invariant_id: self.id().into(),
                message: "Two-plane memory architecture not active".into(),
                severity: ViolationSeverity::Constitutional,
            });
        }
        if !state.all_memory_has_provenance {
            return Err(InvariantViolation {
                invariant_id: self.id().into(),
                message: "Memory entries without provenance binding detected".into(),
                severity: ViolationSeverity::Constitutional,
            });
        }
        Ok(())
    }
}

/// I.3: Commitment Boundary — Only commitments cross boundary.
///
/// "The Commitment Boundary is the hard architectural boundary between cognition
/// and action. No data, message, or control flow may cross this boundary unless
/// it is explicitly typed as a Commitment and approved by governance."
pub struct CommitmentBoundaryInvariant;

impl Invariant for CommitmentBoundaryInvariant {
    fn id(&self) -> &str {
        "I.3"
    }
    fn name(&self) -> &str {
        "Commitment Boundary"
    }
    fn is_constitutional(&self) -> bool {
        true
    }
    fn check(&self, state: &SystemState) -> Result<(), InvariantViolation> {
        if !state.commitment_boundary_enforced {
            return Err(InvariantViolation {
                invariant_id: self.id().into(),
                message:
                    "Commitment boundary not enforced — unapproved actions may cross into execution"
                        .into(),
                severity: ViolationSeverity::Constitutional,
            });
        }
        Ok(())
    }
}

/// I.4: Causal Provenance — No event without lineage.
///
/// "Every event in the system MUST reference its causal parents. Genesis events
/// are the sole exception."
pub struct CausalProvenanceInvariant;

impl Invariant for CausalProvenanceInvariant {
    fn id(&self) -> &str {
        "I.4"
    }
    fn name(&self) -> &str {
        "Causal Provenance"
    }
    fn is_constitutional(&self) -> bool {
        true
    }
    fn check(&self, state: &SystemState) -> Result<(), InvariantViolation> {
        if !state.all_events_have_lineage {
            return Err(InvariantViolation {
                invariant_id: self.id().into(),
                message:
                    "Events without causal lineage detected — all events must reference parents"
                        .into(),
                severity: ViolationSeverity::Constitutional,
            });
        }
        Ok(())
    }
}

/// I.5: Resonance-Bounded Coupling.
///
/// "Coupling between worldlines must remain bounded by attention."
pub struct PreExecutionInvariant;

impl Invariant for PreExecutionInvariant {
    fn id(&self) -> &str {
        "I.5"
    }
    fn name(&self) -> &str {
        "Resonance-Bounded Coupling"
    }
    fn is_constitutional(&self) -> bool {
        true
    }
    fn check(&self, state: &SystemState) -> Result<(), InvariantViolation> {
        if !state.resonance_bounded_coupling {
            return Err(InvariantViolation {
                invariant_id: self.id().into(),
                message:
                    "Unbounded coupling detected — attention limits and agency bounds must hold"
                        .into(),
                severity: ViolationSeverity::Constitutional,
            });
        }
        Ok(())
    }
}

/// I.6: Pluggable Evolution Laws.
///
/// "Evolution operators are pluggable, but swaps must be commitment-gated."
pub struct ImmutabilityInvariant;

impl Invariant for ImmutabilityInvariant {
    fn id(&self) -> &str {
        "I.6"
    }
    fn name(&self) -> &str {
        "Pluggable Evolution Laws"
    }
    fn is_constitutional(&self) -> bool {
        true
    }
    fn check(&self, state: &SystemState) -> Result<(), InvariantViolation> {
        if !state.pluggable_evolution_gated {
            return Err(InvariantViolation {
                invariant_id: self.id().into(),
                message: "Evolution law changed outside gated commitment flow".into(),
                severity: ViolationSeverity::Constitutional,
            });
        }
        Ok(())
    }
}

/// I.7: Safety Overrides Optimization.
///
/// "Safety and agency constraints override optimization objectives."
pub struct BoundedAuthorityInvariant;

impl Invariant for BoundedAuthorityInvariant {
    fn id(&self) -> &str {
        "I.7"
    }
    fn name(&self) -> &str {
        "Safety Overrides Optimization"
    }
    fn is_constitutional(&self) -> bool {
        true
    }
    fn check(&self, state: &SystemState) -> Result<(), InvariantViolation> {
        if !state.safety_overrides_optimization {
            return Err(InvariantViolation {
                invariant_id: self.id().into(),
                message: "Optimization attempted to bypass safety/agency constraints".into(),
                severity: ViolationSeverity::Constitutional,
            });
        }
        Ok(())
    }
}

/// I.8: Substrate Independence — Semantics independent of transport.
///
/// "The semantic content of resonance messages must not depend on the
/// transport layer or substrate. Meaning is preserved across substrates."
pub struct SubstrateIndependenceInvariant;

impl Invariant for SubstrateIndependenceInvariant {
    fn id(&self) -> &str {
        "I.8"
    }
    fn name(&self) -> &str {
        "Substrate Independence"
    }
    fn is_constitutional(&self) -> bool {
        true
    }
    fn check(&self, state: &SystemState) -> Result<(), InvariantViolation> {
        if !state.substrate_independent {
            return Err(InvariantViolation {
                invariant_id: self.id().into(),
                message:
                    "Substrate-dependent semantics detected — meaning must be transport-independent"
                        .into(),
                severity: ViolationSeverity::Constitutional,
            });
        }
        Ok(())
    }
}

/// I.9: Implementation Provenance & Constitutional Evolution.
///
/// "Evolution operator upgrades may occur only via commitments that preserve
/// invariants, are replay-verified, and are evidence-anchored."
pub struct ImplementationProvenanceInvariant;

impl Invariant for ImplementationProvenanceInvariant {
    fn id(&self) -> &str {
        "I.9"
    }
    fn name(&self) -> &str {
        "Implementation Provenance & Constitutional Evolution"
    }
    fn is_constitutional(&self) -> bool {
        true
    }
    fn check(&self, state: &SystemState) -> Result<(), InvariantViolation> {
        if !state.upgrade_replay_verified {
            return Err(InvariantViolation {
                invariant_id: self.id().into(),
                message: "Operator upgrade was not replay-verified against causal history".into(),
                severity: ViolationSeverity::Constitutional,
            });
        }
        if !state.upgrade_evidence_anchored {
            return Err(InvariantViolation {
                invariant_id: self.id().into(),
                message: "Operator upgrade is missing an evidence bundle anchor".into(),
                severity: ViolationSeverity::Constitutional,
            });
        }
        Ok(())
    }
}

// Canonical names (latest WorldLine docs) mapped to existing public types.
pub type WorldlinePrimacyInvariant = NonCollapseInvariant;
pub type ResonanceBoundedCouplingInvariant = PreExecutionInvariant;
pub type PluggableEvolutionLawsInvariant = ImmutabilityInvariant;
pub type SafetyOverridesOptimizationInvariant = BoundedAuthorityInvariant;

// =========================================================================
// INVARIANT ENFORCER
// =========================================================================

/// Invariant Enforcer — enforces ALL constitutional invariants.
///
/// Per I.GCP-2 (Constitutional Immutability): These invariants cannot be
/// weakened by any policy or operator. The enforcer continuously verifies
/// the system state against all registered invariants.
pub struct InvariantEnforcer {
    invariants: Vec<Box<dyn Invariant>>,
}

impl InvariantEnforcer {
    /// Create an empty enforcer (no invariants loaded).
    pub fn new() -> Self {
        Self {
            invariants: Vec::new(),
        }
    }

    /// Create an enforcer with all 9 constitutional invariants.
    pub fn with_constitutional_invariants() -> Self {
        let mut enforcer = Self::new();

        enforcer.register(Box::new(NonCollapseInvariant));
        enforcer.register(Box::new(IntrinsicMemoryInvariant));
        enforcer.register(Box::new(CommitmentBoundaryInvariant));
        enforcer.register(Box::new(CausalProvenanceInvariant));
        enforcer.register(Box::new(PreExecutionInvariant));
        enforcer.register(Box::new(ImmutabilityInvariant));
        enforcer.register(Box::new(BoundedAuthorityInvariant));
        enforcer.register(Box::new(SubstrateIndependenceInvariant));
        enforcer.register(Box::new(ImplementationProvenanceInvariant));

        info!(
            count = enforcer.invariants.len(),
            "Constitutional invariants loaded"
        );

        enforcer
    }

    /// Register an invariant.
    pub fn register(&mut self, invariant: Box<dyn Invariant>) {
        debug!(
            id = invariant.id(),
            name = invariant.name(),
            constitutional = invariant.is_constitutional(),
            "Invariant registered"
        );
        self.invariants.push(invariant);
    }

    /// Check all invariants against the current system state.
    ///
    /// Returns a list of violations (empty if all pass).
    /// Constitutional violations are always returned; they cannot be suppressed.
    pub fn check_all(&self, state: &SystemState) -> Vec<InvariantViolation> {
        let mut violations = Vec::new();

        for invariant in &self.invariants {
            match invariant.check(state) {
                Ok(()) => {
                    debug!(id = invariant.id(), "Invariant holds");
                }
                Err(violation) => {
                    error!(
                        id = invariant.id(),
                        message = %violation.message,
                        severity = ?violation.severity,
                        "INVARIANT VIOLATION"
                    );
                    violations.push(violation);
                }
            }
        }

        violations
    }

    /// Check all invariants and return error if any constitutional violations found.
    pub fn enforce(&self, state: &SystemState) -> Result<(), Vec<InvariantViolation>> {
        let violations = self.check_all(state);

        let constitutional: Vec<InvariantViolation> = violations
            .into_iter()
            .filter(|v| v.severity == ViolationSeverity::Constitutional)
            .collect();

        if constitutional.is_empty() {
            Ok(())
        } else {
            Err(constitutional)
        }
    }

    /// Number of registered invariants.
    pub fn count(&self) -> usize {
        self.invariants.len()
    }

    /// Number of constitutional invariants.
    pub fn constitutional_count(&self) -> usize {
        self.invariants
            .iter()
            .filter(|i| i.is_constitutional())
            .count()
    }
}

impl Default for InvariantEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_system_passes_all_invariants() {
        let enforcer = InvariantEnforcer::with_constitutional_invariants();
        let state = SystemState::healthy();
        let violations = enforcer.check_all(&state);
        assert!(violations.is_empty());
    }

    #[test]
    fn enforcer_has_9_constitutional_invariants() {
        let enforcer = InvariantEnforcer::with_constitutional_invariants();
        assert_eq!(enforcer.count(), 9);
        assert_eq!(enforcer.constitutional_count(), 9);
    }

    #[test]
    fn i1_worldline_primacy_violation() {
        let enforcer = InvariantEnforcer::with_constitutional_invariants();
        let mut state = SystemState::healthy();
        state.worldline_primacy_enforced = false;

        let violations = enforcer.check_all(&state);
        assert!(!violations.is_empty());
        assert!(violations.iter().any(|v| v.invariant_id == "I.1"));
    }

    #[test]
    fn i2_memory_violation() {
        let enforcer = InvariantEnforcer::with_constitutional_invariants();

        // Test missing two-plane
        let mut state = SystemState::healthy();
        state.two_plane_memory_active = false;
        let violations = enforcer.check_all(&state);
        assert!(violations.iter().any(|v| v.invariant_id == "I.2"));

        // Test missing provenance
        let mut state = SystemState::healthy();
        state.all_memory_has_provenance = false;
        let violations = enforcer.check_all(&state);
        assert!(violations.iter().any(|v| v.invariant_id == "I.2"));
    }

    #[test]
    fn i3_commitment_boundary_violation() {
        let enforcer = InvariantEnforcer::with_constitutional_invariants();
        let mut state = SystemState::healthy();
        state.commitment_boundary_enforced = false;

        let violations = enforcer.check_all(&state);
        assert!(violations.iter().any(|v| v.invariant_id == "I.3"));
    }

    #[test]
    fn i4_causal_provenance_violation() {
        let enforcer = InvariantEnforcer::with_constitutional_invariants();
        let mut state = SystemState::healthy();
        state.all_events_have_lineage = false;

        let violations = enforcer.check_all(&state);
        assert!(violations.iter().any(|v| v.invariant_id == "I.4"));
    }

    #[test]
    fn i5_resonance_bounded_coupling_violation() {
        let enforcer = InvariantEnforcer::with_constitutional_invariants();
        let mut state = SystemState::healthy();
        state.resonance_bounded_coupling = false;

        let violations = enforcer.check_all(&state);
        assert!(violations.iter().any(|v| v.invariant_id == "I.5"));
    }

    #[test]
    fn i6_pluggable_evolution_violation() {
        let enforcer = InvariantEnforcer::with_constitutional_invariants();
        let mut state = SystemState::healthy();
        state.pluggable_evolution_gated = false;

        let violations = enforcer.check_all(&state);
        assert!(violations.iter().any(|v| v.invariant_id == "I.6"));
    }

    #[test]
    fn i7_safety_overrides_optimization_violation() {
        let enforcer = InvariantEnforcer::with_constitutional_invariants();
        let mut state = SystemState::healthy();
        state.safety_overrides_optimization = false;

        let violations = enforcer.check_all(&state);
        assert!(violations.iter().any(|v| v.invariant_id == "I.7"));
    }

    #[test]
    fn i8_substrate_independence_violation() {
        let enforcer = InvariantEnforcer::with_constitutional_invariants();
        let mut state = SystemState::healthy();
        state.substrate_independent = false;

        let violations = enforcer.check_all(&state);
        assert!(violations.iter().any(|v| v.invariant_id == "I.8"));
    }

    #[test]
    fn i9_implementation_provenance_violation() {
        let enforcer = InvariantEnforcer::with_constitutional_invariants();
        let mut state = SystemState::healthy();
        state.upgrade_replay_verified = false;

        let violations = enforcer.check_all(&state);
        assert!(violations.iter().any(|v| v.invariant_id == "I.9"));
    }

    #[test]
    fn multiple_violations_detected() {
        let enforcer = InvariantEnforcer::with_constitutional_invariants();
        let mut state = SystemState::healthy();
        state.worldline_primacy_enforced = false;
        state.commitment_boundary_enforced = false;
        state.safety_overrides_optimization = false;

        let violations = enforcer.check_all(&state);
        assert_eq!(violations.len(), 3);
    }

    #[test]
    fn enforce_returns_ok_when_healthy() {
        let enforcer = InvariantEnforcer::with_constitutional_invariants();
        let state = SystemState::healthy();
        assert!(enforcer.enforce(&state).is_ok());
    }

    #[test]
    fn enforce_returns_err_with_constitutional_violations() {
        let enforcer = InvariantEnforcer::with_constitutional_invariants();
        let mut state = SystemState::healthy();
        state.all_events_have_lineage = false;
        state.resonance_bounded_coupling = false;

        let result = enforcer.enforce(&state);
        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert_eq!(violations.len(), 2);
        assert!(violations
            .iter()
            .all(|v| v.severity == ViolationSeverity::Constitutional));
    }

    #[test]
    fn all_constitutional_violations_are_constitutional_severity() {
        let enforcer = InvariantEnforcer::with_constitutional_invariants();
        // Break everything
        let state = SystemState {
            worldline_primacy_enforced: false,
            two_plane_memory_active: false,
            all_memory_has_provenance: false,
            commitment_boundary_enforced: false,
            all_events_have_lineage: false,
            resonance_bounded_coupling: false,
            pluggable_evolution_gated: false,
            safety_overrides_optimization: false,
            substrate_independent: false,
            upgrade_replay_verified: false,
            upgrade_evidence_anchored: false,
        };

        let violations = enforcer.check_all(&state);
        // I.2 and I.9 can each fail on more than one condition.
        assert!(violations.len() >= 9);
        assert!(violations
            .iter()
            .all(|v| v.severity == ViolationSeverity::Constitutional));
    }
}
