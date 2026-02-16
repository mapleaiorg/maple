//! WorldLine self-producing substrate components.

pub use maple_worldline_bootstrap as bootstrap;
pub use maple_worldline_codegen as codegen;
pub use maple_worldline_commitment as commitment;
pub use maple_worldline_compiler as compiler;
pub use maple_worldline_consequence as consequence;
pub use maple_worldline_deployment as deployment;
pub use maple_worldline_evos as evos;
pub use maple_worldline_hardware as hardware;
pub use maple_worldline_intent as intent;
pub use maple_worldline_ir as ir;
pub use maple_worldline_langgen as langgen;
pub use maple_worldline_meaning as meaning;
pub use maple_worldline_observation as observation;
pub use maple_worldline_sal as sal;
pub use maple_worldline_self_mod_gate as self_mod_gate;
pub use worldline_conformance as conformance;

#[cfg(test)]
mod tests {
    use super::{meaning, observation};

    #[test]
    fn substrate_exports_modules() {
        let _ = observation::CollectorConfig::default();
        let _ = meaning::MeaningConfig::default();
    }
}
