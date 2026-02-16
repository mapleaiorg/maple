//! WorldLine governance policies and decision services.

pub use maple_kernel_gate as gate;
pub use maple_kernel_governance as governance;
pub use maple_kernel_profiles as profiles;
pub use maple_kernel_safety as safety;

pub use governance::*;

#[cfg(test)]
mod tests {
    use super::governance::AgentAccountabilityService;

    #[test]
    fn governance_engine_is_available() {
        let _ = AgentAccountabilityService::new();
    }
}
