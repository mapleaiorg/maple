//! Governance tier classification engine.
//!
//! Determines the appropriate [`GovernanceTier`] for a change based on its
//! description, the file paths it affects, and the number of lines changed.

use maple_waf_context_graph::GovernanceTier;

/// Classifies changes into governance tiers based on scope and risk.
pub struct GovernanceTierEngine;

impl GovernanceTierEngine {
    /// Classify a change into a [`GovernanceTier`].
    ///
    /// Classification rules (evaluated in priority order):
    /// - **Tier4**: more than 1000 lines changed, or any affected path contains `"kernel"`.
    /// - **Tier3**: more than 500 lines changed, or any affected path contains `"compiler"`.
    /// - **Tier2**: more than 100 lines changed.
    /// - **Tier1**: more than 10 lines changed.
    /// - **Tier0**: 10 or fewer lines changed with no high-risk paths.
    pub fn classify_change(
        description: &str,
        affected_paths: &[String],
        lines_changed: usize,
    ) -> GovernanceTier {
        let _ = description; // reserved for future heuristic use

        let has_kernel = affected_paths.iter().any(|p| p.contains("kernel"));
        let has_compiler = affected_paths.iter().any(|p| p.contains("compiler"));

        if lines_changed > 1000 || has_kernel {
            GovernanceTier::Tier4
        } else if lines_changed > 500 || has_compiler {
            GovernanceTier::Tier3
        } else if lines_changed > 100 {
            GovernanceTier::Tier2
        } else if lines_changed > 10 {
            GovernanceTier::Tier1
        } else {
            GovernanceTier::Tier0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier0_small_change() {
        let tier = GovernanceTierEngine::classify_change("fix typo", &["src/lib.rs".into()], 5);
        assert_eq!(tier, GovernanceTier::Tier0);
    }

    #[test]
    fn tier1_moderate_change() {
        let tier =
            GovernanceTierEngine::classify_change("refactor helper", &["src/utils.rs".into()], 50);
        assert_eq!(tier, GovernanceTier::Tier1);
    }

    #[test]
    fn tier2_large_change() {
        let tier = GovernanceTierEngine::classify_change(
            "add new module",
            &["src/new_module.rs".into()],
            200,
        );
        assert_eq!(tier, GovernanceTier::Tier2);
    }

    #[test]
    fn tier3_compiler_path() {
        let tier = GovernanceTierEngine::classify_change(
            "update compiler pass",
            &["src/compiler/passes/optimize.rs".into()],
            15,
        );
        assert_eq!(tier, GovernanceTier::Tier3);
    }

    #[test]
    fn tier3_over_500_lines() {
        let tier =
            GovernanceTierEngine::classify_change("big refactor", &["src/lib.rs".into()], 750);
        assert_eq!(tier, GovernanceTier::Tier3);
    }

    #[test]
    fn tier4_kernel_path() {
        let tier = GovernanceTierEngine::classify_change(
            "kernel gate update",
            &["crates/maple-kernel-gate/src/gate.rs".into()],
            20,
        );
        assert_eq!(tier, GovernanceTier::Tier4);
    }
}
