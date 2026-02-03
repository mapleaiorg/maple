//! Conformance test framework

use crate::reports::{ConformanceReport, TestCategory, TestResult};
use palm_platform_pack::{validation, PlatformPack, PlatformPackConfig};
use palm_types::PlatformProfile;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Configuration for conformance testing
#[derive(Debug, Clone)]
pub struct ConformanceConfig {
    /// Run core conformance tests
    pub run_core: bool,

    /// Run behavioral tests
    pub run_behavioral: bool,

    /// Run platform-specific tests
    pub run_platform_specific: bool,

    /// Timeout for individual tests
    pub test_timeout: Duration,

    /// Continue on failure
    pub continue_on_failure: bool,

    /// Verbose output
    pub verbose: bool,
}

impl Default for ConformanceConfig {
    fn default() -> Self {
        Self {
            run_core: true,
            run_behavioral: true,
            run_platform_specific: true,
            test_timeout: Duration::from_secs(30),
            continue_on_failure: true,
            verbose: false,
        }
    }
}

/// Conformance test runner
pub struct ConformanceRunner {
    config: ConformanceConfig,
}

impl ConformanceRunner {
    /// Create a new conformance runner
    pub fn new(config: ConformanceConfig) -> Self {
        Self { config }
    }

    /// Run all conformance tests against a platform pack
    pub async fn run(&self, pack: Arc<dyn PlatformPack>) -> ConformanceReport {
        let start = Instant::now();
        let mut report = ConformanceReport::new(pack.metadata().name.clone());

        tracing::info!("Starting conformance tests for: {}", pack.metadata().name);

        // Core conformance tests
        if self.config.run_core {
            tracing::info!("Running core conformance tests...");
            let core_results = self.run_core_tests(&pack).await;
            report.add_results(TestCategory::Core, core_results);
        }

        // Behavioral conformance tests
        if self.config.run_behavioral {
            tracing::info!("Running behavioral conformance tests...");
            let behavioral_results = self.run_behavioral_tests(&pack).await;
            report.add_results(TestCategory::Behavioral, behavioral_results);
        }

        // Platform-specific conformance tests
        if self.config.run_platform_specific {
            tracing::info!("Running platform-specific conformance tests...");
            let platform_results = self.run_platform_tests(&pack).await;
            report.add_results(TestCategory::PlatformSpecific, platform_results);
        }

        report.duration = start.elapsed();
        report.finalize();

        tracing::info!(
            "Conformance tests complete: {} passed, {} failed, {} skipped",
            report.passed_count(),
            report.failed_count(),
            report.skipped_count()
        );

        report
    }

    async fn run_core_tests(&self, pack: &Arc<dyn PlatformPack>) -> Vec<TestResult> {
        let mut results = Vec::new();

        // Test: Metadata completeness
        results.push(self.test_metadata_completeness(pack).await);

        // Test: Configuration validity
        results.push(self.test_config_validity(pack).await);

        // Test: Capability consistency
        results.push(self.test_capability_consistency(pack).await);

        // Test: Lifecycle callbacks
        results.push(self.test_lifecycle_callbacks(pack).await);

        // Test: Profile matches
        results.push(self.test_profile_matches(pack).await);

        results
    }

    async fn run_behavioral_tests(&self, pack: &Arc<dyn PlatformPack>) -> Vec<TestResult> {
        let mut results = Vec::new();

        // Test: Agent spec validation
        results.push(self.test_agent_spec_validation(pack).await);

        // Test: Resource limits validation
        results.push(self.test_resource_limits(pack).await);

        results
    }

    async fn run_platform_tests(&self, pack: &Arc<dyn PlatformPack>) -> Vec<TestResult> {
        match pack.profile() {
            PlatformProfile::Mapleverse => self.run_mapleverse_tests(pack).await,
            PlatformProfile::Finalverse => self.run_finalverse_tests(pack).await,
            PlatformProfile::IBank => self.run_ibank_tests(pack).await,
            _ => vec![TestResult::skipped(
                "platform_specific",
                "No platform-specific tests for this profile",
            )],
        }
    }

    // Core test implementations

    async fn test_metadata_completeness(&self, pack: &Arc<dyn PlatformPack>) -> TestResult {
        let start = Instant::now();
        let metadata = pack.metadata();

        let mut errors = Vec::new();

        if metadata.name.is_empty() {
            errors.push("metadata.name is empty".to_string());
        }

        if metadata.version.is_empty() {
            errors.push("metadata.version is empty".to_string());
        }

        if metadata.compatibility.min_palm_version.is_empty() {
            errors.push("metadata.min_palm_version is empty".to_string());
        }

        if errors.is_empty() {
            TestResult::passed("metadata_completeness", start.elapsed())
        } else {
            TestResult::failed("metadata_completeness", errors.join("; "), start.elapsed())
        }
    }

    async fn test_config_validity(&self, pack: &Arc<dyn PlatformPack>) -> TestResult {
        let start = Instant::now();

        let config = PlatformPackConfig {
            metadata: pack.metadata().clone(),
            policy: pack.policy_config().clone(),
            health: pack.health_config().clone(),
            state: pack.state_config().clone(),
            resources: pack.resource_config().clone(),
            recovery: pack.recovery_config().clone(),
            capabilities: pack.capabilities().clone(),
        };

        let validation_result = validation::validate_pack(&config);

        if validation_result.valid {
            let mut result = TestResult::passed("config_validity", start.elapsed());
            for warning in &validation_result.warnings {
                result.add_warning(warning.message.clone());
            }
            result
        } else {
            let errors: Vec<String> = validation_result
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect();
            TestResult::failed("config_validity", errors.join("; "), start.elapsed())
        }
    }

    async fn test_capability_consistency(&self, pack: &Arc<dyn PlatformPack>) -> TestResult {
        let start = Instant::now();
        let caps = pack.capabilities();
        let state_config = pack.state_config();
        let policy_config = pack.policy_config();

        let mut errors = Vec::new();

        // Migration capability vs config
        if !caps.supports_migration && state_config.migration.enable_live_migration {
            errors.push("Migration enabled in config but not in capabilities".to_string());
        }

        // Checkpoint capability vs config
        if !caps.supports_checkpoints && state_config.checkpoint.auto_checkpoint {
            errors.push("Auto-checkpoint enabled but checkpoints not supported".to_string());
        }

        // Human approval capability vs policy
        if !caps.supports_human_approval && !policy_config.human_approval.always_required.is_empty()
        {
            errors.push("Human approval required but not supported".to_string());
        }

        if errors.is_empty() {
            TestResult::passed("capability_consistency", start.elapsed())
        } else {
            TestResult::failed("capability_consistency", errors.join("; "), start.elapsed())
        }
    }

    async fn test_lifecycle_callbacks(&self, pack: &Arc<dyn PlatformPack>) -> TestResult {
        let start = Instant::now();

        // Test on_load
        if let Err(e) = pack.on_load().await {
            return TestResult::failed(
                "lifecycle_callbacks",
                format!("on_load failed: {}", e),
                start.elapsed(),
            );
        }

        // Test on_unload
        if let Err(e) = pack.on_unload().await {
            return TestResult::failed(
                "lifecycle_callbacks",
                format!("on_unload failed: {}", e),
                start.elapsed(),
            );
        }

        TestResult::passed("lifecycle_callbacks", start.elapsed())
    }

    async fn test_profile_matches(&self, pack: &Arc<dyn PlatformPack>) -> TestResult {
        let start = Instant::now();
        let profile = pack.profile();
        let metadata = pack.metadata();

        // Profile name should relate to metadata name
        let profile_str = format!("{:?}", profile).to_lowercase();
        let name_lower = metadata.name.to_lowercase();

        if name_lower.contains(&profile_str) || profile_str.contains(&name_lower) {
            TestResult::passed("profile_matches", start.elapsed())
        } else {
            let mut result = TestResult::passed("profile_matches", start.elapsed());
            result.add_warning(format!(
                "Profile {:?} may not match metadata name '{}'",
                profile, metadata.name
            ));
            result
        }
    }

    // Behavioral test implementations

    async fn test_agent_spec_validation(&self, pack: &Arc<dyn PlatformPack>) -> TestResult {
        let start = Instant::now();

        // Create a minimal valid spec
        let spec = palm_types::AgentSpec::new("test-agent", semver::Version::new(1, 0, 0));

        // Should at least not panic
        match pack.validate_agent_spec(&spec).await {
            Ok(_) => TestResult::passed("agent_spec_validation", start.elapsed()),
            Err(e) => {
                let mut result = TestResult::passed("agent_spec_validation", start.elapsed());
                result.add_detail("validation_result", format!("Rejected: {}", e));
                result
            }
        }
    }

    async fn test_resource_limits(&self, pack: &Arc<dyn PlatformPack>) -> TestResult {
        let start = Instant::now();
        let resources = pack.resource_config();

        let mut errors = Vec::new();

        // Check that defaults don't exceed limits
        if resources.defaults.cpu_millicores > resources.limits.max_cpu_millicores {
            errors.push("Default CPU exceeds max limit".to_string());
        }

        if resources.defaults.memory_mb > resources.limits.max_memory_mb {
            errors.push("Default memory exceeds max limit".to_string());
        }

        // Note: attention_pool_size doesn't have a corresponding limit, skipping check

        if errors.is_empty() {
            TestResult::passed("resource_limits", start.elapsed())
        } else {
            TestResult::failed("resource_limits", errors.join("; "), start.elapsed())
        }
    }

    // Platform-specific test implementations

    async fn run_mapleverse_tests(&self, pack: &Arc<dyn PlatformPack>) -> Vec<TestResult> {
        vec![
            self.test_mapleverse_throughput_config(pack).await,
            self.test_mapleverse_no_human_approval(pack).await,
            self.test_mapleverse_fast_recovery(pack).await,
        ]
    }

    async fn test_mapleverse_throughput_config(&self, pack: &Arc<dyn PlatformPack>) -> TestResult {
        let start = Instant::now();
        let caps = pack.capabilities();
        let policy = pack.policy_config();

        let mut errors = Vec::new();

        // Should support high instance counts
        if let Some(max) = caps.max_total_instances {
            if max < 100000 {
                errors.push(format!(
                    "Mapleverse should support >100k instances, got {}",
                    max
                ));
            }
        }

        // Should have high rate limits
        if policy.limits.rate_limit_per_minute < 100 {
            errors.push(format!(
                "Mapleverse should have high rate limit, got {}",
                policy.limits.rate_limit_per_minute
            ));
        }

        if errors.is_empty() {
            TestResult::passed("mapleverse_throughput_config", start.elapsed())
        } else {
            TestResult::failed(
                "mapleverse_throughput_config",
                errors.join("; "),
                start.elapsed(),
            )
        }
    }

    async fn test_mapleverse_no_human_approval(&self, pack: &Arc<dyn PlatformPack>) -> TestResult {
        let start = Instant::now();
        let caps = pack.capabilities();
        let policy = pack.policy_config();

        // Human approval should not be required
        if caps.supports_human_approval {
            return TestResult::failed(
                "mapleverse_no_human_approval",
                "Mapleverse should not support human approval".to_string(),
                start.elapsed(),
            );
        }

        if !policy.human_approval.always_required.is_empty() {
            return TestResult::failed(
                "mapleverse_no_human_approval",
                "Mapleverse should not require human approval".to_string(),
                start.elapsed(),
            );
        }

        TestResult::passed("mapleverse_no_human_approval", start.elapsed())
    }

    async fn test_mapleverse_fast_recovery(&self, pack: &Arc<dyn PlatformPack>) -> TestResult {
        let start = Instant::now();
        let recovery = pack.recovery_config();

        // Should have many restart attempts (using max_attempts field)
        if recovery.auto_recovery.max_attempts < 5 {
            return TestResult::failed(
                "mapleverse_fast_recovery",
                format!(
                    "Mapleverse should allow many recovery attempts, got {}",
                    recovery.auto_recovery.max_attempts
                ),
                start.elapsed(),
            );
        }

        TestResult::passed("mapleverse_fast_recovery", start.elapsed())
    }

    async fn run_finalverse_tests(&self, pack: &Arc<dyn PlatformPack>) -> Vec<TestResult> {
        vec![
            self.test_finalverse_human_approval(pack).await,
            self.test_finalverse_safety_holds(pack).await,
            self.test_finalverse_conservative_recovery(pack).await,
        ]
    }

    async fn test_finalverse_human_approval(&self, pack: &Arc<dyn PlatformPack>) -> TestResult {
        let start = Instant::now();
        let caps = pack.capabilities();
        let policy = pack.policy_config();

        // Must support human approval
        if !caps.supports_human_approval {
            return TestResult::failed(
                "finalverse_human_approval",
                "Finalverse must support human approval".to_string(),
                start.elapsed(),
            );
        }

        // Must require human approval for some destructive ops
        if policy.human_approval.always_required.is_empty() {
            return TestResult::failed(
                "finalverse_human_approval",
                "Finalverse must require human approval for destructive operations".to_string(),
                start.elapsed(),
            );
        }

        TestResult::passed("finalverse_human_approval", start.elapsed())
    }

    async fn test_finalverse_safety_holds(&self, pack: &Arc<dyn PlatformPack>) -> TestResult {
        let start = Instant::now();
        let policy = pack.policy_config();

        // Safety holds must be enabled
        if !policy.safety_holds.enabled {
            return TestResult::failed(
                "finalverse_safety_holds",
                "Finalverse must have safety holds enabled".to_string(),
                start.elapsed(),
            );
        }

        // Must block some operations during safety hold
        if policy.safety_holds.blocked_operations.is_empty() {
            return TestResult::failed(
                "finalverse_safety_holds",
                "Finalverse must block operations during safety hold".to_string(),
                start.elapsed(),
            );
        }

        TestResult::passed("finalverse_safety_holds", start.elapsed())
    }

    async fn test_finalverse_conservative_recovery(
        &self,
        pack: &Arc<dyn PlatformPack>,
    ) -> TestResult {
        let start = Instant::now();
        let recovery = pack.recovery_config();

        // Should have limited recovery attempts
        if recovery.auto_recovery.max_attempts > 5 {
            return TestResult::failed(
                "finalverse_conservative_recovery",
                format!(
                    "Finalverse should have conservative recovery attempts, got {}",
                    recovery.auto_recovery.max_attempts
                ),
                start.elapsed(),
            );
        }

        // Check if checkpoint restore is in recovery actions
        let has_checkpoint_restore = recovery.auto_recovery.recovery_actions.iter().any(|a| {
            matches!(
                a.action_type,
                palm_platform_pack::recovery::RecoveryActionType::RestoreCheckpoint
            )
        });

        if !has_checkpoint_restore {
            let mut result =
                TestResult::passed("finalverse_conservative_recovery", start.elapsed());
            result.add_warning("Finalverse should include checkpoint restore in recovery actions");
            return result;
        }

        TestResult::passed("finalverse_conservative_recovery", start.elapsed())
    }

    async fn run_ibank_tests(&self, pack: &Arc<dyn PlatformPack>) -> Vec<TestResult> {
        vec![
            self.test_ibank_accountability(pack).await,
            self.test_ibank_no_force_operations(pack).await,
            self.test_ibank_audit_retention(pack).await,
        ]
    }

    async fn test_ibank_accountability(&self, pack: &Arc<dyn PlatformPack>) -> TestResult {
        let start = Instant::now();
        let policy = pack.policy_config();

        // Must require accountability proof
        if !policy.accountability.require_proof {
            return TestResult::failed(
                "ibank_accountability",
                "iBank must require accountability proof".to_string(),
                start.elapsed(),
            );
        }

        // Must require pre-audit
        if !policy.accountability.require_pre_audit {
            return TestResult::failed(
                "ibank_accountability",
                "iBank must require pre-audit entries".to_string(),
                start.elapsed(),
            );
        }

        // Must have reconciliation requirements
        if policy.accountability.reconciliation_required.is_empty() {
            return TestResult::failed(
                "ibank_accountability",
                "iBank must have reconciliation requirements".to_string(),
                start.elapsed(),
            );
        }

        TestResult::passed("ibank_accountability", start.elapsed())
    }

    async fn test_ibank_no_force_operations(&self, pack: &Arc<dyn PlatformPack>) -> TestResult {
        let start = Instant::now();
        let policy = pack.policy_config();

        // Force operations should be blocked by safety holds
        if !policy.safety_holds.enabled {
            return TestResult::failed(
                "ibank_no_force_operations",
                "iBank must have safety holds enabled".to_string(),
                start.elapsed(),
            );
        }

        let must_block = ["force_recovery", "force_restart"];
        for op in must_block {
            if !policy
                .safety_holds
                .blocked_operations
                .contains(&op.to_string())
            {
                return TestResult::failed(
                    "ibank_no_force_operations",
                    format!("iBank must block {} operation", op),
                    start.elapsed(),
                );
            }
        }

        // Safety hold should never auto-release
        if policy.safety_holds.auto_release_after_secs.is_some() {
            return TestResult::failed(
                "ibank_no_force_operations",
                "iBank safety holds should never auto-release".to_string(),
                start.elapsed(),
            );
        }

        TestResult::passed("ibank_no_force_operations", start.elapsed())
    }

    async fn test_ibank_audit_retention(&self, pack: &Arc<dyn PlatformPack>) -> TestResult {
        let start = Instant::now();
        let state = pack.state_config();

        // Must have long audit retention (180 days minimum)
        let min_retention_days = 180u32;
        if state.retention.audit_retention_days < min_retention_days {
            return TestResult::failed(
                "ibank_audit_retention",
                format!(
                    "iBank must retain audit logs for at least 180 days, got {} days",
                    state.retention.audit_retention_days
                ),
                start.elapsed(),
            );
        }

        // Must also keep checkpoints for a reasonable time
        if state.retention.checkpoint_retention_days < 30 {
            return TestResult::failed(
                "ibank_audit_retention",
                format!(
                    "iBank should keep checkpoints longer, got {} days",
                    state.retention.checkpoint_retention_days
                ),
                start.elapsed(),
            );
        }

        TestResult::passed("ibank_audit_retention", start.elapsed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ConformanceConfig::default();
        assert!(config.run_core);
        assert!(config.run_behavioral);
        assert!(config.run_platform_specific);
    }
}
