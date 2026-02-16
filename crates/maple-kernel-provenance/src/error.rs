use maple_mwl_types::EventId;
use thiserror::Error;

/// Errors from the Provenance Index.
#[derive(Error, Debug)]
pub enum ProvenanceError {
    #[error("event not found: {0}")]
    EventNotFound(EventId),

    #[error("duplicate event: {0}")]
    DuplicateEvent(EventId),

    #[error(
        "missing causal parent: event {child} references parent {parent} which is not in the index"
    )]
    MissingParent { child: EventId, parent: EventId },

    #[error("event without parents and not a genesis event: {0}")]
    NoParentsNonGenesis(EventId),

    #[error("checkpoint error: {0}")]
    CheckpointError(String),

    #[error("fabric error: {0}")]
    FabricError(#[from] maple_kernel_fabric::FabricError),
}
