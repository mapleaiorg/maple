//! Metrics exporter for Prometheus scraping

use prometheus::{Encoder, Registry, TextEncoder};

/// Export metrics in Prometheus text format
pub fn export_metrics(registry: &Registry) -> String {
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

/// HTTP handler for metrics endpoint (requires "http" feature)
#[cfg(feature = "http")]
pub mod http {
    use axum::{
        extract::State,
        http::StatusCode,
        response::{IntoResponse, Response},
    };
    use prometheus::Registry;
    use std::sync::Arc;

    /// Metrics endpoint state
    #[derive(Clone)]
    pub struct MetricsState {
        pub registry: Arc<Registry>,
    }

    impl MetricsState {
        pub fn new(registry: Arc<Registry>) -> Self {
            Self { registry }
        }
    }

    /// Handler for GET /metrics
    pub async fn metrics_handler(State(state): State<MetricsState>) -> Response {
        let metrics = super::export_metrics(&state.registry);
        (
            StatusCode::OK,
            [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
            metrics,
        )
            .into_response()
    }

    /// Create an axum router for metrics
    pub fn metrics_router(registry: Arc<Registry>) -> axum::Router {
        use axum::routing::get;

        axum::Router::new()
            .route("/metrics", get(metrics_handler))
            .with_state(MetricsState::new(registry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prometheus::IntCounter;

    #[test]
    fn test_export_metrics() {
        let registry = Registry::new();
        let counter = IntCounter::new("test_counter", "A test counter").unwrap();
        registry.register(Box::new(counter.clone())).unwrap();
        counter.inc();

        let output = export_metrics(&registry);
        assert!(output.contains("test_counter"));
        assert!(output.contains("1"));
    }
}
