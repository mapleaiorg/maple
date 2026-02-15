//! # maple-worldline-ir
//!
//! WorldLine Intermediate Representation (WLIR) — an EVOS-native instruction
//! set with first-class events, commitments, provenance, and safety fences.
//!
//! WLIR provides a portable, verifiable representation for WorldLine programs
//! that can be verified, serialized, and compiled to target platforms.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────┐
//! │                    WlirModule                        │
//! │  ┌────────────┐ ┌────────────┐ ┌──────────────────┐ │
//! │  │ WlirFunction│ │ WlirTypeDecl│ │CommitmentDecl   │ │
//! │  │            │ │            │ │                  │ │
//! │  │ Instructions│ │ Underlying │ │ SafetyFences     │ │
//! │  │  Standard  │ │   WlirType │ │ Entry/Exit       │ │
//! │  │  EVOS      │ └────────────┘ └──────────────────┘ │
//! │  │  Safety    │                                      │
//! │  │  Metadata  │ ┌────────────┐                       │
//! │  └────────────┘ │WlirConstant│                       │
//! │                 └────────────┘                       │
//! ├──────────────────────────────────────────────────────┤
//! │  Verifier (5 aspects)    │  Serializer (text/binary) │
//! └──────────────────────────────────────────────────────┘
//! ```
//!
//! ## Instruction Categories
//!
//! - **Standard**: Arithmetic, memory, control flow (Add, Sub, Mul, etc.)
//! - **EVOS-native**: Events, commitments, provenance, operators
//! - **Safety**: Invariants, coercion checks, safety fences
//! - **Metadata**: Source locations, type annotations
//!
//! ## Verification Aspects
//!
//! 1. Type Correctness
//! 2. Commitment Boundary Integrity
//! 3. Provenance Completeness
//! 4. Safety Fence Ordering
//! 5. Control Flow Integrity

#![deny(unsafe_code)]

pub mod error;
pub mod instructions;
pub mod module;
pub mod serialization;
pub mod types;
pub mod verifier;

// ── Re-exports ───────────────────────────────────────────────────────

pub use error::{WlirError, WlirResult};
pub use instructions::{BoundaryDirection, CompareOp, InstructionCategory, WlirInstruction};
pub use module::{CommitmentDeclaration, WlirConstant, WlirFunction, WlirModule, WlirTypeDecl};
pub use serialization::{
    SerializedModule, SimulatedSerializer, WlirFormat, WlirSerializer, WLIR_BINARY_MAGIC,
    WLIR_TEXT_MAGIC,
};
pub use types::{
    FunctionId, InstructionId, MemoryTier, ModuleId, SourceLocation, VerificationStatus,
    WlirConfig, WlirData, WlirModuleSummary, WlirType, WlirValue,
};
pub use verifier::{
    SimulatedVerifier, VerificationAspect, VerificationReport, VerificationResult, WlirVerifier,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_verify_and_serialize_module() {
        // End-to-end: build a module, verify it, serialize it
        let mut module = WlirModule::new("e2e-module", "1.0.0");
        let mut f = WlirFunction::new("main", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::Nop);
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);

        // Verify
        let verifier = SimulatedVerifier::all_pass();
        let config = WlirConfig::default();
        let report = verifier.verify(&module, &config).unwrap();
        assert!(report.all_passed);

        // Serialize round-trip
        let serializer = SimulatedSerializer::new();
        let serialized = serializer.serialize(&module, &WlirFormat::Text).unwrap();
        let deserialized = serializer.deserialize(&serialized).unwrap();
        assert_eq!(deserialized.name, "e2e-module");
        assert_eq!(deserialized.functions.len(), 1);
    }

    #[test]
    fn evos_instructions_are_verifiable() {
        let mut module = WlirModule::new("evos-module", "1.0.0");
        let mut f = WlirFunction::new("emit", vec![], WlirType::EventRef);
        f.push_instruction(WlirInstruction::EmitEvent {
            event_type: "test-event".into(),
            payload_registers: vec![0],
            result_event_id: 1,
        });
        f.push_instruction(WlirInstruction::RecordProvenance {
            operation: "emit".into(),
            inputs: vec![0],
            output: 1,
        });
        f.push_instruction(WlirInstruction::Return { value: Some(1) });
        module.add_function(f);

        let verifier = SimulatedVerifier::all_pass();
        let config = WlirConfig::default();
        let report = verifier.verify(&module, &config).unwrap();
        assert!(report.all_passed);
        assert_eq!(report.aspects_checked, 5);
    }

    #[test]
    fn commitment_boundary_integrity_enforced() {
        use crate::instructions::BoundaryDirection;

        let mut module = WlirModule::new("boundary-test", "1.0.0");
        let mut f = WlirFunction::new("balanced", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::CrossCommitmentBoundary {
            commitment_id: "payment".into(),
            direction: BoundaryDirection::Enter,
        });
        f.push_instruction(WlirInstruction::CrossCommitmentBoundary {
            commitment_id: "payment".into(),
            direction: BoundaryDirection::Exit,
        });
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);

        assert!(module.functions[0].commitment_boundaries_balanced());
        assert!(module.functions[0].crosses_commitment_boundary());

        let verifier = SimulatedVerifier::all_pass();
        let config = WlirConfig::default();
        let report = verifier.verify(&module, &config).unwrap();
        assert!(report.all_passed);
    }

    #[test]
    fn serialization_roundtrip_preserves_instructions() {
        let mut module = WlirModule::new("roundtrip", "2.0.0");
        let mut f = WlirFunction::new("compute", vec![("x".into(), WlirType::I32)], WlirType::I32);
        f.push_instruction(WlirInstruction::LoadLocal { result: 0, local_index: 0 });
        f.push_instruction(WlirInstruction::LoadConst { result: 1, constant_index: 0 });
        f.push_instruction(WlirInstruction::Add { result: 2, a: 0, b: 1 });
        f.push_instruction(WlirInstruction::Return { value: Some(2) });
        module.add_function(f);

        let serializer = SimulatedSerializer::new();

        // Text roundtrip
        let text_data = serializer.serialize(&module, &WlirFormat::Text).unwrap();
        let text_rt = serializer.deserialize(&text_data).unwrap();
        assert_eq!(text_rt.total_instructions(), 4);

        // Binary roundtrip
        let bin_data = serializer.serialize(&module, &WlirFormat::Binary).unwrap();
        let bin_rt = serializer.deserialize(&bin_data).unwrap();
        assert_eq!(bin_rt.total_instructions(), 4);
    }

    #[test]
    fn safety_fence_enforcement() {
        let mut module = WlirModule::new("safety-test", "1.0.0");
        let mut f = WlirFunction::new("safe_op", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::SafetyFence {
            fence_name: "pre-commit".into(),
            preceding_ops: vec![0],
        });
        f.push_instruction(WlirInstruction::AssertInvariant {
            condition: 0,
            invariant_name: "balance_positive".into(),
            message: "balance must be positive".into(),
        });
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);

        assert!(module.functions[0].has_safety_instructions());

        let verifier = SimulatedVerifier::all_pass();
        let config = WlirConfig::default();
        let report = verifier.verify(&module, &config).unwrap();
        assert!(report.all_passed);
    }

    #[test]
    fn public_types_accessible() {
        // Verify all public re-exports are accessible
        let _id = ModuleId::new();
        let _fid = FunctionId::from_name("test");
        let _iid = InstructionId::new();
        let _t = WlirType::I32;
        let _d = WlirData::Integer(42);
        let _v = WlirValue {
            value_type: WlirType::I32,
            data: WlirData::Integer(42),
        };
        let _loc = SourceLocation {
            file: "test.rs".into(),
            line: 1,
            column: 1,
        };
        let _tier = MemoryTier::Working;
        let _status = VerificationStatus::Unverified;
        let _config = WlirConfig::default();
        let _summary = WlirModuleSummary::default();
        let _aspect = VerificationAspect::TypeCorrectness;
        let _format = WlirFormat::Text;
        let _cat = InstructionCategory::Standard;
        let _dir = BoundaryDirection::Enter;
        let _op = CompareOp::Equal;
    }
}
