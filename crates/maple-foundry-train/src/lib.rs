//! MAPLE Foundry Train -- training pipeline orchestrator.
//!
//! Manages training jobs with configurable hyperparameters, status tracking,
//! and pipeline operations for submitting, monitoring, and cancelling jobs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum TrainingError {
    #[error("job not found: {0}")]
    JobNotFound(String),
    #[error("invalid state transition: cannot {operation} job in state {state:?}")]
    InvalidState { operation: String, state: TrainingStatus },
    #[error("configuration error: {0}")]
    ConfigError(String),
    #[error("training error: {0}")]
    Internal(String),
}

pub type TrainingResult<T> = Result<T, TrainingError>;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Status of a training job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingStatus {
    Queued,
    Preparing,
    Training,
    Evaluating,
    Complete,
    Failed,
    Cancelled,
}

/// Hyperparameters for training.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hyperparameters {
    pub learning_rate: f64,
    pub epochs: u32,
    pub batch_size: u32,
    pub warmup_steps: u32,
    pub weight_decay: f64,
    pub max_grad_norm: f64,
}

impl Default for Hyperparameters {
    fn default() -> Self {
        Self {
            learning_rate: 5e-5,
            epochs: 3,
            batch_size: 16,
            warmup_steps: 100,
            weight_decay: 0.01,
            max_grad_norm: 1.0,
        }
    }
}

/// Configuration for a training job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub base_model: String,
    pub dataset: String,
    pub hyperparameters: Hyperparameters,
    pub output_path: String,
    pub description: String,
}

impl TrainingConfig {
    pub fn new(
        base_model: impl Into<String>,
        dataset: impl Into<String>,
        output_path: impl Into<String>,
    ) -> Self {
        Self {
            base_model: base_model.into(),
            dataset: dataset.into(),
            hyperparameters: Hyperparameters::default(),
            output_path: output_path.into(),
            description: String::new(),
        }
    }
}

/// Metrics collected during training.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrainingMetrics {
    pub current_epoch: u32,
    pub current_step: u64,
    pub total_steps: u64,
    pub loss: f64,
    pub eval_loss: Option<f64>,
    pub accuracy: Option<f64>,
    pub learning_rate: f64,
}

/// A training job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingJob {
    pub id: String,
    pub config: TrainingConfig,
    pub status: TrainingStatus,
    pub metrics: TrainingMetrics,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Training Pipeline
// ---------------------------------------------------------------------------

/// Manages training jobs and their lifecycle.
pub struct TrainingPipeline {
    jobs: HashMap<String, TrainingJob>,
}

impl Default for TrainingPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl TrainingPipeline {
    pub fn new() -> Self {
        Self {
            jobs: HashMap::new(),
        }
    }

    /// Submit a new training job.
    pub fn submit(&mut self, config: TrainingConfig) -> TrainingResult<TrainingJob> {
        if config.base_model.is_empty() {
            return Err(TrainingError::ConfigError("base_model is required".into()));
        }
        let id = Uuid::new_v4().to_string();
        let job = TrainingJob {
            id: id.clone(),
            config,
            status: TrainingStatus::Queued,
            metrics: TrainingMetrics::default(),
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error: None,
        };
        self.jobs.insert(id, job.clone());
        Ok(job)
    }

    /// Get the status of a job.
    pub fn status(&self, id: &str) -> TrainingResult<&TrainingJob> {
        self.jobs
            .get(id)
            .ok_or_else(|| TrainingError::JobNotFound(id.to_string()))
    }

    /// Start a queued job (transition to Preparing then Training).
    pub fn start(&mut self, id: &str) -> TrainingResult<&TrainingJob> {
        let job = self
            .jobs
            .get_mut(id)
            .ok_or_else(|| TrainingError::JobNotFound(id.to_string()))?;
        if job.status != TrainingStatus::Queued {
            return Err(TrainingError::InvalidState {
                operation: "start".into(),
                state: job.status,
            });
        }
        job.status = TrainingStatus::Preparing;
        job.started_at = Some(Utc::now());
        job.status = TrainingStatus::Training;
        Ok(job)
    }

    /// Update training metrics.
    pub fn update_metrics(&mut self, id: &str, metrics: TrainingMetrics) -> TrainingResult<()> {
        let job = self
            .jobs
            .get_mut(id)
            .ok_or_else(|| TrainingError::JobNotFound(id.to_string()))?;
        job.metrics = metrics;
        Ok(())
    }

    /// Mark a job as complete.
    pub fn complete(&mut self, id: &str) -> TrainingResult<&TrainingJob> {
        let job = self
            .jobs
            .get_mut(id)
            .ok_or_else(|| TrainingError::JobNotFound(id.to_string()))?;
        if job.status != TrainingStatus::Training && job.status != TrainingStatus::Evaluating {
            return Err(TrainingError::InvalidState {
                operation: "complete".into(),
                state: job.status,
            });
        }
        job.status = TrainingStatus::Complete;
        job.completed_at = Some(Utc::now());
        Ok(job)
    }

    /// Cancel a job.
    pub fn cancel(&mut self, id: &str) -> TrainingResult<&TrainingJob> {
        let job = self
            .jobs
            .get_mut(id)
            .ok_or_else(|| TrainingError::JobNotFound(id.to_string()))?;
        if job.status == TrainingStatus::Complete || job.status == TrainingStatus::Failed {
            return Err(TrainingError::InvalidState {
                operation: "cancel".into(),
                state: job.status,
            });
        }
        job.status = TrainingStatus::Cancelled;
        job.completed_at = Some(Utc::now());
        Ok(job)
    }

    /// Mark a job as failed.
    pub fn fail(&mut self, id: &str, error: &str) -> TrainingResult<()> {
        let job = self
            .jobs
            .get_mut(id)
            .ok_or_else(|| TrainingError::JobNotFound(id.to_string()))?;
        job.status = TrainingStatus::Failed;
        job.error = Some(error.to_string());
        job.completed_at = Some(Utc::now());
        Ok(())
    }

    /// List all jobs, optionally filtered by status.
    pub fn list(&self, status_filter: Option<TrainingStatus>) -> Vec<&TrainingJob> {
        match status_filter {
            Some(status) => self.jobs.values().filter(|j| j.status == status).collect(),
            None => self.jobs.values().collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> TrainingConfig {
        TrainingConfig::new("llama-7b", "my-dataset", "/output/model")
    }

    #[test]
    fn test_submit_job() {
        let mut pipeline = TrainingPipeline::new();
        let job = pipeline.submit(make_config()).unwrap();
        assert_eq!(job.status, TrainingStatus::Queued);
    }

    #[test]
    fn test_start_job() {
        let mut pipeline = TrainingPipeline::new();
        let job = pipeline.submit(make_config()).unwrap();
        let started = pipeline.start(&job.id).unwrap();
        assert_eq!(started.status, TrainingStatus::Training);
        assert!(started.started_at.is_some());
    }

    #[test]
    fn test_complete_job() {
        let mut pipeline = TrainingPipeline::new();
        let job = pipeline.submit(make_config()).unwrap();
        pipeline.start(&job.id).unwrap();
        let completed = pipeline.complete(&job.id).unwrap();
        assert_eq!(completed.status, TrainingStatus::Complete);
    }

    #[test]
    fn test_cancel_job() {
        let mut pipeline = TrainingPipeline::new();
        let job = pipeline.submit(make_config()).unwrap();
        let cancelled = pipeline.cancel(&job.id).unwrap();
        assert_eq!(cancelled.status, TrainingStatus::Cancelled);
    }

    #[test]
    fn test_fail_job() {
        let mut pipeline = TrainingPipeline::new();
        let job = pipeline.submit(make_config()).unwrap();
        pipeline.start(&job.id).unwrap();
        pipeline.fail(&job.id, "OOM").unwrap();
        let status = pipeline.status(&job.id).unwrap();
        assert_eq!(status.status, TrainingStatus::Failed);
        assert_eq!(status.error.as_deref(), Some("OOM"));
    }

    #[test]
    fn test_invalid_start() {
        let mut pipeline = TrainingPipeline::new();
        let job = pipeline.submit(make_config()).unwrap();
        pipeline.start(&job.id).unwrap();
        // Cannot start again
        assert!(pipeline.start(&job.id).is_err());
    }

    #[test]
    fn test_cannot_cancel_completed() {
        let mut pipeline = TrainingPipeline::new();
        let job = pipeline.submit(make_config()).unwrap();
        pipeline.start(&job.id).unwrap();
        pipeline.complete(&job.id).unwrap();
        assert!(pipeline.cancel(&job.id).is_err());
    }

    #[test]
    fn test_list_jobs() {
        let mut pipeline = TrainingPipeline::new();
        pipeline.submit(make_config()).unwrap();
        let j2 = pipeline.submit(make_config()).unwrap();
        pipeline.start(&j2.id).unwrap();
        assert_eq!(pipeline.list(None).len(), 2);
        assert_eq!(pipeline.list(Some(TrainingStatus::Queued)).len(), 1);
        assert_eq!(pipeline.list(Some(TrainingStatus::Training)).len(), 1);
    }

    #[test]
    fn test_update_metrics() {
        let mut pipeline = TrainingPipeline::new();
        let job = pipeline.submit(make_config()).unwrap();
        pipeline.start(&job.id).unwrap();
        pipeline.update_metrics(&job.id, TrainingMetrics {
            current_epoch: 1,
            current_step: 100,
            total_steps: 1000,
            loss: 0.5,
            eval_loss: Some(0.6),
            accuracy: Some(0.85),
            learning_rate: 5e-5,
        }).unwrap();
        let status = pipeline.status(&job.id).unwrap();
        assert_eq!(status.metrics.current_epoch, 1);
    }

    #[test]
    fn test_hyperparameters_default() {
        let hp = Hyperparameters::default();
        assert_eq!(hp.epochs, 3);
        assert_eq!(hp.batch_size, 16);
    }

    #[test]
    fn test_empty_model_config() {
        let mut pipeline = TrainingPipeline::new();
        let config = TrainingConfig::new("", "dataset", "/out");
        assert!(pipeline.submit(config).is_err());
    }
}
