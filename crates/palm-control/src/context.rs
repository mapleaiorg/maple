//! Request context for control plane operations
//!
//! The request context carries information about who is making a request,
//! what platform it's for, and any policy-related data.

use palm_types::{PlatformProfile, PolicyContext};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Context for a control plane request
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Unique request ID for tracing
    pub request_id: Uuid,
    /// Platform profile for this request
    pub platform: PlatformProfile,
    /// Actor making the request (user, service, etc.)
    pub actor: Actor,
    /// Policy context for authorization
    pub policy_context: PolicyContext,
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Optional correlation ID for distributed tracing
    pub correlation_id: Option<String>,
}

/// Actor making a control plane request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Actor {
    /// Human operator
    User {
        /// User identifier
        user_id: String,
        /// Assigned roles
        roles: Vec<String>,
    },
    /// Service account
    Service {
        /// Service identifier
        service_id: String,
        /// Granted scopes
        scopes: Vec<String>,
    },
    /// Internal system operation
    System {
        /// Component name
        component: String,
    },
    /// CLI tool
    Cli {
        /// Session identifier
        session_id: String,
    },
}

impl RequestContext {
    /// Create a new request context
    pub fn new(platform: PlatformProfile, actor: Actor) -> Self {
        Self {
            request_id: Uuid::new_v4(),
            platform,
            actor,
            policy_context: PolicyContext::default(),
            timestamp: chrono::Utc::now(),
            correlation_id: None,
        }
    }

    /// Set a correlation ID for distributed tracing
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Set the policy context
    pub fn with_policy_context(mut self, ctx: PolicyContext) -> Self {
        self.policy_context = ctx;
        self
    }

    /// Create a system context for internal operations
    pub fn system(platform: PlatformProfile, component: impl Into<String>) -> Self {
        Self::new(platform, Actor::System {
            component: component.into(),
        })
    }

    /// Create a user context
    pub fn user(
        platform: PlatformProfile,
        user_id: impl Into<String>,
        roles: Vec<String>,
    ) -> Self {
        Self::new(platform, Actor::User {
            user_id: user_id.into(),
            roles,
        })
    }

    /// Create a service context
    pub fn service(
        platform: PlatformProfile,
        service_id: impl Into<String>,
        scopes: Vec<String>,
    ) -> Self {
        Self::new(platform, Actor::Service {
            service_id: service_id.into(),
            scopes,
        })
    }

    /// Create a CLI context
    pub fn cli(platform: PlatformProfile, session_id: impl Into<String>) -> Self {
        Self::new(platform, Actor::Cli {
            session_id: session_id.into(),
        })
    }

    /// Check if the actor has a specific role (for User actors)
    pub fn has_role(&self, role: &str) -> bool {
        match &self.actor {
            Actor::User { roles, .. } => roles.iter().any(|r| r == role),
            Actor::System { .. } => true, // System has all roles
            _ => false,
        }
    }

    /// Check if the actor has a specific scope (for Service actors)
    pub fn has_scope(&self, scope: &str) -> bool {
        match &self.actor {
            Actor::Service { scopes, .. } => scopes.iter().any(|s| s == scope),
            Actor::System { .. } => true, // System has all scopes
            _ => false,
        }
    }

    /// Get the actor's identity string
    pub fn actor_id(&self) -> String {
        match &self.actor {
            Actor::User { user_id, .. } => format!("user:{}", user_id),
            Actor::Service { service_id, .. } => format!("service:{}", service_id),
            Actor::System { component } => format!("system:{}", component),
            Actor::Cli { session_id } => format!("cli:{}", session_id),
        }
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::system(PlatformProfile::Development, "default")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_context() {
        let ctx = RequestContext::user(
            PlatformProfile::Mapleverse,
            "alice",
            vec!["admin".into(), "operator".into()],
        );

        assert!(ctx.has_role("admin"));
        assert!(ctx.has_role("operator"));
        assert!(!ctx.has_role("viewer"));
        assert_eq!(ctx.actor_id(), "user:alice");
    }

    #[test]
    fn test_service_context() {
        let ctx = RequestContext::service(
            PlatformProfile::IBank,
            "deployment-service",
            vec!["deploy:write".into()],
        );

        assert!(ctx.has_scope("deploy:write"));
        assert!(!ctx.has_scope("deploy:delete"));
        assert_eq!(ctx.actor_id(), "service:deployment-service");
    }

    #[test]
    fn test_system_context() {
        let ctx = RequestContext::system(PlatformProfile::Finalverse, "health-monitor");

        // System has all roles and scopes
        assert!(ctx.has_role("any-role"));
        assert!(ctx.has_scope("any-scope"));
        assert_eq!(ctx.actor_id(), "system:health-monitor");
    }

    #[test]
    fn test_correlation_id() {
        let ctx = RequestContext::default()
            .with_correlation_id("trace-123");

        assert_eq!(ctx.correlation_id, Some("trace-123".into()));
    }
}
