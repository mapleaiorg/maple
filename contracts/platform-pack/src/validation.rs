//! Platform pack validation

use crate::{PlatformPackConfig, PlatformCapabilities};
use serde::{Deserialize, Serialize};

/// Result of pack validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,

    /// Validation errors (if any)
    pub errors: Vec<ValidationError>,

    /// Validation warnings (non-fatal)
    pub warnings: Vec<ValidationWarning>,

    /// Suggestions for improvement
    pub suggestions: Vec<String>,
}

impl ValidationResult {
    /// Create a passing validation result
    pub fn pass() -> Self {
        Self {
            valid: true,
            errors: vec![],
            warnings: vec![],
            suggestions: vec![],
        }
    }

    /// Create a failing validation result
    pub fn fail(errors: Vec<ValidationError>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: vec![],
            suggestions: vec![],
        }
    }

    /// Add an error
    pub fn with_error(mut self, error: ValidationError) -> Self {
        self.errors.push(error);
        self.valid = false;
        self
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: ValidationWarning) -> Self {
        self.warnings.push(warning);
        self
    }

    /// Add a suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    /// Merge another validation result
    pub fn merge(mut self, other: ValidationResult) -> Self {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self.suggestions.extend(other.suggestions);
        self.valid = self.valid && other.valid;
        self
    }
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Field that caused the error (if applicable)
    pub field: Option<String>,

    /// Severity
    pub severity: ErrorSeverity,
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            field: None,
            severity: ErrorSeverity::Error,
        }
    }

    /// Set the field
    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    /// Set as critical
    pub fn critical(mut self) -> Self {
        self.severity = ErrorSeverity::Critical;
        self
    }
}

/// Error severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorSeverity {
    Error,
    Critical,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Warning code
    pub code: String,

    /// Warning message
    pub message: String,

    /// Field that caused the warning (if applicable)
    pub field: Option<String>,
}

impl ValidationWarning {
    /// Create a new validation warning
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            field: None,
        }
    }

    /// Set the field
    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }
}

/// Validate a platform pack configuration
pub fn validate_pack(config: &PlatformPackConfig) -> ValidationResult {
    let mut result = ValidationResult::pass();

    // Validate metadata
    result = result.merge(validate_metadata(&config.metadata));

    // Validate policy
    result = result.merge(validate_policy(&config.policy));

    // Validate health
    result = result.merge(validate_health(&config.health));

    // Validate state
    result = result.merge(validate_state(&config.state));

    // Validate resources
    result = result.merge(validate_resources(&config.resources));

    // Validate recovery
    result = result.merge(validate_recovery(&config.recovery));

    // Validate capabilities consistency
    result = result.merge(validate_capabilities_consistency(config));

    result
}

/// Validate metadata
fn validate_metadata(metadata: &crate::PlatformMetadata) -> ValidationResult {
    let mut result = ValidationResult::pass();

    if metadata.name.is_empty() {
        result = result.with_error(
            ValidationError::new("INVALID_NAME", "Platform name cannot be empty")
                .with_field("metadata.name")
                .critical(),
        );
    }

    if !metadata.name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        result = result.with_error(
            ValidationError::new(
                "INVALID_NAME_CHARS",
                "Platform name must contain only alphanumeric characters, underscores, or hyphens",
            )
            .with_field("metadata.name"),
        );
    }

    if metadata.version.is_empty() {
        result = result.with_error(
            ValidationError::new("INVALID_VERSION", "Platform version cannot be empty")
                .with_field("metadata.version"),
        );
    } else if semver::Version::parse(&metadata.version).is_err() {
        result = result.with_warning(
            ValidationWarning::new(
                "NON_SEMVER_VERSION",
                "Platform version is not valid semver; consider using semver for better compatibility",
            )
            .with_field("metadata.version"),
        );
    }

    result
}

/// Validate policy configuration
fn validate_policy(policy: &crate::PlatformPolicyConfig) -> ValidationResult {
    let mut result = ValidationResult::pass();

    if policy.limits.max_concurrent_deployments == 0 {
        result = result.with_error(
            ValidationError::new(
                "INVALID_LIMIT",
                "max_concurrent_deployments must be greater than 0",
            )
            .with_field("policy.limits.max_concurrent_deployments"),
        );
    }

    if policy.limits.rate_limit_per_minute == 0 {
        result = result.with_warning(
            ValidationWarning::new(
                "ZERO_RATE_LIMIT",
                "rate_limit_per_minute is 0, which will block all operations",
            )
            .with_field("policy.limits.rate_limit_per_minute"),
        );
    }

    // Check for conflicting settings
    if policy.human_approval.allow_auto_approval && !policy.human_approval.always_required.is_empty() {
        result = result.with_suggestion(
            "auto_approval is enabled but some operations always require approval; \
             consider disabling auto_approval for stricter control",
        );
    }

    result
}

/// Validate health configuration
fn validate_health(health: &crate::PlatformHealthConfig) -> ValidationResult {
    let mut result = ValidationResult::pass();

    if health.check_interval_secs == 0 {
        result = result.with_error(
            ValidationError::new("INVALID_INTERVAL", "check_interval_secs must be greater than 0")
                .with_field("health.check_interval_secs"),
        );
    }

    if health.probes.liveness.timeout_secs >= health.probes.liveness.period_secs {
        result = result.with_warning(
            ValidationWarning::new(
                "TIMEOUT_EXCEEDS_PERIOD",
                "Liveness probe timeout should be less than period to avoid overlap",
            )
            .with_field("health.probes.liveness"),
        );
    }

    if health.resonance.attention_exhaustion_threshold > 1.0
        || health.resonance.attention_exhaustion_threshold < 0.0
    {
        result = result.with_error(
            ValidationError::new(
                "INVALID_THRESHOLD",
                "attention_exhaustion_threshold must be between 0 and 1",
            )
            .with_field("health.resonance.attention_exhaustion_threshold"),
        );
    }

    result
}

/// Validate state configuration
fn validate_state(state: &crate::PlatformStateConfig) -> ValidationResult {
    let mut result = ValidationResult::pass();

    if state.checkpoint.auto_checkpoint && state.checkpoint.interval_secs == 0 {
        result = result.with_error(
            ValidationError::new(
                "INVALID_CHECKPOINT_INTERVAL",
                "Checkpoint interval must be greater than 0 when auto_checkpoint is enabled",
            )
            .with_field("state.checkpoint.interval_secs"),
        );
    }

    if state.serialization.encrypt_at_rest && state.serialization.encryption_key_id.is_none() {
        result = result.with_warning(
            ValidationWarning::new(
                "MISSING_ENCRYPTION_KEY",
                "encrypt_at_rest is enabled but encryption_key_id is not set; \
                 key must be provided at runtime",
            )
            .with_field("state.serialization.encryption_key_id"),
        );
    }

    result
}

/// Validate resource configuration
fn validate_resources(resources: &crate::PlatformResourceConfig) -> ValidationResult {
    let mut result = ValidationResult::pass();

    if resources.defaults.cpu_millicores > resources.limits.max_cpu_millicores {
        result = result.with_error(
            ValidationError::new(
                "DEFAULT_EXCEEDS_LIMIT",
                "Default CPU allocation exceeds maximum limit",
            )
            .with_field("resources.defaults.cpu_millicores"),
        );
    }

    if resources.defaults.memory_mb > resources.limits.max_memory_mb {
        result = result.with_error(
            ValidationError::new(
                "DEFAULT_EXCEEDS_LIMIT",
                "Default memory allocation exceeds maximum limit",
            )
            .with_field("resources.defaults.memory_mb"),
        );
    }

    if resources.scaling.min_instances > resources.scaling.max_instances {
        result = result.with_error(
            ValidationError::new(
                "INVALID_SCALING_RANGE",
                "Minimum instances cannot exceed maximum instances",
            )
            .with_field("resources.scaling"),
        );
    }

    if resources.scaling.scale_up_threshold <= resources.scaling.scale_down_threshold {
        result = result.with_warning(
            ValidationWarning::new(
                "OVERLAPPING_THRESHOLDS",
                "Scale-up threshold should be greater than scale-down threshold \
                 to avoid scaling oscillation",
            )
            .with_field("resources.scaling"),
        );
    }

    result
}

/// Validate recovery configuration
fn validate_recovery(recovery: &crate::PlatformRecoveryConfig) -> ValidationResult {
    let mut result = ValidationResult::pass();

    if recovery.auto_recovery.enabled && recovery.auto_recovery.max_attempts == 0 {
        result = result.with_error(
            ValidationError::new(
                "INVALID_RECOVERY_ATTEMPTS",
                "max_attempts must be greater than 0 when auto_recovery is enabled",
            )
            .with_field("recovery.auto_recovery.max_attempts"),
        );
    }

    if recovery.circuit_breaker.enabled {
        if recovery.circuit_breaker.failure_threshold == 0 {
            result = result.with_error(
                ValidationError::new(
                    "INVALID_CIRCUIT_BREAKER",
                    "Circuit breaker failure_threshold must be greater than 0",
                )
                .with_field("recovery.circuit_breaker.failure_threshold"),
            );
        }

        if recovery.circuit_breaker.success_threshold == 0 {
            result = result.with_error(
                ValidationError::new(
                    "INVALID_CIRCUIT_BREAKER",
                    "Circuit breaker success_threshold must be greater than 0",
                )
                .with_field("recovery.circuit_breaker.success_threshold"),
            );
        }
    }

    result
}

/// Validate capabilities consistency with config
fn validate_capabilities_consistency(config: &PlatformPackConfig) -> ValidationResult {
    let mut result = ValidationResult::pass();
    let caps = &config.capabilities;

    // Check migration capability consistency
    if config.state.migration.enable_live_migration && !caps.supports_migration {
        result = result.with_error(
            ValidationError::new(
                "CAPABILITY_MISMATCH",
                "Live migration is enabled in state config but supports_migration capability is false",
            )
            .with_field("capabilities.supports_migration"),
        );
    }

    // Check human approval consistency
    if !config.policy.human_approval.always_required.is_empty() && !caps.supports_human_approval {
        result = result.with_error(
            ValidationError::new(
                "CAPABILITY_MISMATCH",
                "Human approval is required for some operations but supports_human_approval is false",
            )
            .with_field("capabilities.supports_human_approval"),
        );
    }

    // Check checkpoint consistency
    if config.state.checkpoint.auto_checkpoint && !caps.supports_checkpoints {
        result = result.with_error(
            ValidationError::new(
                "CAPABILITY_MISMATCH",
                "Auto checkpoint is enabled but supports_checkpoints capability is false",
            )
            .with_field("capabilities.supports_checkpoints"),
        );
    }

    result
}

/// Validate that a pack configuration can run on a specific runtime
pub fn validate_runtime_compatibility(
    config: &PlatformPackConfig,
    runtime_capabilities: &PlatformCapabilities,
) -> ValidationResult {
    let mut result = ValidationResult::pass();

    // Check required features
    for feature in &config.metadata.compatibility.required_features {
        let supported = match feature.as_str() {
            "live_migration" => runtime_capabilities.supports_migration,
            "hot_reload" => runtime_capabilities.supports_hot_reload,
            "human_approval" => runtime_capabilities.supports_human_approval,
            "checkpoints" => runtime_capabilities.supports_checkpoints,
            "canary_deployments" => runtime_capabilities.supports_canary,
            _ => {
                // Check custom capabilities
                runtime_capabilities.custom.contains_key(feature)
            }
        };

        if !supported {
            result = result.with_error(
                ValidationError::new(
                    "UNSUPPORTED_FEATURE",
                    format!("Required feature '{}' is not supported by the runtime", feature),
                )
                .critical(),
            );
        }
    }

    // Check resource limits
    if let Some(max_deployments) = runtime_capabilities.max_deployments {
        if config.policy.limits.max_concurrent_deployments > max_deployments {
            result = result.with_warning(
                ValidationWarning::new(
                    "EXCEEDS_RUNTIME_LIMIT",
                    format!(
                        "Configured max_concurrent_deployments ({}) exceeds runtime limit ({})",
                        config.policy.limits.max_concurrent_deployments, max_deployments
                    ),
                ),
            );
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_config() {
        let config = PlatformPackConfig::default();
        let result = validate_pack(&config);
        // Default config should have warnings about empty name
        assert!(!result.valid || !result.errors.is_empty());
    }

    #[test]
    fn test_invalid_name() {
        let mut config = PlatformPackConfig::default();
        config.metadata.name = "".to_string();
        let result = validate_pack(&config);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.code == "INVALID_NAME"));
    }

    #[test]
    fn test_invalid_scaling() {
        let mut config = PlatformPackConfig::default();
        config.resources.scaling.min_instances = 10;
        config.resources.scaling.max_instances = 5;
        let result = validate_pack(&config);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.code == "INVALID_SCALING_RANGE"));
    }

    #[test]
    fn test_capability_mismatch() {
        let mut config = PlatformPackConfig::default();
        config.state.migration.enable_live_migration = true;
        config.capabilities.supports_migration = false;
        let result = validate_pack(&config);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.code == "CAPABILITY_MISMATCH"));
    }

    #[test]
    fn test_runtime_compatibility() {
        let mut config = PlatformPackConfig::default();
        config.metadata.compatibility.required_features = vec!["live_migration".to_string()];

        let runtime = PlatformCapabilities {
            supports_migration: false,
            ..Default::default()
        };

        let result = validate_runtime_compatibility(&config, &runtime);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.code == "UNSUPPORTED_FEATURE"));
    }
}
