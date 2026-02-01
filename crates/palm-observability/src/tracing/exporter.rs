//! Tracing initialization and export

use opentelemetry_sdk::{
    runtime,
    trace::{Config, Tracer},
    Resource,
};
use opentelemetry::KeyValue;
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Configuration for tracing initialization
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Service name for traces
    pub service_name: String,

    /// Platform profile
    pub platform: String,

    /// OTLP endpoint (if using OpenTelemetry export)
    pub otlp_endpoint: Option<String>,

    /// Enable console logging
    pub enable_console: bool,

    /// Enable JSON format for console
    pub json_format: bool,

    /// Log level filter
    pub log_level: String,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: "palm".to_string(),
            platform: "development".to_string(),
            otlp_endpoint: None,
            enable_console: true,
            json_format: false,
            log_level: "info".to_string(),
        }
    }
}

impl TracingConfig {
    /// Create config for a specific service
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            ..Default::default()
        }
    }

    /// Set platform
    pub fn with_platform(mut self, platform: impl Into<String>) -> Self {
        self.platform = platform.into();
        self
    }

    /// Set OTLP endpoint
    pub fn with_otlp_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.otlp_endpoint = Some(endpoint.into());
        self
    }

    /// Enable JSON format
    pub fn with_json_format(mut self) -> Self {
        self.json_format = true;
        self
    }

    /// Set log level
    pub fn with_log_level(mut self, level: impl Into<String>) -> Self {
        self.log_level = level.into();
        self
    }
}

/// Initialize tracing with the given configuration
pub fn init_tracing(config: &TracingConfig) -> crate::error::Result<Option<Tracer>> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    // Setup OpenTelemetry if endpoint is configured
    let tracer = if let Some(ref endpoint) = config.otlp_endpoint {
        Some(init_otlp_tracer(config, endpoint)?)
    } else {
        None
    };

    // Build subscriber layers
    let subscriber = tracing_subscriber::registry().with(env_filter);

    if config.enable_console {
        if config.json_format {
            let fmt_layer = fmt::layer()
                .json()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true);
            subscriber.with(fmt_layer).init();
        } else {
            let fmt_layer = fmt::layer()
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false);
            subscriber.with(fmt_layer).init();
        }
    } else if tracer.is_some() {
        // OpenTelemetry only, no console
        subscriber.init();
    } else {
        // No output configured, use default
        let fmt_layer = fmt::layer().with_target(true);
        subscriber.with(fmt_layer).init();
    }

    Ok(tracer)
}

/// Initialize OTLP exporter and return a Tracer
fn init_otlp_tracer(config: &TracingConfig, endpoint: &str) -> crate::error::Result<Tracer> {
    use opentelemetry_otlp::WithExportConfig;

    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint);

    let trace_config = Config::default().with_resource(Resource::new(vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("palm.platform", config.platform.clone()),
    ]));

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(trace_config)
        .install_batch(runtime::Tokio)
        .map_err(|e| crate::error::ObservabilityError::Tracing(e.to_string()))?;

    Ok(tracer)
}

/// Shutdown tracing and flush pending spans
pub fn shutdown_tracing() {
    opentelemetry::global::shutdown_tracer_provider();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracing_config() {
        let config = TracingConfig::new("palm-daemon")
            .with_platform("development")
            .with_log_level("debug")
            .with_json_format();

        assert_eq!(config.service_name, "palm-daemon");
        assert_eq!(config.platform, "development");
        assert_eq!(config.log_level, "debug");
        assert!(config.json_format);
    }

    #[test]
    fn test_config_with_otlp() {
        let config = TracingConfig::new("palm-daemon")
            .with_otlp_endpoint("http://localhost:4317");

        assert_eq!(config.otlp_endpoint, Some("http://localhost:4317".to_string()));
    }
}
