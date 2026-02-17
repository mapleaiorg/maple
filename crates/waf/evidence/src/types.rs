use maple_waf_context_graph::ContentHash;
use serde::{Deserialize, Serialize};

/// Result of a single test execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestResult {
    /// Name of the test.
    pub name: String,
    /// Whether the test passed.
    pub passed: bool,
    /// Execution time in milliseconds.
    pub duration_ms: u64,
    /// Optional error message if failed.
    pub error: Option<String>,
}

/// Result of a single invariant check.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvariantResult {
    /// Invariant identifier (e.g., "I.WAF-1").
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Whether the invariant holds.
    pub holds: bool,
    /// Details (evidence or violation).
    pub details: String,
}

/// Result of a reproducible build check.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReproBuildResult {
    /// Whether the build was reproducible.
    pub reproducible: bool,
    /// Hash of build 1.
    pub build_hash_1: ContentHash,
    /// Hash of build 2.
    pub build_hash_2: ContentHash,
    /// Build environment info.
    pub environment: String,
}

impl ReproBuildResult {
    pub fn verified(hash: ContentHash) -> Self {
        Self {
            reproducible: true,
            build_hash_1: hash.clone(),
            build_hash_2: hash,
            environment: "simulated".into(),
        }
    }

    pub fn failed(hash1: ContentHash, hash2: ContentHash) -> Self {
        Self {
            reproducible: false,
            build_hash_1: hash1,
            build_hash_2: hash2,
            environment: "simulated".into(),
        }
    }
}

/// Equivalence tier for evidence verification depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EquivalenceTier {
    /// E0: Tests pass.
    E0,
    /// E1: Tests pass + replay equivalence.
    E1,
    /// E2: Tests pass + SMT equivalence.
    E2,
    /// E3: Tests pass + zero-knowledge proof.
    E3,
}

impl std::fmt::Display for EquivalenceTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::E0 => write!(f, "E0 (Tests)"),
            Self::E1 => write!(f, "E1 (Replay)"),
            Self::E2 => write!(f, "E2 (SMT)"),
            Self::E3 => write!(f, "E3 (ZK Proof)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_pass() {
        let r = TestResult {
            name: "test_add".into(),
            passed: true,
            duration_ms: 5,
            error: None,
        };
        assert!(r.passed);
    }

    #[test]
    fn test_result_fail() {
        let r = TestResult {
            name: "test_div".into(),
            passed: false,
            duration_ms: 1,
            error: Some("division by zero".into()),
        };
        assert!(!r.passed);
        assert!(r.error.is_some());
    }

    #[test]
    fn invariant_result_holds() {
        let r = InvariantResult {
            id: "I.WAF-1".into(),
            description: "Context Graph Integrity".into(),
            holds: true,
            details: "all hashes verified".into(),
        };
        assert!(r.holds);
    }

    #[test]
    fn repro_build_verified() {
        let h = ContentHash::hash(b"artifact");
        let r = ReproBuildResult::verified(h.clone());
        assert!(r.reproducible);
        assert_eq!(r.build_hash_1, r.build_hash_2);
    }

    #[test]
    fn repro_build_failed() {
        let r = ReproBuildResult::failed(ContentHash::hash(b"a"), ContentHash::hash(b"b"));
        assert!(!r.reproducible);
        assert_ne!(r.build_hash_1, r.build_hash_2);
    }

    #[test]
    fn equivalence_tier_ordering() {
        assert!(EquivalenceTier::E0 < EquivalenceTier::E3);
        assert!(EquivalenceTier::E1 < EquivalenceTier::E2);
    }

    #[test]
    fn equivalence_tier_display() {
        assert_eq!(format!("{}", EquivalenceTier::E0), "E0 (Tests)");
        assert_eq!(format!("{}", EquivalenceTier::E3), "E3 (ZK Proof)");
    }

    #[test]
    fn test_result_serde() {
        let r = TestResult {
            name: "serde_test".into(),
            passed: true,
            duration_ms: 10,
            error: None,
        };
        let json = serde_json::to_string(&r).unwrap();
        let restored: TestResult = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "serde_test");
    }
}
