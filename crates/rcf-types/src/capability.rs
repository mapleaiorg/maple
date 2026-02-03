//! Capability Types
use crate::identity::IdentityRef;
use crate::temporal::TemporalValidity;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapabilityRef {
    pub capability_id: String,
    pub domain: EffectDomain,
    pub scope: ScopeConstraint,
    pub validity: TemporalValidity,
    pub issuer: IdentityRef,
}

impl CapabilityRef {
    pub fn new(
        capability_id: impl Into<String>,
        domain: EffectDomain,
        scope: ScopeConstraint,
        validity: TemporalValidity,
        issuer: IdentityRef,
    ) -> Self {
        Self {
            capability_id: capability_id.into(),
            domain,
            scope,
            validity,
            issuer,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.validity.is_valid_now()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectDomain {
    Communication,
    Finance,
    Infrastructure,
    Data,
    Governance,
    Physical,
    Computation,
    Custom(String),
}

impl EffectDomain {
    /// Get the domain name as a string
    pub fn name(&self) -> &str {
        match self {
            EffectDomain::Communication => "communication",
            EffectDomain::Finance => "finance",
            EffectDomain::Infrastructure => "infrastructure",
            EffectDomain::Data => "data",
            EffectDomain::Governance => "governance",
            EffectDomain::Physical => "physical",
            EffectDomain::Computation => "computation",
            EffectDomain::Custom(name) => name,
        }
    }

    /// Check if this is a critical domain requiring extra scrutiny
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            EffectDomain::Finance | EffectDomain::Governance | EffectDomain::Infrastructure
        )
    }

    /// Check if domains match (for capability checking)
    pub fn matches(&self, other: &EffectDomain) -> bool {
        self == other
    }
}

impl fmt::Display for EffectDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ScopeConstraint {
    pub targets: Vec<String>,
    pub operations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<ResourceLimits>,
}

impl ScopeConstraint {
    pub fn new(targets: Vec<String>, operations: Vec<String>) -> Self {
        Self {
            targets,
            operations,
            limits: None,
        }
    }

    pub fn wildcard() -> Self {
        Self {
            targets: vec!["*".to_string()],
            operations: vec!["*".to_string()],
            limits: None,
        }
    }

    pub fn global() -> Self {
        Self::wildcard()
    }

    pub fn is_global(&self) -> bool {
        self.targets.iter().any(|t| t == "*") && self.operations.iter().any(|o| o == "*")
    }

    pub fn matches(&self, target: &str, operation: &str) -> bool {
        let target_match = self.targets.iter().any(|t| {
            t == "*"
                || t == target
                || (t.ends_with('*') && target.starts_with(t.trim_end_matches('*')))
        });
        let op_match = self.operations.iter().any(|o| o == "*" || o == operation);
        target_match && op_match
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ResourceLimits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_operations: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_duration_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_data_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_concurrent: Option<u32>,
}

impl ResourceLimits {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_value(mut self, value: u64) -> Self {
        self.max_value = Some(value);
        self
    }
    pub fn with_max_operations(mut self, ops: u64) -> Self {
        self.max_operations = Some(ops);
        self
    }
    pub fn with_max_data_bytes(mut self, bytes: u64) -> Self {
        self.max_data_bytes = Some(bytes);
        self
    }
}
