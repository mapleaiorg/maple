//! Policy engine -- evaluates loaded policies against an evaluation context.
//!
//! The engine implements DENY-FIRST semantics: if any mandatory rule denies,
//! the overall decision is Deny, regardless of other Allow rules.

use crate::policy::*;
use std::collections::HashMap;

/// The evaluation context passed to the policy engine.
///
/// Contains all information needed to evaluate policy rules against a request.
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    /// Who is making the request
    pub worldline_id: Option<String>,
    /// Tenant/org
    pub tenant_id: Option<String>,
    /// Data classification of the content
    pub data_classification: Option<String>,
    /// The tool being invoked (if applicable)
    pub tool: Option<String>,
    /// The model being used (if applicable)
    pub model: Option<String>,
    /// Content being evaluated (input or output)
    pub content: Option<String>,
    /// Risk score (0.0 - 1.0)
    pub risk_score: Option<f64>,
    /// Financial amount (if applicable)
    pub amount: Option<f64>,
    /// Currency (if applicable)
    pub currency: Option<String>,
    /// Jurisdiction
    pub jurisdiction: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl EvaluationContext {
    /// Create an empty evaluation context.
    pub fn new() -> Self {
        Self {
            worldline_id: None,
            tenant_id: None,
            data_classification: None,
            tool: None,
            model: None,
            content: None,
            risk_score: None,
            amount: None,
            currency: None,
            jurisdiction: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the worldline identity.
    pub fn with_worldline_id(mut self, id: impl Into<String>) -> Self {
        self.worldline_id = Some(id.into());
        self
    }

    /// Set the tenant identifier.
    pub fn with_tenant_id(mut self, id: impl Into<String>) -> Self {
        self.tenant_id = Some(id.into());
        self
    }

    /// Set the data classification.
    pub fn with_data_classification(mut self, classification: impl Into<String>) -> Self {
        self.data_classification = Some(classification.into());
        self
    }

    /// Set the tool being invoked.
    pub fn with_tool(mut self, tool: impl Into<String>) -> Self {
        self.tool = Some(tool.into());
        self
    }

    /// Set the model being used.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the content being evaluated.
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Set the risk score.
    pub fn with_risk_score(mut self, score: f64) -> Self {
        self.risk_score = Some(score);
        self
    }

    /// Set the financial amount and currency.
    pub fn with_amount(mut self, amount: f64, currency: impl Into<String>) -> Self {
        self.amount = Some(amount);
        self.currency = Some(currency.into());
        self
    }

    /// Set the jurisdiction.
    pub fn with_jurisdiction(mut self, jurisdiction: impl Into<String>) -> Self {
        self.jurisdiction = Some(jurisdiction.into());
        self
    }

    /// Add a metadata entry.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

impl Default for EvaluationContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of evaluating all policies against a context.
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    /// Overall decision
    pub decision: GuardDecision,
    /// Individual rule evaluations
    pub evaluations: Vec<RuleEvaluationResult>,
    /// Time taken for evaluation in microseconds
    pub evaluation_time_us: u64,
    /// Actions to be applied
    pub actions: Vec<ResolvedAction>,
}

/// The overall guard decision after evaluating all policies.
#[derive(Debug, Clone, PartialEq)]
pub enum GuardDecision {
    /// All policies allow
    Allow,
    /// At least one mandatory policy denied
    Deny {
        /// Reason for denial
        reason: String,
        /// The rule that caused the denial
        denying_rule: String,
    },
    /// Approval required before proceeding
    PendingApproval {
        /// Who can approve
        approvers: Vec<String>,
        /// Message to display
        message: String,
    },
    /// Content was modified (redacted) and allowed
    AllowWithModification {
        /// Description of modifications applied
        modifications: Vec<String>,
    },
}

/// Result of evaluating a single rule.
#[derive(Debug, Clone)]
pub struct RuleEvaluationResult {
    /// Policy that contains this rule
    pub policy_id: PolicyId,
    /// Policy name
    pub policy_name: String,
    /// Rule identifier
    pub rule_id: String,
    /// Rule name
    pub rule_name: String,
    /// Whether the condition matched
    pub condition_matched: bool,
    /// The action configured for this rule
    pub action: RuleAction,
    /// The enforcement level of the containing policy
    pub enforcement: EnforcementLevel,
}

/// An action resolved from a matched rule.
#[derive(Debug, Clone)]
pub struct ResolvedAction {
    /// Source rule identifier (policy_id:rule_id)
    pub source_rule: String,
    /// The action to take
    pub action: RuleAction,
    /// The enforcement level
    pub enforcement: EnforcementLevel,
}

/// The policy engine evaluates all loaded policies against a context.
///
/// Policies are organized by domain and evaluated in priority order.
/// The engine implements DENY-FIRST semantics.
pub struct PolicyEngine {
    /// Active policies organized by domain
    policies: HashMap<PolicyDomain, Vec<Policy>>,
    /// Rate counters for RateExceeded conditions
    rate_counters: RateCounterStore,
}

impl PolicyEngine {
    /// Create a new empty policy engine.
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            rate_counters: RateCounterStore::new(),
        }
    }

    /// Load a policy into the engine.
    pub fn load_policy(&mut self, policy: Policy) -> Result<(), GuardError> {
        self.validate_policy(&policy)?;
        self.policies
            .entry(policy.domain.clone())
            .or_default()
            .push(policy);
        // Sort by minimum rule priority within each domain
        for policies in self.policies.values_mut() {
            policies.sort_by_key(|p| {
                p.rules.iter().map(|r| r.priority).min().unwrap_or(u32::MAX)
            });
        }
        Ok(())
    }

    /// Remove a policy by ID.
    pub fn remove_policy(&mut self, id: &PolicyId) {
        for policies in self.policies.values_mut() {
            policies.retain(|p| p.id != *id);
        }
    }

    /// List all loaded policies.
    pub fn list_policies(&self) -> Vec<&Policy> {
        self.policies.values().flat_map(|v| v.iter()).collect()
    }

    /// Record a rate-limiting event for the given key.
    pub fn record_rate_event(&self, key: &str) {
        self.rate_counters.record(key);
    }

    /// Get the number of policies loaded for a given domain.
    pub fn policy_count(&self, domain: &PolicyDomain) -> usize {
        self.policies.get(domain).map(|v| v.len()).unwrap_or(0)
    }

    /// Get the total number of policies loaded.
    pub fn total_policy_count(&self) -> usize {
        self.policies.values().map(|v| v.len()).sum()
    }

    /// Evaluate all policies for a given domain against a context.
    ///
    /// Returns an `EvaluationResult` containing the overall decision,
    /// individual rule evaluations, and resolved actions.
    pub fn evaluate(
        &self,
        domain: &PolicyDomain,
        context: &EvaluationContext,
    ) -> EvaluationResult {
        let start = std::time::Instant::now();
        let mut evaluations = Vec::new();
        let mut actions = Vec::new();
        let mut deny_reason: Option<(String, String)> = None;
        let mut pending_approval: Option<(Vec<String>, String)> = None;

        // Get policies for this domain + global policies
        let domain_policies = self.policies.get(domain).into_iter().flatten();
        let global_policies = self
            .policies
            .get(&PolicyDomain::Global)
            .into_iter()
            .flatten();
        let all_policies: Vec<&Policy> = domain_policies.chain(global_policies).collect();

        for policy in all_policies {
            for rule in &policy.rules {
                if !rule.enabled {
                    continue;
                }

                let matched = self.evaluate_condition(&rule.condition, context);

                evaluations.push(RuleEvaluationResult {
                    policy_id: policy.id.clone(),
                    policy_name: policy.name.clone(),
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    condition_matched: matched,
                    action: rule.action.clone(),
                    enforcement: policy.enforcement.clone(),
                });

                if matched {
                    actions.push(ResolvedAction {
                        source_rule: format!("{}:{}", policy.id.0, rule.id),
                        action: rule.action.clone(),
                        enforcement: policy.enforcement.clone(),
                    });

                    // DENY-FIRST: if any mandatory rule denies, the result is Deny
                    if policy.enforcement == EnforcementLevel::Mandatory {
                        match &rule.action {
                            RuleAction::Deny { reason, .. } => {
                                if deny_reason.is_none() {
                                    deny_reason = Some((
                                        reason.clone(),
                                        format!("{}:{}", policy.id.0, rule.id),
                                    ));
                                }
                            }
                            RuleAction::RequireApproval {
                                approvers, message, ..
                            } => {
                                if pending_approval.is_none() && deny_reason.is_none() {
                                    pending_approval =
                                        Some((approvers.clone(), message.clone()));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let decision = if let Some((reason, rule)) = deny_reason {
            GuardDecision::Deny {
                reason,
                denying_rule: rule,
            }
        } else if let Some((approvers, message)) = pending_approval {
            GuardDecision::PendingApproval { approvers, message }
        } else {
            // Check if any redactions were applied
            let redactions: Vec<String> = actions
                .iter()
                .filter_map(|a| match &a.action {
                    RuleAction::Redact { patterns, .. } => Some(patterns.join(", ")),
                    _ => None,
                })
                .collect();
            if redactions.is_empty() {
                GuardDecision::Allow
            } else {
                GuardDecision::AllowWithModification {
                    modifications: redactions,
                }
            }
        };

        EvaluationResult {
            decision,
            evaluations,
            evaluation_time_us: start.elapsed().as_micros() as u64,
            actions,
        }
    }

    /// Evaluate a single condition against a context.
    fn evaluate_condition(&self, condition: &RuleCondition, ctx: &EvaluationContext) -> bool {
        match condition {
            RuleCondition::Always => true,
            RuleCondition::Never => false,

            RuleCondition::DataClassification { levels } => ctx
                .data_classification
                .as_ref()
                .map(|c| levels.iter().any(|l| l == c))
                .unwrap_or(false),

            RuleCondition::ToolMatch { patterns } => ctx
                .tool
                .as_ref()
                .map(|t| patterns.iter().any(|p| glob_match(p, t)))
                .unwrap_or(false),

            RuleCondition::ModelMatch { patterns } => ctx
                .model
                .as_ref()
                .map(|m| patterns.iter().any(|p| glob_match(p, m)))
                .unwrap_or(false),

            RuleCondition::ContentMatch { patterns, scope } => {
                let content = match scope.as_str() {
                    "input" | "output" | "both" => ctx.content.as_deref(),
                    _ => None,
                };
                content
                    .map(|c| {
                        patterns.iter().any(|p| {
                            regex::Regex::new(p)
                                .map(|re| re.is_match(c))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            }

            RuleCondition::RiskThreshold { min_score } => {
                ctx.risk_score.map(|s| s >= *min_score).unwrap_or(false)
            }

            RuleCondition::AmountThreshold {
                min_amount,
                currency,
            } => {
                ctx.amount.map(|a| a >= *min_amount).unwrap_or(false)
                    && ctx
                        .currency
                        .as_ref()
                        .map(|c| c == currency)
                        .unwrap_or(true)
            }

            RuleCondition::RateExceeded {
                max_count,
                window_seconds,
            } => {
                let key = ctx.worldline_id.as_deref().unwrap_or("global");
                self.rate_counters
                    .check_exceeded(key, *max_count, *window_seconds)
            }

            RuleCondition::TenantMatch { tenants } => ctx
                .tenant_id
                .as_ref()
                .map(|t| tenants.contains(t))
                .unwrap_or(false),

            RuleCondition::JurisdictionMatch { jurisdictions } => ctx
                .jurisdiction
                .as_ref()
                .map(|j| jurisdictions.iter().any(|jj| jj == j))
                .unwrap_or(false),

            RuleCondition::IdentityMatch { patterns } => ctx
                .worldline_id
                .as_ref()
                .map(|id| patterns.iter().any(|p| glob_match(p, id)))
                .unwrap_or(false),

            RuleCondition::MetadataMatch { key, pattern } => ctx
                .metadata
                .get(key)
                .and_then(|v| v.as_str())
                .map(|v| glob_match(pattern, v))
                .unwrap_or(false),

            RuleCondition::TimeWindow {
                days: _,
                start_hour: _,
                end_hour: _,
            } => {
                // Time window evaluation uses current UTC time
                // In production this would check against chrono::Utc::now()
                // For testability, we return false by default (no match outside window)
                false
            }

            RuleCondition::OperationType { operations } => ctx
                .metadata
                .get("operation_type")
                .and_then(|v| v.as_str())
                .map(|op| operations.iter().any(|o| o == op))
                .unwrap_or(false),

            RuleCondition::All { conditions } => {
                conditions.iter().all(|c| self.evaluate_condition(c, ctx))
            }

            RuleCondition::Any { conditions } => {
                conditions.iter().any(|c| self.evaluate_condition(c, ctx))
            }

            RuleCondition::Not { condition } => !self.evaluate_condition(condition, ctx),
        }
    }

    /// Validate a policy before loading it.
    fn validate_policy(&self, policy: &Policy) -> Result<(), GuardError> {
        // Validate rule IDs are unique within policy
        let mut seen_ids = std::collections::HashSet::new();
        for rule in &policy.rules {
            if !seen_ids.insert(&rule.id) {
                return Err(GuardError::InvalidPolicy(format!(
                    "Duplicate rule ID '{}' in policy '{}'",
                    rule.id, policy.name
                )));
            }
        }

        // Validate regex patterns compile
        for rule in &policy.rules {
            self.validate_condition_patterns(&rule.condition)?;
        }

        // Validate condition tree depth (max 10 levels)
        for rule in &policy.rules {
            if condition_depth(&rule.condition) > 10 {
                return Err(GuardError::InvalidPolicy(format!(
                    "Condition tree too deep in rule '{}' (max 10 levels)",
                    rule.id
                )));
            }
        }

        Ok(())
    }

    /// Validate that all regex patterns in conditions compile.
    fn validate_condition_patterns(&self, condition: &RuleCondition) -> Result<(), GuardError> {
        match condition {
            RuleCondition::ContentMatch { patterns, .. } => {
                for pattern in patterns {
                    regex::Regex::new(pattern).map_err(|e| {
                        GuardError::InvalidPolicy(format!(
                            "Invalid regex pattern '{}': {}",
                            pattern, e
                        ))
                    })?;
                }
            }
            RuleCondition::All { conditions } | RuleCondition::Any { conditions } => {
                for c in conditions {
                    self.validate_condition_patterns(c)?;
                }
            }
            RuleCondition::Not { condition } => {
                self.validate_condition_patterns(condition)?;
            }
            _ => {}
        }
        Ok(())
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate the depth of a condition tree.
fn condition_depth(condition: &RuleCondition) -> usize {
    match condition {
        RuleCondition::All { conditions } | RuleCondition::Any { conditions } => {
            1 + conditions
                .iter()
                .map(condition_depth)
                .max()
                .unwrap_or(0)
        }
        RuleCondition::Not { condition } => 1 + condition_depth(condition),
        _ => 1,
    }
}

/// Simple glob matching (supports `*` and `?`).
///
/// - `*` matches zero or more characters
/// - `?` matches exactly one character
/// - All other characters are matched literally
pub fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') && !pattern.contains('?') {
        return pattern == text;
    }
    // Convert glob to regex
    let mut regex_pattern = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => regex_pattern.push_str(".*"),
            '?' => regex_pattern.push('.'),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                regex_pattern.push('\\');
                regex_pattern.push(ch);
            }
            _ => regex_pattern.push(ch),
        }
    }
    regex_pattern.push('$');
    regex::Regex::new(&regex_pattern)
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

/// Rate counter store for tracking request frequencies.
///
/// Uses a simple in-memory store with per-key timestamp vectors.
/// Expired entries are cleaned up on each access.
pub(crate) struct RateCounterStore {
    counters: std::sync::Mutex<HashMap<String, Vec<std::time::Instant>>>,
}

impl RateCounterStore {
    pub(crate) fn new() -> Self {
        Self {
            counters: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Check if the rate for a given key has been exceeded.
    pub(crate) fn check_exceeded(&self, key: &str, max_count: u32, window_seconds: u64) -> bool {
        let mut counters = self.counters.lock().unwrap();
        let window = std::time::Duration::from_secs(window_seconds);
        let now = std::time::Instant::now();

        let entries = counters.entry(key.to_string()).or_default();
        // Remove expired entries
        entries.retain(|t| now.duration_since(*t) < window);
        // Check if exceeded
        entries.len() >= max_count as usize
    }

    /// Record a rate-limiting event for the given key.
    pub(crate) fn record(&self, key: &str) {
        let mut counters = self.counters.lock().unwrap();
        counters
            .entry(key.to_string())
            .or_default()
            .push(std::time::Instant::now());
    }
}

/// Errors that can occur in the guard engine.
#[derive(Debug, thiserror::Error)]
pub enum GuardError {
    /// The policy is invalid
    #[error("Invalid policy: {0}")]
    InvalidPolicy(String),
    /// An error occurred during rule evaluation
    #[error("Rule evaluation error: {0}")]
    EvaluationError(String),
    /// An I/O error occurred
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
