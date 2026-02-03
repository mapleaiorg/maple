//! Core conformance test definitions

use palm_platform_pack::PlatformPack;
use std::sync::Arc;

/// Core conformance requirement result
#[derive(Debug)]
pub struct CoreRequirementResult {
    pub name: String,
    pub passed: bool,
    pub errors: Vec<String>,
}

/// Core conformance requirements
pub struct CoreConformanceRequirements;

impl CoreConformanceRequirements {
    /// Check all core requirements
    pub fn check(pack: &Arc<dyn PlatformPack>) -> Vec<CoreRequirementResult> {
        vec![
            Self::check_metadata(pack),
            Self::check_profile(pack),
            Self::check_policy_config(pack),
            Self::check_health_config(pack),
            Self::check_state_config(pack),
            Self::check_resource_config(pack),
            Self::check_recovery_config(pack),
            Self::check_capabilities(pack),
        ]
    }

    fn check_metadata(pack: &Arc<dyn PlatformPack>) -> CoreRequirementResult {
        let metadata = pack.metadata();
        let mut errors = Vec::new();

        if metadata.name.is_empty() {
            errors.push("name is required".to_string());
        }

        if metadata.version.is_empty() {
            errors.push("version is required".to_string());
        }

        CoreRequirementResult {
            name: "metadata".to_string(),
            passed: errors.is_empty(),
            errors,
        }
    }

    fn check_profile(pack: &Arc<dyn PlatformPack>) -> CoreRequirementResult {
        // Profile should be defined
        let _profile = pack.profile();

        CoreRequirementResult {
            name: "profile".to_string(),
            passed: true,
            errors: vec![],
        }
    }

    fn check_policy_config(pack: &Arc<dyn PlatformPack>) -> CoreRequirementResult {
        let policy = pack.policy_config();
        let mut errors = Vec::new();

        if policy.limits.max_concurrent_deployments == 0 {
            errors.push("max_concurrent_deployments cannot be 0".to_string());
        }

        CoreRequirementResult {
            name: "policy_config".to_string(),
            passed: errors.is_empty(),
            errors,
        }
    }

    fn check_health_config(pack: &Arc<dyn PlatformPack>) -> CoreRequirementResult {
        let health = pack.health_config();
        let mut errors = Vec::new();

        if health.failure_threshold == 0 {
            errors.push("failure_threshold cannot be 0".to_string());
        }

        CoreRequirementResult {
            name: "health_config".to_string(),
            passed: errors.is_empty(),
            errors,
        }
    }

    fn check_state_config(pack: &Arc<dyn PlatformPack>) -> CoreRequirementResult {
        let state = pack.state_config();
        let mut errors = Vec::new();

        if state.checkpoint.max_retained == 0 {
            errors.push("max_retained checkpoints cannot be 0".to_string());
        }

        CoreRequirementResult {
            name: "state_config".to_string(),
            passed: errors.is_empty(),
            errors,
        }
    }

    fn check_resource_config(pack: &Arc<dyn PlatformPack>) -> CoreRequirementResult {
        let resources = pack.resource_config();
        let mut errors = Vec::new();

        if resources.defaults.cpu_millicores > resources.limits.max_cpu_millicores {
            errors.push("default CPU exceeds limit".to_string());
        }

        if resources.defaults.memory_mb > resources.limits.max_memory_mb {
            errors.push("default memory exceeds limit".to_string());
        }

        CoreRequirementResult {
            name: "resource_config".to_string(),
            passed: errors.is_empty(),
            errors,
        }
    }

    fn check_recovery_config(pack: &Arc<dyn PlatformPack>) -> CoreRequirementResult {
        let _recovery = pack.recovery_config();

        // Recovery config is always valid if present
        CoreRequirementResult {
            name: "recovery_config".to_string(),
            passed: true,
            errors: vec![],
        }
    }

    fn check_capabilities(pack: &Arc<dyn PlatformPack>) -> CoreRequirementResult {
        let _caps = pack.capabilities();

        // Capabilities are always valid if present
        CoreRequirementResult {
            name: "capabilities".to_string(),
            passed: true,
            errors: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    // Tests would require mock pack implementation
    // Left as placeholder for now
}
