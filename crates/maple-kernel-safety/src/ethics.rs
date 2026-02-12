use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Ethical Override — Safety > Agency > Accountability > Task.
///
/// Per I.S-3: No Resonator may justify harm by reference to efficiency,
/// intelligence, or emergent behavior. The priority hierarchy is absolute:
///
/// 1. **Safety** — Human safety and well-being (highest priority)
/// 2. **Agency** — Human autonomy and choice
/// 3. **Accountability** — Audit trails, provenance, transparency
/// 4. **Task** — Completing the requested operation (lowest priority)
///
/// When any higher-priority concern conflicts with a lower-priority one,
/// the higher priority wins. Always. No exceptions.

/// Priority levels in the ethical hierarchy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EthicalPriority {
    /// Task completion — lowest priority
    Task = 0,
    /// Accountability — audit trails, transparency
    Accountability = 1,
    /// Agency — human autonomy and choice
    Agency = 2,
    /// Safety — human safety and well-being — highest priority
    Safety = 3,
}

impl EthicalPriority {
    pub fn as_str(&self) -> &str {
        match self {
            EthicalPriority::Task => "Task",
            EthicalPriority::Accountability => "Accountability",
            EthicalPriority::Agency => "Agency",
            EthicalPriority::Safety => "Safety",
        }
    }
}

/// A decision that may be subject to ethical override.
#[derive(Clone, Debug)]
pub struct Decision {
    /// What the decision is about
    pub description: String,
    /// The priority level of this decision
    pub priority: EthicalPriority,
    /// Any safety concerns raised
    pub safety_concerns: Vec<SafetyConcern>,
    /// Any agency concerns raised
    pub agency_concerns: Vec<AgencyConcern>,
}

/// A safety concern that might trigger an override.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SafetyConcern {
    pub description: String,
    pub severity: ConcernSeverity,
}

/// An agency concern.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgencyConcern {
    pub description: String,
    pub severity: ConcernSeverity,
}

/// Severity of a concern.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConcernSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// The result of an ethical override evaluation.
#[derive(Clone, Debug)]
pub enum OverrideDecision {
    /// Proceed — no override needed
    Proceed,
    /// Override — higher priority concern takes precedence
    Override {
        reason: String,
        overriding_priority: EthicalPriority,
        overridden_priority: EthicalPriority,
    },
    /// Block — safety concern prevents action entirely
    Block { reason: String },
}

impl OverrideDecision {
    pub fn is_override(&self) -> bool {
        matches!(self, OverrideDecision::Override { .. } | OverrideDecision::Block { .. })
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, OverrideDecision::Block { .. })
    }
}

/// Evaluate whether an ethical override is needed.
///
/// Per I.S-3: Safety > Agency > Accountability > Task.
/// No exceptions. No justification by efficiency or intelligence.
pub fn ethical_override(decision: &Decision) -> OverrideDecision {
    // Check safety concerns first (highest priority)
    for concern in &decision.safety_concerns {
        match concern.severity {
            ConcernSeverity::Critical => {
                warn!(
                    concern = %concern.description,
                    "CRITICAL safety concern — blocking operation"
                );
                return OverrideDecision::Block {
                    reason: format!(
                        "Safety override (I.S-3): {} — Safety > {}",
                        concern.description,
                        decision.priority.as_str()
                    ),
                };
            }
            ConcernSeverity::High => {
                if decision.priority < EthicalPriority::Safety {
                    info!(
                        concern = %concern.description,
                        decision_priority = decision.priority.as_str(),
                        "Safety override triggered"
                    );
                    return OverrideDecision::Override {
                        reason: format!(
                            "Safety concern: {} — Safety overrides {}",
                            concern.description,
                            decision.priority.as_str()
                        ),
                        overriding_priority: EthicalPriority::Safety,
                        overridden_priority: decision.priority,
                    };
                }
            }
            _ => {} // Lower severity safety concerns don't override
        }
    }

    // Check agency concerns (second highest)
    for concern in &decision.agency_concerns {
        if concern.severity >= ConcernSeverity::High
            && decision.priority < EthicalPriority::Agency
        {
            info!(
                concern = %concern.description,
                decision_priority = decision.priority.as_str(),
                "Agency override triggered"
            );
            return OverrideDecision::Override {
                reason: format!(
                    "Agency concern: {} — Agency overrides {}",
                    concern.description,
                    decision.priority.as_str()
                ),
                overriding_priority: EthicalPriority::Agency,
                overridden_priority: decision.priority,
            };
        }
    }

    OverrideDecision::Proceed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safety_overrides_task() {
        let decision = Decision {
            description: "Complete file operation".into(),
            priority: EthicalPriority::Task,
            safety_concerns: vec![SafetyConcern {
                description: "Operation may harm user data".into(),
                severity: ConcernSeverity::High,
            }],
            agency_concerns: vec![],
        };

        let result = ethical_override(&decision);
        assert!(result.is_override());

        if let OverrideDecision::Override {
            overriding_priority,
            overridden_priority,
            ..
        } = result
        {
            assert_eq!(overriding_priority, EthicalPriority::Safety);
            assert_eq!(overridden_priority, EthicalPriority::Task);
        }
    }

    #[test]
    fn safety_overrides_accountability() {
        let decision = Decision {
            description: "Record audit trail".into(),
            priority: EthicalPriority::Accountability,
            safety_concerns: vec![SafetyConcern {
                description: "Audit data contains PII risk".into(),
                severity: ConcernSeverity::High,
            }],
            agency_concerns: vec![],
        };

        let result = ethical_override(&decision);
        assert!(result.is_override());
    }

    #[test]
    fn safety_overrides_agency() {
        let decision = Decision {
            description: "User chose this action".into(),
            priority: EthicalPriority::Agency,
            safety_concerns: vec![SafetyConcern {
                description: "Action would cause harm".into(),
                severity: ConcernSeverity::High,
            }],
            agency_concerns: vec![],
        };

        let result = ethical_override(&decision);
        assert!(result.is_override());
    }

    #[test]
    fn critical_safety_blocks_everything() {
        let decision = Decision {
            description: "Any operation".into(),
            priority: EthicalPriority::Safety, // Even safety-priority decisions
            safety_concerns: vec![SafetyConcern {
                description: "Critical danger to human".into(),
                severity: ConcernSeverity::Critical,
            }],
            agency_concerns: vec![],
        };

        let result = ethical_override(&decision);
        assert!(result.is_blocked());
    }

    #[test]
    fn agency_overrides_task() {
        let decision = Decision {
            description: "Complete automation".into(),
            priority: EthicalPriority::Task,
            safety_concerns: vec![],
            agency_concerns: vec![AgencyConcern {
                description: "User did not consent to this automation".into(),
                severity: ConcernSeverity::High,
            }],
        };

        let result = ethical_override(&decision);
        assert!(result.is_override());

        if let OverrideDecision::Override {
            overriding_priority, ..
        } = result
        {
            assert_eq!(overriding_priority, EthicalPriority::Agency);
        }
    }

    #[test]
    fn agency_overrides_accountability() {
        let decision = Decision {
            description: "Log user behavior".into(),
            priority: EthicalPriority::Accountability,
            safety_concerns: vec![],
            agency_concerns: vec![AgencyConcern {
                description: "User declined monitoring".into(),
                severity: ConcernSeverity::High,
            }],
        };

        let result = ethical_override(&decision);
        assert!(result.is_override());
    }

    #[test]
    fn agency_does_not_override_safety() {
        let decision = Decision {
            description: "Safety operation".into(),
            priority: EthicalPriority::Safety,
            safety_concerns: vec![],
            agency_concerns: vec![AgencyConcern {
                description: "User prefers different approach".into(),
                severity: ConcernSeverity::High,
            }],
        };

        let result = ethical_override(&decision);
        // Agency does NOT override Safety
        assert!(matches!(result, OverrideDecision::Proceed));
    }

    #[test]
    fn no_override_when_no_concerns() {
        let decision = Decision {
            description: "Normal operation".into(),
            priority: EthicalPriority::Task,
            safety_concerns: vec![],
            agency_concerns: vec![],
        };

        let result = ethical_override(&decision);
        assert!(matches!(result, OverrideDecision::Proceed));
    }

    #[test]
    fn low_severity_does_not_trigger_override() {
        let decision = Decision {
            description: "Task operation".into(),
            priority: EthicalPriority::Task,
            safety_concerns: vec![SafetyConcern {
                description: "Minor concern".into(),
                severity: ConcernSeverity::Low,
            }],
            agency_concerns: vec![],
        };

        let result = ethical_override(&decision);
        assert!(matches!(result, OverrideDecision::Proceed));
    }

    #[test]
    fn priority_ordering_is_correct() {
        assert!(EthicalPriority::Safety > EthicalPriority::Agency);
        assert!(EthicalPriority::Agency > EthicalPriority::Accountability);
        assert!(EthicalPriority::Accountability > EthicalPriority::Task);
    }
}
