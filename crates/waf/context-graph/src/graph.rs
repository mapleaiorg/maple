#![allow(clippy::large_enum_variant)]

use crate::error::ValidationError;
use crate::nodes::{
    CommitmentNode, ConsequenceNode, DeltaNode, EvidenceBundleRef, InferenceNode, IntentNode,
};
use crate::types::{ContentHash, GovernanceTier, NodeContentType};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use worldline_types::{TemporalAnchor, WorldlineId};

/// A node in the WLL Context Graph.
///
/// Every WLL node is content-addressed: `node_id = blake3(canonical_serialize(content + parent_ids + worldline_id + timestamp))`.
/// This enforces invariant **I.WAF-1: Context Graph Integrity**.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WllNode {
    /// Content-addressed identifier (BLAKE3 hash of canonical form).
    pub id: ContentHash,
    /// WorldLine this node belongs to.
    pub worldline_id: WorldlineId,
    /// Content payload.
    pub content: NodeContent,
    /// Parent node IDs (causal predecessors).
    pub parent_ids: Vec<ContentHash>,
    /// When this node was created.
    pub timestamp: TemporalAnchor,
    /// Governance tier of this evolution step.
    pub governance_tier: GovernanceTier,
    /// Ed25519 signature over the content hash (hex-encoded).
    pub signature: Option<String>,
    /// Hex-encoded public key of the signer.
    pub signer_public_key: Option<String>,
}

/// The typed content payload for a WLL node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NodeContent {
    Intent(IntentNode),
    Inference(InferenceNode),
    Delta(DeltaNode),
    Evidence(EvidenceBundleRef),
    Commitment(CommitmentNode),
    Consequence(ConsequenceNode),
}

impl NodeContent {
    pub fn content_type(&self) -> NodeContentType {
        match self {
            Self::Intent(_) => NodeContentType::Intent,
            Self::Inference(_) => NodeContentType::Inference,
            Self::Delta(_) => NodeContentType::Delta,
            Self::Evidence(_) => NodeContentType::Evidence,
            Self::Commitment(_) => NodeContentType::Commitment,
            Self::Consequence(_) => NodeContentType::Consequence,
        }
    }
}

impl WllNode {
    /// Compute the content-addressed ID for the given fields.
    /// This is the canonical hash: `blake3(json(content) || json(parent_ids) || json(worldline_id) || json(timestamp))`.
    pub fn compute_id(
        content: &NodeContent,
        parent_ids: &[ContentHash],
        worldline_id: &WorldlineId,
        timestamp: &TemporalAnchor,
    ) -> ContentHash {
        let mut hasher = blake3::Hasher::new();
        // Canonical serialization: JSON of each field concatenated.
        let content_bytes = serde_json::to_vec(content).expect("content serializable");
        let parents_bytes = serde_json::to_vec(parent_ids).expect("parents serializable");
        let wl_bytes = serde_json::to_vec(worldline_id).expect("worldline_id serializable");
        let ts_bytes = serde_json::to_vec(timestamp).expect("timestamp serializable");

        hasher.update(&content_bytes);
        hasher.update(&parents_bytes);
        hasher.update(&wl_bytes);
        hasher.update(&ts_bytes);

        ContentHash::from_bytes(*hasher.finalize().as_bytes())
    }

    /// Create a new WLL node with content-addressed ID.
    pub fn new(
        worldline_id: WorldlineId,
        content: NodeContent,
        parent_ids: Vec<ContentHash>,
        timestamp: TemporalAnchor,
        governance_tier: GovernanceTier,
    ) -> Self {
        let id = Self::compute_id(&content, &parent_ids, &worldline_id, &timestamp);
        Self {
            id,
            worldline_id,
            content,
            parent_ids,
            timestamp,
            governance_tier,
            signature: None,
            signer_public_key: None,
        }
    }

    /// Verify that the stored ID matches the computed content hash.
    pub fn verify_content_hash(&self) -> Result<(), ValidationError> {
        let computed = Self::compute_id(
            &self.content,
            &self.parent_ids,
            &self.worldline_id,
            &self.timestamp,
        );
        if self.id != computed {
            return Err(ValidationError::HashMismatch {
                expected: self.id.clone(),
                computed,
            });
        }
        Ok(())
    }

    /// Verify the Ed25519 signature on this node.
    pub fn verify_signature(&self) -> Result<(), ValidationError> {
        let sig_hex = self
            .signature
            .as_ref()
            .ok_or(ValidationError::SignatureFailed)?;
        let pk_hex = self
            .signer_public_key
            .as_ref()
            .ok_or(ValidationError::SignatureFailed)?;

        let sig_bytes = hex_decode(sig_hex).map_err(|_| ValidationError::SignatureFailed)?;
        let pk_bytes = hex_decode(pk_hex).map_err(|_| ValidationError::SignatureFailed)?;

        if sig_bytes.len() != 64 {
            return Err(ValidationError::SignatureFailed);
        }
        if pk_bytes.len() != 32 {
            return Err(ValidationError::SignatureFailed);
        }

        let signature = Signature::from_bytes(
            sig_bytes
                .as_slice()
                .try_into()
                .map_err(|_| ValidationError::SignatureFailed)?,
        );
        let verifying_key = VerifyingKey::from_bytes(
            pk_bytes
                .as_slice()
                .try_into()
                .map_err(|_| ValidationError::SignatureFailed)?,
        )
        .map_err(|_| ValidationError::SignatureFailed)?;

        use ed25519_dalek::Verifier;
        verifying_key
            .verify(self.id.as_bytes(), &signature)
            .map_err(|_| ValidationError::SignatureFailed)?;

        Ok(())
    }

    /// Sign this node's content hash with the given signing key.
    pub fn sign(&mut self, signing_key: &ed25519_dalek::SigningKey) {
        use ed25519_dalek::Signer;
        let signature = signing_key.sign(self.id.as_bytes());
        self.signature = Some(hex_encode(signature.to_bytes().as_slice()));
        self.signer_public_key = Some(hex_encode(signing_key.verifying_key().as_bytes()));
    }

    /// Get the content type of this node.
    pub fn content_type(&self) -> NodeContentType {
        self.content.content_type()
    }

    /// Check if this is a root node (no parents).
    pub fn is_root(&self) -> bool {
        self.parent_ids.is_empty()
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(hex: &str) -> Result<Vec<u8>, ()> {
    if hex.len() % 2 != 0 {
        return Err(());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| ()))
        .collect()
}

/// An evolution chain is the ordered sequence of nodes representing one evolution cycle:
/// Intent → Inference → Delta → Evidence → Commitment → Consequence.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvolutionChain {
    /// The worldline this chain belongs to.
    pub worldline_id: WorldlineId,
    /// Intent node (starting point).
    pub intent: ContentHash,
    /// Inference node (reasoning).
    pub inference: Option<ContentHash>,
    /// Delta node (code change).
    pub delta: Option<ContentHash>,
    /// Evidence node (proof of correctness).
    pub evidence: Option<ContentHash>,
    /// Commitment node (swap decision).
    pub commitment: Option<ContentHash>,
    /// Consequence node (production observation).
    pub consequence: Option<ContentHash>,
}

impl EvolutionChain {
    pub fn new(worldline_id: WorldlineId, intent: ContentHash) -> Self {
        Self {
            worldline_id,
            intent,
            inference: None,
            delta: None,
            evidence: None,
            commitment: None,
            consequence: None,
        }
    }

    /// Is the chain fully resolved (all 6 stages)?
    pub fn is_complete(&self) -> bool {
        self.inference.is_some()
            && self.delta.is_some()
            && self.evidence.is_some()
            && self.commitment.is_some()
            && self.consequence.is_some()
    }

    /// Return which stage is the current "head" of this chain.
    pub fn current_stage(&self) -> NodeContentType {
        if self.consequence.is_some() {
            NodeContentType::Consequence
        } else if self.commitment.is_some() {
            NodeContentType::Commitment
        } else if self.evidence.is_some() {
            NodeContentType::Evidence
        } else if self.delta.is_some() {
            NodeContentType::Delta
        } else if self.inference.is_some() {
            NodeContentType::Inference
        } else {
            NodeContentType::Intent
        }
    }

    /// Collect all node IDs in this chain as a vec (in order).
    pub fn node_ids(&self) -> Vec<ContentHash> {
        let mut ids = vec![self.intent.clone()];
        if let Some(ref id) = self.inference {
            ids.push(id.clone());
        }
        if let Some(ref id) = self.delta {
            ids.push(id.clone());
        }
        if let Some(ref id) = self.evidence {
            ids.push(id.clone());
        }
        if let Some(ref id) = self.commitment {
            ids.push(id.clone());
        }
        if let Some(ref id) = self.consequence {
            ids.push(id.clone());
        }
        ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::IntentNode;
    use crate::types::GovernanceTier;
    use worldline_types::EventId;

    fn test_worldline() -> WorldlineId {
        WorldlineId::ephemeral()
    }

    fn test_intent() -> IntentNode {
        IntentNode::new(EventId::new(), "test intent", GovernanceTier::Tier0)
    }

    #[test]
    fn wll_node_content_addressing_deterministic() {
        let wl = test_worldline();
        let intent = test_intent();
        let ts = TemporalAnchor::new(1000, 0, 0);
        let content = NodeContent::Intent(intent.clone());

        let id1 = WllNode::compute_id(&content, &[], &wl, &ts);

        let content2 = NodeContent::Intent(intent);
        let id2 = WllNode::compute_id(&content2, &[], &wl, &ts);

        assert_eq!(id1, id2);
    }

    #[test]
    fn wll_node_different_content_different_hash() {
        let wl = test_worldline();
        let ts = TemporalAnchor::new(1000, 0, 0);

        let intent1 = IntentNode::new(EventId::new(), "intent A", GovernanceTier::Tier0);
        let intent2 = IntentNode::new(EventId::new(), "intent B", GovernanceTier::Tier1);

        let id1 = WllNode::compute_id(&NodeContent::Intent(intent1), &[], &wl, &ts);
        let id2 = WllNode::compute_id(&NodeContent::Intent(intent2), &[], &wl, &ts);

        assert_ne!(id1, id2);
    }

    #[test]
    fn wll_node_different_parents_different_hash() {
        let wl = test_worldline();
        let intent = test_intent();
        let ts = TemporalAnchor::new(1000, 0, 0);
        let content = NodeContent::Intent(intent);

        let id1 = WllNode::compute_id(&content, &[], &wl, &ts);
        let id2 = WllNode::compute_id(&content, &[ContentHash::hash(b"parent")], &wl, &ts);

        assert_ne!(id1, id2);
    }

    #[test]
    fn wll_node_verify_content_hash_ok() {
        let node = WllNode::new(
            test_worldline(),
            NodeContent::Intent(test_intent()),
            vec![],
            TemporalAnchor::new(1000, 0, 0),
            GovernanceTier::Tier0,
        );
        assert!(node.verify_content_hash().is_ok());
    }

    #[test]
    fn wll_node_verify_content_hash_tampered() {
        let mut node = WllNode::new(
            test_worldline(),
            NodeContent::Intent(test_intent()),
            vec![],
            TemporalAnchor::new(1000, 0, 0),
            GovernanceTier::Tier0,
        );
        // Tamper with the hash.
        node.id = ContentHash::hash(b"tampered");
        assert!(node.verify_content_hash().is_err());
    }

    #[test]
    fn wll_node_sign_and_verify() {
        let secret_bytes: [u8; 32] = [42u8; 32];
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret_bytes);
        let mut node = WllNode::new(
            test_worldline(),
            NodeContent::Intent(test_intent()),
            vec![],
            TemporalAnchor::new(1000, 0, 0),
            GovernanceTier::Tier0,
        );
        node.sign(&signing_key);
        assert!(node.signature.is_some());
        assert!(node.verify_signature().is_ok());
    }

    #[test]
    fn wll_node_verify_bad_signature() {
        let secret_bytes: [u8; 32] = [42u8; 32];
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret_bytes);
        let mut node = WllNode::new(
            test_worldline(),
            NodeContent::Intent(test_intent()),
            vec![],
            TemporalAnchor::new(1000, 0, 0),
            GovernanceTier::Tier0,
        );
        node.sign(&signing_key);
        // Tamper with the signature.
        node.signature = Some("00".repeat(64));
        assert!(node.verify_signature().is_err());
    }

    #[test]
    fn wll_node_no_signature_fails_verify() {
        let node = WllNode::new(
            test_worldline(),
            NodeContent::Intent(test_intent()),
            vec![],
            TemporalAnchor::new(1000, 0, 0),
            GovernanceTier::Tier0,
        );
        assert!(node.verify_signature().is_err());
    }

    #[test]
    fn wll_node_content_type() {
        let node = WllNode::new(
            test_worldline(),
            NodeContent::Intent(test_intent()),
            vec![],
            TemporalAnchor::new(1000, 0, 0),
            GovernanceTier::Tier0,
        );
        assert_eq!(node.content_type(), NodeContentType::Intent);
    }

    #[test]
    fn wll_node_is_root() {
        let node = WllNode::new(
            test_worldline(),
            NodeContent::Intent(test_intent()),
            vec![],
            TemporalAnchor::new(1000, 0, 0),
            GovernanceTier::Tier0,
        );
        assert!(node.is_root());

        let node2 = WllNode::new(
            test_worldline(),
            NodeContent::Intent(test_intent()),
            vec![ContentHash::hash(b"parent")],
            TemporalAnchor::new(1000, 0, 0),
            GovernanceTier::Tier0,
        );
        assert!(!node2.is_root());
    }

    #[test]
    fn wll_node_serde_roundtrip() {
        let node = WllNode::new(
            test_worldline(),
            NodeContent::Intent(test_intent()),
            vec![],
            TemporalAnchor::new(1000, 0, 0),
            GovernanceTier::Tier0,
        );
        let json = serde_json::to_string(&node).unwrap();
        let restored: WllNode = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, node.id);
        assert_eq!(restored.content_type(), NodeContentType::Intent);
    }

    #[test]
    fn evolution_chain_stages() {
        let wl = test_worldline();
        let mut chain = EvolutionChain::new(wl, ContentHash::hash(b"intent"));
        assert_eq!(chain.current_stage(), NodeContentType::Intent);
        assert!(!chain.is_complete());

        chain.inference = Some(ContentHash::hash(b"inference"));
        assert_eq!(chain.current_stage(), NodeContentType::Inference);

        chain.delta = Some(ContentHash::hash(b"delta"));
        chain.evidence = Some(ContentHash::hash(b"evidence"));
        chain.commitment = Some(ContentHash::hash(b"commitment"));
        chain.consequence = Some(ContentHash::hash(b"consequence"));
        assert!(chain.is_complete());
        assert_eq!(chain.current_stage(), NodeContentType::Consequence);
        assert_eq!(chain.node_ids().len(), 6);
    }

    #[test]
    fn evolution_chain_serde_roundtrip() {
        let chain = EvolutionChain::new(test_worldline(), ContentHash::hash(b"intent"));
        let json = serde_json::to_string(&chain).unwrap();
        let restored: EvolutionChain = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.intent, chain.intent);
    }

    #[test]
    fn node_content_type_mapping() {
        use crate::nodes::*;
        let cases: Vec<(NodeContent, NodeContentType)> = vec![
            (NodeContent::Intent(test_intent()), NodeContentType::Intent),
            (
                NodeContent::Inference(InferenceNode::new("m", 0.5)),
                NodeContentType::Inference,
            ),
            (
                NodeContent::Delta(DeltaNode::new(SubstrateType::Rust, vec![])),
                NodeContentType::Delta,
            ),
            (
                NodeContent::Evidence(EvidenceBundleRef::new(ContentHash::hash(b"e"))),
                NodeContentType::Evidence,
            ),
            (
                NodeContent::Commitment(CommitmentNode::new(
                    ContentHash::hash(b"a"),
                    ContentHash::hash(b"e"),
                )),
                NodeContentType::Commitment,
            ),
            (
                NodeContent::Consequence(ConsequenceNode::stable(0.9)),
                NodeContentType::Consequence,
            ),
        ];
        for (content, expected) in cases {
            assert_eq!(content.content_type(), expected);
        }
    }
}
