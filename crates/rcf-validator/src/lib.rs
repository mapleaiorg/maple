#![deny(unsafe_code)]
use rcf_commitment::RcfCommitment;
use rcf_intent::RcfIntent;
use rcf_meaning::RcfMeaning;
use rcf_types::{ResonanceArtifact, ResonanceType};

pub struct RcfValidator;
impl RcfValidator {
    pub fn new() -> Self { Self }
    pub fn validate_meaning(&self, m: &RcfMeaning) -> Result<(), ValidationError> { 
        if m.resonance_type() != ResonanceType::Meaning { return Err(ValidationError::WrongType); }
        Ok(()) 
    }
    pub fn validate_intent(&self, i: &RcfIntent) -> Result<(), ValidationError> { 
        if i.resonance_type() != ResonanceType::Intent { return Err(ValidationError::WrongType); }
        Ok(()) 
    }
    pub fn validate_commitment(&self, c: &RcfCommitment) -> Result<(), ValidationError> { 
        if c.resonance_type() != ResonanceType::Commitment { return Err(ValidationError::WrongType); }
        c.validate().map_err(|_| ValidationError::InvalidStructure)?;
        Ok(()) 
    }
}
impl Default for RcfValidator { fn default() -> Self { Self::new() } }

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Wrong type")] WrongType,
    #[error("Invalid structure")] InvalidStructure,
}
