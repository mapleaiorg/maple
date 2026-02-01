//! PALM Observability
//!
//! Provides metrics, tracing, and audit infrastructure for PALM components.
//!
//! ## Features
//!
//! - **Metrics**: Prometheus-compatible metrics for deployments, instances, health, and resonance
//! - **Tracing**: OpenTelemetry integration for distributed tracing
//! - **Audit**: Tamper-evident audit logging with integrity chain
//! - **Correlation**: Event correlation across PALM components

pub mod audit;
pub mod correlation;
pub mod error;
pub mod metrics;
pub mod tracing;

pub use audit::{AuditEntry, AuditQuery, AuditSink, FileAuditSink, MemoryAuditSink};
pub use correlation::{CorrelatedEvent, CorrelationEngine, CorrelationId, EventCorrelation};
pub use error::ObservabilityError;
pub use metrics::{MetricsRegistry, PalmMetrics};
pub use tracing::{init_tracing, PalmContext, PalmSpan, TracingConfig};

use std::path::PathBuf;
use std::sync::Arc;

/// Configuration for observability infrastructure
#[derive(Debug, Clone, Default)]
pub struct ObservabilityConfig {
    /// Tracing configuration
    pub tracing: Option<TracingConfig>,
    /// Audit configuration
    pub audit: Option<AuditConfig>,
    /// Metrics prefix (default: "palm")
    pub metrics_prefix: Option<String>,
}

/// Configuration for audit logging
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// Storage backend
    pub backend: AuditBackend,
    /// Retention period in days
    pub retention_days: u32,
    /// Enable integrity chain verification
    pub enable_integrity_chain: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            backend: AuditBackend::Memory,
            retention_days: 90,
            enable_integrity_chain: true,
        }
    }
}

/// Audit storage backend
#[derive(Debug, Clone)]
pub enum AuditBackend {
    /// In-memory storage (for development/testing)
    Memory,
    /// File-based storage
    File {
        /// Path to audit log file
        path: String,
    },
}

/// Handle to observability infrastructure
pub struct ObservabilityHandle {
    /// Metrics registry
    pub metrics: MetricsRegistry,
    /// Audit sink (if configured)
    pub audit_sink: Option<Arc<dyn AuditSink>>,
    /// Event correlation engine
    pub correlation: CorrelationEngine,
}

/// Initialize all observability infrastructure
pub async fn init(config: ObservabilityConfig) -> error::Result<ObservabilityHandle> {
    // Initialize tracing
    if let Some(ref tracing_config) = config.tracing {
        let _ = init_tracing(tracing_config)?;
    }

    // Initialize metrics registry
    let metrics = match config.metrics_prefix {
        Some(prefix) => MetricsRegistry::with_prefix(&prefix),
        None => MetricsRegistry::new(),
    };

    // Initialize audit sink
    let audit_sink: Option<Arc<dyn AuditSink>> = match config.audit {
        Some(audit_config) => Some(create_audit_sink(audit_config).await?),
        None => None,
    };

    // Initialize correlation engine
    let correlation = CorrelationEngine::new();

    Ok(ObservabilityHandle {
        metrics,
        audit_sink,
        correlation,
    })
}

/// Create an audit sink based on configuration
async fn create_audit_sink(config: AuditConfig) -> error::Result<Arc<dyn AuditSink>> {
    match config.backend {
        AuditBackend::Memory => Ok(Arc::new(MemoryAuditSink::new())),
        AuditBackend::File { path } => {
            let sink = FileAuditSink::new(PathBuf::from(path)).await?;
            Ok(Arc::new(sink))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_default_config() {
        let config = ObservabilityConfig::default();
        let handle = init(config).await.unwrap();

        // Record a metric to ensure the registry works
        handle.metrics.palm().deployment.record_operation("test", "create", "success", 1.0);

        // Metrics should be initialized and exportable
        assert!(!handle.metrics.export().is_empty());

        // No audit sink by default
        assert!(handle.audit_sink.is_none());
    }

    #[tokio::test]
    async fn test_init_with_audit() {
        let config = ObservabilityConfig {
            tracing: None,
            audit: Some(AuditConfig::default()),
            metrics_prefix: Some("test".to_string()),
        };

        let handle = init(config).await.unwrap();
        assert!(handle.audit_sink.is_some());
    }
}
