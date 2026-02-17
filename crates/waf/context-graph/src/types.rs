use serde::{Deserialize, Serialize};
use std::fmt;
use worldline_types::TemporalAnchor;

/// Content-addressed hash (BLAKE3, 32 bytes).
/// Used as the unique identifier for every WLL node.
///
/// Invariant I.WAF-1: `node_id = blake3(canonical_serialize(content + parent_ids + worldline_id + timestamp))`.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ContentHash(pub [u8; 32]);

impl ContentHash {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Compute the BLAKE3 hash of arbitrary data.
    pub fn hash(data: &[u8]) -> Self {
        Self(*blake3::hash(data).as_bytes())
    }

    /// Zero hash — used as sentinel.
    pub fn zero() -> Self {
        Self([0u8; 32])
    }

    pub fn is_zero(&self) -> bool {
        self.0 == [0u8; 32]
    }

    /// Hex-encode for display.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Parse from hex string.
    pub fn from_hex(hex: &str) -> Result<Self, ContentHashError> {
        if hex.len() != 64 {
            return Err(ContentHashError::InvalidLength(hex.len()));
        }
        let mut bytes = [0u8; 32];
        for i in 0..32 {
            bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
                .map_err(|_| ContentHashError::InvalidHex)?;
        }
        Ok(Self(bytes))
    }
}

impl fmt::Debug for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentHash({})", &self.to_hex()[..12])
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.to_hex()[..12])
    }
}

impl Serialize for ContentHash {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for ContentHash {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let hex = String::deserialize(deserializer)?;
        ContentHash::from_hex(&hex).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ContentHashError {
    #[error("invalid hex length: {0} (expected 64)")]
    InvalidLength(usize),
    #[error("invalid hex character")]
    InvalidHex,
}

/// Governance tiers for WAF evolution changes.
///
/// Tier 0: Automatic (telemetry, config)
/// Tier 1: Automatic + evidence (performance tuning)
/// Tier 2: Quorum (operator logic changes)
/// Tier 3: Human review (safety-adjacent)
/// Tier 4: Human + formal verification (language gen)
/// Tier 5: Multi-human + cryptographic proof (kernel swap)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum GovernanceTier {
    Tier0,
    Tier1,
    Tier2,
    Tier3,
    Tier4,
    Tier5,
}

impl GovernanceTier {
    pub fn requires_human_approval(&self) -> bool {
        matches!(self, Self::Tier3 | Self::Tier4 | Self::Tier5)
    }

    pub fn requires_formal_verification(&self) -> bool {
        matches!(self, Self::Tier4 | Self::Tier5)
    }

    pub fn requires_multi_human(&self) -> bool {
        matches!(self, Self::Tier5)
    }
}

impl fmt::Display for GovernanceTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tier0 => write!(f, "Tier 0 (Auto)"),
            Self::Tier1 => write!(f, "Tier 1 (Auto+Evidence)"),
            Self::Tier2 => write!(f, "Tier 2 (Quorum)"),
            Self::Tier3 => write!(f, "Tier 3 (Human Review)"),
            Self::Tier4 => write!(f, "Tier 4 (Human+Formal)"),
            Self::Tier5 => write!(f, "Tier 5 (Multi-Human+Crypto)"),
        }
    }
}

/// Temporal range for graph queries.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemporalRange {
    pub start: TemporalAnchor,
    pub end: TemporalAnchor,
}

impl TemporalRange {
    pub fn new(start: TemporalAnchor, end: TemporalAnchor) -> Self {
        Self { start, end }
    }

    /// Range that covers all time.
    pub fn all() -> Self {
        Self {
            start: TemporalAnchor::genesis(),
            end: TemporalAnchor::new(u64::MAX, u32::MAX, u16::MAX),
        }
    }

    pub fn contains(&self, anchor: &TemporalAnchor) -> bool {
        &self.start <= anchor && anchor <= &self.end
    }
}

/// Result of validating a node or chain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub checks_performed: usize,
    pub checks_passed: usize,
    pub errors: Vec<String>,
}

impl ValidationResult {
    pub fn ok(checks: usize) -> Self {
        Self {
            valid: true,
            checks_performed: checks,
            checks_passed: checks,
            errors: Vec::new(),
        }
    }

    pub fn failed(checks_performed: usize, checks_passed: usize, errors: Vec<String>) -> Self {
        Self {
            valid: false,
            checks_performed,
            checks_passed,
            errors,
        }
    }
}

/// Type discriminator for node content (used in storage queries).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeContentType {
    Intent,
    Inference,
    Delta,
    Evidence,
    Commitment,
    Consequence,
}

impl fmt::Display for NodeContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Intent => write!(f, "Intent"),
            Self::Inference => write!(f, "Inference"),
            Self::Delta => write!(f, "Delta"),
            Self::Evidence => write!(f, "Evidence"),
            Self::Commitment => write!(f, "Commitment"),
            Self::Consequence => write!(f, "Consequence"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_deterministic() {
        let h1 = ContentHash::hash(b"hello world");
        let h2 = ContentHash::hash(b"hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn content_hash_different_data() {
        let h1 = ContentHash::hash(b"hello");
        let h2 = ContentHash::hash(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn content_hash_hex_roundtrip() {
        let h = ContentHash::hash(b"test data");
        let hex = h.to_hex();
        assert_eq!(hex.len(), 64);
        let restored = ContentHash::from_hex(&hex).unwrap();
        assert_eq!(h, restored);
    }

    #[test]
    fn content_hash_serde_roundtrip() {
        let h = ContentHash::hash(b"serde test");
        let json = serde_json::to_string(&h).unwrap();
        let restored: ContentHash = serde_json::from_str(&json).unwrap();
        assert_eq!(h, restored);
    }

    #[test]
    fn content_hash_zero() {
        let z = ContentHash::zero();
        assert!(z.is_zero());
        let h = ContentHash::hash(b"not zero");
        assert!(!h.is_zero());
    }

    #[test]
    fn content_hash_display_short() {
        let h = ContentHash::hash(b"display");
        let s = format!("{}", h);
        assert_eq!(s.len(), 12);
    }

    #[test]
    fn governance_tier_ordering() {
        assert!(GovernanceTier::Tier0 < GovernanceTier::Tier5);
        assert!(GovernanceTier::Tier2 < GovernanceTier::Tier3);
    }

    #[test]
    fn governance_tier_human_approval() {
        assert!(!GovernanceTier::Tier0.requires_human_approval());
        assert!(!GovernanceTier::Tier1.requires_human_approval());
        assert!(!GovernanceTier::Tier2.requires_human_approval());
        assert!(GovernanceTier::Tier3.requires_human_approval());
        assert!(GovernanceTier::Tier4.requires_human_approval());
        assert!(GovernanceTier::Tier5.requires_human_approval());
    }

    #[test]
    fn governance_tier_formal_verification() {
        assert!(!GovernanceTier::Tier3.requires_formal_verification());
        assert!(GovernanceTier::Tier4.requires_formal_verification());
        assert!(GovernanceTier::Tier5.requires_formal_verification());
    }

    #[test]
    fn temporal_range_all() {
        let range = TemporalRange::all();
        assert!(range.contains(&TemporalAnchor::genesis()));
        assert!(range.contains(&TemporalAnchor::now(0)));
    }

    #[test]
    fn temporal_range_bounded() {
        let range = TemporalRange::new(
            TemporalAnchor::new(100, 0, 0),
            TemporalAnchor::new(200, 0, 0),
        );
        assert!(!range.contains(&TemporalAnchor::new(50, 0, 0)));
        assert!(range.contains(&TemporalAnchor::new(150, 0, 0)));
        assert!(!range.contains(&TemporalAnchor::new(250, 0, 0)));
    }

    #[test]
    fn validation_result_ok() {
        let r = ValidationResult::ok(5);
        assert!(r.valid);
        assert_eq!(r.checks_passed, 5);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn validation_result_failed() {
        let r = ValidationResult::failed(3, 2, vec!["hash mismatch".into()]);
        assert!(!r.valid);
        assert_eq!(r.checks_passed, 2);
        assert_eq!(r.errors.len(), 1);
    }

    #[test]
    fn node_content_type_display() {
        assert_eq!(format!("{}", NodeContentType::Intent), "Intent");
        assert_eq!(format!("{}", NodeContentType::Consequence), "Consequence");
    }

    #[test]
    fn content_hash_from_hex_invalid_length() {
        assert!(ContentHash::from_hex("abcd").is_err());
    }

    #[test]
    fn content_hash_from_hex_invalid_chars() {
        let bad = "zz".repeat(32);
        assert!(ContentHash::from_hex(&bad).is_err());
    }
}
