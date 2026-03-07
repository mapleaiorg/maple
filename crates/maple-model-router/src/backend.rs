//! Backend pool abstraction for the model router.
//!
//! Since `maple-model-backends` does not exist yet, this module defines
//! the traits and lightweight types needed by the router to interact
//! with inference backends.

use maple_model_core::ModelCapability;
use serde::{Deserialize, Serialize};

/// Health status of a backend.
#[derive(Debug, Clone)]
pub struct BackendHealth {
    /// Whether the backend can accept requests.
    pub available: bool,
    /// Estimated latency in milliseconds.
    pub latency_ms: Option<u64>,
    /// Number of pending requests.
    pub pending_requests: u32,
}

/// Static pricing info for a backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendPricing {
    /// Cost per 1K input tokens.
    pub input_cost_per_1k: f64,
    /// Cost per 1K output tokens.
    pub output_cost_per_1k: f64,
}

/// Descriptor for a registered backend in the pool.
#[derive(Debug, Clone)]
pub struct BackendInfo {
    /// Unique backend identifier (e.g., "ollama-local", "openai-cloud").
    pub id: String,
    /// Models this backend can serve.
    pub supported_models: Vec<String>,
    /// Capabilities this backend supports.
    pub capabilities: Vec<ModelCapability>,
    /// Current health status.
    pub health: BackendHealth,
    /// Optional pricing information.
    pub pricing: Option<BackendPricing>,
    /// Estimated average latency in milliseconds for an average request.
    pub avg_latency_ms: Option<u64>,
}

/// A registry of available backends.
///
/// The router uses this to discover backends, check health, and select
/// candidates for routing decisions.
#[derive(Debug, Default)]
pub struct BackendRegistry {
    backends: Vec<BackendInfo>,
}

impl BackendRegistry {
    /// Create an empty backend registry.
    pub fn new() -> Self {
        Self {
            backends: Vec::new(),
        }
    }

    /// Register a new backend.
    pub fn register(&mut self, info: BackendInfo) {
        // Replace existing entry with the same id, or append.
        if let Some(existing) = self.backends.iter_mut().find(|b| b.id == info.id) {
            *existing = info;
        } else {
            self.backends.push(info);
        }
    }

    /// Look up a backend by id.
    pub fn get(&self, id: &str) -> Option<&BackendInfo> {
        self.backends.iter().find(|b| b.id == id)
    }

    /// Return all registered backends.
    pub fn all(&self) -> &[BackendInfo] {
        &self.backends
    }

    /// Update the health of a specific backend.
    pub fn update_health(&mut self, id: &str, health: BackendHealth) {
        if let Some(backend) = self.backends.iter_mut().find(|b| b.id == id) {
            backend.health = health;
        }
    }

    /// Return ids of all backends that are currently available.
    pub fn available_backends(&self) -> Vec<&str> {
        self.backends
            .iter()
            .filter(|b| b.health.available)
            .map(|b| b.id.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_backend(id: &str, available: bool) -> BackendInfo {
        BackendInfo {
            id: id.to_string(),
            supported_models: vec!["llama3".to_string()],
            capabilities: vec![ModelCapability::Chat],
            health: BackendHealth {
                available,
                latency_ms: Some(50),
                pending_requests: 0,
            },
            pricing: None,
            avg_latency_ms: Some(100),
        }
    }

    #[test]
    fn test_register_and_get() {
        let mut registry = BackendRegistry::new();
        registry.register(make_backend("ollama", true));
        assert!(registry.get("ollama").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_available_backends() {
        let mut registry = BackendRegistry::new();
        registry.register(make_backend("ollama", true));
        registry.register(make_backend("openai", false));
        let available = registry.available_backends();
        assert_eq!(available, vec!["ollama"]);
    }
}
