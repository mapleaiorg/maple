//! RCL-Intent Layer - Non-executable goals and plans
#![deny(unsafe_code)]

use rcl_meaning::RclMeaning;
use rcl_types::{IdentityRef, ResonanceArtifact, ResonanceType, TemporalAnchor};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RclIntent {
    pub id: String,
    pub author: IdentityRef,
    pub anchor: TemporalAnchor,
    pub goals: Vec<Goal>,
    pub meaning_refs: Vec<String>,
    pub confidence: f64,
    pub schema_version: String,
}

impl RclIntent {
    pub fn new(author: IdentityRef, goals: Vec<Goal>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            author,
            anchor: TemporalAnchor::now(),
            goals,
            meaning_refs: Vec::new(),
            confidence: 0.5,
            schema_version: rcl_types::SCHEMA_VERSION.to_string(),
        }
    }
    
    pub fn from_meaning(meaning: &RclMeaning, goals: Vec<Goal>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            author: meaning.author.clone(),
            anchor: TemporalAnchor::now(),
            goals,
            meaning_refs: vec![meaning.id.clone()],
            confidence: meaning.uncertainty.confidence,
            schema_version: rcl_types::SCHEMA_VERSION.to_string(),
        }
    }
    
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0); self
    }
    
    pub fn is_sufficient_for_commitment(&self) -> bool { self.confidence >= 0.7 && !self.goals.is_empty() }
    pub fn validate(&self) -> Result<(), IntentValidationError> { Ok(()) }
    
    #[inline]
    pub fn is_executable(&self) -> bool { false }
}

impl ResonanceArtifact for RclIntent {
    fn resonance_type(&self) -> ResonanceType { ResonanceType::Intent }
    fn artifact_id(&self) -> &str { &self.id }
    fn is_executable(&self) -> bool { false }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub description: String,
    pub priority: u32,
}

impl Goal {
    pub fn new(description: impl Into<String>) -> Self {
        Self { id: uuid::Uuid::new_v4().to_string(), description: description.into(), priority: 50 }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IntentValidationError {
    #[error("Wrong resonance type")]
    WrongResonanceType(ResonanceType),
    #[error("No goals")]
    NoGoals,
}
