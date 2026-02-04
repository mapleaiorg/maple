//! Definition registry: stores and retrieves workflow definitions
//!
//! Workflow definitions are immutable once registered. To modify,
//! create a new version. The registry tracks all versions.

use std::collections::HashMap;
use workflow_types::{WorkflowDefinition, WorkflowDefinitionId, WorkflowError, WorkflowResult};

/// Registry of workflow definitions
#[derive(Clone, Debug)]
pub struct DefinitionRegistry {
    /// All registered definitions, keyed by ID
    definitions: HashMap<WorkflowDefinitionId, WorkflowDefinition>,
    /// Index by name → list of definition IDs (for versioning)
    by_name: HashMap<String, Vec<WorkflowDefinitionId>>,
}

impl DefinitionRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
            by_name: HashMap::new(),
        }
    }

    /// Register a workflow definition
    ///
    /// Validates the definition before storing. Returns the definition ID.
    pub fn register(
        &mut self,
        definition: WorkflowDefinition,
    ) -> WorkflowResult<WorkflowDefinitionId> {
        // Validate the definition
        definition.validate()?;

        let id = definition.id.clone();
        let name = definition.name.clone();

        self.definitions.insert(id.clone(), definition);
        self.by_name.entry(name).or_default().push(id.clone());

        tracing::info!(definition_id = %id, "Workflow definition registered");
        Ok(id)
    }

    /// Get a definition by ID
    pub fn get(&self, id: &WorkflowDefinitionId) -> WorkflowResult<&WorkflowDefinition> {
        self.definitions
            .get(id)
            .ok_or_else(|| WorkflowError::DefinitionNotFound(id.clone()))
    }

    /// Get the latest version of a definition by name
    pub fn get_latest_by_name(&self, name: &str) -> Option<&WorkflowDefinition> {
        self.by_name
            .get(name)
            .and_then(|ids| ids.last())
            .and_then(|id| self.definitions.get(id))
    }

    /// Get all versions of a definition by name
    pub fn get_versions_by_name(&self, name: &str) -> Vec<&WorkflowDefinition> {
        self.by_name
            .get(name)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.definitions.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// List all registered definitions
    pub fn list(&self) -> Vec<&WorkflowDefinition> {
        self.definitions.values().collect()
    }

    /// Total number of registered definitions
    pub fn count(&self) -> usize {
        self.definitions.len()
    }

    /// Check if a definition exists
    pub fn contains(&self, id: &WorkflowDefinitionId) -> bool {
        self.definitions.contains_key(id)
    }

    /// Remove a definition (only if no instances reference it)
    pub fn remove(&mut self, id: &WorkflowDefinitionId) -> WorkflowResult<WorkflowDefinition> {
        let def = self
            .definitions
            .remove(id)
            .ok_or_else(|| WorkflowError::DefinitionNotFound(id.clone()))?;

        // Clean up the name index
        if let Some(ids) = self.by_name.get_mut(&def.name) {
            ids.retain(|i| i != id);
            if ids.is_empty() {
                self.by_name.remove(&def.name);
            }
        }

        tracing::info!(definition_id = %id, "Workflow definition removed");
        Ok(def)
    }
}

impl Default for DefinitionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use collective_types::CollectiveId;
    use resonator_types::ResonatorId;
    use workflow_types::{NodeId, WorkflowEdge, WorkflowNode};

    fn make_valid_definition(name: &str) -> WorkflowDefinition {
        let mut def =
            WorkflowDefinition::new(name, CollectiveId::new("test"), ResonatorId::new("author"));
        def.add_node(WorkflowNode::start("start")).unwrap();
        def.add_node(WorkflowNode::end("end")).unwrap();
        def.add_edge(WorkflowEdge::new(NodeId::new("start"), NodeId::new("end")))
            .unwrap();
        def
    }

    #[test]
    fn test_register_and_get() {
        let mut registry = DefinitionRegistry::new();
        let def = make_valid_definition("Test Workflow");
        let id = registry.register(def).unwrap();

        let retrieved = registry.get(&id).unwrap();
        assert_eq!(retrieved.name, "Test Workflow");
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_register_invalid() {
        let mut registry = DefinitionRegistry::new();
        // No start or end node
        let def =
            WorkflowDefinition::new("Bad", CollectiveId::new("test"), ResonatorId::new("author"));
        let result = registry.register(def);
        assert!(result.is_err());
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_get_by_name() {
        let mut registry = DefinitionRegistry::new();

        let def1 = make_valid_definition("Review Workflow");
        let def2 = make_valid_definition("Review Workflow");

        registry.register(def1).unwrap();
        let id2 = registry.register(def2).unwrap();

        let versions = registry.get_versions_by_name("Review Workflow");
        assert_eq!(versions.len(), 2);

        let latest = registry.get_latest_by_name("Review Workflow").unwrap();
        assert_eq!(latest.id, id2);

        assert!(registry.get_latest_by_name("Nonexistent").is_none());
    }

    #[test]
    fn test_list() {
        let mut registry = DefinitionRegistry::new();
        registry.register(make_valid_definition("A")).unwrap();
        registry.register(make_valid_definition("B")).unwrap();

        assert_eq!(registry.list().len(), 2);
    }

    #[test]
    fn test_remove() {
        let mut registry = DefinitionRegistry::new();
        let id = registry
            .register(make_valid_definition("Remove Me"))
            .unwrap();

        assert!(registry.contains(&id));
        let removed = registry.remove(&id).unwrap();
        assert_eq!(removed.name, "Remove Me");
        assert!(!registry.contains(&id));
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut registry = DefinitionRegistry::new();
        let result = registry.remove(&WorkflowDefinitionId::new("nonexistent"));
        assert!(matches!(result, Err(WorkflowError::DefinitionNotFound(_))));
    }

    #[test]
    fn test_get_nonexistent() {
        let registry = DefinitionRegistry::new();
        let result = registry.get(&WorkflowDefinitionId::new("nonexistent"));
        assert!(matches!(result, Err(WorkflowError::DefinitionNotFound(_))));
    }
}
