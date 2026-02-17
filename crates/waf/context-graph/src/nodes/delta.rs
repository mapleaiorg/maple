use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The implementation change — substrate type, binary diff, synthesis params.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeltaNode {
    /// Target substrate for this change.
    pub substrate_type: SubstrateType,
    /// Optional compatibility link to git ref (not authoritative).
    pub git_ref_fallback: Option<String>,
    /// The synthesized binary diff or IR blob.
    /// Content-addressed — hash MUST match reproducible build.
    pub binary_diff: Vec<u8>,
    /// Synthesis parameters used to produce this delta.
    pub synthesis_params: HashMap<String, String>,
    /// List of affected modules/files.
    pub affected_paths: Vec<String>,
    /// Size metrics for governance review.
    pub size_metrics: DeltaSizeMetrics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubstrateType {
    Rust,
    Wlir,
    Cuda,
    Wasm,
    Metal,
    Verilog,
}

impl std::fmt::Display for SubstrateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rust => write!(f, "Rust"),
            Self::Wlir => write!(f, "WLIR"),
            Self::Cuda => write!(f, "CUDA"),
            Self::Wasm => write!(f, "WebAssembly"),
            Self::Metal => write!(f, "Metal"),
            Self::Verilog => write!(f, "Verilog"),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DeltaSizeMetrics {
    pub lines_added: usize,
    pub lines_removed: usize,
    pub files_affected: usize,
    pub binary_size_bytes: usize,
}

impl DeltaNode {
    pub fn new(substrate_type: SubstrateType, binary_diff: Vec<u8>) -> Self {
        let binary_size_bytes = binary_diff.len();
        Self {
            substrate_type,
            git_ref_fallback: None,
            binary_diff,
            synthesis_params: HashMap::new(),
            affected_paths: Vec::new(),
            size_metrics: DeltaSizeMetrics {
                binary_size_bytes,
                ..Default::default()
            },
        }
    }

    pub fn with_affected_path(mut self, path: impl Into<String>) -> Self {
        self.affected_paths.push(path.into());
        self.size_metrics.files_affected = self.affected_paths.len();
        self
    }

    pub fn with_lines(mut self, added: usize, removed: usize) -> Self {
        self.size_metrics.lines_added = added;
        self.size_metrics.lines_removed = removed;
        self
    }

    pub fn lines_changed(&self) -> usize {
        self.size_metrics.lines_added + self.size_metrics.lines_removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_node_builder() {
        let delta = DeltaNode::new(SubstrateType::Rust, vec![1, 2, 3])
            .with_affected_path("src/main.rs")
            .with_affected_path("src/lib.rs")
            .with_lines(50, 20);

        assert_eq!(delta.substrate_type, SubstrateType::Rust);
        assert_eq!(delta.affected_paths.len(), 2);
        assert_eq!(delta.size_metrics.files_affected, 2);
        assert_eq!(delta.lines_changed(), 70);
        assert_eq!(delta.size_metrics.binary_size_bytes, 3);
    }

    #[test]
    fn substrate_type_display() {
        assert_eq!(format!("{}", SubstrateType::Rust), "Rust");
        assert_eq!(format!("{}", SubstrateType::Wlir), "WLIR");
        assert_eq!(format!("{}", SubstrateType::Cuda), "CUDA");
    }

    #[test]
    fn delta_serde_roundtrip() {
        let delta = DeltaNode::new(SubstrateType::Wasm, vec![42]);
        let json = serde_json::to_string(&delta).unwrap();
        let restored: DeltaNode = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.substrate_type, SubstrateType::Wasm);
        assert_eq!(restored.binary_diff, vec![42]);
    }
}
