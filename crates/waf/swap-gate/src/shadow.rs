use crate::error::SwapError;
use async_trait::async_trait;
use maple_waf_context_graph::ContentHash;
use serde::{Deserialize, Serialize};

/// Result of shadow execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShadowResult {
    /// Whether shadow execution succeeded.
    pub success: bool,
    /// Behavioral comparison to current system.
    pub behavioral_match: bool,
    /// Performance comparison (ratio: new/old, <1.0 = faster).
    pub performance_ratio: f64,
    /// Summary of shadow run.
    pub summary: String,
    /// Hash of the artifact that was shadow-tested.
    pub artifact_hash: ContentHash,
}

/// Runs a proposed artifact in shadow mode (no side effects).
#[async_trait]
pub trait ShadowRunner: Send + Sync {
    async fn run_shadow(&self, artifact_hash: &ContentHash) -> Result<ShadowResult, SwapError>;
}

/// Simulated shadow runner for testing.
pub struct SimulatedShadowRunner {
    success: bool,
    behavioral_match: bool,
    performance_ratio: f64,
}

impl SimulatedShadowRunner {
    pub fn passing() -> Self {
        Self {
            success: true,
            behavioral_match: true,
            performance_ratio: 0.9,
        }
    }

    pub fn failing() -> Self {
        Self {
            success: false,
            behavioral_match: false,
            performance_ratio: 1.5,
        }
    }

    pub fn with_performance_ratio(mut self, ratio: f64) -> Self {
        self.performance_ratio = ratio;
        self
    }
}

#[async_trait]
impl ShadowRunner for SimulatedShadowRunner {
    async fn run_shadow(&self, artifact_hash: &ContentHash) -> Result<ShadowResult, SwapError> {
        if !self.success {
            return Err(SwapError::ShadowFailed("shadow execution crashed".into()));
        }
        Ok(ShadowResult {
            success: true,
            behavioral_match: self.behavioral_match,
            performance_ratio: self.performance_ratio,
            summary: format!(
                "Shadow OK: behavioral_match={}, perf_ratio={:.2}",
                self.behavioral_match, self.performance_ratio
            ),
            artifact_hash: artifact_hash.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn passing_shadow() {
        let runner = SimulatedShadowRunner::passing();
        let result = runner
            .run_shadow(&ContentHash::hash(b"artifact"))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.behavioral_match);
        assert!(result.performance_ratio < 1.0);
    }

    #[tokio::test]
    async fn failing_shadow() {
        let runner = SimulatedShadowRunner::failing();
        let result = runner.run_shadow(&ContentHash::hash(b"artifact")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn custom_performance() {
        let runner = SimulatedShadowRunner::passing().with_performance_ratio(0.5);
        let result = runner
            .run_shadow(&ContentHash::hash(b"fast"))
            .await
            .unwrap();
        assert_eq!(result.performance_ratio, 0.5);
    }

    #[test]
    fn shadow_result_serde() {
        let r = ShadowResult {
            success: true,
            behavioral_match: true,
            performance_ratio: 0.8,
            summary: "ok".into(),
            artifact_hash: ContentHash::hash(b"a"),
        };
        let json = serde_json::to_string(&r).unwrap();
        let restored: ShadowResult = serde_json::from_str(&json).unwrap();
        assert!(restored.success);
    }
}
