//! Property tests: Evidence bundles must have content-addressed hashes that match their contents.
//!
//! Verifies invariant I.WAF-5: Evidence Completeness.

use maple_waf_context_graph::ContentHash;
use maple_waf_evidence::*;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Helpers / Strategies
// ---------------------------------------------------------------------------

/// Generate a random test result.
fn arb_test_result() -> impl Strategy<Value = TestResult> {
    ("[a-z_]{3,15}", any::<bool>(), 1u64..500).prop_map(|(name, passed, duration_ms)| TestResult {
        name,
        passed,
        duration_ms,
        error: if passed {
            None
        } else {
            Some("assertion failed".into())
        },
    })
}

/// Generate a vector of test results.
fn arb_test_results(min: usize, max: usize) -> impl Strategy<Value = Vec<TestResult>> {
    prop::collection::vec(arb_test_result(), min..max)
}

/// Generate a random invariant result.
fn arb_invariant_result() -> impl Strategy<Value = InvariantResult> {
    (
        prop_oneof![
            Just("I.1"),
            Just("I.2"),
            Just("I.WAF-1"),
            Just("I.WAF-3"),
            Just("I.WAF-5"),
        ],
        any::<bool>(),
    )
        .prop_map(|(id, holds)| InvariantResult {
            id: id.to_string(),
            description: format!("invariant {}", id),
            holds,
            details: if holds {
                "verified".into()
            } else {
                "violated".into()
            },
        })
}

/// Generate a vector of invariant results.
fn arb_invariant_results(min: usize, max: usize) -> impl Strategy<Value = Vec<InvariantResult>> {
    prop::collection::vec(arb_invariant_result(), min..max)
}

// ---------------------------------------------------------------------------
// Property Tests
// ---------------------------------------------------------------------------

proptest! {
    /// A freshly created evidence bundle always has a valid content hash.
    #[test]
    fn fresh_bundle_hash_always_valid(
        test_results in arb_test_results(1, 10),
        invariant_results in arb_invariant_results(1, 5),
        delta_seed in prop::collection::vec(any::<u8>(), 4..16),
        artifact_seed in prop::collection::vec(any::<u8>(), 4..16),
    ) {
        let bundle = EvidenceBundle::new(
            ContentHash::hash(&delta_seed),
            ContentHash::hash(&artifact_seed),
            test_results,
            invariant_results,
            Some(ReproBuildResult::verified(ContentHash::hash(b"build"))),
            EquivalenceTier::E0,
        );

        prop_assert!(bundle.verify_hash());
    }

    /// Changing the delta_hash always invalidates the bundle hash.
    #[test]
    fn tampered_delta_hash_invalidates_bundle(
        test_results in arb_test_results(1, 5),
        invariant_results in arb_invariant_results(1, 3),
    ) {
        let mut bundle = EvidenceBundle::new(
            ContentHash::hash(b"original-delta"),
            ContentHash::hash(b"artifact"),
            test_results,
            invariant_results,
            Some(ReproBuildResult::verified(ContentHash::hash(b"build"))),
            EquivalenceTier::E0,
        );

        // Verify original is valid.
        prop_assert!(bundle.verify_hash());

        // Tamper with delta_hash.
        bundle.delta_hash = ContentHash::hash(b"tampered-delta");
        prop_assert!(!bundle.verify_hash());
    }

    /// Changing the artifact_hash always invalidates the bundle hash.
    #[test]
    fn tampered_artifact_hash_invalidates_bundle(
        test_results in arb_test_results(1, 5),
        invariant_results in arb_invariant_results(1, 3),
    ) {
        let mut bundle = EvidenceBundle::new(
            ContentHash::hash(b"delta"),
            ContentHash::hash(b"original-artifact"),
            test_results,
            invariant_results,
            Some(ReproBuildResult::verified(ContentHash::hash(b"build"))),
            EquivalenceTier::E0,
        );

        prop_assert!(bundle.verify_hash());

        bundle.artifact_hash = ContentHash::hash(b"tampered-artifact");
        prop_assert!(!bundle.verify_hash());
    }

    /// Adding a test result to the bundle invalidates the content hash.
    #[test]
    fn adding_test_result_invalidates_hash(
        initial_tests in arb_test_results(1, 5),
    ) {
        let mut bundle = EvidenceBundle::new(
            ContentHash::hash(b"delta"),
            ContentHash::hash(b"artifact"),
            initial_tests,
            vec![InvariantResult {
                id: "I.1".into(),
                description: "Identity".into(),
                holds: true,
                details: "ok".into(),
            }],
            Some(ReproBuildResult::verified(ContentHash::hash(b"build"))),
            EquivalenceTier::E0,
        );

        prop_assert!(bundle.verify_hash());

        // Add a new test result.
        bundle.test_results.push(TestResult {
            name: "injected_test".into(),
            passed: true,
            duration_ms: 1,
            error: None,
        });

        // Hash should now be invalid.
        prop_assert!(!bundle.verify_hash());
    }

    /// Modifying an invariant result invalidates the content hash.
    #[test]
    fn modifying_invariant_invalidates_hash(
        invariant_results in arb_invariant_results(1, 5),
    ) {
        let mut bundle = EvidenceBundle::new(
            ContentHash::hash(b"delta"),
            ContentHash::hash(b"artifact"),
            vec![TestResult {
                name: "t".into(),
                passed: true,
                duration_ms: 1,
                error: None,
            }],
            invariant_results,
            Some(ReproBuildResult::verified(ContentHash::hash(b"build"))),
            EquivalenceTier::E0,
        );

        prop_assert!(bundle.verify_hash());

        // Flip the first invariant's hold status.
        if let Some(first) = bundle.invariant_results.first_mut() {
            first.holds = !first.holds;
        }

        prop_assert!(!bundle.verify_hash());
    }

    /// Test count and pass count are always consistent.
    #[test]
    fn test_count_consistent(
        test_results in arb_test_results(1, 20),
    ) {
        let bundle = EvidenceBundle::new(
            ContentHash::hash(b"d"),
            ContentHash::hash(b"a"),
            test_results.clone(),
            vec![InvariantResult {
                id: "I.1".into(),
                description: "d".into(),
                holds: true,
                details: "ok".into(),
            }],
            Some(ReproBuildResult::verified(ContentHash::hash(b"b"))),
            EquivalenceTier::E0,
        );

        let expected_count = test_results.len();
        let expected_passed = test_results.iter().filter(|t| t.passed).count();

        prop_assert_eq!(bundle.test_count(), expected_count);
        prop_assert_eq!(bundle.tests_passed(), expected_passed);

        if expected_passed == expected_count && expected_count > 0 {
            prop_assert!(bundle.all_tests_passed());
        }
    }

    /// Serde roundtrip always preserves the content hash.
    #[test]
    fn serde_roundtrip_preserves_hash(
        test_results in arb_test_results(1, 5),
        invariant_results in arb_invariant_results(1, 3),
    ) {
        let bundle = EvidenceBundle::new(
            ContentHash::hash(b"delta"),
            ContentHash::hash(b"artifact"),
            test_results,
            invariant_results,
            Some(ReproBuildResult::verified(ContentHash::hash(b"build"))),
            EquivalenceTier::E0,
        );

        let json = serde_json::to_string(&bundle).unwrap();
        let restored: EvidenceBundle = serde_json::from_str(&json).unwrap();

        let restored_hash = restored.hash.clone();
        prop_assert_eq!(restored_hash, bundle.hash);
        prop_assert!(restored.verify_hash());
    }
}
