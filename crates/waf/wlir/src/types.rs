//! Core types for WLIR provenance tracking and axiomatic constraints.

use maple_waf_context_graph::GovernanceTier;
use serde::{Deserialize, Serialize};

/// Provenance header attached to every WLIR module.
///
/// Records which worldline produced the module, a content hash for
/// integrity verification, the governance tier, and a creation timestamp.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceHeader {
    /// The worldline that produced this module.
    pub worldline_id: String,
    /// BLAKE3 content hash of the module body.
    pub content_hash: String,
    /// Governance tier governing changes to this module.
    pub governance_tier: GovernanceTier,
    /// Creation timestamp in milliseconds since epoch.
    pub timestamp_ms: u64,
}

/// Axiomatic constraints that bound what a WLIR module may do at runtime.
///
/// These constraints are checked before execution and enforced throughout
/// the module lifecycle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AxiomaticConstraints {
    /// Operations that are unconditionally forbidden.
    pub forbidden_operations: Vec<String>,
    /// Maximum recursion depth for operator evaluation.
    pub max_recursion_depth: u32,
    /// Maximum memory budget in megabytes.
    pub memory_limit_mb: u32,
    /// Whether the module is allowed to make network calls.
    pub allow_network: bool,
    /// Whether the module is allowed to access the filesystem.
    pub allow_filesystem: bool,
}

impl Default for AxiomaticConstraints {
    fn default() -> Self {
        Self {
            forbidden_operations: Vec::new(),
            max_recursion_depth: 64,
            memory_limit_mb: 256,
            allow_network: false,
            allow_filesystem: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_header_serde_roundtrip() {
        let header = ProvenanceHeader {
            worldline_id: "wl-001".into(),
            content_hash: "abc123".into(),
            governance_tier: GovernanceTier::Tier2,
            timestamp_ms: 1_700_000_000_000,
        };
        let json = serde_json::to_string(&header).unwrap();
        let restored: ProvenanceHeader = serde_json::from_str(&json).unwrap();
        assert_eq!(header, restored);
    }

    #[test]
    fn axiomatic_constraints_default() {
        let c = AxiomaticConstraints::default();
        assert!(c.forbidden_operations.is_empty());
        assert_eq!(c.max_recursion_depth, 64);
        assert_eq!(c.memory_limit_mb, 256);
        assert!(!c.allow_network);
        assert!(!c.allow_filesystem);
    }

    #[test]
    fn axiomatic_constraints_serde_roundtrip() {
        let c = AxiomaticConstraints {
            forbidden_operations: vec!["eval".into(), "exec".into()],
            max_recursion_depth: 32,
            memory_limit_mb: 128,
            allow_network: true,
            allow_filesystem: false,
        };
        let json = serde_json::to_string(&c).unwrap();
        let restored: AxiomaticConstraints = serde_json::from_str(&json).unwrap();
        assert_eq!(c, restored);
    }

    #[test]
    fn provenance_header_clone() {
        let header = ProvenanceHeader {
            worldline_id: "wl-002".into(),
            content_hash: "def456".into(),
            governance_tier: GovernanceTier::Tier4,
            timestamp_ms: 42,
        };
        let cloned = header.clone();
        assert_eq!(header, cloned);
        assert!(header.governance_tier.requires_formal_verification());
    }
}
