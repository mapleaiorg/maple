use crate::error::FabricError;
use crate::event::{KernelEvent, ResonanceStage};
use crate::types::WorldlineId;

/// Trait for kernel modules that consume events from the fabric.
pub trait FabricConsumer: Send + Sync {
    /// Called when an event is emitted.
    fn on_event(&self, event: &KernelEvent) -> Result<(), FabricError>;

    /// Which resonance stages this consumer is interested in.
    /// Return None to receive all stages.
    fn subscribed_stages(&self) -> Option<Vec<ResonanceStage>>;
}

/// Trait for components that produce events into the fabric.
pub trait FabricProducer: Send + Sync {
    /// Get the worldline ID of the producer.
    fn worldline_id(&self) -> WorldlineId;
}
