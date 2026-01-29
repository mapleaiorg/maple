//! Uncertainty Types
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Uncertainty {
    pub confidence: f64,
    pub uncertainty_type: UncertaintyType,
    pub evidence_refs: Vec<EvidenceRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
}

impl Uncertainty {
    pub fn new(confidence: f64, uncertainty_type: UncertaintyType) -> Self {
        Self {
            confidence: confidence.clamp(0.0, 1.0),
            uncertainty_type,
            evidence_refs: Vec::new(),
            explanation: None,
        }
    }
    
    pub fn is_sufficient_for_intent(&self) -> bool { self.confidence > 0.5 }
    pub fn is_sufficient_for_commitment(&self) -> bool { self.confidence > 0.7 }
}

impl Default for Uncertainty {
    fn default() -> Self {
        Self { confidence: 0.5, uncertainty_type: UncertaintyType::Epistemic, evidence_refs: Vec::new(), explanation: None }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UncertaintyType {
    Epistemic, Aleatoric, Ambiguity, Incompleteness, Conflict, Temporal, Model,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub source: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub hash: [u8; 32],
    pub evidence_type: EvidenceType,
    pub relevance: u8, // 0-100
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    Observation, Inference, Attestation, Historical, Computational, Testimony,
}
