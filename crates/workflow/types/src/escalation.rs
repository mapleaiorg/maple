//! Escalation paths: what happens when things go wrong
//!
//! Escalation is explicit in MAPLE — there is no silent failure.
//! When a node times out, a commitment breaks, or a policy is
//! violated, the escalation path determines what happens next.

use collective_types::RoleId;
use serde::{Deserialize, Serialize};

/// An escalation path — the response to failure or timeout
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EscalationPath {
    /// The trigger that activates this escalation
    pub trigger: EscalationTrigger,
    /// The action to take when triggered
    pub action: EscalationAction,
    /// Maximum number of retry attempts (for Retry action)
    pub max_retries: u32,
    /// Description of this escalation path
    pub description: String,
}

impl EscalationPath {
    /// Create a new escalation path
    pub fn new(trigger: EscalationTrigger, action: EscalationAction) -> Self {
        Self {
            trigger,
            action,
            max_retries: 0,
            description: String::new(),
        }
    }

    /// Create a timeout escalation that aborts
    pub fn timeout_abort(timeout_secs: u64) -> Self {
        Self::new(
            EscalationTrigger::Timeout { timeout_secs },
            EscalationAction::Abort {
                reason: format!("Timeout after {} seconds", timeout_secs),
            },
        )
    }

    /// Create a timeout escalation that retries
    pub fn timeout_retry(timeout_secs: u64, max_retries: u32) -> Self {
        Self {
            trigger: EscalationTrigger::Timeout { timeout_secs },
            action: EscalationAction::Retry,
            max_retries,
            description: format!(
                "Retry up to {} times after {} second timeout",
                max_retries, timeout_secs
            ),
        }
    }

    /// Create an escalation that delegates to a higher role
    pub fn escalate_to_role(trigger: EscalationTrigger, role: RoleId) -> Self {
        Self::new(trigger, EscalationAction::EscalateToRole { role })
    }

    /// Create an escalation that redirects to a different node
    pub fn redirect_to(trigger: EscalationTrigger, node_id: impl Into<String>) -> Self {
        Self::new(
            trigger,
            EscalationAction::RedirectToNode {
                node_id: node_id.into(),
            },
        )
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    /// Check if this escalation allows retries
    pub fn allows_retry(&self) -> bool {
        matches!(self.action, EscalationAction::Retry) && self.max_retries > 0
    }
}

/// What triggers an escalation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EscalationTrigger {
    /// Node has been active for too long
    Timeout {
        /// Timeout duration in seconds
        timeout_secs: u64,
    },

    /// The commitment at this node was broken (not fulfilled)
    CommitmentBroken,

    /// A policy violation was detected
    PolicyViolation {
        /// Description of the violation
        violation: String,
    },

    /// Manual escalation by a resonator
    Manual,

    /// The node has been retried too many times
    MaxRetriesExceeded,

    /// A dependency failed (upstream node)
    DependencyFailed {
        /// The node that failed
        failed_node_id: String,
    },

    /// Budget exhaustion prevented the action
    BudgetExhausted,
}

/// What action to take when escalation triggers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EscalationAction {
    /// Retry the current node
    Retry,

    /// Abort the entire workflow
    Abort {
        /// Reason for aborting
        reason: String,
    },

    /// Escalate to a specific role (e.g., supervisor, admin)
    EscalateToRole {
        /// The role to escalate to
        role: RoleId,
    },

    /// Redirect to a different node in the workflow
    RedirectToNode {
        /// The node to redirect to
        node_id: String,
    },

    /// Pause the workflow and wait for human intervention
    Pause {
        /// Reason for pausing
        reason: String,
    },

    /// Skip this node and proceed to next nodes
    Skip,

    /// Compensate: undo previous actions in reverse order
    Compensate {
        /// Description of compensation actions
        description: String,
    },
}

/// Tracks escalation state for a running workflow node
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EscalationState {
    /// How many retries have been attempted
    pub retry_count: u32,
    /// History of escalation events
    pub events: Vec<EscalationEvent>,
    /// Whether escalation is currently active
    pub active: bool,
}

impl EscalationState {
    pub fn new() -> Self {
        Self {
            retry_count: 0,
            events: Vec::new(),
            active: false,
        }
    }

    /// Record a retry attempt
    pub fn record_retry(&mut self) {
        self.retry_count += 1;
        self.events.push(EscalationEvent {
            event_type: EscalationEventType::Retry {
                attempt: self.retry_count,
            },
            timestamp: chrono::Utc::now(),
            description: format!("Retry attempt #{}", self.retry_count),
        });
    }

    /// Record an escalation
    pub fn record_escalation(&mut self, description: impl Into<String>) {
        self.active = true;
        self.events.push(EscalationEvent {
            event_type: EscalationEventType::Escalated,
            timestamp: chrono::Utc::now(),
            description: description.into(),
        });
    }

    /// Record escalation resolution
    pub fn record_resolution(&mut self, description: impl Into<String>) {
        self.active = false;
        self.events.push(EscalationEvent {
            event_type: EscalationEventType::Resolved,
            timestamp: chrono::Utc::now(),
            description: description.into(),
        });
    }

    /// Check if max retries exceeded
    pub fn retries_exceeded(&self, max: u32) -> bool {
        self.retry_count >= max
    }
}

impl Default for EscalationState {
    fn default() -> Self {
        Self::new()
    }
}

/// An escalation event in the history
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EscalationEvent {
    /// Type of escalation event
    pub event_type: EscalationEventType,
    /// When the event occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Description
    pub description: String,
}

/// Type of escalation event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EscalationEventType {
    /// A retry was attempted
    Retry { attempt: u32 },
    /// The issue was escalated
    Escalated,
    /// The escalation was resolved
    Resolved,
    /// The workflow was aborted
    Aborted,
    /// The workflow was paused
    Paused,
    /// Compensation was performed
    Compensated,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_abort() {
        let esc = EscalationPath::timeout_abort(3600);
        assert!(matches!(
            esc.trigger,
            EscalationTrigger::Timeout { timeout_secs: 3600 }
        ));
        assert!(matches!(esc.action, EscalationAction::Abort { .. }));
        assert!(!esc.allows_retry());
    }

    #[test]
    fn test_timeout_retry() {
        let esc = EscalationPath::timeout_retry(300, 3);
        assert!(matches!(
            esc.trigger,
            EscalationTrigger::Timeout { timeout_secs: 300 }
        ));
        assert!(matches!(esc.action, EscalationAction::Retry));
        assert_eq!(esc.max_retries, 3);
        assert!(esc.allows_retry());
    }

    #[test]
    fn test_escalate_to_role() {
        let esc = EscalationPath::escalate_to_role(
            EscalationTrigger::CommitmentBroken,
            RoleId::new("supervisor"),
        )
        .with_description("Escalate to supervisor when commitment breaks");

        assert!(matches!(esc.trigger, EscalationTrigger::CommitmentBroken));
        assert!(matches!(
            esc.action,
            EscalationAction::EscalateToRole { .. }
        ));
        assert!(!esc.description.is_empty());
    }

    #[test]
    fn test_redirect() {
        let esc = EscalationPath::redirect_to(
            EscalationTrigger::PolicyViolation {
                violation: "Budget exceeded".into(),
            },
            "error_handler",
        );

        assert!(matches!(
            esc.action,
            EscalationAction::RedirectToNode { .. }
        ));
    }

    #[test]
    fn test_escalation_state() {
        let mut state = EscalationState::new();
        assert!(!state.active);
        assert_eq!(state.retry_count, 0);

        state.record_retry();
        state.record_retry();
        assert_eq!(state.retry_count, 2);
        assert!(!state.retries_exceeded(3));
        assert!(state.retries_exceeded(2));

        state.record_escalation("Escalated to supervisor");
        assert!(state.active);

        state.record_resolution("Supervisor approved");
        assert!(!state.active);
        assert_eq!(state.events.len(), 4);
    }

    #[test]
    fn test_escalation_triggers() {
        let triggers = vec![
            EscalationTrigger::Timeout { timeout_secs: 60 },
            EscalationTrigger::CommitmentBroken,
            EscalationTrigger::PolicyViolation {
                violation: "test".into(),
            },
            EscalationTrigger::Manual,
            EscalationTrigger::MaxRetriesExceeded,
            EscalationTrigger::DependencyFailed {
                failed_node_id: "node-1".into(),
            },
            EscalationTrigger::BudgetExhausted,
        ];
        assert_eq!(triggers.len(), 7);
    }

    #[test]
    fn test_escalation_actions() {
        let actions: Vec<EscalationAction> = vec![
            EscalationAction::Retry,
            EscalationAction::Abort {
                reason: "Timed out".into(),
            },
            EscalationAction::EscalateToRole {
                role: RoleId::new("admin"),
            },
            EscalationAction::RedirectToNode {
                node_id: "fallback".into(),
            },
            EscalationAction::Pause {
                reason: "Need human input".into(),
            },
            EscalationAction::Skip,
            EscalationAction::Compensate {
                description: "Reverse transfer".into(),
            },
        ];
        assert_eq!(actions.len(), 7);
    }

    #[test]
    fn test_default_escalation_state() {
        let state = EscalationState::default();
        assert_eq!(state.retry_count, 0);
        assert!(!state.active);
        assert!(state.events.is_empty());
    }
}
