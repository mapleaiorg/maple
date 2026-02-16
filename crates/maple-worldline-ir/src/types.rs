//! Core types for the WorldLine Intermediate Representation.
//!
//! Defines identifiers, WLIR value types, source locations, memory tiers,
//! verification status, configuration, and summary statistics.

use serde::{Deserialize, Serialize};

// ── Identifiers ────────────────────────────────────────────────────────

/// Unique identifier for a WLIR module.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleId(pub String);

impl ModuleId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for ModuleId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "wlir-mod:{}", self.0)
    }
}

/// Unique identifier for a WLIR function.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FunctionId(pub String);

impl FunctionId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_name(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl Default for FunctionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for FunctionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "wlir-fn:{}", self.0)
    }
}

/// Unique identifier for a WLIR instruction.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InstructionId(pub String);

impl InstructionId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for InstructionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for InstructionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "wlir-inst:{}", self.0)
    }
}

// ── WLIR Value Types ──────────────────────────────────────────────────

/// Value types in the WLIR type system.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WlirType {
    I32,
    I64,
    F32,
    F64,
    Bool,
    String,
    Bytes,
    /// Reference to an event (EventId).
    EventRef,
    /// Reference to a commitment (CommitmentId).
    CommitmentRef,
    /// Reference to a worldline (WorldlineId).
    WorldlineRef,
    /// Reference to an operator.
    OperatorRef,
    /// Financial amount with currency.
    Amount {
        currency: std::string::String,
    },
    /// Named record type.
    Record {
        name: std::string::String,
        fields: Vec<(std::string::String, WlirType)>,
    },
    /// Array of elements.
    Array {
        element: Box<WlirType>,
    },
    /// Optional value.
    Option {
        inner: Box<WlirType>,
    },
    /// No value (unit type).
    Void,
}

impl std::fmt::Display for WlirType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::I32 => write!(f, "i32"),
            Self::I64 => write!(f, "i64"),
            Self::F32 => write!(f, "f32"),
            Self::F64 => write!(f, "f64"),
            Self::Bool => write!(f, "bool"),
            Self::String => write!(f, "string"),
            Self::Bytes => write!(f, "bytes"),
            Self::EventRef => write!(f, "event-ref"),
            Self::CommitmentRef => write!(f, "commitment-ref"),
            Self::WorldlineRef => write!(f, "worldline-ref"),
            Self::OperatorRef => write!(f, "operator-ref"),
            Self::Amount { currency } => write!(f, "amount<{}>", currency),
            Self::Record { name, .. } => write!(f, "record<{}>", name),
            Self::Array { element } => write!(f, "array<{}>", element),
            Self::Option { inner } => write!(f, "option<{}>", inner),
            Self::Void => write!(f, "void"),
        }
    }
}

// ── WLIR Data ─────────────────────────────────────────────────────────

/// Runtime data representation for WLIR values.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WlirData {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Text(std::string::String),
    Binary(Vec<u8>),
    Ref(std::string::String),
    Void,
}

/// A typed WLIR value.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WlirValue {
    pub value_type: WlirType,
    pub data: WlirData,
}

// ── Source Location ────────────────────────────────────────────────────

/// Source location metadata for debugging.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: std::string::String,
    pub line: u32,
    pub column: u32,
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

// ── Memory Tier ───────────────────────────────────────────────────────

/// Memory tier for tier-aware memory operations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryTier {
    /// Working memory (fast, volatile).
    Working,
    /// Episodic memory (medium-term).
    Episodic,
    /// Semantic memory (long-term, structured).
    Semantic,
    /// Fabric memory (event fabric layer).
    Fabric,
}

impl std::fmt::Display for MemoryTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Working => write!(f, "working"),
            Self::Episodic => write!(f, "episodic"),
            Self::Semantic => write!(f, "semantic"),
            Self::Fabric => write!(f, "fabric"),
        }
    }
}

// ── Verification Status ───────────────────────────────────────────────

/// Verification status of a WLIR module.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum VerificationStatus {
    /// Not yet verified.
    Unverified,
    /// Type checking passed only.
    TypeChecked,
    /// Full verification passed (all 5 aspects).
    FullyVerified,
    /// Verification failed.
    VerificationFailed(std::string::String),
}

impl VerificationStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::FullyVerified | Self::VerificationFailed(_))
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::FullyVerified)
    }
}

impl std::fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unverified => write!(f, "unverified"),
            Self::TypeChecked => write!(f, "type-checked"),
            Self::FullyVerified => write!(f, "fully-verified"),
            Self::VerificationFailed(reason) => write!(f, "failed: {}", reason),
        }
    }
}

impl Default for VerificationStatus {
    fn default() -> Self {
        Self::Unverified
    }
}

// ── Configuration ─────────────────────────────────────────────────────

/// Configuration for WLIR modules and verification.
#[derive(Clone, Debug)]
pub struct WlirConfig {
    /// Maximum number of functions per module.
    pub max_functions: usize,
    /// Maximum total instructions across all functions.
    pub max_instructions: usize,
    /// Maximum number of constants.
    pub max_constants: usize,
    /// Maximum number of type declarations.
    pub max_types: usize,
    /// Whether to enforce provenance on side-effecting instructions.
    pub enforce_provenance: bool,
    /// Whether to enforce safety fences before safety-critical operations.
    pub enforce_safety_fences: bool,
}

impl Default for WlirConfig {
    fn default() -> Self {
        Self {
            max_functions: 1024,
            max_instructions: 65536,
            max_constants: 4096,
            max_types: 512,
            enforce_provenance: true,
            enforce_safety_fences: true,
        }
    }
}

// ── Summary ───────────────────────────────────────────────────────────

/// Summary statistics for a WLIR module.
#[derive(Clone, Debug, Default)]
pub struct WlirModuleSummary {
    pub total_functions: usize,
    pub total_instructions: usize,
    pub total_types: usize,
    pub total_constants: usize,
    pub total_commitment_declarations: usize,
    pub verification_status: VerificationStatus,
}

impl std::fmt::Display for WlirModuleSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "WlirModule(functions={}, instructions={}, types={}, constants={}, commitments={}, status={})",
            self.total_functions,
            self.total_instructions,
            self.total_types,
            self.total_constants,
            self.total_commitment_declarations,
            self.verification_status,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_id_uniqueness() {
        let a = ModuleId::new();
        let b = ModuleId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn module_id_display() {
        let id = ModuleId::new();
        assert!(id.to_string().starts_with("wlir-mod:"));
    }

    #[test]
    fn function_id_display() {
        let id = FunctionId::new();
        assert!(id.to_string().starts_with("wlir-fn:"));
    }

    #[test]
    fn instruction_id_display() {
        let id = InstructionId::new();
        assert!(id.to_string().starts_with("wlir-inst:"));
    }

    #[test]
    fn wlir_type_display_all_variants() {
        assert_eq!(WlirType::I32.to_string(), "i32");
        assert_eq!(WlirType::I64.to_string(), "i64");
        assert_eq!(WlirType::Bool.to_string(), "bool");
        assert_eq!(WlirType::String.to_string(), "string");
        assert_eq!(WlirType::EventRef.to_string(), "event-ref");
        assert_eq!(WlirType::CommitmentRef.to_string(), "commitment-ref");
        assert_eq!(
            WlirType::Amount {
                currency: "USD".into()
            }
            .to_string(),
            "amount<USD>"
        );
        assert_eq!(
            WlirType::Array {
                element: Box::new(WlirType::I32)
            }
            .to_string(),
            "array<i32>"
        );
        assert_eq!(WlirType::Void.to_string(), "void");
    }

    #[test]
    fn verification_status_is_terminal() {
        assert!(!VerificationStatus::Unverified.is_terminal());
        assert!(!VerificationStatus::TypeChecked.is_terminal());
        assert!(VerificationStatus::FullyVerified.is_terminal());
        assert!(VerificationStatus::VerificationFailed("x".into()).is_terminal());
    }

    #[test]
    fn verification_status_is_success() {
        assert!(VerificationStatus::FullyVerified.is_success());
        assert!(!VerificationStatus::Unverified.is_success());
        assert!(!VerificationStatus::VerificationFailed("x".into()).is_success());
    }

    #[test]
    fn source_location_display() {
        let loc = SourceLocation {
            file: "src/main.rs".into(),
            line: 42,
            column: 10,
        };
        assert_eq!(loc.to_string(), "src/main.rs:42:10");
    }

    #[test]
    fn memory_tier_display() {
        assert_eq!(MemoryTier::Working.to_string(), "working");
        assert_eq!(MemoryTier::Episodic.to_string(), "episodic");
        assert_eq!(MemoryTier::Semantic.to_string(), "semantic");
        assert_eq!(MemoryTier::Fabric.to_string(), "fabric");
    }

    #[test]
    fn config_defaults() {
        let cfg = WlirConfig::default();
        assert_eq!(cfg.max_functions, 1024);
        assert_eq!(cfg.max_instructions, 65536);
        assert_eq!(cfg.max_constants, 4096);
        assert_eq!(cfg.max_types, 512);
        assert!(cfg.enforce_provenance);
        assert!(cfg.enforce_safety_fences);
    }

    #[test]
    fn summary_default() {
        let s = WlirModuleSummary::default();
        assert_eq!(s.total_functions, 0);
        assert_eq!(s.total_instructions, 0);
    }

    #[test]
    fn summary_display() {
        let s = WlirModuleSummary {
            total_functions: 5,
            total_instructions: 100,
            total_types: 10,
            total_constants: 20,
            total_commitment_declarations: 2,
            verification_status: VerificationStatus::FullyVerified,
        };
        let display = s.to_string();
        assert!(display.contains("functions=5"));
        assert!(display.contains("instructions=100"));
        assert!(display.contains("fully-verified"));
    }
}
