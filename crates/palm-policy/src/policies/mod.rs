//! Platform-specific policy implementations
//!
//! Each platform has unique invariants that must be enforced:
//!
//! - **Mapleverse**: Throughput-first, allows high-velocity operations
//! - **Finalverse**: Safety-first, requires human approval for critical operations
//! - **IBank**: Accountability-first, requires comprehensive audit trails

pub mod base;
pub mod finalverse;
pub mod ibank;
pub mod mapleverse;

pub use base::BaseInvariantPolicy;
pub use finalverse::FinalverseSafetyPolicy;
pub use ibank::IBankAccountabilityPolicy;
pub use mapleverse::MapleverseThroughputPolicy;

use crate::gate::{ComposedPolicyGate, EvaluationMode, PolicyGate};
use palm_types::PlatformProfile;
use std::sync::Arc;

/// Create the default policy gate for a platform
pub fn create_platform_policy(platform: PlatformProfile) -> Arc<dyn PolicyGate> {
    match platform {
        PlatformProfile::Mapleverse => {
            Arc::new(
                ComposedPolicyGate::new("mapleverse-policy", "Mapleverse Policy Stack")
                    .add_gate(Arc::new(BaseInvariantPolicy::new()))
                    .add_gate(Arc::new(MapleverseThroughputPolicy::new()))
                    .with_evaluation_mode(EvaluationMode::AllMustAllow),
            )
        }
        PlatformProfile::Finalverse => {
            Arc::new(
                ComposedPolicyGate::new("finalverse-policy", "Finalverse Policy Stack")
                    .add_gate(Arc::new(BaseInvariantPolicy::new()))
                    .add_gate(Arc::new(FinalverseSafetyPolicy::new()))
                    .with_evaluation_mode(EvaluationMode::MostRestrictive),
            )
        }
        PlatformProfile::IBank => {
            Arc::new(
                ComposedPolicyGate::new("ibank-policy", "IBank Policy Stack")
                    .add_gate(Arc::new(BaseInvariantPolicy::new()))
                    .add_gate(Arc::new(IBankAccountabilityPolicy::new()))
                    .with_evaluation_mode(EvaluationMode::AllMustAllow),
            )
        }
        PlatformProfile::Development => {
            // Development mode uses only base invariants
            Arc::new(BaseInvariantPolicy::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::PolicyEvaluationContext;
    use palm_types::policy::PalmOperation;

    #[tokio::test]
    async fn test_create_mapleverse_policy() {
        let policy = create_platform_policy(PlatformProfile::Mapleverse);
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Mapleverse);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_create_development_policy() {
        let policy = create_platform_policy(PlatformProfile::Development);
        let ctx = PolicyEvaluationContext::new("user-1", PlatformProfile::Development);
        let op = PalmOperation::CreateDeployment {
            spec_id: "test-spec".into(),
        };

        let decision = policy.evaluate(&op, &ctx).await.unwrap();
        assert!(decision.is_allowed());
    }
}
