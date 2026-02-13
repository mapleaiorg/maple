use thiserror::Error;

/// Errors from the observation subsystem.
#[derive(Debug, Error)]
pub enum ObservationError {
    #[error("observation buffer full: capacity {capacity}, attempted to add {attempted}")]
    BufferFull { capacity: usize, attempted: usize },

    #[error("profiling session not found: {0}")]
    SessionNotFound(String),

    #[error("invariant violation: {invariant} -- {detail}")]
    InvariantViolation { invariant: String, detail: String },

    #[error("subsystem not registered: {0}")]
    SubsystemNotRegistered(String),

    #[error("sampling rate out of range: {rate} (must be {min}..=1.0)")]
    InvalidSamplingRate { rate: f64, min: f64 },

    #[error("memory budget exceeded: {used} bytes > {limit} bytes")]
    MemoryBudgetExceeded { used: usize, limit: usize },

    #[error("lock acquisition failed")]
    LockError,

    #[error("fabric subscription error: {0}")]
    FabricError(String),

    #[error("baseline not established for metric: {0}")]
    BaselineNotEstablished(String),

    #[error("anomaly detection failed: {0}")]
    AnomalyDetectionFailed(String),

    #[error("persistence error: {0}")]
    PersistenceError(String),
}

impl From<maple_kernel_fabric::FabricError> for ObservationError {
    fn from(e: maple_kernel_fabric::FabricError) -> Self {
        ObservationError::FabricError(e.to_string())
    }
}

impl From<std::io::Error> for ObservationError {
    fn from(e: std::io::Error) -> Self {
        ObservationError::PersistenceError(e.to_string())
    }
}

/// Convenience type alias for observation results.
pub type ObservationResult<T> = Result<T, ObservationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let e = ObservationError::BufferFull {
            capacity: 1000,
            attempted: 1001,
        };
        assert!(e.to_string().contains("1000"));

        let e = ObservationError::InvariantViolation {
            invariant: "I.OBS-1".into(),
            detail: "overhead exceeded 1%".into(),
        };
        assert!(e.to_string().contains("I.OBS-1"));
    }

    #[test]
    fn new_error_variants_display() {
        let e = ObservationError::BaselineNotEstablished("event-fabric.latency".into());
        assert!(e.to_string().contains("event-fabric.latency"));

        let e = ObservationError::AnomalyDetectionFailed("timeout".into());
        assert!(e.to_string().contains("timeout"));

        let e = ObservationError::PersistenceError("disk full".into());
        assert!(e.to_string().contains("disk full"));
    }

    #[test]
    fn io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let obs_err: ObservationError = io_err.into();
        assert!(obs_err.to_string().contains("file missing"));
    }

    #[test]
    fn result_type_works() {
        let ok: ObservationResult<u32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: ObservationResult<u32> = Err(ObservationError::LockError);
        assert!(err.is_err());
    }
}
