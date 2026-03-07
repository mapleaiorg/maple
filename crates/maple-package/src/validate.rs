use crate::manifest::*;

/// Validate a MapleManifest for correctness and completeness.
/// Returns a list of errors (hard failures) and warnings (advisory).
pub fn validate_manifest(manifest: &MapleManifest) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // 1. api_version must be "maple.ai/v1"
    if manifest.api_version != "maple.ai/v1" {
        errors.push(ValidationError::UnsupportedApiVersion(
            manifest.api_version.clone(),
        ));
    }

    // 2. Name must be valid (lowercase, alphanumeric + hyphens, 2-4 segments)
    if let Err(e) = PackageName::parse(&manifest.name.to_qualified()) {
        errors.push(ValidationError::InvalidName(e.to_string()));
    }

    // 3. Kind-specific validation
    match manifest.kind {
        PackageKind::AgentPackage => {
            validate_agent_package(manifest, &mut errors, &mut warnings);
        }
        PackageKind::SkillPackage => {
            validate_skill_package(manifest, &mut errors, &mut warnings);
        }
        PackageKind::ContractBundle => {
            validate_contract_bundle(manifest, &mut errors, &mut warnings);
        }
        PackageKind::ModelPackage => {
            validate_model_package(manifest, &mut errors, &mut warnings);
        }
        PackageKind::EvalSuite => {
            validate_eval_suite(manifest, &mut errors, &mut warnings);
        }
        PackageKind::KnowledgePack => {
            validate_knowledge_pack(manifest, &mut errors, &mut warnings);
        }
        PackageKind::PolicyPack => {
            validate_policy_pack(manifest, &mut errors, &mut warnings);
        }
        PackageKind::EvidencePack => {
            validate_evidence_pack(manifest, &mut errors, &mut warnings);
        }
        PackageKind::UiModule => {
            validate_ui_module(manifest, &mut errors, &mut warnings);
        }
    }

    // 4. Policy: if deny_by_default is true, must have at least one allow rule
    if let Some(ref policy) = manifest.policy {
        if policy.deny_by_default && policy.allow.is_empty() {
            warnings.push(ValidationWarning::DenyAllWithNoAllowRules);
        }
    }

    // 5. Contracts: check for version constraint validity
    for contract in &manifest.contracts {
        if semver::VersionReq::parse(&contract.version).is_err() {
            errors.push(ValidationError::InvalidVersionConstraint {
                reference: contract.reference.clone(),
                constraint: contract.version.clone(),
            });
        }
    }

    // 6. Skills: check for version constraint validity
    for skill in &manifest.skills {
        if semver::VersionReq::parse(&skill.version).is_err() {
            errors.push(ValidationError::InvalidVersionConstraint {
                reference: skill.reference.clone(),
                constraint: skill.version.clone(),
            });
        }
    }

    ValidationResult { errors, warnings }
}

fn validate_agent_package(
    manifest: &MapleManifest,
    _errors: &mut Vec<ValidationError>,
    warnings: &mut Vec<ValidationWarning>,
) {
    // Agent should have model requirements
    if manifest.models.is_none() {
        warnings.push(ValidationWarning::AgentWithoutModelRequirement);
    }
    // Agent should have memory config
    if manifest.memory.is_none() {
        warnings.push(ValidationWarning::AgentWithoutMemoryConfig);
    }
    // Agent should have observability enabled
    if manifest.observability.is_none() {
        warnings.push(ValidationWarning::AgentWithoutObservability);
    }
    // Agent should have at least one contract
    if manifest.contracts.is_empty() {
        warnings.push(ValidationWarning::AgentWithoutContracts);
    }
    // Agent should have eval baselines
    if manifest.eval.is_none() {
        warnings.push(ValidationWarning::AgentWithoutEvals);
    }
}

fn validate_skill_package(
    manifest: &MapleManifest,
    errors: &mut Vec<ValidationError>,
    _warnings: &mut Vec<ValidationWarning>,
) {
    // Skill must not have model requirements (models belong to agents)
    if manifest.models.is_some() {
        errors.push(ValidationError::SkillWithModelRequirement);
    }
}

fn validate_contract_bundle(
    manifest: &MapleManifest,
    _errors: &mut Vec<ValidationError>,
    warnings: &mut Vec<ValidationWarning>,
) {
    // Contract bundle should have enforcement level on all entries
    for contract in &manifest.contracts {
        if contract.enforcement == EnforcementLevel::AuditOnly {
            warnings.push(ValidationWarning::ContractBundleWithAuditOnly(
                contract.reference.clone(),
            ));
        }
    }
}

fn validate_model_package(
    manifest: &MapleManifest,
    errors: &mut Vec<ValidationError>,
    _warnings: &mut Vec<ValidationWarning>,
) {
    // Model package must have runtime constraints with platform info
    if manifest.runtime.is_none() {
        errors.push(ValidationError::ModelWithoutRuntimeConstraints);
    }
}

fn validate_knowledge_pack(
    manifest: &MapleManifest,
    _errors: &mut Vec<ValidationError>,
    warnings: &mut Vec<ValidationWarning>,
) {
    // Knowledge pack should have metadata keywords for discoverability
    if manifest.metadata.keywords.is_empty() {
        warnings.push(ValidationWarning::KnowledgePackWithoutKeywords);
    }
}

fn validate_policy_pack(
    manifest: &MapleManifest,
    errors: &mut Vec<ValidationError>,
    _warnings: &mut Vec<ValidationWarning>,
) {
    // Policy pack must have policy config
    if manifest.policy.is_none() {
        errors.push(ValidationError::PolicyPackWithoutPolicyConfig);
    }
}

fn validate_evidence_pack(
    _manifest: &MapleManifest,
    _errors: &mut Vec<ValidationError>,
    _warnings: &mut Vec<ValidationWarning>,
) {
    // Evidence packs are flexible — minimal validation
}

fn validate_eval_suite(
    _manifest: &MapleManifest,
    _errors: &mut Vec<ValidationError>,
    _warnings: &mut Vec<ValidationWarning>,
) {
    // Eval suites are flexible — minimal validation
}

fn validate_ui_module(
    _manifest: &MapleManifest,
    _errors: &mut Vec<ValidationError>,
    _warnings: &mut Vec<ValidationWarning>,
) {
    // UI modules are optional/future
}

#[derive(Debug)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Unsupported API version: {0}")]
    UnsupportedApiVersion(String),
    #[error("Invalid package name: {0}")]
    InvalidName(String),
    #[error("Invalid version constraint for {reference}: {constraint}")]
    InvalidVersionConstraint {
        reference: String,
        constraint: String,
    },
    #[error("Skill packages must not declare model requirements")]
    SkillWithModelRequirement,
    #[error("Model packages must declare runtime constraints")]
    ModelWithoutRuntimeConstraints,
    #[error("Policy packs must include policy configuration")]
    PolicyPackWithoutPolicyConfig,
}

#[derive(Debug)]
pub enum ValidationWarning {
    DenyAllWithNoAllowRules,
    AgentWithoutModelRequirement,
    AgentWithoutMemoryConfig,
    AgentWithoutObservability,
    AgentWithoutContracts,
    AgentWithoutEvals,
    ContractBundleWithAuditOnly(String),
    KnowledgePackWithoutKeywords,
}

impl std::fmt::Display for ValidationWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DenyAllWithNoAllowRules => {
                write!(f, "deny_by_default is true but no allow rules are defined")
            }
            Self::AgentWithoutModelRequirement => {
                write!(f, "agent package has no model requirements")
            }
            Self::AgentWithoutMemoryConfig => {
                write!(f, "agent package has no memory configuration")
            }
            Self::AgentWithoutObservability => {
                write!(f, "agent package has no observability configuration")
            }
            Self::AgentWithoutContracts => write!(f, "agent package has no contracts"),
            Self::AgentWithoutEvals => write!(f, "agent package has no eval baselines"),
            Self::ContractBundleWithAuditOnly(ref r) => {
                write!(f, "contract '{}' has audit-only enforcement", r)
            }
            Self::KnowledgePackWithoutKeywords => {
                write!(f, "knowledge pack has no keywords for discoverability")
            }
        }
    }
}
