//! HDL generator — generates Verilog/SystemVerilog/Chisel from EPU spec.
//!
//! Transforms an `EpuSpec` into hardware description language source code
//! that implements the EVOS Processing Unit.

use serde::{Deserialize, Serialize};

use crate::epu::EpuSpec;
use crate::error::HardwareResult;
use crate::types::HdlFormat;

// ── Generated HDL ───────────────────────────────────────────────────

/// Generated HDL source code.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneratedHdl {
    pub format: HdlFormat,
    pub module_name: String,
    pub source: String,
    pub line_count: usize,
    pub content_hash: String,
}

impl GeneratedHdl {
    /// Compute a simple hash of source content.
    pub fn compute_hash(source: &str) -> String {
        let hash = source
            .bytes()
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        format!("{:016x}", hash)
    }
}

impl std::fmt::Display for GeneratedHdl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HDL({}, module={}, {} lines)",
            self.format, self.module_name, self.line_count,
        )
    }
}

// ── HDL Generator Trait ─────────────────────────────────────────────

/// Trait for generating HDL from an EPU specification.
pub trait HdlGenerator: Send + Sync {
    /// Generate HDL source code from an EPU spec.
    fn generate(&self, spec: &EpuSpec, format: &HdlFormat) -> HardwareResult<GeneratedHdl>;

    /// Name of this generator.
    fn name(&self) -> &str;
}

/// Simulated HDL generator for deterministic testing.
pub struct SimulatedHdlGenerator;

impl SimulatedHdlGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimulatedHdlGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl HdlGenerator for SimulatedHdlGenerator {
    fn generate(&self, spec: &EpuSpec, format: &HdlFormat) -> HardwareResult<GeneratedHdl> {
        let source = match format {
            HdlFormat::Verilog => generate_verilog(spec),
            HdlFormat::SystemVerilog => generate_systemverilog(spec),
            HdlFormat::Chisel => generate_chisel(spec),
            HdlFormat::Vhdl => generate_vhdl(spec),
        };

        let line_count = source.lines().count();
        let content_hash = GeneratedHdl::compute_hash(&source);

        Ok(GeneratedHdl {
            format: format.clone(),
            module_name: spec.name.clone(),
            source,
            line_count,
            content_hash,
        })
    }

    fn name(&self) -> &str {
        "simulated-hdl-generator"
    }
}

// ── Template generators ─────────────────────────────────────────────

fn generate_verilog(spec: &EpuSpec) -> String {
    let mut s = String::new();
    s.push_str(&format!("// EPU: {} v{}\n", spec.name, spec.version));
    s.push_str(&format!("module {} (\n", spec.name));
    s.push_str("  input wire clk,\n");
    s.push_str("  input wire rst_n,\n");
    for comp in &spec.components {
        s.push_str(&format!("  // {} ({})\n", comp.name, comp.kind));
        s.push_str(&format!("  input wire [{}-1:0] {}_in,\n", comp.port_count * 8, comp.name));
        s.push_str(&format!("  output wire [{}-1:0] {}_out,\n", comp.port_count * 8, comp.name));
    }
    s.push_str("  output wire ready\n");
    s.push_str(");\n\n");
    s.push_str("  assign ready = 1'b1;\n\n");
    s.push_str("endmodule\n");
    s
}

fn generate_systemverilog(spec: &EpuSpec) -> String {
    let mut s = String::new();
    s.push_str(&format!("// EPU: {} v{} (SystemVerilog)\n", spec.name, spec.version));
    s.push_str(&format!("module {} (\n", spec.name));
    s.push_str("  input logic clk,\n");
    s.push_str("  input logic rst_n,\n");
    for comp in &spec.components {
        s.push_str(&format!("  // {} ({})\n", comp.name, comp.kind));
        s.push_str(&format!(
            "  input logic [{}-1:0] {}_in,\n",
            comp.port_count * 8,
            comp.name
        ));
        s.push_str(&format!(
            "  output logic [{}-1:0] {}_out,\n",
            comp.port_count * 8,
            comp.name
        ));
    }
    s.push_str("  output logic ready\n");
    s.push_str(");\n\n");
    s.push_str("  always_comb begin\n");
    s.push_str("    ready = 1'b1;\n");
    s.push_str("  end\n\n");
    s.push_str("endmodule\n");
    s
}

fn generate_chisel(spec: &EpuSpec) -> String {
    let mut s = String::new();
    s.push_str("import chisel3._\n");
    s.push_str("import chisel3.util._\n\n");
    s.push_str(&format!("// EPU: {} v{}\n", spec.name, spec.version));
    s.push_str(&format!("class {} extends Module {{\n", spec.name));
    s.push_str("  val io = IO(new Bundle {\n");
    for comp in &spec.components {
        s.push_str(&format!(
            "    val {}_in = Input(UInt({}.W))\n",
            comp.name,
            comp.port_count * 8
        ));
        s.push_str(&format!(
            "    val {}_out = Output(UInt({}.W))\n",
            comp.name,
            comp.port_count * 8
        ));
    }
    s.push_str("    val ready = Output(Bool())\n");
    s.push_str("  })\n\n");
    s.push_str("  io.ready := true.B\n");
    s.push_str("}\n");
    s
}

fn generate_vhdl(spec: &EpuSpec) -> String {
    let mut s = String::new();
    s.push_str(&format!("-- EPU: {} v{}\n", spec.name, spec.version));
    s.push_str("library IEEE;\n");
    s.push_str("use IEEE.STD_LOGIC_1164.ALL;\n\n");
    s.push_str(&format!("entity {} is\n", spec.name));
    s.push_str("  port (\n");
    s.push_str("    clk   : in  std_logic;\n");
    s.push_str("    rst_n : in  std_logic;\n");
    s.push_str("    ready : out std_logic\n");
    s.push_str("  );\n");
    s.push_str(&format!("end {};\n\n", spec.name));
    s.push_str(&format!("architecture rtl of {} is\n", spec.name));
    s.push_str("begin\n");
    s.push_str("  ready <= '1';\n");
    s.push_str(&format!("end rtl;\n"));
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epu::{EpuDesigner, SimulatedEpuDesigner};

    fn sample_spec() -> EpuSpec {
        let designer = SimulatedEpuDesigner::new();
        designer.design("test_epu", 100).unwrap()
    }

    #[test]
    fn generate_verilog_format() {
        let gen = SimulatedHdlGenerator::new();
        let hdl = gen.generate(&sample_spec(), &HdlFormat::Verilog).unwrap();
        assert_eq!(hdl.format, HdlFormat::Verilog);
        assert!(hdl.source.contains("module test_epu"));
        assert!(hdl.source.contains("endmodule"));
        assert!(hdl.source.contains("input wire clk"));
    }

    #[test]
    fn generate_systemverilog_format() {
        let gen = SimulatedHdlGenerator::new();
        let hdl = gen.generate(&sample_spec(), &HdlFormat::SystemVerilog).unwrap();
        assert_eq!(hdl.format, HdlFormat::SystemVerilog);
        assert!(hdl.source.contains("input logic clk"));
        assert!(hdl.source.contains("always_comb"));
    }

    #[test]
    fn generate_chisel_format() {
        let gen = SimulatedHdlGenerator::new();
        let hdl = gen.generate(&sample_spec(), &HdlFormat::Chisel).unwrap();
        assert_eq!(hdl.format, HdlFormat::Chisel);
        assert!(hdl.source.contains("import chisel3._"));
        assert!(hdl.source.contains("class test_epu"));
    }

    #[test]
    fn generate_vhdl_format() {
        let gen = SimulatedHdlGenerator::new();
        let hdl = gen.generate(&sample_spec(), &HdlFormat::Vhdl).unwrap();
        assert_eq!(hdl.format, HdlFormat::Vhdl);
        assert!(hdl.source.contains("entity test_epu"));
        assert!(hdl.source.contains("IEEE.STD_LOGIC_1164"));
    }

    #[test]
    fn generated_hdl_has_hash() {
        let gen = SimulatedHdlGenerator::new();
        let hdl = gen.generate(&sample_spec(), &HdlFormat::Verilog).unwrap();
        assert!(!hdl.content_hash.is_empty());
        assert_eq!(hdl.content_hash.len(), 16);
    }

    #[test]
    fn generated_hdl_display() {
        let gen = SimulatedHdlGenerator::new();
        let hdl = gen.generate(&sample_spec(), &HdlFormat::Verilog).unwrap();
        let display = hdl.to_string();
        assert!(display.contains("verilog"));
        assert!(display.contains("test_epu"));
    }

    #[test]
    fn generated_hdl_line_count() {
        let gen = SimulatedHdlGenerator::new();
        let hdl = gen.generate(&sample_spec(), &HdlFormat::Verilog).unwrap();
        assert!(hdl.line_count > 5);
    }

    #[test]
    fn generator_name() {
        let gen = SimulatedHdlGenerator::new();
        assert_eq!(gen.name(), "simulated-hdl-generator");
    }
}
