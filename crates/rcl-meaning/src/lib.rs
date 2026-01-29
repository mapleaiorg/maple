//! RCL-Meaning Layer - Non-executable interpretation
#![deny(unsafe_code)]

use rcl_types::{IdentityRef, ResonanceArtifact, ResonanceType, TemporalAnchor, Uncertainty};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RclMeaning {
    pub id: String,
    pub author: IdentityRef,
    pub anchor: TemporalAnchor,
    pub claims: Vec<Claim>,
    pub uncertainty: Uncertainty,
    pub schema_version: String,
}

impl RclMeaning {
    pub fn new(author: IdentityRef, claims: Vec<Claim>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            author,
            anchor: TemporalAnchor::now(),
            claims,
            uncertainty: Uncertainty::default(),
            schema_version: rcl_types::SCHEMA_VERSION.to_string(),
        }
    }
    
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.uncertainty.confidence = confidence.clamp(0.0, 1.0);
        self
    }
    
    pub fn validate(&self) -> Result<(), MeaningValidationError> { Ok(()) }
    
    #[inline]
    pub fn is_executable(&self) -> bool { false }
}

impl ResonanceArtifact for RclMeaning {
    fn resonance_type(&self) -> ResonanceType { ResonanceType::Meaning }
    fn artifact_id(&self) -> &str { &self.id }
    fn is_executable(&self) -> bool { false }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Claim {
    pub claim_type: ClaimType,
    pub content: String,
    pub confidence: f64,
}

impl Claim {
    pub fn belief(content: impl Into<String>) -> Self {
        Self { claim_type: ClaimType::Belief, content: content.into(), confidence: 0.5 }
    }
    pub fn observation(content: impl Into<String>) -> Self {
        Self { claim_type: ClaimType::Observation, content: content.into(), confidence: 0.9 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClaimType { Belief, Hypothesis, Observation, Inference, Explanation }

#[derive(Debug, thiserror::Error)]
pub enum MeaningValidationError {
    #[error("Wrong resonance type: expected Meaning, got {0}")]
    WrongResonanceType(ResonanceType),
}
