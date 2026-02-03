//! UAL Compiler - compile UAL statements into formal artifacts.

#![deny(unsafe_code)]

use chrono::{DateTime, Utc};
use palm_types::policy::PalmOperation;
use rcf_commitment::{IntendedOutcome, RcfCommitment, Reversibility, Target};
use rcf_types::{EffectDomain, IdentityRef, ScopeConstraint, TemporalValidity};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ual_types::{CommitStatement, OperationStatement, ReversibilitySpec, UalStatement};

#[derive(Debug, Error)]
pub enum UalCompileError {
    #[error("Invalid timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("Unsupported statement: {0}")]
    Unsupported(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum UalCompiled {
    Commitment(RcfCommitment),
    Operation(PalmOperation),
}

pub fn compile(statements: &[UalStatement]) -> Result<Vec<UalCompiled>, UalCompileError> {
    let mut compiled = Vec::new();
    for stmt in statements {
        compiled.push(match stmt {
            UalStatement::Commit(c) => UalCompiled::Commitment(compile_commit(c)?),
            UalStatement::Operation(op) => UalCompiled::Operation(compile_operation(op)),
        });
    }
    Ok(compiled)
}

fn compile_commit(stmt: &CommitStatement) -> Result<RcfCommitment, UalCompileError> {
    let domain = parse_domain(&stmt.domain);
    let principal = IdentityRef::new(stmt.principal.clone());

    let mut builder = RcfCommitment::builder(principal, domain)
        .with_outcome(IntendedOutcome::new(stmt.outcome.clone()));

    let scope = match stmt.scope.as_deref() {
        Some(scope) if scope.eq_ignore_ascii_case("GLOBAL") => ScopeConstraint::global(),
        Some(scope) => ScopeConstraint::new(vec![scope.to_string()], vec!["*".to_string()]),
        None => ScopeConstraint::default(),
    };
    builder = builder.with_scope(scope);

    for target in &stmt.targets {
        builder = builder.with_target(Target::resource(target.clone()));
    }

    if let Some(rev) = stmt.reversibility {
        builder = builder.with_reversibility(match rev {
            ReversibilitySpec::Reversible => Reversibility::Reversible,
            ReversibilitySpec::Irreversible => Reversibility::Irreversible,
        });
    }

    if let Some(validity) = parse_validity(&stmt.valid_from, &stmt.valid_until)? {
        builder = builder.with_validity(validity);
    }

    for tag in &stmt.tags {
        builder = builder.with_policy_tag(tag.clone());
    }

    builder
        .build()
        .map_err(|_| UalCompileError::Unsupported("Invalid commitment".to_string()))
}

fn compile_operation(stmt: &OperationStatement) -> PalmOperation {
    match stmt {
        OperationStatement::CreateSpec { spec_id, version } => {
            PalmOperation::CreateSpec {
                spec_id: with_version(spec_id, version.as_deref()),
            }
        }
        OperationStatement::UpdateSpec { spec_id, version } => {
            PalmOperation::UpdateSpec {
                spec_id: with_version(spec_id, version.as_deref()),
            }
        }
        OperationStatement::DeprecateSpec { spec_id } => {
            PalmOperation::DeprecateSpec {
                spec_id: spec_id.clone(),
            }
        }
        OperationStatement::CreateDeployment { spec_id, replicas: _ } => {
            PalmOperation::CreateDeployment {
                spec_id: spec_id.clone(),
            }
        }
        OperationStatement::UpdateDeployment { deployment_id } => {
            PalmOperation::UpdateDeployment {
                deployment_id: deployment_id.clone(),
            }
        }
        OperationStatement::ScaleDeployment { deployment_id, target_replicas } => {
            PalmOperation::ScaleDeployment {
                deployment_id: deployment_id.clone(),
                target_replicas: *target_replicas,
            }
        }
        OperationStatement::DeleteDeployment { deployment_id } => {
            PalmOperation::DeleteDeployment {
                deployment_id: deployment_id.clone(),
            }
        }
        OperationStatement::RollbackDeployment { deployment_id } => {
            PalmOperation::RollbackDeployment {
                deployment_id: deployment_id.clone(),
            }
        }
        OperationStatement::PauseDeployment { deployment_id } => {
            PalmOperation::PauseDeployment {
                deployment_id: deployment_id.clone(),
            }
        }
        OperationStatement::ResumeDeployment { deployment_id } => {
            PalmOperation::ResumeDeployment {
                deployment_id: deployment_id.clone(),
            }
        }
        OperationStatement::RestartInstance { instance_id } => {
            PalmOperation::RestartInstance {
                instance_id: instance_id.clone(),
            }
        }
        OperationStatement::TerminateInstance { instance_id } => {
            PalmOperation::TerminateInstance {
                instance_id: instance_id.clone(),
            }
        }
        OperationStatement::MigrateInstance { instance_id } => {
            PalmOperation::MigrateInstance {
                instance_id: instance_id.clone(),
            }
        }
        OperationStatement::DrainInstance { instance_id } => {
            PalmOperation::DrainInstance {
                instance_id: instance_id.clone(),
            }
        }
        OperationStatement::CreateCheckpoint { instance_id } => {
            PalmOperation::CreateCheckpoint {
                instance_id: instance_id.clone(),
            }
        }
        OperationStatement::RestoreCheckpoint { instance_id } => {
            PalmOperation::RestoreCheckpoint {
                instance_id: instance_id.clone(),
            }
        }
        OperationStatement::DeleteCheckpoint { snapshot_id } => {
            PalmOperation::DeleteCheckpoint {
                snapshot_id: snapshot_id.clone(),
            }
        }
        OperationStatement::HealthCheck { instance_id } => {
            PalmOperation::HealthCheck {
                instance_id: instance_id.clone(),
            }
        }
        OperationStatement::ForceRecovery { instance_id } => {
            PalmOperation::ForceRecovery {
                instance_id: instance_id.clone(),
            }
        }
        OperationStatement::ConfigurePolicy { policy_name } => {
            PalmOperation::ConfigurePolicy {
                policy_name: policy_name.clone(),
            }
        }
        OperationStatement::ViewAuditLog { filter } => {
            PalmOperation::ViewAuditLog {
                filter: filter.clone(),
            }
        }
    }
}

fn parse_domain(domain: &str) -> EffectDomain {
    match domain.to_uppercase().as_str() {
        "COMMUNICATION" => EffectDomain::Communication,
        "FINANCE" => EffectDomain::Finance,
        "INFRASTRUCTURE" => EffectDomain::Infrastructure,
        "DATA" => EffectDomain::Data,
        "GOVERNANCE" => EffectDomain::Governance,
        "PHYSICAL" => EffectDomain::Physical,
        "COMPUTATION" => EffectDomain::Computation,
        other => EffectDomain::Custom(other.to_string()),
    }
}

fn parse_validity(
    valid_from: &Option<String>,
    valid_until: &Option<String>,
) -> Result<Option<TemporalValidity>, UalCompileError> {
    if valid_from.is_none() && valid_until.is_none() {
        return Ok(None);
    }
    let from = match valid_from {
        Some(v) => Some(parse_datetime(v)?),
        None => None,
    };
    let until = match valid_until {
        Some(v) => Some(parse_datetime(v)?),
        None => None,
    };
    Ok(Some(TemporalValidity { valid_from: from, valid_until: until }))
}

fn parse_datetime(value: &str) -> Result<DateTime<Utc>, UalCompileError> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| UalCompileError::InvalidTimestamp(value.to_string()))?;
    Ok(parsed.with_timezone(&Utc))
}

fn with_version(spec_id: &str, version: Option<&str>) -> String {
    match version {
        Some(v) => format!("{}:{}", spec_id, v),
        None => spec_id.to_string(),
    }
}
