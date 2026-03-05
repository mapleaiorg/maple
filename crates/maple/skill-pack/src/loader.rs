//! Skill pack filesystem loader.
//!
//! Discovers and loads skill packs from directories on disk.

use std::path::{Path, PathBuf};

use crate::{
    golden::GoldenTrace,
    manifest::SkillManifest,
    policy::SkillPolicyFile,
    SkillError, SkillPack,
};

/// Loads skill packs from the filesystem.
///
/// A valid skill pack directory must contain at minimum a `manifest.toml`.
/// Optional files: `policy.toml`, `tests/golden/*.json`.
#[derive(Debug, Clone)]
pub struct SkillPackLoader {
    /// Directories to search for skill packs.
    search_paths: Vec<PathBuf>,
}

impl SkillPackLoader {
    /// Create a loader with the given search paths.
    pub fn new(search_paths: Vec<PathBuf>) -> Self {
        Self { search_paths }
    }

    /// Add a search path.
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    /// Load a single skill pack from a directory.
    pub fn load(&self, path: &Path) -> Result<SkillPack, SkillError> {
        let manifest_path = path.join("manifest.toml");
        if !manifest_path.exists() {
            return Err(SkillError::MissingFile(format!(
                "manifest.toml not found in {}",
                path.display()
            )));
        }

        // Parse manifest
        let manifest_str = std::fs::read_to_string(&manifest_path)?;
        let manifest = SkillManifest::from_toml(&manifest_str)?;

        // Parse policies (optional)
        let policies = {
            let policy_path = path.join("policy.toml");
            if policy_path.exists() {
                let policy_str = std::fs::read_to_string(&policy_path)?;
                let file = SkillPolicyFile::from_toml(&policy_str)?;
                file.policies
            } else {
                Vec::new()
            }
        };

        // Load golden traces (optional)
        let golden_traces = self.load_golden_traces(path)?;

        let pack = SkillPack {
            manifest,
            policies,
            golden_traces,
            source_path: Some(path.to_path_buf()),
        };

        // Validate
        pack.validate()?;

        tracing::info!(
            skill = %pack.name(),
            version = %pack.version(),
            policies = pack.policies.len(),
            golden_traces = pack.golden_traces.len(),
            "loaded skill pack"
        );

        Ok(pack)
    }

    /// Load all skill packs found in the configured search paths.
    ///
    /// Each immediate subdirectory containing a `manifest.toml` is treated
    /// as a skill pack.
    pub fn load_all(&self) -> Result<Vec<SkillPack>, SkillError> {
        let mut packs = Vec::new();

        for search_path in &self.search_paths {
            if !search_path.exists() {
                tracing::warn!(path = %search_path.display(), "search path does not exist, skipping");
                continue;
            }

            let entries = std::fs::read_dir(search_path)?;
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() && path.join("manifest.toml").exists() {
                    match self.load(&path) {
                        Ok(pack) => packs.push(pack),
                        Err(e) => {
                            tracing::warn!(
                                path = %path.display(),
                                error = %e,
                                "failed to load skill pack, skipping"
                            );
                        }
                    }
                }
            }
        }

        tracing::info!(count = packs.len(), "loaded all skill packs");
        Ok(packs)
    }

    /// Load golden trace files from `tests/golden/` subdirectory.
    fn load_golden_traces(&self, pack_dir: &Path) -> Result<Vec<GoldenTrace>, SkillError> {
        let golden_dir = pack_dir.join("tests").join("golden");
        if !golden_dir.exists() {
            return Ok(Vec::new());
        }

        let mut traces = Vec::new();
        let entries = std::fs::read_dir(&golden_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let content = std::fs::read_to_string(&path)?;
                // Try parsing as array first, then as single trace
                match GoldenTrace::from_json_array(&content) {
                    Ok(mut array_traces) => traces.append(&mut array_traces),
                    Err(_) => {
                        let trace = GoldenTrace::from_json(&content)?;
                        traces.push(trace);
                    }
                }
            }
        }

        Ok(traces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_skill_dir(dir: &Path) {
        fs::create_dir_all(dir).unwrap();

        fs::write(
            dir.join("manifest.toml"),
            r#"
[skill]
name = "test-loader-skill"
version = "0.1.0"
description = "A skill for testing the loader"

[inputs.query]
type = "string"
required = true
description = "Test input"

[outputs.result]
type = "string"
description = "Test output"

[capabilities]
required = ["cap-test"]

[resources]
max_compute_ms = 1000
max_memory_bytes = 1000000
max_network_bytes = 0

[sandbox]
type = "process"
timeout_ms = 5000
"#,
        )
        .unwrap();

        fs::write(
            dir.join("policy.toml"),
            r#"
[[policies]]
name = "rate-limit"
effect = "deny"
reason = "Rate limited"

[policies.condition]
type = "rate_exceeds"
resource = "test"
max_per_minute = 10
"#,
        )
        .unwrap();

        // Golden traces
        let golden_dir = dir.join("tests").join("golden");
        fs::create_dir_all(&golden_dir).unwrap();
        fs::write(
            golden_dir.join("basic.json"),
            r#"[
                {
                    "name": "basic-test",
                    "input": {"query": "hello"},
                    "expected_output": {"result": "world"}
                }
            ]"#,
        )
        .unwrap();
    }

    #[test]
    fn load_skill_pack_from_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path().join("test-skill");
        create_test_skill_dir(&skill_dir);

        let loader = SkillPackLoader::new(vec![]);
        let pack = loader.load(&skill_dir).unwrap();

        assert_eq!(pack.name(), "test-loader-skill");
        assert_eq!(pack.version(), &semver::Version::new(0, 1, 0));
        assert_eq!(pack.policies.len(), 1);
        assert_eq!(pack.golden_traces.len(), 1);
        assert_eq!(pack.golden_traces[0].name, "basic-test");
    }

    #[test]
    fn load_all_from_search_path() {
        let tmp = tempfile::tempdir().unwrap();
        let search = tmp.path().join("skills");
        fs::create_dir_all(&search).unwrap();

        // Create two skills
        create_test_skill_dir(&search.join("skill-a"));
        create_test_skill_dir(&search.join("skill-b"));

        // Create a non-skill directory (no manifest)
        fs::create_dir_all(search.join("not-a-skill")).unwrap();

        let loader = SkillPackLoader::new(vec![search]);
        let packs = loader.load_all().unwrap();
        assert_eq!(packs.len(), 2);
    }

    #[test]
    fn load_missing_manifest_error() {
        let tmp = tempfile::tempdir().unwrap();
        let loader = SkillPackLoader::new(vec![]);
        let result = loader.load(tmp.path());
        assert!(result.is_err());
        match result.unwrap_err() {
            SkillError::MissingFile(_) => {}
            other => panic!("expected MissingFile, got {other}"),
        }
    }

    #[test]
    fn load_nonexistent_search_path_skipped() {
        let loader = SkillPackLoader::new(vec![PathBuf::from("/nonexistent/path")]);
        let packs = loader.load_all().unwrap();
        assert!(packs.is_empty());
    }
}
