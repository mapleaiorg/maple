//! Audit query support

use super::entry::{ActorType, AuditAction, AuditEntry, AuditOutcome, ResourceType};
use chrono::{DateTime, Utc};

/// Query for audit entries
#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    /// Filter by platform
    pub platform: Option<String>,

    /// Filter by actor ID
    pub actor_id: Option<String>,

    /// Filter by actor type
    pub actor_type: Option<ActorType>,

    /// Filter by resource ID
    pub resource_id: Option<String>,

    /// Filter by resource type
    pub resource_type: Option<ResourceType>,

    /// Filter by action type
    pub action: Option<AuditAction>,

    /// Filter by time range start (inclusive)
    pub from: Option<DateTime<Utc>>,

    /// Filter by time range end (exclusive)
    pub to: Option<DateTime<Utc>>,

    /// Filter by trace ID
    pub trace_id: Option<String>,

    /// Filter by outcome success/failure
    pub success_only: Option<bool>,

    /// Maximum number of results
    pub limit: Option<usize>,

    /// Offset for pagination
    pub offset: Option<usize>,

    /// Sort order (true = newest first)
    pub descending: bool,
}

impl AuditQuery {
    /// Create a new query builder
    pub fn builder() -> AuditQueryBuilder {
        AuditQueryBuilder::default()
    }

    /// Check if an entry matches this query
    pub fn matches(&self, entry: &AuditEntry) -> bool {
        // Platform filter
        if let Some(ref platform) = self.platform {
            if &entry.platform != platform {
                return false;
            }
        }

        // Actor filters
        if let Some(ref actor_id) = self.actor_id {
            if &entry.actor.id != actor_id {
                return false;
            }
        }

        if let Some(actor_type) = self.actor_type {
            if entry.actor.actor_type != actor_type {
                return false;
            }
        }

        // Resource filters
        if let Some(ref resource_id) = self.resource_id {
            if &entry.resource.id != resource_id {
                return false;
            }
        }

        if let Some(resource_type) = self.resource_type {
            if entry.resource.resource_type != resource_type {
                return false;
            }
        }

        // Action filter (compare by serialized form)
        if let Some(ref action) = self.action {
            let entry_action_str = serde_json::to_string(&entry.action).unwrap_or_default();
            let query_action_str = serde_json::to_string(action).unwrap_or_default();
            if entry_action_str != query_action_str {
                return false;
            }
        }

        // Time range filters
        if let Some(from) = self.from {
            if entry.timestamp < from {
                return false;
            }
        }

        if let Some(to) = self.to {
            if entry.timestamp >= to {
                return false;
            }
        }

        // Trace ID filter
        if let Some(ref trace_id) = self.trace_id {
            if entry.trace_id.as_ref() != Some(trace_id) {
                return false;
            }
        }

        // Success filter
        if let Some(success_only) = self.success_only {
            if success_only && !entry.outcome.is_success() {
                return false;
            }
            if !success_only && entry.outcome.is_success() {
                return false;
            }
        }

        true
    }

    /// Apply query to a list of entries
    pub fn apply(&self, entries: &[AuditEntry]) -> Vec<AuditEntry> {
        let mut results: Vec<AuditEntry> = entries
            .iter()
            .filter(|e| self.matches(e))
            .cloned()
            .collect();

        // Sort
        if self.descending {
            results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        } else {
            results.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        }

        // Pagination
        let offset = self.offset.unwrap_or(0);
        if offset > 0 {
            results = results.into_iter().skip(offset).collect();
        }

        if let Some(limit) = self.limit {
            results.truncate(limit);
        }

        results
    }
}

/// Builder for audit queries
#[derive(Debug, Default)]
pub struct AuditQueryBuilder {
    query: AuditQuery,
}

impl AuditQueryBuilder {
    /// Filter by platform
    pub fn platform(mut self, platform: impl Into<String>) -> Self {
        self.query.platform = Some(platform.into());
        self
    }

    /// Filter by actor ID
    pub fn actor_id(mut self, id: impl Into<String>) -> Self {
        self.query.actor_id = Some(id.into());
        self
    }

    /// Filter by actor type
    pub fn actor_type(mut self, actor_type: ActorType) -> Self {
        self.query.actor_type = Some(actor_type);
        self
    }

    /// Filter by resource ID
    pub fn resource_id(mut self, id: impl Into<String>) -> Self {
        self.query.resource_id = Some(id.into());
        self
    }

    /// Filter by resource type
    pub fn resource_type(mut self, resource_type: ResourceType) -> Self {
        self.query.resource_type = Some(resource_type);
        self
    }

    /// Filter by action
    pub fn action(mut self, action: AuditAction) -> Self {
        self.query.action = Some(action);
        self
    }

    /// Filter by time range start
    pub fn from(mut self, time: DateTime<Utc>) -> Self {
        self.query.from = Some(time);
        self
    }

    /// Filter by time range end
    pub fn to(mut self, time: DateTime<Utc>) -> Self {
        self.query.to = Some(time);
        self
    }

    /// Filter by trace ID
    pub fn trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.query.trace_id = Some(trace_id.into());
        self
    }

    /// Filter by success only
    pub fn success_only(mut self) -> Self {
        self.query.success_only = Some(true);
        self
    }

    /// Filter by failures only
    pub fn failures_only(mut self) -> Self {
        self.query.success_only = Some(false);
        self
    }

    /// Set limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.query.limit = Some(limit);
        self
    }

    /// Set offset
    pub fn offset(mut self, offset: usize) -> Self {
        self.query.offset = Some(offset);
        self
    }

    /// Sort descending (newest first)
    pub fn descending(mut self) -> Self {
        self.query.descending = true;
        self
    }

    /// Sort ascending (oldest first)
    pub fn ascending(mut self) -> Self {
        self.query.descending = false;
        self
    }

    /// Build the query
    pub fn build(self) -> AuditQuery {
        self.query
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::entry::{AuditActor, AuditResource, AuditEntry as Entry};
    use chrono::Duration;

    fn create_test_entry(
        platform: &str,
        actor_id: &str,
        resource_id: &str,
        action: AuditAction,
        outcome: AuditOutcome,
    ) -> AuditEntry {
        Entry::builder()
            .platform(platform)
            .actor(AuditActor::system(actor_id))
            .action(action)
            .resource(AuditResource::instance(resource_id))
            .outcome(outcome)
            .build()
            .unwrap()
            .finalize(None)
    }

    #[test]
    fn test_query_by_platform() {
        let entries = vec![
            create_test_entry("development", "a1", "r1", AuditAction::SystemStarted, AuditOutcome::success()),
            create_test_entry("production", "a2", "r2", AuditAction::SystemStarted, AuditOutcome::success()),
            create_test_entry("development", "a3", "r3", AuditAction::SystemStarted, AuditOutcome::success()),
        ];

        let query = AuditQuery::builder()
            .platform("development")
            .build();

        let results = query.apply(&entries);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_by_actor() {
        let entries = vec![
            create_test_entry("dev", "scheduler", "r1", AuditAction::InstanceStarted, AuditOutcome::success()),
            create_test_entry("dev", "reconciler", "r2", AuditAction::InstanceStarted, AuditOutcome::success()),
            create_test_entry("dev", "scheduler", "r3", AuditAction::InstanceStopped, AuditOutcome::success()),
        ];

        let query = AuditQuery::builder()
            .actor_id("scheduler")
            .build();

        let results = query.apply(&entries);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_by_outcome() {
        let entries = vec![
            create_test_entry("dev", "a1", "r1", AuditAction::InstanceStarted, AuditOutcome::success()),
            create_test_entry("dev", "a2", "r2", AuditAction::InstanceStarted, AuditOutcome::failure("error")),
            create_test_entry("dev", "a3", "r3", AuditAction::InstanceStarted, AuditOutcome::success()),
        ];

        let success_query = AuditQuery::builder()
            .success_only()
            .build();
        assert_eq!(success_query.apply(&entries).len(), 2);

        let failure_query = AuditQuery::builder()
            .failures_only()
            .build();
        assert_eq!(failure_query.apply(&entries).len(), 1);
    }

    #[test]
    fn test_query_pagination() {
        let entries: Vec<AuditEntry> = (0..10)
            .map(|i| {
                create_test_entry(
                    "dev",
                    &format!("actor-{}", i),
                    &format!("resource-{}", i),
                    AuditAction::InstanceStarted,
                    AuditOutcome::success(),
                )
            })
            .collect();

        let query = AuditQuery::builder()
            .offset(2)
            .limit(3)
            .build();

        let results = query.apply(&entries);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_query_sorting() {
        let now = Utc::now();
        let entries: Vec<AuditEntry> = (0..3)
            .map(|i| {
                let mut entry = create_test_entry(
                    "dev",
                    &format!("actor-{}", i),
                    "r1",
                    AuditAction::InstanceStarted,
                    AuditOutcome::success(),
                );
                // Manually adjust timestamp for test
                entry.timestamp = now - Duration::hours(i as i64);
                entry
            })
            .collect();

        let desc_query = AuditQuery::builder().descending().build();
        let desc_results = desc_query.apply(&entries);
        assert!(desc_results[0].timestamp > desc_results[1].timestamp);

        let asc_query = AuditQuery::builder().ascending().build();
        let asc_results = asc_query.apply(&entries);
        assert!(asc_results[0].timestamp < asc_results[1].timestamp);
    }
}
