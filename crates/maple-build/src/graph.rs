use petgraph::graph::DiGraph;
use petgraph::visit::EdgeRef;
use maple_package::{PackageKind, PackageName};
use semver::{Version, VersionReq};
use std::collections::HashMap;

/// A node in the dependency graph.
///
/// Each node represents a single package at a specific resolved version.
/// Unresolved optional dependencies have `resolved_version = None`.
#[derive(Debug, Clone)]
pub struct DepNode {
    /// Fully qualified package name
    pub name: PackageName,
    /// Kind of package
    pub kind: PackageKind,
    /// Semver version constraint from the parent
    pub version_req: VersionReq,
    /// Resolved concrete version (None if optional and unresolved)
    pub resolved_version: Option<Version>,
    /// Content-addressed digest of the resolved artifact
    pub resolved_digest: Option<String>,
    /// Whether this dependency is optional
    pub optional: bool,
}

/// An edge in the dependency graph, representing the nature of the dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepEdge {
    /// Runtime dependency (required for execution)
    Runtime,
    /// Build dependency (required for packaging only)
    Build,
    /// Eval dependency (required for testing only)
    Eval,
    /// Model dependency (required model backend)
    Model,
}

impl std::fmt::Display for DepEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runtime => write!(f, "runtime"),
            Self::Build => write!(f, "build"),
            Self::Eval => write!(f, "eval"),
            Self::Model => write!(f, "model"),
        }
    }
}

/// The resolved dependency graph — a DAG of packages.
///
/// Uses `petgraph::DiGraph` internally for efficient graph operations:
/// - Topological sort for build ordering
/// - Cycle detection
/// - Traversal for lockfile generation
pub struct BuildGraph {
    graph: DiGraph<DepNode, DepEdge>,
    root: petgraph::graph::NodeIndex,
    /// Map from qualified package name to node index for O(1) lookup
    name_index: HashMap<String, petgraph::graph::NodeIndex>,
}

impl BuildGraph {
    /// Create a new build graph with the root package.
    pub fn new(root: DepNode) -> Self {
        let mut graph = DiGraph::new();
        let root_key = root.name.to_qualified();
        let root_idx = graph.add_node(root);
        let mut name_index = HashMap::new();
        name_index.insert(root_key, root_idx);
        Self {
            graph,
            root: root_idx,
            name_index,
        }
    }

    /// Add a dependency edge from `parent` to `child`.
    ///
    /// If the child already exists in the graph, verifies version compatibility.
    /// Detects and rejects cyclic dependencies.
    pub fn add_dependency(
        &mut self,
        parent: &str,
        child: DepNode,
        edge: DepEdge,
    ) -> Result<(), BuildError> {
        let parent_idx = *self
            .name_index
            .get(parent)
            .ok_or_else(|| BuildError::UnknownParent(parent.to_string()))?;

        let child_key = child.name.to_qualified();
        let child_idx = if let Some(&idx) = self.name_index.get(&child_key) {
            // Node already exists — check for version conflicts
            let existing = &self.graph[idx];
            if let (Some(v1), Some(v2)) = (&existing.resolved_version, &child.resolved_version) {
                if v1 != v2 {
                    return Err(BuildError::VersionConflict {
                        package: child_key,
                        existing: v1.clone(),
                        requested: v2.clone(),
                    });
                }
            }
            idx
        } else {
            let idx = self.graph.add_node(child);
            self.name_index.insert(child_key, idx);
            idx
        };

        self.graph.add_edge(parent_idx, child_idx, edge);

        // Check for cycles after adding the edge
        if petgraph::algo::is_cyclic_directed(&self.graph) {
            // Remove the edge we just added to restore a valid state
            let edges: Vec<_> = self
                .graph
                .edges_connecting(parent_idx, child_idx)
                .map(|e| e.id())
                .collect();
            if let Some(last) = edges.last() {
                self.graph.remove_edge(*last);
            }
            return Err(BuildError::CyclicDependency);
        }

        Ok(())
    }

    /// Get topological order for build (leaves first, root last).
    ///
    /// Returns packages in the order they should be built:
    /// dependencies before their dependents.
    pub fn build_order(&self) -> Result<Vec<&DepNode>, BuildError> {
        let order = petgraph::algo::toposort(&self.graph, None)
            .map_err(|_| BuildError::CyclicDependency)?;
        // Reverse: toposort gives root-first, we want leaves-first for building
        Ok(order.into_iter().rev().map(|idx| &self.graph[idx]).collect())
    }

    /// Check if a package exists in the graph
    pub fn contains(&self, name: &str) -> bool {
        self.name_index.contains_key(name)
    }

    /// Get a node by its qualified name
    pub fn get_node(&self, name: &str) -> Option<&DepNode> {
        self.name_index
            .get(name)
            .map(|&idx| &self.graph[idx])
    }

    /// Get the root node
    pub fn root(&self) -> &DepNode {
        &self.graph[self.root]
    }

    /// Total number of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Total number of edges in the graph
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Get all direct dependencies of a package
    pub fn dependencies_of(&self, name: &str) -> Vec<(&DepNode, &DepEdge)> {
        if let Some(&idx) = self.name_index.get(name) {
            self.graph
                .edges(idx)
                .map(|edge| (&self.graph[edge.target()], edge.weight()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Generate a lockfile-compatible representation.
    ///
    /// The lockfile captures exact resolved versions and their dependency
    /// relationships for reproducible builds.
    pub fn to_lockfile(&self) -> BuildLockfile {
        let entries: Vec<LockfileEntry> = self
            .graph
            .node_indices()
            .map(|idx| {
                let node = &self.graph[idx];
                LockfileEntry {
                    name: node.name.to_qualified(),
                    kind: node.kind.clone(),
                    version: node
                        .resolved_version
                        .clone()
                        .unwrap_or_else(|| Version::new(0, 0, 0)),
                    digest: node.resolved_digest.clone().unwrap_or_default(),
                    dependencies: self
                        .graph
                        .neighbors(idx)
                        .map(|dep_idx| self.graph[dep_idx].name.to_qualified())
                        .collect(),
                }
            })
            .collect();

        BuildLockfile {
            schema_version: 1,
            entries,
        }
    }
}

/// Serializable lockfile for reproducible builds.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BuildLockfile {
    /// Lockfile schema version
    pub schema_version: u32,
    /// All resolved packages
    pub entries: Vec<LockfileEntry>,
}

/// A single entry in the lockfile.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LockfileEntry {
    /// Fully qualified package name
    pub name: String,
    /// Package kind
    pub kind: PackageKind,
    /// Exact resolved version
    pub version: Version,
    /// Content digest
    pub digest: String,
    /// Direct dependencies (by qualified name)
    pub dependencies: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("Unknown parent package: {0}")]
    UnknownParent(String),

    #[error("Version conflict for {package}: existing {existing}, requested {requested}")]
    VersionConflict {
        package: String,
        existing: Version,
        requested: Version,
    },

    #[error("Cyclic dependency detected")]
    CyclicDependency,

    #[error("Unresolvable dependency: {0}")]
    Unresolvable(String),

    #[error("Registry error: {0}")]
    Registry(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Maximum resolution depth exceeded (limit: {0})")]
    MaxDepthExceeded(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(name: &str, version: Option<&str>) -> DepNode {
        DepNode {
            name: PackageName::parse(name).unwrap(),
            kind: PackageKind::SkillPackage,
            version_req: VersionReq::STAR,
            resolved_version: version.map(|v| Version::parse(v).unwrap()),
            resolved_digest: version.map(|_| "sha256:abc123".to_string()),
            optional: false,
        }
    }

    #[test]
    fn test_single_node_graph() {
        let root = make_node("org/agents/root", Some("1.0.0"));
        let graph = BuildGraph::new(root);
        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.edge_count(), 0);
        assert_eq!(graph.root().name.name, "root");
    }

    #[test]
    fn test_three_skill_deps() {
        let root = make_node("org/agents/root", Some("1.0.0"));
        let mut graph = BuildGraph::new(root);

        graph
            .add_dependency(
                "org/agents/root",
                make_node("org/skills/a", Some("1.0.0")),
                DepEdge::Runtime,
            )
            .unwrap();
        graph
            .add_dependency(
                "org/agents/root",
                make_node("org/skills/b", Some("2.0.0")),
                DepEdge::Runtime,
            )
            .unwrap();
        graph
            .add_dependency(
                "org/agents/root",
                make_node("org/skills/c", Some("3.0.0")),
                DepEdge::Eval,
            )
            .unwrap();

        assert_eq!(graph.node_count(), 4);
        assert_eq!(graph.edge_count(), 3);
    }

    #[test]
    fn test_version_conflict_detected() {
        let root = make_node("org/agents/root", Some("1.0.0"));
        let mut graph = BuildGraph::new(root);

        graph
            .add_dependency(
                "org/agents/root",
                make_node("org/skills/a", Some("1.0.0")),
                DepEdge::Runtime,
            )
            .unwrap();

        // Adding same package with different version should fail
        let result = graph.add_dependency(
            "org/agents/root",
            make_node("org/skills/a", Some("2.0.0")),
            DepEdge::Runtime,
        );
        assert!(matches!(result, Err(BuildError::VersionConflict { .. })));
    }

    #[test]
    fn test_cyclic_dependency_detected() {
        let root = make_node("org/agents/root", Some("1.0.0"));
        let mut graph = BuildGraph::new(root);

        graph
            .add_dependency(
                "org/agents/root",
                make_node("org/skills/a", Some("1.0.0")),
                DepEdge::Runtime,
            )
            .unwrap();
        graph
            .add_dependency(
                "org/skills/a",
                make_node("org/skills/b", Some("1.0.0")),
                DepEdge::Runtime,
            )
            .unwrap();

        // b → root would create a cycle
        let result = graph.add_dependency(
            "org/skills/b",
            make_node("org/agents/root", Some("1.0.0")),
            DepEdge::Runtime,
        );
        assert!(matches!(result, Err(BuildError::CyclicDependency)));

        // Graph should still be valid after rejected cycle
        assert!(graph.build_order().is_ok());
    }

    #[test]
    fn test_topological_build_order() {
        let root = make_node("org/agents/root", Some("1.0.0"));
        let mut graph = BuildGraph::new(root);

        graph
            .add_dependency(
                "org/agents/root",
                make_node("org/skills/a", Some("1.0.0")),
                DepEdge::Runtime,
            )
            .unwrap();
        graph
            .add_dependency(
                "org/skills/a",
                make_node("org/skills/b", Some("1.0.0")),
                DepEdge::Runtime,
            )
            .unwrap();

        let order = graph.build_order().unwrap();
        let names: Vec<_> = order.iter().map(|n| n.name.name.as_str()).collect();

        // b should come before a, and a before root (leaves first)
        let b_pos = names.iter().position(|n| *n == "b").unwrap();
        let a_pos = names.iter().position(|n| *n == "a").unwrap();
        let root_pos = names.iter().position(|n| *n == "root").unwrap();
        assert!(b_pos < a_pos);
        assert!(a_pos < root_pos);
    }

    #[test]
    fn test_lockfile_generation() {
        let root = make_node("org/agents/root", Some("1.0.0"));
        let mut graph = BuildGraph::new(root);

        graph
            .add_dependency(
                "org/agents/root",
                make_node("org/skills/a", Some("1.0.0")),
                DepEdge::Runtime,
            )
            .unwrap();

        let lockfile = graph.to_lockfile();
        assert_eq!(lockfile.schema_version, 1);
        assert_eq!(lockfile.entries.len(), 2);

        let root_entry = lockfile
            .entries
            .iter()
            .find(|e| e.name == "org/agents/root")
            .unwrap();
        assert_eq!(root_entry.version, Version::new(1, 0, 0));
        assert_eq!(root_entry.dependencies.len(), 1);

        // Verify lockfile serializes to JSON
        let json = serde_json::to_string_pretty(&lockfile).unwrap();
        assert!(json.contains("org/agents/root"));
    }

    #[test]
    fn test_optional_unresolved_dep() {
        let root = make_node("org/agents/root", Some("1.0.0"));
        let mut graph = BuildGraph::new(root);

        let optional_node = DepNode {
            name: PackageName::parse("org/skills/optional").unwrap(),
            kind: PackageKind::SkillPackage,
            version_req: VersionReq::STAR,
            resolved_version: None,
            resolved_digest: None,
            optional: true,
        };

        graph
            .add_dependency("org/agents/root", optional_node, DepEdge::Runtime)
            .unwrap();

        assert_eq!(graph.node_count(), 2);
        let opt = graph.get_node("org/skills/optional").unwrap();
        assert!(opt.optional);
        assert!(opt.resolved_version.is_none());
    }

    #[test]
    fn test_contains_and_get_node() {
        let root = make_node("org/agents/root", Some("1.0.0"));
        let graph = BuildGraph::new(root);

        assert!(graph.contains("org/agents/root"));
        assert!(!graph.contains("org/skills/nonexistent"));

        let node = graph.get_node("org/agents/root").unwrap();
        assert_eq!(node.name.name, "root");
    }

    #[test]
    fn test_dependencies_of() {
        let root = make_node("org/agents/root", Some("1.0.0"));
        let mut graph = BuildGraph::new(root);

        graph
            .add_dependency(
                "org/agents/root",
                make_node("org/skills/a", Some("1.0.0")),
                DepEdge::Runtime,
            )
            .unwrap();
        graph
            .add_dependency(
                "org/agents/root",
                make_node("org/skills/b", Some("1.0.0")),
                DepEdge::Eval,
            )
            .unwrap();

        let deps = graph.dependencies_of("org/agents/root");
        assert_eq!(deps.len(), 2);
        assert!(deps.iter().any(|(n, e)| n.name.name == "a" && **e == DepEdge::Runtime));
        assert!(deps.iter().any(|(n, e)| n.name.name == "b" && **e == DepEdge::Eval));
    }

    #[test]
    fn test_unknown_parent_error() {
        let root = make_node("org/agents/root", Some("1.0.0"));
        let mut graph = BuildGraph::new(root);

        let result = graph.add_dependency(
            "org/nonexistent/parent",
            make_node("org/skills/a", Some("1.0.0")),
            DepEdge::Runtime,
        );
        assert!(matches!(result, Err(BuildError::UnknownParent(_))));
    }
}
