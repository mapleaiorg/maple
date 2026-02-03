//! Audit entry types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// An audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique entry ID
    pub id: Uuid,

    /// Timestamp of the event
    pub timestamp: DateTime<Utc>,

    /// Platform profile
    pub platform: String,

    /// Actor who performed the action
    pub actor: AuditActor,

    /// Action performed
    pub action: AuditAction,

    /// Resource affected
    pub resource: AuditResource,

    /// Outcome of the action
    pub outcome: AuditOutcome,

    /// Additional context/details
    pub context: HashMap<String, serde_json::Value>,

    /// Hash of the previous entry (for chain integrity)
    pub previous_hash: Option<String>,

    /// Hash of this entry
    pub entry_hash: String,

    /// Trace ID for correlation
    pub trace_id: Option<String>,
}

impl AuditEntry {
    /// Create a new audit entry builder
    pub fn builder() -> AuditEntryBuilder {
        AuditEntryBuilder::new()
    }
}

/// Builder for audit entries
#[derive(Debug, Default)]
pub struct AuditEntryBuilder {
    platform: Option<String>,
    actor: Option<AuditActor>,
    action: Option<AuditAction>,
    resource: Option<AuditResource>,
    outcome: Option<AuditOutcome>,
    context: HashMap<String, serde_json::Value>,
    trace_id: Option<String>,
}

impl AuditEntryBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set platform
    pub fn platform(mut self, platform: impl Into<String>) -> Self {
        self.platform = Some(platform.into());
        self
    }

    /// Set actor
    pub fn actor(mut self, actor: AuditActor) -> Self {
        self.actor = Some(actor);
        self
    }

    /// Set action
    pub fn action(mut self, action: AuditAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Set resource
    pub fn resource(mut self, resource: AuditResource) -> Self {
        self.resource = Some(resource);
        self
    }

    /// Set outcome
    pub fn outcome(mut self, outcome: AuditOutcome) -> Self {
        self.outcome = Some(outcome);
        self
    }

    /// Add context value
    pub fn context(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.context.insert(key.into(), v);
        }
        self
    }

    /// Set trace ID
    pub fn trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Build the entry (without hash - hash must be set by sink)
    pub fn build(self) -> Result<PartialAuditEntry, &'static str> {
        Ok(PartialAuditEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            platform: self.platform.ok_or("platform is required")?,
            actor: self.actor.ok_or("actor is required")?,
            action: self.action.ok_or("action is required")?,
            resource: self.resource.ok_or("resource is required")?,
            outcome: self.outcome.ok_or("outcome is required")?,
            context: self.context,
            trace_id: self.trace_id,
        })
    }
}

/// Partial audit entry (before hashing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialAuditEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub platform: String,
    pub actor: AuditActor,
    pub action: AuditAction,
    pub resource: AuditResource,
    pub outcome: AuditOutcome,
    pub context: HashMap<String, serde_json::Value>,
    pub trace_id: Option<String>,
}

impl PartialAuditEntry {
    /// Convert to full entry with hashes
    pub fn finalize(self, previous_hash: Option<String>) -> AuditEntry {
        use sha2::{Digest, Sha256};

        // Compute hash of this entry
        let hash_input = format!(
            "{}{}{}{}{}{}{}{}",
            self.id,
            self.timestamp.to_rfc3339(),
            self.platform,
            serde_json::to_string(&self.actor).unwrap_or_default(),
            serde_json::to_string(&self.action).unwrap_or_default(),
            serde_json::to_string(&self.resource).unwrap_or_default(),
            serde_json::to_string(&self.outcome).unwrap_or_default(),
            previous_hash.as_deref().unwrap_or("")
        );

        let mut hasher = Sha256::new();
        hasher.update(hash_input.as_bytes());
        let entry_hash = hex::encode(hasher.finalize());

        AuditEntry {
            id: self.id,
            timestamp: self.timestamp,
            platform: self.platform,
            actor: self.actor,
            action: self.action,
            resource: self.resource,
            outcome: self.outcome,
            context: self.context,
            previous_hash,
            entry_hash,
            trace_id: self.trace_id,
        }
    }
}

/// Actor who performed an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditActor {
    /// Actor type
    pub actor_type: ActorType,

    /// Actor identifier
    pub id: String,

    /// Actor display name
    pub name: Option<String>,

    /// Additional attributes
    #[serde(default)]
    pub attributes: HashMap<String, String>,
}

impl AuditActor {
    /// Create a system actor
    pub fn system(component: impl Into<String>) -> Self {
        Self {
            actor_type: ActorType::System,
            id: component.into(),
            name: None,
            attributes: HashMap::new(),
        }
    }

    /// Create a user actor
    pub fn user(id: impl Into<String>, name: Option<String>) -> Self {
        Self {
            actor_type: ActorType::User,
            id: id.into(),
            name,
            attributes: HashMap::new(),
        }
    }

    /// Create an agent actor
    pub fn agent(id: impl Into<String>, name: Option<String>) -> Self {
        Self {
            actor_type: ActorType::Agent,
            id: id.into(),
            name,
            attributes: HashMap::new(),
        }
    }

    /// Create a service actor
    pub fn service(id: impl Into<String>) -> Self {
        Self {
            actor_type: ActorType::Service,
            id: id.into(),
            name: None,
            attributes: HashMap::new(),
        }
    }

    /// Add an attribute
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

/// Actor types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorType {
    User,
    System,
    Agent,
    Service,
}

/// Audit action types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    // Spec actions
    SpecCreated,
    SpecUpdated,
    SpecDeleted,

    // Deployment actions
    DeploymentCreated,
    DeploymentUpdated,
    DeploymentScaled,
    DeploymentRolledBack,
    DeploymentDeleted,

    // Instance actions
    InstanceStarted,
    InstanceStopped,
    InstanceRestarted,
    InstanceMigrated,
    InstanceHealthChanged,

    // Policy actions
    PolicyEvaluated,
    PolicyViolation,
    PolicyEnforced,

    // System actions
    SystemStarted,
    SystemStopped,
    ConfigChanged,

    // Custom action
    Custom(String),
}

/// Resource affected by an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResource {
    /// Resource type
    pub resource_type: ResourceType,

    /// Resource identifier
    pub id: String,

    /// Resource name (optional)
    pub name: Option<String>,

    /// Additional attributes
    #[serde(default)]
    pub attributes: HashMap<String, String>,
}

impl AuditResource {
    /// Create a spec resource
    pub fn spec(id: impl Into<String>) -> Self {
        Self {
            resource_type: ResourceType::Spec,
            id: id.into(),
            name: None,
            attributes: HashMap::new(),
        }
    }

    /// Create a deployment resource
    pub fn deployment(id: impl Into<String>) -> Self {
        Self {
            resource_type: ResourceType::Deployment,
            id: id.into(),
            name: None,
            attributes: HashMap::new(),
        }
    }

    /// Create an instance resource
    pub fn instance(id: impl Into<String>) -> Self {
        Self {
            resource_type: ResourceType::Instance,
            id: id.into(),
            name: None,
            attributes: HashMap::new(),
        }
    }

    /// Create a policy resource
    pub fn policy(id: impl Into<String>) -> Self {
        Self {
            resource_type: ResourceType::Policy,
            id: id.into(),
            name: None,
            attributes: HashMap::new(),
        }
    }

    /// Create a system resource
    pub fn system(component: impl Into<String>) -> Self {
        Self {
            resource_type: ResourceType::System,
            id: component.into(),
            name: None,
            attributes: HashMap::new(),
        }
    }

    /// Set name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add an attribute
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

/// Resource types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Spec,
    Deployment,
    Instance,
    Policy,
    System,
    Config,
    Custom,
}

/// Outcome of an action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Success,
    Failure { reason: String },
    Denied { reason: String },
    Partial { details: String },
}

impl AuditOutcome {
    /// Create a success outcome
    pub fn success() -> Self {
        Self::Success
    }

    /// Create a failure outcome
    pub fn failure(reason: impl Into<String>) -> Self {
        Self::Failure {
            reason: reason.into(),
        }
    }

    /// Create a denied outcome
    pub fn denied(reason: impl Into<String>) -> Self {
        Self::Denied {
            reason: reason.into(),
        }
    }

    /// Create a partial outcome
    pub fn partial(details: impl Into<String>) -> Self {
        Self::Partial {
            details: details.into(),
        }
    }

    /// Check if outcome is successful
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_entry_builder() {
        let entry = AuditEntry::builder()
            .platform("development")
            .actor(AuditActor::system("scheduler"))
            .action(AuditAction::InstanceStarted)
            .resource(AuditResource::instance("instance-1"))
            .outcome(AuditOutcome::success())
            .context("duration_ms", 1500)
            .trace_id("trace-123")
            .build()
            .unwrap();

        assert_eq!(entry.platform, "development");
        assert!(entry.context.contains_key("duration_ms"));
    }

    #[test]
    fn test_finalize_entry() {
        let partial = AuditEntry::builder()
            .platform("development")
            .actor(AuditActor::system("test"))
            .action(AuditAction::SystemStarted)
            .resource(AuditResource::system("palm-daemon"))
            .outcome(AuditOutcome::success())
            .build()
            .unwrap();

        let entry = partial.finalize(None);
        assert!(!entry.entry_hash.is_empty());
        assert!(entry.previous_hash.is_none());

        // Create a second entry with chain
        let partial2 = AuditEntry::builder()
            .platform("development")
            .actor(AuditActor::system("test"))
            .action(AuditAction::ConfigChanged)
            .resource(AuditResource::system("palm-daemon"))
            .outcome(AuditOutcome::success())
            .build()
            .unwrap();

        let entry2 = partial2.finalize(Some(entry.entry_hash.clone()));
        assert_eq!(entry2.previous_hash, Some(entry.entry_hash));
    }

    #[test]
    fn test_audit_actors() {
        let system = AuditActor::system("scheduler");
        assert_eq!(system.actor_type, ActorType::System);

        let user = AuditActor::user("user-123", Some("Alice".to_string()));
        assert_eq!(user.actor_type, ActorType::User);
        assert_eq!(user.name, Some("Alice".to_string()));

        let agent = AuditActor::agent("agent-456", None).with_attribute("version", "1.0.0");
        assert_eq!(agent.actor_type, ActorType::Agent);
        assert!(agent.attributes.contains_key("version"));
    }

    #[test]
    fn test_audit_outcomes() {
        assert!(AuditOutcome::success().is_success());
        assert!(!AuditOutcome::failure("error").is_success());
        assert!(!AuditOutcome::denied("unauthorized").is_success());
    }
}
