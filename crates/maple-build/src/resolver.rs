use crate::graph::{BuildError, BuildGraph, DepEdge, DepNode};
use maple_package::{MapleManifest, PackageKind, PackageName};
use semver::VersionReq;
use tracing::{debug, info, warn};

/// A registry source that can look up available versions of a package.
///
/// Implementations include:
/// - OCI registry client (Phase G)
/// - Local package store
/// - In-memory mock (for testing)
#[async_trait::async_trait]
pub trait PackageSource: Send + Sync {
    /// List available versions for a package (by qualified name).
    async fn list_versions(&self, name: &str) -> Result<Vec<semver::Version>, BuildError>;

    /// Fetch the manifest for a specific version.
    async fn fetch_manifest(
        &self,
        name: &str,
        version: &semver::Version,
    ) -> Result<MapleManifest, BuildError>;

    /// Get the content digest for a specific version.
    async fn get_digest(
        &self,
        name: &str,
        version: &semver::Version,
    ) -> Result<String, BuildError>;
}

/// Resolve all dependencies for a package into a build graph.
///
/// The resolver:
/// 1. Takes the root manifest
/// 2. Resolves all direct dependencies (skills, contracts, models, evals)
/// 3. Recursively resolves transitive dependencies up to `max_depth`
/// 4. Detects version conflicts and cycles
/// 5. Produces a complete, locked dependency graph
pub struct DependencyResolver {
    sources: Vec<Box<dyn PackageSource>>,
    max_depth: usize,
}

impl DependencyResolver {
    /// Create a new resolver with the given package sources.
    ///
    /// Sources are tried in order — the first source that has a matching
    /// version wins (like cargo's registry priority).
    pub fn new(sources: Vec<Box<dyn PackageSource>>) -> Self {
        Self {
            sources,
            max_depth: 10,
        }
    }

    /// Set the maximum transitive resolution depth (default: 10).
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Resolve all transitive dependencies for a manifest.
    pub async fn resolve(&self, manifest: &MapleManifest) -> Result<BuildGraph, BuildError> {
        let root = DepNode {
            name: manifest.name.clone(),
            kind: manifest.kind.clone(),
            version_req: VersionReq::parse(&format!("={}", manifest.version))
                .map_err(|e| BuildError::Unresolvable(format!("root version: {}", e)))?,
            resolved_version: Some(manifest.version.clone()),
            resolved_digest: None,
            optional: false,
        };

        let mut graph = BuildGraph::new(root);
        let root_name = manifest.name.to_qualified();

        info!(root = %root_name, "Starting dependency resolution");

        // Resolve direct skill dependencies
        for skill in &manifest.skills {
            match self
                .resolve_single(
                    &skill.reference,
                    &skill.version,
                    PackageKind::SkillPackage,
                    skill.optional,
                )
                .await
            {
                Ok(dep) => {
                    debug!(
                        dep = %dep.name,
                        version = ?dep.resolved_version,
                        "Resolved skill dependency"
                    );
                    graph.add_dependency(&root_name, dep, DepEdge::Runtime)?;
                }
                Err(e) if skill.optional => {
                    warn!(
                        skill = %skill.reference,
                        error = %e,
                        "Optional skill dependency not resolved"
                    );
                }
                Err(e) => return Err(e),
            }
        }

        // Resolve contract dependencies
        for contract in &manifest.contracts {
            let dep = self
                .resolve_single(
                    &contract.reference,
                    &contract.version,
                    PackageKind::ContractBundle,
                    false,
                )
                .await?;
            debug!(
                dep = %dep.name,
                version = ?dep.resolved_version,
                "Resolved contract dependency"
            );
            graph.add_dependency(&root_name, dep, DepEdge::Runtime)?;
        }

        // Resolve model requirements
        if let Some(ref models) = manifest.models {
            let model_ref = &models.default.reference;
            // Models may be external references (e.g., "openai:gpt-4o")
            // Only resolve if it looks like a package reference
            if model_ref.contains('/') {
                match self
                    .resolve_single(model_ref, "*", PackageKind::ModelPackage, false)
                    .await
                {
                    Ok(dep) => {
                        debug!(model = %model_ref, "Resolved model dependency");
                        graph.add_dependency(&root_name, dep, DepEdge::Model)?;
                    }
                    Err(e) => {
                        warn!(
                            model = %model_ref,
                            error = %e,
                            "Model dependency not resolved (may be external)"
                        );
                    }
                }
            }
        }

        // Resolve eval suite dependencies
        if let Some(ref eval) = manifest.eval {
            for suite in &eval.suites {
                match self
                    .resolve_single(
                        &suite.reference,
                        "*",
                        PackageKind::EvalSuite,
                        !suite.blocking,
                    )
                    .await
                {
                    Ok(dep) => {
                        debug!(eval = %suite.reference, "Resolved eval dependency");
                        graph.add_dependency(&root_name, dep, DepEdge::Eval)?;
                    }
                    Err(e) if !suite.blocking => {
                        warn!(
                            eval = %suite.reference,
                            error = %e,
                            "Non-blocking eval suite not resolved"
                        );
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        // Recursively resolve transitive dependencies
        self.resolve_transitive(&mut graph).await?;

        info!(
            nodes = graph.node_count(),
            edges = graph.edge_count(),
            "Dependency resolution complete"
        );

        Ok(graph)
    }

    /// Resolve a single dependency reference to a concrete version.
    async fn resolve_single(
        &self,
        reference: &str,
        version_constraint: &str,
        expected_kind: PackageKind,
        optional: bool,
    ) -> Result<DepNode, BuildError> {
        // Strip tag/digest from reference for name parsing
        let name_part = reference.split(':').next().unwrap_or(reference);
        let name_part = name_part.split('@').next().unwrap_or(name_part);

        let name = PackageName::parse(name_part)
            .map_err(|e| BuildError::Unresolvable(format!("{}: {}", reference, e)))?;
        let version_req = VersionReq::parse(version_constraint)
            .map_err(|e| BuildError::Unresolvable(format!("{}: {}", reference, e)))?;

        // Try each source in priority order
        for source in &self.sources {
            if let Ok(versions) = source.list_versions(name_part).await {
                // Find the highest version that satisfies the constraint
                let best = versions
                    .iter()
                    .filter(|v| version_req.matches(v))
                    .max()
                    .cloned();

                if let Some(version) = best {
                    let digest = source.get_digest(name_part, &version).await?;
                    return Ok(DepNode {
                        name,
                        kind: expected_kind,
                        version_req,
                        resolved_version: Some(version),
                        resolved_digest: Some(digest),
                        optional,
                    });
                }
            }
        }

        if optional {
            Ok(DepNode {
                name,
                kind: expected_kind,
                version_req,
                resolved_version: None,
                resolved_digest: None,
                optional: true,
            })
        } else {
            Err(BuildError::Unresolvable(reference.to_string()))
        }
    }

    /// Recursively resolve transitive dependencies via BFS.
    ///
    /// Fetches manifests for resolved nodes and adds their dependencies.
    /// Terminates when no new dependencies are discovered or max depth is reached.
    async fn resolve_transitive(&self, graph: &mut BuildGraph) -> Result<(), BuildError> {
        for depth in 0..self.max_depth {
            let resolved: Vec<_> = graph
                .build_order()?
                .into_iter()
                .filter(|n| n.resolved_version.is_some())
                .map(|n| (n.name.to_qualified(), n.resolved_version.clone().unwrap()))
                .collect();

            let mut added = false;
            for (name, version) in &resolved {
                for source in &self.sources {
                    if let Ok(manifest) = source.fetch_manifest(name, version).await {
                        // Add this manifest's skill dependencies
                        for skill in &manifest.skills {
                            let dep_name = skill.reference.split(':').next().unwrap_or(&skill.reference);
                            if graph.contains(dep_name) {
                                continue; // Already resolved
                            }

                            if let Ok(dep) = self
                                .resolve_single(
                                    &skill.reference,
                                    &skill.version,
                                    PackageKind::SkillPackage,
                                    skill.optional,
                                )
                                .await
                            {
                                if graph.add_dependency(name, dep, DepEdge::Runtime).is_ok() {
                                    added = true;
                                }
                            }
                        }

                        // Add contract dependencies
                        for contract in &manifest.contracts {
                            let dep_name = contract.reference.split(':').next().unwrap_or(&contract.reference);
                            if graph.contains(dep_name) {
                                continue;
                            }

                            if let Ok(dep) = self
                                .resolve_single(
                                    &contract.reference,
                                    &contract.version,
                                    PackageKind::ContractBundle,
                                    false,
                                )
                                .await
                            {
                                if graph.add_dependency(name, dep, DepEdge::Runtime).is_ok() {
                                    added = true;
                                }
                            }
                        }
                        break; // Only need first matching source
                    }
                }
            }

            if !added {
                debug!(depth, "Transitive resolution converged");
                break;
            }

            if depth == self.max_depth - 1 {
                return Err(BuildError::MaxDepthExceeded(self.max_depth));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    /// In-memory mock package source for testing
    struct MockSource {
        packages: Arc<Mutex<HashMap<String, Vec<(semver::Version, MapleManifest, String)>>>>,
    }

    impl MockSource {
        fn new() -> Self {
            Self {
                packages: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        async fn add_package(&self, name: &str, version: &str, manifest: MapleManifest) {
            let mut pkgs = self.packages.lock().await;
            let entry = pkgs.entry(name.to_string()).or_default();
            entry.push((
                semver::Version::parse(version).unwrap(),
                manifest,
                format!("sha256:mock_{}", name.replace('/', "_")),
            ));
        }
    }

    #[async_trait::async_trait]
    impl PackageSource for MockSource {
        async fn list_versions(&self, name: &str) -> Result<Vec<semver::Version>, BuildError> {
            let pkgs = self.packages.lock().await;
            Ok(pkgs
                .get(name)
                .map(|entries| entries.iter().map(|(v, _, _)| v.clone()).collect())
                .unwrap_or_default())
        }

        async fn fetch_manifest(
            &self,
            name: &str,
            version: &semver::Version,
        ) -> Result<MapleManifest, BuildError> {
            let pkgs = self.packages.lock().await;
            pkgs.get(name)
                .and_then(|entries| {
                    entries
                        .iter()
                        .find(|(v, _, _)| v == version)
                        .map(|(_, m, _)| m.clone())
                })
                .ok_or_else(|| BuildError::Registry(format!("not found: {}@{}", name, version)))
        }

        async fn get_digest(
            &self,
            name: &str,
            version: &semver::Version,
        ) -> Result<String, BuildError> {
            let pkgs = self.packages.lock().await;
            pkgs.get(name)
                .and_then(|entries| {
                    entries
                        .iter()
                        .find(|(v, _, _)| v == version)
                        .map(|(_, _, d)| d.clone())
                })
                .ok_or_else(|| BuildError::Registry(format!("not found: {}@{}", name, version)))
        }
    }

    fn make_minimal_manifest(name: &str, version: &str) -> MapleManifest {
        maple_package::parser::parse_maplefile_str(
            &format!(
                r#"
api_version: "maple.ai/v1"
kind: skill-package
name: "{}"
version: "{}"
metadata:
  authors: ["test"]
  keywords: []
  labels: {{}}
"#,
                name, version
            ),
            maple_package::ManifestFormat::Yaml,
        )
        .unwrap()
    }

    fn make_agent_manifest_with_skills(
        name: &str,
        version: &str,
        skills: &[(&str, &str)],
    ) -> MapleManifest {
        let skills_yaml: Vec<String> = skills
            .iter()
            .map(|(reference, ver)| {
                format!(
                    r#"  - reference: "{}"
    version: "{}"
    optional: false
    provides: []"#,
                    reference, ver
                )
            })
            .collect();

        let yaml = format!(
            r#"
api_version: "maple.ai/v1"
kind: agent-package
name: "{}"
version: "{}"
metadata:
  authors: ["test"]
  keywords: []
  labels: {{}}
skills:
{}
"#,
            name,
            version,
            skills_yaml.join("\n")
        );

        maple_package::parser::parse_maplefile_str(&yaml, maple_package::ManifestFormat::Yaml)
            .unwrap()
    }

    #[tokio::test]
    async fn test_resolve_no_deps() {
        let source = MockSource::new();
        let resolver = DependencyResolver::new(vec![Box::new(source)]);

        let manifest = make_minimal_manifest("org/skills/simple", "1.0.0");
        let graph = resolver.resolve(&manifest).await.unwrap();

        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.root().name.name, "simple");
    }

    #[tokio::test]
    async fn test_resolve_with_skills() {
        let source = MockSource::new();
        source
            .add_package(
                "org/skills/dep-a",
                "1.0.0",
                make_minimal_manifest("org/skills/dep-a", "1.0.0"),
            )
            .await;
        source
            .add_package(
                "org/skills/dep-b",
                "2.0.0",
                make_minimal_manifest("org/skills/dep-b", "2.0.0"),
            )
            .await;

        let manifest = make_agent_manifest_with_skills(
            "org/agents/root",
            "1.0.0",
            &[("org/skills/dep-a", "^1.0"), ("org/skills/dep-b", "^2.0")],
        );

        let resolver = DependencyResolver::new(vec![Box::new(source)]);
        let graph = resolver.resolve(&manifest).await.unwrap();

        assert_eq!(graph.node_count(), 3); // root + dep-a + dep-b
        assert!(graph.contains("org/skills/dep-a"));
        assert!(graph.contains("org/skills/dep-b"));
    }

    #[tokio::test]
    async fn test_resolve_optional_not_found() {
        let source = MockSource::new();
        let resolver = DependencyResolver::new(vec![Box::new(source)]);

        let yaml = r#"
api_version: "maple.ai/v1"
kind: agent-package
name: "org/agents/root"
version: "1.0.0"
metadata:
  authors: ["test"]
  keywords: []
  labels: {}
skills:
  - reference: "org/skills/missing"
    version: "^1.0"
    optional: true
    provides: []
"#;
        let manifest =
            maple_package::parser::parse_maplefile_str(yaml, maple_package::ManifestFormat::Yaml)
                .unwrap();

        let graph = resolver.resolve(&manifest).await.unwrap();
        // Root + optional unresolved dep (added with None version)
        assert_eq!(graph.node_count(), 2);
        let missing = graph.get_node("org/skills/missing").unwrap();
        assert!(missing.optional);
        assert!(missing.resolved_version.is_none());
    }

    #[tokio::test]
    async fn test_resolve_mandatory_not_found_errors() {
        let source = MockSource::new();
        let resolver = DependencyResolver::new(vec![Box::new(source)]);

        let manifest = make_agent_manifest_with_skills(
            "org/agents/root",
            "1.0.0",
            &[("org/skills/missing", "^1.0")],
        );

        let result = resolver.resolve(&manifest).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_lockfile_roundtrip() {
        let source = MockSource::new();
        source
            .add_package(
                "org/skills/dep",
                "1.2.3",
                make_minimal_manifest("org/skills/dep", "1.2.3"),
            )
            .await;

        let manifest = make_agent_manifest_with_skills(
            "org/agents/root",
            "1.0.0",
            &[("org/skills/dep", "^1.0")],
        );

        let resolver = DependencyResolver::new(vec![Box::new(source)]);
        let graph = resolver.resolve(&manifest).await.unwrap();

        let lockfile = graph.to_lockfile();
        let json = serde_json::to_string_pretty(&lockfile).unwrap();
        let parsed: crate::graph::BuildLockfile = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.entries.len(), lockfile.entries.len());
        assert!(parsed.entries.iter().any(|e| e.name == "org/skills/dep"));
    }
}
