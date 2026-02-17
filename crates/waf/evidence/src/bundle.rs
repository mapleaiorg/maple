use crate::types::{EquivalenceTier, InvariantResult, ReproBuildResult, TestResult};
use maple_waf_context_graph::ContentHash;
use serde::{Deserialize, Serialize};

/// A complete evidence bundle for an evolution step.
///
/// Content-addressed: the bundle hash covers all evidence data.
/// Invariant I.WAF-5: No swap without satisfying EvidenceBundle.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvidenceBundle {
    /// Content-addressed hash of this bundle.
    pub hash: ContentHash,
    /// Hash of the delta (code change) this evidence covers.
    pub delta_hash: ContentHash,
    /// Hash of the compiled artifact.
    pub artifact_hash: ContentHash,
    /// Test execution results.
    pub test_results: Vec<TestResult>,
    /// Invariant check results.
    pub invariant_results: Vec<InvariantResult>,
    /// Reproducible build verification.
    pub repro_build: Option<ReproBuildResult>,
    /// Equivalence tier achieved.
    pub equivalence_tier: EquivalenceTier,
    /// Timestamp of evidence collection.
    pub collected_at_ms: u64,
}

impl EvidenceBundle {
    /// Compute the content hash for this bundle's data.
    fn compute_hash(
        delta_hash: &ContentHash,
        artifact_hash: &ContentHash,
        test_results: &[TestResult],
        invariant_results: &[InvariantResult],
        repro_build: &Option<ReproBuildResult>,
    ) -> ContentHash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(delta_hash.as_bytes());
        hasher.update(artifact_hash.as_bytes());
        let tests_json = serde_json::to_vec(test_results).expect("serializable");
        hasher.update(&tests_json);
        let inv_json = serde_json::to_vec(invariant_results).expect("serializable");
        hasher.update(&inv_json);
        let repro_json = serde_json::to_vec(repro_build).expect("serializable");
        hasher.update(&repro_json);
        ContentHash::from_bytes(*hasher.finalize().as_bytes())
    }

    /// Create a new evidence bundle with computed content hash.
    pub fn new(
        delta_hash: ContentHash,
        artifact_hash: ContentHash,
        test_results: Vec<TestResult>,
        invariant_results: Vec<InvariantResult>,
        repro_build: Option<ReproBuildResult>,
        equivalence_tier: EquivalenceTier,
    ) -> Self {
        let hash = Self::compute_hash(
            &delta_hash,
            &artifact_hash,
            &test_results,
            &invariant_results,
            &repro_build,
        );
        let collected_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_millis() as u64;
        Self {
            hash,
            delta_hash,
            artifact_hash,
            test_results,
            invariant_results,
            repro_build,
            equivalence_tier,
            collected_at_ms,
        }
    }

    /// Verify the content hash matches the data.
    pub fn verify_hash(&self) -> bool {
        let computed = Self::compute_hash(
            &self.delta_hash,
            &self.artifact_hash,
            &self.test_results,
            &self.invariant_results,
            &self.repro_build,
        );
        self.hash == computed
    }

    /// All tests passed?
    pub fn all_tests_passed(&self) -> bool {
        !self.test_results.is_empty() && self.test_results.iter().all(|t| t.passed)
    }

    /// All invariants hold?
    pub fn all_invariants_hold(&self) -> bool {
        !self.invariant_results.is_empty() && self.invariant_results.iter().all(|i| i.holds)
    }

    /// Reproducible build verified?
    pub fn repro_build_verified(&self) -> bool {
        self.repro_build.as_ref().is_some_and(|r| r.reproducible)
    }

    /// Overall: does this evidence bundle satisfy all requirements?
    pub fn is_sufficient(&self) -> bool {
        self.all_tests_passed() && self.all_invariants_hold() && self.repro_build_verified()
    }

    /// Test pass rate.
    pub fn test_pass_rate(&self) -> f64 {
        if self.test_results.is_empty() {
            return 0.0;
        }
        let passed = self.test_results.iter().filter(|t| t.passed).count();
        passed as f64 / self.test_results.len() as f64
    }

    /// Test count.
    pub fn test_count(&self) -> usize {
        self.test_results.len()
    }

    /// Tests passed count.
    pub fn tests_passed(&self) -> usize {
        self.test_results.iter().filter(|t| t.passed).count()
    }

    /// Invariant count.
    pub fn invariant_count(&self) -> usize {
        self.invariant_results.len()
    }

    /// Invariants that hold.
    pub fn invariants_holding(&self) -> usize {
        self.invariant_results.iter().filter(|i| i.holds).count()
    }

    /// Generate a summary string.
    pub fn summary(&self) -> String {
        format!(
            "Tests: {}/{}, Invariants: {}/{}, Repro: {}, Tier: {}",
            self.tests_passed(),
            self.test_count(),
            self.invariants_holding(),
            self.invariant_count(),
            if self.repro_build_verified() {
                "OK"
            } else {
                "FAIL"
            },
            self.equivalence_tier,
        )
    }

    /// Convert to an EvidenceBundleRef for storage in the context graph.
    pub fn to_ref(&self) -> maple_waf_context_graph::EvidenceBundleRef {
        let mut r = maple_waf_context_graph::EvidenceBundleRef::new(self.hash.clone());
        r.test_count = self.test_count();
        r.tests_passed = self.tests_passed();
        r.invariants_checked = self.invariant_count();
        r.invariants_passed = self.invariants_holding();
        r.repro_build_verified = self.repro_build_verified();
        r.summary = self.summary();
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_passing_bundle() -> EvidenceBundle {
        EvidenceBundle::new(
            ContentHash::hash(b"delta"),
            ContentHash::hash(b"artifact"),
            vec![
                TestResult {
                    name: "test1".into(),
                    passed: true,
                    duration_ms: 5,
                    error: None,
                },
                TestResult {
                    name: "test2".into(),
                    passed: true,
                    duration_ms: 10,
                    error: None,
                },
            ],
            vec![InvariantResult {
                id: "I.1".into(),
                description: "Identity Persistence".into(),
                holds: true,
                details: "verified".into(),
            }],
            Some(ReproBuildResult::verified(ContentHash::hash(b"build"))),
            EquivalenceTier::E0,
        )
    }

    #[test]
    fn bundle_content_addressing() {
        let b = make_passing_bundle();
        assert!(b.verify_hash());
    }

    #[test]
    fn bundle_tamper_detection() {
        let mut b = make_passing_bundle();
        b.delta_hash = ContentHash::hash(b"tampered");
        assert!(!b.verify_hash());
    }

    #[test]
    fn bundle_all_tests_passed() {
        let b = make_passing_bundle();
        assert!(b.all_tests_passed());
        assert_eq!(b.test_pass_rate(), 1.0);
    }

    #[test]
    fn bundle_failing_test() {
        let b = EvidenceBundle::new(
            ContentHash::hash(b"d"),
            ContentHash::hash(b"a"),
            vec![
                TestResult {
                    name: "pass".into(),
                    passed: true,
                    duration_ms: 1,
                    error: None,
                },
                TestResult {
                    name: "fail".into(),
                    passed: false,
                    duration_ms: 1,
                    error: Some("assertion".into()),
                },
            ],
            vec![InvariantResult {
                id: "I.1".into(),
                description: "d".into(),
                holds: true,
                details: "ok".into(),
            }],
            Some(ReproBuildResult::verified(ContentHash::hash(b"b"))),
            EquivalenceTier::E0,
        );
        assert!(!b.all_tests_passed());
        assert!(!b.is_sufficient());
        assert_eq!(b.test_pass_rate(), 0.5);
    }

    #[test]
    fn bundle_invariant_violation() {
        let b = EvidenceBundle::new(
            ContentHash::hash(b"d"),
            ContentHash::hash(b"a"),
            vec![TestResult {
                name: "t".into(),
                passed: true,
                duration_ms: 1,
                error: None,
            }],
            vec![InvariantResult {
                id: "I.WAF-1".into(),
                description: "Context Graph".into(),
                holds: false,
                details: "hash mismatch".into(),
            }],
            Some(ReproBuildResult::verified(ContentHash::hash(b"b"))),
            EquivalenceTier::E0,
        );
        assert!(!b.all_invariants_hold());
        assert!(!b.is_sufficient());
    }

    #[test]
    fn bundle_no_repro() {
        let b = EvidenceBundle::new(
            ContentHash::hash(b"d"),
            ContentHash::hash(b"a"),
            vec![TestResult {
                name: "t".into(),
                passed: true,
                duration_ms: 1,
                error: None,
            }],
            vec![InvariantResult {
                id: "I.1".into(),
                description: "d".into(),
                holds: true,
                details: "ok".into(),
            }],
            None,
            EquivalenceTier::E0,
        );
        assert!(!b.repro_build_verified());
        assert!(!b.is_sufficient());
    }

    #[test]
    fn bundle_is_sufficient() {
        let b = make_passing_bundle();
        assert!(b.is_sufficient());
    }

    #[test]
    fn bundle_summary() {
        let b = make_passing_bundle();
        let s = b.summary();
        assert!(s.contains("Tests: 2/2"));
        assert!(s.contains("Invariants: 1/1"));
        assert!(s.contains("Repro: OK"));
    }

    #[test]
    fn bundle_to_ref() {
        let b = make_passing_bundle();
        let r = b.to_ref();
        assert_eq!(r.bundle_hash, b.hash);
        assert_eq!(r.test_count, 2);
        assert_eq!(r.tests_passed, 2);
        assert!(r.all_passed());
    }

    #[test]
    fn bundle_serde_roundtrip() {
        let b = make_passing_bundle();
        let json = serde_json::to_string(&b).unwrap();
        let restored: EvidenceBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.hash, b.hash);
        assert_eq!(restored.test_results.len(), 2);
    }

    #[test]
    fn empty_tests_not_sufficient() {
        let b = EvidenceBundle::new(
            ContentHash::hash(b"d"),
            ContentHash::hash(b"a"),
            vec![],
            vec![InvariantResult {
                id: "I.1".into(),
                description: "d".into(),
                holds: true,
                details: "ok".into(),
            }],
            Some(ReproBuildResult::verified(ContentHash::hash(b"b"))),
            EquivalenceTier::E0,
        );
        assert!(!b.all_tests_passed());
        assert!(!b.is_sufficient());
    }
}
