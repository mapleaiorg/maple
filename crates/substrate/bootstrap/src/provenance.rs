//! Bootstrap provenance — complete lineage from external origin.
//!
//! Tracks the chain of custody through every bootstrap phase,
//! ensuring no gaps in the lineage from Phase 0 to the current phase.

use serde::{Deserialize, Serialize};

use crate::error::{BootstrapError, BootstrapResult};
use crate::fingerprint::SubstrateFingerprint;
use crate::types::BootstrapPhase;

// ── Bootstrap Provenance ────────────────────────────────────────────

/// Provenance record for a bootstrap phase transition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BootstrapProvenance {
    /// Which phase this provenance covers.
    pub phase: BootstrapPhase,
    /// ID of the parent provenance (None for Phase 0).
    pub parent_id: Option<String>,
    /// Unique ID for this provenance entry.
    pub id: String,
    /// Substrate fingerprint at the time of this phase.
    pub fingerprint: SubstrateFingerprint,
    /// Artifact IDs that were used/produced in this phase.
    pub artifacts_used: Vec<String>,
    /// When this provenance was recorded.
    pub produced_at: chrono::DateTime<chrono::Utc>,
}

impl std::fmt::Display for BootstrapProvenance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Provenance(id={}, phase={}, parent={:?})",
            self.id, self.phase, self.parent_id,
        )
    }
}

// ── Provenance Chain ────────────────────────────────────────────────

/// An ordered chain of provenance records from Phase 0 onward.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProvenanceChain {
    entries: Vec<BootstrapProvenance>,
}

impl ProvenanceChain {
    /// Create an empty provenance chain.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add a provenance entry to the chain.
    pub fn push(&mut self, entry: BootstrapProvenance) {
        self.entries.push(entry);
    }

    /// All entries in the chain.
    pub fn entries(&self) -> &[BootstrapProvenance] {
        &self.entries
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Check whether the chain has gaps.
    ///
    /// A gap exists if:
    /// - The first entry is not Phase 0
    /// - Consecutive entries do not have sequential phase ordinals
    /// - A non-root entry has no parent_id
    /// - A non-root entry's parent_id doesn't match the previous entry's id
    pub fn has_gaps(&self) -> bool {
        if self.entries.is_empty() {
            return false;
        }

        // First entry must be Phase 0
        if self.entries[0].phase != BootstrapPhase::Phase0ExternalSubstrate {
            return true;
        }

        // First entry must have no parent
        if self.entries[0].parent_id.is_some() {
            return true;
        }

        // Check consecutive entries
        for window in self.entries.windows(2) {
            let prev = &window[0];
            let curr = &window[1];

            // Phases must be sequential (+1)
            if curr.phase.ordinal() != prev.phase.ordinal() + 1 {
                return true;
            }

            // Current must reference previous as parent
            match &curr.parent_id {
                None => return true,
                Some(parent) => {
                    if *parent != prev.id {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Verify the chain integrity, returning an error if gaps are found.
    pub fn verify(&self) -> BootstrapResult<()> {
        if self.has_gaps() {
            return Err(BootstrapError::ProvenanceGap(
                "provenance chain has gaps or missing links".into(),
            ));
        }
        Ok(())
    }

    /// Get the latest provenance entry.
    pub fn latest(&self) -> Option<&BootstrapProvenance> {
        self.entries.last()
    }

    /// Get the origin (Phase 0) fingerprint.
    pub fn origin_fingerprint(&self) -> Option<&SubstrateFingerprint> {
        self.entries.first().map(|e| &e.fingerprint)
    }
}

// ── Provenance Tracker Trait ────────────────────────────────────────

/// Trait for tracking bootstrap provenance.
pub trait ProvenanceTracker: Send + Sync {
    /// Record provenance for a phase transition.
    fn record(
        &self,
        phase: &BootstrapPhase,
        parent_id: Option<&str>,
        fingerprint: &SubstrateFingerprint,
        artifacts: &[String],
    ) -> BootstrapResult<BootstrapProvenance>;

    /// Name of this tracker.
    fn name(&self) -> &str;
}

/// Simulated provenance tracker for deterministic testing.
pub struct SimulatedProvenanceTracker;

impl SimulatedProvenanceTracker {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimulatedProvenanceTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ProvenanceTracker for SimulatedProvenanceTracker {
    fn record(
        &self,
        phase: &BootstrapPhase,
        parent_id: Option<&str>,
        fingerprint: &SubstrateFingerprint,
        artifacts: &[String],
    ) -> BootstrapResult<BootstrapProvenance> {
        Ok(BootstrapProvenance {
            phase: phase.clone(),
            parent_id: parent_id.map(|s| s.to_string()),
            id: uuid::Uuid::new_v4().to_string(),
            fingerprint: fingerprint.clone(),
            artifacts_used: artifacts.to_vec(),
            produced_at: chrono::Utc::now(),
        })
    }

    fn name(&self) -> &str {
        "simulated-provenance-tracker"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_fingerprint() -> SubstrateFingerprint {
        SubstrateFingerprint {
            rustc_version: "1.75.0".into(),
            target_triple: "x86_64-unknown-linux-gnu".into(),
            os: "linux".into(),
            cpu_arch: "x86_64".into(),
            cargo_lock_hash: "abc123".into(),
            captured_at: Utc::now(),
            features: vec!["default".into()],
        }
    }

    fn build_valid_chain(num_phases: u8) -> ProvenanceChain {
        let mut chain = ProvenanceChain::new();
        let fp = sample_fingerprint();
        let mut prev_id: Option<String> = None;

        for i in 0..num_phases {
            let phase = BootstrapPhase::from_ordinal(i).unwrap();
            let id = format!("prov-{}", i);
            chain.push(BootstrapProvenance {
                phase,
                parent_id: prev_id.clone(),
                id: id.clone(),
                fingerprint: fp.clone(),
                artifacts_used: vec![format!("artifact-{}", i)],
                produced_at: Utc::now(),
            });
            prev_id = Some(id);
        }
        chain
    }

    #[test]
    fn empty_chain_no_gaps() {
        let chain = ProvenanceChain::new();
        assert!(!chain.has_gaps());
        assert!(chain.is_empty());
    }

    #[test]
    fn single_phase0_no_gaps() {
        let chain = build_valid_chain(1);
        assert!(!chain.has_gaps());
        assert_eq!(chain.len(), 1);
    }

    #[test]
    fn full_chain_no_gaps() {
        let chain = build_valid_chain(6);
        assert!(!chain.has_gaps());
        assert_eq!(chain.len(), 6);
        chain.verify().unwrap();
    }

    #[test]
    fn chain_starts_at_phase1_has_gap() {
        let mut chain = ProvenanceChain::new();
        let fp = sample_fingerprint();
        chain.push(BootstrapProvenance {
            phase: BootstrapPhase::Phase1ConfigSelfTuning,
            parent_id: None,
            id: "prov-1".into(),
            fingerprint: fp,
            artifacts_used: vec![],
            produced_at: Utc::now(),
        });
        assert!(chain.has_gaps());
        assert!(chain.verify().is_err());
    }

    #[test]
    fn chain_missing_parent_has_gap() {
        let mut chain = build_valid_chain(2);
        // Mutate the second entry to have no parent
        chain.entries[1].parent_id = None;
        assert!(chain.has_gaps());
    }

    #[test]
    fn chain_wrong_parent_has_gap() {
        let mut chain = build_valid_chain(3);
        // Mutate parent to point to wrong entry
        chain.entries[2].parent_id = Some("wrong-id".into());
        assert!(chain.has_gaps());
    }

    #[test]
    fn chain_skipped_phase_has_gap() {
        let mut chain = ProvenanceChain::new();
        let fp = sample_fingerprint();
        chain.push(BootstrapProvenance {
            phase: BootstrapPhase::Phase0ExternalSubstrate,
            parent_id: None,
            id: "prov-0".into(),
            fingerprint: fp.clone(),
            artifacts_used: vec![],
            produced_at: Utc::now(),
        });
        // Skip Phase 1, go directly to Phase 2
        chain.push(BootstrapProvenance {
            phase: BootstrapPhase::Phase2OperatorSelfGeneration,
            parent_id: Some("prov-0".into()),
            id: "prov-2".into(),
            fingerprint: fp,
            artifacts_used: vec![],
            produced_at: Utc::now(),
        });
        assert!(chain.has_gaps());
    }

    #[test]
    fn chain_origin_fingerprint() {
        let chain = build_valid_chain(3);
        let origin = chain.origin_fingerprint().unwrap();
        assert_eq!(origin.rustc_version, "1.75.0");
    }

    #[test]
    fn chain_latest() {
        let chain = build_valid_chain(4);
        let latest = chain.latest().unwrap();
        assert_eq!(latest.phase, BootstrapPhase::Phase3ModuleSelfRegeneration);
    }

    #[test]
    fn simulated_tracker() {
        let tracker = SimulatedProvenanceTracker::new();
        let fp = sample_fingerprint();
        let prov = tracker
            .record(
                &BootstrapPhase::Phase0ExternalSubstrate,
                None,
                &fp,
                &["artifact-0".into()],
            )
            .unwrap();
        assert_eq!(prov.phase, BootstrapPhase::Phase0ExternalSubstrate);
        assert!(prov.parent_id.is_none());
        assert!(!prov.id.is_empty());
        assert_eq!(tracker.name(), "simulated-provenance-tracker");
    }
}
