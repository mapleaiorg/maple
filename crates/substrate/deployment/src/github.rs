//! GitHub integration — trait and simulated implementation.
//!
//! The `GitHubIntegration` trait abstracts Git/GitHub operations for the
//! deployment pipeline: branch creation, file commits, PR creation,
//! merging, and revert operations. The `SimulatedGitHub` produces
//! deterministic fake results for testing.

use crate::error::DeploymentResult;

// ── Git Operation Result ───────────────────────────────────────────────

/// Result of a single Git/GitHub operation.
#[derive(Clone, Debug)]
pub struct GitOperationResult {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Name of the operation (e.g., "create_branch", "commit_files").
    pub operation: String,
    /// Output/log message.
    pub output: String,
    /// Commit SHA (if applicable).
    pub commit_sha: Option<String>,
    /// PR URL (if applicable).
    pub pr_url: Option<String>,
}

// ── GitHubIntegration Trait ────────────────────────────────────────────

/// Trait for Git/GitHub integration in the deployment pipeline.
///
/// Real implementations would use the `gh` CLI or GitHub API.
/// Operations are grouped into the typical deployment flow:
/// create_branch → commit_files → push_branch → create_pr → merge_pr
pub trait GitHubIntegration: Send + Sync {
    /// Create a new branch for the deployment.
    fn create_branch(&self, branch_name: &str, base: &str) -> DeploymentResult<GitOperationResult>;

    /// Commit generated files to the branch.
    fn commit_files(
        &self,
        branch_name: &str,
        files: &[String],
        message: &str,
    ) -> DeploymentResult<GitOperationResult>;

    /// Push the branch to remote.
    fn push_branch(&self, branch_name: &str) -> DeploymentResult<GitOperationResult>;

    /// Create a pull request.
    fn create_pr(
        &self,
        branch_name: &str,
        title: &str,
        body: &str,
    ) -> DeploymentResult<GitOperationResult>;

    /// Merge a pull request.
    fn merge_pr(&self, pr_url: &str) -> DeploymentResult<GitOperationResult>;

    /// Revert a commit (for rollback).
    fn revert_commit(&self, commit_sha: &str) -> DeploymentResult<GitOperationResult>;

    /// Name of this integration for logging.
    fn name(&self) -> &str;
}

// ── Simulated GitHub ───────────────────────────────────────────────────

/// A simulated GitHub integration for testing.
///
/// Produces deterministic fake results with predictable commit SHAs
/// and PR URLs.
pub struct SimulatedGitHub {
    should_succeed: bool,
}

impl SimulatedGitHub {
    /// Create a succeeding GitHub integration.
    pub fn succeeding() -> Self {
        Self {
            should_succeed: true,
        }
    }

    /// Create a failing GitHub integration.
    pub fn failing() -> Self {
        Self {
            should_succeed: false,
        }
    }

    fn make_result(&self, operation: &str) -> GitOperationResult {
        GitOperationResult {
            success: self.should_succeed,
            operation: operation.into(),
            output: if self.should_succeed {
                format!("Simulated {} succeeded", operation)
            } else {
                format!("Simulated {} failed", operation)
            },
            commit_sha: None,
            pr_url: None,
        }
    }
}

impl GitHubIntegration for SimulatedGitHub {
    fn create_branch(
        &self,
        branch_name: &str,
        _base: &str,
    ) -> DeploymentResult<GitOperationResult> {
        let mut result = self.make_result("create_branch");
        result.output = format!("Created branch: {}", branch_name);
        Ok(result)
    }

    fn commit_files(
        &self,
        _branch_name: &str,
        files: &[String],
        message: &str,
    ) -> DeploymentResult<GitOperationResult> {
        let mut result = self.make_result("commit_files");
        if self.should_succeed {
            result.commit_sha = Some("abc1234def5678".into());
            result.output = format!("Committed {} files: {}", files.len(), message);
        }
        Ok(result)
    }

    fn push_branch(&self, branch_name: &str) -> DeploymentResult<GitOperationResult> {
        let mut result = self.make_result("push_branch");
        result.output = format!("Pushed branch: {}", branch_name);
        Ok(result)
    }

    fn create_pr(
        &self,
        _branch_name: &str,
        title: &str,
        _body: &str,
    ) -> DeploymentResult<GitOperationResult> {
        let mut result = self.make_result("create_pr");
        if self.should_succeed {
            result.pr_url = Some("https://github.com/org/repo/pull/42".into());
            result.output = format!("Created PR: {}", title);
        }
        Ok(result)
    }

    fn merge_pr(&self, pr_url: &str) -> DeploymentResult<GitOperationResult> {
        let mut result = self.make_result("merge_pr");
        if self.should_succeed {
            result.commit_sha = Some("merge1234abc5678".into());
            result.output = format!("Merged PR: {}", pr_url);
        }
        Ok(result)
    }

    fn revert_commit(&self, commit_sha: &str) -> DeploymentResult<GitOperationResult> {
        let mut result = self.make_result("revert_commit");
        if self.should_succeed {
            result.commit_sha = Some("revert1234abc5678".into());
            result.output = format!("Reverted commit: {}", commit_sha);
        }
        Ok(result)
    }

    fn name(&self) -> &str {
        "simulated-github"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulated_github_create_branch() {
        let gh = SimulatedGitHub::succeeding();
        let result = gh.create_branch("deploy/test", "main").unwrap();
        assert!(result.success);
        assert!(result.output.contains("deploy/test"));
    }

    #[test]
    fn simulated_github_commit_files() {
        let gh = SimulatedGitHub::succeeding();
        let files = vec!["src/config.rs".into()];
        let result = gh
            .commit_files("deploy/test", &files, "deploy changes")
            .unwrap();
        assert!(result.success);
        assert!(result.commit_sha.is_some());
    }

    #[test]
    fn simulated_github_create_pr() {
        let gh = SimulatedGitHub::succeeding();
        let result = gh
            .create_pr("deploy/test", "Deploy changes", "body")
            .unwrap();
        assert!(result.success);
        assert!(result.pr_url.is_some());
        assert!(result.pr_url.unwrap().contains("github.com"));
    }

    #[test]
    fn simulated_github_failing() {
        let gh = SimulatedGitHub::failing();
        let result = gh.create_branch("deploy/test", "main").unwrap();
        assert!(!result.success);
    }

    #[test]
    fn simulated_github_name() {
        let gh = SimulatedGitHub::succeeding();
        assert_eq!(gh.name(), "simulated-github");
    }
}
