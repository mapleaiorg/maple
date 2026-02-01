//! Test harness for running conformance tests

use crate::reports::ConformanceReport;
use crate::{ConformanceConfig, ConformanceRunner};
use palm_platform_pack::PlatformPack;
use std::sync::Arc;

/// Test harness for conformance testing
pub struct TestHarness {
    runner: ConformanceRunner,
}

impl TestHarness {
    /// Create a new test harness with default configuration
    pub fn new() -> Self {
        Self {
            runner: ConformanceRunner::new(ConformanceConfig::default()),
        }
    }

    /// Create a test harness with custom configuration
    pub fn with_config(config: ConformanceConfig) -> Self {
        Self {
            runner: ConformanceRunner::new(config),
        }
    }

    /// Run conformance tests and return report
    pub async fn run(&self, pack: Arc<dyn PlatformPack>) -> ConformanceReport {
        self.runner.run(pack).await
    }

    /// Run tests and assert conformance
    pub async fn assert_conformant(&self, pack: Arc<dyn PlatformPack>) {
        let report = self.runner.run(pack).await;

        if !report.is_conformant() {
            panic!(
                "Platform pack '{}' is not conformant:\n{}",
                report.platform_name,
                report.to_text()
            );
        }
    }

    /// Run tests for multiple packs
    pub async fn run_all(&self, packs: Vec<Arc<dyn PlatformPack>>) -> Vec<ConformanceReport> {
        let mut reports = Vec::new();

        for pack in packs {
            let report = self.runner.run(pack).await;
            reports.push(report);
        }

        reports
    }

    /// Run tests and return whether all passed
    pub async fn check_all_conformant(
        &self,
        packs: Vec<Arc<dyn PlatformPack>>,
    ) -> (bool, Vec<ConformanceReport>) {
        let reports = self.run_all(packs).await;
        let all_conformant = reports.iter().all(|r| r.is_conformant());
        (all_conformant, reports)
    }
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_creation() {
        let harness = TestHarness::new();
        // Just verify it creates without panic
        let _ = harness;
    }

    #[test]
    fn test_harness_with_config() {
        let config = ConformanceConfig {
            run_core: true,
            run_behavioral: false,
            run_platform_specific: false,
            ..Default::default()
        };
        let harness = TestHarness::with_config(config);
        let _ = harness;
    }
}
