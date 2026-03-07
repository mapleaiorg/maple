//! Directory scaffolding for each `PackageKind`.
//!
//! Creates the conventional directory structure that a package of the given
//! kind is expected to contain.

use std::path::{Path, PathBuf};

use maple_package::PackageKind;

use crate::error::InitError;

/// Create the conventional directory structure for the given package kind.
///
/// Returns the list of directories that were created (relative paths from
/// `base_dir`).  Existing directories are silently skipped.
pub fn scaffold_directories(
    kind: PackageKind,
    base_dir: &Path,
) -> Result<Vec<PathBuf>, InitError> {
    let dir_names = directories_for_kind(&kind);

    let mut created = Vec::with_capacity(dir_names.len());
    for dir_name in &dir_names {
        let full = base_dir.join(dir_name);
        std::fs::create_dir_all(&full).map_err(|e| InitError::Io {
            path: full.clone(),
            source: e,
        })?;
        created.push(PathBuf::from(dir_name));
    }

    Ok(created)
}

/// Return the list of directory names for a given package kind.
fn directories_for_kind(kind: &PackageKind) -> Vec<&'static str> {
    match kind {
        PackageKind::AgentPackage => vec!["prompts", "contracts", "eval"],
        PackageKind::SkillPackage => vec!["src", "schema"],
        PackageKind::ModelPackage => vec!["model", "config"],
        PackageKind::ContractBundle => vec!["contracts"],
        PackageKind::EvalSuite => vec!["tests", "vectors"],
        PackageKind::KnowledgePack => vec!["corpus"],
        PackageKind::PolicyPack => vec!["policies"],
        PackageKind::EvidencePack => vec!["evidence"],
        PackageKind::UiModule => vec!["static"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn all_kinds() -> Vec<PackageKind> {
        vec![
            PackageKind::AgentPackage,
            PackageKind::SkillPackage,
            PackageKind::ModelPackage,
            PackageKind::ContractBundle,
            PackageKind::EvalSuite,
            PackageKind::KnowledgePack,
            PackageKind::PolicyPack,
            PackageKind::EvidencePack,
            PackageKind::UiModule,
        ]
    }

    #[test]
    fn scaffold_creates_expected_directories() {
        for kind in all_kinds() {
            let tmp = TempDir::new().unwrap();
            let dirs = scaffold_directories(kind.clone(), tmp.path()).unwrap();

            let expected = directories_for_kind(&kind);
            assert_eq!(
                dirs.len(),
                expected.len(),
                "Wrong number of dirs for {kind}"
            );

            for dir_name in expected {
                let full = tmp.path().join(dir_name);
                assert!(
                    full.is_dir(),
                    "Directory '{}' was not created for {kind}",
                    dir_name
                );
            }
        }
    }

    #[test]
    fn scaffold_agent_has_prompts_contracts_eval() {
        let tmp = TempDir::new().unwrap();
        let dirs = scaffold_directories(PackageKind::AgentPackage, tmp.path()).unwrap();

        let names: Vec<&str> = dirs.iter().map(|p| p.to_str().unwrap()).collect();
        assert!(names.contains(&"prompts"));
        assert!(names.contains(&"contracts"));
        assert!(names.contains(&"eval"));
    }

    #[test]
    fn scaffold_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let dirs1 = scaffold_directories(PackageKind::AgentPackage, tmp.path()).unwrap();
        let dirs2 = scaffold_directories(PackageKind::AgentPackage, tmp.path()).unwrap();
        assert_eq!(dirs1, dirs2);
    }
}
