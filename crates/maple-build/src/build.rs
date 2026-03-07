use crate::graph::BuildLockfile;
use maple_package::MapleManifest;
use maple_package_format::{OciManifest, PackageBuilder};
use std::path::{Path, PathBuf};
use tracing::info;

/// Orchestrates the full build process:
///
/// 1. Parse Maplefile
/// 2. Validate manifest
/// 3. Resolve dependencies
/// 4. Assemble OCI layers (deterministic tar+gzip)
/// 5. Compute content hashes
/// 6. Generate OCI manifest
/// 7. Write lockfile
/// 8. Produce final package artifact
pub struct MapleBuildEngine {
    resolver: crate::resolver::DependencyResolver,
}

impl MapleBuildEngine {
    pub fn new(resolver: crate::resolver::DependencyResolver) -> Self {
        Self { resolver }
    }

    /// Build a package from a Maplefile.
    ///
    /// # Arguments
    /// * `maplefile_path` - Path to the Maplefile.yaml
    /// * `output_dir` - Directory to write build artifacts (lockfile, etc.)
    /// * `tag` - OCI tag for the built package
    pub async fn build(
        &self,
        maplefile_path: &Path,
        output_dir: &Path,
        tag: &str,
    ) -> Result<BuildResult, BuildEngineError> {
        std::fs::create_dir_all(output_dir)?;

        // 1. Parse and validate manifest
        info!(path = %maplefile_path.display(), "Parsing Maplefile");
        let manifest = maple_package::parse_maplefile(maplefile_path)
            .map_err(BuildEngineError::Parse)?;

        let validation = maple_package::validate_manifest(&manifest);
        if !validation.is_valid() {
            return Err(BuildEngineError::Validation(
                validation
                    .errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }

        // Log warnings
        for warning in &validation.warnings {
            tracing::warn!("{}", warning);
        }

        // 2. Resolve dependencies
        info!("Resolving dependencies...");
        let graph = self
            .resolver
            .resolve(&manifest)
            .await
            .map_err(BuildEngineError::Resolution)?;
        let lockfile = graph.to_lockfile();

        // 3. Build package layers
        info!("Assembling package layers...");
        let work_dir = tempfile::tempdir()?;
        let mut builder = PackageBuilder::new(work_dir.path())?;

        // Add kind-specific layers from the source directory
        let source_dir = maplefile_path
            .parent()
            .unwrap_or(Path::new("."));
        self.add_kind_specific_layers(&mut builder, &manifest, source_dir)?;

        // 4. Build OCI manifest
        let (oci_manifest, artifact_dir) = builder
            .build(&manifest)
            .map_err(BuildEngineError::Format)?;

        // 5. Write lockfile
        let lockfile_path = output_dir.join("maple.lock");
        let lockfile_json = serde_json::to_vec_pretty(&lockfile)?;
        std::fs::write(&lockfile_path, &lockfile_json)?;
        info!(lockfile = %lockfile_path.display(), "Lockfile written");

        // 6. Populate build provenance
        let provenance = maple_package::BuildProvenance {
            digest: oci_manifest.config.digest.clone(),
            built_at: chrono::Utc::now(),
            builder: None,
            source: detect_source_reference(source_dir),
            resolved_deps: lockfile
                .entries
                .iter()
                .map(|e| maple_package::ResolvedDependency {
                    name: e.name.clone(),
                    version: e.version.clone(),
                    digest: e.digest.clone(),
                    kind: e.kind.clone(),
                })
                .collect(),
            worldline_event: None,
        };

        info!(
            tag,
            digest = %oci_manifest.config.digest,
            layers = oci_manifest.layers.len(),
            "Package built successfully"
        );

        Ok(BuildResult {
            manifest: oci_manifest,
            lockfile,
            provenance,
            tag: tag.to_string(),
            artifact_dir,
        })
    }

    /// Add kind-specific layers based on the package type.
    fn add_kind_specific_layers(
        &self,
        builder: &mut PackageBuilder,
        manifest: &MapleManifest,
        source_dir: &Path,
    ) -> Result<(), BuildEngineError> {
        let dirs_to_layer = maple_package_format::media_types_for_kind(&manifest.kind);

        for (dir_name, media_type) in dirs_to_layer {
            let dir_path = source_dir.join(dir_name);
            if dir_path.exists() && dir_path.is_dir() {
                builder
                    .add_layer(
                        &dir_path,
                        media_type,
                        [("org.maple.layer.type".into(), dir_name.into())].into(),
                    )
                    .map_err(BuildEngineError::Format)?;
                info!(layer = dir_name, "Added layer");
            }
        }

        Ok(())
    }
}

/// Attempt to detect git source reference from the build directory.
fn detect_source_reference(source_dir: &Path) -> Option<maple_package::SourceReference> {
    // Walk up to find .git directory
    let mut dir = source_dir.to_path_buf();
    loop {
        let git_dir = dir.join(".git");
        if git_dir.exists() {
            // Read HEAD for commit hash
            let head_path = git_dir.join("HEAD");
            if let Ok(head_content) = std::fs::read_to_string(&head_path) {
                let head = head_content.trim();

                // Check if HEAD is a symbolic ref
                let (commit, branch) = if head.starts_with("ref: ") {
                    let ref_path = head.strip_prefix("ref: ").unwrap();
                    let branch = ref_path
                        .strip_prefix("refs/heads/")
                        .unwrap_or(ref_path)
                        .to_string();
                    let commit_path = git_dir.join(ref_path);
                    let commit = std::fs::read_to_string(&commit_path)
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();
                    (commit, Some(branch))
                } else {
                    (head.to_string(), None)
                };

                // Detect repository URL from config
                let config_path = git_dir.join("config");
                let repository = std::fs::read_to_string(&config_path)
                    .ok()
                    .and_then(|config| {
                        config
                            .lines()
                            .skip_while(|l| !l.contains("[remote \"origin\"]"))
                            .nth(1)
                            .and_then(|l| {
                                l.trim()
                                    .strip_prefix("url = ")
                                    .map(|s| s.to_string())
                            })
                    })
                    .unwrap_or_else(|| "unknown".to_string());

                return Some(maple_package::SourceReference {
                    repository,
                    commit,
                    branch,
                    dirty: false, // TODO: detect dirty state via git status
                });
            }
        }

        if !dir.pop() {
            break;
        }
    }

    None
}

/// The result of a successful build.
#[derive(Debug)]
pub struct BuildResult {
    /// The OCI manifest for the built package
    pub manifest: OciManifest,
    /// The resolved dependency lockfile
    pub lockfile: BuildLockfile,
    /// Build provenance (for signing and auditing)
    pub provenance: maple_package::BuildProvenance,
    /// OCI tag assigned to this build
    pub tag: String,
    /// Path to the artifact directory containing blobs
    pub artifact_dir: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum BuildEngineError {
    #[error("Parse error: {0}")]
    Parse(#[source] maple_package::ParseError),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Resolution error: {0}")]
    Resolution(#[source] crate::graph::BuildError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Package format error: {0}")]
    Format(#[from] maple_package_format::PackageFormatError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_source_reference() {
        // Should work in the maple repo itself
        let ref_opt = detect_source_reference(Path::new("/Users/wenyan/ClaudeProjects/maple"));
        // May or may not find git info depending on environment
        if let Some(source_ref) = ref_opt {
            assert!(!source_ref.commit.is_empty());
        }
    }
}
