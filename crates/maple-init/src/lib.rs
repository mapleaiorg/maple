//! # maple-init
//!
//! Maplefile template generator and project scaffolding for MAPLE packages.
//!
//! This crate provides:
//!
//! - **Template generation** — produces a complete `Maplefile.yaml` for any
//!   [`PackageKind`], pre-populated with sensible defaults and documented
//!   with YAML comments.
//! - **Directory scaffolding** — creates the conventional directory structure
//!   that each package kind is expected to contain.
//! - **One-shot initialisation** — [`init_package`] combines both steps into
//!   a single call, writing `Maplefile.yaml` and creating directories under
//!   the given base path.

pub mod error;
pub mod scaffold;
pub mod templates;

pub use error::InitError;
pub use scaffold::scaffold_directories;
pub use templates::generate_template;

use std::path::{Path, PathBuf};

use maple_package::PackageKind;

/// Initialise a new MAPLE package on disk.
///
/// 1. Generates a `Maplefile.yaml` from the template for `kind`.
/// 2. Creates the conventional directory scaffold under `base_dir`.
///
/// Returns the list of paths that were created (the manifest file followed
/// by all scaffold directories).
pub fn init_package(
    kind: PackageKind,
    name: &str,
    org: &str,
    base_dir: &Path,
) -> Result<Vec<PathBuf>, InitError> {
    // Ensure the base directory exists.
    std::fs::create_dir_all(base_dir).map_err(|e| InitError::Io {
        path: base_dir.to_path_buf(),
        source: e,
    })?;

    // 1. Generate and write the Maplefile template.
    let template = generate_template(kind.clone(), name, org);
    let manifest_path = base_dir.join("Maplefile.yaml");
    std::fs::write(&manifest_path, &template).map_err(|e| InitError::Io {
        path: manifest_path.clone(),
        source: e,
    })?;

    // 2. Create scaffold directories.
    let dirs = scaffold_directories(kind, base_dir)?;

    // Build result: manifest first, then directories.
    let mut created = Vec::with_capacity(1 + dirs.len());
    created.push(PathBuf::from("Maplefile.yaml"));
    created.extend(dirs);

    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_package::{parse_maplefile_str, validate_manifest, ManifestFormat};
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
    fn init_package_creates_maplefile_and_directories() {
        for kind in all_kinds() {
            let tmp = TempDir::new().unwrap();
            let created = init_package(kind.clone(), "demo", "testorg", tmp.path())
                .unwrap_or_else(|e| panic!("init_package failed for {kind}: {e}"));

            // Maplefile.yaml must exist.
            let maplefile = tmp.path().join("Maplefile.yaml");
            assert!(maplefile.is_file(), "Maplefile.yaml missing for {kind}");

            // The first entry must be the manifest.
            assert_eq!(
                created[0],
                PathBuf::from("Maplefile.yaml"),
                "First created path should be Maplefile.yaml for {kind}"
            );

            // Must have created at least one directory.
            assert!(
                created.len() > 1,
                "Expected scaffold directories for {kind}"
            );

            // All scaffold directories must exist on disk.
            for dir in &created[1..] {
                let full = tmp.path().join(dir);
                assert!(
                    full.is_dir(),
                    "Scaffold directory {dir:?} missing on disk for {kind}"
                );
            }
        }
    }

    #[test]
    fn init_package_produces_parseable_maplefile() {
        for kind in all_kinds() {
            let tmp = TempDir::new().unwrap();
            init_package(kind.clone(), "demo", "testorg", tmp.path()).unwrap();

            let content = std::fs::read_to_string(tmp.path().join("Maplefile.yaml")).unwrap();
            let manifest = parse_maplefile_str(&content, ManifestFormat::Yaml)
                .unwrap_or_else(|e| panic!("Written Maplefile for {kind} failed to parse: {e}"));

            let result = validate_manifest(&manifest);
            assert!(
                result.is_valid(),
                "Written Maplefile for {kind} has validation errors: {:?}",
                result.errors
            );
        }
    }
}
