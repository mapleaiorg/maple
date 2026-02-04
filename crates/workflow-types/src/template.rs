//! Commitment templates: parameterized commitment declarations
//!
//! A CommitmentTemplate is a blueprint for creating commitments
//! at workflow execution time. Parameters get filled in with
//! runtime data (e.g., the specific resonator assigned to a role).

use collective_types::{ReceiptType, RoleId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A parameterized commitment template instantiated at workflow nodes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitmentTemplate {
    /// Human-readable name for this commitment
    pub name: String,
    /// Description of what the commitment entails
    pub description: String,
    /// The action type this commitment represents
    pub action_type: CommitmentActionType,
    /// Parameters that can be filled at instantiation time
    pub parameters: Vec<TemplateParameter>,
    /// Receipts that must be produced when this commitment is fulfilled
    pub required_receipts: Vec<ReceiptTemplate>,
    /// The role that must fulfill this commitment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fulfiller_role: Option<RoleId>,
    /// Estimated value (for risk-tiered threshold evaluation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_value: Option<u64>,
    /// Maximum time allowed for fulfillment (seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_duration_secs: Option<u64>,
    /// Whether this commitment is reversible
    pub reversible: bool,
    /// Metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl CommitmentTemplate {
    /// Create a new commitment template
    pub fn new(name: impl Into<String>, action_type: CommitmentActionType) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            action_type,
            parameters: Vec::new(),
            required_receipts: Vec::new(),
            fulfiller_role: None,
            estimated_value: None,
            max_duration_secs: None,
            reversible: false,
            metadata: HashMap::new(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_parameter(mut self, param: TemplateParameter) -> Self {
        self.parameters.push(param);
        self
    }

    pub fn with_required_receipt(mut self, receipt: ReceiptTemplate) -> Self {
        self.required_receipts.push(receipt);
        self
    }

    pub fn with_fulfiller_role(mut self, role: RoleId) -> Self {
        self.fulfiller_role = Some(role);
        self
    }

    pub fn with_estimated_value(mut self, value: u64) -> Self {
        self.estimated_value = Some(value);
        self
    }

    pub fn with_max_duration(mut self, secs: u64) -> Self {
        self.max_duration_secs = Some(secs);
        self
    }

    pub fn reversible(mut self) -> Self {
        self.reversible = true;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Validate that all required parameters are accounted for
    pub fn required_parameters(&self) -> Vec<&TemplateParameter> {
        self.parameters.iter().filter(|p| p.required).collect()
    }

    /// Check if a set of parameter values satisfies all required parameters
    pub fn validate_parameters(&self, values: &HashMap<String, String>) -> Vec<String> {
        let mut missing = Vec::new();
        for param in &self.parameters {
            if param.required && !values.contains_key(&param.name) {
                missing.push(param.name.clone());
            }
        }
        missing
    }
}

/// The action type a commitment template represents
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommitmentActionType {
    /// Review and approve/reject something
    Review,
    /// Execute a specific task
    Execute,
    /// Transfer resources (financial, attention, etc.)
    Transfer,
    /// Create or produce something
    Create,
    /// Verify or audit something
    Verify,
    /// Delegate authority or responsibility
    Delegate,
    /// Escalate an issue
    Escalate,
    /// Custom action type
    Custom(String),
}

/// A parameter in a commitment template
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemplateParameter {
    /// Parameter name (used as key)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// The parameter type
    pub param_type: ParameterType,
    /// Whether this parameter is required
    pub required: bool,
    /// Default value (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
}

impl TemplateParameter {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        param_type: ParameterType,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type,
            required: true,
            default_value: None,
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn with_default(mut self, value: impl Into<String>) -> Self {
        self.default_value = Some(value.into());
        self.required = false;
        self
    }
}

/// The type of a template parameter
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterType {
    /// A string value
    String,
    /// A numeric value
    Number,
    /// A boolean value
    Boolean,
    /// A resonator ID
    ResonatorId,
    /// A role ID
    RoleId,
    /// A financial amount
    Amount,
    /// An attention units value
    AttentionUnits,
    /// A duration in seconds
    DurationSecs,
    /// Custom type
    Custom(String),
}

/// A receipt template — describes what receipt must be produced
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiptTemplate {
    /// The type of receipt to produce
    pub receipt_type: ReceiptType,
    /// Description of what this receipt proves
    pub description: String,
    /// Whether this receipt is mandatory
    pub mandatory: bool,
    /// Expected data fields in the receipt
    pub expected_fields: Vec<String>,
}

impl ReceiptTemplate {
    pub fn new(receipt_type: ReceiptType, description: impl Into<String>) -> Self {
        Self {
            receipt_type,
            description: description.into(),
            mandatory: true,
            expected_fields: Vec::new(),
        }
    }

    pub fn optional(mut self) -> Self {
        self.mandatory = false;
        self
    }

    pub fn with_expected_field(mut self, field: impl Into<String>) -> Self {
        self.expected_fields.push(field.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_commitment_template() {
        let template = CommitmentTemplate::new("Review Document", CommitmentActionType::Review)
            .with_description("Review and approve a submitted document")
            .with_fulfiller_role(RoleId::new("reviewer"))
            .with_estimated_value(100)
            .with_max_duration(3600)
            .reversible();

        assert_eq!(template.name, "Review Document");
        assert_eq!(template.action_type, CommitmentActionType::Review);
        assert!(template.reversible);
        assert_eq!(template.estimated_value, Some(100));
        assert_eq!(template.max_duration_secs, Some(3600));
    }

    #[test]
    fn test_template_parameters() {
        let template = CommitmentTemplate::new("Transfer", CommitmentActionType::Transfer)
            .with_parameter(TemplateParameter::new(
                "amount",
                "Amount to transfer",
                ParameterType::Amount,
            ))
            .with_parameter(
                TemplateParameter::new("note", "Optional note", ParameterType::String).optional(),
            )
            .with_parameter(
                TemplateParameter::new("currency", "Currency type", ParameterType::String)
                    .with_default("MAPLE"),
            );

        assert_eq!(template.parameters.len(), 3);
        assert_eq!(template.required_parameters().len(), 1);

        // Validate with missing required param
        let empty: HashMap<String, String> = HashMap::new();
        let missing = template.validate_parameters(&empty);
        assert_eq!(missing, vec!["amount"]);

        // Validate with all required params
        let mut values = HashMap::new();
        values.insert("amount".to_string(), "1000".to_string());
        let missing = template.validate_parameters(&values);
        assert!(missing.is_empty());
    }

    #[test]
    fn test_receipt_template() {
        let receipt = ReceiptTemplate::new(
            ReceiptType::CommitmentFulfilled,
            "Proof that document was reviewed",
        )
        .with_expected_field("reviewer_id")
        .with_expected_field("decision");

        assert!(receipt.mandatory);
        assert_eq!(receipt.expected_fields.len(), 2);

        let optional = ReceiptTemplate::new(ReceiptType::Audit, "Optional audit trail").optional();
        assert!(!optional.mandatory);
    }

    #[test]
    fn test_parameter_types() {
        let params = vec![
            TemplateParameter::new("name", "Name", ParameterType::String),
            TemplateParameter::new("count", "Count", ParameterType::Number),
            TemplateParameter::new("active", "Active", ParameterType::Boolean),
            TemplateParameter::new("agent", "Agent", ParameterType::ResonatorId),
            TemplateParameter::new("role", "Role", ParameterType::RoleId),
            TemplateParameter::new("value", "Value", ParameterType::Amount),
            TemplateParameter::new("attention", "AU", ParameterType::AttentionUnits),
            TemplateParameter::new("timeout", "Timeout", ParameterType::DurationSecs),
        ];
        assert_eq!(params.len(), 8);
        assert!(params.iter().all(|p| p.required));
    }

    #[test]
    fn test_commitment_action_types() {
        let types = vec![
            CommitmentActionType::Review,
            CommitmentActionType::Execute,
            CommitmentActionType::Transfer,
            CommitmentActionType::Create,
            CommitmentActionType::Verify,
            CommitmentActionType::Delegate,
            CommitmentActionType::Escalate,
            CommitmentActionType::Custom("deploy".to_string()),
        ];
        assert_eq!(types.len(), 8);
        assert_eq!(
            CommitmentActionType::Custom("x".to_string()),
            CommitmentActionType::Custom("x".to_string())
        );
    }

    #[test]
    fn test_template_with_receipts() {
        let template = CommitmentTemplate::new("Audit", CommitmentActionType::Verify)
            .with_required_receipt(
                ReceiptTemplate::new(ReceiptType::Audit, "Audit completed")
                    .with_expected_field("findings"),
            )
            .with_required_receipt(ReceiptTemplate::new(
                ReceiptType::CommitmentFulfilled,
                "Verification done",
            ));

        assert_eq!(template.required_receipts.len(), 2);
    }
}
