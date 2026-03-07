#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    #[error("Invalid package name '{name}': {reason}")]
    InvalidPackageName { name: String, reason: String },

    #[error("Invalid package reference: {0}")]
    InvalidReference(String),

    #[error("Unsupported API version: {0}")]
    UnsupportedApiVersion(String),

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

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
