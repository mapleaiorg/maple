//! MAPLE Fleet Tenancy -- multi-tenant isolation and resource quotas.
//!
//! Manages tenants with configurable isolation levels and resource quotas.
//! Tracks usage against quotas to enforce limits.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum TenancyError {
    #[error("tenant not found: {0}")]
    NotFound(String),
    #[error("tenant already exists: {0}")]
    AlreadyExists(String),
    #[error("quota exceeded for tenant {tenant}: {resource} (limit: {limit}, current: {current})")]
    QuotaExceeded {
        tenant: String,
        resource: String,
        limit: u64,
        current: u64,
    },
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

pub type TenancyResult<T> = Result<T, TenancyError>;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Unique identifier for a tenant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(pub String);

impl TenantId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_str(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl Default for TenantId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TenantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Level of isolation between tenants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// Tenants share resources on the same infrastructure.
    Shared,
    /// Tenants have separate namespaces but share infrastructure.
    Namespace,
    /// Tenants have fully dedicated infrastructure.
    Dedicated,
}

/// Resource quota limits for a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub max_instances: u64,
    pub max_memory_mb: u64,
    pub max_models: u64,
    pub max_storage_gb: u64,
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            max_instances: 10,
            max_memory_mb: 4096,
            max_models: 5,
            max_storage_gb: 50,
        }
    }
}

/// Configuration for a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantConfig {
    pub name: String,
    pub resource_quotas: ResourceQuota,
    pub isolation_level: IsolationLevel,
    pub labels: HashMap<String, String>,
}

impl TenantConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            resource_quotas: ResourceQuota::default(),
            isolation_level: IsolationLevel::Shared,
            labels: HashMap::new(),
        }
    }
}

/// A tenant in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: TenantId,
    pub config: TenantConfig,
    pub usage: TenantUsage,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub active: bool,
}

/// Current resource usage for a tenant.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TenantUsage {
    pub instances: u64,
    pub memory_mb: u64,
    pub models: u64,
    pub storage_gb: u64,
}

impl TenantUsage {
    /// Check if usage is within quota limits.
    pub fn within_quota(&self, quota: &ResourceQuota) -> bool {
        self.instances <= quota.max_instances
            && self.memory_mb <= quota.max_memory_mb
            && self.models <= quota.max_models
            && self.storage_gb <= quota.max_storage_gb
    }

    /// Get the usage percentage for each resource.
    pub fn usage_percentages(&self, quota: &ResourceQuota) -> HashMap<String, f64> {
        let mut pcts = HashMap::new();
        if quota.max_instances > 0 {
            pcts.insert("instances".into(), self.instances as f64 / quota.max_instances as f64 * 100.0);
        }
        if quota.max_memory_mb > 0 {
            pcts.insert("memory_mb".into(), self.memory_mb as f64 / quota.max_memory_mb as f64 * 100.0);
        }
        if quota.max_models > 0 {
            pcts.insert("models".into(), self.models as f64 / quota.max_models as f64 * 100.0);
        }
        if quota.max_storage_gb > 0 {
            pcts.insert("storage_gb".into(), self.storage_gb as f64 / quota.max_storage_gb as f64 * 100.0);
        }
        pcts
    }
}

// ---------------------------------------------------------------------------
// Tenant Manager
// ---------------------------------------------------------------------------

/// Manages tenants and their lifecycle.
pub struct TenantManager {
    tenants: HashMap<TenantId, Tenant>,
}

impl Default for TenantManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TenantManager {
    pub fn new() -> Self {
        Self {
            tenants: HashMap::new(),
        }
    }

    /// Create a new tenant.
    pub fn create(&mut self, config: TenantConfig) -> TenancyResult<Tenant> {
        // Check for duplicate names
        if self.tenants.values().any(|t| t.config.name == config.name) {
            return Err(TenancyError::AlreadyExists(config.name));
        }
        let id = TenantId::new();
        let now = Utc::now();
        let tenant = Tenant {
            id: id.clone(),
            config,
            usage: TenantUsage::default(),
            created_at: now,
            updated_at: now,
            active: true,
        };
        self.tenants.insert(id, tenant.clone());
        Ok(tenant)
    }

    /// Get a tenant by ID.
    pub fn get(&self, id: &TenantId) -> TenancyResult<&Tenant> {
        self.tenants
            .get(id)
            .ok_or_else(|| TenancyError::NotFound(id.to_string()))
    }

    /// Update a tenant's configuration.
    pub fn update(&mut self, id: &TenantId, config: TenantConfig) -> TenancyResult<Tenant> {
        let tenant = self
            .tenants
            .get_mut(id)
            .ok_or_else(|| TenancyError::NotFound(id.to_string()))?;
        tenant.config = config;
        tenant.updated_at = Utc::now();
        Ok(tenant.clone())
    }

    /// Deactivate (soft-delete) a tenant.
    pub fn delete(&mut self, id: &TenantId) -> TenancyResult<()> {
        let tenant = self
            .tenants
            .get_mut(id)
            .ok_or_else(|| TenancyError::NotFound(id.to_string()))?;
        tenant.active = false;
        tenant.updated_at = Utc::now();
        Ok(())
    }

    /// List all active tenants.
    pub fn list(&self) -> Vec<&Tenant> {
        self.tenants.values().filter(|t| t.active).collect()
    }

    /// List all tenants including inactive.
    pub fn list_all(&self) -> Vec<&Tenant> {
        self.tenants.values().collect()
    }

    /// Update the usage for a tenant and check quotas.
    pub fn update_usage(&mut self, id: &TenantId, usage: TenantUsage) -> TenancyResult<()> {
        let tenant = self
            .tenants
            .get_mut(id)
            .ok_or_else(|| TenancyError::NotFound(id.to_string()))?;

        if !usage.within_quota(&tenant.config.resource_quotas) {
            // Find which resource is exceeded
            let quota = &tenant.config.resource_quotas;
            if usage.instances > quota.max_instances {
                return Err(TenancyError::QuotaExceeded {
                    tenant: tenant.config.name.clone(),
                    resource: "instances".into(),
                    limit: quota.max_instances,
                    current: usage.instances,
                });
            }
            if usage.memory_mb > quota.max_memory_mb {
                return Err(TenancyError::QuotaExceeded {
                    tenant: tenant.config.name.clone(),
                    resource: "memory_mb".into(),
                    limit: quota.max_memory_mb,
                    current: usage.memory_mb,
                });
            }
        }

        tenant.usage = usage;
        tenant.updated_at = Utc::now();
        Ok(())
    }

    /// Get usage for a tenant.
    pub fn get_usage(&self, id: &TenantId) -> TenancyResult<&TenantUsage> {
        let tenant = self
            .tenants
            .get(id)
            .ok_or_else(|| TenancyError::NotFound(id.to_string()))?;
        Ok(&tenant.usage)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tenant() {
        let mut mgr = TenantManager::new();
        let tenant = mgr.create(TenantConfig::new("acme-corp")).unwrap();
        assert_eq!(tenant.config.name, "acme-corp");
        assert!(tenant.active);
    }

    #[test]
    fn test_duplicate_name() {
        let mut mgr = TenantManager::new();
        mgr.create(TenantConfig::new("acme-corp")).unwrap();
        let result = mgr.create(TenantConfig::new("acme-corp"));
        assert!(result.is_err());
    }

    #[test]
    fn test_get_tenant() {
        let mut mgr = TenantManager::new();
        let created = mgr.create(TenantConfig::new("acme-corp")).unwrap();
        let fetched = mgr.get(&created.id).unwrap();
        assert_eq!(fetched.config.name, "acme-corp");
    }

    #[test]
    fn test_update_tenant() {
        let mut mgr = TenantManager::new();
        let created = mgr.create(TenantConfig::new("acme-corp")).unwrap();
        let mut new_config = TenantConfig::new("acme-corp-updated");
        new_config.isolation_level = IsolationLevel::Dedicated;
        let updated = mgr.update(&created.id, new_config).unwrap();
        assert_eq!(updated.config.name, "acme-corp-updated");
        assert_eq!(updated.config.isolation_level, IsolationLevel::Dedicated);
    }

    #[test]
    fn test_delete_tenant() {
        let mut mgr = TenantManager::new();
        let created = mgr.create(TenantConfig::new("acme-corp")).unwrap();
        mgr.delete(&created.id).unwrap();
        let tenant = mgr.get(&created.id).unwrap();
        assert!(!tenant.active);
    }

    #[test]
    fn test_list_active_only() {
        let mut mgr = TenantManager::new();
        let t1 = mgr.create(TenantConfig::new("tenant-1")).unwrap();
        mgr.create(TenantConfig::new("tenant-2")).unwrap();
        mgr.delete(&t1.id).unwrap();
        assert_eq!(mgr.list().len(), 1);
        assert_eq!(mgr.list_all().len(), 2);
    }

    #[test]
    fn test_usage_within_quota() {
        let usage = TenantUsage {
            instances: 5,
            memory_mb: 2048,
            models: 3,
            storage_gb: 25,
        };
        assert!(usage.within_quota(&ResourceQuota::default()));
    }

    #[test]
    fn test_usage_exceeds_quota() {
        let usage = TenantUsage {
            instances: 100,
            memory_mb: 2048,
            models: 3,
            storage_gb: 25,
        };
        assert!(!usage.within_quota(&ResourceQuota::default()));
    }

    #[test]
    fn test_quota_enforcement() {
        let mut mgr = TenantManager::new();
        let tenant = mgr.create(TenantConfig::new("limited")).unwrap();
        let usage = TenantUsage {
            instances: 100,
            memory_mb: 100,
            models: 1,
            storage_gb: 1,
        };
        let result = mgr.update_usage(&tenant.id, usage);
        assert!(result.is_err());
    }

    #[test]
    fn test_usage_percentages() {
        let usage = TenantUsage {
            instances: 5,
            memory_mb: 2048,
            models: 5,
            storage_gb: 25,
        };
        let pcts = usage.usage_percentages(&ResourceQuota::default());
        assert!((pcts["instances"] - 50.0).abs() < 0.01);
        assert!((pcts["models"] - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_not_found() {
        let mgr = TenantManager::new();
        let fake_id = TenantId::from_str("nonexistent");
        assert!(mgr.get(&fake_id).is_err());
    }
}
