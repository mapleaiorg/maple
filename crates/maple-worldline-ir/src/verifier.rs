//! WLIR module verification.
//!
//! The verifier checks five independent aspects of a WLIR module:
//! 1. **Type Correctness** — register types are consistent
//! 2. **Commitment Boundary Integrity** — matched Enter/Exit pairs
//! 3. **Provenance Completeness** — all side-effecting ops have provenance
//! 4. **Safety Fence Ordering** — safety fences precede critical ops
//! 5. **Control Flow Integrity** — no unreachable code, valid jump targets
//!
//! Each aspect produces a `VerificationResult` (pass/fail with details).
//! The verifier trait is implemented by `SimulatedVerifier` for deterministic testing.

use crate::error::{WlirError, WlirResult};
use crate::module::WlirModule;
use crate::types::WlirConfig;

// ── Verification Aspect ──────────────────────────────────────────────

/// An independent aspect of WLIR verification.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum VerificationAspect {
    /// Register types are consistent across instructions.
    TypeCorrectness,
    /// Commitment boundary Enter/Exit pairs are balanced.
    CommitmentBoundaryIntegrity,
    /// All side-effecting instructions have provenance records.
    ProvenanceCompleteness,
    /// Safety fences precede safety-critical operations.
    SafetyFenceOrdering,
    /// Control flow is valid (no unreachable code, valid targets).
    ControlFlowIntegrity,
}

impl std::fmt::Display for VerificationAspect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TypeCorrectness => write!(f, "type-correctness"),
            Self::CommitmentBoundaryIntegrity => write!(f, "commitment-boundary-integrity"),
            Self::ProvenanceCompleteness => write!(f, "provenance-completeness"),
            Self::SafetyFenceOrdering => write!(f, "safety-fence-ordering"),
            Self::ControlFlowIntegrity => write!(f, "control-flow-integrity"),
        }
    }
}

// ── Verification Result ──────────────────────────────────────────────

/// Result of verifying a single aspect.
#[derive(Clone, Debug)]
pub struct VerificationResult {
    /// Which aspect was checked.
    pub aspect: VerificationAspect,
    /// Whether the aspect passed verification.
    pub passed: bool,
    /// Details about the result (pass message or failure reason).
    pub details: String,
    /// Number of items checked within this aspect.
    pub items_checked: usize,
    /// Number of items that passed.
    pub items_passed: usize,
}

impl VerificationResult {
    /// Create a passing result.
    pub fn pass(aspect: VerificationAspect, items_checked: usize, details: impl Into<String>) -> Self {
        Self {
            aspect,
            passed: true,
            details: details.into(),
            items_checked,
            items_passed: items_checked,
        }
    }

    /// Create a failing result.
    pub fn fail(
        aspect: VerificationAspect,
        items_checked: usize,
        items_passed: usize,
        details: impl Into<String>,
    ) -> Self {
        Self {
            aspect,
            passed: false,
            details: details.into(),
            items_checked,
            items_passed,
        }
    }
}

impl std::fmt::Display for VerificationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} ({}/{} items) — {}",
            self.aspect,
            if self.passed { "PASS" } else { "FAIL" },
            self.items_passed,
            self.items_checked,
            self.details,
        )
    }
}

// ── Verification Report ──────────────────────────────────────────────

/// Aggregated report from verifying all aspects of a module.
#[derive(Clone, Debug)]
pub struct VerificationReport {
    /// Results for each verification aspect.
    pub results: Vec<VerificationResult>,
    /// Whether the entire module passed verification.
    pub all_passed: bool,
    /// Total number of aspects checked.
    pub aspects_checked: usize,
    /// Number of aspects that passed.
    pub aspects_passed: usize,
}

impl VerificationReport {
    /// Create a report from a set of aspect results.
    pub fn from_results(results: Vec<VerificationResult>) -> Self {
        let aspects_checked = results.len();
        let aspects_passed = results.iter().filter(|r| r.passed).count();
        let all_passed = aspects_passed == aspects_checked;
        Self {
            results,
            all_passed,
            aspects_checked,
            aspects_passed,
        }
    }

    /// Get the result for a specific aspect.
    pub fn result_for(&self, aspect: &VerificationAspect) -> Option<&VerificationResult> {
        self.results.iter().find(|r| r.aspect == *aspect)
    }

    /// Get all failed aspects.
    pub fn failures(&self) -> Vec<&VerificationResult> {
        self.results.iter().filter(|r| !r.passed).collect()
    }
}

impl std::fmt::Display for VerificationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "VerificationReport: {} ({}/{} aspects passed)",
            if self.all_passed { "PASS" } else { "FAIL" },
            self.aspects_passed,
            self.aspects_checked,
        )?;
        for result in &self.results {
            writeln!(f, "  {}", result)?;
        }
        Ok(())
    }
}

// ── Verifier Trait ───────────────────────────────────────────────────

/// Trait for verifying WLIR modules.
///
/// Implementations verify all five aspects and produce a `VerificationReport`.
pub trait WlirVerifier: Send + Sync {
    /// Verify a WLIR module, returning a full verification report.
    fn verify(&self, module: &WlirModule, config: &WlirConfig) -> WlirResult<VerificationReport>;

    /// Verify a single aspect of a WLIR module.
    fn verify_aspect(
        &self,
        module: &WlirModule,
        aspect: &VerificationAspect,
        config: &WlirConfig,
    ) -> WlirResult<VerificationResult>;

    /// Name of this verifier implementation.
    fn name(&self) -> &str;
}

// ── Simulated Verifier ───────────────────────────────────────────────

/// Simulated verifier for deterministic testing.
///
/// Each aspect can be individually configured to pass or fail.
pub struct SimulatedVerifier {
    /// Per-aspect pass/fail configuration.
    pub aspect_results: std::collections::HashMap<VerificationAspect, bool>,
}

impl SimulatedVerifier {
    /// Create a verifier where all aspects pass.
    pub fn all_pass() -> Self {
        let mut aspect_results = std::collections::HashMap::new();
        aspect_results.insert(VerificationAspect::TypeCorrectness, true);
        aspect_results.insert(VerificationAspect::CommitmentBoundaryIntegrity, true);
        aspect_results.insert(VerificationAspect::ProvenanceCompleteness, true);
        aspect_results.insert(VerificationAspect::SafetyFenceOrdering, true);
        aspect_results.insert(VerificationAspect::ControlFlowIntegrity, true);
        Self { aspect_results }
    }

    /// Create a verifier where all aspects fail.
    pub fn all_fail() -> Self {
        let mut aspect_results = std::collections::HashMap::new();
        aspect_results.insert(VerificationAspect::TypeCorrectness, false);
        aspect_results.insert(VerificationAspect::CommitmentBoundaryIntegrity, false);
        aspect_results.insert(VerificationAspect::ProvenanceCompleteness, false);
        aspect_results.insert(VerificationAspect::SafetyFenceOrdering, false);
        aspect_results.insert(VerificationAspect::ControlFlowIntegrity, false);
        Self { aspect_results }
    }

    /// Create a verifier with a specific aspect failing.
    pub fn failing_aspect(aspect: VerificationAspect) -> Self {
        let mut verifier = Self::all_pass();
        verifier.aspect_results.insert(aspect, false);
        verifier
    }

    /// All five verification aspects in standard order.
    fn all_aspects() -> Vec<VerificationAspect> {
        vec![
            VerificationAspect::TypeCorrectness,
            VerificationAspect::CommitmentBoundaryIntegrity,
            VerificationAspect::ProvenanceCompleteness,
            VerificationAspect::SafetyFenceOrdering,
            VerificationAspect::ControlFlowIntegrity,
        ]
    }

    /// Simulate verification of a single aspect.
    fn simulate_aspect(
        &self,
        module: &WlirModule,
        aspect: &VerificationAspect,
        config: &WlirConfig,
    ) -> VerificationResult {
        let should_pass = self.aspect_results.get(aspect).copied().unwrap_or(true);
        let func_count = module.functions.len();

        match aspect {
            VerificationAspect::TypeCorrectness => {
                let items = module.total_instructions();
                if should_pass {
                    VerificationResult::pass(
                        aspect.clone(),
                        items,
                        format!("{} instructions type-checked", items),
                    )
                } else {
                    VerificationResult::fail(
                        aspect.clone(),
                        items,
                        items.saturating_sub(1),
                        "type mismatch in instruction".to_string(),
                    )
                }
            }
            VerificationAspect::CommitmentBoundaryIntegrity => {
                // Actually check commitment boundary balance if passing
                if should_pass {
                    let balanced = module.functions.iter().all(|f| f.commitment_boundaries_balanced());
                    if balanced {
                        VerificationResult::pass(
                            aspect.clone(),
                            func_count,
                            format!("{} functions have balanced commitment boundaries", func_count),
                        )
                    } else {
                        VerificationResult::fail(
                            aspect.clone(),
                            func_count,
                            func_count.saturating_sub(1),
                            "unbalanced commitment boundary detected".to_string(),
                        )
                    }
                } else {
                    VerificationResult::fail(
                        aspect.clone(),
                        func_count,
                        0,
                        "commitment boundary violation".to_string(),
                    )
                }
            }
            VerificationAspect::ProvenanceCompleteness => {
                let items = module.functions.iter().filter(|f| {
                    f.instructions.iter().any(|i| i.has_side_effects())
                }).count();
                if should_pass || !config.enforce_provenance {
                    VerificationResult::pass(
                        aspect.clone(),
                        items,
                        format!("{} side-effecting functions have provenance", items),
                    )
                } else {
                    VerificationResult::fail(
                        aspect.clone(),
                        items,
                        0,
                        "missing provenance for side-effecting operations".to_string(),
                    )
                }
            }
            VerificationAspect::SafetyFenceOrdering => {
                let safety_funcs = module.functions.iter().filter(|f| f.has_safety_instructions()).count();
                if should_pass || !config.enforce_safety_fences {
                    VerificationResult::pass(
                        aspect.clone(),
                        safety_funcs,
                        format!("{} safety-containing functions verified", safety_funcs),
                    )
                } else {
                    VerificationResult::fail(
                        aspect.clone(),
                        safety_funcs,
                        0,
                        "safety fence ordering violation".to_string(),
                    )
                }
            }
            VerificationAspect::ControlFlowIntegrity => {
                if should_pass {
                    VerificationResult::pass(
                        aspect.clone(),
                        func_count,
                        format!("{} functions have valid control flow", func_count),
                    )
                } else {
                    VerificationResult::fail(
                        aspect.clone(),
                        func_count,
                        func_count.saturating_sub(1),
                        "unreachable code detected".to_string(),
                    )
                }
            }
        }
    }
}

impl WlirVerifier for SimulatedVerifier {
    fn verify(&self, module: &WlirModule, config: &WlirConfig) -> WlirResult<VerificationReport> {
        let results: Vec<VerificationResult> = Self::all_aspects()
            .iter()
            .map(|aspect| self.simulate_aspect(module, aspect, config))
            .collect();

        let report = VerificationReport::from_results(results);

        if !report.all_passed {
            let failures: Vec<String> = report
                .failures()
                .iter()
                .map(|r| format!("{}: {}", r.aspect, r.details))
                .collect();
            return Err(WlirError::VerificationFailed(failures.join("; ")));
        }

        Ok(report)
    }

    fn verify_aspect(
        &self,
        module: &WlirModule,
        aspect: &VerificationAspect,
        config: &WlirConfig,
    ) -> WlirResult<VerificationResult> {
        let result = self.simulate_aspect(module, aspect, config);
        if !result.passed {
            return Err(WlirError::VerificationFailed(format!(
                "{}: {}",
                result.aspect, result.details
            )));
        }
        Ok(result)
    }

    fn name(&self) -> &str {
        "simulated-verifier"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instructions::WlirInstruction;
    use crate::module::{WlirFunction, WlirModule};
    use crate::types::WlirType;

    fn make_module() -> WlirModule {
        let mut module = WlirModule::new("test-module", "1.0.0");
        let mut f = WlirFunction::new("main", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::Nop);
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);
        module
    }

    fn make_config() -> WlirConfig {
        WlirConfig::default()
    }

    #[test]
    fn all_pass_verifier_succeeds() {
        let verifier = SimulatedVerifier::all_pass();
        let module = make_module();
        let config = make_config();
        let report = verifier.verify(&module, &config).unwrap();
        assert!(report.all_passed);
        assert_eq!(report.aspects_checked, 5);
        assert_eq!(report.aspects_passed, 5);
    }

    #[test]
    fn all_fail_verifier_returns_error() {
        let verifier = SimulatedVerifier::all_fail();
        let module = make_module();
        let config = make_config();
        let result = verifier.verify(&module, &config);
        assert!(result.is_err());
    }

    #[test]
    fn single_aspect_failure() {
        let verifier = SimulatedVerifier::failing_aspect(VerificationAspect::TypeCorrectness);
        let module = make_module();
        let config = make_config();
        let result = verifier.verify(&module, &config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("type mismatch"));
    }

    #[test]
    fn verify_single_aspect_pass() {
        let verifier = SimulatedVerifier::all_pass();
        let module = make_module();
        let config = make_config();
        let result = verifier
            .verify_aspect(&module, &VerificationAspect::ControlFlowIntegrity, &config)
            .unwrap();
        assert!(result.passed);
        assert_eq!(result.aspect, VerificationAspect::ControlFlowIntegrity);
    }

    #[test]
    fn verify_single_aspect_fail() {
        let verifier =
            SimulatedVerifier::failing_aspect(VerificationAspect::CommitmentBoundaryIntegrity);
        let module = make_module();
        let config = make_config();
        let result =
            verifier.verify_aspect(&module, &VerificationAspect::CommitmentBoundaryIntegrity, &config);
        assert!(result.is_err());
    }

    #[test]
    fn verification_report_from_results() {
        let results = vec![
            VerificationResult::pass(VerificationAspect::TypeCorrectness, 10, "ok"),
            VerificationResult::fail(VerificationAspect::ProvenanceCompleteness, 5, 3, "missing"),
        ];
        let report = VerificationReport::from_results(results);
        assert!(!report.all_passed);
        assert_eq!(report.aspects_checked, 2);
        assert_eq!(report.aspects_passed, 1);
        assert_eq!(report.failures().len(), 1);
    }

    #[test]
    fn verification_result_display() {
        let r = VerificationResult::pass(VerificationAspect::TypeCorrectness, 10, "all good");
        let display = r.to_string();
        assert!(display.contains("PASS"));
        assert!(display.contains("type-correctness"));
        assert!(display.contains("10/10"));
    }

    #[test]
    fn verification_report_display() {
        let results = vec![
            VerificationResult::pass(VerificationAspect::TypeCorrectness, 10, "ok"),
            VerificationResult::pass(VerificationAspect::ControlFlowIntegrity, 5, "ok"),
        ];
        let report = VerificationReport::from_results(results);
        let display = report.to_string();
        assert!(display.contains("PASS"));
        assert!(display.contains("2/2"));
    }

    #[test]
    fn verification_aspect_display() {
        assert_eq!(
            VerificationAspect::TypeCorrectness.to_string(),
            "type-correctness"
        );
        assert_eq!(
            VerificationAspect::CommitmentBoundaryIntegrity.to_string(),
            "commitment-boundary-integrity"
        );
        assert_eq!(
            VerificationAspect::ProvenanceCompleteness.to_string(),
            "provenance-completeness"
        );
        assert_eq!(
            VerificationAspect::SafetyFenceOrdering.to_string(),
            "safety-fence-ordering"
        );
        assert_eq!(
            VerificationAspect::ControlFlowIntegrity.to_string(),
            "control-flow-integrity"
        );
    }

    #[test]
    fn report_result_for_aspect() {
        let results = vec![
            VerificationResult::pass(VerificationAspect::TypeCorrectness, 10, "ok"),
            VerificationResult::pass(VerificationAspect::ControlFlowIntegrity, 5, "ok"),
        ];
        let report = VerificationReport::from_results(results);
        assert!(report.result_for(&VerificationAspect::TypeCorrectness).is_some());
        assert!(report.result_for(&VerificationAspect::ProvenanceCompleteness).is_none());
    }

    #[test]
    fn verifier_name() {
        let verifier = SimulatedVerifier::all_pass();
        assert_eq!(verifier.name(), "simulated-verifier");
    }

    #[test]
    fn provenance_respects_config() {
        let verifier = SimulatedVerifier::failing_aspect(VerificationAspect::ProvenanceCompleteness);
        let module = make_module();
        let mut config = make_config();
        // When provenance enforcement is disabled, the aspect should pass even if configured to fail
        config.enforce_provenance = false;
        let result = verifier
            .verify_aspect(&module, &VerificationAspect::ProvenanceCompleteness, &config)
            .unwrap();
        assert!(result.passed);
    }
}
