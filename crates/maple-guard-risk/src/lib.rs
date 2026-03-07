//! MAPLE Guard Risk -- risk scoring engine for AI agent actions.
//!
//! Assesses the risk of agent actions across multiple categories,
//! producing composite risk scores with configurable thresholds and alerts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum RiskError {
    #[error("risk assessment not found: {0}")]
    NotFound(String),
    #[error("invalid score: {0} (must be 0.0-1.0)")]
    InvalidScore(f64),
    #[error("engine error: {0}")]
    EngineError(String),
}

pub type RiskResult<T> = Result<T, RiskError>;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Category of risk being assessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiskCategory {
    DataExfiltration,
    PromptInjection,
    ModelAbuse,
    Unauthorized,
    FinancialRisk,
}

impl std::fmt::Display for RiskCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DataExfiltration => write!(f, "data_exfiltration"),
            Self::PromptInjection => write!(f, "prompt_injection"),
            Self::ModelAbuse => write!(f, "model_abuse"),
            Self::Unauthorized => write!(f, "unauthorized"),
            Self::FinancialRisk => write!(f, "financial_risk"),
        }
    }
}

/// A single risk factor contributing to the overall score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub category: RiskCategory,
    pub weight: f64,
    pub score: f64,
    pub evidence: Vec<String>,
}

impl RiskFactor {
    pub fn new(category: RiskCategory, score: f64) -> Self {
        Self {
            category,
            weight: 1.0,
            score: score.clamp(0.0, 1.0),
            evidence: Vec::new(),
        }
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence.push(evidence.into());
        self
    }

    /// Weighted score contribution.
    pub fn weighted_score(&self) -> f64 {
        self.score * self.weight
    }
}

/// Composite risk score with breakdown by category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskScore {
    pub id: String,
    pub overall: f64,
    pub factors: Vec<RiskFactor>,
    pub breakdown: HashMap<String, f64>,
    pub assessed_at: DateTime<Utc>,
    pub agent_id: Option<String>,
    pub action: Option<String>,
}

impl RiskScore {
    /// Calculate the overall risk score from factors.
    pub fn calculate(factors: Vec<RiskFactor>) -> Self {
        let total_weight: f64 = factors.iter().map(|f| f.weight).sum();
        let weighted_sum: f64 = factors.iter().map(|f| f.weighted_score()).sum();
        let overall = if total_weight > 0.0 {
            (weighted_sum / total_weight).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let mut breakdown = HashMap::new();
        for factor in &factors {
            breakdown.insert(factor.category.to_string(), factor.score);
        }

        Self {
            id: Uuid::new_v4().to_string(),
            overall,
            factors,
            breakdown,
            assessed_at: Utc::now(),
            agent_id: None,
            action: None,
        }
    }

    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }
}

/// Alert generated when risk exceeds threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAlert {
    pub score_id: String,
    pub overall_score: f64,
    pub threshold: f64,
    pub categories: Vec<RiskCategory>,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Risk Engine
// ---------------------------------------------------------------------------

/// Engine for assessing risk and managing thresholds.
pub struct RiskEngine {
    threshold: f64,
    history: Vec<RiskScore>,
    alerts: Vec<RiskAlert>,
    category_thresholds: HashMap<RiskCategory, f64>,
}

impl Default for RiskEngine {
    fn default() -> Self {
        Self::new(0.7)
    }
}

impl RiskEngine {
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold: threshold.clamp(0.0, 1.0),
            history: Vec::new(),
            alerts: Vec::new(),
            category_thresholds: HashMap::new(),
        }
    }

    /// Set the global risk threshold.
    pub fn set_threshold(&mut self, threshold: f64) {
        self.threshold = threshold.clamp(0.0, 1.0);
    }

    /// Set a per-category threshold.
    pub fn set_category_threshold(&mut self, category: RiskCategory, threshold: f64) {
        self.category_thresholds.insert(category, threshold.clamp(0.0, 1.0));
    }

    /// Assess risk from a set of factors and return the score.
    pub fn assess(&mut self, factors: Vec<RiskFactor>) -> RiskScore {
        let score = RiskScore::calculate(factors);

        // Check if any threshold is exceeded
        let mut alerting_categories = Vec::new();
        if score.overall >= self.threshold {
            alerting_categories.push(RiskCategory::Unauthorized); // generic flag
        }

        for factor in &score.factors {
            if let Some(&cat_threshold) = self.category_thresholds.get(&factor.category) {
                if factor.score >= cat_threshold {
                    alerting_categories.push(factor.category);
                }
            }
        }

        if !alerting_categories.is_empty() {
            self.alerts.push(RiskAlert {
                score_id: score.id.clone(),
                overall_score: score.overall,
                threshold: self.threshold,
                categories: alerting_categories,
                message: format!("Risk score {:.2} exceeds threshold {:.2}", score.overall, self.threshold),
                timestamp: Utc::now(),
            });
        }

        self.history.push(score.clone());
        score
    }

    /// Get risk assessment history.
    pub fn history(&self) -> &[RiskScore] {
        &self.history
    }

    /// Get all alerts.
    pub fn alerts(&self) -> &[RiskAlert] {
        &self.alerts
    }

    /// Get the current threshold.
    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Get a specific assessment by ID.
    pub fn get(&self, id: &str) -> RiskResult<&RiskScore> {
        self.history
            .iter()
            .find(|s| s.id == id)
            .ok_or_else(|| RiskError::NotFound(id.to_string()))
    }

    /// Clear history.
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.alerts.clear();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_factor_creation() {
        let factor = RiskFactor::new(RiskCategory::DataExfiltration, 0.8);
        assert!((factor.score - 0.8).abs() < f64::EPSILON);
        assert!((factor.weight - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_score_clamping() {
        let factor = RiskFactor::new(RiskCategory::ModelAbuse, 1.5);
        assert!((factor.score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_risk_score_calculation() {
        let factors = vec![
            RiskFactor::new(RiskCategory::DataExfiltration, 0.3),
            RiskFactor::new(RiskCategory::PromptInjection, 0.7),
        ];
        let score = RiskScore::calculate(factors);
        assert!((score.overall - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_weighted_score() {
        let factors = vec![
            RiskFactor::new(RiskCategory::DataExfiltration, 0.2).with_weight(1.0),
            RiskFactor::new(RiskCategory::FinancialRisk, 0.8).with_weight(3.0),
        ];
        let score = RiskScore::calculate(factors);
        // Weighted: (0.2*1 + 0.8*3) / (1+3) = 2.6/4 = 0.65
        assert!((score.overall - 0.65).abs() < 0.01);
    }

    #[test]
    fn test_assess_triggers_alert() {
        let mut engine = RiskEngine::new(0.5);
        let factors = vec![
            RiskFactor::new(RiskCategory::DataExfiltration, 0.9),
        ];
        engine.assess(factors);
        assert_eq!(engine.alerts().len(), 1);
    }

    #[test]
    fn test_no_alert_below_threshold() {
        let mut engine = RiskEngine::new(0.8);
        let factors = vec![
            RiskFactor::new(RiskCategory::DataExfiltration, 0.3),
        ];
        engine.assess(factors);
        assert!(engine.alerts().is_empty());
    }

    #[test]
    fn test_history_tracking() {
        let mut engine = RiskEngine::new(0.9);
        engine.assess(vec![RiskFactor::new(RiskCategory::ModelAbuse, 0.1)]);
        engine.assess(vec![RiskFactor::new(RiskCategory::ModelAbuse, 0.2)]);
        assert_eq!(engine.history().len(), 2);
    }

    #[test]
    fn test_set_threshold() {
        let mut engine = RiskEngine::new(0.5);
        engine.set_threshold(0.3);
        assert!((engine.threshold() - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn test_category_threshold() {
        let mut engine = RiskEngine::new(0.9);
        engine.set_category_threshold(RiskCategory::FinancialRisk, 0.3);
        let factors = vec![
            RiskFactor::new(RiskCategory::FinancialRisk, 0.5),
        ];
        engine.assess(factors);
        assert_eq!(engine.alerts().len(), 1);
    }

    #[test]
    fn test_get_assessment() {
        let mut engine = RiskEngine::new(0.9);
        let score = engine.assess(vec![RiskFactor::new(RiskCategory::Unauthorized, 0.1)]);
        let fetched = engine.get(&score.id).unwrap();
        assert!((fetched.overall - score.overall).abs() < f64::EPSILON);
    }

    #[test]
    fn test_risk_category_display() {
        assert_eq!(RiskCategory::DataExfiltration.to_string(), "data_exfiltration");
        assert_eq!(RiskCategory::PromptInjection.to_string(), "prompt_injection");
    }

    #[test]
    fn test_clear_history() {
        let mut engine = RiskEngine::new(0.5);
        engine.assess(vec![RiskFactor::new(RiskCategory::ModelAbuse, 0.8)]);
        assert!(!engine.history().is_empty());
        engine.clear_history();
        assert!(engine.history().is_empty());
        assert!(engine.alerts().is_empty());
    }
}
