//! Audit sinks for storing audit entries

use super::entry::{AuditEntry, PartialAuditEntry};
use super::integrity::IntegrityChain;
use crate::error::{ObservabilityError, Result};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Trait for audit sinks
#[async_trait]
pub trait AuditSink: Send + Sync {
    /// Write an audit entry
    async fn write(&self, entry: PartialAuditEntry) -> Result<AuditEntry>;

    /// Flush any buffered entries
    async fn flush(&self) -> Result<()>;

    /// Get the entry count
    async fn entry_count(&self) -> Result<u64>;
}

/// In-memory audit sink for testing
pub struct MemoryAuditSink {
    entries: RwLock<Vec<AuditEntry>>,
    chain: RwLock<IntegrityChain>,
}

impl MemoryAuditSink {
    /// Create a new memory sink
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            chain: RwLock::new(IntegrityChain::new()),
        }
    }

    /// Get all entries
    pub fn entries(&self) -> Vec<AuditEntry> {
        self.entries.read().clone()
    }

    /// Clear all entries
    pub fn clear(&self) {
        let mut entries = self.entries.write();
        let mut chain = self.chain.write();
        entries.clear();
        *chain = IntegrityChain::new();
    }
}

impl Default for MemoryAuditSink {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuditSink for MemoryAuditSink {
    async fn write(&self, partial: PartialAuditEntry) -> Result<AuditEntry> {
        let mut chain = self.chain.write();
        let entry = partial.finalize(chain.previous_hash());
        chain.update(&entry);

        let mut entries = self.entries.write();
        entries.push(entry.clone());

        Ok(entry)
    }

    async fn flush(&self) -> Result<()> {
        // No-op for memory sink
        Ok(())
    }

    async fn entry_count(&self) -> Result<u64> {
        Ok(self.chain.read().entry_count())
    }
}

/// File-based audit sink with append-only writes
pub struct FileAuditSink {
    path: PathBuf,
    chain: Arc<RwLock<IntegrityChain>>,
}

impl FileAuditSink {
    /// Create a new file sink
    pub async fn new(path: PathBuf) -> Result<Self> {
        // Load existing chain state if file exists
        let chain = if path.exists() {
            Self::load_chain_state(&path).await?
        } else {
            // Create parent directories if needed
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            IntegrityChain::new()
        };

        Ok(Self {
            path,
            chain: Arc::new(RwLock::new(chain)),
        })
    }

    /// Load chain state from existing file
    async fn load_chain_state(path: &PathBuf) -> Result<IntegrityChain> {
        let file = File::open(path).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        let mut last_hash = None;
        let mut count = 0u64;

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }

            let entry: AuditEntry = serde_json::from_str(&line)?;
            last_hash = Some(entry.entry_hash);
            count += 1;
        }

        Ok(IntegrityChain::from_state(last_hash, count))
    }

    /// Get the file path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Read all entries from file
    pub async fn read_all(&self) -> Result<Vec<AuditEntry>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut entries = Vec::new();

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            let entry: AuditEntry = serde_json::from_str(&line)?;
            entries.push(entry);
        }

        Ok(entries)
    }
}

#[async_trait]
impl AuditSink for FileAuditSink {
    async fn write(&self, partial: PartialAuditEntry) -> Result<AuditEntry> {
        // Finalize entry with chain - scope the lock to avoid holding across await
        let (entry, json) = {
            let mut chain = self.chain.write();
            let entry = partial.finalize(chain.previous_hash());
            chain.update(&entry);
            let json = serde_json::to_string(&entry)?;
            (entry, json)
        };

        // Append to file (lock is dropped here)
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;

        file.write_all(json.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        Ok(entry)
    }

    async fn flush(&self) -> Result<()> {
        // File is flushed on each write
        Ok(())
    }

    async fn entry_count(&self) -> Result<u64> {
        Ok(self.chain.read().entry_count())
    }
}

/// Composite sink that writes to multiple sinks
pub struct CompositeSink {
    sinks: Vec<Arc<dyn AuditSink>>,
}

impl CompositeSink {
    /// Create a new composite sink
    pub fn new(sinks: Vec<Arc<dyn AuditSink>>) -> Self {
        Self { sinks }
    }

    /// Add a sink
    pub fn add(&mut self, sink: Arc<dyn AuditSink>) {
        self.sinks.push(sink);
    }
}

#[async_trait]
impl AuditSink for CompositeSink {
    async fn write(&self, partial: PartialAuditEntry) -> Result<AuditEntry> {
        if self.sinks.is_empty() {
            return Err(ObservabilityError::Audit("No sinks configured".into()));
        }

        // Write to first sink to get the finalized entry
        let entry = self.sinks[0].write(partial.clone()).await?;

        // Write the same finalized entry to other sinks
        // Note: This means other sinks must accept already-finalized entries
        // For now, we re-finalize for each sink which may create different hashes
        for sink in self.sinks.iter().skip(1) {
            let _ = sink.write(partial.clone()).await?;
        }

        Ok(entry)
    }

    async fn flush(&self) -> Result<()> {
        for sink in &self.sinks {
            sink.flush().await?;
        }
        Ok(())
    }

    async fn entry_count(&self) -> Result<u64> {
        // Return count from first sink
        if let Some(sink) = self.sinks.first() {
            sink.entry_count().await
        } else {
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::entry::{AuditAction, AuditActor, AuditOutcome, AuditResource, AuditEntry as Entry};

    fn create_partial_entry() -> PartialAuditEntry {
        Entry::builder()
            .platform("development")
            .actor(AuditActor::system("test"))
            .action(AuditAction::SystemStarted)
            .resource(AuditResource::system("test"))
            .outcome(AuditOutcome::success())
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn test_memory_sink() {
        let sink = MemoryAuditSink::new();

        let entry1 = sink.write(create_partial_entry()).await.unwrap();
        let entry2 = sink.write(create_partial_entry()).await.unwrap();

        assert_eq!(sink.entry_count().await.unwrap(), 2);
        assert!(entry2.previous_hash.is_some());
        assert_eq!(entry2.previous_hash, Some(entry1.entry_hash.clone()));

        let entries = sink.entries();
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn test_file_sink() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("audit.jsonl");

        let sink = FileAuditSink::new(path.clone()).await.unwrap();

        sink.write(create_partial_entry()).await.unwrap();
        sink.write(create_partial_entry()).await.unwrap();
        sink.write(create_partial_entry()).await.unwrap();

        assert_eq!(sink.entry_count().await.unwrap(), 3);

        // Read back entries
        let entries = sink.read_all().await.unwrap();
        assert_eq!(entries.len(), 3);

        // Verify chain
        use crate::audit::integrity::IntegrityVerifier;
        let result = IntegrityVerifier::verify_chain(&entries).unwrap();
        assert!(result.valid);
    }

    #[tokio::test]
    async fn test_file_sink_persistence() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("audit.jsonl");

        // Write some entries
        {
            let sink = FileAuditSink::new(path.clone()).await.unwrap();
            sink.write(create_partial_entry()).await.unwrap();
            sink.write(create_partial_entry()).await.unwrap();
        }

        // Reopen and continue
        {
            let sink = FileAuditSink::new(path.clone()).await.unwrap();
            assert_eq!(sink.entry_count().await.unwrap(), 2);

            sink.write(create_partial_entry()).await.unwrap();
            assert_eq!(sink.entry_count().await.unwrap(), 3);

            // Verify chain is intact
            let entries = sink.read_all().await.unwrap();
            use crate::audit::integrity::IntegrityVerifier;
            let result = IntegrityVerifier::verify_chain(&entries).unwrap();
            assert!(result.valid);
        }
    }
}
