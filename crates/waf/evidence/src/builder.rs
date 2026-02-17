use crate::bundle::EvidenceBundle;
use crate::error::EvidenceError;
use crate::invariant_checker::InvariantChecker;
use crate::test_runner::TestRunner;
use crate::types::{EquivalenceTier, ReproBuildResult};
use maple_waf_context_graph::ContentHash;

/// Builds an EvidenceBundle by running tests, checking invariants,
/// and verifying reproducible builds.
pub struct EvidenceBuilder<T: TestRunner, I: InvariantChecker> {
    test_runner: T,
    invariant_checker: I,
    delta_hash: ContentHash,
    artifact_hash: ContentHash,
    equivalence_tier: EquivalenceTier,
    repro_build: Option<ReproBuildResult>,
}

impl<T: TestRunner, I: InvariantChecker> EvidenceBuilder<T, I> {
    pub fn new(
        test_runner: T,
        invariant_checker: I,
        delta_hash: ContentHash,
        artifact_hash: ContentHash,
    ) -> Self {
        Self {
            test_runner,
            invariant_checker,
            delta_hash,
            artifact_hash,
            equivalence_tier: EquivalenceTier::E0,
            repro_build: None,
        }
    }

    pub fn with_equivalence_tier(mut self, tier: EquivalenceTier) -> Self {
        self.equivalence_tier = tier;
        self
    }

    pub fn with_repro_build(mut self, result: ReproBuildResult) -> Self {
        self.repro_build = Some(result);
        self
    }

    /// Build the evidence bundle by executing all checks.
    pub async fn build(self) -> Result<EvidenceBundle, EvidenceError> {
        // 1. Run tests.
        let test_results = self.test_runner.run_tests().await;

        // 2. Check invariants.
        let invariant_results = self.invariant_checker.check_all().await;

        // 3. Assemble bundle.
        let bundle = EvidenceBundle::new(
            self.delta_hash,
            self.artifact_hash,
            test_results,
            invariant_results,
            self.repro_build,
            self.equivalence_tier,
        );

        Ok(bundle)
    }

    /// Build and validate — returns error if evidence is insufficient.
    pub async fn build_and_validate(self) -> Result<EvidenceBundle, EvidenceError> {
        let bundle = self.build().await?;

        if !bundle.all_tests_passed() {
            return Err(EvidenceError::TestFailed(format!(
                "{}/{} tests passed",
                bundle.tests_passed(),
                bundle.test_count()
            )));
        }

        if !bundle.all_invariants_hold() {
            let violated: Vec<_> = bundle
                .invariant_results
                .iter()
                .filter(|i| !i.holds)
                .map(|i| i.id.clone())
                .collect();
            return Err(EvidenceError::InvariantViolated(format!(
                "violations: {:?}",
                violated
            )));
        }

        if !bundle.repro_build_verified() {
            return Err(EvidenceError::ReproBuildFailed(
                "reproducible build not verified".into(),
            ));
        }

        Ok(bundle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invariant_checker::SimulatedInvariantChecker;
    use crate::test_runner::SimulatedTestRunner;

    #[tokio::test]
    async fn build_passing_evidence() {
        let runner = SimulatedTestRunner::all_pass(10);
        let checker = SimulatedInvariantChecker::all_pass();
        let repro = ReproBuildResult::verified(ContentHash::hash(b"build"));

        let bundle = EvidenceBuilder::new(
            runner,
            checker,
            ContentHash::hash(b"delta"),
            ContentHash::hash(b"artifact"),
        )
        .with_repro_build(repro)
        .build_and_validate()
        .await
        .unwrap();

        assert!(bundle.is_sufficient());
        assert!(bundle.verify_hash());
        assert_eq!(bundle.test_count(), 10);
        assert_eq!(bundle.invariant_count(), 14);
    }

    #[tokio::test]
    async fn build_failing_tests() {
        let runner = SimulatedTestRunner::new()
            .with_passing_test("ok")
            .with_failing_test("bad", "assertion");
        let checker = SimulatedInvariantChecker::all_pass();
        let repro = ReproBuildResult::verified(ContentHash::hash(b"b"));

        let result = EvidenceBuilder::new(
            runner,
            checker,
            ContentHash::hash(b"d"),
            ContentHash::hash(b"a"),
        )
        .with_repro_build(repro)
        .build_and_validate()
        .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), EvidenceError::TestFailed(_)));
    }

    #[tokio::test]
    async fn build_failing_invariants() {
        let runner = SimulatedTestRunner::all_pass(5);
        let checker = SimulatedInvariantChecker::with_failures(vec!["I.WAF-1".into()]);
        let repro = ReproBuildResult::verified(ContentHash::hash(b"b"));

        let result = EvidenceBuilder::new(
            runner,
            checker,
            ContentHash::hash(b"d"),
            ContentHash::hash(b"a"),
        )
        .with_repro_build(repro)
        .build_and_validate()
        .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EvidenceError::InvariantViolated(_)
        ));
    }

    #[tokio::test]
    async fn build_no_repro() {
        let runner = SimulatedTestRunner::all_pass(5);
        let checker = SimulatedInvariantChecker::all_pass();

        let result = EvidenceBuilder::new(
            runner,
            checker,
            ContentHash::hash(b"d"),
            ContentHash::hash(b"a"),
        )
        .build_and_validate()
        .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EvidenceError::ReproBuildFailed(_)
        ));
    }

    #[tokio::test]
    async fn build_without_validation() {
        let runner = SimulatedTestRunner::new().with_failing_test("f", "err");
        let checker = SimulatedInvariantChecker::all_pass();

        // build() succeeds even with failing tests (no validation).
        let bundle = EvidenceBuilder::new(
            runner,
            checker,
            ContentHash::hash(b"d"),
            ContentHash::hash(b"a"),
        )
        .build()
        .await
        .unwrap();

        assert!(!bundle.all_tests_passed());
        assert!(!bundle.is_sufficient());
    }

    #[tokio::test]
    async fn equivalence_tier_setting() {
        let runner = SimulatedTestRunner::all_pass(1);
        let checker = SimulatedInvariantChecker::all_pass();

        let bundle = EvidenceBuilder::new(
            runner,
            checker,
            ContentHash::hash(b"d"),
            ContentHash::hash(b"a"),
        )
        .with_equivalence_tier(EquivalenceTier::E2)
        .build()
        .await
        .unwrap();

        assert_eq!(bundle.equivalence_tier, EquivalenceTier::E2);
    }
}
