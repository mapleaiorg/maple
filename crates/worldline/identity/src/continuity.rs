use serde::{Deserialize, Serialize};
use worldline_types::{TemporalAnchor, WorldlineId};

use crate::error::ContinuityError;

/// ContinuityChain — links identity across restarts, migrations, and upgrades.
///
/// Per Whitepaper §6.2.1: "AAS maintains continuity chains that link sessions,
/// key rotations, versioned agent descriptors, and execution environments.
/// Every Commitment is attributed not just to an identity, but to a specific
/// continuity context. This prevents responsibility laundering through restarts."
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContinuityChain {
    /// The worldline this chain belongs to
    pub worldline_id: WorldlineId,
    /// Ordered segments of continuous runtime
    pub segments: Vec<ContinuitySegment>,
    /// Current active segment index (if running)
    pub active_segment: Option<usize>,
}

/// One continuous runtime period of a worldline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContinuitySegment {
    /// Segment index in the chain
    pub index: u32,
    /// When this segment started
    pub started: TemporalAnchor,
    /// When this segment ended (None if still active)
    pub ended: Option<TemporalAnchor>,
    /// Hash of the state at segment start (for verification)
    pub start_state_hash: [u8; 32],
    /// Hash of the state at segment end (for verification)
    pub end_state_hash: Option<[u8; 32]>,
    /// Key material active during this segment (reference, not the key itself)
    pub key_ref: KeyRef,
    /// Link to previous segment (for chain integrity)
    pub previous_hash: Option<[u8; 32]>,
}

/// Reference to key material (NOT the key itself).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyRef {
    pub key_id: String,
    pub algorithm: String,
    pub fingerprint: [u8; 32],
}

/// Continuity context for attributing commitments.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContinuityContext {
    pub worldline_id: WorldlineId,
    pub segment_index: u32,
    /// Hash of the full chain up to this point
    pub chain_hash: [u8; 32],
}

impl ContinuitySegment {
    /// Compute a hash of this segment for chain linking.
    pub fn compute_hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"continuity-segment-v1:");
        hasher.update(&self.index.to_le_bytes());
        hasher.update(&self.started.physical_ms.to_le_bytes());
        hasher.update(&self.started.logical.to_le_bytes());
        hasher.update(&self.started.node_id.to_le_bytes());
        hasher.update(&self.start_state_hash);
        if let Some(ref end_hash) = self.end_state_hash {
            hasher.update(end_hash);
        }
        hasher.update(self.key_ref.key_id.as_bytes());
        hasher.update(self.key_ref.algorithm.as_bytes());
        hasher.update(&self.key_ref.fingerprint);
        if let Some(ref prev) = self.previous_hash {
            hasher.update(prev);
        }
        *hasher.finalize().as_bytes()
    }
}

impl ContinuityChain {
    /// Create a new chain for a fresh worldline.
    pub fn new(worldline_id: WorldlineId, initial_key: KeyRef) -> Self {
        let initial_segment = ContinuitySegment {
            index: 0,
            started: TemporalAnchor::now(0),
            ended: None,
            start_state_hash: [0u8; 32], // genesis state
            end_state_hash: None,
            key_ref: initial_key,
            previous_hash: None,
        };
        Self {
            worldline_id,
            segments: vec![initial_segment],
            active_segment: Some(0),
        }
    }

    /// Start a new segment (e.g., after restart).
    /// Links to previous segment via hash chain.
    pub fn start_segment(
        &mut self,
        key_ref: KeyRef,
        state_hash: [u8; 32],
    ) -> Result<u32, ContinuityError> {
        if self.active_segment.is_some() {
            let active_idx = self.segments[self.active_segment.unwrap()].index;
            return Err(ContinuityError::SegmentAlreadyActive(active_idx));
        }

        let previous_hash = self.segments.last().map(|seg| seg.compute_hash());

        let new_index = self.segments.len() as u32;
        let segment = ContinuitySegment {
            index: new_index,
            started: TemporalAnchor::now(0),
            ended: None,
            start_state_hash: state_hash,
            end_state_hash: None,
            key_ref,
            previous_hash,
        };

        self.segments.push(segment);
        self.active_segment = Some(self.segments.len() - 1);
        Ok(new_index)
    }

    /// End the current active segment.
    pub fn end_segment(&mut self, state_hash: [u8; 32]) -> Result<(), ContinuityError> {
        let active_idx = self
            .active_segment
            .ok_or(ContinuityError::NoActiveSegment)?;

        self.segments[active_idx].ended = Some(TemporalAnchor::now(0));
        self.segments[active_idx].end_state_hash = Some(state_hash);
        self.active_segment = None;
        Ok(())
    }

    /// Verify chain integrity (all hashes link correctly).
    pub fn verify_integrity(&self) -> Result<(), ContinuityError> {
        if self.segments.is_empty() {
            return Err(ContinuityError::EmptyChain);
        }

        // First segment should have no previous hash
        if self.segments[0].previous_hash.is_some() {
            return Err(ContinuityError::IntegrityViolation {
                index: 0,
                reason: "genesis segment has previous_hash".into(),
            });
        }

        // Each subsequent segment's previous_hash must match the computed hash of the prior segment
        for i in 1..self.segments.len() {
            let expected = self.segments[i - 1].compute_hash();
            match self.segments[i].previous_hash {
                Some(ref actual) if *actual == expected => {}
                Some(_) => {
                    return Err(ContinuityError::IntegrityViolation {
                        index: self.segments[i].index,
                        reason: "previous_hash does not match prior segment hash".into(),
                    });
                }
                None => {
                    return Err(ContinuityError::IntegrityViolation {
                        index: self.segments[i].index,
                        reason: "missing previous_hash for non-genesis segment".into(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Get the current continuity context for commitment attribution.
    pub fn current_context(&self) -> Option<ContinuityContext> {
        let active_idx = self.active_segment?;
        let segment = &self.segments[active_idx];

        // Compute chain hash up to and including the active segment
        let chain_hash = self.compute_chain_hash();

        Some(ContinuityContext {
            worldline_id: self.worldline_id.clone(),
            segment_index: segment.index,
            chain_hash,
        })
    }

    /// Total runtime duration across all segments.
    pub fn total_runtime(&self) -> std::time::Duration {
        let mut total_ms: u64 = 0;
        for seg in &self.segments {
            let end_ms = seg.ended.map(|e| e.physical_ms).unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64
            });
            total_ms += end_ms.saturating_sub(seg.started.physical_ms);
        }
        std::time::Duration::from_millis(total_ms)
    }

    /// Was this worldline active at a given temporal anchor?
    pub fn was_active_at(&self, anchor: &TemporalAnchor) -> bool {
        self.segments.iter().any(|seg| {
            let after_start = anchor.physical_ms >= seg.started.physical_ms;
            let before_end = match seg.ended {
                Some(ref end) => anchor.physical_ms <= end.physical_ms,
                None => true, // still active
            };
            after_start && before_end
        })
    }

    /// Compute a hash of the full chain (all segments).
    fn compute_chain_hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"continuity-chain-v1:");
        hasher.update(self.worldline_id.identity_hash());
        for seg in &self.segments {
            hasher.update(&seg.compute_hash());
        }
        *hasher.finalize().as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldline_types::IdentityMaterial;

    fn test_key_ref(id: &str) -> KeyRef {
        KeyRef {
            key_id: id.to_string(),
            algorithm: "ed25519".to_string(),
            fingerprint: [0u8; 32],
        }
    }

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    #[test]
    fn new_chain_has_one_segment() {
        let chain = ContinuityChain::new(test_worldline(), test_key_ref("key-0"));
        assert_eq!(chain.segments.len(), 1);
        assert_eq!(chain.active_segment, Some(0));
    }

    #[test]
    fn start_segment_links_to_previous() {
        let mut chain = ContinuityChain::new(test_worldline(), test_key_ref("key-0"));
        chain.end_segment([1u8; 32]).unwrap();

        let idx = chain
            .start_segment(test_key_ref("key-1"), [2u8; 32])
            .unwrap();
        assert_eq!(idx, 1);
        assert_eq!(chain.segments.len(), 2);
        assert!(chain.segments[1].previous_hash.is_some());
    }

    #[test]
    fn end_and_start_preserves_chain() {
        let mut chain = ContinuityChain::new(test_worldline(), test_key_ref("key-0"));

        chain.end_segment([1u8; 32]).unwrap();
        chain
            .start_segment(test_key_ref("key-1"), [2u8; 32])
            .unwrap();
        chain.end_segment([3u8; 32]).unwrap();
        chain
            .start_segment(test_key_ref("key-2"), [4u8; 32])
            .unwrap();

        assert_eq!(chain.segments.len(), 3);
        chain.verify_integrity().unwrap();
    }

    #[test]
    fn verify_integrity_detects_tampered_hash() {
        let mut chain = ContinuityChain::new(test_worldline(), test_key_ref("key-0"));
        chain.end_segment([1u8; 32]).unwrap();
        chain
            .start_segment(test_key_ref("key-1"), [2u8; 32])
            .unwrap();

        // Tamper with the previous_hash
        chain.segments[1].previous_hash = Some([0xff; 32]);

        assert!(chain.verify_integrity().is_err());
    }

    #[test]
    fn was_active_at_returns_correct_results() {
        let chain = ContinuityChain::new(test_worldline(), test_key_ref("key-0"));

        // The initial segment started at "now", so an anchor far in the past shouldn't match
        let past = TemporalAnchor::new(1, 0, 0);
        assert!(!chain.was_active_at(&past));

        // The current segment is active, so "now" should match
        let now = TemporalAnchor::now(0);
        assert!(chain.was_active_at(&now));
    }

    #[test]
    fn simulate_restart_cycle() {
        let wid = test_worldline();
        let mut chain = ContinuityChain::new(wid.clone(), test_key_ref("key-0"));

        // Running...
        assert!(chain.current_context().is_some());

        // Suspend
        chain.end_segment([10u8; 32]).unwrap();
        assert!(chain.current_context().is_none());
        assert!(chain.active_segment.is_none());

        // Resume
        chain
            .start_segment(test_key_ref("key-0"), [10u8; 32])
            .unwrap();
        let ctx = chain.current_context().unwrap();
        assert_eq!(ctx.worldline_id, wid);
        assert_eq!(ctx.segment_index, 1);

        // Verify integrity after restart
        chain.verify_integrity().unwrap();
    }

    #[test]
    fn cannot_start_segment_when_active() {
        let mut chain = ContinuityChain::new(test_worldline(), test_key_ref("key-0"));
        let result = chain.start_segment(test_key_ref("key-1"), [1u8; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn cannot_end_segment_when_none_active() {
        let mut chain = ContinuityChain::new(test_worldline(), test_key_ref("key-0"));
        chain.end_segment([1u8; 32]).unwrap();
        let result = chain.end_segment([2u8; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn chain_serialization_roundtrip() {
        let mut chain = ContinuityChain::new(test_worldline(), test_key_ref("key-0"));
        chain.end_segment([1u8; 32]).unwrap();
        chain
            .start_segment(test_key_ref("key-1"), [2u8; 32])
            .unwrap();

        let json = serde_json::to_string(&chain).unwrap();
        let restored: ContinuityChain = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.segments.len(), 2);
        assert_eq!(restored.active_segment, Some(1));
    }
}
