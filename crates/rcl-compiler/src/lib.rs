#![deny(unsafe_code)]
use rcl_commitment::RclCommitment;
use rcl_validator::RclValidator;
use serde::{Deserialize, Serialize};

pub struct RclCompiler { validator: RclValidator }
impl RclCompiler {
    pub fn new() -> Self { Self { validator: RclValidator::new() } }
    pub fn compile(&self, c: &RclCommitment) -> Result<ExecutionPlan, CompileError> {
        self.validator.validate_commitment(c).map_err(|_| CompileError::ValidationFailed)?;
        Ok(ExecutionPlan { commitment_id: c.commitment_id.clone(), steps: vec![] })
    }
}
impl Default for RclCompiler { fn default() -> Self { Self::new() } }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionPlan { pub commitment_id: rcl_commitment::CommitmentId, pub steps: Vec<ExecutionStep> }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionStep { pub step_id: String, pub operation: String }

#[derive(Debug, thiserror::Error)]
pub enum CompileError { #[error("Validation failed")] ValidationFailed }
