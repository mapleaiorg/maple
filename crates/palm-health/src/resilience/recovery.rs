//! Recovery actions for resilience.
//!
//! Defines the actions that can be taken to recover unhealthy instances.

use chrono::{DateTime, Utc};
use palm_types::InstanceId;
use serde::{Deserialize, Serialize};

/// Actions that can be taken to recover an unhealthy instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryAction {
    /// No action needed.
    None,

    /// Restart the instance gracefully (drain first).
    RestartGraceful {
        /// Timeout for draining in seconds.
        drain_timeout_secs: u64,
    },

    /// Restart the instance forcefully (immediate).
    RestartForce,

    /// Replace the instance with a new one.
    Replace {
        /// Whether to keep the old instance until new one is healthy.
        keep_old_until_healthy: bool,
    },

    /// Isolate the instance from receiving new work.
    Isolate {
        /// Reason for isolation.
        reason: String,
    },

    /// Scale up to add more instances.
    ScaleUp {
        /// Number of instances to add.
        count: u32,
    },

    /// Drain the instance and mark for removal.
    Drain {
        /// Timeout for draining in seconds.
        timeout_secs: u64,
    },

    /// Notify operators (no automated action).
    Notify {
        /// Severity level.
        severity: NotifySeverity,
        /// Message to send.
        message: String,
    },
}

impl RecoveryAction {
    /// Check if this action requires human approval.
    pub fn requires_approval(&self) -> bool {
        match self {
            RecoveryAction::RestartForce => true,
            RecoveryAction::Replace { .. } => true,
            RecoveryAction::ScaleUp { count } if *count > 1 => true,
            _ => false,
        }
    }

    /// Get a description of the action.
    pub fn description(&self) -> String {
        match self {
            RecoveryAction::None => "No action".to_string(),
            RecoveryAction::RestartGraceful { drain_timeout_secs } => {
                format!("Graceful restart with {}s drain", drain_timeout_secs)
            }
            RecoveryAction::RestartForce => "Force restart".to_string(),
            RecoveryAction::Replace {
                keep_old_until_healthy,
            } => {
                if *keep_old_until_healthy {
                    "Replace (keep old until healthy)".to_string()
                } else {
                    "Replace (immediate)".to_string()
                }
            }
            RecoveryAction::Isolate { reason } => format!("Isolate: {}", reason),
            RecoveryAction::ScaleUp { count } => format!("Scale up by {} instances", count),
            RecoveryAction::Drain { timeout_secs } => {
                format!("Drain with {}s timeout", timeout_secs)
            }
            RecoveryAction::Notify { severity, message } => {
                format!("Notify ({:?}): {}", severity, message)
            }
        }
    }

    /// Get the priority of this action (higher = more urgent).
    pub fn priority(&self) -> u32 {
        match self {
            RecoveryAction::None => 0,
            RecoveryAction::Notify { .. } => 1,
            RecoveryAction::Isolate { .. } => 2,
            RecoveryAction::Drain { .. } => 3,
            RecoveryAction::RestartGraceful { .. } => 4,
            RecoveryAction::ScaleUp { .. } => 5,
            RecoveryAction::RestartForce => 6,
            RecoveryAction::Replace { .. } => 7,
        }
    }
}

impl std::fmt::Display for RecoveryAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Severity level for notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotifySeverity {
    /// Informational.
    Info,
    /// Warning.
    Warning,
    /// Error.
    Error,
    /// Critical.
    Critical,
}

/// Context for recovery decision-making.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryContext {
    /// Instance being recovered.
    pub instance_id: InstanceId,

    /// Number of previous recovery attempts.
    pub attempt_count: u32,

    /// Last recovery action taken.
    pub last_action: Option<RecoveryAction>,

    /// Time of last recovery attempt.
    pub last_attempt_at: Option<DateTime<Utc>>,

    /// Whether human approval is available.
    pub human_approval_available: bool,

    /// Current fleet health percentage.
    pub fleet_health_percent: f64,

    /// Additional context data.
    pub metadata: std::collections::HashMap<String, String>,
}

impl RecoveryContext {
    /// Create a new recovery context.
    pub fn new(instance_id: InstanceId) -> Self {
        Self {
            instance_id,
            attempt_count: 0,
            last_action: None,
            last_attempt_at: None,
            human_approval_available: false,
            fleet_health_percent: 100.0,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Record a recovery attempt.
    pub fn record_attempt(&mut self, action: RecoveryAction) {
        self.attempt_count += 1;
        self.last_action = Some(action);
        self.last_attempt_at = Some(Utc::now());
    }

    /// Check if we've exceeded maximum attempts.
    pub fn exceeded_max_attempts(&self, max: u32) -> bool {
        self.attempt_count >= max
    }

    /// Check if enough time has passed since last attempt.
    pub fn cooldown_elapsed(&self, cooldown: std::time::Duration) -> bool {
        match self.last_attempt_at {
            None => true,
            Some(last) => {
                let elapsed = Utc::now().signed_duration_since(last);
                elapsed.to_std().map(|d| d >= cooldown).unwrap_or(true)
            }
        }
    }
}

/// Outcome of a recovery action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryOutcome {
    /// Instance that was recovered.
    pub instance_id: InstanceId,

    /// Action that was attempted.
    pub action: RecoveryAction,

    /// Whether the action succeeded.
    pub success: bool,

    /// Time the action was started.
    pub started_at: DateTime<Utc>,

    /// Time the action completed.
    pub completed_at: DateTime<Utc>,

    /// Duration of the action in milliseconds.
    pub duration_ms: u64,

    /// Error message if failed.
    pub error: Option<String>,

    /// Additional notes.
    pub notes: Vec<String>,
}

impl RecoveryOutcome {
    /// Create a successful outcome.
    pub fn success(
        instance_id: InstanceId,
        action: RecoveryAction,
        started_at: DateTime<Utc>,
    ) -> Self {
        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds().max(0) as u64;

        Self {
            instance_id,
            action,
            success: true,
            started_at,
            completed_at,
            duration_ms,
            error: None,
            notes: Vec::new(),
        }
    }

    /// Create a failed outcome.
    pub fn failure(
        instance_id: InstanceId,
        action: RecoveryAction,
        started_at: DateTime<Utc>,
        error: impl Into<String>,
    ) -> Self {
        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds().max(0) as u64;

        Self {
            instance_id,
            action,
            success: false,
            started_at,
            completed_at,
            duration_ms,
            error: Some(error.into()),
            notes: Vec::new(),
        }
    }

    /// Add a note to the outcome.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_action_priority() {
        assert!(
            RecoveryAction::Replace {
                keep_old_until_healthy: true
            }
            .priority()
                > RecoveryAction::RestartGraceful {
                    drain_timeout_secs: 30
                }
                .priority()
        );

        assert!(
            RecoveryAction::RestartForce.priority()
                > RecoveryAction::Isolate {
                    reason: "test".to_string()
                }
                .priority()
        );
    }

    #[test]
    fn test_recovery_context_attempts() {
        let mut ctx = RecoveryContext::new(InstanceId::generate());

        assert_eq!(ctx.attempt_count, 0);
        assert!(!ctx.exceeded_max_attempts(3));

        ctx.record_attempt(RecoveryAction::RestartGraceful {
            drain_timeout_secs: 30,
        });
        assert_eq!(ctx.attempt_count, 1);

        ctx.record_attempt(RecoveryAction::RestartGraceful {
            drain_timeout_secs: 30,
        });
        ctx.record_attempt(RecoveryAction::RestartForce);
        assert!(ctx.exceeded_max_attempts(3));
    }
}
