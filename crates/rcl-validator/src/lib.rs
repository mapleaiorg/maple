#![deny(unsafe_code)]
use rcl_commitment::RclCommitment;
use rcl_intent::RclIntent;
use rcl_meaning::RclMeaning;
use rcl_types::{ResonanceArtifact, ResonanceType};

pub struct RclValidator;
impl RclValidator {
    pub fn new() -> Self { Self }
    pub fn validate_meaning(&self, m: &RclMeaning) -> Result<(), ValidationError> { 
        if m.resonance_type() != ResonanceType::Meaning { return Err(ValidationError::WrongType); }
        Ok(()) 
    }
    pub fn validate_intent(&self, i: &RclIntent) -> Result<(), ValidationError> { 
        if i.resonance_type() != ResonanceType::Intent { return Err(ValidationError::WrongType); }
        Ok(()) 
    }
    pub fn validate_commitment(&self, c: &RclCommitment) -> Result<(), ValidationError> { 
        if c.resonance_type() != ResonanceType::Commitment { return Err(ValidationError::WrongType); }
        c.validate().map_err(|_| ValidationError::InvalidStructure)?;
        Ok(()) 
    }
}
impl Default for RclValidator { fn default() -> Self { Self::new() } }

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Wrong type")] WrongType,
    #[error("Invalid structure")] InvalidStructure,
}
