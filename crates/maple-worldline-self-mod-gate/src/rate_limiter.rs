//! Rate limiter — prevents self-modification runaway.
//!
//! Enforces I.REGEN-6 (Rate Limiting): controls the frequency of
//! self-modifications per tier, with cooldown after rollbacks and
//! escalation on consecutive failures.

use std::collections::{HashMap, VecDeque};

use chrono::{DateTime, Duration, Utc};

use crate::types::SelfModTier;

// ── Tier Rate Limit ─────────────────────────────────────────────────────

/// Rate limits for a specific tier.
#[derive(Clone, Debug)]
pub struct TierRateLimit {
    /// Maximum modifications per hour.
    pub per_hour: u32,
    /// Maximum modifications per day.
    pub per_day: u32,
    /// Maximum modifications per week.
    pub per_week: u32,
    /// Minimum interval between modifications to the same component (seconds).
    pub min_interval_same_component_secs: u64,
}

impl Default for TierRateLimit {
    fn default() -> Self {
        Self {
            per_hour: 2,
            per_day: 5,
            per_week: 20,
            min_interval_same_component_secs: 3600, // 1 hour
        }
    }
}

// ── Modification Record ─────────────────────────────────────────────────

/// A recorded modification event for rate limiting.
#[derive(Clone, Debug)]
struct ModificationRecord {
    timestamp: DateTime<Utc>,
    tier: SelfModTier,
    affected_components: Vec<String>,
}

// ── Rate Limiter ────────────────────────────────────────────────────────

/// Rate limiter for self-modification frequency control.
///
/// Prevents modification runaway by enforcing per-tier rate limits,
/// same-component intervals, rollback cooldowns, and escalation
/// on consecutive rollback failures.
pub struct RegenerationRateLimiter {
    /// Per-tier rate limits.
    limits: HashMap<SelfModTier, TierRateLimit>,
    /// Modification history.
    history: VecDeque<ModificationRecord>,
    /// Rollback cooldown duration (seconds).
    rollback_cooldown_secs: u64,
    /// Number of consecutive rollbacks that triggers escalation.
    rollback_escalation_threshold: u32,
    /// Current consecutive rollback count.
    consecutive_rollbacks: u32,
    /// Timestamp of last rollback.
    last_rollback: Option<DateTime<Utc>>,
    /// Maximum history entries to retain.
    max_history: usize,
}

impl RegenerationRateLimiter {
    /// Create a new rate limiter with default settings.
    pub fn new() -> Self {
        Self {
            limits: Self::default_limits(),
            history: VecDeque::new(),
            rollback_cooldown_secs: 7200, // 2 hours
            rollback_escalation_threshold: 3,
            consecutive_rollbacks: 0,
            last_rollback: None,
            max_history: 1000,
        }
    }

    /// Create with custom rollback cooldown.
    pub fn with_rollback_cooldown(mut self, cooldown_secs: u64) -> Self {
        self.rollback_cooldown_secs = cooldown_secs;
        self
    }

    /// Create with custom escalation threshold.
    pub fn with_escalation_threshold(mut self, threshold: u32) -> Self {
        self.rollback_escalation_threshold = threshold;
        self
    }

    /// Set custom rate limits for a specific tier.
    pub fn set_tier_limit(&mut self, tier: SelfModTier, limit: TierRateLimit) {
        self.limits.insert(tier, limit);
    }

    /// Check whether a modification at the given tier is allowed.
    pub fn allow(&self, tier: &SelfModTier, affected_components: &[String]) -> bool {
        // 1. Check rollback cooldown
        if self.in_cooldown() {
            return false;
        }

        // 2. Check escalation (consecutive rollbacks block all modifications)
        if self.is_escalated() {
            return false;
        }

        // 3. Check tier-specific rate limits
        let limit = self.limits.get(tier).cloned().unwrap_or_default();
        let now = Utc::now();

        let count_since = |duration: Duration| -> u32 {
            let cutoff = now - duration;
            self.history
                .iter()
                .filter(|r| r.tier == *tier && r.timestamp > cutoff)
                .count() as u32
        };

        if count_since(Duration::hours(1)) >= limit.per_hour {
            return false;
        }
        if count_since(Duration::days(1)) >= limit.per_day {
            return false;
        }
        if count_since(Duration::weeks(1)) >= limit.per_week {
            return false;
        }

        // 4. Check same-component interval
        let min_interval = Duration::seconds(limit.min_interval_same_component_secs as i64);
        let cutoff = now - min_interval;
        for record in self.history.iter().rev() {
            if record.timestamp < cutoff {
                break;
            }
            if record.tier == *tier {
                for comp in affected_components {
                    if record.affected_components.contains(comp) {
                        return false; // Too soon for same component
                    }
                }
            }
        }

        true
    }

    /// Record a successful modification.
    pub fn record_modification(
        &mut self,
        tier: SelfModTier,
        affected_components: Vec<String>,
    ) {
        self.history.push_back(ModificationRecord {
            timestamp: Utc::now(),
            tier,
            affected_components,
        });

        // FIFO eviction
        while self.history.len() > self.max_history {
            self.history.pop_front();
        }

        // Reset consecutive rollbacks on success
        self.consecutive_rollbacks = 0;
    }

    /// Record a rollback event.
    pub fn record_rollback(&mut self) {
        self.consecutive_rollbacks += 1;
        self.last_rollback = Some(Utc::now());
    }

    /// Whether we are currently in rollback cooldown.
    pub fn in_cooldown(&self) -> bool {
        if let Some(last) = self.last_rollback {
            let elapsed = (Utc::now() - last).num_seconds().max(0) as u64;
            elapsed < self.rollback_cooldown_secs
        } else {
            false
        }
    }

    /// Whether consecutive rollbacks have triggered escalation.
    pub fn is_escalated(&self) -> bool {
        self.consecutive_rollbacks >= self.rollback_escalation_threshold
    }

    /// Current consecutive rollback count.
    pub fn consecutive_rollbacks(&self) -> u32 {
        self.consecutive_rollbacks
    }

    /// Default rate limits per tier.
    fn default_limits() -> HashMap<SelfModTier, TierRateLimit> {
        let mut limits = HashMap::new();
        limits.insert(SelfModTier::Tier0Configuration, TierRateLimit {
            per_hour: 5,
            per_day: 20,
            per_week: 100,
            min_interval_same_component_secs: 600, // 10 min
        });
        limits.insert(SelfModTier::Tier1OperatorInternal, TierRateLimit::default());
        limits.insert(SelfModTier::Tier2ApiChange, TierRateLimit {
            per_hour: 1,
            per_day: 3,
            per_week: 10,
            min_interval_same_component_secs: 7200, // 2 hours
        });
        limits.insert(SelfModTier::Tier3KernelChange, TierRateLimit {
            per_hour: 1,
            per_day: 1,
            per_week: 3,
            min_interval_same_component_secs: 86400, // 24 hours
        });
        limits.insert(SelfModTier::Tier4SubstrateChange, TierRateLimit {
            per_hour: 1,
            per_day: 1,
            per_week: 1,
            min_interval_same_component_secs: 604800, // 1 week
        });
        limits.insert(SelfModTier::Tier5ArchitecturalChange, TierRateLimit {
            per_hour: 1,
            per_day: 1,
            per_week: 1,
            min_interval_same_component_secs: 1209600, // 2 weeks
        });
        limits
    }

    /// Record a modification at a specific timestamp (for testing).
    #[cfg(test)]
    pub(crate) fn record_modification_at(
        &mut self,
        tier: SelfModTier,
        affected_components: Vec<String>,
        timestamp: DateTime<Utc>,
    ) {
        self.history.push_back(ModificationRecord {
            timestamp,
            tier,
            affected_components,
        });
    }

    /// Set last rollback timestamp (for testing).
    #[cfg(test)]
    pub(crate) fn set_last_rollback(&mut self, timestamp: DateTime<Utc>) {
        self.last_rollback = Some(timestamp);
    }
}

impl Default for RegenerationRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_within_limits() {
        let limiter = RegenerationRateLimiter::new();
        assert!(limiter.allow(
            &SelfModTier::Tier0Configuration,
            &["config".to_string()],
        ));
    }

    #[test]
    fn deny_when_per_hour_exceeded() {
        let mut limiter = RegenerationRateLimiter::new();
        let now = Utc::now();

        // Fill up Tier0's per_hour limit (5)
        for i in 0..5 {
            limiter.record_modification_at(
                SelfModTier::Tier0Configuration,
                vec![format!("comp-{}", i)],
                now - Duration::minutes(i as i64),
            );
        }

        assert!(!limiter.allow(
            &SelfModTier::Tier0Configuration,
            &["new-comp".to_string()],
        ));
    }

    #[test]
    fn deny_same_component_too_soon() {
        let mut limiter = RegenerationRateLimiter::new();
        let now = Utc::now();

        // Record a modification to "config" 5 minutes ago
        limiter.record_modification_at(
            SelfModTier::Tier0Configuration,
            vec!["config".to_string()],
            now - Duration::minutes(5),
        );

        // Tier0 has 10min same-component interval → should be denied
        assert!(!limiter.allow(
            &SelfModTier::Tier0Configuration,
            &["config".to_string()],
        ));

        // Different component should be allowed
        assert!(limiter.allow(
            &SelfModTier::Tier0Configuration,
            &["other".to_string()],
        ));
    }

    #[test]
    fn deny_during_rollback_cooldown() {
        let mut limiter = RegenerationRateLimiter::new()
            .with_rollback_cooldown(3600); // 1 hour cooldown

        limiter.record_rollback();
        assert!(limiter.in_cooldown());
        assert!(!limiter.allow(
            &SelfModTier::Tier0Configuration,
            &["config".to_string()],
        ));
    }

    #[test]
    fn deny_when_escalated() {
        let mut limiter = RegenerationRateLimiter::new()
            .with_escalation_threshold(3);

        limiter.record_rollback();
        limiter.record_rollback();
        assert!(!limiter.is_escalated());

        limiter.record_rollback();
        assert!(limiter.is_escalated());
        // Even after cooldown passes, escalation blocks
        limiter.set_last_rollback(Utc::now() - Duration::days(1));
        assert!(!limiter.in_cooldown());
        assert!(limiter.is_escalated());
        assert!(!limiter.allow(
            &SelfModTier::Tier0Configuration,
            &["config".to_string()],
        ));
    }

    #[test]
    fn success_resets_consecutive_rollbacks() {
        let mut limiter = RegenerationRateLimiter::new();
        limiter.record_rollback();
        limiter.record_rollback();
        assert_eq!(limiter.consecutive_rollbacks(), 2);

        limiter.record_modification(SelfModTier::Tier0Configuration, vec!["a".into()]);
        assert_eq!(limiter.consecutive_rollbacks(), 0);
    }
}
