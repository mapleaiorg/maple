use maple_waf_context_graph::{ContentHash, SubstrateType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A compiled executable artifact produced by the WAF compiler.
///
/// Content-addressed via BLAKE3 hash of the binary payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutableArtifact {
    /// BLAKE3 hash of `binary`, computed at construction time.
    pub hash: ContentHash,
    /// The substrate this artifact was compiled for.
    pub substrate: SubstrateType,
    /// Raw compiled binary.
    pub binary: Vec<u8>,
    /// Arbitrary metadata (compiler version, flags, etc.).
    pub metadata: HashMap<String, String>,
    /// Wall-clock millisecond timestamp when compilation completed.
    pub compiled_at_ms: u64,
}

impl ExecutableArtifact {
    /// Create a new artifact, computing its content hash from `binary`.
    pub fn new(
        substrate: SubstrateType,
        binary: Vec<u8>,
        metadata: HashMap<String, String>,
        compiled_at_ms: u64,
    ) -> Self {
        let hash = ContentHash::hash(&binary);
        Self {
            hash,
            substrate,
            binary,
            metadata,
            compiled_at_ms,
        }
    }

    /// Verify that `self.hash` still matches the BLAKE3 digest of `self.binary`.
    pub fn verify_hash(&self) -> bool {
        ContentHash::hash(&self.binary) == self.hash
    }
}

/// Target compilation backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompilationTarget {
    /// Compile to native machine code for the host platform.
    Native,
    /// Compile to WebAssembly.
    Wasm,
    /// Debug / interpreted mode — no optimizations, maximum diagnostics.
    Debug,
}

impl fmt::Display for CompilationTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Native => write!(f, "Native"),
            Self::Wasm => write!(f, "Wasm"),
            Self::Debug => write!(f, "Debug"),
        }
    }
}

/// Tuning knobs for a compilation run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompilationConfig {
    /// Optimization level: 0 = none, 1 = basic, 2 = full, 3 = aggressive.
    pub optimization_level: u8,
    /// Target backend.
    pub target: CompilationTarget,
    /// Maximum memory the compiler may allocate (MiB).
    pub max_memory_mb: u64,
    /// Maximum CPU wall-clock seconds allowed.
    pub max_cpu_secs: u64,
    /// Whether to run compilation inside a sandbox.
    pub sandbox_enabled: bool,
}

impl Default for CompilationConfig {
    fn default() -> Self {
        Self {
            optimization_level: 2,
            target: CompilationTarget::Native,
            max_memory_mb: 2048,
            max_cpu_secs: 120,
            sandbox_enabled: true,
        }
    }
}

impl CompilationConfig {
    /// Configuration preset for debug builds.
    pub fn debug() -> Self {
        Self {
            optimization_level: 0,
            target: CompilationTarget::Debug,
            max_memory_mb: 4096,
            max_cpu_secs: 300,
            sandbox_enabled: false,
        }
    }

    /// Configuration preset for WebAssembly targets.
    pub fn wasm() -> Self {
        Self {
            optimization_level: 2,
            target: CompilationTarget::Wasm,
            max_memory_mb: 1024,
            max_cpu_secs: 60,
            sandbox_enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_new_computes_hash() {
        let binary = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let artifact = ExecutableArtifact::new(
            SubstrateType::Rust,
            binary.clone(),
            HashMap::new(),
            1000,
        );
        assert_eq!(artifact.hash, ContentHash::hash(&binary));
        assert!(!artifact.hash.is_zero());
    }

    #[test]
    fn artifact_verify_hash_passes() {
        let artifact = ExecutableArtifact::new(
            SubstrateType::Wasm,
            vec![1, 2, 3],
            HashMap::new(),
            2000,
        );
        assert!(artifact.verify_hash());
    }

    #[test]
    fn artifact_verify_hash_fails_on_tamper() {
        let mut artifact = ExecutableArtifact::new(
            SubstrateType::Cuda,
            vec![10, 20, 30],
            HashMap::new(),
            3000,
        );
        // Tamper with the binary after construction.
        artifact.binary.push(0xFF);
        assert!(!artifact.verify_hash());
    }

    #[test]
    fn artifact_serde_roundtrip() {
        let mut meta = HashMap::new();
        meta.insert("compiler".into(), "waf-0.1".into());
        let artifact = ExecutableArtifact::new(SubstrateType::Metal, vec![42], meta, 5000);
        let json = serde_json::to_string(&artifact).unwrap();
        let restored: ExecutableArtifact = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.hash, artifact.hash);
        assert_eq!(restored.binary, artifact.binary);
        assert_eq!(restored.compiled_at_ms, 5000);
    }

    #[test]
    fn compilation_target_display() {
        assert_eq!(format!("{}", CompilationTarget::Native), "Native");
        assert_eq!(format!("{}", CompilationTarget::Wasm), "Wasm");
        assert_eq!(format!("{}", CompilationTarget::Debug), "Debug");
    }

    #[test]
    fn compilation_config_default() {
        let cfg = CompilationConfig::default();
        assert_eq!(cfg.optimization_level, 2);
        assert_eq!(cfg.target, CompilationTarget::Native);
        assert!(cfg.sandbox_enabled);
        assert_eq!(cfg.max_memory_mb, 2048);
        assert_eq!(cfg.max_cpu_secs, 120);
    }

    #[test]
    fn compilation_config_debug_preset() {
        let cfg = CompilationConfig::debug();
        assert_eq!(cfg.optimization_level, 0);
        assert_eq!(cfg.target, CompilationTarget::Debug);
        assert!(!cfg.sandbox_enabled);
    }

    #[test]
    fn compilation_config_wasm_preset() {
        let cfg = CompilationConfig::wasm();
        assert_eq!(cfg.optimization_level, 2);
        assert_eq!(cfg.target, CompilationTarget::Wasm);
        assert!(cfg.sandbox_enabled);
        assert_eq!(cfg.max_memory_mb, 1024);
        assert_eq!(cfg.max_cpu_secs, 60);
    }
}
