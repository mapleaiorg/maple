//! Agent registry trait and implementations
//!
//! The AgentRegistry manages agent specifications (templates for deployment).

use crate::error::Result;
use async_trait::async_trait;
use palm_types::{AgentSpec, AgentSpecId};
use semver::Version;

/// Registry for agent specifications
#[async_trait]
pub trait AgentRegistry: Send + Sync {
    /// Register a new agent spec
    async fn register(&self, spec: AgentSpec) -> Result<AgentSpecId>;

    /// Get an agent spec by ID
    async fn get(&self, id: &AgentSpecId) -> Result<Option<AgentSpec>>;

    /// Get an agent spec by name and version
    async fn get_by_name_version(
        &self,
        name: &str,
        version: &Version,
    ) -> Result<Option<AgentSpec>>;

    /// List all agent specs
    async fn list(&self) -> Result<Vec<AgentSpec>>;

    /// List all versions of a named spec
    async fn list_versions(&self, name: &str) -> Result<Vec<AgentSpec>>;

    /// Update an agent spec
    async fn update(&self, spec: AgentSpec) -> Result<()>;

    /// Delete an agent spec
    async fn delete(&self, id: &AgentSpecId) -> Result<()>;

    /// Check if a spec exists
    async fn exists(&self, id: &AgentSpecId) -> Result<bool>;
}
