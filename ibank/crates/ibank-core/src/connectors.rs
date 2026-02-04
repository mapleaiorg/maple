use crate::error::IBankError;
use crate::types::{AccountableWireMessage, ConnectorReceipt};
use std::collections::HashMap;
use std::sync::Arc;

/// Pluggable side-effect connector.
///
/// Implementations map iBank commitments to external rails while preserving accountable context.
pub trait SettlementConnector: Send + Sync {
    fn rail(&self) -> &'static str;

    fn execute(&self, message: &AccountableWireMessage) -> Result<ConnectorReceipt, IBankError>;
}

/// Registry for connector plugins.
#[derive(Default)]
pub struct ConnectorRegistry {
    connectors: HashMap<String, Arc<dyn SettlementConnector>>,
}

impl ConnectorRegistry {
    pub fn new() -> Self {
        Self {
            connectors: HashMap::new(),
        }
    }

    pub fn register(&mut self, connector: Arc<dyn SettlementConnector>) {
        self.connectors
            .insert(connector.rail().to_string(), connector);
    }

    pub fn get(&self, rail: &str) -> Option<Arc<dyn SettlementConnector>> {
        self.connectors.get(rail).cloned()
    }

    pub fn has(&self, rail: &str) -> bool {
        self.connectors.contains_key(rail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::BTreeMap;

    struct DummyConnector;

    impl SettlementConnector for DummyConnector {
        fn rail(&self) -> &'static str {
            "dummy"
        }

        fn execute(
            &self,
            _message: &AccountableWireMessage,
        ) -> Result<ConnectorReceipt, IBankError> {
            Ok(ConnectorReceipt {
                settlement_id: "s1".to_string(),
                rail: "dummy".to_string(),
                settled_at: Utc::now(),
                metadata: BTreeMap::new(),
            })
        }
    }

    #[test]
    fn connector_registry_roundtrip() {
        let mut registry = ConnectorRegistry::new();
        registry.register(Arc::new(DummyConnector));
        assert!(registry.has("dummy"));
    }
}
