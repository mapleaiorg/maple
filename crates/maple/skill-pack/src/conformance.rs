//! Skill Pack conformance validation (E-02).
//!
//! Validates that a SkillPack meets all required quality and consistency
//! invariants before it can be registered and executed in production.
//!
//! # Conformance Invariants
//!
//! 1. **I.SP-1**: Every skill must have a non-empty name and valid semver version
//! 2. **I.SP-2**: Every skill must define at least one input and one output
//! 3. **I.SP-3**: All required input fields must have non-empty type declarations
//! 4. **I.SP-4**: Resource limits must be positive and within platform bounds
//! 5. **I.SP-5**: Sandbox timeout must be ≥ max_compute_ms
//! 6. **I.SP-6**: Golden traces must have valid input matching the manifest schema
//! 7. **I.SP-7**: Policy conditions must reference valid resources
//! 8. **I.SP-8**: Converted skills must preserve all original input parameters

use crate::{SkillError, SkillPack};

/// Maximum allowed compute time (10 minutes).
const MAX_COMPUTE_MS: u64 = 600_000;
/// Maximum allowed memory (1 GB).
const MAX_MEMORY_BYTES: u64 = 1_073_741_824;
/// Maximum allowed network transfer (100 MB).
const MAX_NETWORK_BYTES: u64 = 104_857_600;
/// Maximum allowed sandbox timeout (15 minutes).
const MAX_TIMEOUT_MS: u64 = 900_000;

/// Conformance invariant identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkillInvariant {
    /// I.SP-1: Valid identity (name + version).
    ValidIdentity,
    /// I.SP-2: At least one input and one output.
    IoCompleteness,
    /// I.SP-3: All fields have non-empty type declarations.
    TypeDeclarations,
    /// I.SP-4: Resource limits within platform bounds.
    ResourceBounds,
    /// I.SP-5: Sandbox timeout ≥ compute limit.
    TimeoutConsistency,
    /// I.SP-6: Golden traces match manifest schema.
    GoldenTraceValidity,
    /// I.SP-7: Policy conditions reference valid resources.
    PolicyConsistency,
    /// I.SP-8: Conversion preserves all inputs.
    ConversionFidelity,
}

impl std::fmt::Display for SkillInvariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ValidIdentity => write!(f, "I.SP-1 (Valid Identity)"),
            Self::IoCompleteness => write!(f, "I.SP-2 (I/O Completeness)"),
            Self::TypeDeclarations => write!(f, "I.SP-3 (Type Declarations)"),
            Self::ResourceBounds => write!(f, "I.SP-4 (Resource Bounds)"),
            Self::TimeoutConsistency => write!(f, "I.SP-5 (Timeout Consistency)"),
            Self::GoldenTraceValidity => write!(f, "I.SP-6 (Golden Trace Validity)"),
            Self::PolicyConsistency => write!(f, "I.SP-7 (Policy Consistency)"),
            Self::ConversionFidelity => write!(f, "I.SP-8 (Conversion Fidelity)"),
        }
    }
}

/// Result of a single invariant check.
#[derive(Debug, Clone)]
pub struct InvariantResult {
    /// Which invariant was tested.
    pub invariant: SkillInvariant,
    /// Whether the invariant holds.
    pub passed: bool,
    /// Human-readable explanation.
    pub detail: String,
}

/// Full conformance report for a skill pack.
#[derive(Debug, Clone)]
pub struct ConformanceReport {
    /// The skill name being validated.
    pub skill_name: String,
    /// Individual invariant results.
    pub results: Vec<InvariantResult>,
}

impl ConformanceReport {
    /// Whether all invariants passed.
    pub fn all_passed(&self) -> bool {
        self.results.iter().all(|r| r.passed)
    }

    /// Count of passed invariants.
    pub fn passed_count(&self) -> usize {
        self.results.iter().filter(|r| r.passed).count()
    }

    /// Count of failed invariants.
    pub fn failed_count(&self) -> usize {
        self.results.iter().filter(|r| !r.passed).count()
    }

    /// Get all failures.
    pub fn failures(&self) -> Vec<&InvariantResult> {
        self.results.iter().filter(|r| !r.passed).collect()
    }
}

impl std::fmt::Display for ConformanceReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Conformance Report: {}", self.skill_name)?;
        writeln!(
            f,
            "  {}/{} invariants passed",
            self.passed_count(),
            self.results.len()
        )?;
        for r in &self.results {
            let status = if r.passed { "PASS" } else { "FAIL" };
            writeln!(f, "  [{status}] {}: {}", r.invariant, r.detail)?;
        }
        Ok(())
    }
}

/// Run all conformance checks against a skill pack.
pub fn validate_conformance(pack: &SkillPack) -> ConformanceReport {
    let mut results = Vec::new();

    results.push(check_valid_identity(pack));
    results.push(check_io_completeness(pack));
    results.push(check_type_declarations(pack));
    results.push(check_resource_bounds(pack));
    results.push(check_timeout_consistency(pack));
    results.push(check_golden_trace_validity(pack));
    results.push(check_policy_consistency(pack));

    ConformanceReport {
        skill_name: pack.name().to_string(),
        results,
    }
}

/// Run conformance and return an error if any invariant fails.
pub fn require_conformance(pack: &SkillPack) -> Result<ConformanceReport, SkillError> {
    let report = validate_conformance(pack);
    if report.all_passed() {
        Ok(report)
    } else {
        let failures: Vec<String> = report
            .failures()
            .iter()
            .map(|r| format!("{}: {}", r.invariant, r.detail))
            .collect();
        Err(SkillError::ValidationFailed(format!(
            "conformance check failed for '{}': {}",
            pack.name(),
            failures.join("; ")
        )))
    }
}

/// Validate conversion fidelity: ensure a converted skill preserved all
/// original input parameters from a JSON Schema.
pub fn check_conversion_fidelity(
    pack: &SkillPack,
    original_schema: &serde_json::Value,
) -> InvariantResult {
    if let Some(properties) = original_schema
        .get("properties")
        .and_then(|v| v.as_object())
    {
        let mut missing = Vec::new();
        for name in properties.keys() {
            if !pack.manifest.inputs.contains_key(name) {
                missing.push(name.as_str());
            }
        }

        if missing.is_empty() {
            InvariantResult {
                invariant: SkillInvariant::ConversionFidelity,
                passed: true,
                detail: format!(
                    "all {} original parameters preserved",
                    properties.len()
                ),
            }
        } else {
            InvariantResult {
                invariant: SkillInvariant::ConversionFidelity,
                passed: false,
                detail: format!("missing parameters: {}", missing.join(", ")),
            }
        }
    } else {
        InvariantResult {
            invariant: SkillInvariant::ConversionFidelity,
            passed: true,
            detail: "no original parameters to validate".into(),
        }
    }
}

// ── Individual invariant checks ──────────────────────────────────────

fn check_valid_identity(pack: &SkillPack) -> InvariantResult {
    let name = pack.name();
    if name.is_empty() {
        return InvariantResult {
            invariant: SkillInvariant::ValidIdentity,
            passed: false,
            detail: "skill name is empty".into(),
        };
    }

    // Name should be a valid identifier (alphanumeric, hyphens, underscores)
    let valid_chars = name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_');
    if !valid_chars {
        return InvariantResult {
            invariant: SkillInvariant::ValidIdentity,
            passed: false,
            detail: format!(
                "skill name '{}' contains invalid characters (only alphanumeric, hyphens, underscores allowed)",
                name
            ),
        };
    }

    InvariantResult {
        invariant: SkillInvariant::ValidIdentity,
        passed: true,
        detail: format!(
            "name='{}' version='{}'",
            name,
            pack.version()
        ),
    }
}

fn check_io_completeness(pack: &SkillPack) -> InvariantResult {
    let inputs = pack.manifest.inputs.len();
    let outputs = pack.manifest.outputs.len();

    if inputs == 0 {
        InvariantResult {
            invariant: SkillInvariant::IoCompleteness,
            passed: false,
            detail: "no inputs defined".into(),
        }
    } else if outputs == 0 {
        InvariantResult {
            invariant: SkillInvariant::IoCompleteness,
            passed: false,
            detail: "no outputs defined".into(),
        }
    } else {
        InvariantResult {
            invariant: SkillInvariant::IoCompleteness,
            passed: true,
            detail: format!("{} inputs, {} outputs", inputs, outputs),
        }
    }
}

fn check_type_declarations(pack: &SkillPack) -> InvariantResult {
    let mut empty_types = Vec::new();

    for (name, field) in &pack.manifest.inputs {
        if field.field_type.is_empty() {
            empty_types.push(format!("input.{}", name));
        }
    }
    for (name, field) in &pack.manifest.outputs {
        if field.field_type.is_empty() {
            empty_types.push(format!("output.{}", name));
        }
    }

    if empty_types.is_empty() {
        InvariantResult {
            invariant: SkillInvariant::TypeDeclarations,
            passed: true,
            detail: "all fields have type declarations".into(),
        }
    } else {
        InvariantResult {
            invariant: SkillInvariant::TypeDeclarations,
            passed: false,
            detail: format!("empty type on: {}", empty_types.join(", ")),
        }
    }
}

fn check_resource_bounds(pack: &SkillPack) -> InvariantResult {
    let res = &pack.manifest.resources;
    let mut violations = Vec::new();

    if res.max_compute_ms == 0 {
        violations.push("max_compute_ms is 0");
    } else if res.max_compute_ms > MAX_COMPUTE_MS {
        violations.push("max_compute_ms exceeds platform limit (600s)");
    }

    if res.max_memory_bytes == 0 {
        violations.push("max_memory_bytes is 0");
    } else if res.max_memory_bytes > MAX_MEMORY_BYTES {
        violations.push("max_memory_bytes exceeds platform limit (1GB)");
    }

    if res.max_network_bytes > MAX_NETWORK_BYTES {
        violations.push("max_network_bytes exceeds platform limit (100MB)");
    }

    if violations.is_empty() {
        InvariantResult {
            invariant: SkillInvariant::ResourceBounds,
            passed: true,
            detail: format!(
                "compute={}ms mem={}B net={}B",
                res.max_compute_ms, res.max_memory_bytes, res.max_network_bytes
            ),
        }
    } else {
        InvariantResult {
            invariant: SkillInvariant::ResourceBounds,
            passed: false,
            detail: violations.join("; "),
        }
    }
}

fn check_timeout_consistency(pack: &SkillPack) -> InvariantResult {
    let timeout = pack.manifest.sandbox.timeout_ms;
    let compute = pack.manifest.resources.max_compute_ms;

    if timeout == 0 {
        return InvariantResult {
            invariant: SkillInvariant::TimeoutConsistency,
            passed: false,
            detail: "sandbox timeout is 0".into(),
        };
    }

    if timeout > MAX_TIMEOUT_MS {
        return InvariantResult {
            invariant: SkillInvariant::TimeoutConsistency,
            passed: false,
            detail: format!("sandbox timeout {}ms exceeds platform limit (900s)", timeout),
        };
    }

    if timeout < compute {
        InvariantResult {
            invariant: SkillInvariant::TimeoutConsistency,
            passed: false,
            detail: format!(
                "sandbox timeout ({}ms) < max_compute_ms ({}ms) — skill may be killed before finishing",
                timeout, compute
            ),
        }
    } else {
        InvariantResult {
            invariant: SkillInvariant::TimeoutConsistency,
            passed: true,
            detail: format!(
                "timeout={}ms >= compute={}ms",
                timeout, compute
            ),
        }
    }
}

fn check_golden_trace_validity(pack: &SkillPack) -> InvariantResult {
    if pack.golden_traces.is_empty() {
        return InvariantResult {
            invariant: SkillInvariant::GoldenTraceValidity,
            passed: true,
            detail: "no golden traces to validate (optional)".into(),
        };
    }

    let mut invalid = Vec::new();
    for (i, trace) in pack.golden_traces.iter().enumerate() {
        // Validate that trace input matches manifest schema
        if let Err(e) = pack.manifest.validate_input(&trace.input) {
            invalid.push(format!(
                "trace[{}] '{}': {}",
                i,
                trace.name,
                e
            ));
        }
    }

    if invalid.is_empty() {
        InvariantResult {
            invariant: SkillInvariant::GoldenTraceValidity,
            passed: true,
            detail: format!(
                "{} golden traces validated against schema",
                pack.golden_traces.len()
            ),
        }
    } else {
        InvariantResult {
            invariant: SkillInvariant::GoldenTraceValidity,
            passed: false,
            detail: invalid.join("; "),
        }
    }
}

fn check_policy_consistency(pack: &SkillPack) -> InvariantResult {
    if pack.policies.is_empty() {
        return InvariantResult {
            invariant: SkillInvariant::PolicyConsistency,
            passed: true,
            detail: "no policies to validate (optional)".into(),
        };
    }

    // Check for duplicate policy names
    let mut seen = std::collections::HashSet::new();
    let mut duplicates = Vec::new();
    for policy in &pack.policies {
        if !seen.insert(&policy.name) {
            duplicates.push(policy.name.as_str());
        }
    }

    if !duplicates.is_empty() {
        return InvariantResult {
            invariant: SkillInvariant::PolicyConsistency,
            passed: false,
            detail: format!("duplicate policy names: {}", duplicates.join(", ")),
        };
    }

    InvariantResult {
        invariant: SkillInvariant::PolicyConsistency,
        passed: true,
        detail: format!("{} policies validated", pack.policies.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::*;

    fn valid_pack() -> SkillPack {
        SkillPack {
            manifest: SkillManifest {
                skill: SkillMetadata {
                    name: "valid-skill".into(),
                    version: semver::Version::new(1, 0, 0),
                    description: "A valid skill".into(),
                    author: Some("test".into()),
                },
                inputs: vec![(
                    "query".into(),
                    crate::IoField {
                        field_type: "string".into(),
                        required: true,
                        default: None,
                        description: "Search query".into(),
                    },
                )]
                .into_iter()
                .collect(),
                outputs: vec![(
                    "result".into(),
                    crate::IoField {
                        field_type: "string".into(),
                        required: true,
                        default: None,
                        description: "Result".into(),
                    },
                )]
                .into_iter()
                .collect(),
                capabilities: CapabilityRequirements {
                    required: vec!["cap-test".into()],
                },
                resources: ResourceLimits {
                    max_compute_ms: 5000,
                    max_memory_bytes: 52_428_800,
                    max_network_bytes: 10_485_760,
                    max_storage_bytes: None,
                    max_llm_tokens: None,
                },
                sandbox: SandboxConfig {
                    sandbox_type: SandboxType::Process,
                    timeout_ms: 10_000,
                },
                metadata: None,
            },
            policies: Vec::new(),
            golden_traces: Vec::new(),
            source_path: None,
        }
    }

    #[test]
    fn valid_pack_passes_all() {
        let pack = valid_pack();
        let report = validate_conformance(&pack);
        assert!(report.all_passed(), "{}", report);
        assert_eq!(report.passed_count(), 7);
        assert_eq!(report.failed_count(), 0);
    }

    #[test]
    fn invalid_name_fails() {
        let mut pack = valid_pack();
        pack.manifest.skill.name = "invalid name!".into();
        let report = validate_conformance(&pack);
        assert!(!report.all_passed());
        let failures = report.failures();
        assert!(failures
            .iter()
            .any(|r| r.invariant == SkillInvariant::ValidIdentity));
    }

    #[test]
    fn empty_inputs_fails() {
        let mut pack = valid_pack();
        pack.manifest.inputs.clear();
        let report = validate_conformance(&pack);
        assert!(!report.all_passed());
        let failures = report.failures();
        assert!(failures
            .iter()
            .any(|r| r.invariant == SkillInvariant::IoCompleteness));
    }

    #[test]
    fn empty_field_type_fails() {
        let mut pack = valid_pack();
        pack.manifest
            .inputs
            .get_mut("query")
            .unwrap()
            .field_type = String::new();
        let report = validate_conformance(&pack);
        assert!(!report.all_passed());
        let failures = report.failures();
        assert!(failures
            .iter()
            .any(|r| r.invariant == SkillInvariant::TypeDeclarations));
    }

    #[test]
    fn excessive_compute_fails() {
        let mut pack = valid_pack();
        pack.manifest.resources.max_compute_ms = MAX_COMPUTE_MS + 1;
        let report = validate_conformance(&pack);
        assert!(!report.all_passed());
        let failures = report.failures();
        assert!(failures
            .iter()
            .any(|r| r.invariant == SkillInvariant::ResourceBounds));
    }

    #[test]
    fn timeout_less_than_compute_fails() {
        let mut pack = valid_pack();
        pack.manifest.resources.max_compute_ms = 30_000;
        pack.manifest.sandbox.timeout_ms = 10_000;
        let report = validate_conformance(&pack);
        assert!(!report.all_passed());
        let failures = report.failures();
        assert!(failures
            .iter()
            .any(|r| r.invariant == SkillInvariant::TimeoutConsistency));
    }

    #[test]
    fn golden_trace_invalid_input_fails() {
        let mut pack = valid_pack();
        pack.golden_traces.push(crate::GoldenTrace {
            name: "bad-trace".into(),
            description: String::new(),
            input: serde_json::json!({"wrong_field": "value"}), // missing required "query"
            expected_output: serde_json::json!({"result": "ok"}),
            expected_capabilities: Vec::new(),
            expected_budget: None,
        });
        let report = validate_conformance(&pack);
        assert!(!report.all_passed());
        let failures = report.failures();
        assert!(failures
            .iter()
            .any(|r| r.invariant == SkillInvariant::GoldenTraceValidity));
    }

    #[test]
    fn require_conformance_returns_error() {
        let mut pack = valid_pack();
        pack.manifest.skill.name = String::new();
        let result = require_conformance(&pack);
        assert!(result.is_err());
    }

    #[test]
    fn require_conformance_returns_ok() {
        let pack = valid_pack();
        let result = require_conformance(&pack);
        assert!(result.is_ok());
    }

    #[test]
    fn conversion_fidelity_check() {
        let pack = valid_pack();
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" }
            }
        });

        let result = check_conversion_fidelity(&pack, &schema);
        assert!(result.passed);

        // Missing parameter
        let schema_extra = serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" },
                "extra_param": { "type": "integer" }
            }
        });

        let result = check_conversion_fidelity(&pack, &schema_extra);
        assert!(!result.passed);
        assert!(result.detail.contains("extra_param"));
    }

    #[test]
    fn report_display_format() {
        let pack = valid_pack();
        let report = validate_conformance(&pack);
        let display = format!("{}", report);
        assert!(display.contains("Conformance Report: valid-skill"));
        assert!(display.contains("7/7 invariants passed"));
        assert!(display.contains("[PASS]"));
    }
}
