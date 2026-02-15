//! Code generators for WLIR modules.
//!
//! Supports four targets via simulated implementations:
//! - **NativeCodeGen** — x86-64 / ARM64 pseudo-assembly
//! - **WasmCodeGen** — WebAssembly text format (WAT)
//! - **OperatorCallCodeGen** — Direct Rust operator calls
//! - **WlirInterpreter** — Interpreted bytecode (dev/debug)

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::{CompilerError, CompilerResult};
use crate::types::{CompilationTarget, TargetArch, WasmEnvironment};
use maple_worldline_ir::module::WlirModule;

// ── Generated Code ───────────────────────────────────────────────────

/// Output of a code generation step.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneratedCode {
    pub target: CompilationTarget,
    pub content: String,
    pub content_hash: String,
    pub size_bytes: usize,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

impl GeneratedCode {
    /// Compute a simple hash of content for integrity checks.
    pub fn compute_hash(content: &str) -> String {
        // Simple FNV-like hash for deterministic testing
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in content.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{:016x}", hash)
    }
}

// ── Code Generator Trait ─────────────────────────────────────────────

/// Trait for generating code from a WLIR module.
pub trait CodeGenerator: Send + Sync {
    /// The compilation target this generator produces.
    fn target(&self) -> CompilationTarget;

    /// Generate code from a WLIR module.
    fn generate(&self, module: &WlirModule) -> CompilerResult<GeneratedCode>;

    /// Name of this generator implementation.
    fn name(&self) -> &str;
}

// ── Native Code Generator ────────────────────────────────────────────

/// Simulated native code generator (x86-64 / ARM64).
pub struct NativeCodeGen {
    arch: TargetArch,
}

impl NativeCodeGen {
    pub fn new(arch: TargetArch) -> Self {
        Self { arch }
    }
}

impl CodeGenerator for NativeCodeGen {
    fn target(&self) -> CompilationTarget {
        CompilationTarget::Native {
            arch: self.arch.clone(),
        }
    }

    fn generate(&self, module: &WlirModule) -> CompilerResult<GeneratedCode> {
        let mut code = String::new();
        code.push_str(&format!(
            "; Native {} assembly for module '{}' v{}\n",
            self.arch, module.name, module.version
        ));
        code.push_str(&format!("; Functions: {}\n\n", module.functions.len()));

        for func in &module.functions {
            code.push_str(&format!(".global {}\n", func.name));
            code.push_str(&format!("{}:\n", func.name));
            match &self.arch {
                TargetArch::X86_64 => {
                    code.push_str("  push rbp\n");
                    code.push_str("  mov rbp, rsp\n");
                    code.push_str(&format!("  ; {} instructions\n", func.instructions.len()));
                    code.push_str("  pop rbp\n");
                    code.push_str("  ret\n\n");
                }
                TargetArch::Aarch64 => {
                    code.push_str("  stp x29, x30, [sp, #-16]!\n");
                    code.push_str("  mov x29, sp\n");
                    code.push_str(&format!("  ; {} instructions\n", func.instructions.len()));
                    code.push_str("  ldp x29, x30, [sp], #16\n");
                    code.push_str("  ret\n\n");
                }
            }
        }

        let hash = GeneratedCode::compute_hash(&code);
        let size = code.len();

        Ok(GeneratedCode {
            target: self.target(),
            content: code,
            content_hash: hash,
            size_bytes: size,
            generated_at: Utc::now(),
        })
    }

    fn name(&self) -> &str {
        "native-codegen"
    }
}

// ── WASM Code Generator ──────────────────────────────────────────────

/// Simulated WASM code generator (WAT format).
pub struct WasmCodeGen {
    env: WasmEnvironment,
}

impl WasmCodeGen {
    pub fn new(env: WasmEnvironment) -> Self {
        Self { env }
    }
}

impl CodeGenerator for WasmCodeGen {
    fn target(&self) -> CompilationTarget {
        CompilationTarget::Wasm {
            env: self.env.clone(),
        }
    }

    fn generate(&self, module: &WlirModule) -> CompilerResult<GeneratedCode> {
        let mut code = String::new();
        code.push_str(&format!(
            ";; WASM module '{}' v{} (target: {})\n",
            module.name, module.version, self.env
        ));
        code.push_str("(module\n");

        for func in &module.functions {
            code.push_str(&format!(
                "  (func ${} (export \"{}\")\n",
                func.name, func.name
            ));
            code.push_str(&format!(
                "    ;; {} instructions\n",
                func.instructions.len()
            ));
            code.push_str("    nop\n");
            code.push_str("  )\n");
        }

        code.push_str(")\n");

        let hash = GeneratedCode::compute_hash(&code);
        let size = code.len();

        Ok(GeneratedCode {
            target: self.target(),
            content: code,
            content_hash: hash,
            size_bytes: size,
            generated_at: Utc::now(),
        })
    }

    fn name(&self) -> &str {
        "wasm-codegen"
    }
}

// ── Operator Call Code Generator ─────────────────────────────────────

/// Simulated operator call generator (direct Rust calls).
pub struct OperatorCallCodeGen;

impl OperatorCallCodeGen {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OperatorCallCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGenerator for OperatorCallCodeGen {
    fn target(&self) -> CompilationTarget {
        CompilationTarget::OperatorCall
    }

    fn generate(&self, module: &WlirModule) -> CompilerResult<GeneratedCode> {
        let mut code = String::new();
        code.push_str(&format!(
            "// Operator calls for module '{}' v{}\n\n",
            module.name, module.version
        ));

        for func in &module.functions {
            code.push_str(&format!(
                "fn {}() -> Result<(), OperatorError> {{\n",
                func.name
            ));
            code.push_str(&format!(
                "    // {} WLIR instructions → operator calls\n",
                func.instructions.len()
            ));
            code.push_str("    Ok(())\n");
            code.push_str("}\n\n");
        }

        let hash = GeneratedCode::compute_hash(&code);
        let size = code.len();

        Ok(GeneratedCode {
            target: self.target(),
            content: code,
            content_hash: hash,
            size_bytes: size,
            generated_at: Utc::now(),
        })
    }

    fn name(&self) -> &str {
        "operator-call-codegen"
    }
}

// ── WLIR Interpreter ─────────────────────────────────────────────────

/// Simulated WLIR interpreter (bytecode representation for dev/debug).
pub struct WlirInterpreterCodeGen;

impl WlirInterpreterCodeGen {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WlirInterpreterCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGenerator for WlirInterpreterCodeGen {
    fn target(&self) -> CompilationTarget {
        CompilationTarget::Interpreted
    }

    fn generate(&self, module: &WlirModule) -> CompilerResult<GeneratedCode> {
        // Serialize the module as JSON bytecode representation
        let code = serde_json::to_string_pretty(module)
            .map_err(|e| CompilerError::CodeGenerationFailed(e.to_string()))?;

        let hash = GeneratedCode::compute_hash(&code);
        let size = code.len();

        Ok(GeneratedCode {
            target: self.target(),
            content: code,
            content_hash: hash,
            size_bytes: size,
            generated_at: Utc::now(),
        })
    }

    fn name(&self) -> &str {
        "wlir-interpreter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_worldline_ir::instructions::WlirInstruction;
    use maple_worldline_ir::module::WlirFunction;
    use maple_worldline_ir::types::WlirType;

    fn make_module() -> WlirModule {
        let mut module = WlirModule::new("test-module", "1.0.0");
        let mut f = WlirFunction::new("main", vec![], WlirType::Void);
        f.push_instruction(WlirInstruction::Nop);
        f.push_instruction(WlirInstruction::Return { value: None });
        module.add_function(f);
        module
    }

    #[test]
    fn native_codegen_x86_64() {
        let gen = NativeCodeGen::new(TargetArch::X86_64);
        let module = make_module();
        let code = gen.generate(&module).unwrap();
        assert_eq!(code.target, CompilationTarget::Native { arch: TargetArch::X86_64 });
        assert!(code.content.contains("push rbp"));
        assert!(code.content.contains("test-module"));
    }

    #[test]
    fn native_codegen_aarch64() {
        let gen = NativeCodeGen::new(TargetArch::Aarch64);
        let module = make_module();
        let code = gen.generate(&module).unwrap();
        assert!(code.content.contains("stp x29"));
    }

    #[test]
    fn wasm_codegen_browser() {
        let gen = WasmCodeGen::new(WasmEnvironment::Browser);
        let module = make_module();
        let code = gen.generate(&module).unwrap();
        assert_eq!(code.target, CompilationTarget::Wasm { env: WasmEnvironment::Browser });
        assert!(code.content.contains("(module"));
        assert!(code.content.contains("(func $main"));
    }

    #[test]
    fn wasm_codegen_edge() {
        let gen = WasmCodeGen::new(WasmEnvironment::Edge);
        let module = make_module();
        let code = gen.generate(&module).unwrap();
        assert!(code.content.contains("edge"));
    }

    #[test]
    fn operator_call_codegen() {
        let gen = OperatorCallCodeGen::new();
        let module = make_module();
        let code = gen.generate(&module).unwrap();
        assert_eq!(code.target, CompilationTarget::OperatorCall);
        assert!(code.content.contains("fn main()"));
        assert!(code.content.contains("Result<()"));
    }

    #[test]
    fn wlir_interpreter_codegen() {
        let gen = WlirInterpreterCodeGen::new();
        let module = make_module();
        let code = gen.generate(&module).unwrap();
        assert_eq!(code.target, CompilationTarget::Interpreted);
        assert!(code.content.contains("test-module"));
    }

    #[test]
    fn generated_code_hash_deterministic() {
        let h1 = GeneratedCode::compute_hash("hello world");
        let h2 = GeneratedCode::compute_hash("hello world");
        assert_eq!(h1, h2);
        let h3 = GeneratedCode::compute_hash("different");
        assert_ne!(h1, h3);
    }

    #[test]
    fn generated_code_size_tracking() {
        let gen = NativeCodeGen::new(TargetArch::X86_64);
        let module = make_module();
        let code = gen.generate(&module).unwrap();
        assert_eq!(code.size_bytes, code.content.len());
        assert!(code.size_bytes > 0);
    }

    #[test]
    fn codegen_preserves_module_name() {
        let gen = WasmCodeGen::new(WasmEnvironment::Browser);
        let module = make_module();
        let code = gen.generate(&module).unwrap();
        assert!(code.content.contains("test-module"));
    }

    #[test]
    fn codegen_includes_all_functions() {
        let mut module = WlirModule::new("multi", "1.0");
        let f1 = WlirFunction::new("func_a", vec![], WlirType::Void);
        let f2 = WlirFunction::new("func_b", vec![], WlirType::I32);
        module.add_function(f1);
        module.add_function(f2);

        let gen = OperatorCallCodeGen::new();
        let code = gen.generate(&module).unwrap();
        assert!(code.content.contains("func_a"));
        assert!(code.content.contains("func_b"));
    }

    #[test]
    fn generator_names() {
        assert_eq!(NativeCodeGen::new(TargetArch::X86_64).name(), "native-codegen");
        assert_eq!(WasmCodeGen::new(WasmEnvironment::Browser).name(), "wasm-codegen");
        assert_eq!(OperatorCallCodeGen::new().name(), "operator-call-codegen");
        assert_eq!(WlirInterpreterCodeGen::new().name(), "wlir-interpreter");
    }

    #[test]
    fn all_targets_generate_successfully() {
        let module = make_module();
        let generators: Vec<Box<dyn CodeGenerator>> = vec![
            Box::new(NativeCodeGen::new(TargetArch::X86_64)),
            Box::new(NativeCodeGen::new(TargetArch::Aarch64)),
            Box::new(WasmCodeGen::new(WasmEnvironment::Browser)),
            Box::new(WasmCodeGen::new(WasmEnvironment::Edge)),
            Box::new(OperatorCallCodeGen::new()),
            Box::new(WlirInterpreterCodeGen::new()),
        ];
        for gen in &generators {
            let code = gen.generate(&module).unwrap();
            assert!(!code.content.is_empty());
            assert!(!code.content_hash.is_empty());
        }
    }
}
