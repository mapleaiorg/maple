use maple_waf_context_graph::SubstrateType;
use maple_waf_evolution_engine::HardwareContext;
use serde::{Deserialize, Serialize};

use crate::types::CompilationTarget;

/// Selects the optimal [`CompilationTarget`] based on hardware capabilities
/// and the substrate being compiled.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompilationStrategy;

impl CompilationStrategy {
    /// Determine the best compilation target for the given hardware and substrate.
    ///
    /// Rules:
    /// - `Cuda`, `Metal`, `Verilog` always target `Native` (hardware-specific).
    /// - `Wasm` always targets `Wasm`.
    /// - `Wlir` (WorldLine IR) targets `Wasm` (portable by design).
    /// - `Rust` targets `Native` when the hardware has at least 4 cores and
    ///   4 GiB of memory; otherwise falls back to `Wasm`.
    pub fn select_target(
        hardware: &HardwareContext,
        substrate: &SubstrateType,
    ) -> CompilationTarget {
        match substrate {
            SubstrateType::Cuda | SubstrateType::Metal | SubstrateType::Verilog => {
                CompilationTarget::Native
            }
            SubstrateType::Wasm => CompilationTarget::Wasm,
            SubstrateType::Wlir => CompilationTarget::Wasm,
            SubstrateType::Rust => {
                if hardware.cpu_cores >= 4 && hardware.memory_mb >= 4096 {
                    CompilationTarget::Native
                } else {
                    CompilationTarget::Wasm
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn powerful_hw() -> HardwareContext {
        HardwareContext {
            cpu_cores: 8,
            memory_mb: 16384,
            gpu_available: false,
            gpu_name: None,
            gpu_memory_mb: None,
        }
    }

    fn weak_hw() -> HardwareContext {
        HardwareContext {
            cpu_cores: 2,
            memory_mb: 2048,
            gpu_available: false,
            gpu_name: None,
            gpu_memory_mb: None,
        }
    }

    #[test]
    fn cuda_always_native() {
        let target = CompilationStrategy::select_target(&weak_hw(), &SubstrateType::Cuda);
        assert_eq!(target, CompilationTarget::Native);
    }

    #[test]
    fn metal_always_native() {
        let target = CompilationStrategy::select_target(&weak_hw(), &SubstrateType::Metal);
        assert_eq!(target, CompilationTarget::Native);
    }

    #[test]
    fn verilog_always_native() {
        let target = CompilationStrategy::select_target(&weak_hw(), &SubstrateType::Verilog);
        assert_eq!(target, CompilationTarget::Native);
    }

    #[test]
    fn wasm_substrate_targets_wasm() {
        let target = CompilationStrategy::select_target(&powerful_hw(), &SubstrateType::Wasm);
        assert_eq!(target, CompilationTarget::Wasm);
    }

    #[test]
    fn rust_on_powerful_hw_targets_native() {
        let target = CompilationStrategy::select_target(&powerful_hw(), &SubstrateType::Rust);
        assert_eq!(target, CompilationTarget::Native);
    }

    #[test]
    fn rust_on_weak_hw_targets_wasm() {
        let target = CompilationStrategy::select_target(&weak_hw(), &SubstrateType::Rust);
        assert_eq!(target, CompilationTarget::Wasm);
    }
}
