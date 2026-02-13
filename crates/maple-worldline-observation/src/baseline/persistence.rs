//! Baseline persistence — save and load baselines across restarts.
//!
//! Provides the `BaselinePersistence` trait and a `JsonFileBaseline`
//! implementation that stores baselines as a JSON file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::ObservationResult;

use super::types::{MetricBaseline, MetricId};

/// Trait for baseline persistence across restarts.
///
/// Baselines must survive restarts to avoid expensive cold-start periods.
/// The EWMA state (mean, variance, std_dev) is the critical state; percentile
/// buffers are repopulated from incoming observations.
pub trait BaselinePersistence {
    /// Save all baselines to persistent storage.
    fn save(&self, baselines: &HashMap<MetricId, MetricBaseline>) -> ObservationResult<()>;

    /// Load baselines from persistent storage.
    ///
    /// Returns an empty map if no persisted state exists.
    fn load(&self) -> ObservationResult<HashMap<MetricId, MetricBaseline>>;

    /// Incrementally save a single updated baseline.
    ///
    /// Default implementation: load all, merge, save all. Implementations
    /// may override with more efficient strategies.
    fn save_incremental(
        &self,
        metric_id: &MetricId,
        baseline: &MetricBaseline,
    ) -> ObservationResult<()> {
        let mut all = self.load()?;
        all.insert(metric_id.clone(), baseline.clone());
        self.save(&all)
    }
}

/// JSON-file based baseline persistence.
///
/// Stores baselines as a single JSON file. Writes are atomic (write to
/// `.tmp`, then rename) to prevent corruption from interrupted writes.
pub struct JsonFileBaseline {
    path: PathBuf,
}

impl JsonFileBaseline {
    /// Create a new JSON file persistence at the given path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Get the file path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl BaselinePersistence for JsonFileBaseline {
    fn save(&self, baselines: &HashMap<MetricId, MetricBaseline>) -> ObservationResult<()> {
        let json = serde_json::to_string_pretty(baselines).map_err(|e| {
            crate::error::ObservationError::PersistenceError(format!(
                "serialization failed: {}",
                e
            ))
        })?;

        // Atomic write: write to .tmp then rename
        let tmp_path = self.path.with_extension("tmp");
        std::fs::write(&tmp_path, json)?;
        std::fs::rename(&tmp_path, &self.path)?;

        Ok(())
    }

    fn load(&self) -> ObservationResult<HashMap<MetricId, MetricBaseline>> {
        if !self.path.exists() {
            return Ok(HashMap::new());
        }

        let contents = std::fs::read_to_string(&self.path)?;
        let baselines: HashMap<MetricId, MetricBaseline> =
            serde_json::from_str(&contents).map_err(|e| {
                crate::error::ObservationError::PersistenceError(format!(
                    "deserialization failed: {}",
                    e
                ))
            })?;

        Ok(baselines)
    }
}

/// In-memory baseline persistence (for testing).
pub struct InMemoryBaseline {
    data: std::sync::Mutex<HashMap<MetricId, MetricBaseline>>,
}

impl InMemoryBaseline {
    /// Create a new in-memory persistence store.
    pub fn new() -> Self {
        Self {
            data: std::sync::Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryBaseline {
    fn default() -> Self {
        Self::new()
    }
}

impl BaselinePersistence for InMemoryBaseline {
    fn save(&self, baselines: &HashMap<MetricId, MetricBaseline>) -> ObservationResult<()> {
        let mut data = self.data.lock().map_err(|_| {
            crate::error::ObservationError::LockError
        })?;
        *data = baselines.clone();
        Ok(())
    }

    fn load(&self) -> ObservationResult<HashMap<MetricId, MetricBaseline>> {
        let data = self.data.lock().map_err(|_| {
            crate::error::ObservationError::LockError
        })?;
        Ok(data.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_baseline(name: &str, mean: f64) -> (MetricId, MetricBaseline) {
        let mid = MetricId::new("test", name);
        let mut b = MetricBaseline::new(mid.clone());
        b.mean = mean;
        b.sample_count = 100;
        (mid, b)
    }

    #[test]
    fn json_save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("baseline_test_{}", uuid::Uuid::new_v4()));
        let path = dir.join("baselines.json");
        std::fs::create_dir_all(&dir).unwrap();

        let store = JsonFileBaseline::new(&path);
        let mut baselines = HashMap::new();
        let (mid1, b1) = make_baseline("latency", 5.0);
        let (mid2, b2) = make_baseline("error_rate", 0.01);
        baselines.insert(mid1.clone(), b1);
        baselines.insert(mid2.clone(), b2);

        store.save(&baselines).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(loaded.len(), 2);
        assert!((loaded[&mid1].mean - 5.0).abs() < f64::EPSILON);
        assert!((loaded[&mid2].mean - 0.01).abs() < f64::EPSILON);

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn json_load_nonexistent_returns_empty() {
        let store = JsonFileBaseline::new("/tmp/nonexistent_baseline_test_12345.json");
        let loaded = store.load().unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn json_save_incremental() {
        let dir = std::env::temp_dir().join(format!("baseline_incr_{}", uuid::Uuid::new_v4()));
        let path = dir.join("baselines.json");
        std::fs::create_dir_all(&dir).unwrap();

        let store = JsonFileBaseline::new(&path);

        // Save initial
        let mut baselines = HashMap::new();
        let (mid1, b1) = make_baseline("latency", 5.0);
        baselines.insert(mid1.clone(), b1);
        store.save(&baselines).unwrap();

        // Incremental save of a new metric
        let (mid2, b2) = make_baseline("throughput", 1000.0);
        store.save_incremental(&mid2, &b2).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded.len(), 2);
        assert!((loaded[&mid2].mean - 1000.0).abs() < f64::EPSILON);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn in_memory_persistence() {
        let store = InMemoryBaseline::new();

        let mut baselines = HashMap::new();
        let (mid, b) = make_baseline("test", 42.0);
        baselines.insert(mid.clone(), b);

        store.save(&baselines).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.len(), 1);
        assert!((loaded[&mid].mean - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn persistence_trait_object() {
        let store: Box<dyn BaselinePersistence> = Box::new(InMemoryBaseline::new());
        let baselines = HashMap::new();
        store.save(&baselines).unwrap();
        let loaded = store.load().unwrap();
        assert!(loaded.is_empty());
    }
}
