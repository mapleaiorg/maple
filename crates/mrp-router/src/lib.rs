#![deny(unsafe_code)]
use mrp_types::{Destination, MrpEnvelope};
use rcl_types::ResonanceType;
use std::collections::HashMap;

pub struct MrpRouter { defaults: HashMap<ResonanceType, Destination> }
impl MrpRouter {
    pub fn new() -> Self {
        let mut defaults = HashMap::new();
        defaults.insert(ResonanceType::Commitment, Destination::service("aas"));
        defaults.insert(ResonanceType::Consequence, Destination::service("eve"));
        Self { defaults }
    }
    pub fn route(&self, envelope: &MrpEnvelope) -> Result<Vec<Destination>, RoutingError> {
        let mut dests = envelope.header.routing_constraints.required_destinations.clone();
        if dests.is_empty() {
            if let Some(d) = self.defaults.get(&envelope.header.resonance_type) {
                dests.push(d.clone());
            }
        }
        if dests.is_empty() { return Err(RoutingError::NoDestinations); }
        Ok(dests)
    }
}
impl Default for MrpRouter { fn default() -> Self { Self::new() } }

#[derive(Debug, thiserror::Error)]
pub enum RoutingError {
    #[error("No destinations")] NoDestinations,
    #[error("Invariant violation")] InvariantViolation,
}
