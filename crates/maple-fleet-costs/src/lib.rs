//! MAPLE Fleet Costs -- cost budgets and usage tracking.
//!
//! Tracks costs by agent, type, and time period. Supports budget limits
//! with configurable alert thresholds and report generation.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum CostError {
    #[error("budget exceeded: {budget_name} (limit: {limit}, current: {current})")]
    BudgetExceeded {
        budget_name: String,
        limit: f64,
        current: f64,
    },
    #[error("budget not found: {0}")]
    BudgetNotFound(String),
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

pub type CostResult<T> = Result<T, CostError>;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Type of cost incurred.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CostType {
    ModelInference,
    ToolExecution,
    Storage,
    Network,
    Compute,
}

impl std::fmt::Display for CostType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModelInference => write!(f, "model_inference"),
            Self::ToolExecution => write!(f, "tool_execution"),
            Self::Storage => write!(f, "storage"),
            Self::Network => write!(f, "network"),
            Self::Compute => write!(f, "compute"),
        }
    }
}

/// A single cost entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEntry {
    pub id: String,
    pub agent_id: String,
    pub cost_type: CostType,
    pub amount: f64,
    pub currency: String,
    pub timestamp: DateTime<Utc>,
    pub description: Option<String>,
}

impl CostEntry {
    pub fn new(agent_id: impl Into<String>, cost_type: CostType, amount: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            agent_id: agent_id.into(),
            cost_type,
            amount,
            currency: "USD".to_string(),
            timestamp: Utc::now(),
            description: None,
        }
    }
}

/// Budget period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BudgetPeriod {
    Daily,
    Weekly,
    Monthly,
}

impl BudgetPeriod {
    /// Return the duration for this period.
    pub fn duration(&self) -> Duration {
        match self {
            Self::Daily => Duration::days(1),
            Self::Weekly => Duration::weeks(1),
            Self::Monthly => Duration::days(30),
        }
    }
}

/// An alert threshold within a budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThreshold {
    /// Percentage of budget (0.0-1.0) at which to alert.
    pub percentage: f64,
    pub label: String,
}

/// A cost budget with limits and alerts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBudget {
    pub id: String,
    pub name: String,
    pub period: BudgetPeriod,
    pub limit: f64,
    pub currency: String,
    pub alerts: Vec<AlertThreshold>,
    pub agent_filter: Option<String>,
}

impl CostBudget {
    pub fn new(name: impl Into<String>, period: BudgetPeriod, limit: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            period,
            limit,
            currency: "USD".to_string(),
            alerts: vec![
                AlertThreshold { percentage: 0.75, label: "75% warning".into() },
                AlertThreshold { percentage: 0.90, label: "90% critical".into() },
            ],
            agent_filter: None,
        }
    }
}

/// Report of costs broken down by various dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostReport {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_cost: f64,
    pub by_agent: HashMap<String, f64>,
    pub by_type: HashMap<String, f64>,
    pub entry_count: usize,
}

/// Alert generated when a budget threshold is crossed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAlert {
    pub budget_name: String,
    pub threshold_label: String,
    pub current_usage: f64,
    pub limit: f64,
    pub percentage: f64,
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Usage Tracker
// ---------------------------------------------------------------------------

/// Tracks cost entries and checks budgets.
pub struct UsageTracker {
    entries: Vec<CostEntry>,
    budgets: HashMap<String, CostBudget>,
}

impl Default for UsageTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl UsageTracker {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            budgets: HashMap::new(),
        }
    }

    /// Add a cost entry.
    pub fn add_cost(&mut self, entry: CostEntry) {
        self.entries.push(entry);
    }

    /// Add a budget.
    pub fn add_budget(&mut self, budget: CostBudget) {
        self.budgets.insert(budget.id.clone(), budget);
    }

    /// Get total usage for a given period.
    pub fn get_usage(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> f64 {
        self.entries
            .iter()
            .filter(|e| e.timestamp >= since && e.timestamp <= until)
            .map(|e| e.amount)
            .sum()
    }

    /// Get usage for a specific agent.
    pub fn get_agent_usage(&self, agent_id: &str, since: DateTime<Utc>, until: DateTime<Utc>) -> f64 {
        self.entries
            .iter()
            .filter(|e| e.agent_id == agent_id && e.timestamp >= since && e.timestamp <= until)
            .map(|e| e.amount)
            .sum()
    }

    /// Check all budgets and return alerts for any that are exceeded.
    pub fn check_budgets(&self) -> Vec<BudgetAlert> {
        let now = Utc::now();
        let mut alerts = Vec::new();

        for budget in self.budgets.values() {
            let period_start = now - budget.period.duration();
            let usage = match &budget.agent_filter {
                Some(agent) => self.get_agent_usage(agent, period_start, now),
                None => self.get_usage(period_start, now),
            };

            let pct = if budget.limit > 0.0 {
                usage / budget.limit
            } else {
                0.0
            };

            for threshold in &budget.alerts {
                if pct >= threshold.percentage {
                    alerts.push(BudgetAlert {
                        budget_name: budget.name.clone(),
                        threshold_label: threshold.label.clone(),
                        current_usage: usage,
                        limit: budget.limit,
                        percentage: pct,
                        timestamp: now,
                    });
                }
            }
        }

        alerts
    }

    /// Generate a cost report for a time range.
    pub fn generate_report(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> CostReport {
        let filtered: Vec<&CostEntry> = self
            .entries
            .iter()
            .filter(|e| e.timestamp >= since && e.timestamp <= until)
            .collect();

        let mut by_agent: HashMap<String, f64> = HashMap::new();
        let mut by_type: HashMap<String, f64> = HashMap::new();
        let mut total = 0.0;

        for entry in &filtered {
            total += entry.amount;
            *by_agent.entry(entry.agent_id.clone()).or_default() += entry.amount;
            *by_type.entry(entry.cost_type.to_string()).or_default() += entry.amount;
        }

        CostReport {
            period_start: since,
            period_end: until,
            total_cost: total,
            by_agent,
            by_type,
            entry_count: filtered.len(),
        }
    }

    /// Return all entries.
    pub fn entries(&self) -> &[CostEntry] {
        &self.entries
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tracker_with_entries() -> UsageTracker {
        let mut tracker = UsageTracker::new();
        tracker.add_cost(CostEntry::new("agent-1", CostType::ModelInference, 0.50));
        tracker.add_cost(CostEntry::new("agent-1", CostType::Compute, 0.25));
        tracker.add_cost(CostEntry::new("agent-2", CostType::ModelInference, 1.00));
        tracker.add_cost(CostEntry::new("agent-2", CostType::Storage, 0.10));
        tracker
    }

    #[test]
    fn test_add_cost() {
        let mut tracker = UsageTracker::new();
        tracker.add_cost(CostEntry::new("agent-1", CostType::ModelInference, 1.0));
        assert_eq!(tracker.entries().len(), 1);
    }

    #[test]
    fn test_get_usage() {
        let tracker = make_tracker_with_entries();
        let since = Utc::now() - Duration::hours(1);
        let usage = tracker.get_usage(since, Utc::now());
        assert!((usage - 1.85).abs() < 0.01);
    }

    #[test]
    fn test_get_agent_usage() {
        let tracker = make_tracker_with_entries();
        let since = Utc::now() - Duration::hours(1);
        let usage = tracker.get_agent_usage("agent-1", since, Utc::now());
        assert!((usage - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_generate_report() {
        let tracker = make_tracker_with_entries();
        let since = Utc::now() - Duration::hours(1);
        let report = tracker.generate_report(since, Utc::now());
        assert_eq!(report.entry_count, 4);
        assert!((report.total_cost - 1.85).abs() < 0.01);
        assert_eq!(report.by_agent.len(), 2);
    }

    #[test]
    fn test_cost_type_display() {
        assert_eq!(CostType::ModelInference.to_string(), "model_inference");
        assert_eq!(CostType::Storage.to_string(), "storage");
    }

    #[test]
    fn test_budget_period_duration() {
        assert_eq!(BudgetPeriod::Daily.duration(), Duration::days(1));
        assert_eq!(BudgetPeriod::Weekly.duration(), Duration::weeks(1));
    }

    #[test]
    fn test_budget_alert() {
        let mut tracker = UsageTracker::new();
        // Add a budget of $1.00 daily
        let budget = CostBudget::new("daily-budget", BudgetPeriod::Daily, 1.00);
        tracker.add_budget(budget);
        // Add costs that exceed 90%
        tracker.add_cost(CostEntry::new("agent-1", CostType::ModelInference, 0.95));
        let alerts = tracker.check_budgets();
        // Should trigger 75% and 90% alerts
        assert_eq!(alerts.len(), 2);
    }

    #[test]
    fn test_no_alerts_within_budget() {
        let mut tracker = UsageTracker::new();
        let budget = CostBudget::new("daily-budget", BudgetPeriod::Daily, 100.00);
        tracker.add_budget(budget);
        tracker.add_cost(CostEntry::new("agent-1", CostType::ModelInference, 0.50));
        let alerts = tracker.check_budgets();
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_report_by_type() {
        let tracker = make_tracker_with_entries();
        let since = Utc::now() - Duration::hours(1);
        let report = tracker.generate_report(since, Utc::now());
        assert!(report.by_type.contains_key("model_inference"));
        assert!(report.by_type.contains_key("compute"));
    }

    #[test]
    fn test_cost_entry_default_currency() {
        let entry = CostEntry::new("agent-1", CostType::Network, 0.01);
        assert_eq!(entry.currency, "USD");
    }

    #[test]
    fn test_empty_tracker() {
        let tracker = UsageTracker::new();
        let since = Utc::now() - Duration::hours(1);
        let report = tracker.generate_report(since, Utc::now());
        assert_eq!(report.entry_count, 0);
        assert!((report.total_cost - 0.0).abs() < f64::EPSILON);
    }
}
