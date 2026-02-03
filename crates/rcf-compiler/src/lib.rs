#![deny(unsafe_code)]
use rcf_commitment::RcfCommitment;
use rcf_validator::RcfValidator;
use serde::{Deserialize, Serialize};

pub struct RcfCompiler { validator: RcfValidator }
impl RcfCompiler {
    pub fn new() -> Self { Self { validator: RcfValidator::new() } }
    pub fn compile(&self, c: &RcfCommitment) -> Result<ExecutionPlan, CompileError> {
        self.validator.validate_commitment(c).map_err(|_| CompileError::ValidationFailed)?;
        Ok(ExecutionPlan { commitment_id: c.commitment_id.clone(), steps: vec![] })
    }
}
impl Default for RcfCompiler { fn default() -> Self { Self::new() } }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionPlan { pub commitment_id: rcf_commitment::CommitmentId, pub steps: Vec<ExecutionStep> }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionStep { pub step_id: String, pub operation: String }

#[derive(Debug, thiserror::Error)]
pub enum CompileError { #[error("Validation failed")] ValidationFailed }
